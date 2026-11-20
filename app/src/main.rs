#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]
// Tabler MIT notice is retained in executable distributions.
#[used]
static TABLER_LICENSE: [u8; 1073] = *include_bytes!("../ui/tabler/LICENSE");


mod startup;
mod startup_gate;
mod startup_data;
mod startup_cover;
mod startup_runtime;
mod startup_placement;
mod startup_resize;
mod startup_sheen;
mod motion_ui;
mod native_login;
mod theme_ui;
use andromeda_core::settings::Settings;
use slint::ComponentHandle;
use std::{
    path::PathBuf,
    rc::Rc,
};
slint::include_modules!();

fn data_dir() -> PathBuf {
    // Opt-in portable sandbox for ready-to-open phase test builds only.
    // The normal launcher has no marker and keeps its existing data directory.
    if let Ok(exe) = std::env::current_exe() {
        if let Some(folder) = exe.parent() {
            if folder.join(".andromeda-test").is_file() {
                return folder.join("data");
            }
        }
    }
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
    labels.extend(items.iter().map(|i| {
        format!(
            "{} [{}]",
            i.name,
            match i.loader {
                Some(andromeda_core::instances::Loader::Fabric { .. }) => "Fabric",
                Some(andromeda_core::instances::Loader::Quilt { .. }) => "Quilt",
                Some(andromeda_core::instances::Loader::Forge { .. }) => "Forge",
                Some(andromeda_core::instances::Loader::NeoForge { .. }) => "NeoForge",
                None => "Vanilla",
            }
        )
        .into()
    }));
    let mut titles: Vec<slint::SharedString> = vec!["Default profile".into()];
    let mut details: Vec<slint::SharedString> = vec!["1.21.11 · Vanilla".into()];
    for item in &items {
        titles.push(item.name.clone().into());
        let loader = match &item.loader {
            Some(andromeda_core::instances::Loader::Fabric { version }) => {
                format!("Fabric {version}")
            }
            Some(andromeda_core::instances::Loader::Quilt { version }) => {
                format!("Quilt {version}")
            }
            Some(andromeda_core::instances::Loader::Forge { version }) => {
                format!("Forge {version}")
            }
            Some(andromeda_core::instances::Loader::NeoForge { version }) => {
                format!("NeoForge {version}")
            }
            None => "Vanilla".into(),
        };
        details.push(format!("{} · {}", item.version, loader).into());
    }
    ui.set_profile_titles(slint::ModelRc::new(slint::VecModel::from(titles)));
    ui.set_profile_details(slint::ModelRc::new(slint::VecModel::from(details)));
    *shared.lock().expect("instance state") = items;
    ui.set_instance_names(slint::ModelRc::new(slint::VecModel::from(labels)));
    ui.set_instance_index(0);
}
struct PreparedMain { ui: MainWindow, _motion_timer: Rc<slint::Timer>, auth_cancel: std::sync::Arc<std::sync::atomic::AtomicBool> }
fn main() -> Result<(), slint::PlatformError> {startup_runtime::run(data_dir())}
fn prepare_main(dir: PathBuf, prepared: startup_data::Prepared) -> Result<PreparedMain, slint::PlatformError> {
    let settings_path = dir.join("settings.json");
    let s = prepared.settings;
    let ui = MainWindow::new()?;
    let _motion_timer = motion_ui::install(&ui);
    tracing::info!("Launcher UI created");
    ui.set_reduce_animations(s.reduce_animations);
    ui.set_dark_mode(s.dark_mode);
    let packs = prepared.themes;
    let errors = prepared.warnings;
    for error in &errors {
        tracing::warn!("Theme rejected: {error}");
    }
    ui.set_theme_status(if errors.is_empty() {
        "Custom packs: place validated JSON files in data/themes, then restart. Seasonal overlays are not enabled in this phase.".into()
    } else { format!("{} invalid theme pack(s) skipped. See the launcher log.", errors.len()).into() });
    ui.set_theme_choices(theme_ui::choices(&packs));
    let packs = Rc::new(packs);
    let theme_timer = Rc::new(slint::Timer::default());
    let initial = packs
        .iter()
        .find(|p| p.id == s.theme_id)
        .unwrap_or(&packs[0]);
    theme_ui::apply(&ui, initial, false, &theme_timer);
    let theme_weak = ui.as_weak();
    let theme_packs = packs.clone();
    let switch_timer = theme_timer.clone();
    let theme_settings = settings_path.clone();
    ui.on_theme_selected(move |id| {
        let Some(ui) = theme_weak.upgrade() else {
            return;
        };
        let Some(pack) = theme_packs.iter().find(|p| p.id == id.as_str()) else {
            return;
        };
        theme_ui::apply(&ui, pack, true, &switch_timer);
        if let Err(e) = theme_ui::save_selection(&ui, &theme_settings) {
            ui.set_theme_status(
                format!("Theme applied for this session, but could not save it: {e}").into(),
            );
        }
    });
    let weak = ui.as_weak();
    ui.on_settings_changed(move || {
        if let Some(ui) = weak.upgrade() {
            ui.global::<ThemePalette>()
                .set_reduce_motion(ui.get_reduce_animations());
            let new = Settings {
                reduce_animations: ui.get_reduce_animations(),
                dark_mode: ui.get_dark_mode(),
                theme_id: ui.get_selected_theme().to_string(),
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
        if ui.get_launch_busy() || ui.get_auth_busy() {
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
    if prepared.has_saved_session {
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
    let instance_root = dir.join("instances");
    let instances = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    refresh_instances(&ui, &instances, prepared.instances);
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
                    "create" | "createfabric" | "createquilt" | "createforge" | "createneoforge" => {
                        // Every creation is verified against the live loader
                        // catalogs before an instance is written: the version
                        // pickers feed these values once the UI exposes them,
                        // until then the pinned defaults are the request.
                        use andromeda_core::catalog::{verify_selection, LoaderKind};
                        let dl = andromeda_core::vanilla::Downloader::new()?;
                        let cache = root.join("catalog-cache");
                        let loader = match action.as_str() {
                            "create" => None,
                            "createfabric" => Some((LoaderKind::Fabric, "0.20.1")),
                            "createquilt" => Some((LoaderKind::Quilt, "0.30.2")),
                            "createforge" => Some((LoaderKind::Forge, "61.2.5")),
                            _ => Some((LoaderKind::NeoForge, "21.11.48")),
                        };
                        let sel = verify_selection(&dl, &cache, "1.21.11", loader)?;
                        andromeda_core::instances::create_selected(&root, &name, &local_name, 2048, &sel)?;
                    }
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
        let loader = selected.as_ref().and_then(|i| i.loader.clone());
        let mc = selected
            .as_ref()
            .map(|i| i.version.clone())
            .unwrap_or_else(|| "1.21.11".to_string());
        ui.set_launch_busy(true);
        ui.set_launch_status("Preparing installed vanilla 1.21.11...".into());
        let weak = launch_weak.clone();
        let root = vanilla_dir.clone();
        let name = name.to_string();
        let use_microsoft = ui.get_use_microsoft();
        let account_path = session_path.clone();
        std::thread::spawn(move || {
            let run = (|| -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
                let session = if use_microsoft {
                    let http = andromeda_core::auth::Http::new()?;
                    Some(andromeda_core::session_store::refresh(
                        &account_path,
                        &http,
                    )?)
                } else {
                    None
                };
                let plan = match loader {
                    Some(andromeda_core::instances::Loader::Fabric { version }) => {
                        let game = game_dir.unwrap_or_else(|| root.join("game"));
                        andromeda_core::launch::prepare_fabric(
                            &root,
                            &mc,
                            &name,
                            memory,
                            &game,
                            session.as_ref(),
                            &version,
                        )?
                    }
                    Some(andromeda_core::instances::Loader::Quilt { version }) => {
                        let game = game_dir.unwrap_or_else(|| root.join("game"));
                        andromeda_core::launch::prepare_quilt(
                            &root,
                            &mc,
                            &name,
                            memory,
                            &game,
                            session.as_ref(),
                            &version,
                        )?
                    }
                    Some(andromeda_core::instances::Loader::Forge { version }) => {
                        let game = game_dir.unwrap_or_else(|| root.join("game"));
                        let spec = andromeda_core::upstream_install::Spec::new(
                            andromeda_core::upstream_install::Kind::Forge,
                            &mc,
                            &version,
                        )?;
                        andromeda_core::launch::prepare_upstream(
                            &root, &mc, &name, memory, &game, session.as_ref(), &spec,
                        )?
                    }
                    Some(andromeda_core::instances::Loader::NeoForge { version }) => {
                        let game = game_dir.unwrap_or_else(|| root.join("game"));
                        let spec = andromeda_core::upstream_install::Spec::new(
                            andromeda_core::upstream_install::Kind::NeoForge,
                            &mc,
                            &version,
                        )?;
                        andromeda_core::launch::prepare_upstream(
                            &root, &mc, &name, memory, &game, session.as_ref(), &spec,
                        )?
                    }
                    None => match session {
                        Some(session) => {
                            let game = game_dir.unwrap_or_else(|| root.join("game"));
                            andromeda_core::launch::prepare_authenticated(
                                &root, &mc, memory, &game, &session,
                            )?
                        }
                        None => match game_dir {
                            Some(game) => andromeda_core::launch::prepare_in_game_dir(
                                &root, &mc, &name, memory, &game,
                            )?,
                            None => {
                                andromeda_core::launch::prepare(&root, &mc, &name, memory)?
                            }
                        },
                    },
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

    Ok(PreparedMain {ui,_motion_timer,auth_cancel})
}
