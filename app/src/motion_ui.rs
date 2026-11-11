//! One motion clock for all controls. Dropping the owner stops the timer.
use crate::{InteractionMotion, MainWindow};
use slint::ComponentHandle;
use std::{
    rc::Rc,
    time::{Duration, Instant},
};
pub fn install(ui: &MainWindow) -> Rc<slint::Timer> {
    let motion = ui.global::<InteractionMotion>();
    motion.on_sample(|from, target, velocity, t| {
        andromeda_core::motion::sample(from, target, velocity, t).0
    });
    motion.on_velocity(|from, target, velocity, t| {
        andromeda_core::motion::sample(from, target, velocity, t).1
    });
    motion.set_reduced(ui.get_reduce_animations());
    let timer = Rc::new(slint::Timer::default());
    let weak = ui.as_weak();
    let start = Instant::now();
    timer.start(
        slint::TimerMode::Repeated,
        Duration::from_millis(16),
        move || {
            if let Some(ui) = weak.upgrade() {
                let m = ui.global::<InteractionMotion>();
                m.set_reduced(ui.get_reduce_animations());
                m.set_clock(start.elapsed().as_secs_f32());
            }
        },
    );
    timer
}
