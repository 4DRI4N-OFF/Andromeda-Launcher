#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

use andromeda_core::settings::Settings;
use slint::ComponentHandle;
use std::path::PathBuf;

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
    tracing::info!("Andromeda arranca");

    let settings_path = dir.join("settings.json");
    let s = Settings::load(&settings_path);

    let ui = MainWindow::new()?;
    ui.set_reduce_animations(s.reduce_animations);
    ui.set_dark_mode(s.dark_mode);

    let weak = ui.as_weak();
    ui.on_settings_changed(move || {
        if let Some(ui) = weak.upgrade() {
            let new = Settings {
                reduce_animations: ui.get_reduce_animations(),
                dark_mode: ui.get_dark_mode(),
            };
            if let Err(e) = new.save(&settings_path) {
                tracing::error!("no se pudo guardar ajustes: {e}");
            }
        }
    });

    ui.run()
}
