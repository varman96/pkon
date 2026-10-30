//! EXPERIMENT N3 full trial: the exact N3 config (context A x MED branch, `In::Ab`) vs unchanged X4 on all 20 JPEG-origin M6 images.
//! Thresholds unchanged from N3: CANDIDATE >= 0.5% smaller aggregate, no image > +0.1% larger, all exact, enc/dec <= 1.15x control
//! (and smaller on >= 5/6 of the original six-image subset, reported separately). REJECT < 0.2% aggregate or any mismatch.
use nanoscope::canon::Canonical;
use nanoscope::pkon2::*;
use nanoscope::pkon2x::*;
use std::path::PathBuf;

const SUBSET: [&str; 6] = ["indoor_0", "document_1", "outdoor_1", "pet_1", "foliage_0", "night_0"];

fn main() {
    let data = PathBuf::from("data");
    let m6: Vec<serde_json::Value> = {
        let mut d: Vec<PathBuf> = std::fs::read_dir("historical").unwrap().filter_map(|e| e.ok()).map(|e| e.path()).filter(|p| p.to_string_lossy().ends_with("-pkon2-m6")).collect();
        d.sort();
        serde_json::from_str(&std::fs::read_to_string(d.pop().unwrap().join("results.json")).unwrap()).unwrap()
    };
    let manifest: Vec<serde_json::Value> = serde_json::from_str(&std::fs::read_to_string(data.join("manifest.json")).unwrap()).unwrap();
    let mk = |name, a| XCfg { name, inputs: vec![a, In::B, In::C, In::D], sign_ctx: true, bias: Bias::Off, ens: Ens::Off };
    let cfgs = [mk("X4 control", In::A), mk("N3 A x MED branch", In::Ab)];
    let mut out = String::from("# Experiment N3 full trial: MED branch identity, 20 JPEG-origin images\n\n| image | bps | control B | N3 B | change | exact | enc ms ctl/N3 | dec ms ctl/N3 |\n|---|---:|---:|---:|---:|---|---|---|\n");
    // (control, n3, enc ctl, enc n3, dec ctl, dec n3, smaller count, worst, exact) for all and subset
    #[derive(Default)]
    struct Agg { c: f64, n: f64, ec: f64, en: f64, dc: f64, dn: f64, smaller: i32, cnt: i32, worst: f64 }
    let (mut all, mut sub, mut allx) = (Agg { worst: f64::MIN, ..Default::default() }, Agg { worst: f64::MIN, ..Default::default() }, true);
    let mut m6sum = 0usize;
    for rec in m6.iter().filter(|x| x["src"] == "jpeg") {
        let name = rec["name"].as_str().unwrap();
        let m = manifest.iter().find(|m| m["name"] == name).unwrap();
        let img = center_crop(&Canonical::load(&data.join(m["file"].as_str().unwrap())).unwrap(), 1024);
        let r: Vec<_> = cfgs.iter().map(|c| {
            let (mut e, mut d, mut o) = (f64::MAX, f64::MAX, None);
            for _ in 0..2 { let x = measure_x(&img, c); e = e.min(x.enc_ms); d = d.min(x.dec_ms); o = Some(x); }
            (o.unwrap(), e, d)
        }).collect();
        let frozen = rec["runs"][5]["coded"].as_u64().unwrap() as usize;
        assert_eq!(frozen, r[0].0.coded_bytes, "control drift on {name}");
        m6sum += frozen;
        let (c, n) = (r[0].0.coded_bytes as f64, r[1].0.coded_bytes as f64);
        let ex = r[0].0.exact && r[1].0.exact;
        let samples = (img.spec.width * img.spec.height * 3) as f64;
        out += &format!("| {name}{} | {:.3} | {c} | {n} | {:+.3}% | {ex} | {:.0}/{:.0} | {:.0}/{:.0} |\n", if SUBSET.contains(&name) { " *" } else { "" }, c * 8.0 / samples, (n / c - 1.0) * 100.0, r[0].1, r[1].1, r[0].2, r[1].2);
        eprintln!("{name}: {c} -> {n} ({:+.3}%) exact={ex}", (n / c - 1.0) * 100.0);
        allx &= ex;
        for (a, on) in [(&mut all, true), (&mut sub, SUBSET.contains(&name))] {
            if !on { continue; }
            a.c += c; a.n += n; a.ec += r[0].1; a.en += r[1].1; a.dc += r[0].2; a.dn += r[1].2;
            a.smaller += (n < c) as i32; a.cnt += 1; a.worst = a.worst.max(n / c - 1.0);
        }
    }
    let line = |t: &str, a: &Agg| format!("- **{t}** ({} images): {} -> {} ({:+.3}%), smaller on {}/{}, worst image {:+.3}%, enc x{:.2}, dec x{:.2}\n", a.cnt, a.c, a.n, (a.n / a.c - 1.0) * 100.0, a.smaller, a.cnt, a.worst * 100.0, a.en / a.ec, a.dn / a.dc);
    out += &format!("\n(* = original six-image subset)\n\nFrozen M6 X4 JPEG total: {m6sum} (control total {} equals it: {}).\n\n", all.c, all.c as usize == m6sum);
    out += &line("Full set", &all);
    out += &line("Original subset", &sub);
    let gain = 1.0 - all.n / all.c;
    let verdict = if !allx || gain < 0.002 { "REJECT" } else if gain >= 0.005 && sub.smaller >= 5 && all.worst <= 0.001 && all.en <= 1.15 * all.ec && all.dn <= 1.15 * all.dc { "CANDIDATE" } else { "INCONCLUSIVE" };
    out += &format!("\nAll exact: {allx}. **Verdict: {verdict}**\n");
    let dir = PathBuf::from("historical").join("20261027-exp-n3-full20");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("report.md"), &out).unwrap();
    println!("{out}");
}
