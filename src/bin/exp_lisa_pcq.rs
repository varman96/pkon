//! Experiment runner for the Predictive Context Quantizer (Stage 3 research).
//!   exp_lisa_pcq split   <manifest.json>
//!   exp_lisa_pcq stats   <manifest.json> <statsdir>                      (all images; writes <id>.stats)
//!   exp_lisa_pcq train   <manifest.json> <statsdir> <outdir>             (train/val stats only; writes maps + analysis)
//!   exp_lisa_pcq eval    <manifest.json> <out.json> <map[,map...]> <id>...  (map = file path of a KEYS-byte table)
use nanoscope::lisa::*;
use nanoscope::lisa_pcq::*;
use serde_json::{json, Value};
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

fn timed<T>(reps: usize, mut f: impl FnMut() -> T) -> (T, f64) {
    let mut best = f64::MAX;
    let mut out = None;
    for _ in 0..reps {
        let t = Instant::now();
        out = Some(f());
        best = best.min(t.elapsed().as_secs_f64());
    }
    (out.unwrap(), best)
}

fn load(manifest: &Value, id: &str) -> (Vec<u16>, Stage3Output, u8) {
    let m = manifest.as_array().unwrap().iter().find(|m| m["id"].as_str() == Some(id)).expect("id not in manifest");
    let (w, h) = (m["width"].as_u64().unwrap() as usize, m["height"].as_u64().unwrap() as usize);
    let bd = m["bit_depth"].as_u64().unwrap() as u8;
    let bytes = std::fs::read(m["canon_path"].as_str().unwrap()).unwrap();
    let samples: Vec<u16> = bytes.chunks_exact(2).map(|c| u16::from_le_bytes([c[0], c[1]])).collect();
    let s1 = encode_with_metadata(&samples, w, h, bd, meta_of(m, w, h)).unwrap();
    (samples, classify_stage3(organize_stage2(s1).unwrap()).unwrap(), bd)
}

/// Deterministic split by sorted id: every 3rd (index%3==2) is TEST; of the rest, every 4th (index%4==3) is VAL.
fn split(manifest: &Value) -> (Vec<String>, Vec<String>, Vec<String>) {
    let mut ids: Vec<String> = manifest.as_array().unwrap().iter().map(|m| m["id"].as_str().unwrap().to_string()).collect();
    ids.sort();
    let (mut train, mut val, mut test, mut rest) = (vec![], vec![], vec![], vec![]);
    for (i, id) in ids.into_iter().enumerate() {
        if i % 3 == 2 { test.push(id) } else { rest.push(id) }
    }
    for (j, id) in rest.into_iter().enumerate() {
        if j % 4 == 3 { val.push(id) } else { train.push(id) }
    }
    (train, val, test)
}

fn read_stats(dir: &str, id: &str) -> Stats {
    let b = std::fs::read(format!("{dir}/{id}.stats")).unwrap();
    let v: Vec<u32> = b.chunks_exact(4).map(|c| u32::from_le_bytes(c.try_into().unwrap())).collect();
    let (a, b2) = (KEYS * E_BINS, 4 * E_BINS);
    Stats { pat: v[..a].to_vec(), base: v[a..a + b2].to_vec(), avg: v[a + b2..].to_vec() }
}

fn pool(dir: &str, ids: &[String]) -> Stats {
    let mut p = Stats { pat: vec![0; KEYS * E_BINS], base: vec![0; 4 * E_BINS], avg: vec![0; LEVELS * E_BINS] };
    for id in ids {
        let s = read_stats(dir, id);
        for (d, x) in p.pat.iter_mut().zip(&s.pat) { *d += x }
        for (d, x) in p.base.iter_mut().zip(&s.base) { *d += x }
        for (d, x) in p.avg.iter_mut().zip(&s.avg) { *d += x }
    }
    p
}

