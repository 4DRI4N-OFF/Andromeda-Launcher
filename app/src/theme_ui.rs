//! Theme adapter shared by the application and the offscreen UI proof.
use crate::{MainWindow, ThemeChoice, ThemePalette};
use andromeda_core::themes::{rgba, spring_scale, ThemePack};
use slint::ComponentHandle;
use std::{
    rc::Rc,
    time::{Duration, Instant},
};
fn color(hex: &str) -> slint::Color {
    let [r, g, b, a] = rgba(hex).expect("validated theme color");
    slint::Color::from_argb_u8(a, r, g, b)
}
pub fn choices(packs: &[ThemePack]) -> slint::ModelRc<ThemeChoice> {
    slint::ModelRc::new(slint::VecModel::from(
        packs
            .iter()
            .map(|p| ThemeChoice {
                id: p.id.clone().into(),
                name: p.name.clone().into(),
                description: p.description.clone().into(),
                background: color(&p.colors.background),
                background_end: color(&p.colors.background_end),
                accent: color(&p.colors.accent),
                accent_end: color(&p.colors.accent_end),
                text_color: color(&p.colors.text),
                display_font: p.fonts.display.clone().into(),
            })
            .collect::<Vec<_>>(),
    ))
}
fn blend(a: slint::Color, b: slint::Color, t: f32) -> slint::Color {
    let mix = |a: u8, b: u8| (a as f32 + (b as f32 - a as f32) * t).round() as u8;
    slint::Color::from_argb_u8(
        mix(a.alpha(), b.alpha()),
        mix(a.red(), b.red()),
        mix(a.green(), b.green()),
        mix(a.blue(), b.blue()),
    )
}
pub fn apply(ui: &MainWindow, pack: &ThemePack, animate: bool, timer: &Rc<slint::Timer>) {
    timer.stop();
    let reduced = ui.get_reduce_animations() || !animate;
    let p = ui.global::<ThemePalette>();
    let from = [
        p.get_bg(),
        p.get_bg_end(),
        p.get_panel(),
        p.get_panel_light(),
        p.get_border(),
        p.get_text_color(),
        p.get_muted(),
        p.get_accent(),
        p.get_accent_end(),
        p.get_nav_accent(),
        p.get_list_accent(),
        p.get_on_accent(),
        p.get_field(),
        p.get_danger(),
    ];
    let c = &pack.colors;
    let to = [
        &c.background,
        &c.background_end,
        &c.panel,
        &c.panel_light,
        &c.border,
        &c.text,
        &c.muted,
        &c.accent,
        &c.accent_end,
        &c.accent,
        &c.accent,
        &c.on_accent,
        &c.field,
        &c.danger,
    ]
    .map(|v| color(v));
    p.set_reduce_motion(reduced);
    p.set_selection_scale(1.0);
    p.set_body_font(pack.fonts.body.clone().into());
    p.set_value_font(pack.fonts.values.clone().into());
    p.set_background_style(pack.background.clone().into());
    ui.set_selected_theme(pack.id.clone().into());
    ui.set_dark_mode(pack.dark);
    let font: slint::SharedString = pack.fonts.display.clone().into();
    if reduced {
        set_colors(&p, &to);
        p.set_display_font(font);
        p.set_display_opacity(1.0);
        p.set_reduce_motion(ui.get_reduce_animations());
        return;
    }
    let weak = ui.as_weak();
    let start = Instant::now();
    let timer_weak = Rc::downgrade(timer);
    timer.start(
        slint::TimerMode::Repeated,
        Duration::from_millis(16),
        move || {
            let Some(ui) = weak.upgrade() else {
                if let Some(t) = timer_weak.upgrade() {
                    t.stop();
                }
                return;
            };
            let p = ui.global::<ThemePalette>();
            let t = start.elapsed().as_secs_f32();
            let reduced = ui.get_reduce_animations();
            let mut colors = to;
            for (i, output) in colors.iter_mut().enumerate() {
                let delay = match i {
                    9 => 0.0,
                    10 => 0.04,
                    7 | 8 => 0.08,
                    _ => 0.0,
                };
                let progress = if reduced {
                    1.0
                } else {
                    ((t - delay) / 0.4).clamp(0.0, 1.0)
                };
                *output = blend(from[i], to[i], progress * progress * (3.0 - 2.0 * progress));
            }
            set_colors(&p, &colors);
            p.set_selection_scale(spring_scale(t, reduced));
            let opacity = if reduced {
                1.0
            } else if t < 0.10 {
                1.0 - t * 7.0
            } else {
                (0.3 + (t - 0.10) * 7.0).min(1.0)
            };
            if reduced || t >= 0.10 {
                p.set_display_font(font.clone());
            }
            p.set_display_opacity(opacity);
            if reduced || t >= 0.5 {
                set_colors(&p, &to);
                p.set_selection_scale(1.0);
                p.set_display_opacity(1.0);
                if let Some(timer) = timer_weak.upgrade() {
                    timer.stop();
                }
            }
        },
    );
}
fn set_colors(p: &ThemePalette<'_>, c: &[slint::Color; 14]) {
    p.set_bg(c[0]);
    p.set_bg_end(c[1]);
    p.set_panel(c[2]);
    p.set_panel_light(c[3]);
    p.set_border(c[4]);
    p.set_text_color(c[5]);
    p.set_muted(c[6]);
    p.set_accent(c[7]);
    p.set_accent_end(c[8]);
    p.set_nav_accent(c[9]);
    p.set_list_accent(c[10]);
    p.set_on_accent(c[11]);
    p.set_field(c[12]);
    p.set_danger(c[13]);
}

/// Save only appearance choices through the same Settings schema as startup.
pub fn save_selection(ui: &MainWindow, path: &std::path::Path) -> std::io::Result<()> {
    let mut settings = andromeda_core::settings::Settings::load(path);
    settings.theme_id = ui.get_selected_theme().to_string();
    settings.dark_mode = ui.get_dark_mode();
    settings.reduce_animations = ui.get_reduce_animations();
    settings.save(path)
}
