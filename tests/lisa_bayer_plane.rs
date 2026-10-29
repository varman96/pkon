//! Experimental Bayer-plane Stage 1 and the reduced (64/32-leaf) OR-128 trees: exact reconstruction across all four CFA
//! phases and bit depths, metadata preservation, determinism, and the unchanged production path.
mod or128_cases;
use nanoscope::lisa::*;
use nanoscope::lisa_bayer_plane::{decode_stage1, encode_stage1, Stage1Mode};
use or128_cases::{cases, image};

fn phases() -> Vec<(&'static str, Vec<ColorFilter>)> {
    use ColorFilter::*;
    vec![("RGGB", vec![Red, Green, Green, Blue]), ("GRBG", vec![Green, Red, Blue, Green]),
         ("GBRG", vec![Green, Blue, Red, Green]), ("BGGR", vec![Blue, Green, Green, Red])]
}

fn smooth(w: usize, h: usize, bits: u8) -> Vec<u16> {
    let maxv = (1u32 << bits) - 1;
    let noise = image(w, h, bits);
    (0..w * h).map(|i| (((i % w) * 3 + (i / w) * 5) as u32 % (maxv + 1)).wrapping_add(u32::from(noise[i]) % 7).min(maxv) as u16).collect()
}

fn full_chain(samples: &[u16], meta: &ImageMetadata, bits: u8, mode: Stage1Mode, rule: Stage3Rule) {
    let s1 = encode_stage1(samples, meta.width, meta.height, bits, meta.clone(), mode).unwrap();
    assert_eq!(s1.metadata, *meta);
    let s3 = classify_stage3_with(organize_stage2(s1).unwrap(), rule).unwrap();
    let s4 = encode_stage4(s3.clone()).unwrap();
    assert_eq!(s4, encode_stage4(s3.clone()).unwrap(), "non-deterministic");
    let d3 = decode_stage4(&s4).unwrap();
    assert_eq!(d3, s3);
    let (s1b, _) = restore_stage1(&d3.into_stage2()).unwrap();
    assert_eq!(s1b.metadata, *meta, "CFA metadata changed");
    assert_eq!(decode_stage1(&s1b, mode).unwrap(), samples);
}

#[test]
fn planar_stage1_exact_all_phases_depths_and_geometries() {
    for (name, entries) in phases() {
        for &(w, h) in &[(23usize, 17usize), (64, 48), (2, 2), (1, 7), (5, 1), (1, 1), (3, 3)] {
            for &bits in &[8u8, 10, 12, 14, 16] {
                for (ox, oy) in [(0, 0), (1, 0), (0, 1), (1, 1)] {
                    let cfa = CfaPattern { width: 2, height: 2, entries: entries.clone(), origin_x: ox, origin_y: oy };
                    let meta = ImageMetadata::with_cfa(w, h, SampleLayout::BayerCfa, cfa);
                    for samples in [image(w, h, bits), smooth(w, h, bits)] {
                        let tag = format!("{name} {w}x{h} {bits}b origin {ox},{oy}");
                        let s1 = encode_stage1(&samples, w, h, bits, meta.clone(), Stage1Mode::BayerPlanar).unwrap();
                        assert_eq!(s1, encode_stage1(&samples, w, h, bits, meta.clone(), Stage1Mode::BayerPlanar).unwrap(), "{tag}");
                        assert_eq!(s1.metadata, meta, "{tag}");
                        assert_eq!(decode_stage1(&s1, Stage1Mode::BayerPlanar).unwrap(), samples, "{tag}");
                    }
                }
            }
        }
    }
}

#[test]
fn planar_full_chain_every_rule() {
    for rule in [Stage3Rule::Or128, Stage3Rule::Or128Tree, Stage3Rule::Or128Tree64, Stage3Rule::Or128Tree32] {
        for (name, entries) in phases() {
            for &bits in &[8u8, 12, 14, 16] {
                let meta = ImageMetadata::with_cfa(37, 29, SampleLayout::BayerCfa, CfaPattern { width: 2, height: 2, entries: entries.clone(), origin_x: 0, origin_y: 0 });
                full_chain(&smooth(37, 29, bits), &meta, bits, Stage1Mode::BayerPlanar, rule);
                let _ = name;
            }
        }
    }
}

#[test]
fn non_bayer_layouts_fall_back_to_production_stage1() {
    for (name, samples, meta, bits) in cases() {
        let a = encode_stage1(&samples, meta.width, meta.height, bits, meta.clone(), Stage1Mode::BayerPlanar).unwrap();
        let b = encode_with_metadata(&samples, meta.width, meta.height, bits, meta.clone()).unwrap();
        if meta.layout != SampleLayout::BayerCfa {
            assert_eq!(a, b, "{name}");
        }
        full_chain(&samples, &meta, bits, Stage1Mode::BayerPlanar, Stage3Rule::Or128);
    }
}

#[test]
fn mosaic_mode_is_the_production_stage1() {
    for (name, samples, meta, bits) in cases() {
        let a = encode_stage1(&samples, meta.width, meta.height, bits, meta.clone(), Stage1Mode::Mosaic).unwrap();
        assert_eq!(a, encode_with_metadata(&samples, meta.width, meta.height, bits, meta.clone()).unwrap(), "{name}");
    }
}

