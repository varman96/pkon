//! PKON2 milestone 7: adaptive bias correction of the MED prediction in front of the frozen X4 coder.
//!
//! usage: probe_m7 [--data data] [--crop 1024] [--out historical] [--m6 historical/<m6 dir>] [--only N]
//!
//! HYPOTHESIS: M6's sign contexts gained ~2.4% (JPEG), i.e. MED's signed errors correlate with neighbouring
//! signed errors and with earlier planes' errors at the same pixel. Learning that mean error per context
//! (JPEG-LS A/N accumulators, CALIC error feedback) and subtracting it BEFORE entropy coding should leave
//! residuals X4 codes more cheaply. Ladder: P1 neighbours only; P2 + previous plane; P3 + B sees both R and G.
//! D1/D3 (dead-zone variants) were ADDED after the first 3-image smoke run showed P1-P3 losing ~2%: they test whether
//! rounding the MEAN error moves the prediction off the mode. They are diagnostics and do not enter the verdict.
//! The X4 coder, SubG transform, MED predictor and all contexts are unchanged; only the prediction is shifted.
//! PREDECLARED RULES (written before the first run; same bars as M4/M5, now vs X4; candidate = P3, JPEG n=20):
//!   KEEP        aggregate >= 1.0% smaller AND smaller on >= 15/20 AND no image > 0.5% larger AND all exact
//!               AND encode and decode time each <= 1.25x X4.
//!   REJECT      aggregate < 0.3% smaller, or any mismatch.
//!   INVESTIGATE anything else; report names ONE follow-up.
//! Integrity: the X4 control bytes must equal the X4 column of the frozen M6 results.json.
//! Timings: min of 3 runs, single-threaded.
use nanoscope::canon::Canonical;
use nanoscope::pkon2::*;
use nanoscope::pkon2x::*;
use serde::Serialize;
use std::fmt::Write as _;
use std::path::PathBuf;

#[derive(Serialize, Clone)]
struct Run {
    cfg: String,
    coded: usize,
    enc_ms: f64,
    dec_ms: f64,
    exact: bool,
    state_bytes: usize,
}
#[derive(Serialize)]
struct Img {
    name: String,
    category: String,
    src: String,
    samples: usize,
    runs: Vec<Run>,
}

fn xcfgs() -> Vec<XCfg> {
    let mk = |name, bias| XCfg { name, inputs: vec![In::A, In::B, In::C, In::D], sign_ctx: true, bias, ens: Ens::Off };
    vec![
        mk("X4 control (no bias correction)", Bias::Off),
        mk("P1 bias: W,N signed errors", Bias::Nb),
        mk("P2 bias: + previous plane (R<-G, B<-R)", Bias::NbPrev),
        mk("P3 bias: + B<-R and G (candidate)", Bias::NbBoth),
        mk("D1 diag: P1 with dead-zone (trunc)", Bias::NbTrunc),
        mk("D3 diag: P3 with dead-zone (trunc)", Bias::NbBothTrunc),
    ]
}
fn names() -> Vec<String> {
    xcfgs().iter().map(|c| c.name.to_string()).collect()
}

fn utc_stamp() -> String {
    let secs = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs() as i64;
    let (days, rem) = (secs.div_euclid(86400), secs.rem_euclid(86400));
    let z = days + 719468;
    let era = z.div_euclid(146097);
    let doe = z.rem_euclid(146097);
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + (m <= 2) as i64;
    format!("{y:04}{m:02}{d:02}-{:02}{:02}{:02}Z", rem / 3600, rem % 3600 / 60, rem % 60)
}

