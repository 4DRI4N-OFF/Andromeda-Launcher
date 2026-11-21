//! Static backdrop sampled through a temporary occlusion mask. Moving logo stays native Paths.
use slint::{Image, Rgba8Pixel, SharedPixelBuffer};
const BG: &[u8] = include_bytes!("../assets/startup/backdrop.rgb");
const ALPHA: &[u8] = include_bytes!("../assets/startup/outer-alpha.bin");
const VIVID: &[u8] = include_bytes!("../assets/startup/ambience.rgb");
pub fn cover(cx: f64, cy: f64, size: f64, width: u32, height: u32) -> Image {
    cover_ambience(cx, cy, size, width, height, 0.)
}
pub fn cover_ambience(
    cx: f64,
    cy: f64,
    size: f64,
    width: u32,
    height: u32,
    ambience: f32,
) -> Image {
    let mut out = SharedPixelBuffer::<Rgba8Pixel>::new(width, height);
    let scale = width as f64 / 1920.;
    for (i, p) in out.make_mut_slice().iter_mut().enumerate() {
        let x = (i % width as usize) as f64 + 0.5;
        let y = (i / width as usize) as f64 + 0.5;
        let gx = x / scale;
        let gy = y / scale;
        let alpha = if gx < cx {
            255
        } else {
            let u = (gx - cx + size / 2.) / size * 1000.;
            let v = (gy - cy + size / 2.) / size * 1000.;
            if u < 0. || v < 0. || u >= 1000. || v >= 1000. {
                0
            } else {
                ALPHA[v as usize * 1000 + u as usize]
            }
        };
        let ix = (gx as usize).min(1919);
        let iy = (gy as usize).min(1079);
        let b = (iy * 1920 + ix) * 3;
        let blend = |j: usize| {
            (BG[j] as f32 + (VIVID[j] as f32 - BG[j] as f32) * ambience.clamp(0., 1.)).round() as u8
        };
        *p = Rgba8Pixel::new(blend(b), blend(b + 1), blend(b + 2), alpha);
    }
    Image::from_rgba8(out)
}
