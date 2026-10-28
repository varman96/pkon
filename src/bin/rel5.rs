//! REL4: X (cross-plane residual) as an extra mixer input on top of production codec 2. Frozen rules: results/20261027-rel5-x-sign/criteria.md.
//!
//!   cargo run --release --bin rel5 -- --out results/20261027-rel5-x-sign
//!
//! Loads ONLY the 25 development + 10 validation images of results/learn1-splits.json (never the test list).
use nanoscope::canon::{Canonical, Layout};
use nanoscope::pkon2::center_crop;
use nanoscope::pkonfile::{crc32, encode_with_codec, HEADER_LEN, CODEC_X4P};
use nanoscope::pkonlearn::prepared;
use nanoscope::pkonrel::*;
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;

const CROP: u32 = 1024;
const SPLITS: &str = "results/learn1-splits.json";
const CRITERIA_SHA: &str = "a47ceb43de35c3260344d02f9e87de4ffc00f0540f73835dde42187c7ba8c0be";
const REL1_CSV: &str = "results/20261027-rel4-x-on-p/per_image.csv";

fn sha_bytes(b: &[u8]) -> String {
    Sha256::digest(b).iter().map(|x| format!("{x:02x}")).collect()
}
fn sha(s: &str) -> String {
    sha_bytes(s.as_bytes())
}
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
    stream: Vec<u8>,
    file: Vec<u8>,
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
fn run_one(im: &Img, arm: &Arm) -> Res {
    let e = encode_rel(&im.sub, arm, None);
    let e2 = encode_rel(&im.sub, arm, None);
    let d = decode_rel(&e.stream, arm, im.sub.w, im.sub.h, im.sub.p.len());
    let planes = invert(d.planes, im.sub.w, im.sub.h);
    let exact = planes == im.orig && interleave_rgb(&planes, im.sub.w * im.sub.h) == im.canon.data;
    // complete file: production 24-byte header layout (codec byte 3 for the research arms); decoded back from the FILE bytes
    let mut file = encode(&im.canon).unwrap()[..HEADER_LEN].to_vec();
    if arm.name != "P" {
        file[6] = 3;
    }
    file[16..20].copy_from_slice(&(e.stream.len() as u32).to_le_bytes());
    file.extend_from_slice(&e.stream);
    let plen = u32::from_le_bytes(file[16..20].try_into().unwrap()) as usize;
    let fd = decode_rel(&file[HEADER_LEN..HEADER_LEN + plen], arm, im.sub.w, im.sub.h, im.sub.p.len());
    let fpl = invert(fd.planes, im.sub.w, im.sub.h);
    let crc_ok = u32::from_le_bytes(file[20..24].try_into().unwrap()) == crc32(&interleave_rgb(&fpl, im.sub.w * im.sub.h)) && file.len() == HEADER_LEN + plen;
    let exact = exact && fpl == im.orig && crc_ok;
    Res { bytes: file.len(), exact, trace_ok: d.trace == e.trace, state_ok: d.state == e.state, repeat_ok: e.stream == e2.stream && e.trace == e2.trace, overread: d.consumed > e.stream.len() + 4, bits: e.bits, stream: e.stream, file }
}
fn pct(base: f64, x: f64) -> f64 {
    100.0 * (base - x) / base
}

