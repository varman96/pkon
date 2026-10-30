//! OR-128 final signal diagnostic (read-only measurement; nothing here feeds back into any stage).
//! Reuses the production path (Stage 1 -> Stage 2 -> `classify_stage3_with(Or128Tree)`) to get residuals and leaf ids, and
//! a separate MED (LOCO-I / JPEG-LS median edge detector) residual on the same samples for the entropy comparison.
//!
//! Residual representation (all entropies): zigzag residual v, x = v + 1, e = bit length of x, m1 = bit below the leading
//! one (0 if e < 2). Modelled symbol s = (e, m1) (what Stage 4 adapts); the remaining max(e-2, 0) bits are coded at p = 1/2.
//! Bits per residual = H(s | context) + mean(max(e-2, 0)). Static empirical (plug-in) entropy, one image at a time.
//!
//!   diag_or128_signal <manifest.json> <split.json> <out_dir>
use nanoscope::lisa::*;
use nanoscope::lisa_bayer_plane::encode_stage2;
use serde_json::{json, Value};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;

const NS: usize = 80; // symbol alphabet bound: e <= 33 -> s = 2e + m1 < 68
const L: usize = 128;

#[inline]
fn sym(v: u32) -> (usize, u64) {
    let x = u64::from(v) + 1;
    let e = (64 - x.leading_zeros()) as usize;
    let m1 = if e >= 2 { ((x >> (e - 2)) & 1) as usize } else { 0 };
    (2 * e + m1, e.saturating_sub(2) as u64)
}

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

/// MED residual histogram with neighbours `stride` apart (1 = raw raster, 2 = same-colour Bayer plane).
fn med_hist(s: &[u16], w: usize, h: usize, bd: u8, stride: usize) -> ([u64; NS], u64) {
    let (mut hist, mut raw) = ([0u64; NS], 0u64);
    let mid = 1i32 << (bd - 1);
    for y in 0..h {
        for x in 0..w {
            let px = |xx: usize, yy: usize| i32::from(s[yy * w + xx]);
            let pred = match (x >= stride, y >= stride) {
                (false, false) => mid,
                (true, false) => px(x - stride, y),
                (false, true) => px(x, y - stride),
                (true, true) => {
                    let (a, b, c) = (px(x - stride, y), px(x, y - stride), px(x - stride, y - stride));
                    if c >= a.max(b) { a.min(b) } else if c <= a.min(b) { a.max(b) } else { a + b - c }
                }
            };
            let (k, r) = sym(zigzag(px(x, y) - pred));
            hist[k] += 1;
            raw += r;
        }
    }
    (hist, raw)
}

fn bits(hist: &[u64]) -> f64 {
    let n: u64 = hist.iter().sum();
    hist.iter().filter(|&&c| c > 0).map(|&c| c as f64 * (n as f64 / c as f64).log2()).sum()
}

struct Img {
    id: String,
    n: u64,
    raw: u64,
    h0: [u64; NS],
    leaf: Vec<[u64; NS]>,
    med: [u64; NS],
    med_raw: u64,
    med_pl: [u64; NS],
    med_pl_raw: u64,
    bayer: bool,
}

