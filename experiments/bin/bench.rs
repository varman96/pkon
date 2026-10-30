//! Sanity benchmark: run every available baseline over the corpus, gate on exactness,
//! write results.json / results.csv / summary.md into a fresh historical/<timestamp>/ dir.
//!
//! usage: bench [--dir PNG_DIR] [--out historical] [--reps 3] [--allow-debug]
use nanoscope::canon::{first_mismatch, Canonical};
use nanoscope::codec::{Codec, PngCodec, RawCodec};
use nanoscope::corpus::{self, Entry};
use nanoscope::envinfo;
use nanoscope::ext::Ffmpeg;
use nanoscope::verify::{round_trip, truncation_check, TripError, Verdict};
use serde::Serialize;
use std::fmt::Write as _;
use std::path::PathBuf;

#[derive(Serialize, Clone)]
struct Row {
    image: String,
    kind: &'static str,
    content_hash: String,
    format: String,
    width: u32,
    height: u32,
    source_file_bytes: Option<u64>,
    canonical_bytes: usize,
    codec: String,
    codec_version: String,
    codec_config: String,
    /// ok | mismatch | suspect | unsupported | failed | unavailable
    status: &'static str,
    detail: String,
    encoded_bytes: Option<usize>,
    container_overhead_bytes: Option<u64>,
    /// canonical_bytes / encoded_bytes (>1 = smaller than raw samples). Counts the whole
    /// encoded file, container included.
    ratio: Option<f64>,
    bits_per_pixel: Option<f64>,
    encode_ms_best: Option<f64>,
    decode_ms_best: Option<f64>,
}

