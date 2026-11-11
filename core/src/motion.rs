//! Analytic damped harmonic oscillator, in seconds. Retargets preserve velocity.
const OMEGA: f32 = 24.0;
const DAMPING: f32 = 0.64;
pub fn sample(from: f32, target: f32, velocity: f32, elapsed: f32) -> (f32, f32) {
    if elapsed >= 0.8 {
        return (target, 0.0);
    }
    let t = elapsed.max(0.0);
    let decay = DAMPING * OMEGA;
    let frequency = OMEGA * (1.0 - DAMPING * DAMPING).sqrt();
    let a = from - target;
    let b = (velocity + decay * a) / frequency;
    let (sin, cos) = (frequency * t).sin_cos();
    let envelope = (-decay * t).exp();
    let displacement = a * cos + b * sin;
    (
        target + envelope * displacement,
        envelope * ((-a * frequency * sin + b * frequency * cos) - decay * displacement),
    )
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn initial_position_velocity() {
        let (x, v) = sample(0.3, 1.0, -2.0, 0.0);
        assert!((x - 0.3).abs() < 1e-6);
        assert!((v + 2.0).abs() < 1e-5);
    }
    #[test]
    fn overshoot_and_settle() {
        assert!(sample(0.0, 1.0, 0.0, 0.17).0 > 1.0);
        assert_eq!(sample(0.0, 1.0, 0.0, 0.8), (1.0, 0.0));
    }
    #[test]
    fn retarget_has_continuous_position_and_velocity() {
        let (x, v) = sample(0.0, 1.0, 0.0, 0.07);
        let (next, speed) = sample(x, 0.0, v, 0.0);
        assert!((x - next).abs() < 1e-6);
        assert!((speed - v).abs() < 1e-5);
    }
    #[test]
    fn no_nan_at_small_and_large_elapsed() {
        for t in [-1.0, 0.001, 0.1, 0.5, 1000.0] {
            let (x, v) = sample(-0.8, 2.0, 8.0, t);
            assert!(x.is_finite() && v.is_finite());
        }
    }
}
