//! External-codec benchmark: production PKON2 X4 (`x4_cfg()`) vs JPEG XL lossless, WebP lossless and PNG, all fed the
//! identical decoded pixel buffers (the same 1024-px centre crops every PKON2 report used). Measurement only: no PKON2 code
//! is changed. Every stream is decoded from its bytes and compared byte-for-byte with the canonical buffer.
//!
//!   cargo run --release --bin bench_external -- --smoke     # 3 real images + a tiny odd-sized image, 1 rep, no files written
//!   cargo run --release --bin bench_external                # full 23 images -> benchmarks/pkon2/<stamp>-ext-benchmark/
use nanoscope::canon::{first_mismatch, Canonical, Layout, Spec};
use nanoscope::codec::{Codec, CodecError, PngCodec};
use nanoscope::pkon2::*;
use nanoscope::pkon2x::*;
use nanoscope::verify::{round_trip, truncation_check, Verdict};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

const CROP: u32 = 1024;
const REPS: u32 = 2;

// ---------------------------------------------------------------- ffmpeg-driven JXL / WebP

#[derive(Clone, Copy)]
enum Kind {
    Jxl(u8),
    Webp(u8),
}
struct Ff {
    kind: Kind,
    /// 0 = library default (all cores); n = `-threads n`
    threads: u32,
}
impl Ff {
    fn fmt(spec: &Spec) -> Result<&'static str, CodecError> {
        match (spec.layout, spec.bit_depth) {
            (Layout::Rgb, 8) => Ok("rgb24"),
            (Layout::Rgba, 8) => Ok("rgba"),
            _ => Err(CodecError::Unsupported(spec.label())),
        }
    }
    fn run(&self, args: &[String]) -> Result<(), CodecError> {
        let mut child = Command::new("ffmpeg")
            .args(["-hide_banner", "-loglevel", "error", "-xerror", "-y", "-nostdin"])
            .args(args)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| CodecError::Failed(e.to_string()))?;
        let deadline = Instant::now() + Duration::from_secs(240);
        loop {
            match child.try_wait() {
                Ok(Some(_)) => break,
                Ok(None) if Instant::now() > deadline => {
                    let _ = child.kill();
                    return Err(CodecError::Failed("ffmpeg timeout".into()));
                }
                Ok(None) => std::thread::sleep(Duration::from_millis(1)),
                Err(e) => return Err(CodecError::Failed(e.to_string())),
            }
        }
        let out = child.wait_with_output().map_err(|e| CodecError::Failed(e.to_string()))?;
        if out.status.success() {
            Ok(())
        } else {
            Err(CodecError::Failed(String::from_utf8_lossy(&out.stderr).lines().last().unwrap_or("ffmpeg failed").into()))
        }
    }
    fn ext(&self) -> &'static str {
        match self.kind {
            Kind::Jxl(_) => "jxl",
            Kind::Webp(_) => "webp",
        }
    }
}
struct Tmp(PathBuf);
impl Tmp {
    fn new() -> Self {
        static N: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
        let p = std::env::temp_dir().join(format!("nsx-{}-{}", std::process::id(), N.fetch_add(1, std::sync::atomic::Ordering::Relaxed)));
        std::fs::create_dir_all(&p).unwrap();
        Tmp(p)
    }
}
impl Drop for Tmp {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}
impl Codec for Ff {
    fn name(&self) -> String {
        let t = if self.threads == 0 { "mt".to_string() } else { format!("t{}", self.threads) };
        match self.kind {
            Kind::Jxl(e) => format!("jxl-e{e}-{t}"),
            Kind::Webp(l) => format!("webp-m{l}-{t}"),
        }
    }
    fn version(&self) -> String {
        String::new()
    }
    fn config(&self) -> String {
        String::new()
    }
    fn encode(&self, img: &Canonical) -> Result<Vec<u8>, CodecError> {
        let fmt = Ff::fmt(&img.spec)?;
        let tmp = Tmp::new();
        let (inp, outp) = (tmp.0.join("in.raw"), tmp.0.join(format!("o.{}", self.ext())));
        std::fs::write(&inp, &img.data).map_err(|e| CodecError::Failed(e.to_string()))?;
        let s = |x: &str| x.to_string();
        let mut a = vec![s("-f"), s("rawvideo"), s("-pix_fmt"), s(fmt), s("-video_size"), format!("{}x{}", img.spec.width, img.spec.height), s("-i"), inp.to_string_lossy().into(), s("-frames:v"), s("1")];
        match self.kind {
            Kind::Jxl(e) => a.extend([s("-c:v"), s("libjxl"), s("-distance"), s("0"), s("-effort"), e.to_string(), s("-pix_fmt"), s(fmt)]),
            // libwebp lossless takes bgra; rgb24 -> bgra with alpha=255 is exact (verified by the decode compare)
            Kind::Webp(l) => a.extend([s("-c:v"), s("libwebp"), s("-lossless"), s("1"), s("-compression_level"), l.to_string(), s("-quality"), s("100"), s("-pix_fmt"), s("bgra")]),
        }
        if self.threads > 0 {
            a.extend([s("-threads"), self.threads.to_string()]);
        }
        a.extend([s("-update"), s("1"), s("-f"), s("image2"), outp.to_string_lossy().into()]);
        self.run(&a)?;
        std::fs::read(&outp).map_err(|e| CodecError::Failed(e.to_string()))
    }
    fn decode(&self, enc: &[u8], hint: &Spec) -> Result<Canonical, CodecError> {
        let fmt = Ff::fmt(hint)?;
        let tmp = Tmp::new();
        let (inp, outp) = (tmp.0.join(format!("i.{}", self.ext())), tmp.0.join("out.raw"));
        std::fs::write(&inp, enc).map_err(|e| CodecError::Failed(e.to_string()))?;
        let s = |x: &str| x.to_string();
        let mut a = vec![];
        if self.threads > 0 {
            a.extend([s("-threads"), self.threads.to_string()]);
        }
        a.extend([s("-i"), inp.to_string_lossy().into(), s("-frames:v"), s("1"), s("-f"), s("rawvideo"), s("-pix_fmt"), s(fmt), outp.to_string_lossy().into()]);
        self.run(&a)?;
        Canonical::new(*hint, std::fs::read(&outp).map_err(|e| CodecError::Failed(e.to_string()))?).map_err(CodecError::Failed)
    }
}

