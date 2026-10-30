//! REL1: do predictor-disagreement (P) and cross-plane residual (X) information, given to the X4 mixer as extra inputs, reduce real coded
//! size? Research copy `pkonrel`; production X4 and the file format are untouched. Frozen rules: historical/20261027-rel1-pred-xplane/criteria.md.
//!
//!   cargo run --release --bin rel1 -- --out historical/20261027-rel1-pred-xplane
//!
//! Loads ONLY the 25 development + 10 validation images named in historical/learn1-splits.json (never the test list).
use nanoscope::canon::{Canonical, Layout};
use nanoscope::pkon2::center_crop;
use nanoscope::pkon2x::{measure_x, x4_cfg};
use nanoscope::pkonfile::encode_payload;
use nanoscope::pkonlearn::prepared;
use nanoscope::pkonrel::*;
use serde_json::json;
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;

const CROP: u32 = 1024;
const SPLITS: &str = "historical/learn1-splits.json";
const CRITERIA_SHA: &str = "f7eec0082c88eaa2ad705f31a24b8e4e803eaa6aa047f3b93371da8e82420c22";
const TEXTURED: [&str; 4] = ["forest_n0", "text_1", "river_n0", "text_0"];

fn sha(s: &str) -> String {
    Sha256::digest(s.as_bytes()).iter().map(|x| format!("{x:02x}")).collect()
}
fn sha_bytes(b: &[u8]) -> String {
    Sha256::digest(b).iter().map(|x| format!("{x:02x}")).collect()
}
// same rule as learn1 (re-implemented here so the committed split file is CHECKED, not trusted)
fn group_of(name: &str) -> String {
    name.split('_').next().unwrap().trim_end_matches(|c: char| c.is_ascii_digit()).to_string()
}
fn rederive(names: &[String]) -> (Vec<String>, Vec<String>, Vec<String>) {
    let mut groups: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for n in names {
        groups.entry(group_of(n)).or_default().push(n.clone());
    }
    let mut order: Vec<(String, Vec<String>)> = groups.into_iter().collect();
    order.sort_by_key(|(g, _)| sha(&format!("group:{g}")));
    let (mut test, mut val, mut train) = (vec![], vec![], vec![]);
    for (_, ns) in order {
        if test.len() < 15 {
            test.extend(ns);
        } else if val.len() < 10 {
            val.extend(ns);
        } else {
            train.extend(ns);
        }
    }
    train.sort_by_key(|n| sha(&format!("img:{n}")));
    (train, val, test)
}

struct Img {
    name: String,
    dev: bool,
    canon: Canonical,
    sub: nanoscope::pkon2::Planes,
    orig: Vec<Vec<u8>>,
}

#[derive(Clone, Default)]
struct Res {
    bytes: usize,
    exact: bool,
    trace_ok: bool,
    state_ok: bool,
    repeat_ok: bool,
    overread: bool,
    bits: [[f64; NCAT]; MAXP],
    count: [[u64; NCAT]; MAXP],
    extra_w: Vec<(usize, usize, f64, f64)>,
    stream: Vec<u8>,
}

fn interleave_rgb(planes: &[Vec<u8>], n: usize) -> Vec<u8> {
    let mut d = vec![0u8; n * 3];
    for i in 0..n {
        d[i * 3 + 1] = planes[0][i];
        d[i * 3] = planes[1][i];
        d[i * 3 + 2] = planes[2][i];
    }
    d
}

fn run_one(im: &Img, arm: &Arm, stats: Option<&mut Stats>) -> Res {
    let e = encode_rel(&im.sub, arm, stats);
    let e2 = encode_rel(&im.sub, arm, None);
    let d = decode_rel(&e.stream, arm, im.sub.w, im.sub.h, im.sub.p.len());
    let planes = invert(d.planes, im.sub.w, im.sub.h);
    let exact = planes == im.orig && interleave_rgb(&planes, im.sub.w * im.sub.h) == im.canon.data;
    Res {
        bytes: e.coded_bytes,
        exact,
        trace_ok: d.trace == e.trace,
        state_ok: d.state == e.state,
        repeat_ok: e.stream == e2.stream && e.trace == e2.trace,
        overread: d.consumed > e.stream.len() + 4,
        bits: e.bits,
        count: e.count,
        extra_w: e.extra_w,
        stream: e.stream,
    }
}

