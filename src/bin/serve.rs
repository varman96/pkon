//! Minimal localhost-only converter UI: PNG/JPEG -> .pkon (production encoder, current default codec) and .pkon -> PNG.
//!
//!   cargo run --release --bin serve            # then open http://127.0.0.1:8080/
//!
//! Pixels are what `Canonical::load_bytes` decodes (8-bit gray / gray+alpha / RGB / RGBA, row-major, interleaved).
//! A JPEG is encoded from its *decoded* pixels; the pre-JPEG original is not recoverable. Nothing is stored on disk.
use nanoscope::canon::{Canonical, Layout};
use nanoscope::pkonfile::{decode, encode};
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::time::Duration;

const PORT: u16 = 8080;
const MAX_UPLOAD: usize = 64 << 20;
const PAGE: &str = include_str!("serve.html");
const VIEWER: &str = include_str!("viewer.html");

fn convert(input: Vec<u8>) -> Result<(Vec<u8>, String), String> {
    let img = Canonical::load_bytes(input).map_err(|e| format!("cannot read image (PNG or JPEG, 8-bit): {e}"))?;
    let out = encode(&img).map_err(|e| e.to_string())?;
    Ok((out, format!("{}x{} {}", img.spec.width, img.spec.height, img.spec.label())))
}

fn restore(input: &[u8]) -> Result<(Vec<u8>, String), String> {
    use image::{codecs::png::PngEncoder, ExtendedColorType, ImageEncoder};
    let img = decode(input).map_err(|e| e.to_string())?;
    let ct = match img.spec.layout {
        Layout::Gray => ExtendedColorType::L8,
        Layout::GrayAlpha => ExtendedColorType::La8,
        Layout::Rgb => ExtendedColorType::Rgb8,
        Layout::Rgba => ExtendedColorType::Rgba8,
    };
    let mut png = Vec::new();
    PngEncoder::new(&mut png).write_image(&img.data, img.spec.width, img.spec.height, ct).map_err(|e| e.to_string())?;
    Ok((png, format!("{}x{} {}", img.spec.width, img.spec.height, img.spec.label())))
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
        "/convert" => convert(body),
        "/restore" => restore(&body),
        _ => view_data(&body),
    });
    match result {
        Ok(Ok((out, info))) => {
            let ctype = if op == "/restore" { "image/png" } else { "application/octet-stream" };
            let extra = if op == "/view-data" { info } else { format!("X-Info: {info}\r\n") };
            respond(&mut s, "200 OK", ctype, &extra, &out)
        }
        Ok(Err(e)) => respond(&mut s, "400 Bad Request", "text/plain; charset=utf-8", "", e.as_bytes()),
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
}