fn lv(k: usize) -> [usize; 3] {
    [k / (LEVELS * LEVELS), (k / LEVELS) % LEVELS, k % LEVELS]
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let manifest: Value = serde_json::from_slice(&std::fs::read(&args[2]).unwrap()).unwrap();
    match args[1].as_str() {
        "split" => {
            let (tr, va, te) = split(&manifest);
            println!("{}", serde_json::to_string_pretty(&json!({"train": tr, "val": va, "test": te})).unwrap());
        }
        "stats" => {
            std::fs::create_dir_all(&args[3]).unwrap();
            for m in manifest.as_array().unwrap() {
                let id = m["id"].as_str().unwrap();
                let t = Instant::now();
                let (_, s3, _) = load(&manifest, id);
                let st = collect(&s3);
                let mut b = Vec::new();
                for v in st.pat.iter().chain(&st.base).chain(&st.avg) { b.extend_from_slice(&v.to_le_bytes()) }
                std::fs::write(format!("{}/{id}.stats", args[3]), b).unwrap();
                println!("{id}: stats done in {:.1}s", t.elapsed().as_secs_f64());
            }
        }
        "train" => train(&manifest, &args[3], &args[4]),
        "eval" => eval(&manifest, &args[3], &args[4], &args[5..]),
        _ => panic!("unknown subcommand"),
    }
}

