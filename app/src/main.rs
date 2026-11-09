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
        match slint::Image::load_from_data(intro::FRAMES[0], Some("png")) {
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
                        match slint::Image::load_from_data(intro::FRAMES[frame], Some("png")) {
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
    ui.run()
}