fn utc_stamp() -> String {
    let secs = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs() as i64;
    let (days, rem) = (secs.div_euclid(86400), secs.rem_euclid(86400));
    // civil-from-days (Howard Hinnant)
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

fn base_row(e: &Entry, c: &dyn Codec) -> Row {
    Row {
        image: e.name.clone(),
        kind: e.kind,
        content_hash: e.image.content_hash(),
        format: e.image.spec.label(),
        width: e.image.spec.width,
        height: e.image.spec.height,
        source_file_bytes: e.source_bytes,
        canonical_bytes: e.image.data.len(),
        codec: c.name(),
        codec_version: c.version(),
        codec_config: c.config(),
        status: "ok",
        detail: String::new(),
        encoded_bytes: None,
        container_overhead_bytes: None,
        ratio: None,
        bits_per_pixel: None,
        encode_ms_best: None,
        decode_ms_best: None,
    }
}

/// Gate: the comparator and the raw reference must behave before any number is trusted.
/// (`cargo test --release` is the fuller gate; this is the cheap in-run one.)
fn self_check(entries: &[Entry]) -> Result<(), String> {
    for e in entries {
        let mut flipped: Canonical = e.image.clone();
        let last = flipped.data.len() - 1;
        flipped.data[last] ^= 1;
        if first_mismatch(&e.image, &flipped).is_none() {
            return Err(format!("comparator missed a 1-bit flip in {}", e.name));
        }
        let t = round_trip(&RawCodec, &e.image, 1).map_err(|x| format!("raw on {}: {x}", e.name))?;
        if !matches!(t.verdict, Verdict::Exact) || t.encoded.len() != e.image.data.len() + 10 {
            return Err(format!("raw reference not exact / mis-accounted on {}", e.name));
        }
        truncation_check(&RawCodec, &e.image, &t.encoded).map_err(|x| format!("raw on {}: {x}", e.name))?;
    }
    Ok(())
}

fn measure(e: &Entry, c: &dyn Codec, reps: u32) -> Row {
    let mut r = base_row(e, c);
    if !c.supports(&e.image.spec) {
        r.status = "unsupported";
        r.detail = format!("{} not representable losslessly by this codec/invocation", r.format);
        return r;
    }
    match round_trip(c, &e.image, reps) {
        Err(TripError::Unsupported(s)) => {
            r.status = "unsupported";
            r.detail = s;
        }
        Err(err) => {
            r.status = "failed";
            r.detail = err.to_string();
        }
        Ok(t) => {
            let n = t.encoded.len();
            r.encoded_bytes = Some(n);
            r.container_overhead_bytes = c.container_overhead(&t.encoded);
            r.ratio = Some(e.image.data.len() as f64 / n as f64);
            r.bits_per_pixel = Some(n as f64 * 8.0 / (e.image.spec.width as f64 * e.image.spec.height as f64));
            r.encode_ms_best = Some(t.enc_time.as_secs_f64() * 1000.0);
            r.decode_ms_best = Some(t.dec_time.as_secs_f64() * 1000.0);
            match t.verdict {
                Verdict::Mismatch(m) => {
                    r.status = "mismatch";
                    r.detail = m.to_string();
                }
                Verdict::Exact => {
                    if let Err(why) = truncation_check(c, &e.image, &t.encoded) {
                        r.status = "suspect";
                        r.detail = why;
                    }
                }
            }
        }
    }
    r
}

fn csv_field(s: &str) -> String {
    if s.contains([',', '"', '\n']) { format!("\"{}\"", s.replace('"', "\"\"")) } else { s.to_string() }
}

fn main() {
    let mut args = std::env::args().skip(1);
    let (mut dir, mut out, mut reps, mut allow_debug) = (None::<PathBuf>, PathBuf::from("historical"), 3u32, false);
    while let Some(a) = args.next() {
        match a.as_str() {
            "--dir" => dir = args.next().map(PathBuf::from),
            "--out" => out = PathBuf::from(args.next().expect("--out needs a path")),
            "--reps" => reps = args.next().and_then(|s| s.parse().ok()).expect("--reps needs a number"),
            "--allow-debug" => allow_debug = true,
            other => {
                eprintln!("unknown argument {other}\nusage: bench [--dir PNG_DIR] [--out historical] [--reps 3] [--allow-debug]");
                std::process::exit(2);
            }
        }
    }
    let env = envinfo::collect();
    if env.debug_assertions && !allow_debug {
        eprintln!("refusing to time a debug build; run `cargo run --release --bin bench` (or pass --allow-debug)");
        std::process::exit(2);
    }

    let mut entries = corpus::synthetic();
    let mut skipped = vec![];
    if let Some(d) = &dir {
        match corpus::from_dir(d) {
            Ok((mut ex, sk)) => {
                entries.append(&mut ex);
                skipped = sk;
            }
            Err(e) => {
                eprintln!("{e}");
                std::process::exit(2);
            }
        }
    }
    eprintln!("corpus: {} images ({} skipped as unrepresentable)", entries.len(), skipped.len());

    if let Err(e) = self_check(&entries) {
        eprintln!("SELF-CHECK FAILED, results would not be trustworthy: {e}");
        std::process::exit(1);
    }
    eprintln!("self-check ok (comparator, raw reference, truncation check)");

    let codecs: Vec<Box<dyn Codec>> = vec![
        Box::new(RawCodec),
        Box::new(PngCodec),
        Box::new(Ffmpeg::jxl(7)),
        Box::new(Ffmpeg::jxl(9)),
        Box::new(Ffmpeg::webp(6)),
    ];

    let mut rows: Vec<Row> = vec![];
    let mut unavailable: Vec<(String, String)> = vec![];
    for c in &codecs {
        if let Err(why) = c.available() {
            eprintln!("{}: UNAVAILABLE ({why})", c.name());
            unavailable.push((c.name(), why.clone()));
            for e in &entries {
                let mut r = base_row(e, c.as_ref());
                r.status = "unavailable";
                r.detail = why.clone();
                rows.push(r);
            }
            continue;
        }
        let t0 = std::time::Instant::now();
        for e in &entries {
            rows.push(measure(e, c.as_ref(), reps));
        }
        eprintln!("{}: done in {:.1}s", c.name(), t0.elapsed().as_secs_f64());
    }

    // ---- write evidence ----
    let stamp = utc_stamp();
    std::fs::create_dir_all(&out).expect("create results dir");
    let mut run_dir = out.join(&stamp);
    let mut n = 1;
    while run_dir.exists() {
        run_dir = out.join(format!("{stamp}-{n}"));
        n += 1;
    }
    std::fs::create_dir(&run_dir).expect("create run dir (never overwrites)");

    let config = serde_json::json!({
        "reps": reps, "timing": "min over reps of Codec::encode / Codec::decode only",
        "corpus_dir": dir.as_ref().map(|d| d.display().to_string()),
        "skipped_inputs": skipped, "unavailable_codecs": unavailable,
        "ratio_definition": "canonical_bytes / encoded_bytes; encoded_bytes is the whole output incl. container",
    });
    let doc = serde_json::json!({ "env": env, "config": config, "rows": rows });
    std::fs::write(run_dir.join("results.json"), serde_json::to_string_pretty(&doc).unwrap()).unwrap();

    let mut csv = String::from("image,kind,content_hash,format,width,height,source_file_bytes,canonical_bytes,codec,status,encoded_bytes,container_overhead_bytes,ratio,bits_per_pixel,encode_ms_best,decode_ms_best,detail\n");
    let f = |o: Option<f64>| o.map(|v| format!("{v:.4}")).unwrap_or_default();
    let u = |o: Option<u64>| o.map(|v| v.to_string()).unwrap_or_default();
    for r in &rows {
        let _ = writeln!(
            csv,
            "{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{}",
            csv_field(&r.image), r.kind, r.content_hash, r.format, r.width, r.height, u(r.source_file_bytes),
            r.canonical_bytes, csv_field(&r.codec), r.status, u(r.encoded_bytes.map(|v| v as u64)),
            u(r.container_overhead_bytes), f(r.ratio), f(r.bits_per_pixel), f(r.encode_ms_best),
            f(r.decode_ms_best), csv_field(&r.detail)
        );
    }
    std::fs::write(run_dir.join("results.csv"), csv).unwrap();

    let summary = summarize(&env, &rows, &entries, reps, &unavailable, &skipped);
    std::fs::write(run_dir.join("summary.md"), &summary).unwrap();
    println!("{summary}\nevidence written to {}", run_dir.display());
}

fn summarize(
    env: &envinfo::EnvInfo,
    rows: &[Row],
    entries: &[Entry],
    reps: u32,
    unavailable: &[(String, String)],
    skipped: &[(String, String)],
) -> String {
    let mut s = String::new();
    let mut codecs: Vec<String> = vec![];
    for r in rows {
        if !codecs.contains(&r.codec) {
            codecs.push(r.codec.clone());
        }
    }
    let get = |img: &str, c: &str| rows.iter().find(|r| r.image == img && r.codec == c).unwrap();
    let synth = entries.iter().all(|e| e.kind == "synthetic");

    let _ = writeln!(s, "# Nanoscope sanity benchmark\n");
    let _ = writeln!(
        s,
        "**Corpus: {} images, {}.** {}Initial machinery check only: tiny sample, no statistical claims, \
         and nothing here says any future codec is competitive.\n",
        entries.len(),
        if synth { "all SYNTHETIC (diagnostic, not natural images)" } else { "synthetic + external" },
        ""
    );
    let _ = writeln!(s, "- Build: {} {} opt={} debug_assertions={} target={}", env.rustc, env.profile, env.opt_level, env.debug_assertions, env.target);
    let _ = writeln!(s, "- Host: {} | {} | {} logical CPUs | git {}", env.os, env.cpu, env.logical_cpus, env.git_describe);
    let _ = writeln!(s, "- Compiled features: {:?}; CPU runtime features: {:?}", env.compiled_target_features, env.cpu_runtime_features);
    let _ = writeln!(s, "- ffmpeg: {} (no-op spawn median {} ms)", env.ffmpeg.as_deref().unwrap_or("NOT FOUND"), env.ffmpeg_spawn_ms.map_or("n/a".into(), |v| format!("{v:.0}")));
    let _ = writeln!(s, "- Ratio = canonical sample bytes / whole encoded bytes (higher is better). Byte counts are the primary result.");
    let _ = writeln!(s, "- Timing: min of {reps} runs of encode/decode only. **ffmpeg-based timings include process spawn + temp-file I/O (floor ≈ the spawn figure above) and are not comparable to in-process png-rs/raw.** Peak memory not measured.\n");

    if !unavailable.is_empty() {
        let _ = writeln!(s, "## Unavailable codecs (not run)\n");
        for (c, w) in unavailable {
            let _ = writeln!(s, "- {c}: {w}");
        }
        let _ = writeln!(s);
    }
    if !skipped.is_empty() {
        let _ = writeln!(s, "## External inputs skipped (unrepresentable, not converted)\n");
        for (f, w) in skipped {
            let _ = writeln!(s, "- {f}: {w}");
        }
        let _ = writeln!(s);
    }

    let _ = writeln!(s, "## Encoded bytes per image (ratio)\n");
    let _ = write!(s, "| image | format | canonical B |");
    for c in &codecs {
        let _ = write!(s, " {c} |");
    }
    let _ = writeln!(s);
    let _ = write!(s, "|---|---|---:|");
    for _ in &codecs {
        let _ = write!(s, "---:|");
    }
    let _ = writeln!(s);
    for e in entries {
        let _ = write!(s, "| {} | {} | {} |", e.name, e.image.spec.label(), e.image.data.len());
        for c in &codecs {
            let r = get(&e.name, c);
            let cell = match r.status {
                "ok" => format!("{} ({:.2}x)", r.encoded_bytes.unwrap(), r.ratio.unwrap()),
                "mismatch" => format!("**MISMATCH** ({} B)", r.encoded_bytes.unwrap_or(0)),
                "suspect" => "**SUSPECT**".into(),
                "unsupported" => "unsupported".into(),
                "unavailable" => "unavailable".into(),
                _ => "**FAILED**".into(),
            };
            let _ = write!(s, " {cell} |");
        }
        let _ = writeln!(s);
    }

    // Totals only over images where every compared codec produced an exact result.
    let totals = |s: &mut String, title: &str, set: &[&str]| {
        let set: Vec<&String> = codecs.iter().filter(|c| set.iter().any(|p| c.starts_with(p))).collect();
        if set.is_empty() {
            return;
        }
        if set.iter().any(|c| rows.iter().any(|r| &r.codec == *c && r.status == "unavailable")) {
            return;
        }
        let common: Vec<&Entry> = entries.iter().filter(|e| set.iter().all(|c| get(&e.name, c).status == "ok")).collect();
        let _ = writeln!(s, "\n### {title}: {} of {} images exact for all of [{}]\n", common.len(), entries.len(),
            set.iter().map(|c| c.as_str()).collect::<Vec<_>>().join(", "));
        if common.is_empty() {
            return;
        }
        let canon: usize = common.iter().map(|e| e.image.data.len()).sum();
        let _ = writeln!(s, "| codec | encoded B | ratio | enc ms (sum of bests) | dec ms (sum of bests) |\n|---|---:|---:|---:|---:|");
        for c in &set {
            let (mut b, mut te, mut td) = (0usize, 0.0, 0.0);
            for e in &common {
                let r = get(&e.name, c);
                b += r.encoded_bytes.unwrap();
                te += r.encode_ms_best.unwrap();
                td += r.decode_ms_best.unwrap();
            }
            let _ = writeln!(s, "| {c} | {b} | {:.3}x | {te:.1} | {td:.1} |", canon as f64 / b as f64);
        }
    };
    let _ = writeln!(s, "\n## Totals (only images every listed codec reproduced exactly)");
    totals(&mut s, "All baselines", &["raw", "png-rs", "jxl-e7", "jxl-e9", "webp"]);
    totals(&mut s, "PNG vs JPEG XL", &["png-rs", "jxl-e7", "jxl-e9"]);

    let bad: Vec<&Row> = rows.iter().filter(|r| !matches!(r.status, "ok" | "unsupported" | "unavailable")).collect();
    let _ = writeln!(s, "\n## Correctness findings\n");
    if bad.is_empty() {
        let _ = writeln!(s, "All runnable (codec, image) pairs were bit-exact and passed the truncation check.");
    }
    for r in bad {
        let _ = writeln!(s, "- **{}** on `{}` → {}: {}", r.codec, r.image, r.status, r.detail);
    }
    s
}