fn train(manifest: &Value, dir: &str, out: &str) {
    std::fs::create_dir_all(out).unwrap();
    let (tr, va, _te) = split(manifest);
    let (t, v) = (pool(dir, &tr), pool(dir, &va));
    let nt: u64 = t.base.iter().map(|&x| u64::from(x)).sum();
    let nv: u64 = v.base.iter().map(|&x| u64::from(x)).sum();
    println!("train samples {nt}, val samples {nv}");
    let mut rep = serde_json::Map::new();

    // ---- Exp 1: feature families, held-out cross-entropy of the target bit length (bits / sample).
    let fam = |name: &str, nctx: usize, f: &dyn Fn(usize) -> usize| -> Value {
        let (ta, va_) = (aggregate(&t.pat, nctx, f), aggregate(&v.pat, nctx, f));
        let used = (0..nctx).filter(|&r| ta[r * E_BINS..(r + 1) * E_BINS].iter().any(|&x| x > 0)).count();
        let (h_tr, h_va) = (xent(&ta, &ta), xent(&ta, &va_));
        println!("{name:<34} contexts {used:>6}  train {h_tr:.4}  held-out {h_va:.4}");
        json!({"name": name, "contexts_observed": used, "train_bps": h_tr, "heldout_bps": h_va})
    };
    let mut fams = Vec::new();
    let (hb_tr, hb_va) = (xent(&t.base, &t.base), xent(&t.base, &v.base));
    println!("{:<34} contexts {:>6}  train {hb_tr:.4}  held-out {hb_va:.4}", "BASELINE 4-class (production)", 4);
    fams.push(json!({"name": "baseline_4class", "contexts_observed": 4, "train_bps": hb_tr, "heldout_bps": hb_va}));
    let (ha_tr, ha_va) = (xent(&t.avg, &t.avg), xent(&t.avg, &v.avg));
    println!("{:<34} contexts {:>6}  train {ha_tr:.4}  held-out {ha_va:.4}", "average, half-octave levels", LEVELS);
    fams.push(json!({"name": "average_levels", "contexts_observed": LEVELS, "train_bps": ha_tr, "heldout_bps": ha_va}));
    let sorted = |k: usize| {
        let mut a = lv(k);
        a.sort_unstable_by(|x, y| y.cmp(x));
        a
    };
    let mx = |k: usize| *lv(k).iter().max().unwrap();
    let mn = |k: usize| *lv(k).iter().min().unwrap();
    fams.push(fam("max level only", LEVELS, &|k| mx(k)));
    fams.push(fam("min level only", LEVELS, &|k| mn(k)));
    fams.push(fam("W only", LEVELS, &|k| lv(k)[0]));
    fams.push(fam("W,N", LEVELS * LEVELS, &|k| lv(k)[0] * LEVELS + lv(k)[1]));
    fams.push(fam("mean of levels (rounded)", LEVELS, &|k| (lv(k).iter().sum::<usize>() + 1) / 3));
    fams.push(fam("(max,min)", LEVELS * LEVELS, &|k| mx(k) * LEVELS + mn(k)));
    fams.push(fam("(mean of levels, range)", LEVELS * LEVELS, &|k| ((lv(k).iter().sum::<usize>() + 1) / 3) * LEVELS + (mx(k) - mn(k))));
    fams.push(fam("sorted triple (order-free)", KEYS, &|k| {
        let s = sorted(k);
        (s[0] * LEVELS + s[1]) * LEVELS + s[2]
    }));
    fams.push(fam("(max level, argmax position)", LEVELS * 3, &|k| {
        let a = lv(k);
        let m = mx(k);
        m * 3 + a.iter().position(|&x| x == m).unwrap()
    }));
    fams.push(fam("ordered triple W,N,NW (full key)", KEYS, &|k| k));
    rep.insert("families".into(), Value::Array(fams));

    // ---- population of the full key space
    let keymass: Vec<u64> = (0..KEYS).map(|k| t.pat[k * E_BINS..(k + 1) * E_BINS].iter().map(|&x| u64::from(x)).sum()).collect();
    let seen = keymass.iter().filter(|&&m| m > 0).count();
    let mut sorted_mass: Vec<u64> = keymass.iter().copied().filter(|&m| m > 0).collect();
    sorted_mass.sort_unstable_by(|a, b| b.cmp(a));
    let (mut acc, mut k99) = (0u64, 0usize);
    for (i, &m) in sorted_mass.iter().enumerate() {
        acc += m;
        if acc as f64 >= 0.99 * nt as f64 {
            k99 = i + 1;
            break;
        }
    }
    let rare_mass: u64 = keymass.iter().filter(|&&m| m > 0 && m < 1000).sum();
    let rare_keys = keymass.iter().filter(|&&m| m > 0 && m < 1000).count();
    println!("keys observed {seen}/{KEYS}; keys holding 99% of train mass {k99}; keys with <1000 samples {rare_keys} hold {:.4}% of mass", 100.0 * rare_mass as f64 / nt as f64);
    rep.insert("population".into(), json!({"keys_total": KEYS, "keys_observed": seen, "keys_for_99pct_mass": k99, "rare_keys_lt1000": rare_keys, "rare_mass_frac": rare_mass as f64 / nt as f64}));

    // ---- motivating example: (0,0,12) vs (4,4,4) in zigzag terms
    let mut ex = Vec::new();
    for (name, p) in [("(0,0,12)", [0u32, 0, 12]), ("(4,4,4)", [4, 4, 4])] {
        let k = key(Some(p[0]), Some(p[1]), Some(p[2]));
        let row = &t.pat[k * E_BINS..(k + 1) * E_BINS];
        let n: f64 = row.iter().map(|&x| f64::from(x)).sum();
        let mean_e: f64 = row.iter().enumerate().map(|(e, &c)| f64::from(c) * (e as f64 + 1.0)).sum::<f64>() / n.max(1.0);
        let dist: Vec<f64> = row.iter().map(|&c| f64::from(c) / n.max(1.0)).collect();
        println!("example {name}: key {k} train n {n} mean bit-length {mean_e:.3}");
        ex.push(json!({"pattern": name, "key": k, "n": n, "mean_e": mean_e, "dist": dist}));
    }
    rep.insert("example".into(), Value::Array(ex));

    // ---- Exp 2: agglomerative quantizer, K sweep
    let cps = [2usize, 3, 4, 6, 8, 12, 16, 24, 32, 48, 64, 96, 128, 192, 256];
    let t0 = Instant::now();
    let maps = agglomerate(&t.pat, &cps);
    println!("agglomerate: {:.1}s", t0.elapsed().as_secs_f64());
    let mut sweep = Vec::new();
    for (k, table) in &maps {
        std::fs::write(format!("{out}/map_K{k}.bin"), table).unwrap();
        let f = |key: usize| table[key] as usize;
        let (ta, va_) = (aggregate(&t.pat, *k, f), aggregate(&v.pat, *k, f));
        let (h_tr, h_va) = (xent(&ta, &ta), xent(&ta, &va_));
        println!("PCQ K={k:<4} train {h_tr:.4}  held-out {h_va:.4}  (baseline held-out {hb_va:.4}, delta {:+.4} bps)", h_va - hb_va);
        sweep.push(json!({"k": k, "train_bps": h_tr, "heldout_bps": h_va, "delta_vs_baseline_heldout_bps": h_va - hb_va}));
    }
    rep.insert("sweep".into(), Value::Array(sweep));
    rep.insert("split".into(), json!({"train": tr, "val": va, "test": _te}));
    std::fs::write(format!("{out}/analysis.json"), serde_json::to_vec_pretty(&Value::Object(rep)).unwrap()).unwrap();
}