/// Bytes that are container framing rather than codec payload.
fn overhead(codec: &str, enc: &[u8]) -> Option<u64> {
    if codec.starts_with("jxl") {
        // bare codestream (ff 0a signature) has no container box
        return if enc.starts_with(&[0xff, 0x0a]) { Some(0) } else { None };
    }
    if codec.starts_with("webp") {
        // RIFF(12) + chunk header(8) per chunk; payload = VP8L chunk data
        if enc.len() < 20 || &enc[0..4] != b"RIFF" || &enc[8..12] != b"WEBP" {
            return None;
        }
        let (mut pos, mut payload) = (12usize, 0u64);
        while pos + 8 <= enc.len() {
            let len = u32::from_le_bytes(enc[pos + 4..pos + 8].try_into().ok()?) as usize;
            if &enc[pos..pos + 4] == b"VP8L" {
                payload += len as u64;
            }
            pos += 8 + len + (len & 1);
        }
        return Some(enc.len() as u64 - payload);
    }
    if codec.starts_with("png") {
        return PngCodec.container_overhead(enc);
    }
    None
}

// ---------------------------------------------------------------- PKON2 X4 (production x4_cfg)

fn plane_order(l: Layout) -> Vec<usize> {
    // mirrors pkon2::order (private): Rgb -> [G,R,B]
    match l {
        Layout::Rgb => vec![1, 0, 2],
        Layout::Rgba => vec![1, 0, 2, 3],
        l => (0..l.channels()).collect(),
    }
}

struct PkonRun {
    stream: Vec<u8>,
    enc: Duration,
    dec: Duration,
    verdict: Verdict,
}

