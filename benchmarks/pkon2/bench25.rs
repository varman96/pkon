//! 25-image diversity benchmark: `.pkon2` v1 file (production default, codec id 3 = REL6) vs JPEG XL lossless e7/e9,
//! WebP lossless m6 and PNG, all fed the identical decoded pixel buffers (1024-px centre crops, as in every earlier PKON2
//! report). Measurement only: no PKON2 code is changed. The ffmpeg/PNG harness is copied from `bench50.rs`; what differs is
//! the manifest (`data/dataset25.json`), a pre-encode manifest written first, a codec-id assertion, and per-rep timing capture.
//!
//!   cargo run --release --bin bench25 -- --smoke    # 3 images, 1 rep, nothing written
//!   cargo run --release --bin bench25               # full set from data/dataset25.json -> results/<stamp>-bench25-diversity/
use nanoscope::canon::{first_mismatch, Canonical, Layout, Spec};
use nanoscope::codec::{Codec, CodecError, PngCodec};
use nanoscope::pkon2::center_crop;
use nanoscope::pkonfile::{self, crc32, HEADER_LEN};
use nanoscope::verify::{round_trip, truncation_check, Verdict};
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

const CROP: u32 = 1024;
/// Timed repetitions per (image, codec) after one untimed warm-up repetition; every repetition is decoded and compared.
const REPS: u32 = 3;


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


// ---------------------------------------------------------------- .pkon2 v1 file (production X4)

/// The real `.pkon2` file: encode() writes the 24-byte header + payload, decode() validates header, length and CRC-32.
struct Pkon2File;
impl Codec for Pkon2File {
    fn name(&self) -> String {
        "pkon2-file".into()
    }
    fn version(&self) -> String {
        format!("pkon2 v1 file, codec id {} (REL6)", pkonfile::CODEC_CURRENT)
    }
    fn config(&self) -> String {
        String::new()
    }
    fn encode(&self, img: &Canonical) -> Result<Vec<u8>, CodecError> {
        pkonfile::encode(img).map_err(|e| CodecError::Failed(e.to_string()))
    }
    fn decode(&self, enc: &[u8], _hint: &Spec) -> Result<Canonical, CodecError> {
        pkonfile::decode(enc).map_err(|e| CodecError::Failed(e.to_string()))
    }
    fn container_overhead(&self, enc: &[u8]) -> Option<u64> {
        (enc.len() >= HEADER_LEN).then_some(HEADER_LEN as u64)
    }
}

/// Independent metadata/integrity check of a produced file against the canonical input.
fn check_pkon2_file(file: &[u8], img: &Canonical) -> Result<(), String> {
    let h = pkonfile::parse_header(file).map_err(|e| e.to_string())?;
    if h.spec != img.spec {
        return Err(format!("header spec {:?} != input spec {:?}", h.spec, img.spec));
    }
    if file.len() != HEADER_LEN + h.payload_len as usize {
        return Err("file length != 24 + payload_len".into());
    }
    if h.crc32 != crc32(&img.data) {
        return Err("header CRC-32 != CRC-32 of input pixels".into());
    }
    if h.spec.bit_depth != 8 {
        return Err("not 8-bit".into());
    }
    Ok(())
}

// ---------------------------------------------------------------- dataset + features

struct Img {
    name: String,
    meta: serde_json::Value,
    full: (u32, u32),
    full_label: String,
    notes: Vec<String>,
    img: Canonical,
    feat: Feat,
}

#[derive(Clone, Default)]
struct Feat {
    med_mae: f64,
    med_zero_frac: f64,
    mean_g: f64,
    sat_frac: f64,
}

