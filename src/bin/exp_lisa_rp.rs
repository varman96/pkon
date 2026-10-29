//! Experiment runner: reward-penalty estimator vs production Stage 4 estimator (ideal cost only).
//! Usage: exp_lisa_rp <dataset_manifest.json> <out.json> <id> [<id>...]
use nanoscope::lisa::*;
use nanoscope::lisa_rp::compare;
use serde_json::{json, Value};

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

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let manifest: Value = serde_json::from_slice(&std::fs::read(&args[1]).unwrap()).unwrap();
    let mut out = Vec::new();
    for id in &args[3..] {
        let m = manifest.as_array().unwrap().iter().find(|m| m["id"].as_str() == Some(id)).expect("id not in manifest");
        let (w, h) = (m["width"].as_u64().unwrap() as usize, m["height"].as_u64().unwrap() as usize);
        let bd = m["bit_depth"].as_u64().unwrap() as u8;
        let bytes = std::fs::read(m["canon_path"].as_str().unwrap()).unwrap();
        let samples: Vec<u16> = bytes.chunks_exact(2).map(|c| u16::from_le_bytes([c[0], c[1]])).collect();
        let meta = meta_of(m, w, h);
        let s1 = encode_with_metadata(&samples, w, h, bd, meta).unwrap();
        let s2 = organize_stage2(s1).unwrap();
        let s3 = classify_stage3(s2).unwrap();
        let (base, cand, n) = compare(&s3, samples.len());
        let (bb, cb) = (base / n as f64, cand / n as f64);
        let pct = 100.0 * (cb - bb) / bb;
        println!("{id}: n={n} baseline {bb:.5} bps candidate {cb:.5} bps change {pct:+.3}%");
        out.push(json!({"id": id, "n": n, "baseline_bps": bb, "candidate_bps": cb, "pct_change": pct}));
    }
    std::fs::write(&args[2], serde_json::to_vec_pretty(&out).unwrap()).unwrap();
}
