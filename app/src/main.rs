#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

mod intro;
use andromeda_core::settings::Settings;
use slint::ComponentHandle;
use std::{
    path::PathBuf,
    rc::Rc,
    time::{Duration, Instant},
};
slint::include_modules!();

fn data_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("workspace root")
        .join("data")
}
fn refresh_instances(
    ui: &MainWindow,
    shared: &std::sync::Arc<std::sync::Mutex<Vec<andromeda_core::instances::Instance>>>,
    items: Vec<andromeda_core::instances::Instance>,
) {
    let mut labels: Vec<slint::SharedString> = vec!["Default profile (phase 2)".into()];
    labels.extend(items.iter().map(|i| i.name.clone().into()));
    *shared.lock().expect("instance state") = items;
    ui.set_instance_names(slint::ModelRc::new(slint::VecModel::from(labels)));
    ui.set_instance_index(0);
}
fn main() -> Result<(), slint::PlatformError> {
    let dir = data_dir();
    let _ = std::fs::create_dir_all(dir.join("logs"));
    let appender = tracing_appender::rolling::daily(dir.join("logs"), "andromeda.log");
    let (writer, _guard) = tracing_appender::non_blocking(appender);
    tracing_subscriber::fmt()
        .with_writer(writer)
        .with_ansi(false)
        .init();
    tracing::info!("Andromeda starting");
    let settings_path = dir.join("settings.json");
    let s = Settings::load(&settings_path);
    let ui = MainWindow::new()?;
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
    if session_path.is_file() {
        ui.set_account_status(
            "A protected session is saved. It will be verified when you play with Microsoft."
                .into(),
        );
    }
    let auth_cancel = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    let auth_weak = ui.as_weak();
    let auth_path = session_path.clone();
    let auth_cancel_action = auth_cancel.clone();
    ui.on_account_action(move |action| {
        let Some(ui) = auth_weak.upgrade() else { return; };
        if action == "cancel" {
            auth_cancel_action.store(true, std::sync::atomic::Ordering::Relaxed);
            ui.set_account_status("Cancelling sign-in...".into());
            return;
        }
        if ui.get_launch_busy() || ui.get_auth_busy() { return; }
        if action == "signout" {
            match andromeda_core::session_store::remove(&auth_path) {
                Ok(()) => { ui.set_use_microsoft(false); ui.set_account_status("Local session removed. Microsoft consent is unchanged.".into()); }
                Err(e) => ui.set_account_status(e.to_string().into()),
            }
            return;
        }
        if action != "signin" { return; }
        auth_cancel_action.store(false, std::sync::atomic::Ordering::Relaxed);
        ui.set_auth_busy(true);
        ui.set_account_status("Requesting a code from Microsoft...".into());
        let weak = auth_weak.clone(); let path = auth_path.clone(); let cancel = auth_cancel_action.clone();
        std::thread::spawn(move || {
            let result = (|| -> Result<String, andromeda_core::auth::AuthError> {
                let http = andromeda_core::auth::Http::new()?;
                let code = andromeda_core::auth::begin(&http)?;
                let instructions = format!("Open {} and enter {}. Expires in {} minutes. Check that the page names your Kimberlite app before approving.", code.verification_uri, code.user_code, code.expires_in / 60);
                weak.upgrade_in_event_loop(move |ui| ui.set_account_status(instructions.into())).map_err(|_| andromeda_core::auth::AuthError::Cancelled)?;
                let session = andromeda_core::auth::finish(&http, &code, &cancel)?;
                if cancel.load(std::sync::atomic::Ordering::Relaxed) { return Err(andromeda_core::auth::AuthError::Cancelled); }
                andromeda_core::session_store::save(&path, &session)?;
                Ok(session.profile.name)
            })();
            let _ = weak.upgrade_in_event_loop(move |ui| {
                ui.set_auth_busy(false);
                match result {
                    Ok(name) => { ui.set_use_microsoft(true); ui.set_account_status(format!("Signed in as {name}. Session protected for this Windows user.").into()); }
                    Err(e) => ui.set_account_status(e.to_string().into()),
                }
            });
        });
    });
    let instance_root = dir.join("instances");
    let instances = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    match andromeda_core::instances::list(&instance_root) {
        Ok(items) => refresh_instances(&ui, &instances, items),
        Err(e) => ui.set_launch_status(format!("Could not read instances: {e}").into()),
    }
    let last_trash = std::sync::Arc::new(std::sync::Mutex::new(None::<String>));
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
    let manage_trash = last_trash.clone();
    ui.on_manage_instance(move |action, name| {
        let Some(ui) = manage_weak.upgrade() else { return; };
        if ui.get_launch_busy() || ui.get_auth_busy() { return; }
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
        let trash = manage_trash.clone();
        let local_name = ui.get_local_name().to_string();
        let name = name.to_string(); let action = action.to_string();
        std::thread::spawn(move || {
            let result = (|| -> Result<Vec<andromeda_core::instances::Instance>, Box<dyn std::error::Error + Send + Sync>> {
                match action.as_str() {
                    "create" => { andromeda_core::instances::create(&root, &name, &local_name, 2048)?; }
                    "duplicate" => { let i = selected.ok_or("Select an instance, not the default profile")?; andromeda_core::instances::duplicate(&root, &i.id, &name)?; }
                    "trash" => { let i = selected.ok_or("The default profile cannot be trashed here")?; andromeda_core::instances::trash(&root, &i.id)?; *trash.lock().map_err(|_| "Instance lock unavailable")? = Some(i.id); }
                    "restore" => { let id = trash.lock().map_err(|_| "Instance lock unavailable")?.clone().ok_or("No instance to restore in this session")?; andromeda_core::instances::restore(&root, &id)?; *trash.lock().map_err(|_| "Instance lock unavailable")? = None; }
                    _ => return Err("Unknown instance action".into()),
                }
                andromeda_core::instances::list(&root)
            })();
            let can_restore = trash.lock().map(|t| t.is_some()).unwrap_or(false);
            let _ = weak.upgrade_in_event_loop(move |ui| {
                match result {
                    Ok(items) => { refresh_instances(&ui, &shared, items); ui.set_launch_status("Instances updated. Saved worlds remain in their own folders.".into()); }
                    Err(e) => ui.set_launch_status(format!("Could not update instances: {e}").into()),
                }
                ui.set_can_restore(can_restore); ui.set_launch_busy(false);
            });
        });
    });
    let launch_weak = ui.as_weak();
    let vanilla_dir = dir.join("vanilla");
    ui.on_launch_offline(move |name| {
        let Some(ui) = launch_weak.upgrade() else {
            return;
        };
        if ui.get_launch_busy() || ui.get_auth_busy() {
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
        ui.set_launch_status("Preparing installed vanilla 1.21.11...".into());
        let weak = launch_weak.clone();
        let root = vanilla_dir.clone();
        let name = name.to_string();
        let use_microsoft = ui.get_use_microsoft();
        let account_path = session_path.clone();
        std::thread::spawn(move || {
            let run = (|| -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
                let plan = if use_microsoft {
                    let http = andromeda_core::auth::Http::new()?;
                    let session = andromeda_core::session_store::refresh(&account_path, &http)?;
                    let game = game_dir.unwrap_or_else(|| root.join("game"));
                    andromeda_core::launch::prepare_authenticated(
                        &root, "1.21.11", memory, &game, &session,
                    )?
                } else {
                    match game_dir {
                        Some(game) => andromeda_core::launch::prepare_in_game_dir(
                            &root, "1.21.11", &name, memory, &game,
                        )?,
                        None => andromeda_core::launch::prepare(&root, "1.21.11", &name, memory)?,
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
                        "Minecraft exited with {status}; see data/vanilla/game/launcher-logs"
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

    let result = ui.run();
    auth_cancel.store(true, std::sync::atomic::Ordering::Relaxed);
    result
}