fn main() {
    let a: Vec<String> = std::env::args().collect();
    let out = PathBuf::from(a.iter().position(|x| x == "--out").map(|i| a[i + 1].clone()).expect("--out"));
    let crit = std::fs::read(out.join("criteria.md")).expect("criteria.md");
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
    let crop_hash: BTreeMap<String, String> = {
        let m: serde_json::Value = serde_json::from_str(&std::fs::read_to_string("results/20261027-155433Z-bench50/dataset_manifest.json").unwrap()).unwrap();
        m["images"].as_array().unwrap().iter().map(|i| (i["name"].as_str().unwrap().to_string(), i["crop_sha256"].as_str().unwrap().to_string())).collect()
    };
    // REL4 reference file bytes for the cross-run repeatability gate G6
    let rel1: BTreeMap<(String, String), usize> = std::fs::read_to_string(REL1_CSV)
        .unwrap()
        .lines()
        .skip(1)
        .map(|l| {
            let f: Vec<&str> = l.split(',').collect();
            ((f[0].to_string(), f[2].to_string()), f[3].parse().unwrap())
        })
        .collect();
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
    println!("loaded {} images; criteria {CRITERIA_SHA}", imgs.len());

    let na = XS_ARMS.len();
    let tasks: Vec<(usize, usize)> = (0..imgs.len()).flat_map(|i| (0..na).map(move |a| (i, a))).collect();
    let next = AtomicUsize::new(0);
    let results = Mutex::new(vec![vec![Res::default(); na]; imgs.len()]);
    let fails: Mutex<Vec<String>> = Mutex::new(vec![]);
    std::thread::scope(|s| {
        for _ in 0..std::thread::available_parallelism().map(|n| n.get()).unwrap_or(4) {
            s.spawn(|| loop {
                let t = next.fetch_add(1, Ordering::Relaxed);
                if t >= tasks.len() {
                    break;
                }
                let (i, ai) = tasks[t];
                let (im, arm) = (&imgs[i], &XS_ARMS[ai]);
                let r = run_one(im, arm);
                let mut why = vec![];
                if !r.exact {
                    why.push("pixels differ".to_string());
                }
                if !r.trace_ok {
                    why.push("trace differs".into());
                }
                if !r.state_ok {
                    why.push("model state differs".into());
                }
                if !r.repeat_ok {
                    why.push("re-encode differs".into());
                }
                if r.overread {
                    why.push("decoder over-read".into());
                }
                if ai == 0 && encode(&im.canon).unwrap() != r.file {
                    why.push("P != production codec 2".into());
                }
                if r.bytes != HEADER_LEN + r.stream.len() {
                    why.push("file bytes != 24 + payload".into());
                }
                if let Some(&b) = rel1.get(&(im.name.clone(), arm.name.to_string())) {
                    if b != r.bytes {
                        why.push(format!("bytes {} != REL4 {}", r.bytes, b));
                    }
                }
                if !why.is_empty() {
                    fails.lock().unwrap().push(format!("{} / {}: {}", im.name, arm.name, why.join(", ")));
                }
                results.lock().unwrap()[i][ai] = r;
            });
        }
    });
    let results = results.into_inner().unwrap();
    let fails = fails.into_inner().unwrap();
    if !fails.is_empty() {
        std::fs::write(out.join("gate_failure.txt"), fails.join("\n")).unwrap();
        eprintln!("CORRECTNESS GATE FAILED:\n{}", fails.join("\n"));
        std::process::exit(2);
    }
    println!("gates G1-G6 PASS (G7, G8 next) on {} x {} runs", imgs.len(), na);

    // G7: non-sign isolation (zero, ge2, class, mantissa = categories 0..=3): exactly equal cross-entropy to P, per image and plane
    let mut g7 = vec![];
    for (i, im) in imgs.iter().enumerate() {
        for ai in 1..na {
            for k in 0..MAXP {
                for c in 0..4 {
                    if results[i][ai].bits[k][c] != results[i][0].bits[k][c] {
                        g7.push(format!("{} / {} plane {k} cat {c}: {} != {}", im.name, XS_ARMS[ai].name, results[i][ai].bits[k][c], results[i][0].bits[k][c]));
                    }
                }
            }
        }
    }
    // G8: equal model counts / allocation between X-sign and dummy-sign
    let mc = |ai: usize| Coder::new(3, &XS_ARMS[ai]).extra_model_counts(3, &XS_ARMS[ai]);
    let (mx, md_) = (mc(1), mc(2));
    if mx != md_ || mx.2 != 2 * 49 {
        g7.push(format!("model counts differ or unexpected: Xsign {mx:?} dummysign {md_:?}"));
    }
    if !g7.is_empty() {
        std::fs::write(out.join("gate_failure.txt"), g7.join("\n")).unwrap();
        eprintln!("G7/G8 FAILED:\n{}", g7.join("\n"));
        std::process::exit(2);
    }
    println!("gates G7 (non-sign CE exactly equal to P) and G8 (model counts {mx:?} = {md_:?}) PASS");

    // timing: single thread, VAL, best of 2
    let mut timing = vec![(0.0f64, 0.0f64); na];
    for (ai, arm) in XS_ARMS.iter().enumerate() {
        for im in imgs.iter().filter(|i| !i.dev) {
            let e1 = encode_rel(&im.sub, arm, None);
            let e2 = encode_rel(&im.sub, arm, None);
            let d1 = decode_rel(&e1.stream, arm, im.sub.w, im.sub.h, 3);
            let d2 = decode_rel(&e1.stream, arm, im.sub.w, im.sub.h, 3);
            timing[ai].0 += e1.ms.min(e2.ms);
            timing[ai].1 += d1.ms.min(d2.ms);
        }
    }

    let (c_i, a_i, d_i) = (0usize, 1usize, 2usize);
    let dev = |im: &Img| im.dev;
    let val = |im: &Img| !im.dev;
    let sum = |ai: usize, f: &dyn Fn(&Img) -> bool| (0..imgs.len()).filter(|&i| f(&imgs[i])).map(|i| results[i][ai].bytes as f64).sum::<f64>();
    let s_vs = |ai: usize, f: &dyn Fn(&Img) -> bool| pct(sum(c_i, f), sum(ai, f));
    let imp = |ai: usize, f: &dyn Fn(&Img) -> bool| -> (usize, usize) {
        let idx: Vec<usize> = (0..imgs.len()).filter(|&i| f(&imgs[i])).collect();
        (idx.iter().filter(|&&i| results[i][ai].bytes < results[i][c_i].bytes).count(), idx.len())
    };
    let mut md = String::from("# REL5: X on the sign decision only, on top of production codec 2

");
    md += &format!("Criteria frozen before the run (sha256 {CRITERIA_SHA}). DEV 25 / VAL 10, test list never loaded. Sizes = complete file (24-byte header + payload). Baseline `P` = production codec 2.

**Correctness gates G1-G8: PASS** (exact pixels from the file bytes + CRC, P file == production `encode` output on every image, enc/dec trace+state hashes equal, re-encode identical, no over-read, P bytes equal REL4 per-image P bytes, file = 24 + payload, non-sign cross-entropy of both extra arms exactly equal to P, equal model counts).

");
    md += "| arm | DEV bytes | S(DEV) % vs P | improves | VAL bytes | S(VAL) % | improves | enc ms (VAL, 1 thr) | dec ms |
|---|---|---|---|---|---|---|---|---|
";
    for (k, arm) in XS_ARMS.iter().enumerate() {
        let (a1, b1) = imp(k, &dev);
        let (a2, b2) = imp(k, &val);
        md += &format!("| {} | {} | {:+.3} | {}/{} | {} | {:+.3} | {}/{} | {:.0} | {:.0} |
", arm.name, sum(k, &dev) as u64, s_vs(k, &dev), a1, b1, sum(k, &val) as u64, s_vs(k, &val), a2, b2, timing[k].0, timing[k].1);
    }
    md += "
## Cross-entropy by decision type (DEV, % fewer bits vs P)

| arm | zero | ge2 | class | mantissa | sign | total |
|---|---|---|---|---|---|---|
";
    let ce = |ai: usize, c: Option<usize>| -> f64 {
        (0..imgs.len()).filter(|&i| imgs[i].dev).map(|i| (0..MAXP).map(|k| (0..NCAT).filter(|&cc| c.map_or(true, |x| x == cc)).map(|cc| results[i][ai].bits[k][cc]).sum::<f64>()).sum::<f64>()).sum()
    };
    for k in 1..na {
        md += &format!("| {} |", XS_ARMS[k].name);
        for c in 0..NCAT {
            md += &format!(" {:+.3} |", pct(ce(0, Some(c)), ce(k, Some(c))));
        }
        md += &format!(" {:+.3} |
", pct(ce(0, None), ce(k, None)));
    }
    // sign vs non-sign (bytes saved from the exact coder cross-entropy, DEV and VAL), per-image X vs dummy, capacity/memory
    md += "
## Sign vs non-sign decisions (bytes saved vs P, from exact coder cross-entropy)

| arm | set | sign bytes saved | non-sign bytes saved | sign share of P's bits |
|---|---|---|---|---|
";
    let ces = |ai: usize, f: &dyn Fn(&Img) -> bool, sign: bool| -> f64 {
        (0..imgs.len()).filter(|&i| f(&imgs[i])).map(|i| (0..MAXP).map(|k| (0..NCAT).filter(|&cc| (cc == 4) == sign).map(|cc| results[i][ai].bits[k][cc]).sum::<f64>()).sum::<f64>()).sum::<f64>() / 8.0
    };
    for k in 1..na {
        for (nm, f) in [("DEV", &dev as &dyn Fn(&Img) -> bool), ("VAL", &val as &dyn Fn(&Img) -> bool)] {
            md += &format!("| {} | {} | {:.0} | {:.0} | {:.2}% |
", XS_ARMS[k].name, nm, ces(0, f, true) - ces(k, f, true), ces(0, f, false) - ces(k, f, false), 100.0 * ces(0, f, true) / (ces(0, f, true) + ces(0, f, false)));
        }
    }
    let xb = |f: &dyn Fn(&Img) -> bool| (0..imgs.len()).filter(|&i| f(&imgs[i]) && results[i][a_i].bytes < results[i][d_i].bytes).count();
    let ni = |f: &dyn Fn(&Img) -> bool| (0..imgs.len()).filter(|&i| f(&imgs[i])).count();
    let mut dl: Vec<f64> = (0..imgs.len()).map(|i| pct(results[i][0].bytes as f64, results[i][a_i].bytes as f64)).collect();
    dl.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let ddl = {
        let mut v: Vec<f64> = (0..imgs.len()).map(|i| pct(results[i][0].bytes as f64, results[i][d_i].bytes as f64)).collect();
        v.sort_by(|a, b| a.partial_cmp(b).unwrap());
        v
    };
    md += &format!("
P+Xsign beats P+dummysign on {}/{} DEV and {}/{} VAL images (informational). Per-image saving vs P, all 35: Xsign min {:+.3}% / median {:+.3}% / max {:+.3}%; dummysign min {:+.3}% / median {:+.3}% / max {:+.3}%.
", xb(&dev), ni(&dev), xb(&val), ni(&val), dl[0], dl[dl.len() / 2], dl[dl.len() - 1], ddl[0], ddl[ddl.len() / 2], ddl[ddl.len() - 1]);
    md += &format!("
Gates G7/G8 passed: non-sign cross-entropy of both extra arms is exactly equal to P's on every image and plane; extra-input model counts (allocated models, bytes, reachable sign-node models) Xsign {mx:?} = dummysign {md_:?}. Encode/decode times above are single-thread best-of-2 on VAL; peak process memory was not measured (the added state is the allocated-model figure above plus no extra mixer storage).
");
    let (sd, sv) = (s_vs(a_i, &dev), s_vs(a_i, &val));
    let (id, iv) = (imp(a_i, &dev), imp(a_i, &val));
    let c2 = sd >= 0.5 && id.0 as f64 >= 0.8 * id.1 as f64;
    let c3 = sv >= 0.25 && iv.0 >= 7;
    let c4 = sd >= 2.0 * s_vs(d_i, &dev).max(0.0);
    let verdict = if c2 && c3 && c4 { "PASS" } else if sd < 0.2 || sv <= 0.0 { "FAIL" } else { "AMBIGUOUS" };
    md += "
## Decision (criteria.md)

";
    md += &format!("- (1) S(P+Xsign,DEV) = {:+.3} (>=0.5), improves {}/{} (>=80%): {}
", sd, id.0, id.1, c2);
    md += &format!("- (2) S(P+Xsign,VAL) = {:+.3} (>=0.25), improves {}/{} (>=7): {}
", sv, iv.0, iv.1, c3);
    md += &format!("- (3) S(P+Xsign,DEV) >= 2 x max(S(P+dummysign,DEV)={:+.3}, 0): {}
", s_vs(d_i, &dev), c4);
    md += &format!("
**Verdict: {verdict}.**
");
    std::fs::write(out.join("report.md"), &md).unwrap();
    let mut csv = String::from("image,split,arm,bytes,vs_P_pct,exact
");
    for (i, im) in imgs.iter().enumerate() {
        for (k, arm) in XS_ARMS.iter().enumerate() {
            let r = &results[i][k];
            csv += &format!("{},{},{},{},{:+.4},{}
", im.name, if im.dev { "dev" } else { "val" }, arm.name, r.bytes, pct(results[i][c_i].bytes as f64, r.bytes as f64), r.exact);
        }
    }
    std::fs::write(out.join("per_image.csv"), csv).unwrap();
    println!("{md}");
}

/// Codec 2 (the control/baseline these research runs were frozen against); the production default is now codec 3.
fn encode(c: &nanoscope::canon::Canonical) -> Result<Vec<u8>, nanoscope::pkonfile::PkonError> {
    encode_with_codec(c, CODEC_X4P)
}
