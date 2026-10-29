//! Residual histogram / zero-order entropy measurement: Bayer-plane Stage 1 (production default) vs legacy mosaic Stage 1.
//! Residuals are read from the unmodified pipeline (`encode_stage2_with_mode` output, which is exactly what Stage 3/4 code),
//! un-zigzagged to their signed values. Stage 4 payload bytes come from the unmodified `encode_stage4` (OR-128 default rule).
//! Usage: exp_residual_entropy <manifest.json> <split.json> <out_dir>
use nanoscope::lisa::*;
use nanoscope::lisa_bayer_plane::{encode_stage2_with_mode, Stage1Mode};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::fmt::Write as _;

type Hist = BTreeMap<i32, u64>;

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

fn signed_hist(d: &OrganizedResiduals) -> Hist {
    let mut h = Hist::new();
    let mut add = |v: &[u32]| v.iter().for_each(|&u| *h.entry(unzigzag(u)).or_default() += 1);
    match d {
        OrganizedResiduals::Morton(v) => add(v),
        OrganizedResiduals::BayerPlanes(p) => p.iter().for_each(|v| add(v)),
    }
    h
}

/// Value at 0-indexed rank k in the sorted multiset.
fn kth(h: &Hist, k: u64) -> i32 {
    let mut c = 0;
    for (&v, &n) in h {
        c += n;
        if c > k {
            return v;
        }
    }
    unreachable!()
}

fn median(h: &Hist) -> f64 {
    let n: u64 = h.values().sum();
    (kth(h, (n - 1) / 2) as f64 + kth(h, n / 2) as f64) / 2.0
}

fn entropy(h: &Hist) -> f64 {
    let n: u64 = h.values().sum();
    -h.values().map(|&c| { let p = c as f64 / n as f64; p * p.log2() }).sum::<f64>()
}

fn stats(h: &Hist) -> Value {
    let n: u64 = h.values().sum();
    let nf = n as f64;
    let mean = h.iter().map(|(&v, &c)| v as f64 * c as f64).sum::<f64>() / nf;
    let var = h.iter().map(|(&v, &c)| (v as f64 - mean).powi(2) * c as f64).sum::<f64>() / nf;
    let mut ah = Hist::new();
    for (&v, &c) in h {
        *ah.entry(v.abs()).or_default() += c;
    }
    let mean_abs = ah.iter().map(|(&v, &c)| v as f64 * c as f64).sum::<f64>() / nf;
    let pct = |f: &dyn Fn(i32) -> bool| h.iter().filter(|(&v, _)| f(v)).map(|(_, &c)| c).sum::<u64>() as f64 * 100.0 / nf;
    json!({
        "samples": n, "min": *h.keys().next().unwrap(), "max": *h.keys().last().unwrap(), "mean": mean, "std": var.sqrt(),
        "median": median(h), "mean_abs": mean_abs, "median_abs": median(&ah), "max_abs": *ah.keys().last().unwrap(),
        "zero_pct": pct(&|v| v == 0), "positive_pct": pct(&|v| v > 0), "negative_pct": pct(&|v| v < 0),
        "distinct_values": h.len(), "entropy_bits": entropy(h),
    })
}

struct Cfg {
    hist: Hist,
    stats: Value,
    payload: usize,
    deterministic: bool,
    s3_matches_s2: bool,
    count_ok: bool,
}

fn measure(samples: &[u16], w: usize, h: usize, bd: u8, meta: &ImageMetadata, mode: Stage1Mode) -> Cfg {
    let s2 = encode_stage2_with_mode(samples, w, h, bd, meta.clone(), mode).unwrap();
    let hist = signed_hist(s2.data());
    let s2b = encode_stage2_with_mode(samples, w, h, bd, meta.clone(), mode).unwrap();
    let s3 = classify_stage3_with(s2.clone(), Stage3Rule::default()).unwrap();
    let s3_matches_s2 = s3.data() == s2.data();
    let s4 = encode_stage4(s3.clone()).unwrap();
    let s4b = encode_stage4(classify_stage3_with(s2b.clone(), Stage3Rule::default()).unwrap()).unwrap();
    let deterministic = s2 == s2b && signed_hist(s2b.data()) == hist && s4 == s4b && s4.payload() == s4b.payload();
    let count_ok = hist.values().sum::<u64>() == (w * h) as u64;
    Cfg { stats: stats(&hist), hist, payload: s4.payload().len(), deterministic, s3_matches_s2, count_ok }
}

