//! Diagnostic runner for the reward-penalty regression. Usage: exp_lisa_rp_diag <manifest> <out.json> <id>...
use nanoscope::lisa::*;
use nanoscope::lisa_rp::*;
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
        let m = manifest.as_array().unwrap().iter().find(|m| m["id"].as_str() == Some(id)).unwrap();
        let (w, h) = (m["width"].as_u64().unwrap() as usize, m["height"].as_u64().unwrap() as usize);
        let bd = m["bit_depth"].as_u64().unwrap() as u8;
        let bytes = std::fs::read(m["canon_path"].as_str().unwrap()).unwrap();
        let samples: Vec<u16> = bytes.chunks_exact(2).map(|c| u16::from_le_bytes([c[0], c[1]])).collect();
        let s3 = classify_stage3_with(organize_stage2(encode_with_metadata(&samples, w, h, bd, meta_of(m, w, h)).unwrap()).unwrap(), Stage3Rule::Legacy4).unwrap();
        let d = diagnose(&s3, samples.len());
        let n = samples.len() as f64;
        let tb: f64 = d.class.iter().map(|a| a[0]).sum::<f64>() + d.fixed_bits;
        let tc: f64 = d.class.iter().map(|a| a[1]).sum::<f64>() + d.fixed_bits;
        println!(
            "== {id}: n={} adaptive decisions/sample {:.2}; total base {:.5} cand {:.5} delta {:+.5} bps; fixed-bit share {:.3} bps",
            samples.len(), d.n_dec as f64 / n, tb / n, tc / n, (tc - tb) / n, d.fixed_bits / n
        );
        let f = |a: &[f64; 3]| format!("dec {:>10} base/dec {:.4} cand/dec {:.4} delta_bps {:+.5}", a[2], a[0] / a[2], a[1] / a[2], (a[1] - a[0]) / n);
        for (i, a) in d.plane.iter().enumerate() { println!("plane {i}: {}", f(a)); }
        for (i, a) in d.class.iter().enumerate() { println!("class {i}: {}", f(a)); }
        for (i, a) in d.seg.iter().enumerate() { println!("seg {i:>2}: {}", f(a)); }
        for (i, a) in d.pbin.iter().enumerate() { println!("obs-p-bin {i}: {}", f(a)); }
        let mut rows = Vec::new();
        let mut stat_tot = 0.0;
        for c in 0..4 { for k in 0..2 { for s in 0..=33 {
            let a = &d.slot[slot_index(c, k, s)];
            if a.n == 0 { continue; }
            let st = static_cost(a);
            stat_tot += st;
            rows.push((c, k, s, a.n, a.ones as f64 / a.n as f64, a.base, a.cand, st));
        }}}
        println!("oracle static adaptive cost {:.5} bps vs base {:.5} cand {:.5}", stat_tot / n, (tb - d.fixed_bits) / n, (tc - d.fixed_bits) / n);
        rows.sort_by(|a, b| ((b.6 - b.5).abs()).partial_cmp(&(a.6 - a.5).abs()).unwrap());
        for r in rows.iter().take(12) {
            println!("  cls {} {} slot {:>2}: n {:>9} p1 {:.3} | base-static {:+.5} cand-static {:+.5} cand-base {:+.5} bps",
                r.0, if r.1 == 0 { "unary" } else { "mant " }, r.2, r.3, r.4, (r.5 - r.7) / n, (r.6 - r.7) / n, (r.6 - r.5) / n);
        }
        out.push(json!({"id": id, "delta_bps": (tc - tb) / n,
            "plane_delta_bps": d.plane.iter().map(|a| (a[1]-a[0])/n).collect::<Vec<_>>(),
            "class_delta_bps": d.class.iter().map(|a| (a[1]-a[0])/n).collect::<Vec<_>>(),
            "seg_delta_bps": d.seg.iter().map(|a| (a[1]-a[0])/n).collect::<Vec<_>>(),
            "pbin_delta_bps": d.pbin.iter().map(|a| (a[1]-a[0])/n).collect::<Vec<_>>(),
            "static_adaptive_bps": stat_tot / n}));
    }
    std::fs::write(&args[2], serde_json::to_vec_pretty(&out).unwrap()).unwrap();
}
