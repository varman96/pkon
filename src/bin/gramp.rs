//! GRAMP: one controlled trial of the adaptive G-plane predictor on top of production codec 2. Frozen rules: results/20261027-204338Z-gramp-ramp-only/criteria.md.
//!
//!   cargo run --release --bin gramp -- --out results/20261027-204338Z-gramp-ramp-only
//!
//! Loads ONLY the 25 development + 10 validation images of results/learn1-splits.json (never the test list). Single-threaded so timings are clean.
use nanoscope::canon::{Canonical, Layout};
use nanoscope::pkon2::center_crop;
use nanoscope::pkonfile::{crc32, encode_with_codec, HEADER_LEN, CODEC_X4P};
use nanoscope::pkonlearn::prepared;
use nanoscope::pkonrel::*;
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

const CROP: u32 = 1024;
const CRITERIA_SHA: &str = "ea7005e1f3688fd177f27da40358eef4b353812ce42b7814e73fd46871efa4d8";

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
fn interleave_rgb(planes: &[Vec<u8>], n: usize) -> Vec<u8> {
    let mut d = vec![0u8; n * 3];
    for i in 0..n {
        d[i * 3 + 1] = planes[0][i];
        d[i * 3] = planes[1][i];
        d[i * 3 + 2] = planes[2][i];
    }
    d
}

struct Img {
    name: String,
    set: &'static str,
    group: &'static str,
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
    prod_eq: bool,
    enc_ms: f64,
    dec_ms: f64,
}
fn run_one(im: &Img, arm: &Arm) -> Res {
    let (w, h) = (im.sub.w, im.sub.h);
    let e = encode_rel(&im.sub, arm, None);
    let e2 = encode_rel(&im.sub, arm, None);
    let mut file = encode(&im.canon).unwrap()[..HEADER_LEN].to_vec();
    if arm.name != "P" {
        file[6] = 3;
    }
    file[16..20].copy_from_slice(&(e.stream.len() as u32).to_le_bytes());
    file.extend_from_slice(&e.stream);
    let plen = u32::from_le_bytes(file[16..20].try_into().unwrap()) as usize;
    let d = decode_rel(&file[HEADER_LEN..HEADER_LEN + plen], arm, w, h, 3);
    let d2 = decode_rel(&file[HEADER_LEN..HEADER_LEN + plen], arm, w, h, 3);
    let planes = invert(d.planes.clone(), w, h);
    let crc_ok = u32::from_le_bytes(file[20..24].try_into().unwrap()) == crc32(&interleave_rgb(&planes, w * h)) && file.len() == HEADER_LEN + plen;
    let exact = planes == im.orig && interleave_rgb(&planes, w * h) == im.canon.data && crc_ok;
    let prod_eq = arm.name != "P" || encode(&im.canon).unwrap() == file;
    Res {
        bytes: file.len(),
        exact,
        trace_ok: d.trace == e.trace && d.state == e.state,
        state_ok: d2.state == d.state,
        repeat_ok: e.stream == e2.stream && e.trace == e2.trace,
        overread: d.consumed > e.stream.len() + 4,
        prod_eq,
        enc_ms: e.ms.min(e2.ms),
        dec_ms: d.ms.min(d2.ms),
    }
}
fn pct(base: f64, x: f64) -> f64 {
    100.0 * (x - base) / base
}

