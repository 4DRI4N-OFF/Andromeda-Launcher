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
use crate::InteractionMotion as Motion;
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
    ui.window().set_size(slint::PhysicalSize::new(1100, 760));
    // Bind the production sampler, but drive the production clock deterministically.
    let timer = motion_ui::install(&ui);
    timer.stop();
    let theme_timer = Rc::new(slint::Timer::default());
    let packs = andromeda_core::themes::builtins();
    theme_ui::apply(&ui, &packs[0], false, &theme_timer);
    ui.set_theme_choices(theme_ui::choices(&packs));
    ui.set_instance_names(slint::ModelRc::new(slint::VecModel::from(vec![
        "Default profile".into(),
        "Fabric test".into(),
        "Quilt test".into(),
        "Vanilla test".into(),
        "Creative world".into(),
    ])));
    ui.set_theme_status(
        "Motion proof: deterministic 30 fps, real MainWindow and pointer events".into(),
    );
    ui.show()?;
    std::fs::create_dir_all("motion-frames").unwrap();
    let mut nav_peak = 0f32;
    for frame in 0..150 {
        ui.global::<Motion>().set_clock(1.0 + frame as f32 / 30.0);
        // Render once before events, so hit targets and bindings are materialized.
        if frame == 0 {
            capture(&window, "motion-initial.bmp", 1100, 760);
        }
        match frame {
            12 => pointer(&window, 700., 440., true),
            18 => pointer(&window, 700., 440., false),
            30 => {
                pointer(&window, 300., 45., true);
                pointer(&window, 300., 45., false);
                assert!(ui.get_settings_open());
            }
            60 => {
                pointer(&window, 375., 45., true);
                pointer(&window, 375., 45., false);
                assert!(ui.get_appearance_open());
            }
            75 => pointer(&window, 550., 350., true),
            81 => pointer(&window, 550., 350., false),
            86 => {
                ui.set_settings_open(false);
                ui.set_appearance_open(false);
            }
            90 => {
                ui.set_settings_open(true);
            } // rapid retarget while moving
            93 => {
                ui.set_reduce_animations(true);
            }
            96 => {
                ui.set_settings_open(false);
            }
            108 => {
                ui.set_appearance_open(true);
            }
            120 => {
                ui.set_reduce_animations(false);
                ui.set_settings_open(false);
                ui.set_appearance_open(false);
            }
            _ => {}
        }
        slint::platform::update_timers_and_animations();
        capture(&window, &format!("motion-frames/{frame:04}.bmp"), 1100, 760);
        if (31..54).contains(&frame) {
            nav_peak = nav_peak.max(ui.get_nav_position());
        }
        if frame == 54 {
            assert!((ui.get_nav_position() - 1.).abs() < 0.001);
        }
        if frame == 94 {
            assert_eq!(ui.get_nav_position(), 1.);
            assert_eq!(ui.get_page_phase(), 1.);
        }
        if frame == 97 {
            assert_eq!(ui.get_nav_position(), 0.);
            assert_eq!(ui.get_page_phase(), 1.);
        }
    }
    assert!(
        nav_peak > 1.01,
        "nav spring must overshoot rather than interpolate linearly"
    );
    assert!((ui.get_nav_position()).abs() < 0.001);
    ui.set_reduce_animations(true);
    ui.window().set_size(slint::PhysicalSize::new(860, 620));
    for p in &packs {
        theme_ui::apply(&ui, p, false, &theme_timer);
        ui.set_appearance_open(true);
        capture(&window, &format!("motion-minimum-{}.bmp", p.id), 860, 620);
    }
    let path = std::env::temp_dir().join(format!("motion-settings-{}.json", std::process::id()));
    theme_ui::save_selection(&ui, &path).unwrap();
    std::fs::remove_file(path).unwrap();
    println!("Motion proof passed: 150 deterministic frames, real pointer press/release, tab overshoot/settlement, rapid retarget, reduced motion mid-flight and instant pages, minimum-size themes.");
    Ok(())
}
