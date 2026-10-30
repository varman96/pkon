//! LISA Stage 1 A/B driver: repeated timed runs of the full pipeline on canonical samples, with residual-stream and
//! payload hashes and an exact-decode gate. Uses only the public `lisa` API, so it builds against any revision.
//! Usage: bench_lisa_stage1 <dataset_manifest.json> <out.json> <reps> <id>...
use nanoscope::lisa::*;
use nanoscope::lisa_bayer_plane::{decode_stage1, encode_stage1, organize_stage2_with_mode, restore_stage1_with_mode, Stage1Mode};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::time::Instant;

fn hex(d: impl AsRef<[u8]>) -> String {
    Sha256::digest(d).iter().map(|b| format!("{b:02x}")).collect()
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let manifest: Vec<Value> = serde_json::from_slice(&std::fs::read(&args[1]).unwrap()).unwrap();
    let reps: usize = args[3].parse().unwrap();
    let mut out = Vec::new();
    for id in &args[4..] {
        let m = manifest.iter().find(|m| m["id"] == id.as_str()).expect("id not in manifest");
        let (w, h) = (m["width"].as_u64().unwrap() as usize, m["height"].as_u64().unwrap() as usize);
        let bd = m["bit_depth"].as_u64().unwrap() as u8;
        let bytes = std::fs::read(m["canon_path"].as_str().unwrap()).unwrap();
        let samples: Vec<u16> = bytes.chunks_exact(2).map(|c| u16::from_le_bytes([c[0], c[1]])).collect();
        let dim: Vec<usize> = m["cfa_dim"].as_array().unwrap().iter().map(|v| v.as_u64().unwrap() as usize).collect();
        let org: Vec<usize> = m["cfa_origin"].as_array().unwrap().iter().map(|v| v.as_u64().unwrap() as usize).collect();
        let entries: Vec<ColorFilter> = m["cfa"].as_array().unwrap().iter()
            .map(|v| ColorFilter::try_from(v.as_u64().unwrap() as u8).unwrap()).collect();
        let pat = CfaPattern { width: dim[1], height: dim[0], entries, origin_x: org[0], origin_y: org[1] };
        let meta = ImageMetadata::with_cfa(w, h, SampleLayout::BayerCfa, pat);
        meta.validate().expect("bayer metadata");

        let mut runs = Vec::new();
        for _ in 0..reps {
            let t = Instant::now();
            let s1 = encode_stage1(&samples, w, h, bd, meta.clone(), Stage1Mode::default()).unwrap();
            let t_s1 = t.elapsed().as_secs_f64();
            let res_hash = hex(s1.residuals.iter().flat_map(|r| r.to_le_bytes()).collect::<Vec<u8>>());
            let t = Instant::now();
            let s3 = classify_stage3_with(organize_stage2_with_mode(s1, Stage1Mode::default()).unwrap(), Stage3Rule::Legacy4).unwrap();
            let s4 = encode_stage4(s3).unwrap();
            let t_rest = t.elapsed().as_secs_f64();
            let t = Instant::now();
            let back3 = decode_stage4(&s4).unwrap();
            let (s1b, mode) = restore_stage1_with_mode(&back3.into_stage2()).unwrap();
            let dec = decode_stage1(&s1b, mode).unwrap();
            let t_dec = t.elapsed().as_secs_f64();
            let exact = dec == samples;
            let r = json!({"stage1_s": t_s1, "encode_s": t_s1 + t_rest, "decode_s": t_dec, "residual_sha256": res_hash,
                "payload_bytes": s4.payload().len(), "payload_sha256": hex(s4.payload()), "exact": exact});
            eprintln!("{id}: {r}");
            runs.push(r);
        }
        out.push(json!({"id": id, "width": w, "height": h, "bit_depth": bd, "runs": runs}));
    }
    std::fs::write(&args[2], serde_json::to_vec_pretty(&out).unwrap()).unwrap();
}
