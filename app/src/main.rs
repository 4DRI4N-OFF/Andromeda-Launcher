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
    let launch_weak = ui.as_weak();
    let vanilla_dir = dir.join("vanilla");
    ui.on_launch_offline(move |name| {
        let Some(ui) = launch_weak.upgrade() else {
            return;
        };
        if ui.get_launch_busy() {
            return;
        }
        ui.set_launch_busy(true);
        ui.set_launch_status("Preparing installed vanilla 1.21.11...".into());
        let weak = launch_weak.clone();
        let root = vanilla_dir.clone();
        let name = name.to_string();
        std::thread::spawn(move || {
            let run = (|| -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
                let plan = andromeda_core::launch::prepare(&root, "1.21.11", &name, 2048)?;
                let mut game = plan.spawn()?;
                tracing::info!(pid = game.id(), "Local-profile Minecraft started");
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
                Ok(()) => "Game closed. Ready to play offline again.".to_owned(),
                Err(e) => {
                    tracing::error!("Local-profile launch failed: {e}");
                    format!("Could not play offline: {e}")
                }
            };
            let _ = weak.upgrade_in_event_loop(move |ui| {
                ui.set_launch_busy(false);
                ui.set_launch_status(status.into());
            });
        });
    });

    ui.run()
}
