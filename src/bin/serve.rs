//! Minimal localhost-only converter UI: PNG/JPEG -> .pkon (container v2, PKON2-X4), DNG -> .pkon (LISA RAW), and
//! .pkon/.pkon2 -> PNG (standard images) or lossless u16/u8 sample export (LISA RAW), with integrity reporting.
//!
//!   cargo run --release --bin serve            # then open http://127.0.0.1:8080/
//!
//! Standard pixels are what `Canonical::load_bytes` decodes (8-bit gray / gray+alpha / RGB / RGBA, row-major, interleaved).
//! A JPEG is encoded from its *decoded* pixels; the pre-JPEG original, EXIF and ICC are not recoverable. A DNG is encoded
//! from its stored raster, untouched (see `dng.rs`). Nothing is stored on disk. Every response from /convert and /restore
//! carries an `X-Report` JSON header (codec, geometry, sizes, File Integrity); all hashing happens here, on the canonical
//! sample bytes (`integrity.rs`), never on a preview.
use nanoscope::canon::{Canonical, Layout};
use nanoscope::dng;
use nanoscope::integrity::{self, pixel_sha256_standard, pixel_sha256_u16, sha256_hex};
use nanoscope::lisa::{ColorFilter, ImageMetadata};
use nanoscope::pkon::{self, bytes_per_sample, decode as decode_any, encode_raw, encode_standard as encode, pixel_crc_u16, ContainerError, Decoded};
use nanoscope::pkonfile::{self, PkonError};
use serde_json::{json, Value};
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::time::Duration;

const PORT: u16 = 8080;
const MAX_UPLOAD: usize = 64 << 20;
const PAGE: &str = include_str!("../../frontend/serve.html");
const VIEWER: &str = include_str!("../../frontend/viewer.html");

/// Decodes a `.pkon` (v2) or legacy `.pkon2` (v1) file; this converter only handles standard images, not LISA RAW.
fn decode(input: &[u8]) -> Result<Canonical, String> {
    match decode_any(input).map_err(|e| e.to_string())? {
        Decoded::Standard(img) => Ok(img),
        Decoded::Raw(_) => Err("this is a RAW (LISA) .pkon file; the viewer only shows standard images".into()),
    }
}

/// A successful conversion/restore: response body, content type, short `X-Info` text and the `X-Report` JSON.
struct Out {
    body: Vec<u8>,
    ctype: &'static str,
    info: String,
    report: Value,
}

#[derive(Debug)]
/// Failure; `report` is set for integrity failures so the UI can still show the measured values.
struct Fail {
    msg: String,
    report: Option<Value>,
}
impl From<String> for Fail {
    fn from(msg: String) -> Self {
        Fail { msg, report: None }
    }
}
impl From<&str> for Fail {
    fn from(msg: &str) -> Self {
        Fail { msg: msg.to_string(), report: None }
    }
}

fn hex32(v: u32) -> String {
    format!("0x{v:08x}")
}

/// CRC-32 stored in the header of a v1 or v2 file.
fn stored_crc(input: &[u8]) -> Option<u32> {
    match pkon::container_version(input).ok()? {
        pkon::VERSION_V1 => pkonfile::parse_header(input).ok().map(|h| h.crc32),
        _ => pkon::parse_header(input).ok().map(|h| h.pixel_crc32),
    }
}

fn integrity_json(orig: Option<&str>, recon: Option<&str>, file: &str, stored: u32, recomputed: u32, repr: &str) -> Value {
    json!({
        "pixel_byte_representation": repr,
        "original_pixel_sha256": orig,
        "reconstructed_pixel_sha256": recon,
        "pixel_sha256_match": orig.zip(recon).map(|(a, b)| a == b),
        "file_sha256": file,
        "crc_stored": hex32(stored),
        "crc_recomputed": hex32(recomputed),
        "crc_match": stored == recomputed,
    })
}

