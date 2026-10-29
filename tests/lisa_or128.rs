//! Production OR-128 (Stage 3 rule version 2): full pixel round trips, encoder/decoder agreement, determinism, frozen
//! table integrity, and legacy (rule version 1) behavior.
mod or128_cases;
use nanoscope::lisa::*;
use nanoscope::lisa_or128::{self, OR128_K, OR128_TABLE};
use nanoscope::lisa_pcq::{frozen_ctx, pcq_encode};
use or128_cases::cases;
use sha2::{Digest, Sha256};

const TABLE_SHA256: &str = "e6dd165df4b46dc8eccbe0c63dac4bae1f6850c895859efd3cf7c630e43f1c51";
/// Payload size and sha256 of the legacy (rule v1, four-class) Stage 4 payload of each case, produced by the
/// pre-OR-128 revision 5612966 (examples/legacy_hash.rs run on a clean `git archive` of that commit).
const LEGACY: &[(&str, usize, &str)] = &[
    ("bayer_23x17_12", 318, "b9f10f388c767895c00753f3a533846eeaa3a334e81de574a5f49b81fa9311b9"),
    ("bayer_64x48_14", 2995, "d709acfe139ed0df37cc1c366d62bf27271bc7a5c4348e4a0e7cacf8730ba7cf"),
    ("mono_19x14_8", 177, "bac9f3c02b716f661d4a17bb96771327065f290281fc71a70b8f596c031a600e"),
    ("mono_31x29_10", 706, "982301864038274fb9973fc4f0e155ccb86aa47f9835d3d032535b1859f8fec7"),
    ("mono_19x14_16", 319, "305cbfa3e67f74823267c63d9e90a1a6060194df4300f86a52744a73b82bfe3e"),
    ("othercfa_24x12_12", 228, "25451a5e072a30249d9a755bef3e89bc97de99afeeef150f6bf7d3c8165fca41"),
    ("mono_1x1_8", 5, "bcba02f2e70beff23c31ed344e82cf2947d89df51daa4b95387dcc5ac4d180c6"),
];

fn hex(b: &[u8]) -> String {
    b.iter().map(|x| format!("{x:02x}")).collect()
}

fn s3_of(samples: &[u16], meta: &ImageMetadata, bits: u8, rule: Stage3Rule) -> Stage3Output {
    let s1 = encode_with_metadata(samples, meta.width, meta.height, bits, meta.clone()).unwrap();
    classify_stage3_with(organize_stage2(s1).unwrap(), rule).unwrap()
}

fn pixels_back(s4: &Stage4Output) -> Vec<u16> {
    let s3 = decode_stage4(s4).unwrap();
    let (s1, _) = restore_stage1(&s3.into_stage2()).unwrap();
    decode_with_metadata(&s1).unwrap()
}

#[test]
fn frozen_table_matches_recorded_hash_and_research_artifact() {
    assert_eq!(hex(&Sha256::digest(OR128_TABLE)), TABLE_SHA256);
    assert_eq!(OR128_K, 128);
    assert_eq!(OR128_TABLE, nanoscope::lisa_pcq_table::PCQ_TABLE, "differs from the research table");
    let art = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("results/20261029-lisa-pcq1/train/map_K128.bin");
    if let Ok(b) = std::fs::read(art) {
        assert_eq!(b, OR128_TABLE, "differs from map_K128.bin");
    }
    assert!(OR128_TABLE.iter().all(|&c| usize::from(c) < OR128_K));
    for c in 0..OR128_K {
        assert!(OR128_TABLE.contains(&(c as u8)), "context {c} unused");
    }
}

#[test]
fn context_is_total_and_equals_research_mapping() {
    let vals = [0u32, 1, 2, 3, 4, 5, 6, 7, 11, 12, 48, 1023, 1535, 65535, 131070, u32::MAX];
    let opts: Vec<Option<u32>> = std::iter::once(None).chain(vals.iter().map(|&v| Some(v))).collect();
    for &l in &opts {
        for &a in &opts {
            for &al in &opts {
                let c = lisa_or128::context(l, a, al);
                assert!(usize::from(c) < OR128_K);
                assert_eq!(usize::from(c), frozen_ctx(l, a, al));
                assert_eq!(Stage3Rule::Or128.context(l, a, al), c);
            }
        }
    }
}