fn eval(manifest: &Value, out: &str, maps: &str, ids: &[String]) {
    // "frozen" selects the table compiled into the crate (lisa_pcq_table.rs).
    let tables: Vec<(String, Vec<u8>)> = maps
        .split(',')
        .map(|p| (p.to_string(), if p == "frozen" { nanoscope::lisa_pcq_table::PCQ_TABLE.to_vec() } else { std::fs::read(p).unwrap() }))
        .collect();
    for (p, t) in &tables {
        assert_eq!(t.len(), KEYS, "{p}: wrong table size");
    }
    let (mut tot_base, mut tot_n) = (0usize, 0usize);
    let mut tot_map = vec![0usize; tables.len()];
    let mut rows = Vec::new();
    for id in ids {
        let (samples, s3, bd) = load(manifest, id);
        let n = samples.len();
        let (s4, prod_enc_t) = timed(3, || encode_stage4(s3.clone()).unwrap());
        let (d3, prod_dec_t) = timed(3, || decode_stage4(&s4).unwrap());
        let (s1b, _) = restore_stage1(&d3.into_stage2()).unwrap();
        let prod_ok = decode_with_metadata(&s1b).unwrap() == samples;
        let base_bytes = s4.payload().len();

        // Control: baseline classifier through the experimental coder must reproduce production bytes.
        let base_ctx = |l, a, al| classify_activity(l, a, al).index();
        let (ctrl, ctrl_ids) = pcq_encode(&s3, 4, base_ctx);
        let (ctrl_res, ctrl_dids, ctrl_end) = pcq_decode(&ctrl, &s3, 4, base_ctx);
        let control_ok = ctrl == s4.payload() && ctrl_res == *s3.data() && ctrl_ids == ctrl_dids && ctrl_end;

        tot_base += base_bytes;
        tot_n += n;
        let mut per_map = Vec::new();
        for (mi, (path, table)) in tables.iter().enumerate() {
            let k = *table.iter().max().unwrap() as usize + 1;
            let ctx = |l, a, al| table[key(l, a, al)] as usize;
            let ((bytes, enc_ids), enc_t) = timed(3, || pcq_encode(&s3, k, ctx));
            let ((res, dec_ids, end), dec_t) = timed(3, || pcq_decode(&bytes, &s3, k, ctx));
            let lossless = res == *s3.data() && end;
            let ids_equal = enc_ids == dec_ids;
            let (_, again_ids_same) = {
                let (b2, i2) = pcq_encode(&s3, k, ctx);
                (b2.len(), b2 == bytes && i2 == enc_ids)
            };
            let pct = 100.0 * (bytes.len() as f64 - base_bytes as f64) / base_bytes as f64;
            println!(
                "{id} [{path}] K={k}: n={n} base {base_bytes} B ({:.5} bps) pcq {} B ({:.5} bps) {:+.4}% | lossless={lossless} ctx_ids_equal={ids_equal} deterministic={again_ids_same} | enc s base {prod_enc_t:.3} pcq {enc_t:.3} dec s base {prod_dec_t:.3} pcq {dec_t:.3} | prod_ok={prod_ok} control_ok={control_ok}",
                base_bytes as f64 * 8.0 / n as f64,
                bytes.len(),
                bytes.len() as f64 * 8.0 / n as f64,
                pct
            );
            tot_map[mi] += bytes.len();
            per_map.push(json!({"map": path, "k": k, "bytes": bytes.len(), "bps": bytes.len() as f64 * 8.0 / n as f64, "pct_change": pct,
                "lossless": lossless, "ctx_ids_equal": ids_equal, "deterministic": again_ids_same, "enc_s": enc_t, "dec_s": dec_t}));
        }
        rows.push(json!({"id": id, "bit_depth": bd, "n": n, "base_bytes": base_bytes, "base_bps": base_bytes as f64 * 8.0 / n as f64,
            "prod_lossless": prod_ok, "control_identical_to_production": control_ok,
            "base_enc_s": prod_enc_t, "base_dec_s": prod_dec_t, "maps": per_map}));
    }
    let mut agg = Vec::new();
    for (mi, (path, _)) in tables.iter().enumerate() {
        let pct = 100.0 * (tot_map[mi] as f64 - tot_base as f64) / tot_base as f64;
        println!("aggregate [{path}]: base {tot_base} B pcq {} B change {pct:+.4}% ({:.5} -> {:.5} bps)", tot_map[mi], tot_base as f64 * 8.0 / tot_n as f64, tot_map[mi] as f64 * 8.0 / tot_n as f64);
        agg.push(json!({"map": path, "bytes": tot_map[mi], "pct_change": pct}));
    }
    std::fs::write(out, serde_json::to_vec_pretty(&json!({"images": rows, "agg_base_bytes": tot_base, "agg": agg})).unwrap()).unwrap();
}
