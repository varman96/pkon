//! PKON2 milestone 2, stage A: does a decoder-reproducible, signalling-free choice between the SubG and
//! Coupled cross-channel predictions beat the best fixed transform?
//!
//! usage: probe_m2 [--data data] [--crop 1024] [--out historical] [--m1 historical/<m1 dir>]
//!
//! PREDECLARED DECISION RULES (written before the first measurement run; primary = Adaptive(4)):
//!   Primary comparison: total coded bytes on the 20 camera-JPEG images, vs frozen baseline Med/SubG/ActX.
//!   Rationale for the bar: the coder is deterministic (no run-to-run noise in bytes), so "noise" is
//!   image-to-image spread plus the 0.6% adaptive-learning gap seen in M1; the rule adds state and ~extra
//!   time, so it must clear more than the learning gap.
//!   KEEP        aggregate >= 1.0% smaller AND smaller on >= 15/20 images AND no image > 0.3% larger
//!               AND every image decodes exactly.
//!   REJECT      aggregate < 0.3% smaller (or any mismatch).
//!   INVESTIGATE anything in between; the report must name the condition that explains the spread.
//!   Window sizes 2 and 6 are sensitivity checks only, not candidates for the primary verdict.
//!   Baseline integrity: Med/SubG/ActX bytes must equal the frozen M1 results.json byte-for-byte.
use nanoscope::canon::Canonical;
use nanoscope::pkon2::*;
use serde::Serialize;
use std::fmt::Write as _;
use std::path::PathBuf;

#[derive(Serialize, Clone)]
struct Run {
    cfg: String,
    coded: usize,
    hctx: f64,
    enc_ms: f64,
    dec_ms: f64,
    exact: bool,
    coupled_frac: f64,
}

#[derive(Serialize)]
struct Img {
    name: String,
    category: String,
    src: String,
    samples: usize,
    runs: Vec<Run>,
}

fn cfgs() -> Vec<(String, Config)> {
    let c = |color| Config { pred: Pred::Med, color, ctx: Ctx::ActX };
    vec![
        ("baseline SubG (frozen)".into(), c(Color::SubG)),
        ("fixed Coupled".into(), c(Color::Coupled)),
        ("Adaptive win4 (primary)".into(), c(Color::Adaptive(4))),
        ("Adaptive win2 (sens.)".into(), c(Color::Adaptive(2))),
        ("Adaptive win6 (sens.)".into(), c(Color::Adaptive(6))),
    ]
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
    let mut m1: Option<PathBuf> = None;
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--data" => data = args[{ i += 1; i }].clone().into(),
            "--crop" => crop = args[{ i += 1; i }].parse().unwrap(),
            "--out" => out = args[{ i += 1; i }].clone().into(),
            "--m1" => m1 = Some(args[{ i += 1; i }].clone().into()),
            a => panic!("unknown argument {a}"),
        }
        i += 1;
    }
    assert!(!cfg!(debug_assertions), "run with --release");
    let m1 = m1.unwrap_or_else(|| {
        let mut d: Vec<PathBuf> = std::fs::read_dir(&out).unwrap().filter_map(|e| e.ok()).map(|e| e.path()).filter(|p| p.to_string_lossy().ends_with("-pkon2-m1")).collect();
        d.sort();
        d.pop().expect("no M1 results dir")
    });
    let m1j: Vec<serde_json::Value> = serde_json::from_str(&std::fs::read_to_string(m1.join("results.json")).unwrap()).unwrap();
    let manifest: Vec<serde_json::Value> = serde_json::from_str(&std::fs::read_to_string(data.join("manifest.json")).unwrap()).unwrap();
    let list = cfgs();
    let mut imgs: Vec<Img> = vec![];
    let mut baseline_ok = true;
    for m in &manifest {
        let s = |k: &str| m[k].as_str().unwrap_or("").to_string();
        let name = s("name");
        let full = Canonical::load(&data.join(s("file"))).unwrap();
        let img = center_crop(&full, crop);
        let mut runs = vec![];
        for (label, c) in &list {
            let (mut best_enc, mut best_dec) = (f64::MAX, f64::MAX);
            let mut o = None;
            for _ in 0..2 {
                let r = measure(&img, *c);
                best_enc = best_enc.min(r.enc_ms);
                best_dec = best_dec.min(r.dec_ms);
                o = Some(r);
            }
            let o = o.unwrap();
            runs.push(Run { cfg: label.clone(), coded: o.coded_bytes, hctx: o.hctx_bytes, enc_ms: best_enc, dec_ms: best_dec, exact: o.exact, coupled_frac: o.coupled_frac });
        }
        let m1row = m1j.iter().find(|r| r["name"] == name.as_str()).expect("image missing from M1 results");
        let frozen = m1row["runs"][9]["coded"].as_u64().unwrap() as usize;
        if frozen != runs[0].coded {
            baseline_ok = false;
            eprintln!("BASELINE DRIFT {name}: M1 {frozen} vs now {}", runs[0].coded);
        }
        eprintln!("{name}: base {} adaptive4 {} ({:+.2}%) exact {}", runs[0].coded, runs[2].coded, (runs[2].coded as f64 / runs[0].coded as f64 - 1.0) * 100.0, runs.iter().all(|r| r.exact));
        let src = if s("source_format").contains("jpeg") { "jpeg" } else { "png" };
        imgs.push(Img { name, category: s("category").trim_end_matches('2').into(), src: src.into(), samples: img.spec.width as usize * img.spec.height as usize * img.spec.layout.channels(), runs });
    }
    let report = report(&imgs, &list, baseline_ok, &m1);
    let dir = out.join(format!("{}-pkon2-m2a", utc_stamp()));
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("report.md"), &report).unwrap();
    std::fs::write(dir.join("results.json"), serde_json::to_string_pretty(&imgs).unwrap()).unwrap();
    println!("{report}\nwritten to {}", dir.display());
}

