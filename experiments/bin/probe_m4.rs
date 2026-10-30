//! PKON2 milestone 4: does a CALIC-style error-energy context beat the activity context?
//!
//! usage: probe_m4 [--data data] [--crop 1024] [--out results] [--m1 results/<m1 dir>]
//!
//! HYPOTHESIS: the 3-pixel activity bucket (pixel-domain gradients) is a weaker predictor of the next
//! residual's magnitude than E = zz(eW)+zz(eN)+zz(eNW)+zz(eNE) (what MED actually missed nearby), so
//! replacing it (same 8 bucket edges, same x4 previous-plane residual buckets, same model count)
//! shrinks actual coded bytes. Overlap caveat: both measure local roughness; the test is whether the
//! residual-domain version adds information. CALIC's thresholds are NOT reused.
//!
//! PREDECLARED DECISION RULES (written before the first measurement run), primary comparison =
//! Med/SubG/ErrX vs frozen Med/SubG/ActX, total coded bytes on the 20 camera JPEGs:
//!   KEEP        aggregate >= 1.0% smaller AND smaller on >= 15/20 images AND no image > 0.5% larger
//!               AND all images exact AND encode and decode time each <= 1.25x baseline.
//!   REJECT      aggregate < 0.3% smaller, or any mismatch.
//!   INVESTIGATE anything else; report names ONE follow-up.
//!   Rationale: a context swap adds no state (same 32 models/plane, cur_z already exists) and no
//!   signalling, so the cost is only time; the bar must clear the ~0.76% adaptive-learning gap seen in
//!   M1-M3 (a gain smaller than that is within what model dilution could erase) and be in proportion to
//!   the 6%/21% M1 context effects. Bytes are deterministic, so the bar is about practical value.
//!   Ablation (reported, not part of the verdict): Err vs Act without the previous-plane term.
//!   Bucket edges are fixed to Act's 0,1-2,3-5,6-10,11-20,21-40,41-80,>80 applied to E in zigzag units
//!   (~2x sum|e|); not tuned. Baseline integrity: ActX and Act bytes must equal frozen M1 results.json.
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
    vec![
        ("Act x prev (frozen baseline)".into(), Config { pred: Pred::Med, color: Color::SubG, ctx: Ctx::ActX }),
        ("Err x prev (primary)".into(), Config { pred: Pred::Med, color: Color::SubG, ctx: Ctx::ErrX }),
        ("Act alone (frozen M1)".into(), Config { pred: Pred::Med, color: Color::SubG, ctx: Ctx::Act }),
        ("Err alone (ablation)".into(), Config { pred: Pred::Med, color: Color::SubG, ctx: Ctx::Err }),
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
    let (mut data, mut crop, mut out) = (PathBuf::from("data"), 1024u32, PathBuf::from("results"));
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
            for _ in 0..3 {
                let r = measure(&img, *c);
                best_enc = best_enc.min(r.enc_ms);
                best_dec = best_dec.min(r.dec_ms);
                o = Some(r);
            }
            let o = o.unwrap();
            runs.push(Run { cfg: label.clone(), coded: o.coded_bytes, hctx: o.hctx_bytes, enc_ms: best_enc, dec_ms: best_dec, exact: o.exact, coupled_frac: o.coupled_frac });
        }
        let m1row = m1j.iter().find(|r| r["name"] == name.as_str()).expect("image missing from M1 results");
        for (j, m1idx) in [(0usize, 9usize), (2, 6)] {
            let frozen = m1row["runs"][m1idx]["coded"].as_u64().unwrap() as usize;
            if frozen != runs[j].coded {
                baseline_ok = false;
                eprintln!("BASELINE DRIFT {name} cfg {j}: M1 {frozen} vs now {}", runs[j].coded);
            }
        }
        eprintln!("{name}: base {} err {} ({:+.2}%) exact {}", runs[0].coded, runs[1].coded, (runs[1].coded as f64 / runs[0].coded as f64 - 1.0) * 100.0, runs.iter().all(|r| r.exact));
        let src = if s("source_format").contains("jpeg") { "jpeg" } else { "png" };
        imgs.push(Img { name, category: s("category").trim_end_matches('2').into(), src: src.into(), samples: img.spec.width as usize * img.spec.height as usize * img.spec.layout.channels(), runs });
    }
    let report = report(&imgs, &list, baseline_ok, &m1);
    let dir = out.join(format!("{}-pkon2-m4", utc_stamp()));
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("report.md"), &report).unwrap();
    std::fs::write(dir.join("results.json"), serde_json::to_string_pretty(&imgs).unwrap()).unwrap();
    println!("{report}\nwritten to {}", dir.display());
}

fn report(imgs: &[Img], list: &[(String, Config)], baseline_ok: bool, m1: &std::path::Path) -> String {
    let mut s = String::new();
    let n = list.len();
    writeln!(s, "# PKON2 milestone 4: error-energy context vs activity context\n").unwrap();
    writeln!(s, "Baseline = Med/SubG/ActX (and Act-alone) from `{}`; baseline bytes identical to frozen M1 on every image: **{baseline_ok}**. Coded bytes include the {HEADER_BYTES}-byte header and all adaptive-model learning; the context swap adds no signalling. Timings: min of 3 runs, single-threaded. All configs exact on all images: **{}**.\n", m1.display(), imgs.iter().all(|i| i.runs.iter().all(|r| r.exact))).unwrap();
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
    let sum = |j: usize| g.iter().map(|i| i.runs[j].coded as f64).sum::<f64>();
    let agg = 1.0 - sum(1) / sum(0);
    let smaller = g.iter().filter(|i| i.runs[1].coded < i.runs[0].coded).count();
    let worst = g.iter().map(|i| i.runs[1].coded as f64 / i.runs[0].coded as f64 - 1.0).fold(f64::MIN, f64::max);
    let t = |j: usize, f: &dyn Fn(&Run) -> f64| g.iter().map(|i| f(&i.runs[j])).sum::<f64>();
    let (er, dr) = (t(1, &|r| r.enc_ms) / t(0, &|r| r.enc_ms), t(1, &|r| r.dec_ms) / t(0, &|r| r.dec_ms));
    let exact = imgs.iter().all(|i| i.runs.iter().all(|r| r.exact));
    let verdict = if !exact || !baseline_ok || agg < 0.003 {
        "REJECT"
    } else if agg >= 0.01 && smaller >= 15 && worst <= 0.005 && er <= 1.25 && dr <= 1.25 {
        "KEEP"
    } else {
        "INVESTIGATE"
    };
    writeln!(s, "
## Predeclared verdict (JPEG n=20)

aggregate gain {:.3}% (keep >= 1.0%, reject < 0.3%), smaller on {smaller}/20 (keep >= 15), worst image {:+.3}% (keep <= +0.5%), enc time x{er:.2}, dec time x{dr:.2} (keep <= 1.25), exact: {exact}. **Verdict: {verdict}**
", agg * 100.0, worst * 100.0).unwrap();
    let ab = 1.0 - sum(3) / sum(2);
    let ab_smaller = g.iter().filter(|i| i.runs[3].coded < i.runs[2].coded).count();
    writeln!(s, "## Ablation (JPEG n=20): Err alone vs Act alone (no previous-plane term)

aggregate {:+.3}% bytes, Err smaller on {ab_smaller}/20 images.
", -ab * 100.0).unwrap();
    s
}
