//! Bounded physical-pixel resize path. No window, disk, account or network access.
use crate::startup_placement::Placement;
pub const SECONDS: f32 = 0.65;
pub fn ease(t: f32) -> f32 {
    let t = t.clamp(0., 1.) as f64;
    (t * t * t * (10. + t * (-15. + 6. * t))) as f32
}
#[derive(Clone, Copy, Debug)]
pub struct Resize {
    pub from: Placement,
    pub to: Placement,
}
impl Resize {
    pub fn at(self, t: f32) -> Placement {
        let q = ease(t);
        let blend = |a: u32, b: u32| (a as f32 + (b as f32 - a as f32) * q).round() as u32;
        let width = blend(self.from.width, self.to.width);
        let height = blend(self.from.height, self.to.height);
        // Interpolate doubled centre, then derive origin. Negative monitors supported.
        let center = |a: i32, aw: u32, b: i32, bw: u32| {
            let a = a as f64 + aw as f64 / 2.;
            let b = b as f64 + bw as f64 / 2.;
            a + (b - a) * q as f64
        };
        Placement {
            x: (center(self.from.x, self.from.width, self.to.x, self.to.width) - width as f64 / 2.)
                .round() as i32,
            y: (center(self.from.y, self.from.height, self.to.y, self.to.height)
                - height as f64 / 2.)
                .round() as i32,
            width,
            height,
        }
    }
}
/// Stalled UI timer: do not play a late catch-up burst of native resize calls.
pub fn should_snap(reduced: bool, explicit_fallback: bool, tick_gap: f32) -> bool {
    reduced || explicit_fallback || tick_gap > 0.1
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::startup_placement::{fit, fit_main};
    #[test]
    fn exact_ends() {
        let r = Resize {
            from: fit(0, 0, 1920, 1040).unwrap(),
            to: fit_main(0, 0, 1920, 1040).unwrap(),
        };
        assert_eq!(r.at(0.), r.from);
        assert_eq!(r.at(1.), r.to);
        assert_eq!(r.at(-1.), r.from);
        assert_eq!(r.at(2.), r.to);
    }
    #[test]
    fn bounded_monotone_centered() {
        for (l, t, r, b) in [
            (0, 0, 1920, 1040),
            (-2560, -300, 0, 1100),
            (0, 0, 3840, 2080),
            (0, 0, 1080, 1880),
        ] {
            let m = Resize {
                from: fit(l, t, r, b).unwrap(),
                to: fit_main(l, t, r, b).unwrap(),
            };
            let mut prev = m.from;
            for i in 0..=1000 {
                let p = m.at(i as f32 / 1000.);
                assert!(p.width >= prev.width && p.height >= prev.height);
                assert!(p.width <= m.to.width && p.height <= m.to.height);
                assert!(
                    p.x >= l && p.y >= t && p.x + p.width as i32 <= r && p.y + p.height as i32 <= b
                );
                assert!((p.x as f64 + p.width as f64 / 2. - (l + r) as f64 / 2.).abs() <= 0.51);
                assert!((p.y as f64 + p.height as f64 / 2. - (t + b) as f64 / 2.).abs() <= 0.51);
                prev = p;
            }
        }
    }
    #[test]
    fn no_rebound() {
        let mut last = 0.;
        for i in 0..=10000 {
            let q = ease(i as f32 / 10000.);
            assert!((0. ..=1.).contains(&q));
            assert!(q + 0.000001 >= last);
            last = q;
        }
    }
    #[test]
    fn fallback_and_reduced() {
        assert!(should_snap(true, false, 0.016));
        assert!(should_snap(false, true, 0.016));
        assert!(should_snap(false, false, 0.15));
        assert!(!should_snap(false, false, 0.016));
    }
}
