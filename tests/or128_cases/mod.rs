//! Deterministic synthetic cases shared by the OR-128 tests (and by the one-off tool that pinned the legacy payload hashes
//! on the pre-OR-128 revision). Uses only the public `lisa` API of both revisions.
use nanoscope::lisa::*;

pub fn image(w: usize, h: usize, bits: u8) -> Vec<u16> {
    let maxv = (1u32 << bits) - 1;
    let mut s = 4242u32;
    let mut rnd = move || {
        s = s.wrapping_mul(1664525).wrapping_add(1013904223);
        s >> 16
    };
    (0..w * h)
        .map(|i| {
            let (x, y) = (i % w, i / w);
            let amp = match (x * 4 / w, y * 2 / h) {
                (0, _) => 0,
                (1, _) => 3,
                (2, _) => 24,
                _ => maxv,
            };
            ((maxv / 2) as i64 + (rnd() as i64 % (amp as i64 + 1)) - amp as i64 / 2).clamp(0, maxv as i64) as u16
        })
        .collect()
}

pub fn cfa(entries: Vec<ColorFilter>, dim: (usize, usize)) -> CfaPattern {
    CfaPattern { width: dim.0, height: dim.1, entries, origin_x: 0, origin_y: 0 }
}

/// (name, samples, metadata, bit depth): Bayer planes (odd and even geometry, 12/14-bit), identity/Morton for
/// monochrome (8/10/16-bit) and for a non-Bayer CFA.
pub fn cases() -> Vec<(&'static str, Vec<u16>, ImageMetadata, u8)> {
    use ColorFilter::*;
    let rggb = || cfa(vec![Red, Green, Green, Blue], (2, 2));
    let mut v = Vec::new();
    v.push(("bayer_23x17_12", image(23, 17, 12), ImageMetadata::with_cfa(23, 17, SampleLayout::BayerCfa, rggb()), 12));
    v.push(("bayer_64x48_14", image(64, 48, 14), ImageMetadata::with_cfa(64, 48, SampleLayout::BayerCfa, rggb()), 14));
    v.push(("mono_19x14_8", image(19, 14, 8), ImageMetadata::without_cfa(19, 14, SampleLayout::Monochrome), 8));
    v.push(("mono_31x29_10", image(31, 29, 10), ImageMetadata::without_cfa(31, 29, SampleLayout::Monochrome), 10));
    v.push(("mono_19x14_16", image(19, 14, 16), ImageMetadata::without_cfa(19, 14, SampleLayout::Monochrome), 16));
    let xt = cfa(vec![Green, Red, Green, Blue, Green, Green, Green, Blue, Green, Red, Green, Green, Blue, Green, Red, Green, Green, Green], (6, 3));
    let xm = ImageMetadata::with_cfa(24, 12, SampleLayout::OtherCfa, xt);
    v.push(("othercfa_24x12_12", image(24, 12, 12), xm, 12));
    v.push(("mono_1x1_8", vec![77], ImageMetadata::without_cfa(1, 1, SampleLayout::Monochrome), 8));
    v
}
