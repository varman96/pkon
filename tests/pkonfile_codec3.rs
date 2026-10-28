//! Codec 3 (REL6 joint sign context): exact round trips, header acceptance, equality with the independent REL6 research arm, and
//! that codecs 1 and 2 are untouched by its existence.
use nanoscope::canon::{Canonical, Layout, Spec};
use nanoscope::pkonfile::*;
use nanoscope::pkonlearn::prepared;
use nanoscope::pkonrel::{encode_rel, J_ARMS};

fn img(w: u32, h: u32, layout: Layout, seed: u32) -> Canonical {
    let mut x = seed;
    let n = w as usize * h as usize * layout.channels();
    let data = (0..n)
        .map(|i| {
            x = x.wrapping_mul(1664525).wrapping_add(1013904223);
            ((i / layout.channels()) as u8).wrapping_add((x >> 24) as u8 & 15)
        })
        .collect();
    Canonical::new(Spec { width: w, height: h, layout, bit_depth: 8 }, data).unwrap()
}

#[test]
fn codec3_round_trips_and_is_accepted_by_the_production_decoder() {
    for layout in [Layout::Gray, Layout::GrayAlpha, Layout::Rgb, Layout::Rgba] {
        for &(w, h) in &[(1u32, 1u32), (1, 7), (9, 1), (3, 3), (17, 13), (64, 33)] {
            let i = img(w, h, layout, w * 31 + h);
            let f = encode_with_codec(&i, CODEC_X4PJ).unwrap();
            assert_eq!(f[6], CODEC_X4PJ);
            assert_eq!(parse_header(&f).unwrap().codec, CODEC_X4PJ);
            let d = decode(&f).unwrap();
            assert_eq!((d.spec, d.data), (i.spec, i.data.clone()), "{w}x{h} {layout:?}");
            // codecs 1 and 2 keep their own ids and still round-trip
            for c in [CODEC_X4, CODEC_X4P] {
                let g = encode_with_codec(&i, c).unwrap();
                assert_eq!(g[6], c);
                assert_eq!(decode(&g).unwrap().data, i.data);
            }
            assert_eq!(encode(&i).unwrap(), f, "default encode must equal explicit codec 3");
            assert_eq!(encode(&i).unwrap()[6], CODEC_X4PJ, "default must be codec 3");
            assert_eq!(CODEC_CURRENT, CODEC_X4PJ);
            // a patched codec byte must not decode to the right pixels
            // (tiny images can code to identical streams under both codecs, so only check where the payloads differ)
            if encode_payload_codec(&i, CODEC_X4P) != encode_payload_codec(&i, CODEC_X4PJ) {
                let mut p = f.clone();
                p[6] = CODEC_X4P;
                assert!(decode(&p).is_err(), "{w}x{h} {layout:?}: codec-3 stream read as codec 2 was accepted");
            }
        }
    }
}

#[test]
fn codec3_payload_equals_independent_research_arm_c() {
    for &(w, h) in &[(17u32, 13u32), (64, 33)] {
        let i = img(w, h, Layout::Rgb, 5);
        let (sub, _) = prepared(&i);
        assert_eq!(encode_payload_codec(&i, CODEC_X4PJ), encode_rel(&sub, &J_ARMS[2], None).stream);
        assert_eq!(encode_payload_codec(&i, CODEC_X4P), encode_rel(&sub, &J_ARMS[0], None).stream);
    }
}