/// Same pipeline as `measure_x`, but keeps the stream and rebuilds the interleaved canonical buffer so the comparison is
/// against the canonical bytes. The timed regions include plane split / colour transform / interleave (measure_x excludes them).
fn pkon_rt(img: &Canonical, cfg: &XCfg, reps: u32) -> PkonRun {
    let (mut be, mut bd) = (Duration::MAX, Duration::MAX);
    let mut keep = None;
    for _ in 0..reps.max(1) {
        let t = Instant::now();
        let mut work = to_planes(img);
        let np = work.p.len();
        color_fwd(&mut work, Color::SubG);
        let mut coder = Coder::new(np, cfg);
        let mut enc = BinEnc::new();
        run_x(&mut work, cfg, &mut coder, &mut enc);
        let stream = enc.finish();
        be = be.min(t.elapsed());

        let t = Instant::now();
        let dec = pkon_decode(&stream, &img.spec, cfg);
        bd = bd.min(t.elapsed());
        let verdict = match first_mismatch(img, &dec) {
            None => Verdict::Exact,
            Some(m) => Verdict::Mismatch(m),
        };
        if keep.is_none() {
            keep = Some((stream, verdict));
        }
    }
    let (stream, verdict) = keep.unwrap();
    PkonRun { stream, enc: be, dec: bd, verdict }
}

fn pkon_decode(stream: &[u8], spec: &Spec, cfg: &XCfg) -> Canonical {
    let (w, h, ch) = (spec.width as usize, spec.height as usize, spec.layout.channels());
    let mut pl = Planes { w, h, p: vec![vec![0; w * h]; ch] };
    let mut coder = Coder::new(ch, cfg);
    run_x(&mut pl, cfg, &mut coder, &mut BinDec::new(stream));
    color_inv(&mut pl, Color::SubG);
    let mut data = vec![0u8; w * h * ch];
    for (j, &c) in plane_order(spec.layout).iter().enumerate() {
        for i in 0..w * h {
            data[i * ch + c] = pl.p[j][i];
        }
    }
    Canonical::new(*spec, data).unwrap()
}

// ---------------------------------------------------------------- driver

struct Img {
    name: String,
    src: &'static str,
    full: (u32, u32),
    img: Canonical,
    notes: Vec<String>,
}

