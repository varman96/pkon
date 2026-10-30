//! LISA benchmark 1 runner: complete four-stage pipeline on frozen canonical samples, with an exact-decode gate and
//! diagnostics. Uses only the public `lisa` API; the algorithm is untouched.
//! Usage: bench_lisa1 <dataset_manifest.json> <out.json> [--only id]
use nanoscope::lisa::*;
use nanoscope::lisa_bayer_plane::{decode_stage1, encode_stage1, organize_stage2_with_mode, restore_stage1_with_mode, Stage1Mode};
use serde_json::{json, Value};
use std::time::Instant;

fn entropy(hist: &[u64], n: u64) -> f64 {
    if n == 0 {
        return 0.0;
    }
    hist.iter().filter(|&&c| c > 0).map(|&c| {
        let p = c as f64 / n as f64;
        -p * p.log2()
    }).sum()
}

/// (count, mean zigzag residual, zero fraction, order-0 entropy in bits/sample) of a residual list.
fn stats(res: impl Iterator<Item = u32> + Clone, bd: u8) -> Value {
    let cap = 1usize << (bd as usize + 1);
    let mut hist = vec![0u64; cap + 1];
    let (mut n, mut sum) = (0u64, 0u64);
    for v in res {
        hist[(v as usize).min(cap)] += 1;
        sum += v as u64;
        n += 1;
    }
    json!({"n": n, "mean_zz": if n > 0 { sum as f64 / n as f64 } else { 0.0 },
           "zero_frac": if n > 0 { hist[0] as f64 / n as f64 } else { 0.0 }, "h0_bits": entropy(&hist, n)})
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let manifest: Vec<Value> = serde_json::from_slice(&std::fs::read(&args[1]).unwrap()).unwrap();
    let only = args.iter().position(|a| a == "--only").map(|i| args[i + 1].clone());
    let mut out = Vec::new();
    for m in &manifest {
        let id = m["id"].as_str().unwrap();
        if only.as_deref().map_or(false, |o| o != id) {
            continue;
        }
        let (w, h) = (m["width"].as_u64().unwrap() as usize, m["height"].as_u64().unwrap() as usize);
        let bd = m["bit_depth"].as_u64().unwrap() as u8;
        let bytes = std::fs::read(m["canon_path"].as_str().unwrap()).unwrap();
        assert_eq!(bytes.len(), w * h * 2);
        let samples: Vec<u16> = bytes.chunks_exact(2).map(|c| u16::from_le_bytes([c[0], c[1]])).collect();

        let layout_and_meta = || -> ImageMetadata {
            let Some(cfa) = m["cfa"].as_array().filter(|a| !a.is_empty()) else {
                return ImageMetadata::without_cfa(w, h, SampleLayout::Monochrome);
            };
            let dim: Vec<usize> = m["cfa_dim"].as_array().unwrap().iter().map(|v| v.as_u64().unwrap() as usize).collect();
            let org: Vec<usize> = m["cfa_origin"].as_array().unwrap().iter().map(|v| v.as_u64().unwrap() as usize).collect();
            let entries: Vec<ColorFilter> =
                cfa.iter().map(|v| ColorFilter::try_from(v.as_u64().unwrap() as u8).unwrap()).collect();
            let pat = CfaPattern { width: dim[1], height: dim[0], entries, origin_x: org[0], origin_y: org[1] };
            let bayer = ImageMetadata::with_cfa(w, h, SampleLayout::BayerCfa, pat.clone());
            if (dim[0], dim[1]) == (2, 2) && bayer.validate().is_ok() {
                bayer
            } else {
                ImageMetadata::with_cfa(w, h, SampleLayout::OtherCfa, pat)
            }
        };
        let meta = layout_and_meta();

        let t = Instant::now();
        let s1 = encode_stage1(&samples, w, h, bd, meta.clone(), Stage1Mode::default()).expect("stage1");
        let t_s1 = t.elapsed().as_secs_f64();
        let s1_stats = stats(s1.residuals.iter().copied(), bd);
        let t = Instant::now();
        let s2 = organize_stage2_with_mode(s1, Stage1Mode::default()).expect("stage2");
        let t_s2 = t.elapsed().as_secs_f64();
        let t = Instant::now();
        let s3 = classify_stage3_with(s2, Stage3Rule::Legacy4).expect("stage3");
        let t_s3 = t.elapsed().as_secs_f64();

        // Diagnostics on Stage 2/3 data (read-only).
        let mode = match s3.mode() {
            Some(Stage2Mode::BayerPlanes { .. }) => "bayer_planes",
            Some(Stage2Mode::Identity) => "identity_morton",
            None => "unknown",
        };
        let mut class_hist: [Vec<u64>; 4] = std::array::from_fn(|_| vec![0u64; (1usize << (bd as usize + 1)) + 1]);
        let mut class_n = [0u64; 4];
        let mut class_sum = [0u64; 4];
        let mut plane_stats = Vec::new();
        {
            let cap = 1usize << (bd as usize + 1);
            let mut tally = |res: &[u32], ctx: &[u8]| {
                for (&v, c) in res.iter().zip(ctx) {
                    let k = usize::from(*c);
                    class_hist[k][(v as usize).min(cap)] += 1;
                    class_n[k] += 1;
                    class_sum[k] += v as u64;
                }
            };
            match (s3.data(), s3.contexts()) {
                (OrganizedResiduals::Morton(r), OrganizedContexts::Morton(c)) => tally(r, c),
                (OrganizedResiduals::BayerPlanes(r), OrganizedContexts::BayerPlanes(c)) => {
                    for (r, c) in r.iter().zip(c) {
                        tally(r, c);
                    }
                    for r in r.iter() {
                        plane_stats.push(stats(r.iter().copied(), bd));
                    }
                }
                _ => unreachable!(),
            }
        }
        let total: u64 = class_n.iter().sum();
        let mut classes = Vec::new();
        let mut static_bits = 0f64;
        for k in 0..4 {
            let hk = entropy(&class_hist[k], class_n[k]);
            static_bits += hk * class_n[k] as f64;
            let cname = ["Low", "Moderate", "High", "VeryHigh"][k];
            classes.push(json!({"class": cname, "n": class_n[k],
                "frac": class_n[k] as f64 / total as f64,
                "mean_zz": if class_n[k] > 0 { class_sum[k] as f64 / class_n[k] as f64 } else { 0.0 }, "h0_bits": hk}));
        }

        let t = Instant::now();
        let s4 = encode_stage4(s3.clone()).expect("stage4");
        let t_s4 = t.elapsed().as_secs_f64();
        let payload = s4.payload().len();

        // Correctness gate: decode from the Stage 4 output only, and compare against the original samples.
        let t = Instant::now();
        let back3 = decode_stage4(&s4).expect("decode stage4");
        let t_d4 = t.elapsed().as_secs_f64();
        let eq3 = back3 == s3;
        let t = Instant::now();
        let (s1b, mode) = restore_stage1_with_mode(&back3.into_stage2()).expect("restore stage1");
        let dec = decode_stage1(&s1b, mode).expect("decode stage1");
        let t_d1 = t.elapsed().as_secs_f64();
        let exact = dec == samples && s1b.metadata == meta && s1b.metadata.width == w && s1b.metadata.height == h;
        let first_bad = dec.iter().zip(&samples).position(|(a, b)| a != b);

        let n = (w * h) as f64;
        println!("{id}: {mode:?} payload {payload} B ({:.3} bpp) exact={exact} enc {:.1}s dec {:.1}s", payload as f64 * 8.0 / n,
            t_s1 + t_s2 + t_s3 + t_s4, t_d4 + t_d1);
        out.push(json!({
            "id": id, "width": w, "height": h, "bit_depth": bd, "canon_bytes": bytes.len(),
            "layout": format!("{:?}", meta.layout), "stage2_mode": format!("{:?}", mode),
            "payload_bytes": payload, "bits_per_sample": payload as f64 * 8.0 / n,
            "exact": exact, "stage3_roundtrip_equal": eq3, "first_mismatch": first_bad,
            "stage1": s1_stats, "planes": plane_stats, "classes": classes,
            "class_static_h0_bytes": static_bits / 8.0,
            "time_s": {"stage1": t_s1, "stage2": t_s2, "stage3": t_s3, "stage4": t_s4, "dec_stage4": t_d4, "dec_restore": t_d1},
            "coding_version": STAGE4_CODING_VERSION, "stage3_rule_version": STAGE3_RULE_VERSION,
        }));
    }
    std::fs::write(&args[2], serde_json::to_vec_pretty(&out).unwrap()).unwrap();
}
