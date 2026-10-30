//! REL2: the one final diagnostic. Is P's REL1 gain attributable to predictor-disagreement information, or to "an extra adaptive mixer input"?
//! Adds a one-class constant-context arm (`Extra::Const`) to the REL1 harness. Frozen rules: historical/20261027-rel2-final-diag/criteria.md.
//!
//!   cargo run --release --bin rel2 -- --out historical/20261027-rel2-final-diag
//!
//! Loads ONLY the 25 development + 10 validation images of historical/learn1-splits.json (never the test list).
use nanoscope::canon::{Canonical, Layout};
use nanoscope::pkon2::center_crop;
use nanoscope::pkon2x::{measure_x, x4_cfg};
use nanoscope::pkonfile::encode_payload;
use nanoscope::pkonlearn::prepared;
use nanoscope::pkonrel::*;
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;

const CROP: u32 = 1024;
const SPLITS: &str = "historical/learn1-splits.json";
const CRITERIA_SHA: &str = "050c72195dc4d6c64805bdfd52f20589e78084f74f891bf6f711afb6c7b324a4";
const REL1_CSV: &str = "historical/20261027-rel1-pred-xplane/per_image.csv";

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
    Res { bytes: e.coded_bytes, exact, trace_ok: d.trace == e.trace, state_ok: d.state == e.state, repeat_ok: e.stream == e2.stream && e.trace == e2.trace, overread: d.consumed > e.stream.len() + 4, bits: e.bits, stream: e.stream }
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
        let m: serde_json::Value = serde_json::from_str(&std::fs::read_to_string("benchmarks/pkon2/20261027-155433Z-bench50/dataset_manifest.json").unwrap()).unwrap();
        m["images"].as_array().unwrap().iter().map(|i| (i["name"].as_str().unwrap().to_string(), i["crop_sha256"].as_str().unwrap().to_string())).collect()
    };
    // REL1 reference bytes for the cross-run repeatability gate G6
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

    let na = DIAG_ARMS.len();
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
                let (im, arm) = (&imgs[i], &DIAG_ARMS[ai]);
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
                if ai == 0 {
                    let prod = encode_payload(&im.canon);
                    if prod != r.stream || measure_x(&im.canon, &x4_cfg()).coded_bytes != r.bytes {
                        why.push("control != production".into());
                    }
                }
                if let Some(&b) = rel1.get(&(im.name.clone(), arm.name.to_string())) {
                    if b != r.bytes {
                        why.push(format!("bytes {} != REL1 {}", r.bytes, b));
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
    println!("gates G1-G6 PASS on {} x {} runs", imgs.len(), na);

    // timing: single thread, VAL, best of 2
    let mut timing = vec![(0.0f64, 0.0f64); na];
    for (ai, arm) in DIAG_ARMS.iter().enumerate() {
        for im in imgs.iter().filter(|i| !i.dev) {
            let e1 = encode_rel(&im.sub, arm, None);
            let e2 = encode_rel(&im.sub, arm, None);
            let d1 = decode_rel(&e1.stream, arm, im.sub.w, im.sub.h, 3);
            let d2 = decode_rel(&e1.stream, arm, im.sub.w, im.sub.h, 3);
            timing[ai].0 += e1.ms.min(e2.ms);
            timing[ai].1 += d1.ms.min(d2.ms);
        }
    }

    let ix = |n: &str| DIAG_ARMS.iter().position(|a| a.name == n).unwrap();
    let (c_i, k_i, d_i, p_i, pk_i) = (ix("control"), ix("const"), ix("dummy"), ix("P"), ix("P+const"));
    let dev = |im: &Img| im.dev;
    let val = |im: &Img| !im.dev;
    let sum = |ai: usize, f: &dyn Fn(&Img) -> bool| (0..imgs.len()).filter(|&i| f(&imgs[i])).map(|i| results[i][ai].bytes as f64).sum::<f64>();
    let s_vs = |base: usize, ai: usize, f: &dyn Fn(&Img) -> bool| pct(sum(base, f), sum(ai, f));
    let imp_vs = |base: usize, ai: usize, f: &dyn Fn(&Img) -> bool| -> (usize, usize) {
        let idx: Vec<usize> = (0..imgs.len()).filter(|&i| f(&imgs[i])).collect();
        (idx.iter().filter(|&&i| results[i][ai].bytes < results[i][base].bytes).count(), idx.len())
    };
    let mut md = String::from("# REL2: final diagnostic (constant-context bias input vs predictor-disagreement P)\n\n");
    md += &format!("Criteria frozen before the run (sha256 {CRITERIA_SHA}); REL1 criteria untouched. DEV 25 / VAL 10, test list never loaded. Sizes = stream + 11 B header.\n\n**Correctness gates G1-G6: PASS** (exact pixels incl. interleaved compare, control == production `encode_payload` and `measure_x`, enc/dec trace+state hashes equal, re-encode identical, no over-read, control/dummy/P bytes equal REL1 per-image bytes on all 35 images).\n\n");
    md += "## Aggregate (savings vs control)\n\n| arm | DEV bytes | S(DEV) % | improves | VAL bytes | S(VAL) % | improves | enc ms (VAL, 1 thr) | dec ms |\n|---|---|---|---|---|---|---|---|---|\n";
    for (k, arm) in DIAG_ARMS.iter().enumerate() {
        let (a1, b1) = imp_vs(c_i, k, &dev);
        let (a2, b2) = imp_vs(c_i, k, &val);
        md += &format!("| {} | {} | {:+.3} | {}/{} | {} | {:+.3} | {}/{} | {:.0} | {:.0} |\n", arm.name, sum(k, &dev) as u64, s_vs(c_i, k, &dev), a1, b1, sum(k, &val) as u64, s_vs(c_i, k, &val), a2, b2, timing[k].0, timing[k].1);
    }
    md += "\n## Bias-controlled attribution (savings relative to the `const` arm)\n\n| arm | Sc(DEV) % | improves vs const | Sc(VAL) % | improves vs const |\n|---|---|---|---|---|\n";
    for k in [p_i, pk_i, d_i] {
        let (a1, b1) = imp_vs(k_i, k, &dev);
        let (a2, b2) = imp_vs(k_i, k, &val);
        md += &format!("| {} | {:+.3} | {}/{} | {:+.3} | {}/{} |\n", DIAG_ARMS[k].name, s_vs(k_i, k, &dev), a1, b1, s_vs(k_i, k, &val), a2, b2);
    }
    md += &format!("\nP+const vs P alone (does bias add anything once P is present?): DEV {:+.3} %, VAL {:+.3} %.\n", s_vs(p_i, pk_i, &dev), s_vs(p_i, pk_i, &val));
    md += "\n## Cross-entropy by decision type (DEV, % fewer bits vs control)\n\n| arm | zero | ge2 | class | mantissa | sign | total |\n|---|---|---|---|---|---|---|\n";
    let ce = |ai: usize, c: Option<usize>| -> f64 {
        (0..imgs.len())
            .filter(|&i| imgs[i].dev)
            .map(|i| (0..MAXP).map(|k| (0..NCAT).filter(|&cc| c.map_or(true, |x| x == cc)).map(|cc| results[i][ai].bits[k][cc]).sum::<f64>()).sum::<f64>())
            .sum()
    };
    for k in 1..na {
        md += &format!("| {} |", DIAG_ARMS[k].name);
        for c in 0..NCAT {
            md += &format!(" {:+.3} |", pct(ce(0, Some(c)), ce(k, Some(c))));
        }
        md += &format!(" {:+.3} |\n", pct(ce(0, None), ce(k, None)));
    }
    let (sc_dev, sc_val) = (s_vs(k_i, pk_i, &dev), s_vs(k_i, pk_i, &val));
    let (imp_dev, imp_val) = (imp_vs(k_i, pk_i, &dev), imp_vs(k_i, pk_i, &val));
    let sp = s_vs(c_i, p_i, &dev);
    let ctrl_max = s_vs(c_i, k_i, &dev).max(s_vs(c_i, d_i, &dev));
    let c2 = sc_dev >= 0.5 && imp_dev.0 as f64 >= 0.8 * imp_dev.1 as f64;
    let c3 = sc_val >= 0.25 && imp_val.0 >= 7;
    let c4 = sp >= 2.0 * ctrl_max;
    let supported = c2 && c3 && c4;
    let contradicted = sc_dev < 0.2 || (imp_dev.0 as f64) < 0.5 * imp_dev.1 as f64;
    md += "\n## Decision (criteria.md)\n\n";
    md += &format!("- (2) Sc(P+const,DEV) = {:+.3} (>=0.5), improves {}/{} (>=80%): {}\n", sc_dev, imp_dev.0, imp_dev.1, c2);
    md += &format!("- (3) Sc(P+const,VAL) = {:+.3} (>=0.25), improves {}/{} (>=7): {}\n", sc_val, imp_val.0, imp_val.1, c3);
    md += &format!("- (4) S(P,DEV) = {:+.3} >= 2 x max(S(const)={:+.3}, S(dummy)={:+.3}) = {:.3}: {}\n", sp, s_vs(c_i, k_i, &dev), s_vs(c_i, d_i, &dev), 2.0 * ctrl_max, c4);
    let verdict = if supported { "P SUPPORTED" } else if contradicted { "P CONTRADICTED" } else { "AMBIGUOUS" };
    md += &format!("\n**Verdict: {verdict}.** REL1's recorded CONFOUNDED label (S(dummy,DEV) >= 0.10) is not altered by this diagnostic.\n");
    std::fs::write(out.join("report.md"), &md).unwrap();
    let mut csv = String::from("image,split,arm,bytes,vs_control_pct,vs_const_pct,exact\n");
    for (i, im) in imgs.iter().enumerate() {
        for (k, arm) in DIAG_ARMS.iter().enumerate() {
            let r = &results[i][k];
            csv += &format!("{},{},{},{},{:+.4},{:+.4},{}\n", im.name, if im.dev { "dev" } else { "val" }, arm.name, r.bytes, pct(results[i][c_i].bytes as f64, r.bytes as f64), pct(results[i][k_i].bytes as f64, r.bytes as f64), r.exact);
        }
    }
    std::fs::write(out.join("per_image.csv"), csv).unwrap();
    println!("{md}");
}