fn pct(base: f64, x: f64) -> f64 {
    100.0 * (base - x) / base
}

// ---- static information estimate: plug-in entropy with Miller-Madow correction, in total bits
fn code_len(cells: &[[u64; 2]]) -> f64 {
    let mut bits = 0.0;
    for c in cells {
        let n = (c[0] + c[1]) as f64;
        if n == 0.0 {
            continue;
        }
        let mut m = 0;
        for &v in c {
            if v > 0 {
                bits -= v as f64 * (v as f64 / n).log2();
                m += 1;
            }
        }
        bits += (m as f64 - 1.0) / (2.0 * std::f64::consts::LN_2);
    }
    bits
}
/// Total information (bits) the feature `map(f)` (ng classes) carries about the binary outcome given context c (or none) in plane k.
fn mi_bits(counts: &[u32], k: usize, nc: usize, use_c: bool, map: &dyn Fn(usize) -> usize, ng: usize) -> f64 {
    let ne = if use_c { nc } else { 1 };
    let mut cg = vec![[0u64; 2]; ne * ng];
    for c in 0..nc {
        let ce = if use_c { c } else { 0 };
        for f in 0..FCARD {
            let base = ((k * nc + c) * FCARD + f) * 2;
            let g = map(f);
            cg[ce * ng + g][0] += counts[base] as u64;
            cg[ce * ng + g][1] += counts[base + 1] as u64;
        }
    }
    let mut cm = vec![[0u64; 2]; ne];
    for ce in 0..ne {
        for g in 0..ng {
            cm[ce][0] += cg[ce * ng + g][0];
            cm[ce][1] += cg[ce * ng + g][1];
        }
    }
    code_len(&cm) - code_len(&cg)
}
fn entropy_of(h: &[u64]) -> (f64, usize) {
    let n: u64 = h.iter().sum();
    let mut e = 0.0;
    for &v in h {
        if v > 0 {
            e -= v as f64 / n as f64 * (v as f64 / n as f64).log2();
        }
    }
    (e, h.iter().filter(|&&v| v > 0).count())
}