/// Cheap content descriptors on the G plane of the crop (MED/LOCO-I residual magnitude, share of exactly-predicted
/// pixels, mean level, clipped share). Descriptors only, used to group results; not part of any codec.
fn features(img: &Canonical) -> Feat {
    let (w, h, ch) = (img.spec.width as usize, img.spec.height as usize, img.spec.layout.channels());
    let g = |x: usize, y: usize| img.data[(y * w + x) * ch + if ch >= 3 { 1 } else { 0 }] as i32;
    let (mut sum, mut zero, mut n, mut tot, mut sat) = (0f64, 0u64, 0u64, 0f64, 0u64);
    for y in 0..h {
        for x in 0..w {
            let v = g(x, y);
            tot += v as f64;
            if v == 0 || v == 255 {
                sat += 1;
            }
            if x == 0 || y == 0 {
                continue;
            }
            let (a, b, c) = (g(x - 1, y), g(x, y - 1), g(x - 1, y - 1));
            let p = if c >= a.max(b) { a.min(b) } else if c <= a.min(b) { a.max(b) } else { a + b - c };
            let r = (v - p).abs();
            sum += r as f64;
            zero += (r == 0) as u64;
            n += 1;
        }
    }
    let px = (w * h) as f64;
    Feat { med_mae: sum / n as f64, med_zero_frac: zero as f64 / n as f64, mean_g: tot / px, sat_frac: sat as f64 / px }
}

