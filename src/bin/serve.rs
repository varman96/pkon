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
    if method != "POST" || !(path == "/convert" || path == "/restore") {
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
    let convert_op = path == "/convert";
    // The codecs must never take the server down: a panic on hostile input becomes a 500.
    let result = std::panic::catch_unwind(|| if convert_op { convert(body) } else { restore(&body) });
    match result {
        Ok(Ok((out, info))) => {
            let ctype = if convert_op { "application/octet-stream" } else { "image/png" };
            respond(&mut s, "200 OK", ctype, &format!("X-Info: {info}\r\n"), &out)
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

    #[test]
    fn garbage_is_rejected() {
        assert!(convert(b"not an image".to_vec()).is_err());
        assert!(restore(b"not a pkon file at all, definitely").is_err());
        assert!(restore(&[]).is_err());
    }
}
