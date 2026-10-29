//! Targeted measurement: (1) Bayer colour-plane Stage 1 vs production Stage 1, (2) 64/32-leaf OR-128 trees vs the
//! 128-leaf tree and the fixed table. Production Stage 2/3/4 are used unmodified; every size is a real Stage 4 payload.
//! Usage: exp_bayer_small_tree <manifest.json> <split.json> <out_dir> [threads=2]
use nanoscope::lisa::*;
use nanoscope::lisa_bayer_plane::{decode_stage1, encode_stage1, Stage1Mode};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;

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

/// Per-image warm-up threshold from the parent-fallback idea (N < 2,048): leaves visited fewer times are "low occupancy".
const WARMUP: u64 = 2048;

fn visits(s3: &Stage3Output, k: usize) -> Vec<u64> {
    let mut v = vec![0u64; k];
    match s3.contexts() {
        OrganizedContexts::Morton(c) => c.iter().for_each(|&x| v[x as usize] += 1),
        OrganizedContexts::BayerPlanes(p) => p.iter().flatten().for_each(|&x| v[x as usize] += 1),
    }
    v
}

fn occupancy(v: &[u64]) -> Value {
    let total: u64 = v.iter().sum();
    let mut s = v.to_vec();
    s.sort_unstable();
    let bottom: u64 = s[..s.len() / 2].iter().sum();
    let low: Vec<u64> = v.iter().copied().filter(|&c| c < WARMUP).collect();
    json!({"leaves": v.len(), "used": v.iter().filter(|&&c| c > 0).count(), "min": s[0], "median": s[s.len() / 2], "max": s[s.len() - 1],
           "bottom50_share": bottom as f64 / total as f64,
           "leaves_lt_2048": low.len(), "visits_share_in_leaves_lt_2048": low.iter().sum::<u64>() as f64 / total as f64,
           "visits": v})
}

