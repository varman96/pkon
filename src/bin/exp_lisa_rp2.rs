//! Experiment runner: production vs fixed-rate RP vs second-difference RP, lockstep ideal cost + diagnostics.
//! Usage: exp_lisa_rp2 <dataset_manifest.json> <out.json> <id> [<id>...]
use nanoscope::lisa::*;
use nanoscope::lisa_rp2::*;
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

/// Per-decision means plus bps deltas (fixed-prod, dyn-prod, dyn-fixed), as text and JSON.
fn row(a: &Acc, n: f64) -> (String, Value) {
    let s = format!(
        "dec {:>10} prod/dec {:.4} fixed/dec {:.4} dyn/dec {:.4} | d_bps fixed-prod {:+.5} dyn-prod {:+.5} dyn-fixed {:+.5}",
        a.n, a.c[0] / a.n, a.c[1] / a.n, a.c[2] / a.n, (a.c[1] - a.c[0]) / n, (a.c[2] - a.c[0]) / n, (a.c[2] - a.c[1]) / n
    );
    let j = json!({"decisions": a.n, "bits": a.c, "d_bps_fixed_vs_prod": (a.c[1]-a.c[0])/n,
        "d_bps_dyn_vs_prod": (a.c[2]-a.c[0])/n, "d_bps_dyn_vs_fixed": (a.c[2]-a.c[1])/n});
    (s, j)
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
        let s3 = classify_stage3(organize_stage2(encode_with_metadata(&samples, w, h, bd, meta_of(m, w, h)).unwrap()).unwrap()).unwrap();
        let d = diagnose(&s3, samples.len());
        let n = samples.len() as f64;

        // integrity: identical symbol sequences, identical reset count
        let t = &d.trace;
        let identical = t[0].n == t[1].n
            && t[1].n == t[2].n
            && t[0].ones == t[1].ones
            && t[1].ones == t[2].ones
            && t[0].hash == t[1].hash
            && t[1].hash == t[2].hash
            && t[0].n == d.n_dec
            && d.resets[0] == d.resets[1]
            && d.resets[1] == d.resets[2];
        assert!(identical, "estimators saw different symbol sequences or reset points");

        let tot: [f64; 3] = std::array::from_fn(|i| d.class.iter().map(|a| a.c[i]).sum::<f64>() + d.fixed_bits);
        let bps: Vec<f64> = tot.iter().map(|x| x / n).collect();
        println!(
            "== {id}: n={} decisions/sample {:.2} fixed-bit share {:.3} bps; integrity OK (n={}, ones={}, hash={:016x})",
            samples.len(), d.n_dec as f64 / n, d.fixed_bits / n, t[0].n, t[0].ones, t[0].hash
        );
        println!("TOTAL ideal bits: prod {:.0} fixed {:.0} dyn {:.0}", tot[0], tot[1], tot[2]);
        println!("BPS: prod {:.5} fixed {:.5} dyn {:.5}", bps[0], bps[1], bps[2]);
        println!(
            "PCT: fixed vs prod {:+.3}% | dyn vs prod {:+.3}% | dyn vs fixed {:+.3}%",
            pct(tot[1], tot[0]), pct(tot[2], tot[0]), pct(tot[2], tot[1])
        );

        let r = &d.rate;
        let nn = r.n as f64;
        println!(
            "RATE (dynamic decisions {} of {}; fixed 1/32 warm-up decisions {}): mean {:.5} min {:.5} max {:.5}",
            r.n, d.n_dec, d.warmup.n, r.sum / nn, r.min, r.max
        );
        println!(
            "  quantiles (from {HIST}-bin histogram): p01 {:.4} p10 {:.4} p25 {:.4} p50 {:.4} p75 {:.4} p90 {:.4} p99 {:.4}",
            r.quantile(0.01), r.quantile(0.1), r.quantile(0.25), r.quantile(0.5), r.quantile(0.75), r.quantile(0.9), r.quantile(0.99)
        );
        println!(
            "  reporting bins on u=(alpha-amin)/(amax-amin): near-min u<{U_NEAR_MIN} | mid | near-max u>{U_NEAR_MAX}: {:.2}% / {:.2}% / {:.2}%",
            100.0 * r.near_min as f64 / nn, 100.0 * r.mid as f64 / nn, 100.0 * r.near_max as f64 / nn
        );
        let (mut jplane, mut jclass, mut jseg, mut jpbin, mut jkind, mut jabin) = (vec![], vec![], vec![], vec![], vec![], vec![]);
        for (i, a) in d.plane.iter().enumerate() {
            let (s, j) = row(a, n);
            println!("plane {i}: {s}");
            jplane.push(j);
        }
        for (i, a) in d.class.iter().enumerate() {
            let (s, mut j) = row(a, n);
            let rc = &d.rate_class[i];
            let k = rc.n.max(1) as f64;
            println!("class {i}: {s} | dyn-rate mean {:.4} near-min {:.1}% near-max {:.1}%", rc.sum / k, 100.0 * rc.near_min as f64 / k, 100.0 * rc.near_max as f64 / k);
            j["rate_mean"] = json!(rc.sum / k);
            j["rate_near_min_frac"] = json!(rc.near_min as f64 / k);
            j["rate_near_max_frac"] = json!(rc.near_max as f64 / k);
            jclass.push(j);
        }
        for (i, a) in d.seg.iter().enumerate() {
            let (s, j) = row(a, n);
            println!("seg {i:>2}: {s}");
            jseg.push(j);
        }
        for (i, a) in d.kind.iter().enumerate() {
            let (s, j) = row(a, n);
            println!("{}: {s}", if i == 0 { "unary" } else { "mant " });
            jkind.push(j);
        }
        println!("by production probability of the observed outcome (bins <0.1,<0.3,<0.7,<0.9,<0.99,>=0.99):");
        for (i, a) in d.pbin.iter().enumerate() {
            let (s, j) = row(a, n);
            println!("obs-p-bin {i}: {s}");
            jpbin.push(j);
        }
        println!("by |second difference| (edges 0,.01,.1,.5,1,2,5,10,inf), dynamic decisions only:");
        for (i, (a, sa)) in d.abin.iter().enumerate() {
            if a.n == 0.0 {
                continue;
            }
            let (s, mut j) = row(a, n);
            println!("|a|-bin [{}, {}): mean alpha {:.4} {s}", A_EDGES[i], A_EDGES[i + 1], sa / a.n);
            j["mean_alpha"] = json!(sa / a.n);
            j["abs_a_lo"] = json!(A_EDGES[i]);
            jabin.push(j);
        }
        if d.warmup.n > 0.0 {
            println!("warm-up (fixed-rate) decisions: {}", row(&d.warmup, n).0);
        }
        println!("dynamic-rate decisions: {}", row(&d.dynamic, n).0);

        let mut rows: Vec<(usize, usize, usize, &Acc)> = vec![];
        for c in 0..4 {
            for k in 0..2 {
                for s in 0..=33 {
                    let a = &d.slot[slot_index(c, k, s)];
                    if a.n > 0.0 {
                        rows.push((c, k, s, a));
                    }
                }
            }
        }
        rows.sort_by(|a, b| (b.3.c[2] - b.3.c[1]).abs().partial_cmp(&(a.3.c[2] - a.3.c[1]).abs()).unwrap());
        let mut jslots = vec![];
        for r in rows.iter().take(12) {
            let a = r.3;
            println!(
                "  cls {} {} slot {:>2}: n {:>9} | d_bps dyn-fixed {:+.5} dyn-prod {:+.5} fixed-prod {:+.5}",
                r.0, if r.1 == 0 { "unary" } else { "mant " }, r.2, a.n, (a.c[2] - a.c[1]) / n, (a.c[2] - a.c[0]) / n, (a.c[1] - a.c[0]) / n
            );
            jslots.push(json!({"class": r.0, "kind": r.1, "slot": r.2, "n": a.n, "d_bps_dyn_vs_fixed": (a.c[2]-a.c[1])/n,
                "d_bps_dyn_vs_prod": (a.c[2]-a.c[0])/n, "d_bps_fixed_vs_prod": (a.c[1]-a.c[0])/n}));
        }
        let mut jun = vec![];
        for s in 1..=33 {
            let mut a = Acc::default();
            for c in 0..4 {
                let x = &d.slot[slot_index(c, 0, s)];
                for i in 0..3 {
                    a.c[i] += x.c[i];
                }
                a.n += x.n;
            }
            if a.n > 0.0 {
                println!(
                    "unary slot {s:>2}: n {:>10} d_bps dyn-fixed {:+.5} dyn-prod {:+.5} fixed-prod {:+.5}",
                    a.n, (a.c[2] - a.c[1]) / n, (a.c[2] - a.c[0]) / n, (a.c[1] - a.c[0]) / n
                );
                jun.push(json!({"slot": s, "n": a.n, "d_bps_dyn_vs_fixed": (a.c[2]-a.c[1])/n,
                    "d_bps_dyn_vs_prod": (a.c[2]-a.c[0])/n, "d_bps_fixed_vs_prod": (a.c[1]-a.c[0])/n}));
            }
        }
        out.push(json!({"id": id, "n": samples.len(), "decisions": d.n_dec, "fixed_bits": d.fixed_bits,
            "integrity": {"identical_sequences": identical, "decisions": t[0].n, "ones": t[0].ones, "hash": format!("{:016x}", t[0].hash)},
            "total_bits": {"production": tot[0], "fixed_rp": tot[1], "second_diff_rp": tot[2]},
            "bps": {"production": bps[0], "fixed_rp": bps[1], "second_diff_rp": bps[2]},
            "pct": {"fixed_vs_prod": pct(tot[1], tot[0]), "dyn_vs_prod": pct(tot[2], tot[0]), "dyn_vs_fixed": pct(tot[2], tot[1])},
            "rate": {"dynamic_decisions": r.n, "warmup_decisions": d.warmup.n, "mean": r.sum / nn, "min": r.min, "max": r.max,
                "quantiles": {"p01": r.quantile(0.01), "p10": r.quantile(0.1), "p25": r.quantile(0.25), "p50": r.quantile(0.5),
                    "p75": r.quantile(0.75), "p90": r.quantile(0.9), "p99": r.quantile(0.99)},
                "bins_u": {"near_min_lt": U_NEAR_MIN, "near_max_gt": U_NEAR_MAX, "near_min_frac": r.near_min as f64 / nn,
                    "mid_frac": r.mid as f64 / nn, "near_max_frac": r.near_max as f64 / nn},
                "histogram_100bins_alpha_min_to_max": r.hist.to_vec()},
            "by_plane": jplane, "by_class": jclass, "by_segment": jseg, "by_kind": jkind, "by_prod_prob_bin": jpbin,
            "by_abs_second_diff": jabin, "by_unary_slot": jun, "top_slots_dyn_vs_fixed": jslots}));
    }
    std::fs::write(&args[2], serde_json::to_vec_pretty(&out).unwrap()).unwrap();
}
