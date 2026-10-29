//! Experiment runner: production vs fixed-rate RP vs Kalman-gain RP, lockstep ideal cost + rate diagnostics.
//! Usage: exp_lisa_rp3 <dataset_manifest.json> <out.json> <id> [<id>...]
use nanoscope::lisa::*;
use nanoscope::lisa_rp3::*;
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
fn pct(a: f64, b: f64) -> f64 {
    100.0 * (a - b) / b
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let manifest: Value = serde_json::from_slice(&std::fs::read(&args[1]).unwrap()).unwrap();
    let mut out = Vec::new();
    let (mut tn, mut tc) = (0.0f64, [0.0f64; 3]);
    for id in &args[3..] {
        let m = manifest.as_array().unwrap().iter().find(|m| m["id"].as_str() == Some(id)).expect("id not in manifest");
        let (w, h) = (m["width"].as_u64().unwrap() as usize, m["height"].as_u64().unwrap() as usize);
        let bd = m["bit_depth"].as_u64().unwrap() as u8;
        let bytes = std::fs::read(m["canon_path"].as_str().unwrap()).unwrap();
        let samples: Vec<u16> = bytes.chunks_exact(2).map(|c| u16::from_le_bytes([c[0], c[1]])).collect();
        let s3 = classify_stage3(organize_stage2(encode_with_metadata(&samples, w, h, bd, meta_of(m, w, h)).unwrap()).unwrap()).unwrap();
        let d = diagnose(&s3, samples.len());
        let n = samples.len() as f64;
        assert!(d.trace[0] == d.trace[1] && d.trace[1] == d.trace[2] && d.trace[0].0 == d.n_dec, "estimators saw different symbol streams");
        let tot = [d.total.c[0] + d.fixed_bits, d.total.c[1] + d.fixed_bits, d.total.c[2] + d.fixed_bits];
        let bps = [tot[0] / n, tot[1] / n, tot[2] / n];
        for i in 0..3 {
            tc[i] += tot[i];
        }
        tn += n;
        println!("{id}: n={} adaptive decisions={}", samples.len(), d.n_dec);
        println!("  bps production {:.5} fixed {:.5} kalman {:.5}", bps[0], bps[1], bps[2]);
        println!(
            "  kalman vs fixed {:+.3}%  kalman vs production {:+.3}%  fixed vs production {:+.3}%",
            pct(bps[2], bps[1]),
            pct(bps[2], bps[0]),
            pct(bps[1], bps[0])
        );
        let k = d.n_rate as f64;
        println!("  RATE n={} mean {:.6} min {:.3e} max {:.6} zero-alpha {} invalid-state {}", d.n_rate, d.sum / k, d.min, d.max, d.zero, d.invalid);
        println!("  mean alpha if Q=0 (counterfactual) {:.6}; Q raises mean by {:+.3}%", d.sum_no_q / k, pct(d.sum, d.sum_no_q));
        let mut oct = vec![];
        for (i, &c) in d.oct.iter().enumerate() {
            if c > 0 {
                println!("  alpha in [2^-{}, 2^-{}): {:>10} ({:.3}%)", i + 1, i, c, 100.0 * c as f64 / k);
                oct.push(json!({"octave_lo_exp": -(i as i32) - 1, "count": c}));
            }
        }
        let mut abj = vec![];
        for (i, a) in d.a_bin.iter().enumerate() {
            if a.0 > 0 {
                println!("  |a| in [{}, {}): n {:>10} mean alpha {:.6} mean alpha(Q=0) {:.6}", A_EDGES[i], A_EDGES[i + 1], a.0, a.1 / a.0 as f64, a.2 / a.0 as f64);
                abj.push(json!({"lo": A_EDGES[i], "n": a.0, "mean_alpha": a.1 / a.0 as f64, "mean_alpha_no_q": a.2 / a.0 as f64}));
            }
        }
        let mut agj = vec![];
        for (i, a) in d.age_bin.iter().enumerate() {
            if a.0 > 0 {
                let lo = if i == 0 { 2 } else { AGE_EDGES[i - 1] };
                println!("  stream age >= {lo}: n {:>10} mean alpha {:.6} mean alpha(Q=0) {:.6}", a.0, a.1 / a.0 as f64, a.2 / a.0 as f64);
                agj.push(json!({"age_lo": lo, "n": a.0, "mean_alpha": a.1 / a.0 as f64, "mean_alpha_no_q": a.2 / a.0 as f64}));
            }
        }
        let segs: Vec<f64> = d.seg.iter().map(|a| (a.c[2] - a.c[1]) / n).collect();
        println!("  per-segment kalman-fixed d_bps: {}", segs.iter().map(|x| format!("{x:+.4}")).collect::<Vec<_>>().join(" "));
        out.push(json!({"id": id, "n": samples.len(), "decisions": d.n_dec,
            "bps": {"production": bps[0], "fixed_rp": bps[1], "kalman_rp": bps[2]},
            "pct": {"kalman_vs_fixed": pct(bps[2], bps[1]), "kalman_vs_production": pct(bps[2], bps[0]), "fixed_vs_production": pct(bps[1], bps[0])},
            "total_bits": tot, "fixed_raw_bits": d.fixed_bits,
            "rate": {"n": d.n_rate, "mean": d.sum / k, "min": d.min, "max": d.max, "zero": d.zero, "mean_no_q": d.sum_no_q / k, "octave_hist": oct, "by_abs_a": abj, "by_age": agj},
            "invalid_states": d.invalid, "segment_d_bps_kalman_minus_fixed": segs}));
    }
    let agg = [tc[0] / tn, tc[1] / tn, tc[2] / tn];
    println!(
        "AGGREGATE (total bits / total samples): production {:.5} fixed {:.5} kalman {:.5}; kalman vs fixed {:+.3}%, vs production {:+.3}%",
        agg[0],
        agg[1],
        agg[2],
        pct(agg[2], agg[1]),
        pct(agg[2], agg[0])
    );
    std::fs::write(&args[2], serde_json::to_vec_pretty(&json!({"images": out, "aggregate_bps": agg})).unwrap()).unwrap();
}