fn report(imgs: &[Img], list: &[(String, Config)], baseline_ok: bool, m1: &std::path::Path) -> String {
    let mut s = String::new();
    let n = list.len();
    writeln!(s, "# PKON2 milestone 2A: adaptive SubG/Coupled selection\n").unwrap();
    writeln!(s, "Baseline = Med/SubG/ActX from `{}`; baseline bytes identical to frozen M1 on every image: **{baseline_ok}**. Coded bytes include the {HEADER_BYTES}-byte header and all adaptive-model learning; the adaptive rule has zero signalling bits. Timings: min of 2 runs, single-threaded. All configs exact on all images: **{}**.\n", m1.display(), imgs.iter().all(|i| i.runs.iter().all(|r| r.exact))).unwrap();
    for (title, src) in [("Camera JPEG photos (primary, n=20)", "jpeg"), ("Non-JPEG-origin (PNG) images, reported separately", "png")] {
        let g: Vec<&Img> = imgs.iter().filter(|i| i.src == src).collect();
        writeln!(s, "## {title}\n").unwrap();
        writeln!(s, "| config | total bytes | bits/sample | vs baseline (aggregate) | mean per-image | smaller/larger | worst image | Hctx gap | coupled share | enc ms | dec ms |\n|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|").unwrap();
        let tot = |j: usize| g.iter().map(|i| i.runs[j].coded as f64).sum::<f64>();
        for j in 0..n {
            let d: Vec<f64> = g.iter().map(|i| i.runs[j].coded as f64 / i.runs[0].coded as f64 - 1.0).collect();
            let samples: f64 = g.iter().map(|i| i.samples as f64).sum();
            let hctx: f64 = g.iter().map(|i| i.runs[j].hctx).sum();
            let k = g.len() as f64;
            writeln!(s, "| {} | {:.0} | {:.4} | {:+.3}% | {:+.3}% | {}/{} | {:+.2}% | {:+.2}% | {:.3} | {:.0} | {:.0} |", list[j].0, tot(j), tot(j) * 8.0 / samples, (tot(j) / tot(0) - 1.0) * 100.0, d.iter().sum::<f64>() / k * 100.0,
                d.iter().filter(|&&x| x < -1e-6).count(), d.iter().filter(|&&x| x > 1e-6).count(), d.iter().cloned().fold(f64::MIN, f64::max) * 100.0,
                (tot(j) / hctx - 1.0) * 100.0, g.iter().map(|i| i.runs[j].coupled_frac).sum::<f64>() / k,
                g.iter().map(|i| i.runs[j].enc_ms).sum::<f64>() / k, g.iter().map(|i| i.runs[j].dec_ms).sum::<f64>() / k).unwrap();
        }
        writeln!(s).unwrap();
    }
    writeln!(s, "## Per image: bytes and change vs frozen baseline\n").unwrap();
    write!(s, "| image | cat | src | baseline B |").unwrap();
    for j in 1..n {
        write!(s, " {} |", list[j].0).unwrap();
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
    let agg = 1.0 - g.iter().map(|i| i.runs[2].coded as f64).sum::<f64>() / g.iter().map(|i| i.runs[0].coded as f64).sum::<f64>();
    let smaller = g.iter().filter(|i| i.runs[2].coded < i.runs[0].coded).count();
    let worst = g.iter().map(|i| i.runs[2].coded as f64 / i.runs[0].coded as f64 - 1.0).fold(f64::MIN, f64::max);
    let exact = imgs.iter().all(|i| i.runs.iter().all(|r| r.exact));
    let verdict = if !exact || !baseline_ok {
        "REJECT/INVALID (mismatch or baseline drift)"
    } else if agg >= 0.01 && smaller >= 15 && worst <= 0.003 {
        "KEEP"
    } else if agg < 0.003 {
        "REJECT"
    } else {
        "INVESTIGATE"
    };
    writeln!(s, "\n## Predeclared verdict (primary = Adaptive win4, JPEG n=20)\n\naggregate gain {:.3}% (keep >= 1.0%, reject < 0.3%), smaller on {smaller}/20 (keep >= 15), worst image {:+.3}% (keep <= +0.3%), exact: {exact}. **Verdict: {verdict}**\n", agg * 100.0, worst * 100.0).unwrap();
    s
}
