//! Offscreen rendering of the real Slint component: no native window or account access.
use slint::{
    platform::{
        software_renderer::{MinimalSoftwareWindow, RepaintBufferType},
        Platform, WindowAdapter,
    },
    ComponentHandle, Rgb8Pixel,
};
use std::{rc::Rc, time::Duration};
slint::include_modules!();
#[allow(dead_code)]
#[path = "../src/motion_ui.rs"]
mod motion_ui;
#[allow(dead_code)]
#[path = "../src/theme_ui.rs"]
mod theme_ui;
struct Offscreen(Rc<MinimalSoftwareWindow>);
impl Platform for Offscreen {
    fn create_window_adapter(&self) -> Result<Rc<dyn WindowAdapter>, slint::PlatformError> {
        Ok(self.0.clone())
    }
}
fn capture(window: &MinimalSoftwareWindow, path: &str, w: usize, h: usize) {
    use std::io::Write;
    window.request_redraw();
    let mut pixels = vec![Rgb8Pixel::default(); w * h];
    window.draw_if_needed(|r| {
        r.render(&mut pixels, w);
    });
    let stride = (w * 3 + 3) & !3;
    let mut f = std::fs::File::create(path).unwrap();
    let mut header = vec![0u8; 54];
    header[0..2].copy_from_slice(b"BM");
    header[2..6].copy_from_slice(&((54 + stride * h) as u32).to_le_bytes());
    header[10..14].copy_from_slice(&54u32.to_le_bytes());
    header[14..18].copy_from_slice(&40u32.to_le_bytes());
    header[18..22].copy_from_slice(&(w as i32).to_le_bytes());
    header[22..26].copy_from_slice(&(h as i32).to_le_bytes());
    header[26..28].copy_from_slice(&1u16.to_le_bytes());
    header[28..30].copy_from_slice(&24u16.to_le_bytes());
    f.write_all(&header).unwrap();
    for row in (0..h).rev() {
        let mut bytes = vec![0u8; stride];
        for x in 0..w {
            let p = pixels[row * w + x];
            bytes[x * 3..x * 3 + 3].copy_from_slice(&[p.b, p.g, p.r]);
        }
        f.write_all(&bytes).unwrap();
    }
}
fn tick(ms: u64) {
    std::thread::sleep(Duration::from_millis(ms));
    slint::platform::update_timers_and_animations();
}
fn main() -> Result<(), slint::PlatformError> {
    let window = MinimalSoftwareWindow::new(RepaintBufferType::NewBuffer);
    slint::platform::set_platform(Box::new(Offscreen(window.clone()))).unwrap();
    let ui = MainWindow::new()?;
    let _motion_timer = motion_ui::install(&ui);
    ui.window().set_size(slint::PhysicalSize::new(1600, 900));
    ui.set_theme_choices(theme_ui::choices(&andromeda_core::themes::builtins()));
    ui.set_instance_names(slint::ModelRc::new(slint::VecModel::from(vec![
        "Default profile (phase 2)".into(),
        "Loader test [Fabric]".into(),
        "Local test [Quilt]".into(),
    ])));
    ui.set_theme_status(
        "Custom packs: data/themes/*.json. Validated locally. No seasonal overlays in this phase."
            .into(),
    );
    ui.set_reduce_animations(true);
    ui.set_active_tab(3);
    ui.set_settings_open(true);
    ui.set_active_tab(3);
    ui.set_appearance_open(true);
    let packs = andromeda_core::themes::builtins();
    let timer = Rc::new(slint::Timer::default());
    for p in &packs {
        theme_ui::apply(&ui, p, false, &timer);
        ui.show()?;
        tick(1);
        capture(&window, &format!("theme-{}.bmp", p.id), 1600, 900);
    }
    let proof_dir =
        std::env::temp_dir().join(format!("andromeda-theme-proof-{}", std::process::id()));
    std::fs::create_dir_all(&proof_dir).unwrap();
    let settings_path = proof_dir.join("settings.json");
    let weak = ui.as_weak();
    let callback_packs = packs.clone();
    let callback_timer = timer.clone();
    let callback_path = settings_path.clone();
    ui.on_theme_selected(move |id| {
        let ui = weak.upgrade().unwrap();
        let p = callback_packs.iter().find(|p| p.id == id.as_str()).unwrap();
        theme_ui::apply(&ui, p, true, &callback_timer);
        theme_ui::save_selection(&ui, &callback_path).unwrap();
    });
    // A real pointer click on the middle card must call through to the engine and settings.
    theme_ui::apply(&ui, &packs[0], false, &timer);
    ui.set_reduce_animations(true);
    capture(&window, "theme-before-click.bmp", 1600, 900);
    for pressed in [true, false] {
        let position = slint::LogicalPosition::new(900.0, 490.0);
        let button = slint::platform::PointerEventButton::Left;
        window.dispatch_event(if pressed {
            slint::platform::WindowEvent::PointerPressed { position, button }
        } else {
            slint::platform::WindowEvent::PointerReleased { position, button }
        });
    }
    assert_eq!(ui.get_selected_theme().as_str(), "alba");
    let restored = andromeda_core::settings::Settings::load(&settings_path);
    assert_eq!(restored.theme_id, "alba");
    assert!(restored.reduce_animations);
    capture(&window, "theme-pointer-applied.bmp", 1600, 900);
    std::fs::remove_dir_all(proof_dir).unwrap();
    theme_ui::apply(&ui, &packs[0], false, &timer);
    ui.set_reduce_animations(false);
    theme_ui::apply(&ui, &packs[1], true, &timer);
    tick(80);
    capture(&window, "theme-transition-080.bmp", 1600, 900);
    assert_ne!(
        ui.global::<ThemePalette>().get_bg(),
        slint::Color::from_rgb_u8(247, 221, 206)
    );
    tick(120);
    capture(&window, "theme-transition-200.bmp", 1600, 900);
    tick(400);
    capture(&window, "theme-transition-settled.bmp", 1600, 900);
    assert_eq!(
        ui.global::<ThemePalette>().get_bg(),
        slint::Color::from_rgb_u8(247, 221, 206)
    );
    assert_eq!(ui.global::<ThemePalette>().get_selection_scale(), 1.0);
    ui.set_reduce_animations(true);
    theme_ui::apply(&ui, &packs[2], true, &timer);
    assert_eq!(
        ui.global::<ThemePalette>().get_bg(),
        slint::Color::from_rgb_u8(22, 18, 16)
    );
    assert_eq!(ui.global::<ThemePalette>().get_selection_scale(), 1.0);
    assert!(!timer.running());
    capture(&window, "theme-reduced-motion.bmp", 1600, 900);
    // Turning reduced motion on during a transition settles on the next frame.
    ui.set_reduce_animations(false);
    theme_ui::apply(&ui, &packs[0], true, &timer);
    tick(40);
    ui.set_reduce_animations(true);
    tick(20);
    assert!(!timer.running());
    assert_eq!(ui.global::<ThemePalette>().get_selection_scale(), 1.0);
    ui.set_appearance_open(false);
    ui.set_settings_section(0);
    ui.set_active_tab(0);
    ui.set_settings_open(false);
    capture(&window, "theme-play.bmp", 1600, 900);
    ui.set_active_tab(3);
    ui.set_settings_open(true);
    capture(&window, "theme-settings.bmp", 1600, 900);
    ui.window().set_size(slint::PhysicalSize::new(860, 620));
    ui.set_active_tab(3);
    ui.set_appearance_open(true);
    tick(1);
    capture(&window, "theme-minimum.bmp", 860, 620);
    println!("UI proof passed: all three packs, transition settles, reduce-motion immediate and mid-transition, Play/Settings/minimum-size renders. No native window opened.");
    Ok(())
}
