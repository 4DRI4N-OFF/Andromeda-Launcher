//! Offscreen rendering of the real Slint component: no native window or account access.
use slint::{
    platform::{
        software_renderer::{MinimalSoftwareWindow, RepaintBufferType},
        Platform, WindowAdapter,
    },
    ComponentHandle, Rgb8Pixel,
};
use std::rc::Rc;
slint::include_modules!();
#[path = "../src/motion_ui.rs"]
mod motion_ui;
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
fn pointer(window: &MinimalSoftwareWindow, x: f32, y: f32, down: bool) {
    let position = slint::LogicalPosition::new(x, y);
    let button = slint::platform::PointerEventButton::Left;
    window.dispatch_event(if down {
        slint::platform::WindowEvent::PointerPressed { position, button }
    } else {
        slint::platform::WindowEvent::PointerReleased { position, button }
    });
}
fn main() -> Result<(), slint::PlatformError> {
    let window = MinimalSoftwareWindow::new(RepaintBufferType::NewBuffer);
    slint::platform::set_platform(Box::new(Offscreen(window.clone()))).unwrap();
    let ui = MainWindow::new()?;
    ui.window().set_size(slint::PhysicalSize::new(1600, 900));
    let timer = motion_ui::install(&ui);
    timer.stop();
    let theme_timer = Rc::new(slint::Timer::default());
    let packs = andromeda_core::themes::builtins();
    ui.set_theme_choices(theme_ui::choices(&packs));
    ui.set_instance_names(slint::ModelRc::new(slint::VecModel::from(vec![
        "Default profile".into(),
        "Fabric test".into(),
        "Quilt test".into(),
    ])));
    ui.set_theme_status("Custom packs: data/themes/*.json. Loaded on restart.".into());
    ui.set_reduce_animations(true);
    ui.show()?;
    for pack in &packs {
        theme_ui::apply(&ui, pack, false, &theme_timer);
        for tab in 0..5 {
            pointer(&window, 616.0 + tab as f32 * 92.0, 69.0, true);
            pointer(&window, 616.0 + tab as f32 * 92.0, 69.0, false);
            assert_eq!(ui.get_active_tab(), tab);
            capture(&window, &format!("shell-{}-{tab}.bmp", pack.id), 1600, 900);
        }
    }
    let settings_path = std::env::temp_dir().join("shell-proof-settings.json");
    theme_ui::save_selection(&ui, &settings_path).unwrap();
    std::fs::remove_file(settings_path).unwrap();
    for pack in &packs {
        theme_ui::apply(&ui, pack, false, &theme_timer);
        ui.window().set_size(slint::PhysicalSize::new(860, 620));
        ui.set_active_tab(3);
        capture(&window, &format!("shell-minimum-{}.bmp", pack.id), 860, 620);
    }
    println!("Shell proof passed: all five top routes via real pointer, three theme shells at reference and minimum size. No native window or account/game side effects.");
    Ok(())
}