fn cfa_json(m: &ImageMetadata) -> Value {
    let layout = format!("{:?}", m.layout);
    match &m.cfa {
        None => json!({ "layout": layout }),
        Some(c) => {
            let letter = |f: &ColorFilter| match f {
                ColorFilter::Red => 'R',
                ColorFilter::Green => 'G',
                ColorFilter::Blue => 'B',
                ColorFilter::Cyan => 'C',
                ColorFilter::Magenta => 'M',
                ColorFilter::Yellow => 'Y',
                _ => 'W',
            };
            json!({ "layout": layout, "pattern": c.entries.iter().map(letter).collect::<String>(),
                    "rows": c.height, "cols": c.width, "origin_x": c.origin_x, "origin_y": c.origin_y })
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn base_report(kind: &str, codec: &str, codec_id: u8, container: u8, w: usize, h: usize, bit_depth: u8, uncompressed: u64, pkon_len: usize) -> Value {
    json!({ "kind": kind, "codec": codec, "codec_id": codec_id, "container_version": container, "width": w, "height": h,
            "bit_depth": bit_depth, "uncompressed_bytes": uncompressed, "pkon_bytes": pkon_len,
            "ratio": uncompressed as f64 / pkon_len as f64 })
}

fn convert_report(input: Vec<u8>) -> Result<Out, Fail> {
    if dng::looks_like_tiff(&input) {
        return convert_dng(&input);
    }
    let img = Canonical::load_bytes(input).map_err(|e| format!("cannot read image (PNG or JPEG, 8-bit, or DNG): {e}"))?;
    let out = encode(&img).map_err(|e| e.to_string())?;
    let (orig, file) = (pixel_sha256_standard(&img), sha256_hex(&out));
    let stored = stored_crc(&out).ok_or("cannot read the CRC of the file just written")?;
    // Verify by decoding the exact bytes that will be offered for download.
    let back = decode(&out).map_err(|e| format!("integrity failure: the new file does not decode: {e}"))?;
    let recon = pixel_sha256_standard(&back);
    let mut crc = pkon::Crc32::new();
    crc.update(&back.data);
    let recomputed = crc.finish();
    let mut report = base_report("standard", "PKON2-X4", out[6], out[4], img.spec.width as usize, img.spec.height as usize, 8, img.data.len() as u64, out.len());
    report["layout"] = json!(img.spec.label());
    report["integrity"] = integrity_json(Some(&orig), Some(&recon), &file, stored, recomputed, integrity::STANDARD_BYTE_REPR);
    if orig != recon || back.data != img.data || stored != recomputed {
        return Err(Fail { msg: "integrity failure: reconstructed pixels differ from the original".into(), report: Some(report) });
    }
    let info = format!("{}x{} {}", img.spec.width, img.spec.height, img.spec.label());
    Ok(Out { body: out, ctype: "application/octet-stream", info, report })
}

fn convert_dng(input: &[u8]) -> Result<Out, Fail> {
    let d = dng::ingest_dng(input)?;
    let (w, h) = (d.metadata.width, d.metadata.height);
    let out = encode_raw(&d.samples, w, h, d.bit_depth, d.metadata.clone()).map_err(|e| e.to_string())?;
    let (orig, file) = (pixel_sha256_u16(&d.samples, d.bit_depth), sha256_hex(&out));
    let stored = stored_crc(&out).ok_or("cannot read the CRC of the file just written")?;
    let back = match decode_any(&out).map_err(|e| format!("integrity failure: the new file does not decode: {e}"))? {
        Decoded::Raw(r) => r,
        Decoded::Standard(_) => return Err("integrity failure: the new file decoded as a standard image".into()),
    };
    let recon = pixel_sha256_u16(&back.samples, back.bit_depth);
    let recomputed = pixel_crc_u16(&back.samples, back.bit_depth);
    let mut report = base_report("raw", "LISA", out[6], out[4], w, h, d.bit_depth, (w * h * bytes_per_sample(d.bit_depth)) as u64, out.len());
    report["declared_bits"] = json!(d.declared_bits);
    report["cfa"] = cfa_json(&d.metadata);
    report["camera"] = json!(d.camera);
    report["input_bytes"] = json!(input.len());
    report["input_compression"] = json!(d.dng_compression);
    report["integrity"] = integrity_json(Some(&orig), Some(&recon), &file, stored, recomputed, integrity::raw_byte_repr(d.bit_depth));
    if back.samples != d.samples || back.metadata != d.metadata || back.bit_depth != d.bit_depth || orig != recon || stored != recomputed {
        return Err(Fail { msg: "integrity failure: reconstructed RAW samples differ from the original".into(), report: Some(report) });
    }
    let info = format!("{w}x{h} RAW {}-bit {:?}", d.bit_depth, d.metadata.layout);
    Ok(Out { body: out, ctype: "application/octet-stream", info, report })
}

#[cfg(test)]
fn convert(input: Vec<u8>) -> Result<(Vec<u8>, String), String> {
    convert_report(input).map(|o| (o.body, o.info)).map_err(|f| f.msg)
}

fn restore_report(input: &[u8]) -> Result<Out, Fail> {
    use image::{codecs::png::PngEncoder, ExtendedColorType, ImageEncoder};
    let file = sha256_hex(input);
    let decoded = decode_any(input).map_err(|e| match e {
        ContainerError::ChecksumMismatch { expected, actual } | ContainerError::V1(PkonError::ChecksumMismatch { expected, actual }) => Fail {
            msg: format!("integrity failure: pixel CRC-32 mismatch (stored {}, recomputed {})", hex32(expected), hex32(actual)),
            report: Some(json!({ "integrity": integrity_json(None, None, &file, expected, actual, "") })),
        },
        e => Fail::from(e.to_string()),
    })?;
    let stored = stored_crc(input).ok_or("cannot read the CRC of this file")?;
    let (container, codec_id) = (input[4], if input[4] == pkon::VERSION_V1 { input[5] } else { input[6] });
    match decoded {
        Decoded::Standard(img) => {
            let ct = match img.spec.layout {
                Layout::Gray => ExtendedColorType::L8,
                Layout::GrayAlpha => ExtendedColorType::La8,
                Layout::Rgb => ExtendedColorType::Rgb8,
                Layout::Rgba => ExtendedColorType::Rgba8,
            };
            let mut png = Vec::new();
            PngEncoder::new(&mut png).write_image(&img.data, img.spec.width, img.spec.height, ct).map_err(|e| e.to_string())?;
            let mut crc = pkon::Crc32::new();
            crc.update(&img.data);
            let recon = pixel_sha256_standard(&img);
            let mut report = base_report("standard", "PKON2-X4", codec_id, container, img.spec.width as usize, img.spec.height as usize, 8, img.data.len() as u64, input.len());
            report["layout"] = json!(img.spec.label());
            report["integrity"] = integrity_json(None, Some(&recon), &file, stored, crc.finish(), integrity::STANDARD_BYTE_REPR);
            Ok(Out { body: png, ctype: "image/png", info: format!("{}x{} {}", img.spec.width, img.spec.height, img.spec.label()), report })
        }
        Decoded::Raw(r) => {
            let (w, h) = (r.metadata.width, r.metadata.height);
            let wide = bytes_per_sample(r.bit_depth) == 2;
            let body: Vec<u8> = if wide { r.samples.iter().flat_map(|s| s.to_le_bytes()).collect() } else { r.samples.iter().map(|&s| s as u8).collect() };
            let recon = pixel_sha256_u16(&r.samples, r.bit_depth);
            let mut report = base_report("raw", "LISA", codec_id, container, w, h, r.bit_depth, body.len() as u64, input.len());
            report["cfa"] = cfa_json(&r.metadata);
            report["export_ext"] = json!(if wide { ".u16" } else { ".u8" });
            report["integrity"] = integrity_json(None, Some(&recon), &file, stored, pixel_crc_u16(&r.samples, r.bit_depth), integrity::raw_byte_repr(r.bit_depth));
            Ok(Out { body, ctype: "application/octet-stream", info: format!("{w}x{h} RAW {}-bit {:?}", r.bit_depth, r.metadata.layout), report })
        }
    }
}

#[cfg(test)]
fn restore(input: &[u8]) -> Result<(Vec<u8>, String), String> {
    restore_report(input).map(|o| (o.body, o.info)).map_err(|f| f.msg)
}

/// Raw decoded samples (row-major, interleaved, 8-bit) plus the X-* metadata headers; no re-encoding.
fn view_data(input: &[u8]) -> Result<(Vec<u8>, String), String> {
    let img = decode(input).map_err(|e| e.to_string())?;
    let ch = img.spec.layout.channels();
    if img.data.len() != img.spec.byte_len() || img.data.len() != img.spec.width as usize * img.spec.height as usize * ch {
        return Err("decoded buffer size does not match the image dimensions".into());
    }
    // `decode` succeeded, so the header is valid and byte 6 is the codec id.
    let info = format!(
        "X-Width: {}\r\nX-Height: {}\r\nX-Channels: {ch}\r\nX-Codec: {}\r\nX-Info: {} {}\r\n",
        img.spec.width, img.spec.height, input[6], img.spec.width, img.spec.label()
    );
    Ok((img.data, info))
}

fn respond(s: &mut TcpStream, status: &str, ctype: &str, extra: &str, body: &[u8]) {
    let head = format!("HTTP/1.1 {status}\r\nContent-Type: {ctype}\r\nContent-Length: {}\r\nCache-Control: no-store\r\nConnection: close\r\n{extra}\r\n", body.len());
    let _ = s.write_all(head.as_bytes()).and_then(|_| s.write_all(body));
}

fn handle(mut s: TcpStream) {
    let _ = s.set_read_timeout(Some(Duration::from_secs(30)));
    let mut buf = Vec::new();
    let mut chunk = [0u8; 65536];
    let head_end = loop {
        if let Some(p) = buf.windows(4).position(|w| w == b"\r\n\r\n") {
            break p + 4;
        }
        if buf.len() > 16384 {
            return respond(&mut s, "431 Request Header Fields Too Large", "text/plain", "", b"headers too large");
        }
        match s.read(&mut chunk) {
            Ok(0) | Err(_) => return,
            Ok(n) => buf.extend_from_slice(&chunk[..n]),
        }
    };
    let head = String::from_utf8_lossy(&buf[..head_end]).to_string();
    let mut lines = head.lines();
    let mut req = lines.next().unwrap_or("").split_whitespace();
    let (method, path) = (req.next().unwrap_or(""), req.next().unwrap_or(""));
    let len = lines
        .filter_map(|l| l.split_once(':'))
        .find(|(k, _)| k.eq_ignore_ascii_case("content-length"))
        .and_then(|(_, v)| v.trim().parse::<usize>().ok())
        .unwrap_or(0);
    if method == "GET" && (path == "/" || path == "/index.html") {
        return respond(&mut s, "200 OK", "text/html; charset=utf-8", "", PAGE.as_bytes());
    }
    if method == "GET" && path == "/viewer" {
        return respond(&mut s, "200 OK", "text/html; charset=utf-8", "", VIEWER.as_bytes());
    }
    if method != "POST" || !(path == "/convert" || path == "/restore" || path == "/view-data") {
        return respond(&mut s, "404 Not Found", "text/plain", "", b"not found");
    }
    if len > MAX_UPLOAD {
        return respond(&mut s, "413 Payload Too Large", "text/plain", "", format!("upload exceeds {} MiB limit", MAX_UPLOAD >> 20).as_bytes());
    }
    let mut body = buf[head_end..].to_vec();
    while body.len() < len {
        match s.read(&mut chunk) {
            Ok(0) | Err(_) => return respond(&mut s, "400 Bad Request", "text/plain", "", b"incomplete upload"),
            Ok(n) => body.extend_from_slice(&chunk[..n]),
        }
    }
    body.truncate(len);
    let op = path;
    // The codecs must never take the server down: a panic on hostile input becomes a 500.
    let result = std::panic::catch_unwind(|| match op {
        "/convert" => convert_report(body),
        "/restore" => restore_report(&body),
        _ => view_data(&body).map(|(body, info)| Out { body, ctype: "application/octet-stream", info, report: Value::Null }).map_err(Fail::from),
    });
    match result {
        Ok(Ok(o)) => {
            let extra = if op == "/view-data" { o.info } else { format!("X-Info: {}\r\nX-Report: {}\r\n", o.info, o.report) };
            respond(&mut s, "200 OK", o.ctype, &extra, &o.body)
        }
        // Integrity failures carry the measured values as JSON so the page can show them.
        Ok(Err(Fail { msg, report: Some(r) })) => {
            respond(&mut s, "422 Unprocessable Entity", "application/json", "", json!({ "error": msg, "report": r }).to_string().as_bytes())
        }
        Ok(Err(f)) => respond(&mut s, "400 Bad Request", "text/plain; charset=utf-8", "", f.msg.as_bytes()),
        Err(_) => respond(&mut s, "500 Internal Server Error", "text/plain", "", b"internal error while processing this file"),
    }
}

fn main() {
    let port = std::env::args().nth(1).and_then(|a| a.parse().ok()).unwrap_or(PORT);
    let listener = TcpListener::bind(("127.0.0.1", port)).unwrap_or_else(|e| panic!("cannot bind 127.0.0.1:{port}: {e}"));
    println!("PKON converter: http://127.0.0.1:{port}/   (Ctrl+C to stop)");
    for s in listener.incoming().flatten() {
        handle(s);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{codecs::png::PngEncoder, ExtendedColorType, ImageEncoder};
    use nanoscope::canon::first_mismatch;

    #[test]
    fn rgb_png_roundtrip_exact() {
        let (w, h) = (37u32, 21u32);
        let px: Vec<u8> = (0..w * h * 3).map(|i| (i.wrapping_mul(2654435761) >> 13) as u8).collect();
        let mut png = Vec::new();
        PngEncoder::new(&mut png).write_image(&px, w, h, ExtendedColorType::Rgb8).unwrap();
        let src = Canonical::load_bytes(png.clone()).unwrap();
        let (pkon, _) = convert(png).unwrap();
        let (back, _) = restore(&pkon).unwrap();
        let got = Canonical::load_bytes(back).unwrap();
        assert!(first_mismatch(&src, &got).is_none());
        assert_eq!(got.data, px);
    }

    /// Real JPEG end to end: restored PNG must equal the pixels PKON was given (the decoded JPEG), not the pre-JPEG original.
    #[test]
    fn real_jpeg_roundtrip_exact() {
        let Ok(jpeg) = std::fs::read("data/foliage_0.jpg") else { return };
        let src = Canonical::load_bytes(jpeg.clone()).unwrap();
        let (pkon, _) = convert(jpeg.clone()).unwrap();
        let (png, _) = restore(&pkon).unwrap();
        let got = Canonical::load_bytes(png).unwrap();
        assert!(first_mismatch(&src, &got).is_none());
        println!("jpeg {} B -> pkon {} B, {} samples identical", jpeg.len(), pkon.len(), src.data.len());
    }

    fn sample_pkon(ch: u32, codec: u8) -> (Vec<u8>, Canonical) {
        use nanoscope::pkonfile::encode_with_codec;
        let (w, h) = (23u32, 17u32);
        let px: Vec<u8> = (0..w * h * ch).map(|i| (i.wrapping_mul(2654435761) >> 11) as u8).collect();
        let ct = match ch {
            1 => ExtendedColorType::L8,
            2 => ExtendedColorType::La8,
            3 => ExtendedColorType::Rgb8,
            _ => ExtendedColorType::Rgba8,
        };
        let mut png = Vec::new();
        PngEncoder::new(&mut png).write_image(&px, w, h, ct).unwrap();
        let img = Canonical::load_bytes(png).unwrap();
        (encode_with_codec(&img, codec).unwrap(), img)
    }

    #[test]
    fn view_data_matches_decoder_all_layouts_and_codecs() {
        for codec in 1..=3u8 {
            for ch in 1..=4u32 {
                let (pkon, src) = sample_pkon(ch, codec);
                let (data, info) = view_data(&pkon).unwrap();
                assert_eq!(data, src.data, "codec {codec} channels {ch}");
                assert!(info.contains("X-Width: 23\r\n") && info.contains("X-Height: 17\r\n"));
                assert!(info.contains(&format!("X-Channels: {ch}\r\n")) && info.contains(&format!("X-Codec: {codec}\r\n")));
            }
        }
    }

    #[test]
    fn view_data_rejects_malformed() {
        assert!(view_data(b"").is_err());
        assert!(view_data(b"not a pkon file at all, definitely").is_err());
        let (mut pkon, _) = sample_pkon(3, 3);
        pkon[6] = 99; // unassigned codec
        assert!(view_data(&pkon).is_err());
        let (mut pkon, _) = sample_pkon(3, 3);
        pkon.truncate(pkon.len() - 5);
        assert!(view_data(&pkon).is_err());
    }

    fn post(path: &str, body: &[u8]) -> Vec<u8> {
        let mut req = format!("POST {path} HTTP/1.1\r\nHost: x\r\nContent-Length: {}\r\n\r\n", body.len()).into_bytes();
        req.extend_from_slice(body);
        http(req)
    }

    fn http(req: Vec<u8>) -> Vec<u8> {
        let l = TcpListener::bind(("127.0.0.1", 0)).unwrap();
        let addr = l.local_addr().unwrap();
        let t = std::thread::spawn(move || handle(l.accept().unwrap().0));
        let mut c = TcpStream::connect(addr).unwrap();
        c.write_all(&req).unwrap();
        let mut out = Vec::new();
        c.read_to_end(&mut out).unwrap();
        t.join().unwrap();
        out
    }

    #[test]
    fn view_routes_over_http() {
        let page = String::from_utf8_lossy(&http(b"GET /viewer HTTP/1.1\r\nHost: x\r\n\r\n".to_vec())).to_string();
        assert!(page.starts_with("HTTP/1.1 200") && page.contains("<canvas"));
        let (pkon, src) = sample_pkon(3, 3);
        let ok = post("/view-data", &pkon);
        let split = ok.windows(4).position(|w| w == b"\r\n\r\n").unwrap() + 4;
        let head = String::from_utf8_lossy(&ok[..split]).to_string();
        assert!(head.starts_with("HTTP/1.1 200") && head.contains("X-Channels: 3"));
        assert_eq!(&ok[split..], &src.data[..]);
        let bad = post("/view-data", b"garbage garbage garbage garbage");
        assert!(String::from_utf8_lossy(&bad).starts_with("HTTP/1.1 400"));
        // the existing converter route still answers
        assert!(String::from_utf8_lossy(&post("/restore", &pkon)[..15]).starts_with("HTTP/1.1 200"));
    }

    #[test]
    fn garbage_is_rejected() {
        assert!(convert(b"not an image".to_vec()).is_err());
        assert!(restore(b"not a pkon file at all, definitely").is_err());
        assert!(restore(&[]).is_err());
    }

    fn rep_str<'a>(v: &'a Value, path: &[&str]) -> &'a str {
        path.iter().fold(v, |a, k| &a[*k]).as_str().unwrap()
    }

    #[test]
    fn standard_report_integrity_and_file_hash_reproducible() {
        let (pkon, src) = sample_pkon(3, 3);
        let out = restore_report(&pkon).unwrap();
        let g = &out.report["integrity"];
        assert_eq!(rep_str(g, &["reconstructed_pixel_sha256"]), integrity::pixel_sha256_standard(&src));
        assert_eq!(rep_str(g, &["file_sha256"]), sha256_hex(&pkon));
        assert_eq!(g["crc_match"], true);
        assert_eq!(out.report["codec"], "PKON2-X4");
        // identical bytes -> identical complete-file digest; a flipped payload bit is rejected
        assert_eq!(restore_report(&pkon).unwrap().report["integrity"]["file_sha256"], g["file_sha256"]);
        let mut bad = pkon.clone();
        let n = bad.len();
        bad[n - 3] ^= 0x10;
        assert!(restore_report(&bad).is_err());
    }

    #[test]
    fn convert_standard_reports_matching_hashes() {
        let Ok(jpeg) = std::fs::read("data/foliage_0.jpg") else { return };
        let src = Canonical::load_bytes(jpeg.clone()).unwrap();
        let o = convert_report(jpeg).unwrap();
        let g = &o.report["integrity"];
        assert_eq!(g["pixel_sha256_match"], true);
        assert_eq!(rep_str(g, &["original_pixel_sha256"]), integrity::pixel_sha256_standard(&src));
        assert_eq!(rep_str(g, &["file_sha256"]), sha256_hex(&o.body));
        assert_eq!(o.report["codec_id"], 3);
        assert_eq!(g["crc_stored"], g["crc_recomputed"]);
    }

    /// v1 files have no header checksum, so a corrupted stored CRC reaches the pixel-CRC comparison.
    #[test]
    fn crc_mismatch_is_reported_as_integrity_failure() {
        let mut png = Vec::new();
        PngEncoder::new(&mut png).write_image(&[7u8; 12 * 9 * 3], 12, 9, ExtendedColorType::Rgb8).unwrap();
        let img = Canonical::load_bytes(png).unwrap();
        let mut v1 = nanoscope::pkonfile::encode(&img).unwrap();
        let hdr = nanoscope::pkonfile::parse_header(&v1).unwrap();
        let crc_at = v1.len() - hdr.payload_len as usize - 4;
        v1[crc_at] ^= 1;
        match restore_report(&v1) {
            Err(Fail { msg, report: Some(r) }) => {
                assert!(msg.contains("integrity failure"), "{msg}");
                assert_eq!(r["integrity"]["crc_match"], false);
            }
            other => panic!("expected integrity failure, got ok={}", other.is_ok()),
        }
        assert!(restore_report(&v1[..10]).is_err());
    }

    #[test]
    fn invalid_inputs_rejected() {
        assert!(convert_report(b"plain text".to_vec()).is_err());
        assert!(convert_report(Vec::new()).is_err());
        // TIFF magic but not a DNG: must fail cleanly through the DNG path, never reach PKON2-X4
        let mut tiff = b"II*\0".to_vec();
        tiff.extend_from_slice(&[0u8; 64]);
        assert!(convert_report(tiff).is_err());
    }

    /// Real DNG end to end: ingestion equals the frozen canonical samples; LISA round trip; hashes and CRC agree.
    #[test]
    fn dng_raw_roundtrip_via_lisa() {
        let (dng_path, canon_path) = ("data/praw1/dng/nikon_d80_piano.dng", "data/praw1/canon/nikon_d80_piano.u16");
        let (Ok(dng), Ok(canon)) = (std::fs::read(dng_path), std::fs::read(canon_path)) else { return };
        let ing = dng::ingest_dng(&dng).unwrap();
        let flat: Vec<u8> = ing.samples.iter().flat_map(|s| s.to_le_bytes()).collect();
        assert!(flat == canon, "ingested samples differ from the frozen canonical raster");
        let o = convert_report(dng).unwrap();
        assert_eq!(o.report["codec"], "LISA");
        assert_eq!(o.report["codec_id"], 4);
        let g = &o.report["integrity"];
        assert_eq!(g["pixel_sha256_match"], true);
        assert_eq!(g["crc_match"], true);
        assert_eq!(rep_str(g, &["original_pixel_sha256"]), integrity::pixel_sha256_u16(&ing.samples, ing.bit_depth));
        let r = restore_report(&o.body).unwrap();
        assert_eq!(r.body, flat, "exported samples must equal the ingested samples byte for byte");
        assert_eq!(r.report["cfa"], o.report["cfa"]);
        assert_eq!(r.report["bit_depth"], o.report["bit_depth"]);
        assert_eq!(r.report["integrity"]["reconstructed_pixel_sha256"], g["original_pixel_sha256"]);
        assert_eq!(r.report["integrity"]["file_sha256"], g["file_sha256"]);
        println!("{} -> {} B pkon, ratio {:.3}, bits {}, cfa {}", dng_path, o.body.len(), o.report["ratio"].as_f64().unwrap(), o.report["bit_depth"], o.report["cfa"]);
        // a corrupted RAW .pkon must be rejected, not silently decoded
        let mut bad = o.body.clone();
        let n = bad.len();
        bad[n / 2] ^= 0x40;
        assert!(restore_report(&bad).is_err());
    }
}
