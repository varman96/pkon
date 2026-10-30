//! PKON2 milestone 6: binarized count-adaptive coder + logistic context mixing (see src/pkon2x.rs).
//!
//! usage: probe_m6 [--data data] [--crop 1024] [--out results] [--m1 results/<m1 dir>] [--only N]
//!
//! HYPOTHESIS: the baseline's 256-ary frequency tables make every extra context expensive (M5 dilution
//! control +0.34%). Binarizing the residual and mixing per-decision predictions from several contexts
//! lets more information in without that cost. Ablation ladder separates the contributions:
//!   X1   binarization + count-adaptive bit models, baseline context only   (coder effect alone)
//!   X1s  + sign conditioned on neighbour / previous-plane residual signs   (new information)
//!   X2   + M5 context as a second mixed input
//!   X4   + neighbour-shape and wide-energy inputs                           (candidate)
//! PREDECLARED RULES (candidate = last config vs frozen Med/SubG/ActX, 20 JPEGs, total coded bytes):
//!   KEEP >= 1.0% smaller, smaller on >= 75% of images, no image > +0.5% larger, all exact (time is reported,
//!   not gated: a mixing coder is expected to be slower). REJECT < 0.3% smaller or any mismatch. Else INVESTIGATE.
//! Baseline integrity: ActX bytes must equal frozen M1 results.json. Timings: single run each.
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
    vec![
        XCfg { name: "X1 bin, ctx A", inputs: vec![In::A], sign_ctx: false, bias: Bias::Off, ens: Ens::Off },
        XCfg { name: "X1s bin, ctx A, sign ctx", inputs: vec![In::A], sign_ctx: true, bias: Bias::Off, ens: Ens::Off },
        XCfg { name: "X2 mix A+C, sign ctx", inputs: vec![In::A, In::C], sign_ctx: true, bias: Bias::Off, ens: Ens::Off },
        XCfg { name: "X4 mix A+B+C+D, sign ctx (candidate)", inputs: vec![In::A, In::B, In::C, In::D], sign_ctx: true, bias: Bias::Off, ens: Ens::Off },
    ]
}
fn names() -> Vec<String> {
    let mut v = vec!["Act x prev, 256-ary (frozen baseline)".to_string(), "Act x Err3 x prev, 256-ary (M5, 96)".to_string()];
    v.extend(xcfgs().iter().map(|c| c.name.to_string()));
    v
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
    let (mut data, mut crop, mut out) = (PathBuf::from("data"), 1024u32, PathBuf::from("results"));
    let (mut m1, mut only): (Option<PathBuf>, usize) = (None, usize::MAX);
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--data" => data = args[{ i += 1; i }].clone().into(),
            "--crop" => crop = args[{ i += 1; i }].parse().unwrap(),
            "--out" => out = args[{ i += 1; i }].clone().into(),
            "--m1" => m1 = Some(args[{ i += 1; i }].clone().into()),
            "--only" => only = args[{ i += 1; i }].parse().unwrap(),
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
    let base = Config { pred: Pred::Med, color: Color::SubG, ctx: Ctx::ActX };
    let m5 = Config { pred: Pred::Med, color: Color::SubG, ctx: Ctx::ActErrX };
    let xs = xcfgs();
    let nm = names();
    let mut imgs: Vec<Img> = vec![];
    let mut baseline_ok = true;
    for m in manifest.iter().take(only) {
        let s = |k: &str| m[k].as_str().unwrap_or("").to_string();
        let name = s("name");
        let img = center_crop(&Canonical::load(&data.join(s("file"))).unwrap(), crop);
        let mut runs = vec![];
        for (j, c) in [base, m5].iter().enumerate() {
            let o = measure(&img, *c);
            runs.push(Run { cfg: nm[j].clone(), coded: o.coded_bytes, enc_ms: o.enc_ms, dec_ms: o.dec_ms, exact: o.exact, state_bytes: nmodels(img.spec.layout.channels(), c.ctx) * 256 * 2 });
        }
        for (j, c) in xs.iter().enumerate() {
            let o = measure_x(&img, c);
            runs.push(Run { cfg: nm[j + 2].clone(), coded: o.coded_bytes, enc_ms: o.enc_ms, dec_ms: o.dec_ms, exact: o.exact, state_bytes: o.state_bytes });
        }
        let frozen = m1j.iter().find(|r| r["name"] == name.as_str()).expect("image missing from M1 results")["runs"][9]["coded"].as_u64().unwrap() as usize;
        if frozen != runs[0].coded {
            baseline_ok = false;
            eprintln!("BASELINE DRIFT {name}: M1 {frozen} vs now {}", runs[0].coded);
        }
        let b = runs[0].coded as f64;
        eprintln!("{name}: {}", runs.iter().map(|r| format!("{:+.2}%", (r.coded as f64 / b - 1.0) * 100.0)).collect::<Vec<_>>().join(" "));
        let src = if s("source_format").contains("jpeg") { "jpeg" } else { "png" };
        imgs.push(Img { name, category: s("category").trim_end_matches('2').into(), src: src.into(), samples: img.spec.width as usize * img.spec.height as usize * img.spec.layout.channels(), runs });
    }
    let report = report(&imgs, &nm, baseline_ok, &m1);
    if only == usize::MAX {
        let dir = out.join(format!("{}-pkon2-m6", utc_stamp()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("report.md"), &report).unwrap();
        std::fs::write(dir.join("results.json"), serde_json::to_string_pretty(&imgs).unwrap()).unwrap();
        println!("{report}\nwritten to {}", dir.display());
    } else {
        println!("{report}\n(--only {only}: not written)");
    }
}

fn report(imgs: &[Img], nm: &[String], baseline_ok: bool, m1: &std::path::Path) -> String {
    let mut s = String::new();
    let n = nm.len();
    writeln!(s, "# PKON2 milestone 6: binarized adaptive coder + logistic mixing\n").unwrap();
    writeln!(s, "Baseline = Med/SubG/ActX from `{}`; baseline bytes identical to frozen M1 on every image: **{baseline_ok}**. All configs use the identical decoded pixels, MED predictor, SubG transform and 11-byte header; only the entropy coder differs. Timings: single run each (single-threaded). All configs exact on all images: **{}**.\n", m1.display(), imgs.iter().all(|i| i.runs.iter().all(|r| r.exact))).unwrap();
    for (title, src) in [("Camera JPEG photos (primary)", "jpeg"), ("Non-JPEG-origin (PNG) images, reported separately", "png")] {
        let g: Vec<&Img> = imgs.iter().filter(|i| i.src == src).collect();
        if g.is_empty() {
            continue;
        }
        writeln!(s, "## {title}, n={}\n", g.len()).unwrap();
        writeln!(s, "| config | total bytes | bits/sample | vs baseline (aggregate) | mean per-image | smaller/larger | worst image | enc ms/img | dec ms/img | state KiB |\n|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|").unwrap();
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
    writeln!(s, "## Per image: bytes and change vs frozen baseline\n").unwrap();
    write!(s, "| image | cat | src | baseline B |").unwrap();
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
    let c = n - 1;
    let sum = |j: usize| g.iter().map(|i| i.runs[j].coded as f64).sum::<f64>();
    let agg = 1.0 - sum(c) / sum(0);
    let smaller = g.iter().filter(|i| i.runs[c].coded < i.runs[0].coded).count();
    let worst = g.iter().map(|i| i.runs[c].coded as f64 / i.runs[0].coded as f64 - 1.0).fold(f64::MIN, f64::max);
    let exact = imgs.iter().all(|i| i.runs.iter().all(|r| r.exact));
    let t = |j: usize, f: &dyn Fn(&Run) -> f64| g.iter().map(|i| f(&i.runs[j])).sum::<f64>();
    let verdict = if !exact || !baseline_ok || agg < 0.003 {
        "REJECT"
    } else if agg >= 0.01 && smaller * 4 >= 3 * g.len() && worst <= 0.005 {
        "KEEP"
    } else {
        "INVESTIGATE"
    };
    writeln!(s, "\n## Predeclared verdict (JPEG, candidate = last config)\n\naggregate gain {:.3}% (keep >= 1.0%, reject < 0.3%), smaller on {smaller}/{} (keep >= 75%), worst image {:+.3}% (keep <= +0.5%), exact: {exact}; enc time x{:.2}, dec time x{:.2} vs baseline (reported, not gated). **Verdict: {verdict}**\n",
        agg * 100.0, g.len(), worst * 100.0, t(c, &|r| r.enc_ms) / t(0, &|r| r.enc_ms), t(c, &|r| r.dec_ms) / t(0, &|r| r.dec_ms)).unwrap();
    s
}
