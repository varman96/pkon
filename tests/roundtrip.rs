use nanoscope::canon::{first_mismatch, Canonical, Layout, Mismatch, Spec};
use nanoscope::codec::{Codec, CodecError, PngCodec, RawCodec};
use nanoscope::corpus;
use nanoscope::ext::Ffmpeg;
use nanoscope::verify::{round_trip, truncation_check, Verdict};
use std::sync::Mutex;

fn img(w: u32, h: u32, layout: Layout, depth: u8, f: impl FnMut(usize) -> u8) -> Canonical {
    let spec = Spec { width: w, height: h, layout, bit_depth: depth };
    Canonical::new(spec, (0..spec.byte_len()).map(f).collect()).unwrap()
}

/// Small hand-made fixtures covering the cases the brief calls out.
fn fixtures() -> Vec<(&'static str, Canonical)> {
    let mut x = 0x1234_5678u32;
    let mut rnd = move |_| {
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        x as u8
    };
    let mut hidden = img(5, 4, Layout::Rgba, 8, |i| (i * 37) as u8 | 1);
    for px in hidden.data.chunks_mut(4).step_by(2) {
        px[3] = 0; // fully transparent, RGB left non-trivial
    }
    vec![
        ("1x1 rgb", img(1, 1, Layout::Rgb, 8, |i| 10 + i as u8)),
        ("3x5 rgba", img(3, 5, Layout::Rgba, 8, |i| (i * 11) as u8)),
        ("7x3 gray", img(7, 3, Layout::Gray, 8, |i| (i * 9) as u8)),
        ("graya", img(6, 6, Layout::GrayAlpha, 8, |i| (i * 5) as u8)),
        ("odd 13x11 rgb", img(13, 11, Layout::Rgb, 8, |i| (i % 251) as u8)),
        ("constant", img(16, 16, Layout::Rgb, 8, |_| 77)),
        ("gradient", img(32, 8, Layout::Rgb, 8, |i| (i / 3 % 32 * 8) as u8)),
        ("noise", img(20, 20, Layout::Rgb, 8, &mut rnd)),
        ("hidden-rgb-under-alpha0", hidden),
        ("rgb16", img(5, 5, Layout::Rgb, 16, |i| (i * 13) as u8)),
        ("gray16", img(9, 2, Layout::Gray, 16, |i| (i * 29) as u8)),
        ("rgba16", img(4, 4, Layout::Rgba, 16, |i| (i * 7) as u8)),
    ]
}

fn assert_exact(codec: &dyn Codec, name: &str, c: &Canonical) {
    eprintln!("  {} <- {name}", codec.name());
    match round_trip(codec, c, 1) {
        Ok(t) => {
            if let Verdict::Mismatch(m) = &t.verdict {
                panic!("{} on {name}: {m}", codec.name());
            }
            truncation_check(codec, c, &t.encoded).unwrap_or_else(|e| panic!("{} on {name}: {e}", codec.name()));
        }
        Err(e) => panic!("{} on {name}: {e}", codec.name()),
    }
}

#[test]
fn in_process_codecs_roundtrip_all_fixtures() {
    for codec in [&RawCodec as &dyn Codec, &PngCodec] {
        for (name, c) in fixtures() {
            assert_exact(codec, name, &c);
        }
    }
}

#[test]
fn in_process_codecs_roundtrip_synthetic_corpus() {
    for e in corpus::synthetic() {
        assert_exact(&RawCodec, &e.name, &e.image);
        assert_exact(&PngCodec, &e.name, &e.image);
    }
}

/// Lossy-on-purpose codec: zeroes RGB under alpha=0, as many encoders do.
struct ZapsHidden;
impl Codec for ZapsHidden {
    fn name(&self) -> String { "zaps-hidden".into() }
    fn version(&self) -> String { "t".into() }
    fn config(&self) -> String { "t".into() }
    fn encode(&self, i: &Canonical) -> Result<Vec<u8>, CodecError> {
        let mut c = i.clone();
        for px in c.data.chunks_mut(4) {
            if px[3] == 0 { px[..3].fill(0); }
        }
        RawCodec.encode(&c)
    }
    fn decode(&self, e: &[u8], h: &Spec) -> Result<Canonical, CodecError> { RawCodec.decode(e, h) }
}

#[test]
fn mismatch_is_detected_and_located() {
    let (_, c) = fixtures().into_iter().find(|(n, _)| n.starts_with("hidden")).unwrap();
    let t = round_trip(&ZapsHidden, &c, 1).ok().unwrap();
    let Verdict::Mismatch(m) = t.verdict else { panic!("lossy codec passed as lossless") };
    // pixel 0 has alpha 0 (step_by(2) starts at 0); its first RGB sample is nonzero.
    match m {
        Mismatch::Sample { x, y, channel, expected, actual, .. } => {
            assert_eq!((x, y, channel, actual), (0, 0, "R", 0));
            assert_eq!(expected, c.sample(0, 0, 0));
        }
        other => panic!("wrong mismatch kind: {other}"),
    }
}

#[test]
fn single_low_bit_16bit_difference_is_found() {
    let a = img(4, 4, Layout::Gray, 16, |i| i as u8);
    let mut b = a.clone();
    b.data[2 * 9] ^= 1; // pixel 9 = (x=1,y=2)
    match first_mismatch(&a, &b) {
        Some(Mismatch::Sample { x: 1, y: 2, channel: "Y", differing_samples: 1, .. }) => {}
        other => panic!("{other:?}"),
    }
}

