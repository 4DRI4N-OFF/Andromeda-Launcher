slint::include_modules!();

fn main() -> Result<(), slint::PlatformError> {
    let t0 = std::time::Instant::now();
    let ui = MainWindow::new()?;
    let ready_ms = t0.elapsed().as_millis();
    if std::env::var("UI_LAB_DEMO").is_ok() {
        ui.set_demo_mode(true);
        ui.on_demo_done(move || {
            let done_ms = t0.elapsed().as_millis();
            if let Ok(path) = std::env::var("UI_LAB_METRICS") {
                let _ = std::fs::write(
                    path,
                    format!(
                        "{{\"app\":\"slint\",\"window_create_ms\":{},\"demo_done_ms\":{}}}",
                        ready_ms, done_ms
                    ),
                );
            }
            let _ = slint::quit_event_loop();
        });
    }
    ui.run()
}