fn main() {
    let (mut data, mut crop, mut out) = (PathBuf::from("data"), 1024u32, PathBuf::from("historical"));
    let (mut m6, mut only): (Option<PathBuf>, usize) = (None, usize::MAX);
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--data" => data = args[{ i += 1; i }].clone().into(),
            "--crop" => crop = args[{ i += 1; i }].parse().unwrap(),
            "--out" => out = args[{ i += 1; i }].clone().into(),
            "--m6" => m6 = Some(args[{ i += 1; i }].clone().into()),
            "--only" => only = args[{ i += 1; i }].parse().unwrap(),
            a => panic!("unknown argument {a}"),
        }
        i += 1;
    }
    assert!(!cfg!(debug_assertions), "run with --release");
    let m6 = m6.unwrap_or_else(|| {
        let mut d: Vec<PathBuf> = std::fs::read_dir(&out).unwrap().filter_map(|e| e.ok()).map(|e| e.path()).filter(|p| p.to_string_lossy().ends_with("-pkon2-m6")).collect();
        d.sort();
        d.pop().expect("no M6 results dir")
    });
    let m6j: Vec<serde_json::Value> = serde_json::from_str(&std::fs::read_to_string(m6.join("results.json")).unwrap()).unwrap();
    let manifest: Vec<serde_json::Value> = serde_json::from_str(&std::fs::read_to_string(data.join("manifest.json")).unwrap()).unwrap();
    let xs = xcfgs();
    let nm = names();
    let mut imgs: Vec<Img> = vec![];
    let mut baseline_ok = true;
    for m in manifest.iter().take(only) {
        let s = |k: &str| m[k].as_str().unwrap_or("").to_string();
        let name = s("name");
        let img = center_crop(&Canonical::load(&data.join(s("file"))).unwrap(), crop);
        let mut runs = vec![];
        for (j, c) in xs.iter().enumerate() {
            let (mut be, mut bd, mut last) = (f64::MAX, f64::MAX, None);
            for _ in 0..3 {
                let o = measure_x(&img, c);
                be = be.min(o.enc_ms);
                bd = bd.min(o.dec_ms);
                last = Some(o);
            }
            let o = last.unwrap();
            // extra state beyond the coder: accumulators + the signed-error planes kept for contexts
            let extra = if c.bias == Bias::Off { 0 } else { 1225 * 8 + img.spec.layout.channels() * img.spec.width as usize * img.spec.height as usize };
            runs.push(Run { cfg: nm[j].clone(), coded: o.coded_bytes, enc_ms: be, dec_ms: bd, exact: o.exact, state_bytes: o.state_bytes + extra });
        }
        let frozen = m6j.iter().find(|r| r["name"] == name.as_str()).expect("image missing from M6 results")["runs"][5]["coded"].as_u64().unwrap() as usize;
        if frozen != runs[0].coded {
            baseline_ok = false;
            eprintln!("X4 DRIFT {name}: M6 {frozen} vs now {}", runs[0].coded);
        }
        let b = runs[0].coded as f64;
        eprintln!("{name}: {}", runs.iter().map(|r| format!("{:+.2}%", (r.coded as f64 / b - 1.0) * 100.0)).collect::<Vec<_>>().join(" "));
        let src = if s("source_format").contains("jpeg") { "jpeg" } else { "png" };
        imgs.push(Img { name, category: s("category").trim_end_matches('2').into(), src: src.into(), samples: img.spec.width as usize * img.spec.height as usize * img.spec.layout.channels(), runs });
    }
    let report = report(&imgs, &nm, baseline_ok, &m6);
    if only == usize::MAX {
        let dir = out.join(format!("{}-pkon2-m7", utc_stamp()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("report.md"), &report).unwrap();
        std::fs::write(dir.join("results.json"), serde_json::to_string_pretty(&imgs).unwrap()).unwrap();
        println!("{report}\nwritten to {}", dir.display());
    } else {
        println!("{report}\n(--only {only}: not written)");
    }
}

