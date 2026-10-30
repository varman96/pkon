//! LISA CPU-time / speed-compression experiment harness (historical/20261029-lisa-speed-compression). Production default path,
//! same boundaries as exp_lisa_vs_jxl_png (in-memory &[u16] -> Stage4Output), but with the four stages timed separately:
//!   S1 = planar Stage 1 prediction (`encode_stage1`), S2 = `organize_stage2_with_mode`, S3 = `classify_stage3`,
//!   S4 = `encode_stage4`. Stage timers are Instant (wall clock, single thread, so == CPU time when the machine is idle).
//! Usage: exp_lisa_speed <source_manifest.json> <split.json> <split-key|all> <out.json> [reps=3] [dec=0|1]
use nanoscope::lisa::*;
use nanoscope::lisa_bayer_plane::{decode_stage1, encode_stage1, restore_stage1_with_mode, Stage1Mode};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::time::Instant;

fn meta_of(m: &Value, w: usize, h: usize) -> ImageMetadata {
    let Some(cfa) = m["cfa"].as_array().filter(|a| !a.is_empty()) else {
        return ImageMetadata::without_cfa(w, h, SampleLayout::Monochrome);
    };
    let dim: Vec<usize> = m["cfa_dim"].as_array().unwrap().iter().map(|v| v.as_u64().unwrap() as usize).collect();
    let org: Vec<usize> = m["cfa_origin"].as_array().unwrap().iter().map(|v| v.as_u64().unwrap() as usize).collect();
    let entries: Vec<ColorFilter> = cfa.iter().map(|v| ColorFilter::try_from(v.as_u64().unwrap() as u8).unwrap()).collect();
    let pat = CfaPattern { width: dim[1], height: dim[0], entries, origin_x: org[0], origin_y: org[1] };
    let bayer = ImageMetadata::with_cfa(w, h, SampleLayout::BayerCfa, pat.clone());
    if (dim[0], dim[1]) == (2, 2) && bayer.validate().is_ok() { bayer } else { ImageMetadata::with_cfa(w, h, SampleLayout::OtherCfa, pat) }
}

/// Same composition as the production `enc` in exp_lisa_vs_jxl_png, split into timed stages.
fn enc_timed(samples: &[u16], w: usize, h: usize, bd: u8, meta: &ImageMetadata) -> (Stage4Output, [f64; 4]) {
    let t = Instant::now();
    let s1 = encode_stage1(samples, w, h, bd, meta.clone(), Stage1Mode::default()).unwrap();
    let t1 = t.elapsed().as_secs_f64();
    let t = Instant::now();
    let s2 = nanoscope::lisa_bayer_plane::organize_stage2_with_mode(s1, Stage1Mode::default()).unwrap();
    let t2 = t.elapsed().as_secs_f64();
    let t = Instant::now();
    let s3 = classify_stage3(s2).unwrap();
    let t3 = t.elapsed().as_secs_f64();
    let t = Instant::now();
    let s4 = encode_stage4(s3).unwrap();
    let t4 = t.elapsed().as_secs_f64();
    (s4, [t1, t2, t3, t4])
}

fn dec(s4: &Stage4Output) -> Vec<u16> {
    let d3 = decode_stage4(s4).unwrap();
    let (s1, mode) = restore_stage1_with_mode(&d3.into_stage2()).unwrap();
    decode_stage1(&s1, mode).unwrap()
}

fn median(mut v: Vec<f64>) -> f64 {
    v.sort_by(|a, b| a.partial_cmp(b).unwrap());
    v[v.len() / 2]
}

fn main() {
    let a: Vec<String> = std::env::args().collect();
    let manifest: Value = serde_json::from_slice(&std::fs::read(&a[1]).unwrap()).unwrap();
    let split: Value = serde_json::from_slice(&std::fs::read(&a[2]).unwrap()).unwrap();
    let ids: Option<Vec<String>> = (a[3] != "all").then(|| split[&a[3]].as_array().unwrap().iter().map(|v| v.as_str().unwrap().to_string()).collect());
    let reps: usize = a.get(5).map_or(3, |s| s.parse().unwrap());
    let do_dec = a.get(6).map_or(false, |s| s == "1");
    let mut out = vec![];
    for m in manifest.as_array().unwrap() {
        let id = m["id"].as_str().unwrap();
        if ids.as_ref().map_or(false, |l| !l.iter().any(|x| x == id)) {
            continue;
        }
        let (w, h) = (m["width"].as_u64().unwrap() as usize, m["height"].as_u64().unwrap() as usize);
        let bd = m["bit_depth"].as_u64().unwrap() as u8;
        let path = format!("data/praw1/canon/{}", m["canon_path"].as_str().unwrap().replace('\\', "/").rsplit('/').next().unwrap());
        let bytes = std::fs::read(&path).unwrap();
        let sha_ok = format!("{:x}", Sha256::digest(&bytes)) == m["canon_sha256"].as_str().unwrap() && bytes.len() == w * h * 2;
        let samples: Vec<u16> = bytes.chunks_exact(2).map(|c| u16::from_le_bytes([c[0], c[1]])).collect();
        drop(bytes);
        let meta = meta_of(m, w, h);
        let (warm, _) = enc_timed(&samples, w, h, bd, &meta); // warm-up + determinism reference
        let psha = format!("{:x}", Sha256::digest(warm.payload()));
        let mut stages: Vec<[f64; 4]> = vec![];
        let mut totals = vec![];
        let mut deterministic = true;
        for _ in 0..reps {
            let t = Instant::now();
            let (s4, st) = std::hint::black_box(enc_timed(&samples, w, h, bd, &meta));
            totals.push(t.elapsed().as_secs_f64());
            stages.push(st);
            deterministic &= s4.payload() == warm.payload();
        }
        let mut dec_s = vec![];
        let mut mismatches = 0usize;
        for r in 0..(if do_dec { reps } else { 1 }) {
            let t = Instant::now();
            let d = std::hint::black_box(dec(&warm));
            if do_dec || r == 0 {
                dec_s.push(t.elapsed().as_secs_f64());
            }
            mismatches = d.iter().zip(&samples).filter(|(x, y)| x != y).count() + d.len().abs_diff(samples.len());
        }
        let med: Vec<f64> = (0..4).map(|i| median(stages.iter().map(|s| s[i]).collect())).collect();
        println!("{id}: {w}x{h} payload {} B enc {:.3}s [S1 {:.3} S2 {:.3} S3 {:.3} S4 {:.3}] dec {:.3}s mism={mismatches} det={deterministic} sha_ok={sha_ok} psha={}",
            warm.payload().len(), median(totals.clone()), med[0], med[1], med[2], med[3], median(dec_s.clone()), &psha[..12]);
        out.push(json!({"id": id, "width": w, "height": h, "pixels": w * h, "payload_bytes": warm.payload().len(), "payload_sha256": psha,
            "mismatches": mismatches, "deterministic": deterministic, "canon_sha_ok": sha_ok,
            "enc_s": totals, "enc_median_s": median(totals), "stage_median_s": med, "dec_s": dec_s, "dec_median_s": median(dec_s),
            "dec_timing_is_repeated": do_dec}));
    }
    std::fs::write(&a[4], serde_json::to_vec_pretty(&out).unwrap()).unwrap();
}