fn load_images(only: Option<usize>) -> Vec<Img> {
    let data = PathBuf::from("data");
    let manifest: Vec<serde_json::Value> = serde_json::from_str(&std::fs::read_to_string(data.join("manifest.json")).unwrap()).unwrap();
    let mut v = vec![];
    for src in ["jpeg", "png"] {
        for m in manifest.iter().filter(|m| m["source_format"].as_str().unwrap().ends_with(src)) {
            let full = Canonical::load(&data.join(m["file"].as_str().unwrap())).unwrap();
            let crop = center_crop(&full, CROP);
            v.push(Img { name: m["name"].as_str().unwrap().into(), src: if src == "jpeg" { "jpeg" } else { "png" }, full: (full.spec.width, full.spec.height), notes: full.notes.clone(), img: crop });
        }
    }
    if let Some(n) = only {
        // smoke: first jpeg, a mid jpeg, and the first screen png
        let pick = [0usize, n, v.len() - 2];
        v = v.into_iter().enumerate().filter(|(i, _)| pick.contains(i)).map(|(_, x)| x).collect();
    }
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

#[derive(Clone)]
struct Rec {
    image: String,
    src: &'static str,
    codec: String,
    bytes: usize,
    payload: Option<u64>,
    raw: usize,
    pixels: usize,
    enc_ms: f64,
    dec_ms: f64,
    exact: bool,
    note: String,
}

fn ms(d: Duration) -> f64 {
    d.as_secs_f64() * 1e3
}

fn tiny_image() -> Canonical {
    // 33x17 odd-sized RGB gradient+noise, used for spawn calibration and an edge-case check
    let (w, h) = (33u32, 17u32);
    let mut x = 12345u32;
    let data = (0..w * h * 3)
        .map(|i| {
            x = x.wrapping_mul(1664525).wrapping_add(1013904223);
            ((i / 3) as u8).wrapping_add((x >> 24) as u8 & 7)
        })
        .collect();
    Canonical::new(Spec { width: w, height: h, layout: Layout::Rgb, bit_depth: 8 }, data).unwrap()
}

fn main() {
    let smoke = std::env::args().any(|a| a == "--smoke");
    let imgs = load_images(if smoke { Some(9) } else { None });
    let reps = if smoke { 1 } else { REPS };
    let cfg = x4_cfg();
    let ffs: Vec<Ff> = vec![
        Ff { kind: Kind::Jxl(7), threads: 1 },
        Ff { kind: Kind::Jxl(9), threads: 1 },
        Ff { kind: Kind::Webp(6), threads: 1 },
        Ff { kind: Kind::Jxl(7), threads: 0 },
        Ff { kind: Kind::Jxl(9), threads: 0 },
    ];
    let png = PngCodec;
    let tiny = tiny_image();

    println!("== preflight ==");
    println!("x4_cfg = {:?}", cfg);
    for i in &imgs {
        let s = &i.img.spec;
        println!("{:<11} {:4} full {}x{} -> crop {}x{} {} stride {} B/row sha256 {} notes {:?}", i.name, i.src, i.full.0, i.full.1, s.width, s.height, s.label(), s.width as usize * s.layout.channels(), &i.img.content_hash()[..16], i.notes);
    }

    // ---- smoke: genuine losslessness, conversions, anti-cheat
    println!("== smoke ==");
    // channel-layout conversion round trip: rgb24 -> bgra(alpha 255) -> rgb24
    for i in imgs.iter().chain(std::iter::once(&Img { name: "tiny".into(), src: "synthetic", full: (33, 17), img: tiny.clone(), notes: vec![] })) {
        let ch = i.img.spec.layout.channels();
        let (bgra, back): (Vec<u8>, Vec<u8>) = if ch == 3 {
            let b: Vec<u8> = i.img.data.chunks(3).flat_map(|p| [p[2], p[1], p[0], 255]).collect();
            let r = b.chunks(4).flat_map(|p| [p[2], p[1], p[0]]).collect();
            (b, r)
        } else {
            let b: Vec<u8> = i.img.data.chunks(4).flat_map(|p| [p[2], p[1], p[0], p[3]]).collect();
            let r = b.chunks(4).flat_map(|p| [p[2], p[1], p[0], p[3]]).collect();
            (b, r)
        };
        assert_eq!(bgra.len() / 4, i.img.data.len() / ch);
        assert_eq!(back, i.img.data, "rgb(a)<->bgra conversion not exact for {}", i.name);
    }
    println!("rgb24/rgba <-> bgra channel conversion (rgb: a=255) exact on all inputs (WebP path)");
    let mut calib: Vec<(String, f64, f64)> = vec![];
    let mut codecs: Vec<&dyn Codec> = ffs.iter().map(|f| f as &dyn Codec).collect();
    codecs.push(&png);
    for c in &codecs {
        // warm-up + calibration on the tiny image (also the odd-size edge case)
        let _ = round_trip(*c, &tiny, 1);
        let t = round_trip(*c, &tiny, 5).unwrap_or_else(|e| panic!("{}: {e}", c.name()));
        assert!(matches!(t.verdict, Verdict::Exact), "{} not exact on tiny image", c.name());
        truncation_check(*c, &tiny, &t.encoded).unwrap_or_else(|e| panic!("{}: {e}", c.name()));
        println!("{:<12} tiny exact, truncation rejected; spawn/overhead floor enc {:.1} ms dec {:.1} ms", c.name(), ms(t.enc_time), ms(t.dec_time));
        calib.push((c.name(), ms(t.enc_time), ms(t.dec_time)));
    }
    {
        let r = pkon_rt(&tiny, &cfg, 1);
        assert!(matches!(r.verdict, Verdict::Exact));
        let cut = std::panic::catch_unwind(|| pkon_decode(&r.stream[..r.stream.len() / 2], &tiny.spec, &x4_cfg()));
        let rejected = cut.map(|d| first_mismatch(&tiny, &d).is_some()).unwrap_or(true);
        assert!(rejected, "pkon2 half-stream decode reproduced original");
        println!("{:<12} tiny exact, truncation rejected", "pkon2-x4");
    }

    let prior = std::fs::read_to_string("historical/20261027-x4-promotion-verify/report.md").unwrap_or_default();
    let prior_b = |name: &str| -> Option<usize> {
        let l = prior.lines().find(|l| l.starts_with(&format!("| {name} |")))?;
        // columns: "", image, src, frozen M6 X4 B, production B, change, ...
        l.split('|').nth(4)?.trim().parse().ok()
    };

    let mut recs: Vec<Rec> = vec![];
    let mut mx_cross: Vec<(String, usize, usize, Option<usize>, f64, f64)> = vec![];
    let mut all_exact = true;
    for (n, i) in imgs.iter().enumerate() {
        let (px, raw) = ((i.img.spec.width * i.img.spec.height) as usize, i.img.spec.byte_len());
        let base = |codec: &str, bytes: usize, payload: Option<u64>, e: Duration, d: Duration, v: &Verdict, note: String| Rec {
            image: i.name.clone(), src: i.src, codec: codec.into(), bytes, payload, raw, pixels: px, enc_ms: ms(e), dec_ms: ms(d), exact: matches!(v, Verdict::Exact), note,
        };
        // PKON2 X4
        let r = pkon_rt(&i.img, &cfg, reps);
        let total = r.stream.len() + HEADER_BYTES;
        let mx = measure_x(&i.img, &cfg);
        assert_eq!(mx.coded_bytes, total, "{}: replica bytes differ from measure_x", i.name);
        mx_cross.push((i.name.clone(), total, mx.coded_bytes, prior_b(&i.name), mx.enc_ms, mx.dec_ms));
        let rec = base("pkon2-x4", total, Some(r.stream.len() as u64), r.enc, r.dec, &r.verdict, format!("11-byte header counted, dims passed out-of-band to decoder; planes G,R-G,B-G"));
        eprintln!("[{}/{}] {:<11} pkon2-x4   {:>8} B enc {:>7.0} ms dec {:>7.0} ms exact {}", n + 1, imgs.len(), i.name, rec.bytes, rec.enc_ms, rec.dec_ms, rec.exact);
        all_exact &= rec.exact;
        recs.push(rec);
        for c in &codecs {
            match round_trip(*c, &i.img, reps) {
                Ok(t) => {
                    let ov = overhead(&c.name(), &t.encoded);
                    let rec = base(&c.name(), t.encoded.len(), ov.map(|o| t.encoded.len() as u64 - o), t.enc_time, t.dec_time, &t.verdict, String::new());
                    eprintln!("[{}/{}] {:<11} {:<10} {:>8} B enc {:>7.0} ms dec {:>7.0} ms exact {}", n + 1, imgs.len(), i.name, c.name(), rec.bytes, rec.enc_ms, rec.dec_ms, rec.exact);
                    all_exact &= rec.exact;
                    recs.push(rec);
                }
                Err(e) => {
                    eprintln!("{} {}: {e}", i.name, c.name());
                    all_exact = false;
                    recs.push(Rec { image: i.name.clone(), src: i.src, codec: c.name(), bytes: 0, payload: None, raw, pixels: px, enc_ms: 0.0, dec_ms: 0.0, exact: false, note: e.to_string() });
                }
            }
        }
    }
    println!("all exact: {all_exact}");
    if smoke {
        println!("smoke complete (nothing written)");
        return;
    }
    write_outputs(&imgs, &recs, &calib, &mx_cross, all_exact, reps);
}

// ---------------------------------------------------------------- reporting

fn write_outputs(imgs: &[Img], recs: &[Rec], calib: &[(String, f64, f64)], mx: &[(String, usize, usize, Option<usize>, f64, f64)], all_exact: bool, reps: u32) {
    let dir = PathBuf::from("benchmarks/pkon2").join(format!("{}-ext-benchmark", utc_stamp()));
    std::fs::create_dir_all(&dir).unwrap();
    let names: Vec<String> = {
        let mut v: Vec<String> = vec![];
        for r in recs {
            if !v.contains(&r.codec) {
                v.push(r.codec.clone());
            }
        }
        v
    };
    // csv
    let mut csv = String::from("image,source,codec,total_bytes,payload_bytes,raw_bytes,pixels,bpp,ratio,pct_of_raw,enc_ms,dec_ms,exact,note\n");
    for r in recs {
        csv += &format!("{},{},{},{},{},{},{},{:.4},{:.4},{:.3},{:.1},{:.1},{},\"{}\"\n", r.image, r.src, r.codec, r.bytes, r.payload.map(|p| p.to_string()).unwrap_or_default(), r.raw, r.pixels, r.bytes as f64 * 8.0 / r.pixels as f64, r.raw as f64 / r.bytes as f64, r.bytes as f64 / r.raw as f64 * 100.0, r.enc_ms, r.dec_ms, r.exact, r.note);
    }
    std::fs::write(dir.join("per_image.csv"), &csv).unwrap();
    let js: Vec<_> = recs.iter().map(|r| serde_json::json!({"image": r.image, "source": r.src, "codec": r.codec, "total_bytes": r.bytes, "payload_bytes": r.payload, "raw_bytes": r.raw, "pixels": r.pixels, "enc_ms": r.enc_ms, "dec_ms": r.dec_ms, "exact": r.exact, "note": r.note})).collect();
    std::fs::write(dir.join("results.json"), serde_json::to_string_pretty(&serde_json::json!({"reps_best_of": reps, "crop": CROP, "calibration_ms": calib, "records": js})).unwrap()).unwrap();

    let ff_version = nanoscope::ext::ffmpeg_version().unwrap_or_default();
    let sets: [(&str, Vec<&Rec>); 3] = [
        ("20 JPEG-origin", recs.iter().filter(|r| r.src == "jpeg").collect()),
        ("3 PNG-origin", recs.iter().filter(|r| r.src == "png").collect()),
        ("All 23 combined", recs.iter().collect()),
    ];
    let mut s = String::new();
    s += "# External codec benchmark: PKON2 X4 vs JPEG XL / WebP / PNG (lossless)\n\n";
    s += &format!("Inputs: {} images, each the exact decoded RGB8 pixel buffer centre-cropped to at most {CROP}x{CROP} (identical crops/pipeline to every PKON2 report). The 20 JPEG-origin inputs are rgb8 and the 3 PNG-origin inputs are rgba8 (alpha is constant 255 in all three, but the 4-channel layout is preserved for every codec); interleaved, no row padding. bpp = bits per pixel of the canonical buffer (RGB = 24 raw, RGBA = 32 raw); ratio = canonical raw bytes / stored bytes. Every stream was decoded from its bytes and compared byte-for-byte with the canonical buffer. **All exact: {all_exact}**. Timing: best of {reps} (encode and decode timed independently per round trip), single process, nothing else running.\n\n", imgs.len());
    s += "## Aggregate size (sum of actual stored bytes; bpp = stored bits / pixel; raw = 24 bpp RGB, 32 bpp RGBA)\n\n";
    for (label, set) in &sets {
        let raw: usize = set.iter().filter(|r| r.codec == "pkon2-x4").map(|r| r.raw).sum();
        let px: usize = set.iter().filter(|r| r.codec == "pkon2-x4").map(|r| r.pixels).sum();
        let pk: usize = set.iter().filter(|r| r.codec == "pkon2-x4").map(|r| r.bytes).sum();
        s += &format!("### {label} ({} images, raw {raw} B, {px} px)\n\n| codec | total bytes | payload bytes | bpp | ratio | % of raw | vs PKON2 X4 | exact |\n|---|---:|---:|---:|---:|---:|---:|---|\n", set.iter().filter(|r| r.codec == "pkon2-x4").count());
        for n in names.iter().filter(|n| !n.ends_with("-mt")) {
            let rs: Vec<&&Rec> = set.iter().filter(|r| &r.codec == n).collect();
            let b: usize = rs.iter().map(|r| r.bytes).sum();
            let pay: Option<u64> = rs.iter().map(|r| r.payload).sum();
            s += &format!("| {n} | {b} | {} | {:.3} | {:.3} | {:.2}% | {:+.2}% | {} |\n", pay.map(|p| p.to_string()).unwrap_or("n/a".into()), b as f64 * 8.0 / px as f64, raw as f64 / b as f64, b as f64 / raw as f64 * 100.0, (b as f64 / pk as f64 - 1.0) * 100.0, rs.iter().all(|r| r.exact));
        }
        s += "\n";
    }
    s += "\"vs PKON2 X4\" = (codec total bytes / PKON2 X4 total bytes - 1); positive means the codec is larger than PKON2. Computed from summed bytes, not averaged per-image percentages.\n\n";

    s += "## Timing (sum over images of best-of-N, ms; MP/s = raw megapixels per second)\n\n";
    s += "ffmpeg-driven codecs (JXL, WebP) are measured through a process spawn plus temp-file I/O; PNG and PKON2 are in-process. `net` subtracts the per-call floor measured on a 33x17 image through the same path (see calibration). The `-t1` rows use `-threads 1` and are the like-for-like comparison with single-threaded PKON2; `-mt` rows (library default threads) are timing-only.\n\n";
    s += "| codec | set | enc total ms | enc net ms | enc MP/s (net) | dec total ms | dec net ms | dec MP/s (net) |\n|---|---|---:|---:|---:|---:|---:|---:|\n";
    for n in &names {
        let (ce, cd) = calib.iter().find(|c| &c.0 == n).map(|c| (c.1, c.2)).unwrap_or((0.0, 0.0));
        for (label, set) in &sets {
            let rs: Vec<&&Rec> = set.iter().filter(|r| &r.codec == n).collect();
            let (e, d): (f64, f64) = (rs.iter().map(|r| r.enc_ms).sum(), rs.iter().map(|r| r.dec_ms).sum());
            let k = rs.len() as f64;
            let (en, dn) = ((e - ce * k).max(1.0), (d - cd * k).max(1.0));
            let mp: f64 = rs.iter().map(|r| r.pixels as f64).sum::<f64>() / 1e6;
            s += &format!("| {n} | {label} | {e:.0} | {en:.0} | {:.2} | {d:.0} | {dn:.0} | {:.2} |\n", mp / (en / 1e3), mp / (dn / 1e3));
        }
    }
    s += "\nCalibration (floor per call, best of 5 on 33x17): ";
    s += &calib.iter().map(|c| format!("{} enc {:.1} / dec {:.1} ms", c.0, c.1, c.2)).collect::<Vec<_>>().join("; ");
    s += "\n\n";

    s += "## Per-image results\n\nColumns: total bytes | bpp | ratio | enc ms | dec ms (exact in every row unless marked FAIL).\n\n";
    s += &format!("| image | src | {} |\n|---|---|{}\n", names.iter().filter(|n| !n.ends_with("-mt")).cloned().collect::<Vec<_>>().join(" | "), "---|".repeat(names.iter().filter(|n| !n.ends_with("-mt")).count()));
    for i in imgs {
        s += &format!("| {} | {} |", i.name, i.src);
        for n in names.iter().filter(|n| !n.ends_with("-mt")) {
            let r = recs.iter().find(|r| r.image == i.name && &r.codec == n).unwrap();
            s += &format!(" {} B / {:.2} / {:.2}x / {:.0} / {:.0}{} |", r.bytes, r.bytes as f64 * 8.0 / r.pixels as f64, r.raw as f64 / r.bytes as f64, r.enc_ms, r.dec_ms, if r.exact { "" } else { " **FAIL**" });
        }
        s += "\n";
    }
    s += "\n## PKON2 X4 cross-checks\n\nReplica pipeline bytes == `measure_x(x4_cfg())` bytes on every image (asserted). Compared with the prior `x4-promotion-verify` report's production bytes:\n\n| image | bench bytes | measure_x bytes | prior report bytes | match | measure_x enc/dec ms (excl. plane split/colour) |\n|---|---:|---:|---:|---|---|\n";
    for m in mx {
        s += &format!("| {} | {} | {} | {} | {} | {:.0}/{:.0} |\n", m.0, m.1, m.2, m.3.map(|x| x.to_string()).unwrap_or("-".into()), m.3.map(|x| (x == m.1).to_string()).unwrap_or("n/a".into()), m.4, m.5);
    }
    s += &format!("\n## Environment\n\n- ffmpeg: {ff_version}\n- see `environment.md` for codec library versions and exact settings.\n");
    std::fs::write(dir.join("report.md"), &s).unwrap();
    println!("{s}");
    println!("wrote {}", Path::new(&dir).display());
}