fn run_image(m: &Value, split: &str) -> Value {
    let id = m["id"].as_str().unwrap().to_string();
    let (w, h) = (m["width"].as_u64().unwrap() as usize, m["height"].as_u64().unwrap() as usize);
    let bd = m["bit_depth"].as_u64().unwrap() as u8;
    let bytes = std::fs::read(m["canon_path"].as_str().unwrap()).unwrap();
    let sha_ok = format!("{:x}", Sha256::digest(&bytes)) == m["canon_sha256"].as_str().unwrap() && bytes.len() == w * h * 2;
    let samples: Vec<u16> = bytes.chunks_exact(2).map(|c| u16::from_le_bytes([c[0], c[1]])).collect();
    drop(bytes);
    let meta = meta_of(m, w, h);
    let mut checks: Vec<(&str, bool)> = vec![];
    let mut rows = serde_json::Map::new();

    // Production Stage 1 once; its pixel exactness is checked once, then each rule checks Stage 3 residual/context equality.
    let s1 = encode_stage1(&samples, w, h, bd, meta.clone(), Stage1Mode::Mosaic).unwrap();
    checks.push(("mosaic_mode_is_production", s1 == encode_with_metadata(&samples, w, h, bd, meta.clone()).unwrap()));
    let s2 = organize_stage2(s1).unwrap();
    let mut mosaic_exact = false;
    for (name, rule) in [("table128_prod", Stage3Rule::Or128), ("tree128", Stage3Rule::Or128Tree), ("tree64", Stage3Rule::Or128Tree64), ("tree32", Stage3Rule::Or128Tree32)] {
        let s3 = classify_stage3_with(s2.clone(), rule).unwrap();
        let s4 = encode_stage4(s3.clone()).unwrap();
        let det = encode_stage4(s3.clone()).unwrap() == s4;
        let d3 = decode_stage4(&s4).unwrap();
        let eq = d3 == s3 && s3.rule() == Some(rule);
        if !mosaic_exact {
            let (s1b, _) = restore_stage1(&d3.into_stage2()).unwrap();
            mosaic_exact = decode_with_metadata(&s1b).unwrap() == samples && s1b.metadata == meta;
        }
        checks.push((name, det && eq));
        rows.insert(name.into(), json!({"bytes": s4.payload().len(), "deterministic": det, "decoded_stage3_equal": eq,
                                       "occupancy": occupancy(&visits(&s3, rule.context_count()))}));
    }
    checks.push(("mosaic_pixels_exact", mosaic_exact));
    drop(s2);

    // Bayer colour-plane Stage 1 (differs from production only for BayerCfa metadata; otherwise identical by construction).
    let planar_applies = meta.layout == SampleLayout::BayerCfa;
    let p1 = encode_stage1(&samples, w, h, bd, meta.clone(), Stage1Mode::BayerPlanar).unwrap();
    let p1b = encode_stage1(&samples, w, h, bd, meta.clone(), Stage1Mode::BayerPlanar).unwrap();
    let p_s3 = classify_stage3_with(organize_stage2(p1.clone()).unwrap(), Stage3Rule::Or128).unwrap();
    let p_s4 = encode_stage4(p_s3.clone()).unwrap();
    let p_det = p1 == p1b && encode_stage4(p_s3.clone()).unwrap() == p_s4;
    let p_d3 = decode_stage4(&p_s4).unwrap();
    let p_eq = p_d3 == p_s3;
    let (p1r, _) = restore_stage1(&p_d3.into_stage2()).unwrap();
    let p_exact = decode_stage1(&p1r, Stage1Mode::BayerPlanar).unwrap() == samples && p1r.metadata == meta;
    checks.push(("planar_table128", p_det && p_eq && p_exact));
    rows.insert("planar_table128".into(), json!({"bytes": p_s4.payload().len(), "applies": planar_applies, "deterministic": p_det,
        "decoded_stage3_equal": p_eq, "pixels_exact": p_exact, "metadata_equal": p1r.metadata == meta,
        "occupancy": occupancy(&visits(&p_s3, 128))}));

    let ok = sha_ok && checks.iter().all(|c| c.1);
    let failed: Vec<&str> = checks.iter().filter(|c| !c.1).map(|c| c.0).collect();
    println!("{id} [{split}] {w}x{h} {bd}b: prod {} B | planar {} B | tree128 {} | tree64 {} | tree32 {} | ok={ok} {failed:?}",
        rows["table128_prod"]["bytes"], rows["planar_table128"]["bytes"], rows["tree128"]["bytes"], rows["tree64"]["bytes"], rows["tree32"]["bytes"]);
    json!({"id": id, "split": split, "width": w, "height": h, "bit_depth": bd, "samples": w * h, "layout": format!("{:?}", meta.layout),
           "canon_sha_ok": sha_ok, "ok": ok, "failed": failed, "variants": rows})
}

fn main() {
    let a: Vec<String> = std::env::args().collect();
    let manifest: Value = serde_json::from_slice(&std::fs::read(&a[1]).unwrap()).unwrap();
    let split: Value = serde_json::from_slice(&std::fs::read(&a[2]).unwrap()).unwrap();
    let threads: usize = a.get(4).map_or(2, |s| s.parse().unwrap());
    let mut jobs: Vec<(Value, String)> = vec![];
    for s in ["train", "val", "test"] {
        for id in split[s].as_array().unwrap() {
            let m = manifest.as_array().unwrap().iter().find(|m| m["id"] == *id).expect("id not in manifest");
            jobs.push((m.clone(), s.to_string()));
        }
    }
    let next = AtomicUsize::new(0);
    let out = Mutex::new(Vec::new());
    std::thread::scope(|sc| {
        for _ in 0..threads {
            sc.spawn(|| loop {
                let i = next.fetch_add(1, Ordering::SeqCst);
                let Some((m, s)) = jobs.get(i) else { break };
                let r = run_image(m, s);
                out.lock().unwrap().push((i, r));
            });
        }
    });
    let mut v = out.into_inner().unwrap();
    v.sort_by_key(|x| x.0);
    let rows: Vec<Value> = v.into_iter().map(|x| x.1).collect();
    let all_ok = rows.iter().all(|r| r["ok"] == true);
    std::fs::create_dir_all(&a[3]).unwrap();
    std::fs::write(format!("{}/results.json", a[3]), serde_json::to_vec_pretty(&rows).unwrap()).unwrap();
    println!("ALL_OK={all_ok}");
    if !all_ok {
        std::process::exit(1);
    }
}