fn main() {
    let a: Vec<String> = std::env::args().collect();
    let out = PathBuf::from(a.iter().position(|x| x == "--out").map(|i| a[i + 1].clone()).expect("--out"));
    let crit = std::fs::read(out.join("criteria.md")).expect("criteria.md");
    assert_eq!(sha_bytes(&crit), CRITERIA_SHA, "criteria.md changed after it was frozen");
    let sj: serde_json::Value = serde_json::from_slice(&std::fs::read("results/learn1-splits.json").unwrap()).unwrap();
    let get = |v: &serde_json::Value, k: &str| v[k].as_array().unwrap().iter().map(|s| s.as_str().unwrap().to_string()).collect::<Vec<_>>();
    let (dev_names, val_names, test_names) = (get(&sj, "train_pool"), get(&sj, "val"), get(&sj, "test"));
    let man: Vec<serde_json::Value> = serde_json::from_str(&std::fs::read_to_string("data/dataset50.json").unwrap()).unwrap();
    let all_names: Vec<String> = man.iter().map(|m| m["name"].as_str().unwrap().to_string()).collect();
    let (rd, rv, rt) = rederive(&all_names);
    assert!(rd == dev_names && rv == val_names && rt == test_names, "split file does not match its derivation rule");
    let gj: serde_json::Value = serde_json::from_slice(&std::fs::read("results/20261027-diag1-residual-maps/groups.json").unwrap()).unwrap();
    let (lose, ctrl, other) = (get(&gj, "lose"), get(&gj, "control"), get(&gj, "other"));
    assert_eq!(lose.len() + ctrl.len() + other.len(), 35);
    assert!(lose.iter().chain(&ctrl).chain(&other).all(|n| !test_names.contains(n)));
    let crop_hash: BTreeMap<String, String> = {
        let m: serde_json::Value = serde_json::from_str(&std::fs::read_to_string("results/20261027-155433Z-bench50/dataset_manifest.json").unwrap()).unwrap();
        m["images"].as_array().unwrap().iter().map(|i| (i["name"].as_str().unwrap().to_string(), i["crop_sha256"].as_str().unwrap().to_string())).collect()
    };
    let mut imgs: Vec<Img> = vec![];
    for (names, set) in [(&dev_names, "DEV"), (&val_names, "VAL")] {
        for n in names {
            let m = man.iter().find(|m| m["name"].as_str() == Some(n)).unwrap();
            let full = Canonical::load(&Path::new("data").join(m["file"].as_str().unwrap())).unwrap();
            let crop = center_crop(&full, CROP);
            assert_eq!(crop.content_hash(), crop_hash[n], "crop hash mismatch for {n}");
            assert_eq!(crop.spec.layout, Layout::Rgb);
            let (sub, orig) = prepared(&crop);
            let group = if lose.contains(n) { "LOSE" } else if ctrl.contains(n) { "CONTROL" } else { "OTHER" };
            imgs.push(Img { name: n.clone(), set, group, canon: crop, sub, orig });
        }
    }
    println!("loaded {} images", imgs.len());
    let diags: Vec<nanoscope::gramp::RampDiag> = imgs.iter().map(|im| nanoscope::gramp::diagnose(&im.orig[0], im.sub.w, im.sub.h)).collect();
    let mut res: Vec<[Res; 2]> = vec![];
    let mut fails = vec![];
    for im in &imgs {
        let r = [run_one(im, &GR_ARMS[0]), run_one(im, &GR_ARMS[1])];
        for (ai, x) in r.iter().enumerate() {
            let mut why = vec![];
            if !x.exact {
                why.push("pixels/CRC");
            }
            if !x.trace_ok {
                why.push("enc/dec trace or state");
            }
            if !x.state_ok {
                why.push("decode not repeatable");
            }
            if !x.repeat_ok {
                why.push("re-encode differs");
            }
            if x.overread {
                why.push("over-read");
            }
            if !x.prod_eq {
                why.push("P != production");
            }
            if !why.is_empty() {
                fails.push(format!("{} / {}: {}", im.name, GR_ARMS[ai].name, why.join(", ")));
            }
        }
        println!("{} {} P={} GB={} ({:+.3}%) enc {:.0}/{:.0} ms dec {:.0}/{:.0} ms", im.name, im.group, r[0].bytes, r[1].bytes, pct(r[0].bytes as f64, r[1].bytes as f64), r[0].enc_ms, r[1].enc_ms, r[0].dec_ms, r[1].dec_ms);
        res.push(r);
    }
    // per-image csv
    let mut csv = String::from("image,set,group,baseline_bytes,candidate_bytes,change_pct,exact,enc_ms_base,enc_ms_cand,dec_ms_base,dec_ms_cand,g_samples,ramp_samples,ramp_pct,planar_differs_from_med,zero_frac_med,zero_frac_planar,mean_abs_med,mean_abs_planar\n");
    let frac = |a: u64, b: u64| if b == 0 { 0.0 } else { a as f64 / b as f64 };
    for ((im, r), d) in imgs.iter().zip(&res).zip(&diags) {
        csv += &format!(
            "{},{},{},{},{},{:.4},{},{:.1},{:.1},{:.1},{:.1},{},{},{:.4},{},{:.4},{:.4},{:.4},{:.4}\n",
            im.name, im.set, im.group, r[0].bytes, r[1].bytes, pct(r[0].bytes as f64, r[1].bytes as f64), r[1].exact && r[0].exact, r[0].enc_ms, r[1].enc_ms, r[0].dec_ms, r[1].dec_ms,
            d.samples, d.ramp, 100.0 * frac(d.ramp, d.samples), d.differs, frac(d.zero_med, d.ramp), frac(d.zero_planar, d.ramp), frac(d.abs_med, d.ramp), frac(d.abs_planar, d.ramp)
        );
    }
    std::fs::write(out.join("per_image.csv"), csv).unwrap();
    if !fails.is_empty() {
        std::fs::write(out.join("gate_failure.txt"), fails.join("\n")).unwrap();
        eprintln!("CORRECTNESS GATE FAILED:\n{}", fails.join("\n"));
        std::process::exit(2);
    }
    let grp = |g: &str| -> Vec<usize> { (0..imgs.len()).filter(|&i| g == "ALL" || imgs[i].group == g).collect() };
    let sum = |idx: &[usize], ai: usize| idx.iter().map(|&i| res[i][ai].bytes as f64).sum::<f64>();
    let t = |idx: &[usize], ai: usize, enc: bool| idx.iter().map(|&i| if enc { res[i][ai].enc_ms } else { res[i][ai].dec_ms }).sum::<f64>();
    let mut md = String::from("# GRAMP: ramp-only planar G-plane predictor, one controlled trial\n\nCriteria frozen before the run (sha256 ");
    md += &format!("{CRITERIA_SHA}). 35 DEV+VAL images, TEST never loaded. Sizes = complete file (24-byte header + payload); added metadata/framing overhead: **0 bytes** (everything is derived from decoded pixels; header differs only in the codec byte). Baseline `P` = production codec 2 (byte-identical to `pkonfile::encode` on every image). Timings single thread, best of 2.\n\n**Correctness gates: PASS** (exact pixels + CRC from the file bytes on all 35 x 2 arms, enc/dec trace+state hashes equal, repeatable decode, identical re-encode, no over-read, P == production).\n\n");
    md += "| group | images | baseline B | candidate B | change | improved | enc x | dec x |\n|---|---|---|---|---|---|---|---|\n";

    let mut summary = BTreeMap::new();
    for g in ["ALL", "LOSE", "CONTROL", "OTHER"] {
        let idx = grp(g);
        let (b, c) = (sum(&idx, 0), sum(&idx, 1));
        let imp = idx.iter().filter(|&&i| res[i][1].bytes < res[i][0].bytes).count();
        md += &format!("| {g} | {} | {} | {} | {:+.3}% | {}/{} | {:.3} | {:.3} |\n", idx.len(), b as u64, c as u64, pct(b, c), imp, idx.len(), t(&idx, 1, true) / t(&idx, 0, true), t(&idx, 1, false) / t(&idx, 0, false));
        summary.insert(g, (pct(b, c), t(&idx, 1, true) / t(&idx, 0, true), t(&idx, 1, false) / t(&idx, 0, false)));
    }
    md += "\n## Per image\n\n| image | set | group | baseline B | candidate B | change | enc ms base -> cand | dec ms base -> cand | ramp samples (%) |\n|---|---|---|---|---|---|---|---|---|\n";
    for ((im, r), d) in imgs.iter().zip(&res).zip(&diags) {
        md += &format!("| {} | {} | {} | {} | {} | {:+.3}% | {:.0} -> {:.0} | {:.0} -> {:.0} | {} ({:.2}%) |\n", im.name, im.set, im.group, r[0].bytes, r[1].bytes, pct(r[0].bytes as f64, r[1].bytes as f64), r[0].enc_ms, r[1].enc_ms, r[0].dec_ms, r[1].dec_ms, d.ramp, 100.0 * d.ramp as f64 / d.samples as f64);
    }
    // ramp diagnostics (research instrumentation over the original G plane; NOT coded-bit savings)
    md += "\n## Ramp diagnostics (residual statistics only: these are NOT coded-bit savings)\n\n| group | G samples | ramp samples | ramp % | planar != MED | zero-resid MED | zero-resid planar | mean abs MED | mean abs planar |\n|---|---|---|---|---|---|---|---|---|\n";
    for g in ["ALL", "LOSE", "CONTROL", "OTHER"] {
        let mut t = nanoscope::gramp::RampDiag::default();
        for i in grp(g) {
            let d = &diags[i];
            t.samples += d.samples;
            t.ramp += d.ramp;
            t.differs += d.differs;
            t.zero_med += d.zero_med;
            t.zero_planar += d.zero_planar;
            t.abs_med += d.abs_med;
            t.abs_planar += d.abs_planar;
        }
        let f = |a: u64| if t.ramp == 0 { 0.0 } else { a as f64 / t.ramp as f64 };
        md += &format!("| {g} | {} | {} | {:.3}% | {} ({:.1}% of ramp) | {:.4} | {:.4} | {:.4} | {:.4} |\n", t.samples, t.ramp, 100.0 * t.ramp as f64 / t.samples as f64, t.differs, 100.0 * f(t.differs), f(t.zero_med), f(t.zero_planar), f(t.abs_med), f(t.abs_planar));
    }
    md += "\nPer-region coded bits are not reported: the coder attributes bits per node category, not per pixel, so an ramp-region cost cannot be measured without changing the coder. Whole-file bytes are the only compression evidence.\n";
    let all = summary["ALL"];
    let worst = res.iter().map(|r| pct(r[0].bytes as f64, r[1].bytes as f64)).fold(f64::MIN, f64::max);
    let lose = summary["LOSE"].0;
    // candidate criteria
    let k_exact = true; // correctness gate above exits before this point on any mismatch
    let k_all = -all.0 >= 0.5;
    let k_lose = -lose >= 0.5;
    let k_worst = worst <= 0.3;
    let k_time = all.1 <= 1.15 && all.2 <= 1.15;
    // reject criteria
    let r_exact = !k_exact;
    let r_all = -all.0 < 0.2;
    let r_lose = !(lose < 0.0);
    let r_worst = worst > 1.0;
    let r_time = all.1 > 1.25 || all.2 > 1.25;
    let candidate = k_exact && k_all && k_lose && k_worst && k_time;
    let reject = r_exact || r_all || r_lose || r_worst || r_time;
    let verdict = if candidate { "CANDIDATE" } else if reject { "REJECT" } else { "INCONCLUSIVE" };
    md += &format!(
        "\n## Decision (criteria.md, predeclared)\n\n### Candidate criteria (all required)\n- bit-exact on every image: {k_exact}\n- aggregate (35) {:+.3}% (need <= -0.5%): {k_all}\n- LOSE aggregate {:+.3}% (need <= -0.5%): {k_lose}\n- worst single image {:+.3}% (need <= +0.3%): {k_worst}\n- aggregate encode x{:.3}, decode x{:.3} (need <= 1.15 each): {k_time}\n\n### Reject conditions (any)\n- reconstruction mismatch: {r_exact}\n- aggregate improvement < 0.2% ({:+.3}%): {r_all}\n- LOSE group does not improve ({:+.3}%): {r_lose}\n- worst single image > +1.0% ({:+.3}%): {r_worst}\n- encode or decode time > 1.25x (x{:.3} / x{:.3}): {r_time}\n\n**Verdict: {verdict}**\n",
        all.0, lose, worst, all.1, all.2, all.0, lose, worst, all.1, all.2
    );
    std::fs::write(out.join("report.md"), md).unwrap();
    println!("verdict: {verdict}");
}

/// Codec 2 (the control/baseline these research runs were frozen against); the production default is now codec 3.
fn encode(c: &nanoscope::canon::Canonical) -> Result<Vec<u8>, nanoscope::pkonfile::PkonError> {
    encode_with_codec(c, CODEC_X4P)
}
