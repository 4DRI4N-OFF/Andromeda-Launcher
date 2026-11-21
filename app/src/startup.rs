//! Pure startup sequence. No network, account, window or filesystem side effects.
#[derive(Clone, Copy, Debug)]
pub struct Glyph {
    pub visible: bool,
    pub x: f64,
    pub y: f64,
    pub sx: f64,
    pub sy: f64,
}
#[derive(Clone, Debug)]
pub struct Frame {
    pub x: f64,
    pub y: f64,
    pub size: f64,
    pub glyphs: [Glyph; 9],
}
fn ease(x: f64) -> f64 {
    let t = x.clamp(0., 1.);
    t * t * t * (t * (t * 6. - 15.) + 10.)
}
pub const INTRO_SECONDS: f64 = 2.1;
pub fn frame(seconds: f64) -> Frame {
    let f = seconds.max(0.) * 60.;
    let q = ease(f / 94.);
    let xs = [
        566.5425816900001,
        672.9801216900001,
        781.0859616900002,
        884.0756816900002,
        977.7229216900002,
        1085.0502216900002,
        1222.5181416900002,
        1310.82682169,
        1408.5892016900002,
    ];
    let shift = -1120. * (1. - q);
    let glyphs = std::array::from_fn(|i| Glyph {
        visible: true,
        x: xs[i] + shift,
        y: 550.4080013399999,
        sx: 1.,
        sy: 1.,
    });
    Frame {
        x: 960. + (480.96902124999997 - 960.) * q,
        y: 540. + (508.59999932999995 - 540.) * q,
        size: 650. + (160.76932338 - 650.) * q,
        glyphs,
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn planet_moves_continuously_left_and_shrinks() {
        let mut prev = frame(0.);
        for f in 1..126 {
            let next = frame(f as f64 / 60.);
            assert!(next.x <= prev.x + 1e-8);
            assert!(next.size <= prev.size + 1e-8);
            prev = next;
        }
    }
    #[test]
    fn whole_train_has_fixed_spacing_and_sizes() {
        let end = frame(2.1);
        for f in 0..126 {
            let st = frame(f as f64 / 60.);
            let shift = st.glyphs[0].x - end.glyphs[0].x;
            for (g, e) in st.glyphs.iter().zip(end.glyphs.iter()) {
                assert!((g.x - e.x - shift).abs() < 1e-9);
                assert_eq!((g.sx, g.sy), (1., 1.));
                assert!(g.visible);
            }
        }
    }
}
