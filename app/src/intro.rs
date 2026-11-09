//! Original-pose Blockbench renders. Aspect ratio is preserved in the UI.
pub const FRAMES: &[&[u8]] = &[
    include_bytes!("../assets/intro/0000.qoi"),
    include_bytes!("../assets/intro/0001.qoi"),
    include_bytes!("../assets/intro/0002.qoi"),
    include_bytes!("../assets/intro/0003.qoi"),
    include_bytes!("../assets/intro/0004.qoi"),
    include_bytes!("../assets/intro/0005.qoi"),
    include_bytes!("../assets/intro/0006.qoi"),
    include_bytes!("../assets/intro/0007.qoi"),
    include_bytes!("../assets/intro/0008.qoi"),
    include_bytes!("../assets/intro/0009.qoi"),
    include_bytes!("../assets/intro/0010.qoi"),
    include_bytes!("../assets/intro/0011.qoi"),
    include_bytes!("../assets/intro/0012.qoi"),
    include_bytes!("../assets/intro/0013.qoi"),
    include_bytes!("../assets/intro/0014.qoi"),
    include_bytes!("../assets/intro/0015.qoi"),
    include_bytes!("../assets/intro/0016.qoi"),
    include_bytes!("../assets/intro/0017.qoi"),
    include_bytes!("../assets/intro/0018.qoi"),
    include_bytes!("../assets/intro/0019.qoi"),
    include_bytes!("../assets/intro/0020.qoi"),
    include_bytes!("../assets/intro/0021.qoi"),
    include_bytes!("../assets/intro/0022.qoi"),
    include_bytes!("../assets/intro/0023.qoi"),
    include_bytes!("../assets/intro/0024.qoi"),
    include_bytes!("../assets/intro/0025.qoi"),
    include_bytes!("../assets/intro/0026.qoi"),
    include_bytes!("../assets/intro/0027.qoi"),
    include_bytes!("../assets/intro/0028.qoi"),
    include_bytes!("../assets/intro/0029.qoi"),
    include_bytes!("../assets/intro/0030.qoi"),
    include_bytes!("../assets/intro/0031.qoi"),
    include_bytes!("../assets/intro/0032.qoi"),
    include_bytes!("../assets/intro/0033.qoi"),
    include_bytes!("../assets/intro/0034.qoi"),
    include_bytes!("../assets/intro/0035.qoi"),
    include_bytes!("../assets/intro/0036.qoi"),
    include_bytes!("../assets/intro/0037.qoi"),
    include_bytes!("../assets/intro/0038.qoi"),
    include_bytes!("../assets/intro/0039.qoi"),
    include_bytes!("../assets/intro/0040.qoi"),
    include_bytes!("../assets/intro/0041.qoi"),
    include_bytes!("../assets/intro/0042.qoi"),
    include_bytes!("../assets/intro/0043.qoi"),
    include_bytes!("../assets/intro/0044.qoi"),
    include_bytes!("../assets/intro/0045.qoi"),
    include_bytes!("../assets/intro/0046.qoi"),
    include_bytes!("../assets/intro/0047.qoi"),
    include_bytes!("../assets/intro/0048.qoi"),
    include_bytes!("../assets/intro/0049.qoi"),
    include_bytes!("../assets/intro/0050.qoi"),
    include_bytes!("../assets/intro/0051.qoi"),
    include_bytes!("../assets/intro/0052.qoi"),
    include_bytes!("../assets/intro/0053.qoi"),
    include_bytes!("../assets/intro/0054.qoi"),
    include_bytes!("../assets/intro/0055.qoi"),
    include_bytes!("../assets/intro/0056.qoi"),
    include_bytes!("../assets/intro/0057.qoi"),
    include_bytes!("../assets/intro/0058.qoi"),
    include_bytes!("../assets/intro/0059.qoi"),
    include_bytes!("../assets/intro/0060.qoi"),
    include_bytes!("../assets/intro/0061.qoi"),
    include_bytes!("../assets/intro/0062.qoi"),
    include_bytes!("../assets/intro/0063.qoi"),
    include_bytes!("../assets/intro/0064.qoi"),
    include_bytes!("../assets/intro/0065.qoi"),
    include_bytes!("../assets/intro/0066.qoi"),
    include_bytes!("../assets/intro/0067.qoi"),
    include_bytes!("../assets/intro/0068.qoi"),
    include_bytes!("../assets/intro/0069.qoi"),
    include_bytes!("../assets/intro/0070.qoi"),
    include_bytes!("../assets/intro/0071.qoi"),
    include_bytes!("../assets/intro/0072.qoi"),
    include_bytes!("../assets/intro/0073.qoi"),
    include_bytes!("../assets/intro/0074.qoi"),
    include_bytes!("../assets/intro/0075.qoi"),
    include_bytes!("../assets/intro/0076.qoi"),
    include_bytes!("../assets/intro/0077.qoi"),
    include_bytes!("../assets/intro/0078.qoi"),
    include_bytes!("../assets/intro/0079.qoi"),
    include_bytes!("../assets/intro/0080.qoi"),
    include_bytes!("../assets/intro/0081.qoi"),
    include_bytes!("../assets/intro/0082.qoi"),
    include_bytes!("../assets/intro/0083.qoi"),
    include_bytes!("../assets/intro/0084.qoi"),
    include_bytes!("../assets/intro/0085.qoi"),
    include_bytes!("../assets/intro/0086.qoi"),
    include_bytes!("../assets/intro/0087.qoi"),
    include_bytes!("../assets/intro/0088.qoi"),
    include_bytes!("../assets/intro/0089.qoi"),
    include_bytes!("../assets/intro/0090.qoi"),
    include_bytes!("../assets/intro/0091.qoi"),
    include_bytes!("../assets/intro/0092.qoi"),
    include_bytes!("../assets/intro/0093.qoi"),
    include_bytes!("../assets/intro/0094.qoi"),
    include_bytes!("../assets/intro/0095.qoi"),
    include_bytes!("../assets/intro/0096.qoi"),
    include_bytes!("../assets/intro/0097.qoi"),
    include_bytes!("../assets/intro/0098.qoi"),
    include_bytes!("../assets/intro/0099.qoi"),
    include_bytes!("../assets/intro/0100.qoi"),
    include_bytes!("../assets/intro/0101.qoi"),
    include_bytes!("../assets/intro/0102.qoi"),
    include_bytes!("../assets/intro/0103.qoi"),
    include_bytes!("../assets/intro/0104.qoi"),
    include_bytes!("../assets/intro/0105.qoi"),
    include_bytes!("../assets/intro/0106.qoi"),
    include_bytes!("../assets/intro/0107.qoi"),
    include_bytes!("../assets/intro/0108.qoi"),
    include_bytes!("../assets/intro/0109.qoi"),
    include_bytes!("../assets/intro/0110.qoi"),
    include_bytes!("../assets/intro/0111.qoi"),
    include_bytes!("../assets/intro/0112.qoi"),
    include_bytes!("../assets/intro/0113.qoi"),
    include_bytes!("../assets/intro/0114.qoi"),
    include_bytes!("../assets/intro/0115.qoi"),
    include_bytes!("../assets/intro/0116.qoi"),
    include_bytes!("../assets/intro/0117.qoi"),
    include_bytes!("../assets/intro/0118.qoi"),
    include_bytes!("../assets/intro/0119.qoi"),
    include_bytes!("../assets/intro/0120.qoi"),
    include_bytes!("../assets/intro/0121.qoi"),
    include_bytes!("../assets/intro/0122.qoi"),
    include_bytes!("../assets/intro/0123.qoi"),
    include_bytes!("../assets/intro/0124.qoi"),
    include_bytes!("../assets/intro/0125.qoi"),
    include_bytes!("../assets/intro/0126.qoi"),
    include_bytes!("../assets/intro/0127.qoi"),
    include_bytes!("../assets/intro/0128.qoi"),
    include_bytes!("../assets/intro/0129.qoi"),
    include_bytes!("../assets/intro/0130.qoi"),
    include_bytes!("../assets/intro/0131.qoi"),
    include_bytes!("../assets/intro/0132.qoi"),
    include_bytes!("../assets/intro/0133.qoi"),
    include_bytes!("../assets/intro/0134.qoi"),
    include_bytes!("../assets/intro/0135.qoi"),
    include_bytes!("../assets/intro/0136.qoi"),
    include_bytes!("../assets/intro/0137.qoi"),
    include_bytes!("../assets/intro/0138.qoi"),
    include_bytes!("../assets/intro/0139.qoi"),
    include_bytes!("../assets/intro/0140.qoi"),
    include_bytes!("../assets/intro/0141.qoi"),
    include_bytes!("../assets/intro/0142.qoi"),
    include_bytes!("../assets/intro/0143.qoi"),
    include_bytes!("../assets/intro/0144.qoi"),
    include_bytes!("../assets/intro/0145.qoi"),
    include_bytes!("../assets/intro/0146.qoi"),
    include_bytes!("../assets/intro/0147.qoi"),
    include_bytes!("../assets/intro/0148.qoi"),
    include_bytes!("../assets/intro/0149.qoi"),
    include_bytes!("../assets/intro/0150.qoi"),
    include_bytes!("../assets/intro/0151.qoi"),
    include_bytes!("../assets/intro/0152.qoi"),
    include_bytes!("../assets/intro/0153.qoi"),
    include_bytes!("../assets/intro/0154.qoi"),
    include_bytes!("../assets/intro/0155.qoi"),
    include_bytes!("../assets/intro/0156.qoi"),
    include_bytes!("../assets/intro/0157.qoi"),
    include_bytes!("../assets/intro/0158.qoi"),
    include_bytes!("../assets/intro/0159.qoi"),
    include_bytes!("../assets/intro/0160.qoi"),
    include_bytes!("../assets/intro/0161.qoi"),
    include_bytes!("../assets/intro/0162.qoi"),
    include_bytes!("../assets/intro/0163.qoi"),
    include_bytes!("../assets/intro/0164.qoi"),
    include_bytes!("../assets/intro/0165.qoi"),
    include_bytes!("../assets/intro/0166.qoi"),
    include_bytes!("../assets/intro/0167.qoi"),
    include_bytes!("../assets/intro/0168.qoi"),
    include_bytes!("../assets/intro/0169.qoi"),
    include_bytes!("../assets/intro/0170.qoi"),
    include_bytes!("../assets/intro/0171.qoi"),
    include_bytes!("../assets/intro/0172.qoi"),
    include_bytes!("../assets/intro/0173.qoi"),
    include_bytes!("../assets/intro/0174.qoi"),
    include_bytes!("../assets/intro/0175.qoi"),
    include_bytes!("../assets/intro/0176.qoi"),
    include_bytes!("../assets/intro/0177.qoi"),
    include_bytes!("../assets/intro/0178.qoi"),
    include_bytes!("../assets/intro/0179.qoi"),
    include_bytes!("../assets/intro/0180.qoi"),
];
pub fn should_play(reduce_animations: bool) -> bool {
    !reduce_animations
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn reduced_motion_skips_intro() {
        assert!(!should_play(true));
        assert!(should_play(false));
    }
    #[test]
    fn all_frames_are_embedded_qoi() {
        assert_eq!(FRAMES.len(), 181);
        assert!(FRAMES.iter().all(|f| f.starts_with(b"qoif")));
    }
}

#[cfg(test)]
mod decode_tests {
    #[test]
    fn every_frame_decodes_with_original_aspect_ratio() {
        let mut times = Vec::new();
        for frame in super::FRAMES {
            let start = std::time::Instant::now();
            let image = super::decode(frame).expect("valid intro image");
            assert_eq!((image.size().width, image.size().height), (960, 540));
            times.push(start.elapsed());
        }
        times.sort();
        println!(
            "QOI decode: p50={:?}, p95={:?}, max={:?}",
            times[90], times[171], times[180]
        );
    }
}

pub fn decode(data: &[u8]) -> Result<slint::Image, qoi::Error> {
    let mut decoder = qoi::Decoder::new(data)?.with_channels(qoi::Channels::Rgba);
    let width = decoder.header().width;
    let height = decoder.header().height;
    let mut buffer = slint::SharedPixelBuffer::<slint::Rgba8Pixel>::new(width, height);
    decoder.decode_to_buf(buffer.make_mut_bytes())?;
    Ok(slint::Image::from_rgba8(buffer))
}
