#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

mod intro;
mod native_login;
use andromeda_core::settings::Settings;
use slint::ComponentHandle;
use std::{
    rc::Rc,
    time::{Duration, Instant},
};
slint::include_modules!();

/// Daily logs would otherwise accumulate forever; keep the last two weeks.
fn prune_old_logs(logs: &std::path::Path, keep: Duration) {
    let Ok(entries) = std::fs::read_dir(logs) else {
        return;
    };
    let now = std::time::SystemTime::now();
    for entry in entries.flatten() {
        let Ok(meta) = entry.metadata() else {
            continue;
        };
        if !meta.is_file() {
            continue;
        }
        let expired = meta
            .modified()
            .ok()
            .and_then(|modified| now.duration_since(modified).ok())
            .is_some_and(|age| age > keep);
        if expired {
            let _ = std::fs::remove_file(entry.path());
        }
    }
}
fn refresh_instances(
    ui: &MainWindow,
    shared: &std::sync::Arc<std::sync::Mutex<Vec<andromeda_core::instances::Instance>>>,
    items: Vec<andromeda_core::instances::Instance>,
    select: Option<&str>,
) {
    // Keep the current selection when the instance still exists, otherwise
    // jump to the freshly created one, otherwise fall back to the default.
    let previous = match shared.lock() {
        Ok(guard) => ui
            .get_instance_index()
            .checked_sub(1)
            .and_then(|n| guard.get(n as usize))
            .map(|i| i.id.clone()),
        Err(_) => None,
    };
    let desired = select.map(str::to_owned).or(previous);
    let index = desired
        .as_deref()
        .and_then(|id| items.iter().position(|i| i.id == id))
        .map(|n| n + 1)
        .unwrap_or(0);
    let local_name = if index > 0 {
        Some(items[index - 1].local_name.clone())
    } else {
        None
    };
    let mut labels: Vec<slint::SharedString> = vec!["Default profile (phase 2)".into()];
    labels.extend(items.iter().map(|i| i.name.clone().into()));
    *shared.lock().expect("instance state") = items;
    ui.set_instance_names(slint::ModelRc::new(slint::VecModel::from(labels)));
    ui.set_instance_index(index as i32);
    if let Some(name) = local_name {
        ui.set_local_name(name.into());
    }
}
fn main() -> Result<(), slint::PlatformError> {
    let dir = andromeda_core::paths::data_dir();
    let logs = dir.join("logs");
    let _ = std::fs::create_dir_all(&logs);
    prune_old_logs(&logs, Duration::from_secs(14 * 24 * 60 * 60));
    let appender = tracing_appender::rolling::daily(&logs, "andromeda.log");
    let (writer, _guard) = tracing_appender::non_blocking(appender);
    tracing_subscriber::fmt()
        .with_writer(writer)
        .with_ansi(false)
        .init();
    // A panic inside a worker thread would otherwise vanish silently in a
    // windowed release build.
    let previous_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        tracing::error!("panic: {info}");
        previous_hook(info);
    }));
    tracing::info!("Andromeda starting");
    let settings_path = dir.join("settings.json");
    let s = Settings::load(&settings_path);
    let ui = MainWindow::new()?;
    tracing::info!("Launcher UI created");
    ui.set_reduce_animations(s.reduce_animations);
    ui.set_dark_mode(s.dark_mode);
    let timer = Rc::new(slint::Timer::default());
    if intro::should_play(s.reduce_animations) {
        match intro::decode(intro::FRAMES[0]) {
            Ok(image) => {
                ui.set_intro_frame(image);
                ui.set_intro_visible(true);
                let weak = ui.as_weak();
                let timer_weak = Rc::downgrade(&timer);
                let start = Instant::now();
                timer.start(
                    slint::TimerMode::Repeated,
                    Duration::from_millis(33),
                    move || {
                        let Some(ui) = weak.upgrade() else {
                            if let Some(t) = timer_weak.upgrade() {
                                t.stop();
                            }
                            return;
                        };
                        let frame = (start.elapsed().as_secs_f64() * 30.0) as usize;
                        if !ui.get_intro_visible() || frame >= intro::FRAMES.len() {
                            ui.set_intro_visible(false);
                            if let Some(t) = timer_weak.upgrade() {
                                t.stop();
                            }
                            return;
                        }
                        match intro::decode(intro::FRAMES[frame]) {
                            Ok(image) => ui.set_intro_frame(image),
                            Err(e) => {
                                tracing::warn!("Intro frame failed: {e}");
                                ui.set_intro_visible(false);
                                if let Some(t) = timer_weak.upgrade() {
                                    t.stop();
                                }
                            }
                        }
                    },
                );
            }
            Err(e) => tracing::warn!("Intro unavailable, opening launcher: {e}"),
        }
    }
    let weak = ui.as_weak();
    ui.on_settings_changed(move || {
        if let Some(ui) = weak.upgrade() {
            let new = Settings {
                reduce_animations: ui.get_reduce_animations(),
                dark_mode: ui.get_dark_mode(),
            };
            if let Err(e) = new.save(&settings_path) {
                tracing::error!("could not save settings: {e}");
            }
        }
    });
    let session_path = dir.join("auth/session.dpapi");
    let auth_cancel = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    let auth_weak = ui.as_weak();
    let auth_path = session_path.clone();
    let auth_cancel_action = auth_cancel.clone();
    ui.on_account_action(move |action| {
        let Some(ui) = auth_weak.upgrade() else {
            return;
        };
        if action == "cancel" {
            auth_cancel_action.store(true, std::sync::atomic::Ordering::Relaxed);
            ui.set_account_status("Cancelling sign-in...".into());
            return;
        }
        if ui.get_launch_busy() || ui.get_auth_busy() || ui.get_install_busy() {
            return;
        }
        if action == "signout" {
            match andromeda_core::session_store::remove(&auth_path) {
                Ok(()) => {
                    ui.set_use_microsoft(false);
                    ui.set_account_status(
                        "Local session removed. Microsoft consent is unchanged.".into(),
                    );
                }
                Err(e) => ui.set_account_status(e.to_string().into()),
            }
            return;
        }
        if action != "signin" {
            return;
        }
        auth_cancel_action.store(false, std::sync::atomic::Ordering::Relaxed);
        ui.set_auth_busy(true);
        ui.set_account_status("Opening the Microsoft sign-in window...".into());
        let weak = auth_weak.clone();
        let path = auth_path.clone();
        let cancel = auth_cancel_action.clone();
        std::thread::spawn(move || {
            let result = (|| -> Result<String, andromeda_core::auth::AuthError> {
                tracing::info!("Sign-in worker entered");
                let session = native_login::sign_in(&cancel)?;
                if cancel.load(std::sync::atomic::Ordering::Relaxed) {
                    return Err(andromeda_core::auth::AuthError::Cancelled);
                }
                andromeda_core::session_store::save(&path, &session)?;
                Ok(session.profile.name)
            })();
            let _ = weak.upgrade_in_event_loop(move |ui| {
                ui.set_auth_busy(false);
                match result {
                    Ok(name) => {
                        ui.set_use_microsoft(true);
                        ui.set_account_status(
                            format!(
                                "Signed in as {name}. Session protected for this Windows user."
                            )
                            .into(),
                        );
                    }
                    Err(e) => ui.set_account_status(e.to_string().into()),
                }
            });
        });
    });
    // Restore the protected account without opening the Microsoft page.
    // Do not delete a saved session on transient network/refresh failures.
    if session_path.is_file() {
        ui.set_auth_busy(true);
        ui.set_account_status("Restoring your protected Microsoft session...".into());
        let weak = ui.as_weak();
        let path = session_path.clone();
        let cancel = auth_cancel.clone();
        std::thread::spawn(move || {
            let result = (|| {
                let http = andromeda_core::auth::Http::new()?;
                andromeda_core::session_store::refresh(&path, &http)
            })();
            if cancel.load(std::sync::atomic::Ordering::Relaxed) {
                return;
            }
            let _ = weak.upgrade_in_event_loop(move |ui| {
                ui.set_auth_busy(false);
                match result {
                    Ok(session) => {
                        ui.set_use_microsoft(true);
                        ui.set_account_status(format!("Signed in as {}. Protected session restored.", session.profile.name).into());
                        tracing::info!("Protected Microsoft session restored");
                    }
                    Err(e) => {
                        ui.set_use_microsoft(false);
                        ui.set_account_status(format!("Could not restore your saved account: {e} Your protected session was kept. Sign in again or retry Microsoft Play.").into());
                        tracing::warn!("Protected Microsoft session restore failed: {e}");
                    }
                }
            });
        });
    }
    if !dir.join("vanilla/install-receipt.json").is_file() {
        let version = andromeda_core::launch::VERSION;
        ui.set_launch_status(format!(
            "Game files for {version} are not installed yet. Choose Install or repair to download them."
        ).into());
    }
    let instance_root = dir.join("instances");
    let instances = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    match andromeda_core::instances::list(&instance_root) {
        Ok(items) => refresh_instances(&ui, &instances, items, None),
        Err(e) => ui.set_launch_status(format!("Could not read instances: {e}").into()),
    }
    // The trash lives on disk: a restart must not hide restorable instances.
    ui.set_can_restore(andromeda_core::instances::last_trashed(&instance_root).is_some());
    let change_weak = ui.as_weak();
    let change_instances = instances.clone();
    ui.on_instance_selected(move |index| {
        if let Some(ui) = change_weak.upgrade() {
            ui.set_trash_armed(false);
            if let Some(i) = index
                .checked_sub(1)
                .and_then(|n| change_instances.lock().ok()?.get(n as usize).cloned())
            {
                ui.set_local_name(i.local_name.into());
            }
        }
    });
    let manage_weak = ui.as_weak();
    let manage_instances = instances.clone();
    let manage_root = instance_root.clone();
    ui.on_manage_instance(move |action, name| {
        let Some(ui) = manage_weak.upgrade() else { return; };
        if ui.get_launch_busy() || ui.get_auth_busy() || ui.get_install_busy() { return; }
        let selected = ui.get_instance_index().checked_sub(1).and_then(|n| manage_instances.lock().ok()?.get(n as usize).cloned());
        if action == "trash" && !ui.get_trash_armed() {
            ui.set_trash_armed(true);
            ui.set_launch_status("Click Confirm trash to move this instance and its saves to recoverable launcher trash.".into());
            return;
        }
        ui.set_trash_armed(false);
        ui.set_launch_busy(true);
        ui.set_launch_status("Updating instances...".into());
        let weak = manage_weak.clone();
        let root = manage_root.clone();
        let shared = manage_instances.clone();
        let local_name = ui.get_local_name().to_string();
        let name = name.to_string(); let action = action.to_string();
        std::thread::spawn(move || {
            let mut select: Option<String> = None;
            let result = (|| -> Result<Vec<andromeda_core::instances::Instance>, Box<dyn std::error::Error + Send + Sync>> {
                match action.as_str() {
                    "create" => { let made = andromeda_core::instances::create(&root, &name, &local_name, 2048)?; select = Some(made.id); }
                    "duplicate" => { let i = selected.ok_or("Select an instance, not the default profile")?; let made = andromeda_core::instances::duplicate(&root, &i.id, &name)?; select = Some(made.id); }
                    "trash" => { let i = selected.ok_or("The default profile cannot be trashed here")?; andromeda_core::instances::trash(&root, &i.id)?; }
                    "restore" => { let id = andromeda_core::instances::last_trashed(&root).ok_or("Trash is empty: nothing to restore")?; andromeda_core::instances::restore(&root, &id)?; }
                    _ => return Err("Unknown instance action".into()),
                }
                andromeda_core::instances::list(&root)
            })();
            let can_restore = andromeda_core::instances::last_trashed(&root).is_some();
            let _ = weak.upgrade_in_event_loop(move |ui| {
                match result {
                    Ok(items) => { refresh_instances(&ui, &shared, items, select.as_deref()); ui.set_launch_status("Instances updated. Saved worlds remain in their own folders.".into()); }
                    Err(e) => ui.set_launch_status(format!("Could not update instances: {e}").into()),
                }
                ui.set_can_restore(can_restore); ui.set_launch_busy(false);
            });
        });
    });
    let launch_weak = ui.as_weak();
    let vanilla_dir = dir.join("vanilla");
    let install_root = vanilla_dir.clone();
    ui.on_launch_offline(move |name| {
        let Some(ui) = launch_weak.upgrade() else {
            return;
        };
        if ui.get_launch_busy() || ui.get_auth_busy() || ui.get_install_busy() {
            return;
        }
        let selected = ui
            .get_instance_index()
            .checked_sub(1)
            .and_then(|n| instances.lock().ok()?.get(n as usize).cloned());
        let selected_game = selected
            .as_ref()
            .map(|i| andromeda_core::instances::game_dir(&instance_root, &i.id));
        if let Some(Err(e)) = selected_game.as_ref() {
            ui.set_launch_status(format!("Could not open instance: {e}").into());
            return;
        }
        let game_dir = selected_game.and_then(Result::ok);
        let memory = selected.as_ref().map(|i| i.memory_mb).unwrap_or(2048);
        ui.set_launch_busy(true);
        ui.set_launch_status(
            format!(
                "Preparing installed vanilla {}...",
                andromeda_core::launch::VERSION
            )
            .into(),
        );
        let weak = launch_weak.clone();
        let root = vanilla_dir.clone();
        let name = name.to_string();
        let use_microsoft = ui.get_use_microsoft();
        let account_path = session_path.clone();
        std::thread::spawn(move || {
            let run = (|| -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
                let version = andromeda_core::launch::VERSION;
                let plan = if use_microsoft {
                    let http = andromeda_core::auth::Http::new()?;
                    let session = andromeda_core::session_store::refresh(&account_path, &http)?;
                    let game = game_dir.unwrap_or_else(|| root.join("game"));
                    andromeda_core::launch::prepare_authenticated(
                        &root, version, memory, &game, &session,
                    )?
                } else {
                    match game_dir {
                        Some(game) => andromeda_core::launch::prepare_in_game_dir(
                            &root, version, &name, memory, &game,
                        )?,
                        None => andromeda_core::launch::prepare(&root, version, &name, memory)?,
                    }
                };
                let mut game = plan.spawn()?;
                tracing::info!(pid = game.id(), "Minecraft started");
                let _ = weak.upgrade_in_event_loop(|ui| {
                    ui.set_launch_status(
                        "Minecraft is running. Close the game to play again.".into(),
                    );
                });
                let status = game.wait()?;
                if !status.success() {
                    return Err(format!(
                        "Minecraft exited with {status}; see the game folder's launcher-logs"
                    )
                    .into());
                }
                Ok(())
            })();
            let status = match run {
                Ok(()) => "Game closed. Ready to play again.".to_owned(),
                Err(e) => {
                    tracing::error!("Minecraft launch failed: {e}");
                    format!("Could not start Minecraft: {e}")
                }
            };
            let _ = weak.upgrade_in_event_loop(move |ui| {
                ui.set_launch_busy(false);
                ui.set_launch_status(status.into());
            });
        });
    });

    let install_weak = ui.as_weak();
    ui.on_install_game(move || {
        let Some(ui) = install_weak.upgrade() else {
            return;
        };
        if ui.get_launch_busy() || ui.get_auth_busy() || ui.get_install_busy() {
            return;
        }
        ui.set_install_busy(true);
        ui.set_install_progress(0.0);
        let version = andromeda_core::launch::VERSION;
        ui.set_launch_status(format!("Preparing to install vanilla {version}...").into());
        let weak = install_weak.clone();
        let root = install_root.clone();
        // Throttle UI updates to whole-percent changes: install reports after
        // every file and thousands of assets would flood the event loop.
        let last_pct = std::sync::atomic::AtomicU32::new(u32::MAX);
        let slot = std::sync::Mutex::new(install_weak.clone());
        let progress = move |phase: &str, done: u64, total: u64| {
            let pct = (done * 100).checked_div(total).unwrap_or(0).min(100) as u32;
            if last_pct.swap(pct, std::sync::atomic::Ordering::Relaxed) == pct {
                return;
            }
            let text = format!("Installing - {phase}: {pct}% ({done}/{total} files)");
            if let Ok(weak) = slot.lock() {
                let _ = weak.upgrade_in_event_loop(move |ui| {
                    ui.set_install_progress(pct as f32 / 100.0);
                    ui.set_launch_status(text.into());
                });
            }
        };
        std::thread::spawn(move || {
            let result = andromeda_core::vanilla::Downloader::new().and_then(|downloader| {
                downloader.install_with_progress(&root, andromeda_core::launch::VERSION, &progress)
            });
            let status = match result {
                Ok(_) => format!(
                    "Game files for {} installed and verified. Ready to play.",
                    andromeda_core::launch::VERSION
                ),
                Err(e) => {
                    tracing::error!("Game install failed: {e}");
                    format!("Install failed: {e}")
                }
            };
            let _ = weak.upgrade_in_event_loop(move |ui| {
                ui.set_install_busy(false);
                ui.set_install_progress(0.0);
                ui.set_launch_status(status.into());
            });
        });
    });

    tracing::info!("Showing launcher window");
    ui.show()?;
    tracing::info!("Launcher shown, entering event loop");
    let result = ui.run();
    auth_cancel.store(true, std::sync::atomic::Ordering::Relaxed);
    result
}