fn main() {
    let a: Vec<String> = std::env::args().collect();
    let out = PathBuf::from(a.iter().position(|x| x == "--out").map(|i| a[i + 1].clone()).expect("--out"));

    // ---- 0. frozen criteria + split verification
    let crit_path = out.join("criteria.md");
    let crit = std::fs::read(&crit_path).expect("criteria.md");
    assert_eq!(sha_bytes(&crit), CRITERIA_SHA, "criteria.md changed after it was frozen");
    let split_bytes = std::fs::read(SPLITS).expect("splits file");
    let sj: serde_json::Value = serde_json::from_slice(&split_bytes).unwrap();
    let get = |k: &str| sj[k].as_array().unwrap().iter().map(|s| s.as_str().unwrap().to_string()).collect::<Vec<_>>();
    let (dev_names, val_names, test_names) = (get("train_pool"), get("val"), get("test"));
    let man: Vec<serde_json::Value> = serde_json::from_str(&std::fs::read_to_string("data/dataset50.json").unwrap()).unwrap();
    let all_names: Vec<String> = man.iter().map(|m| m["name"].as_str().unwrap().to_string()).collect();
    let (rd, rv, rt) = rederive(&all_names);
    assert!(rd == dev_names && rv == val_names && rt == test_names, "split file does not match its derivation rule");
    assert_eq!((dev_names.len(), val_names.len(), test_names.len()), (25, 10, 15));
    assert!(dev_names.iter().chain(&val_names).all(|n| !test_names.contains(n)), "test image in dev/val");
    let crop_hash: BTreeMap<String, String> = {
        let m: serde_json::Value = serde_json::from_str(&std::fs::read_to_string("benchmarks/pkon2/20261027-155433Z-bench50/dataset_manifest.json").unwrap()).unwrap();
        m["images"].as_array().unwrap().iter().map(|i| (i["name"].as_str().unwrap().to_string(), i["crop_sha256"].as_str().unwrap().to_string())).collect()
    };
    println!("splits verified (file sha256 {}), criteria frozen ({})", sha_bytes(&split_bytes), CRITERIA_SHA);

    // ---- 1. load dev + val only
    let mut imgs: Vec<Img> = vec![];
    for (names, dev) in [(&dev_names, true), (&val_names, false)] {
        for n in names {
            let m = man.iter().find(|m| m["name"].as_str() == Some(n)).unwrap();
            let full = Canonical::load(&Path::new("data").join(m["file"].as_str().unwrap())).unwrap();
            let crop = center_crop(&full, CROP);
            assert_eq!(crop.content_hash(), crop_hash[n], "crop hash mismatch for {n}");
            assert_eq!(crop.spec.layout, Layout::Rgb);
            let (sub, orig) = prepared(&crop);
            imgs.push(Img { name: n.clone(), dev, canon: crop, sub, orig });
        }
    }
    println!("loaded {} images ({} dev, {} val); all crop hashes match the bench50 manifest", imgs.len(), dev_names.len(), val_names.len());

    // ---- 2. main pass (parallel): every (image, arm): encode x2, decode, compare; control also collects information statistics
    let na = ARMS.len();
    let tasks: Vec<(usize, usize)> = (0..imgs.len()).flat_map(|i| (0..na).map(move |a| (i, a))).collect();
    let next = AtomicUsize::new(0);
    let results = Mutex::new(vec![vec![Res::default(); na]; imgs.len()]);
    let prod_fail: Mutex<Vec<String>> = Mutex::new(vec![]);
    let stats_dev = Mutex::new(Stats::new());
    let stats_val = Mutex::new(Stats::new());
    let t0 = std::time::Instant::now();
    std::thread::scope(|s| {
        for _ in 0..std::thread::available_parallelism().map(|n| n.get()).unwrap_or(4) {
            s.spawn(|| loop {
                let t = next.fetch_add(1, Ordering::Relaxed);
                if t >= tasks.len() {
                    break;
                }
                let (i, ai) = tasks[t];
                let im = &imgs[i];
                let mut st = if ai == 0 { Some(Stats::new()) } else { None };
                let r = run_one(im, &ARMS[ai], st.as_mut());
                if ai == 0 {
                    let prod = encode_payload(&im.canon);
                    let sz = measure_x(&im.canon, &x4_cfg()).coded_bytes;
                    if prod != r.stream || sz != r.bytes {
                        prod_fail.lock().unwrap().push(format!("{}: control != production (stream equal {}, size {} vs {})", im.name, prod == r.stream, r.bytes, sz));
                        let _ = std::fs::write(out.join(format!("FAIL_{}_control.bin", im.name)), &r.stream);
                        let _ = std::fs::write(out.join(format!("FAIL_{}_production.bin", im.name)), &prod);
                    }
                    let acc = if im.dev { &stats_dev } else { &stats_val };
                    let mut acc = acc.lock().unwrap();
                    let st = st.unwrap();
                    for (d, s) in acc.sign.iter_mut().zip(&st.sign) {
                        *d += s;
                    }
                    for (d, s) in acc.zero.iter_mut().zip(&st.zero) {
                        *d += s;
                    }
                    for k in 0..MAXP {
                        acc.npix[k] += st.npix[k];
                    }
                }
                results.lock().unwrap()[i][ai] = r;
            });
        }
    });
    println!("main pass {:.0}s", t0.elapsed().as_secs_f64());
    let results = results.into_inner().unwrap();

    // ---- 3. correctness gates
    let mut fails = prod_fail.into_inner().unwrap();
    for (i, im) in imgs.iter().enumerate() {
        for (ai, arm) in ARMS.iter().enumerate() {
            let r = &results[i][ai];
            let mut why = vec![];
            if !r.exact { why.push("pixels differ"); }
            if !r.trace_ok { why.push("context/symbol trace differs enc vs dec"); }
            if !r.state_ok { why.push("final model state differs enc vs dec"); }
            if !r.repeat_ok { why.push("re-encode not identical"); }
            if r.overread { why.push("decoder over-read on a valid stream"); }
            if !why.is_empty() {
                fails.push(format!("{} / {}: {}", im.name, arm.name, why.join(", ")));
                let _ = std::fs::write(out.join(format!("FAIL_{}_{}.bin", im.name, arm.name)), &r.stream);
            }
        }
    }
    if !fails.is_empty() {
        let msg = fails.join("\n");
        std::fs::write(out.join("gate_failure.txt"), &msg).unwrap();
        eprintln!("CORRECTNESS GATE FAILED (artifacts kept in {}):\n{msg}", out.display());
        std::process::exit(2);
    }
    println!("gates G1 (exact), G2 (control == production), G3 (enc/dec trace + state), G6 (repeatable): PASS on {} x {} runs", imgs.len(), na);

    // ---- 4. timing pass: single thread, validation images only, best of 2
    let val_idx: Vec<usize> = (0..imgs.len()).filter(|&i| !imgs[i].dev).collect();
    let mut timing = vec![(0.0f64, 0.0f64); na];
    for (ai, arm) in ARMS.iter().enumerate() {
        for &i in &val_idx {
            let im = &imgs[i];
            let e1 = encode_rel(&im.sub, arm, None);
            let e2 = encode_rel(&im.sub, arm, None);
            let d1 = decode_rel(&e1.stream, arm, im.sub.w, im.sub.h, 3);
            let d2 = decode_rel(&e1.stream, arm, im.sub.w, im.sub.h, 3);
            timing[ai].0 += e1.ms.min(e2.ms);
            timing[ai].1 += d1.ms.min(d2.ms);
        }
    }

    // ---- 5. aggregates and frozen decision rules
    let sum = |ai: usize, f: &dyn Fn(&Img) -> bool| (0..imgs.len()).filter(|&i| f(&imgs[i])).map(|i| results[i][ai].bytes as f64).sum::<f64>();
    let dev = |im: &Img| im.dev;
    let val = |im: &Img| !im.dev;
    let s_of = |ai: usize, f: &dyn Fn(&Img) -> bool| pct(sum(0, f), sum(ai, f));
    let improved = |ai: usize, f: &dyn Fn(&Img) -> bool| -> (usize, usize) {
        let idx: Vec<usize> = (0..imgs.len()).filter(|&i| f(&imgs[i])).collect();
        (idx.iter().filter(|&&i| results[i][ai].bytes < results[i][0].bytes).count(), idx.len())
    };
    let ai = |n: &str| ARMS.iter().position(|a| a.name == n).unwrap();
    let (d_i, p_i, x_i, px_i, j_i, d2_i, dj_i) = (ai("dummy"), ai("P"), ai("X"), ai("P+X"), ai("PxX"), ai("dummy2"), ai("dummyJ"));
    let e_of = |f: &dyn Fn(&Img) -> bool| s_of(j_i, f) - s_of(px_i, f);
    let ecap_of = |f: &dyn Fn(&Img) -> bool| (s_of(j_i, f) - s_of(dj_i, f)) - (s_of(px_i, f) - s_of(d2_i, f));
    let val_support = |arm: usize| s_of(arm, &val) >= 0.25 && improved(arm, &val).0 >= 7;
    let gates_ok = true;
    let confounded = s_of(d_i, &dev) >= 0.10 || !gates_ok;
    let (sp, sx) = (s_of(p_i, &dev), s_of(x_i, &dev));
    let p_prom = sp >= 0.5 && improved(p_i, &dev).0 as f64 >= 0.8 * 25.0 && sp >= 2.0 * sx.max(0.0) && val_support(p_i);
    let x_prom = sx >= 0.5 && sx > sp && val_support(x_i);
    let j_prom = e_of(&dev) >= 0.3 && ecap_of(&dev) >= 0.15 && e_of(&val) > 0.0 && ecap_of(&val) > 0.0;
    let little = sp < 0.2 && sx < 0.2 && !val_support(p_i) && !val_support(x_i);
    let verdict = |b: bool| if b { "PASS" } else { "no" };

    // ---- 6. information estimates (DEV set, per plane), and realised cross-entropy savings of the matching arm
    let sd = stats_dev.into_inner().unwrap();
    let sv = stats_val.into_inner().unwrap();
    let mut info_md = String::from("| set | plane | feature | I(sign;F) | I(sign;F\\|S1) | I(zero;F\\|Ab) | realised sign saving (arm CE) | realised zero+ge2 saving (arm CE) |\n|---|---|---|---|---|---|---|---|\n");
    let mut info_json = vec![];
    for (sname, st, f) in [("DEV", &sd, &dev as &dyn Fn(&Img) -> bool), ("VAL", &sv, &val as &dyn Fn(&Img) -> bool)] {
        let ce = |ai: usize, k: usize, c: usize| -> f64 { (0..imgs.len()).filter(|&i| f(&imgs[i])).map(|i| results[i][ai].bits[k][c]).sum() };
        for k in 0..3 {
            for (fname, map, ng, arm_i) in [
                ("P", Box::new(|f: usize| f / 49) as Box<dyn Fn(usize) -> usize>, 49usize, p_i),
                ("X", Box::new(|f: usize| f % 49), 49, x_i),
                ("PxX", Box::new(|f: usize| f), FCARD, j_i),
            ] {
                if k == 0 && fname != "P" {
                    continue; // X does not exist on plane 0
                }
                let npix = st.npix[k] as f64;
                let a = mi_bits(&st.sign, k, 27, false, &*map, ng) / npix;
                let b = mi_bits(&st.sign, k, 27, true, &*map, ng) / npix;
                let c = mi_bits(&st.zero, k, 96, true, &*map, ng) / npix;
                let rs = (ce(0, k, 4) - ce(arm_i, k, 4)) / npix;
                let rz = ((ce(0, k, 0) + ce(0, k, 1)) - (ce(arm_i, k, 0) + ce(arm_i, k, 1))) / npix;
                info_md += &format!("| {sname} | {k} | {fname} | {a:.5} | {b:.5} | {c:.5} | {rs:.5} | {rz:.5} |\n");
                info_json.push(json!({"set": sname, "plane": k, "feature": fname, "I_sign": a, "I_sign_given_S1": b, "I_zero_given_Ab": c, "realised_sign": rs, "realised_zero_ge2": rz}));
            }
        }
    }
    // context usage (DEV, from the zero-flag counts: every pixel contributes once)
    let mut usage_md = String::from("| feature | plane | distinct classes used | entropy of class usage (bits) | max possible (bits) |\n|---|---|---|---|---|\n");
    for k in 0..3 {
        let mut hp = vec![0u64; 49];
        let mut hx = vec![0u64; 49];
        for c in 0..96 {
            for f in 0..FCARD {
                let base = ((k * 96 + c) * FCARD + f) * 2;
                let n = (sd.zero[base] + sd.zero[base + 1]) as u64;
                hp[f / 49] += n;
                hx[f % 49] += n;
            }
        }
        let (ep, np) = entropy_of(&hp);
        usage_md += &format!("| P | {k} | {np} | {ep:.3} | {:.3} |\n", 49f64.log2());
        if k > 0 {
            let (ex, nx) = entropy_of(&hx);
            usage_md += &format!("| X | {k} | {nx} | {ex:.3} | {:.3} |\n", 49f64.log2());
        }
    }
    usage_md += &format!("| dummy (hash, uniform by construction) | all | 49 | ~{:.3} | {:.3} |\n", 49f64.log2(), 49f64.log2());

    // ---- 7. report
    let mut md = String::from("# REL1: predictor disagreement (P) and cross-plane residual (X) as extra mixer inputs\n\n");
    md += &format!("Criteria frozen before the run (sha256 {CRITERIA_SHA}); split file sha256 {}; DEV = 25 images, VAL = 10, test list never loaded. Sizes = stream + 11-byte header (no per-arm signalling; the fixed container header is identical for all arms).\n\n", sha_bytes(&split_bytes));
    md += "**Correctness gates: PASS** (every arm x every image: exact pixels incl. interleaved compare to the canonical crop; control stream == `pkonfile::encode_payload` and size == `measure_x`; encoder/decoder context+symbol trace hash and final model-state hash equal; re-encode identical; no decoder over-read). Unit tests (edge sizes, constant/checkerboard/noise/ramp, Gray+RGB, truncation/corruption, causality of P, feature ranges) are in `src/pkonrel.rs`.\n\n";
    md += "## Aggregate actual encoded size\n\n| arm | DEV bytes | S(DEV) % | improves DEV | VAL bytes | S(VAL) % | improves VAL | enc ms (VAL sum, 1 thread) | dec ms | extra inputs |\n|---|---|---|---|---|---|---|---|---|---|\n";
    for (k, arm) in ARMS.iter().enumerate() {
        let (di, dn) = improved(k, &dev);
        let (vi, vn) = improved(k, &val);
        md += &format!("| {} | {} | {:+.3} | {}/{} | {} | {:+.3} | {}/{} | {:.0} | {:.0} | {} |\n", arm.name, sum(k, &dev) as u64, s_of(k, &dev), di, dn, sum(k, &val) as u64, s_of(k, &val), vi, vn, timing[k].0, timing[k].1, arm.extras.len());
    }
    md += &format!("\nTime ratios vs control (VAL, best of 2, single thread): {}\n", ARMS.iter().enumerate().skip(1).map(|(k, a)| format!("{} enc x{:.2} dec x{:.2}", a.name, timing[k].0 / timing[0].0, timing[k].1 / timing[0].1)).collect::<Vec<_>>().join("; "));
    md += &format!("\n## Entanglement\n\nE = S(PxX) - S(P+X): DEV {:+.3} pp, VAL {:+.3} pp. Capacity-adjusted E_cap = [S(PxX)-S(dummyJ)] - [S(P+X)-S(dummy2)]: DEV {:+.3} pp, VAL {:+.3} pp. (dummyJ = 1 meaningless input with the joint's 2401 classes; dummy2 = 2 meaningless inputs of 49.) Per-image E is in `per_image.csv` (`e_joint_minus_sep_pp`).\n", e_of(&dev), e_of(&val), ecap_of(&dev), ecap_of(&val));
    md += "\n## Secondary: textured vs smooth (DEV; not decisive)\n\n| arm | textured S % | smooth S % |\n|---|---|---|\n";
    let tex = |im: &Img| im.dev && TEXTURED.contains(&im.name.as_str());
    let smo = |im: &Img| im.dev && !TEXTURED.contains(&im.name.as_str());
    for k in 1..na {
        md += &format!("| {} | {:+.3} | {:+.3} |\n", ARMS[k].name, s_of(k, &tex), s_of(k, &smo));
    }
    md += "\n## Cross-entropy by decision type (DEV, % change in bits vs control, positive = fewer bits)\n\n| arm | zero | ge2 | class | mantissa | sign | total |\n|---|---|---|---|---|---|---|\n";
    let cebits = |ai: usize, f: &dyn Fn(&Img) -> bool, c: Option<usize>| -> f64 { (0..imgs.len()).filter(|&i| f(&imgs[i])).map(|i| (0..MAXP).map(|k| (0..NCAT).filter(|&cc| c.map_or(true, |x| x == cc)).map(|cc| results[i][ai].bits[k][cc]).sum::<f64>()).sum::<f64>()).sum() };
    for k in 1..na {
        md += &format!("| {} |", ARMS[k].name);
        for c in 0..NCAT {
            md += &format!(" {:+.3} |", pct(cebits(0, &dev, Some(c)), cebits(k, &dev, Some(c))));
        }
        md += &format!(" {:+.3} |\n", pct(cebits(0, &dev, None), cebits(k, &dev, None)));
    }
    let max_gap = (0..imgs.len()).flat_map(|i| (0..na).map(move |a| (i, a))).map(|(i, a)| { let r = &results[i][a]; let ce: f64 = r.bits.iter().flatten().sum::<f64>() / 8.0; ((ce - (r.bytes - 11) as f64) / (r.bytes - 11) as f64 * 100.0).abs() }).fold(0.0, f64::max);
    md += &format!("\nSanity: max |cross-entropy/8 - stream bytes| over all runs = {max_gap:.4}%.\n");
    md += "\n## Static information carried by the features (bits per pixel of the plane; DEV and VAL)\n\nEstimator: plug-in conditional entropy with Miller-Madow correction, computed in-sample from the exact features/contexts the coder used, on the images' final pass (static distribution per plane over the whole image set; no adaptation, no learning cost). F = feature; S1 = existing sign context #2 (W,N,NE residual signs, 27 states); Ab = existing magnitude context A/b (96 states); zero = (residual == 0). It measures how much the feature tells about the outcome *beyond one existing context*; it does NOT measure what the other inputs already capture jointly, is optimistic for the 2401-class PxX (finite-sample), and is not an achievable size bound. The last two columns are the coder's realised cross-entropy savings of the matching arm (P->P, X->X, PxX->PxX), per pixel, for comparison.\n\n";
    md += &info_md;
    md += "\nContext usage (DEV):\n\n";
    md += &usage_md;
    md += "\n## Adaptation of the extra inputs (mean final mixer weight, DEV images; weight 1.0 = 65536)\n\n| arm | extra # | plane | node-0 weight | sign weight |\n|---|---|---|---|---|\n";
    for k in 1..na {
        let mut acc: BTreeMap<(usize, usize), (f64, f64, usize)> = BTreeMap::new();
        for i in (0..imgs.len()).filter(|&i| imgs[i].dev) {
            for &(p, j, w0, ws) in &results[i][k].extra_w {
                let e = acc.entry((p, j)).or_default();
                e.0 += w0;
                e.1 += ws;
                e.2 += 1;
            }
        }
        for ((p, j), (w0, ws, n)) in acc {
            md += &format!("| {} | {} | {} | {:.3} | {:.3} |\n", ARMS[k].name, j, p, w0 / n as f64, ws / n as f64);
        }
    }
    md += "\n## Decision rules (criteria.md)\n\n";
    md += &format!("- S(Dummy,DEV) = {:+.3} (confounded if >= +0.10): confounded = {}\n", s_of(d_i, &dev), confounded);
    md += &format!("- P promising: S(P,DEV) {:+.3} (>=0.5), improves {}/25 (>=20), S(P)>=2*max(S(X),0): {}, VAL S {:+.3} improves {}/10 -> **{}**\n", sp, improved(p_i, &dev).0, sp >= 2.0 * sx.max(0.0), s_of(p_i, &val), improved(p_i, &val).0, verdict(p_prom));
    md += &format!("- X promising: S(X,DEV) {:+.3} (>=0.5), > S(P): {}, VAL S {:+.3} improves {}/10 -> **{}**\n", sx, sx > sp, s_of(x_i, &val), improved(x_i, &val).0, verdict(x_prom));
    md += &format!("- Joint promising: E(DEV) {:+.3} (>=0.3), E_cap(DEV) {:+.3} (>=0.15), E(VAL) {:+.3} (>0), E_cap(VAL) {:+.3} (>0) -> **{}**\n", e_of(&dev), ecap_of(&dev), e_of(&val), ecap_of(&val), verdict(j_prom));
    md += &format!("- Little practical signal: **{}**\n", verdict(little));
    let overall = if confounded { "CONFOUNDED (extra capacity alone helps)" } else if p_prom || x_prom || j_prom { "AT LEAST ONE PROMISING RULE PASSED (see above)" } else if little { "LITTLE PRACTICAL SIGNAL" } else { "AMBIGUOUS" };
    md += &format!("\n**Overall: {overall}**\n");
    std::fs::write(out.join("report.md"), &md).unwrap();

    let mut csv = String::from("image,split,arm,bytes,vs_control_pct,exact,ce_zero,ce_ge2,ce_class,ce_mant,ce_sign,ce_total_bits,e_joint_minus_sep_pp\n");
    for (i, im) in imgs.iter().enumerate() {
        for (k, arm) in ARMS.iter().enumerate() {
            let r = &results[i][k];
            let c: Vec<f64> = (0..NCAT).map(|c| r.bits.iter().map(|b| b[c]).sum()).collect();
            let e = if k == j_i { format!("{:.4}", pct(results[i][0].bytes as f64, r.bytes as f64) - pct(results[i][0].bytes as f64, results[i][px_i].bytes as f64)) } else { String::new() };
            csv += &format!("{},{},{},{},{:+.4},{},{:.1},{:.1},{:.1},{:.1},{:.1},{:.1},{}\n", im.name, if im.dev { "dev" } else { "val" }, arm.name, r.bytes, pct(results[i][0].bytes as f64, r.bytes as f64), r.exact, c[0], c[1], c[2], c[3], c[4], c.iter().sum::<f64>(), e);
        }
    }
    std::fs::write(out.join("per_image.csv"), csv).unwrap();
    let rj = json!({"criteria_sha256": CRITERIA_SHA, "splits_sha256": sha_bytes(&split_bytes), "overall": overall,
        "S_dev": ARMS.iter().enumerate().map(|(k, a)| json!({"arm": a.name, "S_dev": s_of(k, &dev), "S_val": s_of(k, &val)})).collect::<Vec<_>>(),
        "E_dev": e_of(&dev), "E_val": e_of(&val), "Ecap_dev": ecap_of(&dev), "Ecap_val": ecap_of(&val), "info": info_json,
        "p_promising": p_prom, "x_promising": x_prom, "joint_promising": j_prom, "little_signal": little, "confounded": confounded});
    std::fs::write(out.join("results.json"), serde_json::to_string_pretty(&rj).unwrap()).unwrap();
    println!("{md}");
}