fn report(imgs: &[Img], nm: &[String], baseline_ok: bool, m6: &std::path::Path) -> String {
    let mut s = String::new();
    let n = nm.len();
    writeln!(s, "# PKON2 milestone 7: adaptive bias correction in front of X4\n").unwrap();
    writeln!(s, "Control = X4 (frozen M6, `{}`); control bytes identical to frozen M6 on every image: **{baseline_ok}**. All configs use the identical decoded pixels, SubG transform, X4 coder and 11-byte header; only the prediction differs. Timings: min of 3 runs (single-threaded). All configs exact on all images: **{}**.\n", m6.display(), imgs.iter().all(|i| i.runs.iter().all(|r| r.exact))).unwrap();
    for (title, src) in [("Camera JPEG photos (primary)", "jpeg"), ("Non-JPEG-origin (PNG) images, reported separately", "png")] {
        let g: Vec<&Img> = imgs.iter().filter(|i| i.src == src).collect();
        if g.is_empty() {
            continue;
        }
        writeln!(s, "## {title}, n={}\n", g.len()).unwrap();
        writeln!(s, "| config | total bytes | bits/sample | vs X4 (aggregate) | mean per-image | smaller/larger | worst image | enc ms/img | dec ms/img | state KiB |\n|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|").unwrap();
        let tot = |j: usize| g.iter().map(|i| i.runs[j].coded as f64).sum::<f64>();
        let k = g.len() as f64;
        for j in 0..n {
            let d: Vec<f64> = g.iter().map(|i| i.runs[j].coded as f64 / i.runs[0].coded as f64 - 1.0).collect();
            let samples: f64 = g.iter().map(|i| i.samples as f64).sum();
            writeln!(s, "| {} | {:.0} | {:.4} | {:+.3}% | {:+.3}% | {}/{} | {:+.2}% | {:.0} | {:.0} | {:.0} |", nm[j], tot(j), tot(j) * 8.0 / samples, (tot(j) / tot(0) - 1.0) * 100.0, d.iter().sum::<f64>() / k * 100.0,
                d.iter().filter(|&&x| x < -1e-6).count(), d.iter().filter(|&&x| x > 1e-6).count(), d.iter().cloned().fold(f64::MIN, f64::max) * 100.0,
                g.iter().map(|i| i.runs[j].enc_ms).sum::<f64>() / k, g.iter().map(|i| i.runs[j].dec_ms).sum::<f64>() / k, g[0].runs[j].state_bytes as f64 / 1024.0).unwrap();
        }
        writeln!(s).unwrap();
    }
    writeln!(s, "## Per image: bytes and change vs X4 control\n").unwrap();
    write!(s, "| image | cat | src | X4 B |").unwrap();
    for j in 1..n {
        write!(s, " {} |", nm[j]).unwrap();
    }
    writeln!(s, "\n|---|---|---|---:|{}", "---:|".repeat(n - 1)).unwrap();
    for i in imgs {
        write!(s, "| {} | {} | {} | {} |", i.name, i.category, i.src, i.runs[0].coded).unwrap();
        for j in 1..n {
            write!(s, " {} ({:+.2}%) |", i.runs[j].coded, (i.runs[j].coded as f64 / i.runs[0].coded as f64 - 1.0) * 100.0).unwrap();
        }
        writeln!(s).unwrap();
    }
    let g: Vec<&Img> = imgs.iter().filter(|i| i.src == "jpeg").collect();
    let c = 3; // P3 (candidate); D1/D3 are diagnostics
    let sum = |j: usize| g.iter().map(|i| i.runs[j].coded as f64).sum::<f64>();
    let agg = 1.0 - sum(c) / sum(0);
    let smaller = g.iter().filter(|i| i.runs[c].coded < i.runs[0].coded).count();
    let worst = g.iter().map(|i| i.runs[c].coded as f64 / i.runs[0].coded as f64 - 1.0).fold(f64::MIN, f64::max);
    let exact = imgs.iter().all(|i| i.runs.iter().all(|r| r.exact));
    let t = |j: usize, f: &dyn Fn(&Run) -> f64| g.iter().map(|i| f(&i.runs[j])).sum::<f64>();
    let verdict = if !exact || !baseline_ok || agg < 0.003 {
        "REJECT"
    } else if agg >= 0.01 && smaller * 4 >= 3 * g.len() && worst <= 0.005 && t(c, &|r| r.enc_ms) <= 1.25 * t(0, &|r| r.enc_ms) && t(c, &|r| r.dec_ms) <= 1.25 * t(0, &|r| r.dec_ms) {
        "KEEP"
    } else {
        "INVESTIGATE"
    };
    writeln!(s, "\n## Predeclared verdict (JPEG, candidate = P3)\n\naggregate gain {:.3}% (keep >= 1.0%, reject < 0.3%), smaller on {smaller}/{} (keep >= 15/20), worst image {:+.3}% (keep <= +0.5%), exact: {exact}; enc time x{:.2}, dec time x{:.2} vs X4 (keep <= 1.25). **Verdict: {verdict}**\n",
        agg * 100.0, g.len(), worst * 100.0, t(c, &|r| r.enc_ms) / t(0, &|r| r.enc_ms), t(c, &|r| r.dec_ms) / t(0, &|r| r.dec_ms)).unwrap();
    s
}