fn main() {
    let a: Vec<String> = std::env::args().collect();
    let manifest: Value = serde_json::from_slice(&std::fs::read(&a[1]).unwrap()).unwrap();
    let split: Value = serde_json::from_slice(&std::fs::read(&a[2]).unwrap()).unwrap();
    let out_dir = &a[3];
    std::fs::create_dir_all(out_dir).unwrap();

    let mut rows: Vec<Value> = vec![];
    let mut hist_csv = String::from("image,split,config,residual,count\n");
    let mut pooled: BTreeMap<(&str, &str), Hist> = BTreeMap::new(); // (subset, config)
    let mut totals: BTreeMap<(&str, &str), (u64, u64)> = BTreeMap::new(); // (bytes, pixels)
    let mut ent_sum: BTreeMap<(&str, &str), (f64, u32)> = BTreeMap::new();
    let mut all_ok = true;

    for s in ["train", "val", "test"] {
        for id in split[s].as_array().unwrap() {
            let m = manifest.as_array().unwrap().iter().find(|m| m["id"] == *id).expect("id not in manifest");
            let id = id.as_str().unwrap();
            let (w, h) = (m["width"].as_u64().unwrap() as usize, m["height"].as_u64().unwrap() as usize);
            let bd = m["bit_depth"].as_u64().unwrap() as u8;
            let bytes = std::fs::read(m["canon_path"].as_str().unwrap()).unwrap();
            let sha_ok = format!("{:x}", Sha256::digest(&bytes)) == m["canon_sha256"].as_str().unwrap() && bytes.len() == w * h * 2;
            let samples: Vec<u16> = bytes.chunks_exact(2).map(|c| u16::from_le_bytes([c[0], c[1]])).collect();
            drop(bytes);
            let meta = meta_of(m, w, h);
            let px = (w * h) as u64;
            let mut row = json!({"id": id, "split": s, "width": w, "height": h, "pixels": px, "bit_depth": bd,
                                 "layout": format!("{:?}", meta.layout), "canon_sha_ok": sha_ok});
            let mut line = format!("{id} [{s}]");
            for (name, mode) in [("bayer", Stage1Mode::BayerPlanar), ("mosaic", Stage1Mode::Mosaic)] {
                let c = measure(&samples, w, h, bd, &meta, mode);
                let ok = c.deterministic && c.s3_matches_s2 && c.count_ok;
                all_ok &= ok && sha_ok;
                let ent = c.stats["entropy_bits"].as_f64().unwrap();
                let bpp = c.payload as f64 * 8.0 / px as f64;
                line += &format!(" | {name}: H0 {ent:.4} b/res, {} B ({bpp:.4} bpp) ok={ok}", c.payload);
                for (&v, &n) in &c.hist {
                    writeln!(hist_csv, "{id},{s},{name},{v},{n}").unwrap();
                }
                let subsets: &[&str] = if s == "test" { &["all", "heldout"] } else { &["all"] };
                for &sub in subsets {
                    let p = pooled.entry((sub, name)).or_default();
                    c.hist.iter().for_each(|(&v, &n)| *p.entry(v).or_default() += n);
                    let t = totals.entry((sub, name)).or_default();
                    t.0 += c.payload as u64;
                    t.1 += px;
                    let e = ent_sum.entry((sub, name)).or_default();
                    e.0 += ent;
                    e.1 += 1;
                }
                row[name] = json!({"stage1_mode_applied": format!("{:?}", mode.effective(&meta)), "residuals": c.stats, "stage4_payload_bytes": c.payload,
                                   "stage4_bpp": bpp, "deterministic": c.deterministic, "stage3_data_equals_stage2_data": c.s3_matches_s2,
                                   "sample_count_equals_pixels": c.count_ok});
            }
            println!("{line}");
            rows.push(row);
        }
    }

    let mut agg = serde_json::Map::new();
    let mut md = String::new();
    for sub in ["all", "heldout"] {
        let mut o = serde_json::Map::new();
        for cfg in ["bayer", "mosaic"] {
            let (b, p) = totals[&(sub, cfg)];
            let (es, en) = ent_sum[&(sub, cfg)];
            o.insert(cfg.into(), json!({"images": en, "pixels": p, "stage4_payload_bytes": b, "stage4_bpp": b as f64 * 8.0 / p as f64,
                "pooled_residuals": stats(&pooled[&(sub, cfg)]), "per_image_mean_entropy_bits": es / en as f64}));
        }
        let (bb, mb) = (totals[&(sub, "bayer")].0 as f64, totals[&(sub, "mosaic")].0 as f64);
        o.insert("payload_change_pct_bayer_vs_mosaic".into(), json!((bb - mb) / mb * 100.0));
        agg.insert(sub.into(), Value::Object(o));
    }

    writeln!(md, "# Residual histogram and zero-order entropy: Bayer-plane vs mosaic Stage 1\n").unwrap();
    writeln!(md, "Generated by `src/bin/exp_residual_entropy.rs`. Residuals = Stage 2 output data (identical to Stage 3 data; checked), un-zigzagged to signed values. Payload = `Stage4Output::payload().len()` (range-coder bytes only, no container/metadata), Stage 3 rule = default OR-128, Stage 4 = default. bpp = payload bytes*8 / (width*height). H0 in bits per residual sample (pooled = counts summed over images first).\n").unwrap();
    writeln!(md, "## Aggregate\n").unwrap();
    writeln!(md, "| subset | config | images | pixels | payload B | bpp | pooled H0 | mean per-image H0 |\n|---|---|---|---|---|---|---|---|").unwrap();
    for sub in ["all", "heldout"] {
        for cfg in ["bayer", "mosaic"] {
            let o = &agg[sub][cfg];
            writeln!(md, "| {sub} | {cfg} | {} | {} | {} | {:.5} | {:.5} | {:.5} |", o["images"], o["pixels"], o["stage4_payload_bytes"],
                o["stage4_bpp"].as_f64().unwrap(), o["pooled_residuals"]["entropy_bits"].as_f64().unwrap(), o["per_image_mean_entropy_bits"].as_f64().unwrap()).unwrap();
        }
        writeln!(md, "| {sub} | Bayer vs mosaic payload change | | | {:+.4}% | | | |", agg[sub]["payload_change_pct_bayer_vs_mosaic"].as_f64().unwrap()).unwrap();
    }
    writeln!(md, "\n## Pooled signed residual statistics\n").unwrap();
    writeln!(md, "| subset | config | samples | min | max | mean | std | median | mean abs | median abs | max abs | zero % | pos % | neg % | distinct |\n|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|").unwrap();
    for sub in ["all", "heldout"] {
        for cfg in ["bayer", "mosaic"] {
            let r = &agg[sub][cfg]["pooled_residuals"];
            let f = |k: &str| r[k].as_f64().unwrap();
            writeln!(md, "| {sub} | {cfg} | {} | {} | {} | {:.4} | {:.4} | {} | {:.4} | {} | {} | {:.4} | {:.4} | {:.4} | {} |", r["samples"], r["min"], r["max"],
                f("mean"), f("std"), f("median"), f("mean_abs"), f("median_abs"), r["max_abs"], f("zero_pct"), f("positive_pct"), f("negative_pct"), r["distinct_values"]).unwrap();
        }
    }
    writeln!(md, "\n## Per image\n").unwrap();
    writeln!(md, "| image | split | pixels | config | min | max | mean | std | median | mean abs | median abs | max abs | zero % | pos % | neg % | H0 | payload B | bpp |\n|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|").unwrap();
    for r in &rows {
        for cfg in ["bayer", "mosaic"] {
            let s = &r[cfg]["residuals"];
            let f = |k: &str| s[k].as_f64().unwrap();
            writeln!(md, "| {} | {} | {} | {cfg} | {} | {} | {:.4} | {:.4} | {} | {:.4} | {} | {} | {:.3} | {:.3} | {:.3} | {:.5} | {} | {:.5} |",
                r["id"].as_str().unwrap(), r["split"].as_str().unwrap(), r["pixels"], s["min"], s["max"], f("mean"), f("std"), f("median"), f("mean_abs"),
                f("median_abs"), s["max_abs"], f("zero_pct"), f("positive_pct"), f("negative_pct"), f("entropy_bits"), r[cfg]["stage4_payload_bytes"],
                r[cfg]["stage4_bpp"].as_f64().unwrap()).unwrap();
        }
    }
    std::fs::write(format!("{out_dir}/report_tables.md"), md).unwrap();
    std::fs::write(format!("{out_dir}/histograms.csv"), hist_csv).unwrap();
    std::fs::write(format!("{out_dir}/results.json"), serde_json::to_vec_pretty(&json!({"aggregate": agg, "images": rows, "all_ok": all_ok})).unwrap()).unwrap();
    println!("ALL_OK={all_ok}");
    if !all_ok {
        std::process::exit(1);
    }
}