fn run(m: &Value) -> Img {
    let id = m["id"].as_str().unwrap().to_string();
    let (w, h) = (m["width"].as_u64().unwrap() as usize, m["height"].as_u64().unwrap() as usize);
    let bd = m["bit_depth"].as_u64().unwrap() as u8;
    let bytes = std::fs::read(m["canon_path"].as_str().unwrap()).unwrap();
    let samples: Vec<u16> = bytes.chunks_exact(2).map(|c| u16::from_le_bytes([c[0], c[1]])).collect();
    drop(bytes);
    let meta = meta_of(m, w, h);
    let bayer = meta.layout == SampleLayout::BayerCfa;
    let s3 = classify_stage3_with(encode_stage2(&samples, w, h, bd, meta).unwrap(), Stage3Rule::Or128Tree).unwrap();
    assert_eq!(s3.rule(), Some(Stage3Rule::Or128Tree));
    let pairs: Vec<(&[u32], &[u8])> = match (s3.data(), s3.contexts()) {
        (OrganizedResiduals::Morton(r), OrganizedContexts::Morton(c)) => vec![(&r[..], &c[..])],
        (OrganizedResiduals::BayerPlanes(r), OrganizedContexts::BayerPlanes(c)) => (0..4).map(|i| (&r[i][..], &c[i][..])).collect(),
        _ => panic!("layout mismatch"),
    };
    let (mut h0, mut leaf, mut raw, mut n) = ([0u64; NS], vec![[0u64; NS]; L], 0u64, 0u64);
    for (r, c) in pairs {
        assert_eq!(r.len(), c.len());
        for (&v, &k) in r.iter().zip(c) {
            let (s, rb) = sym(v);
            h0[s] += 1;
            leaf[usize::from(k)][s] += 1;
            raw += rb;
            n += 1;
        }
    }
    assert_eq!(n as usize, w * h);
    let (med, med_raw) = med_hist(&samples, w, h, bd, 1);
    let (med_pl, med_pl_raw) = if bayer { med_hist(&samples, w, h, bd, 2) } else { (med, med_raw) };
    eprintln!("done {id} n={n}");
    Img { id, n, raw, h0, leaf, med, med_raw, med_pl, med_pl_raw, bayer }
}

fn add(a: &mut [u64; NS], b: &[u64; NS]) {
    for (x, y) in a.iter_mut().zip(b) {
        *x += y;
    }
}

/// Cross-entropy (bits) of `test` symbols under the add-1/2 smoothed distribution of `fit`.
fn xent(fit: &[u64; NS], test: &[u64; NS]) -> f64 {
    let tot: f64 = fit.iter().sum::<u64>() as f64 + 0.5 * NS as f64;
    test.iter().zip(fit).filter(|(&t, _)| t > 0).map(|(&t, &f)| -(t as f64) * ((f as f64 + 0.5) / tot).log2()).sum()
}

