//! LISA side of the LISA vs JPEG XL vs PNG benchmark. Production default path only (Bayer-plane Stage 1 for Bayer CFA,
//! mosaic fallback otherwise; default Stage 3 rule; Stage 4). In-memory timing, 1 warm-up + N measured reps, median.
//! Encode boundary: &[u16] samples -> Stage4Output. Decode boundary: Stage4Output -> Vec<u16> samples.
//! Usage (from the repo root): cargo run --release --bin exp_lisa_vs_jxl_png -- <manifest.json> <out.json> [reps=3]
//! Benchmark layout: benchmarks/lisa-vs-jxl-png/ (this file, exp_lisa_vs_jxl_png.py, and the recorded results). Paths in the manifest (data/praw1/canon/...) are relative to the repo root.
use nanoscope::lisa::*;
use nanoscope::lisa_bayer_plane::{decode_stage1, encode_stage2, restore_stage1_with_mode};
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

fn enc(samples: &[u16], w: usize, h: usize, bd: u8, meta: &ImageMetadata) -> Stage4Output {
    let s2 = encode_stage2(samples, w, h, bd, meta.clone()).unwrap();
    encode_stage4(classify_stage3(s2).unwrap()).unwrap()
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
    let reps: usize = a.get(3).map_or(3, |s| s.parse().unwrap());
    let mut out = vec![];
    for m in manifest.as_array().unwrap() {
        let id = m["id"].as_str().unwrap();
        let (w, h) = (m["width"].as_u64().unwrap() as usize, m["height"].as_u64().unwrap() as usize);
        let bd = m["bit_depth"].as_u64().unwrap() as u8;
        let bytes = std::fs::read(m["canon_path"].as_str().unwrap().replace("\\", "/")).unwrap();
        let sha_ok = format!("{:x}", Sha256::digest(&bytes)) == m["canon_sha256"].as_str().unwrap() && bytes.len() == w * h * 2;
        let samples: Vec<u16> = bytes.chunks_exact(2).map(|c| u16::from_le_bytes([c[0], c[1]])).collect();
        drop(bytes);
        let meta = meta_of(m, w, h);
        let warm = enc(&samples, w, h, bd, &meta); // warm-up (also reference for determinism)
        let _ = std::hint::black_box(dec(&warm));
        let (mut te, mut td) = (vec![], vec![]);
        let mut deterministic = true;
        let mut last = None;
        for _ in 0..reps {
            let t = Instant::now();
            let s4 = std::hint::black_box(enc(&samples, w, h, bd, &meta));
            te.push(t.elapsed().as_secs_f64());
            deterministic &= s4.payload() == warm.payload();
            last = Some(s4);
        }
        let s4 = last.unwrap();
        let mut decoded = vec![];
        for _ in 0..reps {
            let t = Instant::now();
            decoded = std::hint::black_box(dec(&s4));
            td.push(t.elapsed().as_secs_f64());
        }
        let mismatches = decoded.iter().zip(&samples).filter(|(x, y)| x != y).count() + decoded.len().abs_diff(samples.len());
        let mode = match warm.history().iter().find_map(|r| if let ReconstructionRecord::Stage2Organization(_) = r { Some(format!("{r:?}")) } else { None }) { Some(s) => s.chars().take(60).collect::<String>(), None => "?".into() };
        println!("{id}: {w}x{h} {bd}b {:?} payload {} B enc {:.3}s dec {:.3}s mismatches={mismatches} det={deterministic} sha_ok={sha_ok}", meta.layout, s4.payload().len(), median(te.clone()), median(td.clone()));
        out.push(json!({"id": id, "layout": format!("{:?}", meta.layout), "stage2_record": mode, "payload_bytes": s4.payload().len(),
            "decoded_width_height": [w, h], "bit_depth": bd, "mismatches": mismatches, "deterministic": deterministic, "canon_sha_ok": sha_ok,
            "enc_s": te, "dec_s": td, "enc_median_s": median(te), "dec_median_s": median(td)}));
    }
    std::fs::write(&a[2], serde_json::to_vec_pretty(&out).unwrap()).unwrap();
}
