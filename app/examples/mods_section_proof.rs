#![allow(deprecated)] // slint inline-import emits a deprecated fallback const in this proof target; no behavior change
//! Mods section UI proof with fixture data and REAL input events.
//! The Mods section lives in Settings/Profiles (active-tab=3, settings-section=0),
//! below the profile card, inside the outer settings ScrollView.
//! Proves at 1280x800 and 860x620:
//!  1. The section renders (non-background pixels) and is reachable by real wheel scroll.
//!  2. A real click on an installed-mod row selects it (mod-index changes).
//!  3. The inner mods list scrolls with real wheel events (later rows become clickable).
//!  4. On/Off, Update, Remove, Search and Install selected all dispatch their callbacks via real clicks (counters registered on each callback).
//!
//! Fixture data only; catalog/network behaviour of the core is proven elsewhere.
//! Captures: mods-proof/mods-section-{w}-full.bmp, -scrolled.bmp, -buttons.bmp.
use slint::{
    platform::{
        software_renderer::{MinimalSoftwareWindow, RepaintBufferType},
        Platform, PointerEventButton, WindowAdapter, WindowEvent,
    },
    ComponentHandle, LogicalPosition, ModelRc, Rgb8Pixel, SharedString, VecModel,
};
use std::cell::RefCell;
use std::rc::Rc;
slint::slint! { import { MainWindow } from "../ui/main.slint"; }
struct Offscreen(Rc<MinimalSoftwareWindow>);
impl Platform for Offscreen {
    fn create_window_adapter(&self) -> Result<Rc<dyn WindowAdapter>, slint::PlatformError> {
        Ok(self.0.clone())
    }
}
fn render(w: &MinimalSoftwareWindow, width: usize, height: usize) -> Vec<Rgb8Pixel> {
    w.request_redraw();
    let mut p = vec![Rgb8Pixel::default(); width * height];
    w.draw_if_needed(|r| {
        r.render(&mut p, width);
    });
    p
}
fn bmp(pixels: &[Rgb8Pixel], path: &str, width: usize, height: usize) {
    use std::io::Write;
    let stride = (width * 3 + 3) & !3;
    if let Some(d) = std::path::Path::new(path).parent() {
        std::fs::create_dir_all(d).unwrap();
    }
    let mut f = std::fs::File::create(path).unwrap();
    let mut h = vec![0u8; 54];
    h[0..2].copy_from_slice(b"BM");
    h[2..6].copy_from_slice(&((54 + stride * height) as u32).to_le_bytes());
    h[10..14].copy_from_slice(&54u32.to_le_bytes());
    h[14..18].copy_from_slice(&40u32.to_le_bytes());
    h[18..22].copy_from_slice(&(width as i32).to_le_bytes());
    h[22..26].copy_from_slice(&(height as i32).to_le_bytes());
    h[26..28].copy_from_slice(&1u16.to_le_bytes());
    h[28..30].copy_from_slice(&24u16.to_le_bytes());
    f.write_all(&h).unwrap();
    for y in (0..height).rev() {
        let mut row = vec![0u8; stride];
        for x in 0..width {
            let p = pixels[y * width + x];
            row[x * 3..x * 3 + 3].copy_from_slice(&[p.b, p.g, p.r]);
        }
        f.write_all(&row).unwrap();
    }
}
fn pump() {
    slint::platform::update_timers_and_animations();
}
fn wheel(ui: &MainWindow, x: f32, y: f32, dy: f32) {
    let w = ui.window();
    w.dispatch_event(WindowEvent::PointerScrolled {
        position: LogicalPosition::new(x, y),
        delta_x: 0.,
        delta_y: dy,
    });
    pump();
}
fn click(ui: &MainWindow, x: f32, y: f32) {
    let w = ui.window();
    let p = LogicalPosition::new(x, y);
    w.dispatch_event(WindowEvent::PointerMoved { position: p });
    w.dispatch_event(WindowEvent::PointerPressed {
        position: p,
        button: PointerEventButton::Left,
    });
    w.dispatch_event(WindowEvent::PointerReleased {
        position: p,
        button: PointerEventButton::Left,
    });
    pump();
}
// Accent (primary) button pixels, same detector family as earlier proofs.
fn accent(q: Rgb8Pixel) -> bool {
    q.r > 120 && q.r > q.g && q.g > q.b && q.b < 160 && q.g > 80
}
// Lowest tall, wide accent region below y_min: a filled primary button.
fn find_primary(p: &[Rgb8Pixel], width: usize, height: usize, y_min: usize) -> Option<(f32, f32)> {
    let mut rows: Vec<(usize, usize, usize)> = Vec::new();
    for y in y_min..height {
        let mut first = None;
        let mut last = 0usize;
        let mut count = 0usize;
        for x in 0..width {
            if accent(p[y * width + x]) {
                if first.is_none() {
                    first = Some(x);
                }
                last = x;
                count += 1;
            }
        }
        if let Some(a) = first {
            let span = last + 1 - a;
            if span > 300 && count * 5 >= span * 2 {
                rows.push((y, a, last));
            }
        }
    }
    let mut best: Option<(usize, usize, usize, usize)> = None;
    let mut cur: Option<(usize, usize, usize, usize)> = None;
    for (y, a, b) in rows {
        match cur {
            Some((y0, y1, ca, cb)) if y <= y1 + 1 => {
                cur = Some((y0, y, ca.min(a), cb.max(b)));
            }
            _ => {
                if let Some(g) = cur {
                    if g.1 - g.0 >= 8 {
                        best = Some(g);
                    }
                }
                cur = Some((y, y, a, b));
            }
        }
    }
    if let Some(g) = cur {
        if g.1 - g.0 >= 8 {
            best = Some(g);
        }
    }
    best.map(|(y0, y1, a, b)| ((a + b) as f32 / 2., (y0 + y1) as f32 / 2.))
}
// First mod-row visible in the window: click-scan candidate rows and return the
// first click that sets mod-index to a non-negative value.
fn scan_mod_row(ui: &MainWindow, width: usize, height: usize) -> Option<(f32, f32, i32)> {
    let mut y = 180f32;
    while y < height as f32 - 30. {
        click(ui, width as f32 * 0.55, y);
        let mi = ui.get_mod_index();
        if mi >= 0 {
            return Some((width as f32 * 0.55, y, mi));
        }
        y += 23.;
    }
    None
}
fn main() -> Result<(), slint::PlatformError> {
    let w = MinimalSoftwareWindow::new(RepaintBufferType::NewBuffer);
    slint::platform::set_platform(Box::new(Offscreen(w.clone()))).unwrap();
    for (width, height) in [(1280usize, 800usize), (860usize, 620usize)] {
        let ui = MainWindow::new()?;
        ui.set_reduce_animations(true);
        ui.set_active_tab(3);
        ui.set_settings_section(0);
        // Fixture inventory: 8 rows so the 140px list (46px rows) must scroll.
        let rows: Vec<SharedString> = [
            "Sodium",
            "Sodium Extra",
            "Lithium",
            "Iris Shaders",
            "Mod Menu",
            "Fabric API",
            "Entity Culling",
            "FerriteCore",
        ]
        .iter()
        .map(|s| SharedString::from(*s))
        .collect();
        let details: Vec<SharedString> = [
            "0.5.8 · enabled",
            "0.5.4 · enabled",
            "0.12.7 · enabled",
            "1.7.0 · disabled",
            "9.0.0 · enabled",
            "0.97.0 · enabled",
            "1.6.2 · disabled",
            "5.0.2 · enabled",
        ]
        .iter()
        .map(|s| SharedString::from(*s))
        .collect();
        ui.set_mod_supported(true);
        ui.set_mod_rows(ModelRc::new(VecModel::from(rows)));
        ui.set_mod_row_details(ModelRc::new(VecModel::from(details)));
        ui.set_mod_index(-1);
        ui.set_mod_status(SharedString::from("8 mods · 6 enabled"));
        ui.set_instance_names(ModelRc::new(VecModel::from(vec![
            SharedString::from("Default profile (phase 2)"),
            SharedString::from("ProofMain"),
        ])));
        ui.set_instance_index(1);
        ui.set_instance_name(SharedString::from("ProofMain"));
        // Callback counters: proof that the buttons dispatch.
        let toggles = Rc::new(RefCell::new(0u32));
        let updates = Rc::new(RefCell::new(0u32));
        let removes = Rc::new(RefCell::new(0u32));
        let searches = Rc::new(RefCell::new(0u32));
        let installs = Rc::new(RefCell::new(0u32));
        {
            let c = toggles.clone();
            ui.on_mod_toggle(move || *c.borrow_mut() += 1);
        }
        {
            let c = updates.clone();
            ui.on_mod_update(move || *c.borrow_mut() += 1);
        }
        {
            let c = removes.clone();
            ui.on_mod_remove(move || *c.borrow_mut() += 1);
        }
        {
            let c = searches.clone();
            ui.on_mods_search(move || *c.borrow_mut() += 1);
        }
        {
            let c = installs.clone();
            ui.on_mod_install(move || *c.borrow_mut() += 1);
        }
        eprintln!("phase: size {width}x{height}");
        ui.window()
            .set_size(slint::PhysicalSize::new(width as u32, height as u32));
        pump();
        ui.set_mod_index(-1);
        // 1+2: scroll the outer settings ScrollView until a mod row accepts a real click.
        // Inner lists that cannot scroll further let the wheel reach the outer view.
        let mut first = None;
        for step in 0..60 {
            if let Some(hit) = scan_mod_row(&ui, width, height) {
                first = Some(hit);
                println!("event proof {width}x{height}: mod row {} clicked at y={:.0} after {step} outer scroll steps",hit.2,hit.1);
                break;
            }
            wheel(&ui, width as f32 * 0.55, 300., -120.);
        }
        let Some((rx, ry, ridx)) = first else {
            panic!("Mods section must be reachable by scrolling at {width}x{height}")
        };
        // Render sanity: the section actually drew pixels (not a blank panel).
        let p = render(&w, width, height);
        let nonbg = p
            .iter()
            .filter(|q| q.r > 40 || q.g > 40 || q.b > 60)
            .count();
        assert!(
            nonbg > width * height / 20,
            "Mods section must render visible content at {width}x{height} (nonbg={nonbg})"
        );
        bmp(
            &p,
            &format!("mods-proof/mods-section-{width}-full.bmp"),
            width,
            height,
        );
        // 3: inner list scroll reveals later rows via real wheel events over the list.
        wheel(&ui, rx, ry, -120.);
        wheel(&ui, rx, ry, -120.);
        wheel(&ui, rx, ry, -120.);
        ui.set_mod_index(-1);
        let Some((_sx, _sy, sidx)) = scan_mod_row(&ui, width, height) else {
            panic!("mod rows must stay clickable after inner scroll at {width}x{height}")
        };
        assert!(sidx>ridx,"inner wheel scroll must reveal later mod rows at {width}x{height} (first {ridx} -> {sidx})");
        println!("event proof {width}x{height}: inner list scroll moved first clickable row {ridx} -> {sidx}");
        let p = render(&w, width, height);
        bmp(
            &p,
            &format!("mods-proof/mods-section-{width}-scrolled.bmp"),
            width,
            height,
        );
        // 4: results + query fixtures so Search and Install become enabled.
        let res: Vec<SharedString> = [
            "Sodium Extra",
            "Reese's Sodium Options",
            "Sodium Dynamic Lights",
        ]
        .iter()
        .map(|s| SharedString::from(*s))
        .collect();
        let resd: Vec<SharedString> = [
            "compatible · 1.20.4",
            "compatible · 1.20.4",
            "compatible · 1.20.4",
        ]
        .iter()
        .map(|s| SharedString::from(*s))
        .collect();
        ui.set_mod_results(ModelRc::new(VecModel::from(res)));
        ui.set_mod_result_details(ModelRc::new(VecModel::from(resd)));
        ui.set_mod_result_index(0);
        ui.set_mod_query(SharedString::from("sodium"));
        pump();
        // 4a: keep scrolling the outer view until the lowest primary button in view is
        // "Install selected" (a click on it increments the install counter; a click on
        // "Create profile" is a no-op here because manage-instance is not wired).
        let i0 = *installs.borrow();
        let mut ipos = None;
        for _step in 0..30 {
            let p = render(&w, width, height);
            if let Some((cx, cy)) = find_primary(&p, width, height, (height as f32 * 0.4) as usize)
            {
                click(&ui, cx, cy);
                if *installs.borrow() > i0 {
                    ipos = Some((cx, cy));
                    println!(
                        "event proof {width}x{height}: Install selected click at ({cx:.0},{cy:.0})"
                    );
                    break;
                }
            }
            wheel(&ui, width as f32 * 0.55, 300., -120.);
        }
        let Some((icx, icy)) = ipos else {
            panic!("Install selected must become clickable at {width}x{height}")
        };
        let p = render(&w, width, height);
        bmp(
            &p,
            &format!("mods-proof/mods-section-{width}-buttons.bmp"),
            width,
            height,
        );
        // 4b: the On/Off, Update, Remove and Search controls sit between the mods list
        // and the results box, above Install: scan that band for the remaining buttons.
        let (mut t0, mut u0, mut r0, mut s0) = (false, false, false, false);
        let (mut x0t, mut y0t, mut x0u, mut y0u, mut x0r, mut y0r, mut x0s, mut y0s) =
            (0., 0., 0., 0., 0., 0., 0., 0.);
        let mut y = (icy - 320.).max(180.);
        'band: while y < icy - 40. {
            let mut x = width as f32 * 0.40;
            while x < width as f32 * 0.95 {
                click(&ui, x, y);
                if !t0 && *toggles.borrow() > 0 {
                    t0 = true;
                    x0t = x;
                    y0t = y;
                }
                if !u0 && *updates.borrow() > 0 {
                    u0 = true;
                    x0u = x;
                    y0u = y;
                }
                if !r0 && *removes.borrow() > 0 {
                    r0 = true;
                    x0r = x;
                    y0r = y;
                }
                if !s0 && *searches.borrow() > 0 {
                    s0 = true;
                    x0s = x;
                    y0s = y;
                }
                if t0 && u0 && r0 && s0 {
                    break 'band;
                }
                x += 10.;
            }
            y += 8.;
        }
        assert!(t0, "On/Off must dispatch mod-toggle at {width}x{height}");
        assert!(u0, "Update must dispatch mod-update at {width}x{height}");
        assert!(r0, "Remove must dispatch mod-remove at {width}x{height}");
        assert!(s0, "Search must dispatch mods-search at {width}x{height}");
        println!("event proof {width}x{height}: On/Off ({x0t:.0},{y0t:.0}) Update ({x0u:.0},{y0u:.0}) Remove ({x0r:.0},{y0r:.0}) Search ({x0s:.0},{y0s:.0}) Install ({icx:.0},{icy:.0})");
    }
    println!("Mods section proof complete: renders + scrolls + all five actions dispatch at 1280x800 and 860x620, 6 captures.");
    Ok(())
}
