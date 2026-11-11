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
    ])));
    ui.set_theme_status("Custom packs: data/themes/*.json. Loaded on restart.".into());
    ui.set_reduce_animations(true);
    ui.show()?;
    capture(&window, "grouped-play.bmp", 1100, 760);
    ui.set_settings_open(true);
    for i in 0..3 {
        ui.set_settings_section(i);
        capture(&window, &format!("grouped-section-{i}.bmp"), 1100, 760);
    }
    for p in &packs {
        theme_ui::apply(&ui, p, false, &theme_timer);
        ui.window().set_size(slint::PhysicalSize::new(860, 620));
        capture(&window, &format!("grouped-minimum-{}.bmp", p.id), 860, 620);
    }
    ui.window().set_size(slint::PhysicalSize::new(1100, 760));
    theme_ui::apply(&ui, &packs[0], false, &theme_timer);
    let path = std::env::temp_dir().join(format!("grouped-settings-{}.json", std::process::id()));
    theme_ui::save_selection(&ui, &path).unwrap();
    std::fs::remove_file(path).unwrap();
    // Initialize actual interaction callbacks after the review captures.
    let selected = Rc::new(std::cell::RefCell::new(String::new()));
    let result = selected.clone();
    ui.on_theme_selected(move |id| {
        *result.borrow_mut() = id.to_string();
    });
    // Actual pointer routes select sections and call the same callbacks as production.
    for (y, expected) in [(165., 0), (250., 1), (340., 2)] {
        pointer(&window, 120., y, true);
        pointer(&window, 120., y, false);
        capture(&window, "grouped-pointer.bmp", 1100, 760);
        assert_eq!(ui.get_settings_section(), expected);
    }
    pointer(&window, 650., 320., true);
    pointer(&window, 650., 320., false);
    assert_eq!(selected.borrow().as_str(), "alba");
    let actions = Rc::new(std::cell::RefCell::new(Vec::<String>::new()));
    let result = actions.clone();
    ui.on_account_action(move |action| result.borrow_mut().push(action.to_string()));
    ui.set_settings_section(1);
    capture(&window, "grouped-account-before-click.bmp", 1100, 760);
    // Sign-in/cancel/sign-out are callback-only spies, no auth window is opened.
    pointer(&window, 430., 275., true);
    pointer(&window, 430., 275., false);
    ui.set_auth_busy(true);
    pointer(&window, 660., 275., true);
    pointer(&window, 660., 275., false);
    ui.set_auth_busy(false);
    pointer(&window, 900., 275., true);
    pointer(&window, 900., 275., false);
    assert_eq!(&*actions.borrow(), &["signin", "cancel", "signout"]);
    ui.set_instance_index(1);
    ui.set_can_restore(true);
    ui.set_settings_section(0);
    let actions = Rc::new(std::cell::RefCell::new(Vec::<String>::new()));
    let result = actions.clone();
    ui.on_manage_instance(move |action, _| result.borrow_mut().push(action.to_string()));
    capture(&window, "grouped-profiles-before-click.bmp", 1100, 760);
    for x in [380., 530., 680., 830., 980.] {
        pointer(&window, x, 274., true);
        pointer(&window, x, 274., false);
    }
    for x in [430., 680., 930.] {
        pointer(&window, x, 408., true);
        pointer(&window, x, 408., false);
    }
    assert_eq!(
        &*actions.borrow(),
        &[
            "create",
            "createfabric",
            "createquilt",
            "createforge",
            "createneoforge",
            "duplicate",
            "trash",
            "restore"
        ]
    );
    ui.set_auth_busy(true);
    let count = actions.borrow().len();
    pointer(&window, 380., 274., true);
    pointer(&window, 380., 274., false);
    assert_eq!(actions.borrow().len(), count);
    ui.set_auth_busy(false);
    ui.set_settings_section(2);
    capture(&window, "grouped-before-motion-toggle.bmp", 1100, 760);
    pointer(&window, 1020., 478., true);
    pointer(&window, 1020., 478., false);
    assert!(!ui.get_reduce_animations());
    pointer(&window, 1020., 478., true);
    pointer(&window, 1020., 478., false);
    assert!(ui.get_reduce_animations());
    // Identity toggle remains local state, never a sign-in or launch.
    ui.set_settings_section(1);
    capture(&window, "grouped-before-identity-toggle.bmp", 1100, 760);
    pointer(&window, 1020., 303., true);
    pointer(&window, 1020., 303., false);
    assert!(ui.get_use_microsoft());
    ui.set_settings_section(0);
    ui.window().set_size(slint::PhysicalSize::new(860, 620));
    capture(&window, "grouped-minimum-profiles.bmp", 860, 620);
    ui.set_settings_section(1);
    capture(&window, "grouped-minimum-account.bmp", 860, 620);

    let _ = &pointer;
    let _ = ui.global::<Motion>();
    println!("Grouped proof passed: section pointer navigation, theme selection, five loader callbacks, duplicate/trash/undo, busy guard, signin/cancel/signout, identity and motion toggles; all sections and minimum-size themes rendered. No native window opened.");
    Ok(())
}
