//! OR-128 learned context tree (Stage 3 rule version 3): frozen-tree integrity, round trips on every Stage 2 path,
//! encoder/decoder agreement, determinism, and that the historical fixed-table OR-128 (v2) stays the unchanged default.
mod or128_cases;
use nanoscope::lisa::*;
use nanoscope::lisa_or128_tree::{self as tree, NF, OR128_TREE_LEAVES};
use or128_cases::cases;
use sha2::{Digest, Sha256};

const TREE_DATA_SHA256: &str = "8df3a7e6f0c250ddb17de7c7430b0ed9000b1c4b6d75c723c0ac89647477dc78";

fn s3_of(samples: &[u16], meta: &ImageMetadata, bits: u8, rule: Stage3Rule) -> Stage3Output {
    let s1 = encode_with_metadata(samples, meta.width, meta.height, bits, meta.clone()).unwrap();
    classify_stage3_with(organize_stage2(s1).unwrap(), rule).unwrap()
}
fn pixels_back(s4: &Stage4Output) -> Vec<u16> {
    let (s1, _) = restore_stage1(&decode_stage4(s4).unwrap().into_stage2()).unwrap();
    decode_with_metadata(&s1).unwrap()
}

#[test]
fn frozen_tree_source_matches_recorded_hash() {
    // Line endings are normalized so a Windows checkout hashes the same.
    let src = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/src/lisa_or128_tree_data.rs")).unwrap().replace("\r\n", "\n");
    let hex: String = Sha256::digest(src.as_bytes()).iter().map(|b| format!("{b:02x}")).collect();
    assert_eq!(hex, TREE_DATA_SHA256, "frozen tree changed: bump the Stage 3 rule version and update this hash");
    assert_eq!(OR128_TREE_LEAVES, 128);
}

#[test]
fn default_rule_is_still_the_fixed_table() {
    assert_eq!(Stage3Rule::default(), Stage3Rule::Or128);
    assert_eq!(Stage3Rule::Or128.version(), 2);
    assert_eq!(Stage3Rule::Or128Tree.version(), 3);
}

#[test]
fn tree_context_matches_rule_and_is_total() {
    let vals = [None, Some(0), Some(1), Some(3), Some(8), Some(100), Some(70000), Some(u32::MAX)];
    for &l in &vals {
        for &a in &vals {
            for &al in &vals {
                let c = tree::context(l, a, al);
                assert!(usize::from(c) < OR128_TREE_LEAVES);
                assert_eq!(Stage3Rule::Or128Tree.context(l, a, al), c);
                assert_eq!(tree::features(l, a, al).len(), NF);
            }
        }
    }
}

#[test]
fn tree_round_trips_all_paths_and_decoder_agrees() {
    for (name, samples, meta, bits) in cases() {
        let s3 = s3_of(&samples, &meta, bits, Stage3Rule::Or128Tree);
        assert_eq!(s3.rule(), Some(Stage3Rule::Or128Tree), "{name}");
        let Some(ReconstructionRecord::Stage3Context(r3)) = s3.history().last() else { panic!() };
        assert_eq!((r3.rule_version, usize::from(r3.class_count)), (3, OR128_TREE_LEAVES), "{name}");
        let s4 = encode_stage4(s3.clone()).unwrap();
        // Stage3Output equality covers residuals AND context ids: the decoder reaches the same leaf for every residual.
        assert_eq!(decode_stage4(&s4).unwrap(), s3, "{name}");
        assert_eq!(pixels_back(&s4), samples, "{name}");
        assert_eq!(encode_stage4(s3).unwrap(), s4, "{name}: non-deterministic");
    }
}

#[test]
fn v2_streams_are_unchanged_by_the_tree_rule() {
    // The fixed-table rule must keep producing and decoding exactly what it did before the tree existed.
    for (name, samples, meta, bits) in cases() {
        let s4 = encode_stage4(s3_of(&samples, &meta, bits, Stage3Rule::default())).unwrap();
        let Some(ReconstructionRecord::Stage3Context(r3)) = s4.history().iter().rev().nth(1) else { panic!() };
        assert_eq!((r3.rule_version, r3.class_count), (2, 128), "{name}");
        assert_eq!(pixels_back(&s4), samples, "{name}");
    }
}
