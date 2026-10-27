//! EXPERIMENT N3: does MED branch identity (min / max / gradient) carry information X4 discards?
//!
//! Single change: context family A becomes A x branch (3x contexts, 96 total). B, C, D, signs, MED, SubG and the coder are unchanged.
//! The branch is a pure function of W, N, NW (the same causal neighbours MED uses), so the decoder derives it identically.
//! PREDECLARED (6 images, aggregate bytes): CANDIDATE >= 0.5% smaller, smaller on >= 5/6, no image > +0.1% larger, all exact,
//! enc and dec time each <= 1.15x control. REJECT < 0.2% smaller or any mismatch. Otherwise INCONCLUSIVE.
use nanoscope::canon::Canonical;
use nanoscope::pkon2::*;
use nanoscope::pkon2x::*;
use std::path::PathBuf;

const TEST: [&str; 6] = ["indoor_0", "document_1", "outdoor_1", "pet_1", "foliage_0", "night_0"];

fn main() {
    let data = PathBuf::from("data");
    let m6: Vec<serde_json::Value> = {
        let mut d: Vec<PathBuf> = std::fs::read_dir("results").unwrap().filter_map(|e| e.ok()).map(|e| e.path()).filter(|p| p.to_string_lossy().ends_with("-pkon2-m6")).collect();
        d.sort();
        serde_json::from_str(&std::fs::read_to_string(d.pop().unwrap().join("results.json")).unwrap()).unwrap()
    };
    let manifest: Vec<serde_json::Value> = serde_json::from_str(&std::fs::read_to_string(data.join("manifest.json")).unwrap()).unwrap();
    let mk = |name, a| XCfg { name, inputs: vec![a, In::B, In::C, In::D], sign_ctx: true, bias: Bias::Off, ens: Ens::Off };
    let cfgs = [mk("X4 control", In::A), mk("N3 A x MED branch", In::Ab)];
    let mut out = String::from("# Experiment N3: MED branch identity in context A\n\n| image | control B | N3 B | change | exact | enc ms ctl/N3 | dec ms ctl/N3 |\n|---|---:|---:|---:|---|---|---|\n");
    let (mut tc, mut tn, mut smaller, mut worst, mut allx, mut ec, mut en, mut dc, mut dn) = (0f64, 0f64, 0, f64::MIN, true, 0f64, 0f64, 0f64, 0f64);
    for name in TEST {
        let m = manifest.iter().find(|m| m["name"] == name).unwrap();
        let img = center_crop(&Canonical::load(&data.join(m["file"].as_str().unwrap())).unwrap(), 1024);
        let r: Vec<_> = cfgs.iter().map(|c| {
            let (mut e, mut d, mut o) = (f64::MAX, f64::MAX, None);
            for _ in 0..2 { let x = measure_x(&img, c); e = e.min(x.enc_ms); d = d.min(x.dec_ms); o = Some(x); }
            (o.unwrap(), e, d)
        }).collect();
        let frozen = m6.iter().find(|x| x["name"] == name).unwrap()["runs"][5]["coded"].as_u64().unwrap() as usize;
        assert_eq!(frozen, r[0].0.coded_bytes, "control drift on {name}");
        let (c, n) = (r[0].0.coded_bytes as f64, r[1].0.coded_bytes as f64);
        let ex = r[0].0.exact && r[1].0.exact;
        out += &format!("| {name} | {c} | {n} | {:+.3}% | {ex} | {:.0}/{:.0} | {:.0}/{:.0} |\n", (n / c - 1.0) * 100.0, r[0].1, r[1].1, r[0].2, r[1].2);
        eprintln!("{name}: {c} -> {n} ({:+.3}%) exact={ex}", (n / c - 1.0) * 100.0);
        tc += c; tn += n; smaller += (n < c) as i32; worst = worst.max(n / c - 1.0); allx &= ex;
        ec += r[0].1; en += r[1].1; dc += r[0].2; dn += r[1].2;
    }
    let gain = 1.0 - tn / tc;
    let verdict = if !allx || gain < 0.002 { "REJECT" } else if gain >= 0.005 && smaller >= 5 && worst <= 0.001 && en <= 1.15 * ec && dn <= 1.15 * dc { "CANDIDATE" } else { "INCONCLUSIVE" };
    out += &format!("\nAggregate: {tc} -> {tn} ({:+.3}%), smaller on {smaller}/6, worst {:+.3}%, exact {allx}, enc x{:.2}, dec x{:.2}. Control bytes equal frozen M6. **Verdict: {verdict}**\n", (tn / tc - 1.0) * 100.0, worst * 100.0, en / ec, dn / dc);
    let dir = PathBuf::from("results").join("20261027-exp-n3-med-branch");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("report.md"), &out).unwrap();
    println!("{out}");
}