#[test]
fn or128_full_pixel_round_trip_all_paths() {
    for (name, samples, meta, bits) in cases() {
        let s3 = s3_of(&samples, &meta, bits, Stage3Rule::Or128);
        assert_eq!(s3.rule(), Some(Stage3Rule::Or128), "{name}");
        let Some(ReconstructionRecord::Stage3Context(r3)) = s3.history().last() else { panic!("{name}: no Stage 3 record") };
        assert_eq!((r3.rule_version, r3.class_count), (2, 128), "{name}");

        let s4 = encode_stage4(s3.clone()).unwrap();
        let Some(ReconstructionRecord::Stage4Entropy(r4)) = s4.history().last() else { panic!("{name}: no Stage 4 record") };
        assert_eq!((r4.coding_version, r4.model_count), (1, 128), "{name}");
        assert_eq!(&s4.history()[..s4.history().len() - 1], s3.history(), "{name}");

        // Encoder and decoder context ids agree exactly (Stage3Output equality covers residuals and contexts).
        assert_eq!(decode_stage4(&s4).unwrap(), s3, "{name}: decoded Stage 3 differs");
        assert_eq!(pixels_back(&s4), samples, "{name}: pixels differ");

        // Production bytes equal the experiment coder fed the frozen research mapping; repeat encodes are identical.
        assert_eq!(s4.payload(), &pcq_encode(&s3, OR128_K, frozen_ctx).0[..], "{name}: differs from PCQ-128 experiment");
        assert_eq!(encode_stage4(s3.clone()).unwrap(), s4, "{name}: non-deterministic");
        let ctx_ids: Vec<u8> = match s3.contexts() {
            OrganizedContexts::Morton(c) => c.clone(),
            OrganizedContexts::BayerPlanes(p) => p.iter().flatten().copied().collect(),
        };
        assert!(ctx_ids.iter().all(|&c| usize::from(c) < OR128_K), "{name}");
    }
}

#[test]
fn rp1_estimator_round_trips_with_or128() {
    let (name, samples, meta, bits) = cases().remove(0);
    let s3 = s3_of(&samples, &meta, bits, Stage3Rule::Or128);
    let s4 = encode_stage4_with(s3, Stage4Estimator::RewardPenalty).unwrap();
    assert_eq!(pixels_back(&s4), samples, "{name}");
}

#[test]
fn legacy_rule_is_byte_identical_to_pre_or128_production_and_still_decodes() {
    assert_eq!(LEGACY.len(), cases().len());
    for ((name, samples, meta, bits), &(lname, len, sha)) in cases().into_iter().zip(LEGACY) {
        assert_eq!(name, lname);
        let s3 = s3_of(&samples, &meta, bits, Stage3Rule::Legacy4);
        let Some(ReconstructionRecord::Stage3Context(r3)) = s3.history().last() else { panic!() };
        assert_eq!((r3.rule_version, r3.class_count, r3.thresholds), (1, 4, STAGE3_THRESHOLDS), "{name}");
        let s4 = encode_stage4(s3).unwrap();
        let Some(ReconstructionRecord::Stage4Entropy(r4)) = s4.history().last() else { panic!() };
        assert_eq!((r4.coding_version, r4.model_count), (1, 4), "{name}");
        assert_eq!((s4.payload().len(), hex(&Sha256::digest(s4.payload()))), (len, sha.to_string()), "{name}: legacy bytes changed");
        assert_eq!(pixels_back(&s4), samples, "{name}: legacy stream no longer decodes");
        // The same pixels under OR-128 yield a different stream (legacy bytes are not silently the new format).
        if samples.len() > 1 {
            let s4n = encode_stage4(s3_of(&samples, &meta, bits, Stage3Rule::Or128)).unwrap();
            assert_ne!(s4n.payload(), s4.payload(), "{name}");
        }
    }
}