fn load_images(only: Option<usize>) -> Vec<Img> {
    let data = PathBuf::from("data");
    let manifest: Vec<serde_json::Value> = serde_json::from_str(&std::fs::read_to_string(data.join("dataset25.json")).expect("data/dataset25.json (run tools/build_manifest25.py)")).unwrap();
    let mut v = vec![];
    for (i, m) in manifest.iter().enumerate() {
        if let Some(n) = only {
            if ![0, n, manifest.len() - 1].contains(&i) {
                continue;
            }
        }
        let full = Canonical::load(&data.join(m["file"].as_str().unwrap())).unwrap();
        let crop = center_crop(&full, CROP);
        let feat = features(&crop);
        let mut m = m.clone();
        let src = std::fs::read(data.join(m["file"].as_str().unwrap())).unwrap();
        let rel = format!("data/{}", m["file"].as_str().unwrap());
        m.as_object_mut().unwrap().insert("source_path".into(), rel.into());
        m.as_object_mut().unwrap().insert("source_file_sha256".into(), sha_hex(&src).into());
        v.push(Img { name: m["name"].as_str().unwrap().into(), meta: m, full: (full.spec.width, full.spec.height), full_label: full.spec.label(), notes: full.notes.clone(), img: crop, feat });
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

fn ms(d: Duration) -> f64 {
    d.as_secs_f64() * 1e3
}

fn tiny_image() -> Canonical {
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

fn sha_hex(b: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    Sha256::digest(b).iter().map(|x| format!("{x:02x}")).collect()
}

/// One image through one codec: 1 untimed warm-up rep, then `reps` timed reps. Every rep (including warm-up) is decoded from
/// its own bytes and compared with the canonical buffer; encoded bytes must be identical across reps. Timed regions are
/// exactly `Codec::encode` / `Codec::decode`.
struct Timed {
    encoded: Vec<u8>,
    enc_ms: Vec<f64>,
    dec_ms: Vec<f64>,
    exact_all: bool,
    deterministic: bool,
    first_mismatch: String,
}
fn timed_trip(c: &dyn Codec, img: &Canonical, reps: u32) -> Result<Timed, String> {
    let mut t = Timed { encoded: vec![], enc_ms: vec![], dec_ms: vec![], exact_all: true, deterministic: true, first_mismatch: String::new() };
    for r in 0..=reps {
        let t0 = Instant::now();
        let enc = c.encode(img).map_err(|e| format!("encode: {e}"))?;
        let e = t0.elapsed();
        let t0 = Instant::now();
        let dec = c.decode(&enc, &img.spec).map_err(|e| format!("decode: {e}"))?;
        let d = t0.elapsed();
        if let Some(m) = first_mismatch(img, &dec) {
            t.exact_all = false;
            if t.first_mismatch.is_empty() {
                t.first_mismatch = m.to_string();
            }
        }
        if r == 0 {
            t.encoded = enc;
            continue; // warm-up: not timed
        }
        if enc != t.encoded {
            t.deterministic = false;
        }
        t.enc_ms.push(ms(e));
        t.dec_ms.push(ms(d));
    }
    Ok(t)
}

fn cmd_first_line(exe: &str, args: &[&str]) -> String {
    Command::new(exe)
        .args(args)
        .output()
        .ok()
        .and_then(|o| String::from_utf8_lossy(if o.stdout.is_empty() { &o.stderr } else { &o.stdout }).lines().next().map(|l| l.trim().to_string()))
        .unwrap_or_default()
}

fn main() {
    let smoke = std::env::args().any(|a| a == "--smoke");
    let imgs = load_images(if smoke { Some(12) } else { None });
    let reps = if smoke { 1 } else { REPS };
    let ffs: Vec<Ff> = vec![Ff { kind: Kind::Jxl(7), threads: 1 }, Ff { kind: Kind::Jxl(9), threads: 1 }, Ff { kind: Kind::Webp(6), threads: 1 }];
    let png = PngCodec;
    let pk = Pkon2File;
    let mut codecs: Vec<&dyn Codec> = vec![&pk];
    codecs.extend(ffs.iter().map(|f| f as &dyn Codec));
    codecs.push(&png);
    let tiny = tiny_image();

    // ---- frozen-input manifest, written BEFORE any encoding
    let manifest_images: Vec<_> = imgs
        .iter()
        .map(|i| {
            let mut m = i.meta.clone();
            let o = m.as_object_mut().unwrap();
            o.insert("decoded_full_format".into(), i.full_label.clone().into());
            o.insert("decoded_full_width".into(), i.full.0.into());
            o.insert("decoded_full_height".into(), i.full.1.into());
            o.insert("crop_width".into(), i.img.spec.width.into());
            o.insert("crop_height".into(), i.img.spec.height.into());
            o.insert("crop_format".into(), i.img.spec.label().into());
            o.insert("crop_channels".into(), i.img.spec.layout.channels().into());
            o.insert("crop_raw_bytes".into(), i.img.spec.byte_len().into());
            o.insert("crop_sha256".into(), i.img.content_hash().into());
            o.insert("decoder_notes".into(), i.notes.clone().into());
            o.insert("med_mae_g".into(), i.feat.med_mae.into());
            o.insert("med_zero_frac_g".into(), i.feat.med_zero_frac.into());
            o.insert("mean_g".into(), i.feat.mean_g.into());
            o.insert("clipped_frac_g".into(), i.feat.sat_frac.into());
            m
        })
        .collect();
    let dir = PathBuf::from("results").join(format!("{}-bench25-diversity", utc_stamp()));
    if !smoke {
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("manifest_pre_encode.json"), serde_json::to_string_pretty(&manifest_images).unwrap()).unwrap();
        println!("manifest written to {} before encoding", dir.display());
    }
    let env = serde_json::json!({
        "ffmpeg": cmd_first_line("ffmpeg", &["-hide_banner", "-version"]),
        "png_codec": PngCodec.version(), "png_config": PngCodec.config(),
        "pkon2_codec_current": pkonfile::CODEC_CURRENT, "header_len": HEADER_LEN,
        "logical_cpus": std::thread::available_parallelism().map(|n| n.get()).unwrap_or(0),
        "reps_timed": reps, "warmup_reps": 1, "crop": CROP,
    });

    println!("== preflight ==");
    assert_eq!(pkonfile::CODEC_CURRENT, 3, "production default is not codec id 3");
    for i in &imgs {
        let s = &i.img.spec;
        println!("{:<14} full {}x{} {} -> crop {}x{} {} sha256 {} notes {:?}", i.name, i.full.0, i.full.1, i.full_label, s.width, s.height, s.label(), &i.img.content_hash()[..16], i.notes);
    }
    let mut calib: Vec<(String, f64, f64)> = vec![];
    for c in &codecs {
        let _ = round_trip(*c, &tiny, 1);
        let t = round_trip(*c, &tiny, 5).unwrap_or_else(|e| panic!("{}: {e}", c.name()));
        assert!(matches!(t.verdict, Verdict::Exact), "{} not exact on tiny image", c.name());
        truncation_check(*c, &tiny, &t.encoded).unwrap_or_else(|e| panic!("{}: {e}", c.name()));
        println!("{:<12} tiny exact, truncation rejected; floor enc {:.1} ms dec {:.1} ms", c.name(), ms(t.enc_time), ms(t.dec_time));
        calib.push((c.name(), ms(t.enc_time), ms(t.dec_time)));
    }

    let mut recs: Vec<serde_json::Value> = vec![];
    let mut all_exact = true;
    let (t0, total_jobs) = (Instant::now(), imgs.len());
    for (n, i) in imgs.iter().enumerate() {
        let (px, raw) = ((i.img.spec.width * i.img.spec.height) as usize, i.img.spec.byte_len());
        for c in &codecs {
            let rec = match timed_trip(*c, &i.img, reps) {
                Ok(t) => {
                    let mut valid = t.exact_all && t.deterministic;
                    let mut note = t.first_mismatch.clone();
                    if !t.deterministic {
                        note += " nondeterministic encode across reps";
                    }
                    let mut meta_ok = serde_json::Value::Null;
                    let mut codec_id = serde_json::Value::Null;
                    if c.name() == "pkon2-file" {
                        let r = check_pkon2_file(&t.encoded, &i.img);
                        meta_ok = r.is_ok().into();
                        if let Err(e) = r {
                            valid = false;
                            note = format!("{note} metadata: {e}");
                        }
                        let h = pkonfile::parse_header(&t.encoded).unwrap();
                        codec_id = h.codec.into();
                        if h.codec != 3 {
                            valid = false;
                            note += " header codec id != 3";
                        }
                        // integrity: flipping one payload bit must be rejected by decode (range-coder end check or CRC)
                        let mut bad = t.encoded.clone();
                        let at = HEADER_LEN + (bad.len() - HEADER_LEN) / 2;
                        bad[at] ^= 0x10;
                        if pkonfile::decode(&bad).is_ok() {
                            valid = false;
                            note += " corrupted file was accepted";
                        }
                    }
                    let ov = overhead(&c.name(), &t.encoded).or_else(|| c.container_overhead(&t.encoded));
                    let med = |v: &Vec<f64>| {
                        let mut s = v.clone();
                        s.sort_by(|a, b| a.partial_cmp(b).unwrap());
                        s[s.len() / 2]
                    };
                    eprintln!("[{}/{}] {:<14} {:<10} {:>8} B enc {:>7.0} ms dec {:>7.0} ms (median) valid {}", n + 1, total_jobs, i.name, c.name(), t.encoded.len(), med(&t.enc_ms), med(&t.dec_ms), valid);
                    all_exact &= valid;
                    serde_json::json!({"image": i.name, "codec": c.name(), "total_bytes": t.encoded.len(), "payload_bytes": ov.map(|o| t.encoded.len() as u64 - o), "raw_bytes": raw, "pixels": px,
                        "enc_ms_reps": t.enc_ms, "dec_ms_reps": t.dec_ms, "exact_all_reps": t.exact_all, "deterministic": t.deterministic,
                        "pkon2_metadata_ok": meta_ok, "pkon2_codec_id": codec_id, "valid": valid, "note": note,
                        "file_sha256": sha_hex(&t.encoded), "magic_hex": t.encoded.iter().take(12).map(|b| format!("{b:02x}")).collect::<String>()})
                }
                Err(e) => {
                    eprintln!("{} {}: {e}", i.name, c.name());
                    all_exact = false;
                    serde_json::json!({"image": i.name, "codec": c.name(), "total_bytes": null, "raw_bytes": raw, "pixels": px, "valid": false, "exact_all_reps": false, "note": e})
                }
            };
            recs.push(rec);
        }
        let el = t0.elapsed().as_secs_f64();
        let eta = el / (n + 1) as f64 * (total_jobs - n - 1) as f64;
        eprintln!("== {}/{} images done, elapsed {:.0}s, ETA {:.0}s ==", n + 1, total_jobs, el, eta);
    }
    println!("all valid+exact: {all_exact}");
    if smoke {
        println!("smoke complete (nothing written)");
        return;
    }
    std::fs::write(dir.join("results.json"), serde_json::to_string_pretty(&serde_json::json!({"env": env, "calibration_ms": calib, "all_valid": all_exact, "images": manifest_images, "records": recs})).unwrap()).unwrap();
    println!("wrote {}", dir.display());
}