/// A decoder that ignores its input and returns a stashed copy of the original.
struct Cheat(Mutex<Option<Canonical>>);
impl Codec for Cheat {
    fn name(&self) -> String { "cheat".into() }
    fn version(&self) -> String { "t".into() }
    fn config(&self) -> String { "t".into() }
    fn encode(&self, i: &Canonical) -> Result<Vec<u8>, CodecError> {
        *self.0.lock().unwrap() = Some(i.clone());
        Ok(vec![0; 4])
    }
    fn decode(&self, _: &[u8], _: &Spec) -> Result<Canonical, CodecError> {
        Ok(self.0.lock().unwrap().clone().unwrap())
    }
}

#[test]
fn cheating_codec_passes_roundtrip_but_fails_truncation_check() {
    let c = &fixtures()[1].1;
    let cheat = Cheat(Mutex::new(None));
    let t = round_trip(&cheat, c, 1).ok().unwrap();
    assert!(matches!(t.verdict, Verdict::Exact), "cheat looks exact to a plain compare");
    assert!(truncation_check(&cheat, c, &t.encoded).is_err(), "truncation check must expose it");
}

#[test]
fn malformed_and_truncated_input_errors_without_panicking() {
    let c = &fixtures()[1].1;
    let hint = c.spec;
    for codec in [&RawCodec as &dyn Codec, &PngCodec] {
        let good = codec.encode(c).unwrap();
        assert!(codec.decode(&[], &hint).is_err(), "{}: empty", codec.name());
        assert!(codec.decode(b"not an image at all", &hint).is_err(), "{}: garbage", codec.name());
        for cut in [1, good.len() / 2] {
            // cutting real payload must not silently return the original (a cut of only the PNG IEND CRC legitimately can)
            if let Ok(d) = codec.decode(&good[..cut], &hint) {
                assert!(first_mismatch(c, &d).is_some(), "{}: truncated at {cut} decoded to original", codec.name());
            }
        }
    }
    // Hostile header: claims a huge image; must be rejected before allocating.
    let mut evil = vec![0xff, 0xff, 0xff, 0x7f, 0xff, 0xff, 0xff, 0x7f, 0x14, 8];
    evil.extend([0u8; 16]);
    assert!(RawCodec.decode(&evil, &hint).is_err());
}

#[test]
fn canonical_layout_is_little_endian_row_major() {
    let c = img(2, 1, Layout::Rgb, 16, |i| i as u8); // bytes 0..12
    assert_eq!(c.sample(0, 0, 0), 0x0100);
    assert_eq!(c.sample(1, 0, 2), 0x0b0a);
}

#[test]
fn synthetic_corpus_is_deterministic_with_pinned_hashes() {
    let (a, b) = (corpus::synthetic(), corpus::synthetic());
    assert_eq!(a.len(), b.len());
    for (x, y) in a.iter().zip(&b) {
        assert_eq!(x.image.content_hash(), y.image.content_hash(), "{} not deterministic", x.name);
    }
    let names: std::collections::HashSet<_> = a.iter().map(|e| &e.name).collect();
    assert_eq!(names.len(), a.len(), "duplicate fixture names");
    // Pinned: a changed hash means the corpus changed, which invalidates old results.
    let h = |n: &str| a.iter().find(|e| e.name == n).unwrap().image.content_hash();
    assert_eq!(&h("flat_rgb")[..16], "dd7c220a045ecaf1");
    assert_eq!(&h("noise_rgb")[..16], "36457a6ca6d748f4");
}

#[test]
fn external_png_loader_matches_encoder_input() {
    let dir = std::env::temp_dir().join(format!("nanoscope-test-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    for (name, c) in fixtures().into_iter().filter(|(n, _)| ["3x5 rgba", "rgba16", "gray16", "graya", "7x3 gray"].contains(n)) {
        let p = dir.join(format!("{}.png", name.replace(' ', "_")));
        std::fs::write(&p, PngCodec.encode(&c).unwrap()).unwrap();
        let loaded = Canonical::load(&p).unwrap();
        assert!(first_mismatch(&c, &loaded).is_none(), "{name}");
    }
    let (entries, skipped) = corpus::from_dir(&dir).unwrap();
    assert_eq!(entries.len(), 5);
    assert!(skipped.is_empty());
    let _ = std::fs::remove_dir_all(&dir);
}

// ---- External baselines: run only if the tool is present, and say so loudly if not. ----

#[test]
fn ffmpeg_jxl_lossless_invocation_is_really_lossless() {
    for effort in [1u8, 7] {
        let jxl = Ffmpeg::jxl(effort);
        if let Err(why) = jxl.available() {
            eprintln!("SKIPPED (NOT verified): {} - {why}", jxl.name());
            return;
        }
        for (name, c) in fixtures() {
            assert_exact(&jxl, name, &c); // includes hidden RGB under alpha=0 and 16-bit
        }
        for e in corpus::synthetic() {
            assert_exact(&jxl, &e.name, &e.image);
        }
    }
}

#[test]
fn ffmpeg_webp_lossless_on_opaque_rgb() {
    let webp = Ffmpeg::webp(6);
    if let Err(why) = webp.available() {
        eprintln!("SKIPPED (NOT verified): {} - {why}", webp.name());
        return;
    }
    // Alpha with hidden RGB is deliberately NOT asserted: whether libwebp keeps RGB under
    // alpha=0 is a finding for the bench to report, not a harness invariant.
    for (name, c) in fixtures().into_iter().filter(|(_, c)| c.spec.layout == Layout::Rgb && c.spec.bit_depth == 8) {
        assert_exact(&webp, name, &c);
    }
    assert!(matches!(webp.encode(&fixtures()[2].1), Err(CodecError::Unsupported(_))), "gray must be reported unsupported");
}