fn main() {
    let a: Vec<String> = std::env::args().collect();
    let manifest: Value = serde_json::from_slice(&std::fs::read(&a[1]).unwrap()).unwrap();
    let split: Value = serde_json::from_slice(&std::fs::read(&a[2]).unwrap()).unwrap();
    let out = std::path::PathBuf::from(&a[3]);
    assert!(!out.exists(), "refusing to overwrite {}", out.display());
    std::fs::create_dir_all(&out).unwrap();
    let names = |k: &str| -> Vec<String> { split[k].as_array().unwrap().iter().map(|v| v.as_str().unwrap().to_string()).collect() };
    let parts = [("train", names("train")), ("val", names("val")), ("test", names("test"))];
    let ids: Vec<String> = parts.iter().flat_map(|p| p.1.clone()).collect();
    let next = AtomicUsize::new(0);
    let res: Mutex<Vec<Img>> = Mutex::new(vec![]);
    std::thread::scope(|sc| {
        for _ in 0..4 {
            sc.spawn(|| loop {
                let i = next.fetch_add(1, Ordering::Relaxed);
                let Some(id) = ids.get(i) else { break };
                let m = manifest.as_array().unwrap().iter().find(|m| m["id"].as_str() == Some(id)).unwrap();
                let img = run(m);
                res.lock().unwrap().push(img);
            });
        }
    });
    let imgs = res.into_inner().unwrap();
    let get = |id: &str| imgs.iter().find(|i| i.id == id).unwrap();

    // Pooled per-split histograms.
    let mut pooled: Vec<(&str, Vec<&Img>, [u64; NS], Vec<[u64; NS]>)> = vec![];
    for (name, list) in &parts {
        let im: Vec<&Img> = list.iter().map(|i| get(i)).collect();
        let (mut h0, mut lf) = ([0u64; NS], vec![[0u64; NS]; L]);
        for i in &im {
            add(&mut h0, &i.h0);
            for k in 0..L {
                add(&mut lf[k], &i.leaf[k]);
            }
        }
        pooled.push((name, im, h0, lf));
    }
    let train_h0 = pooled[0].2;
    let train_lf = pooled[0].3.clone();

    // ---- per image
    let mut per_image = vec![];
    let mut csv = String::from("split,id,bayer,n,leaves_used,h0,h_tree,h_med_raster,h_med_plane,signaling_ratio_total,signaling_ratio_modelled,bottom50_visit_share,leaves_lt100,leaves_lt1000,leaves_lt10000\n");
    let mut agg = vec![];
    for (name, im, ..) in &pooled {
        let (mut n, mut b0, mut bt, mut bm, mut bp, mut bs0, mut bst) = (0u64, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0);
        let mut shares = vec![];
        for i in im {
            let nf = i.n as f64;
            let raw = i.raw as f64;
            let s0 = bits(&i.h0);
            let st: f64 = i.leaf.iter().map(|h| bits(h)).sum();
            let (h0, ht) = ((s0 + raw) / nf, (st + raw) / nf);
            let hm = (bits(&i.med) + i.med_raw as f64) / nf;
            let hp = (bits(&i.med_pl) + i.med_pl_raw as f64) / nf;
            let mut occ: Vec<u64> = i.leaf.iter().map(|h| h.iter().sum()).collect();
            occ.sort();
            let b50: u64 = occ[..L / 2].iter().sum();
            let share = b50 as f64 / nf;
            shares.push(share);
            let lt = |t: u64| occ.iter().filter(|&&o| o < t).count();
            csv += &format!(
                "{name},{},{},{},{},{h0:.5},{ht:.5},{hm:.5},{hp:.5},{:.5},{:.5},{share:.6},{},{},{}\n",
                i.id, i.bayer, i.n, occ.iter().filter(|&&o| o > 0).count(), (h0 - ht) / h0, (s0 - st) / s0, lt(100), lt(1000), lt(10000)
            );
            per_image.push(json!({"split": name, "id": i.id, "bayer": i.bayer, "n": i.n, "h_stage1_order0": h0, "h_tree": ht,
                "h_med_raster": hm, "h_med_plane": hp, "bottom50_share": share}));
            n += i.n;
            b0 += s0 + raw;
            bt += st + raw;
            bm += bits(&i.med) + i.med_raw as f64;
            bp += bits(&i.med_pl) + i.med_pl_raw as f64;
            bs0 += s0;
            bst += st;
        }
        let nf = n as f64;
        agg.push(json!({"split": name, "images": im.len(), "n": n, "h_stage1_order0": b0 / nf, "h_tree": bt / nf,
            "h_med_raster": bm / nf, "h_med_plane": bp / nf,
            "signaling_ratio_total": (b0 - bt) / b0, "signaling_ratio_modelled": (bs0 - bst) / bs0,
            "tree_minus_med_raster": (bt - bm) / nf, "tree_minus_med_plane": (bt - bp) / nf,
            "bottom50_share_per_image": {"min": shares.iter().cloned().fold(f64::MAX, f64::min), "max": shares.iter().cloned().fold(0.0, f64::max)}}));
    }

    // ---- pooled occupancy + out-of-sample signal, per-leaf rows
    let mut leaf_csv = String::from("leaf,train_visits,val_visits,test_visits,train_active_symbols,train_gain_bits_insample,val_gain_bits_heldout,test_gain_bits_heldout\n");
    let mut occ_json = vec![];
    let mut oos = vec![];
    for (si, (name, im, h0, lf)) in pooled.iter().enumerate() {
        let n: u64 = h0.iter().sum();
        let mut occ: Vec<u64> = lf.iter().map(|h| h.iter().sum()).collect();
        let raw: u64 = im.iter().map(|i| i.raw).sum();
        // fit = train pooled histograms; evaluated on this split (train = in-sample)
        let ce0 = xent(&train_h0, h0);
        let cet: f64 = (0..L).map(|k| xent(&train_lf[k], &lf[k])).sum();
        oos.push(json!({"split": name, "fit": "train pooled, add-1/2 smoothing", "n": n,
            "bits_per_residual_order0": (ce0 + raw as f64) / n as f64, "bits_per_residual_tree": (cet + raw as f64) / n as f64,
            "signaling_ratio_total": (ce0 - cet) / (ce0 + raw as f64), "signaling_ratio_modelled": (ce0 - cet) / ce0,
            "gain_bits": ce0 - cet}));
        occ.sort();
        let thr = [1u64, 10, 100, 1000, 10_000, 100_000, 1_000_000];
        let pct = |p: f64| occ[((L - 1) as f64 * p).round() as usize];
        occ_json.push(json!({"split": name, "total_visits": n, "leaves": L, "empty_leaves": occ.iter().filter(|&&o| o == 0).count(),
            "min": occ[0], "p10": pct(0.1), "p25": pct(0.25), "median": pct(0.5), "p75": pct(0.75), "p90": pct(0.9), "max": occ[L - 1],
            "mean": n as f64 / L as f64,
            "bottom50_leaves_visits": occ[..L / 2].iter().sum::<u64>(), "bottom50_share": occ[..L / 2].iter().sum::<u64>() as f64 / n as f64,
            "top10_share": occ[L - 10..].iter().sum::<u64>() as f64 / n as f64,
            "below_threshold": thr.iter().map(|&t| { let c = occ.iter().filter(|&&o| o < t).count();
                json!({"visits_lt": t, "leaves": c, "pct_leaves": 100.0 * c as f64 / L as f64,
                    "visits_in_those_leaves": occ.iter().filter(|&&o| o < t).sum::<u64>(),
                    "pct_visits": 100.0 * occ.iter().filter(|&&o| o < t).sum::<u64>() as f64 / n as f64}) }).collect::<Vec<_>>()}));
    }
    for k in 0..L {
        let gain = |si: usize| -> f64 {
            // order-0 cost of this leaf's symbols under the train order-0 model minus cost under the train leaf model
            xent(&train_h0, &pooled[si].3[k]) - xent(&train_lf[k], &pooled[si].3[k])
        };
        let v = |si: usize| -> u64 { pooled[si].3[k].iter().sum() };
        leaf_csv += &format!("{k},{},{},{},{},{:.1},{:.1},{:.1}\n", v(0), v(1), v(2), train_lf[k].iter().filter(|&&c| c > 0).count(), gain(0), gain(1), gain(2));
    }
    std::fs::write(out.join("per_image.csv"), csv).unwrap();
    std::fs::write(out.join("per_leaf.csv"), leaf_csv).unwrap();
    let mut leaf_img = String::from("id,split");
    for k in 0..L {
        leaf_img += &format!(",leaf{k}");
    }
    leaf_img.push('\n');
    for (name, im, ..) in &pooled {
        for i in im {
            leaf_img += &format!("{},{name}", i.id);
            for h in &i.leaf {
                leaf_img += &format!(",{}", h.iter().sum::<u64>());
            }
            leaf_img.push('\n');
        }
    }
    std::fs::write(out.join("per_image_leaf_visits.csv"), leaf_img).unwrap();
    let doc = json!({"representation": "bits/residual = H(s|ctx) + mean(max(e-2,0)); s=(e,m1) of zigzag(v)+1; static plug-in entropy per image",
        "per_split": agg, "out_of_sample_fit_on_train": oos, "occupancy_pooled": occ_json, "per_image": per_image});
    std::fs::write(out.join("diag.json"), serde_json::to_string_pretty(&doc).unwrap()).unwrap();
    println!("{}", serde_json::to_string_pretty(&doc["per_split"]).unwrap());
    println!("{}", serde_json::to_string_pretty(&doc["out_of_sample_fit_on_train"]).unwrap());
    println!("{}", serde_json::to_string_pretty(&doc["occupancy_pooled"]).unwrap());
}
