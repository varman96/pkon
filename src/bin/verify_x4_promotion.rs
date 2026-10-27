//! Verification of the N3 promotion: production `x4_cfg()` must reproduce the N3 full-trial bytes exactly on the 20 JPEG-origin
//! images (parsed from the N3 report), and is measured against the frozen M6 X4 on the 3 PNG-origin images (not assumed to transfer).
use nanoscope::canon::Canonical;
use nanoscope::pkon2::*;
use nanoscope::pkon2x::*;
use std::path::PathBuf;

fn main() {
    let data = PathBuf::from("data");
    let m6: Vec<serde_json::Value> = {
        let mut d: Vec<PathBuf> = std::fs::read_dir("results").unwrap().filter_map(|e| e.ok()).map(|e| e.path()).filter(|p| p.to_string_lossy().ends_with("-pkon2-m6")).collect();
        d.sort();
        serde_json::from_str(&std::fs::read_to_string(d.pop().unwrap().join("results.json")).unwrap()).unwrap()
    };
    let manifest: Vec<serde_json::Value> = serde_json::from_str(&std::fs::read_to_string(data.join("manifest.json")).unwrap()).unwrap();
    let n3 = std::fs::read_to_string("results/20261027-exp-n3-full20/report.md").unwrap();
    let n3_bytes = |name: &str| -> usize {
        let l = n3.lines().find(|l| l.starts_with(&format!("| {name} |")) || l.starts_with(&format!("| {name} * |"))).unwrap();
        l.split('|').nth(4).unwrap().trim().parse().unwrap()
    };
    let prod = x4_cfg();
    let mut out = String::from("# X4 promotion verification (production `x4_cfg()` = context A x MED branch)\n\n| image | src | frozen M6 X4 B | production B | change | N3 report B | matches N3 | exact | enc ms | dec ms |\n|---|---|---:|---:|---:|---:|---|---|---:|---:|\n");
    let (mut jf, mut jp, mut pf, mut pp, mut ok, mut allx) = (0usize, 0usize, 0usize, 0usize, true, true);
    for src in ["jpeg", "png"] {
        for rec in m6.iter().filter(|x| x["src"] == src) {
            let name = rec["name"].as_str().unwrap();
            let m = manifest.iter().find(|m| m["name"] == name).unwrap();
            let img = center_crop(&Canonical::load(&data.join(m["file"].as_str().unwrap())).unwrap(), 1024);
            let (mut e, mut d, mut o) = (f64::MAX, f64::MAX, None);
            for _ in 0..2 { let x = measure_x(&img, &prod); e = e.min(x.enc_ms); d = d.min(x.dec_ms); o = Some(x); }
            let o = o.unwrap();
            let frozen = rec["runs"][5]["coded"].as_u64().unwrap() as usize;
            let (mt, n3s) = if src == "jpeg" { let n = n3_bytes(name); ok &= n == o.coded_bytes; (if n == o.coded_bytes { "yes".to_string() } else { "NO".to_string() }, n.to_string()) } else { ("n/a".into(), "-".into()) };
            allx &= o.exact;
            if src == "jpeg" { jf += frozen; jp += o.coded_bytes } else { pf += frozen; pp += o.coded_bytes }
            out += &format!("| {name} | {src} | {frozen} | {} | {:+.3}% | {n3s} | {mt} | {} | {e:.0} | {d:.0} |\n", o.coded_bytes, (o.coded_bytes as f64 / frozen as f64 - 1.0) * 100.0, o.exact);
            eprintln!("{name} {src}: {frozen} -> {} exact={}", o.coded_bytes, o.exact);
        }
    }
    out += &format!("\nJPEG (20): frozen M6 {jf} -> production {jp} ({:+.3}%); equals 15,687,437 baseline: {}; equals N3 aggregate 15,598,046: {}; every image matches N3 report: {ok}.\nPNG (3): frozen M6 {pf} -> production {pp} ({:+.3}%).\nAll exact: {allx}.\n", (jp as f64 / jf as f64 - 1.0) * 100.0, jf == 15687437, jp == 15598046, (pp as f64 / pf as f64 - 1.0) * 100.0);
    let dir = PathBuf::from("results").join("20261027-x4-promotion-verify");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("report.md"), &out).unwrap();
    println!("{out}");
}