#[test]
fn reduced_trees_round_trip_deterministic_on_production_stage1() {
    for rule in [Stage3Rule::Or128Tree64, Stage3Rule::Or128Tree32] {
        let want = if rule == Stage3Rule::Or128Tree64 { (4u8, 64usize) } else { (5, 32) };
        assert_eq!((rule.version(), rule.context_count()), want);
        for (name, samples, meta, bits) in cases() {
            full_chain(&samples, &meta, bits, Stage1Mode::Mosaic, rule);
            let s3 = classify_stage3_with(organize_stage2(encode_with_metadata(&samples, meta.width, meta.height, bits, meta.clone()).unwrap()).unwrap(), rule).unwrap();
            assert_eq!(s3.rule(), Some(rule), "{name}");
        }
    }
}

#[test]
fn reduced_tree_is_prefix_of_128_tree() {
    use nanoscope::lisa_or128_tree_small_data::{l32, l64};
    let nodes128 = nanoscope::lisa_or128_tree::OR128_TREE_NODES;
    // Same growth order: the first L-1 nodes carry the same (feature, threshold) as the frozen tree.
    for (l, nodes) in [(64usize, l64::OR128_TREE_NODES), (32, l32::OR128_TREE_NODES)] {
        assert_eq!(nodes.len(), l - 1);
        for (a, b) in nodes.iter().zip(nodes128) {
            assert_eq!((a.0, a.1), (b.0, b.1));
        }
    }
}

// ---- promoted path: the mode is carried by the stream history, not out of band ----

use nanoscope::lisa_bayer_plane::{organize_stage2_with_mode, restore_stage1_with_mode};

/// Encode with `mode`, run Stage 2-4, then decode using only the stream (no mode passed); returns (stream, mode read back).
fn stream_roundtrip(samples: &[u16], meta: &ImageMetadata, bits: u8, mode: Stage1Mode) -> (Stage4Output, Stage1Mode) {
    let s1 = encode_stage1(samples, meta.width, meta.height, bits, meta.clone(), mode).unwrap();
    let s3 = classify_stage3(organize_stage2_with_mode(s1, mode).unwrap()).unwrap();
    let s4 = encode_stage4(s3.clone()).unwrap();
    assert_eq!(s4, encode_stage4(s3).unwrap(), "non-deterministic");
    let d3 = decode_stage4(&s4).unwrap();
    let (s1b, got) = restore_stage1_with_mode(&d3.into_stage2()).unwrap();
    assert_eq!(s1b.metadata, *meta, "metadata changed");
    assert_eq!(decode_stage1(&s1b, got).unwrap(), samples, "pixels changed");
    (s4, got)
}

#[test]
fn default_mode_is_bayer_planar_and_both_modes_decode_from_the_stream() {
    assert_eq!(Stage1Mode::default(), Stage1Mode::BayerPlanar);
    for (name, samples, meta, bits) in cases() {
        let bayer = meta.layout == SampleLayout::BayerCfa;
        let (planar, got) = stream_roundtrip(&samples, &meta, bits, Stage1Mode::default());
        assert_eq!(got, if bayer { Stage1Mode::BayerPlanar } else { Stage1Mode::Mosaic }, "{name}");
        let (mosaic, got) = stream_roundtrip(&samples, &meta, bits, Stage1Mode::Mosaic);
        assert_eq!(got, Stage1Mode::Mosaic, "{name}");
        if !bayer {
            assert_eq!(planar, mosaic, "{name}: non-Bayer must be identical to the existing path");
        }
    }
}

#[test]
fn legacy_streams_are_unchanged_and_read_back_as_mosaic() {
    for (name, samples, meta, bits) in cases() {
        // Exactly what the pre-promotion pipeline produced.
        let s1 = encode_with_metadata(&samples, meta.width, meta.height, bits, meta.clone()).unwrap();
        let legacy = encode_stage4(classify_stage3(organize_stage2(s1).unwrap()).unwrap()).unwrap();
        assert!(matches!(legacy.history()[0], ReconstructionRecord::Stage1Decorrelation { .. }), "{name}");
        let (via_mode, _) = stream_roundtrip(&samples, &meta, bits, Stage1Mode::Mosaic);
        assert_eq!(legacy, via_mode, "{name}: fallback is not byte/record-identical to legacy");
        let (s1b, mode) = restore_stage1_with_mode(&decode_stage4(&legacy).unwrap().into_stage2()).unwrap();
        assert_eq!(mode, Stage1Mode::Mosaic, "{name}");
        assert_eq!(decode_with_metadata(&s1b).unwrap(), samples, "{name}");
    }
}

#[test]
fn planar_streams_record_a_distinct_stage1_record() {
    let (name, samples, meta, bits) = cases().into_iter().find(|c| c.2.layout == SampleLayout::BayerCfa).expect("a Bayer case");
    let (s4, _) = stream_roundtrip(&samples, &meta, bits, Stage1Mode::BayerPlanar);
    assert!(matches!(s4.history()[0], ReconstructionRecord::Stage1BayerPlanar { .. }), "{name}");
}
