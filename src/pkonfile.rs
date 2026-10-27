//! The `.pkon2` container, format version 1. Normative spec: `docs/pkon2-format-v1.md`.
//!
//! A file is a fixed 24-byte header followed by exactly `payload_len` bytes of PKON2 range-coder stream (the
//! unchanged output of `pkon2x::run_x` + `BinEnc::finish`); codec 1 = X4, codec 2 = X4 + predictor disagreement (X4P). All integers are little-endian and written/read byte by
//! byte; nothing here depends on struct layout or host endianness. Parsing (`parse_header`), validation, and the
//! pixel pipeline (`encode_payload` / `decode_payload`) are separate functions so each can be audited on its own.
use crate::canon::{Canonical, Layout, Spec};
use crate::pkon2::{color_fwd, color_inv, to_planes, Color, Planes};
use crate::pkon2x::{run_x, x4p_cfg, BinDec, BinEnc, Coder, In, XCfg, Bias, Ens};
use std::fmt;

pub const MAGIC: [u8; 4] = [0x89, b'P', b'K', b'2'];
pub const VERSION: u8 = 1;
pub const HEADER_LEN: usize = 24;

/// Codec id 1: PKON2 X4 as promoted by N3 (inputs Ab,B,C,D; sign contexts; no bias; no ensemble), planes coded in
/// G,R,B,(A) order for RGB(A) / natural order otherwise, SubG colour transform when >= 3 planes.
pub const CODEC_X4: u8 = 1;
/// Codec id 2: codec 1 plus the predictor-disagreement mixer input (`In::P`, PKON2P), nothing else changed. This is what `encode` writes.
/// Files with codec id 1 remain valid and decodable; `encode_with_codec(img, CODEC_X4)` still produces them.
pub const CODEC_X4P: u8 = 2;
/// The codec `encode` uses.
pub const CODEC_CURRENT: u8 = CODEC_X4P;

/// Default resource policy: largest reconstructed pixel buffer a decode will allocate. Peak decode memory is about
/// 3-4x this (planes + two residual planes + output). Raise per call with `decode_with_limit`.
pub const DEFAULT_MAX_DECODED_BYTES: u64 = 1 << 28;

/// Every sample codes at least one binary decision whose cost is >= -log2(65520/65536) ~ 3.5e-4 bits (probability
/// clamp), i.e. >= 1 payload byte per ~22,700 samples. A header claiming more than 32,768 samples per payload byte
/// cannot be a valid stream and is rejected before any decoding work is done.
const MAX_SAMPLES_PER_PAYLOAD_BYTE: u64 = 32_768;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PkonError {
    /// Input ended before the header or the declared payload was complete.
    Truncated { need: usize, have: usize },
    BadMagic,
    UnsupportedVersion(u8),
    /// Non-zero flags byte (all flag bits are reserved and mandatory-to-understand in v1).
    ReservedFlags(u8),
    UnsupportedCodec(u8),
    UnsupportedPixelFormat(u8),
    /// Zero width or height.
    InvalidDimensions { width: u32, height: u32 },
    /// Reconstructed buffer would exceed the resource limit (or overflow arithmetic).
    TooLarge { width: u32, height: u32, limit: u64 },
    /// `payload_len` is impossible for the declared image.
    InvalidPayloadLength { payload_len: u32 },
    TrailingBytes(usize),
    /// The range decoder did not end exactly at the end of the payload.
    CorruptPayload,
    ChecksumMismatch { expected: u32, actual: u32 },
    /// Encoder input the format cannot represent (e.g. 16-bit samples).
    UnsupportedImage(String),
}

impl fmt::Display for PkonError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use PkonError::*;
        match self {
            Truncated { need, have } => write!(f, "truncated: need {need} bytes, have {have}"),
            BadMagic => write!(f, "not a .pkon2 file (bad magic)"),
            UnsupportedVersion(v) => write!(f, "unsupported format version {v}"),
            ReservedFlags(b) => write!(f, "reserved flags set: {b:#04x}"),
            UnsupportedCodec(c) => write!(f, "unsupported codec id {c}"),
            UnsupportedPixelFormat(p) => write!(f, "unsupported pixel format {p}"),
            InvalidDimensions { width, height } => write!(f, "invalid dimensions {width}x{height}"),
            TooLarge { width, height, limit } => write!(f, "{width}x{height} image exceeds the {limit}-byte decode limit"),
            InvalidPayloadLength { payload_len } => write!(f, "payload length {payload_len} is impossible for the declared image"),
            TrailingBytes(n) => write!(f, "{n} unexpected trailing bytes after payload"),
            CorruptPayload => write!(f, "corrupt payload (range decoder did not end at payload end)"),
            ChecksumMismatch { expected, actual } => write!(f, "checksum mismatch: header {expected:#010x}, pixels {actual:#010x}"),
            UnsupportedImage(s) => write!(f, "unsupported image: {s}"),
        }
    }
}
impl std::error::Error for PkonError {}

/// Parsed and validated header (everything except the payload).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Header {
    pub spec: Spec,
    pub codec: u8,
    pub payload_len: u32,
    /// CRC-32 (IEEE) of the reconstructed pixel bytes.
    pub crc32: u32,
}

fn format_code(l: Layout) -> u8 {
    match l {
        Layout::Gray => 0,
        Layout::GrayAlpha => 1,
        Layout::Rgb => 2,
        Layout::Rgba => 3,
    }
}
fn format_layout(code: u8) -> Option<Layout> {
    Some(match code {
        0 => Layout::Gray,
        1 => Layout::GrayAlpha,
        2 => Layout::Rgb,
        3 => Layout::Rgba,
        _ => return None,
    })
}

/// CRC-32/ISO-HDLC (zlib, PNG, Ethernet): poly 0xEDB88320 reflected, init and xorout 0xFFFFFFFF. Check("123456789") = 0xCBF43926.
/// Detects accidental corruption only; it is not authentication.
pub fn crc32(data: &[u8]) -> u32 {
    let mut crc = !0u32;
    for &b in data {
        crc ^= b as u32;
        for _ in 0..8 {
            crc = (crc >> 1) ^ (0xEDB8_8320 & (crc & 1).wrapping_neg());
        }
    }
    !crc
}

fn rd32(b: &[u8], at: usize) -> u32 {
    u32::from_le_bytes([b[at], b[at + 1], b[at + 2], b[at + 3]])
}

/// Parse and validate the 24-byte header with the default resource limit.
pub fn parse_header(bytes: &[u8]) -> Result<Header, PkonError> {
    parse_header_with_limit(bytes, DEFAULT_MAX_DECODED_BYTES)
}

/// Header validation only: no allocation, no payload access (payload presence is checked by `decode`).
pub fn parse_header_with_limit(bytes: &[u8], max_decoded_bytes: u64) -> Result<Header, PkonError> {
    if bytes.len() < HEADER_LEN {
        // A wrong magic is more useful than "truncated" when the prefix we do have already differs.
        let n = bytes.len().min(4);
        if bytes[..n] != MAGIC[..n] {
            return Err(PkonError::BadMagic);
        }
        return Err(PkonError::Truncated { need: HEADER_LEN, have: bytes.len() });
    }
    if bytes[0..4] != MAGIC {
        return Err(PkonError::BadMagic);
    }
    if bytes[4] != VERSION {
        return Err(PkonError::UnsupportedVersion(bytes[4]));
    }
    if bytes[5] != 0 {
        return Err(PkonError::ReservedFlags(bytes[5]));
    }
    if bytes[6] != CODEC_X4 && bytes[6] != CODEC_X4P {
        return Err(PkonError::UnsupportedCodec(bytes[6]));
    }
    let layout = format_layout(bytes[7]).ok_or(PkonError::UnsupportedPixelFormat(bytes[7]))?;
    let (width, height) = (rd32(bytes, 8), rd32(bytes, 12));
    let payload_len = rd32(bytes, 16);
    let crc32 = rd32(bytes, 20);
    if width == 0 || height == 0 {
        return Err(PkonError::InvalidDimensions { width, height });
    }
    let too_large = PkonError::TooLarge { width, height, limit: max_decoded_bytes };
    let samples = (width as u64).checked_mul(height as u64).and_then(|p| p.checked_mul(layout.channels() as u64)).ok_or(too_large.clone())?;
    // 8-bit samples only, so decoded bytes == samples. The limit must also be addressable on this platform.
    if samples > max_decoded_bytes || usize::try_from(samples).is_err() {
        return Err(too_large);
    }
    if payload_len == 0 || samples / MAX_SAMPLES_PER_PAYLOAD_BYTE > payload_len as u64 {
        return Err(PkonError::InvalidPayloadLength { payload_len });
    }
    Ok(Header { spec: Spec { width, height, layout, bit_depth: 8 }, codec: bytes[6], payload_len, crc32 })
}

/// The frozen configuration behind codec id 1. Spelled out (not `x4_cfg()`) so a later change to the research
/// default cannot silently change how existing files decode; a test asserts it equals `x4_cfg()` today.
fn codec1_cfg() -> XCfg {
    XCfg { name: "X4", inputs: vec![In::Ab, In::B, In::C, In::D], sign_ctx: true, bias: Bias::Off, ens: Ens::Off }
}

/// The frozen configuration behind codec id 2, spelled out for the same reason as `codec1_cfg`.
fn codec2_cfg() -> XCfg {
    XCfg { name: "X4P", inputs: vec![In::Ab, In::B, In::C, In::D, In::P], sign_ctx: true, bias: Bias::Off, ens: Ens::Off }
}

fn cfg_for(codec: u8) -> XCfg {
    match codec {
        CODEC_X4 => codec1_cfg(),
        _ => codec2_cfg(),
    }
}

/// Coding-order plane -> interleaved channel index. RGB(A) is coded as G,R,B,(A); gray(+alpha) in natural order.
fn plane_channels(l: Layout) -> &'static [usize] {
    match l {
        Layout::Gray => &[0],
        Layout::GrayAlpha => &[0, 1],
        Layout::Rgb => &[1, 0, 2],
        Layout::Rgba => &[1, 0, 2, 3],
    }
}

/// The bare codec-1 (X4) range-coder stream for `img` (what `measure_x` counts, minus its notional 11 header bytes).
pub fn encode_payload(img: &Canonical) -> Vec<u8> {
    encode_payload_codec(img, CODEC_X4)
}

/// The bare range-coder stream of codec `codec` (1 or 2).
pub fn encode_payload_codec(img: &Canonical, codec: u8) -> Vec<u8> {
    let cfg = cfg_for(codec);
    let mut work = to_planes(img);
    color_fwd(&mut work, Color::SubG);
    let mut coder = Coder::new(work.p.len(), &cfg);
    let mut enc = BinEnc::new();
    run_x(&mut work, &cfg, &mut coder, &mut enc);
    enc.finish()
}

/// Decode a bare codec-1 stream for an already-validated `spec` (8-bit). Caller has checked the size against its limit.
pub fn decode_payload(stream: &[u8], spec: &Spec) -> Result<Vec<u8>, PkonError> {
    decode_payload_codec(stream, spec, CODEC_X4)
}

/// Decode a bare stream of codec `codec` (1 or 2, already validated by `parse_header`).
pub fn decode_payload_codec(stream: &[u8], spec: &Spec, codec: u8) -> Result<Vec<u8>, PkonError> {
    let cfg = cfg_for(codec);
    let (w, h) = (spec.width as usize, spec.height as usize);
    let order = plane_channels(spec.layout);
    let mut pl = Planes { w, h, p: vec![vec![0; w * h]; order.len()] };
    let mut coder = Coder::new(order.len(), &cfg);
    let mut dec = BinDec::new(stream);
    run_x(&mut pl, &cfg, &mut coder, &mut dec);
    // Encoder flush makes the decoder consume exactly the stream; anything else is a damaged or mis-sized stream.
    if dec.consumed() != stream.len() {
        return Err(PkonError::CorruptPayload);
    }
    color_inv(&mut pl, Color::SubG);
    let ch = order.len();
    let mut data = vec![0u8; w * h * ch];
    for (plane, &c) in pl.p.iter().zip(order) {
        for (i, &v) in plane.iter().enumerate() {
            data[i * ch + c] = v;
        }
    }
    Ok(data)
}

/// Encode a complete `.pkon2` file with the current codec (id 2). Only 8-bit gray / gray+alpha / RGB / RGBA are representable.
pub fn encode(img: &Canonical) -> Result<Vec<u8>, PkonError> {
    encode_with_codec(img, CODEC_CURRENT)
}

/// Encode a complete `.pkon2` file with a specific codec id (1 = X4, 2 = X4P).
pub fn encode_with_codec(img: &Canonical, codec: u8) -> Result<Vec<u8>, PkonError> {
    if codec != CODEC_X4 && codec != CODEC_X4P {
        return Err(PkonError::UnsupportedCodec(codec));
    }
    if img.spec.bit_depth != 8 {
        return Err(PkonError::UnsupportedImage(format!("{}-bit samples (v1 is 8-bit only)", img.spec.bit_depth)));
    }
    if img.spec.width == 0 || img.spec.height == 0 || img.data.len() != img.spec.byte_len() {
        return Err(PkonError::UnsupportedImage("inconsistent image buffer".into()));
    }
    let payload = encode_payload_codec(img, codec);
    let payload_len = u32::try_from(payload.len()).map_err(|_| PkonError::UnsupportedImage("payload exceeds 4 GiB".into()))?;
    let mut out = Vec::with_capacity(HEADER_LEN + payload.len());
    out.extend_from_slice(&MAGIC);
    out.extend_from_slice(&[VERSION, 0, codec, format_code(img.spec.layout)]);
    out.extend_from_slice(&img.spec.width.to_le_bytes());
    out.extend_from_slice(&img.spec.height.to_le_bytes());
    out.extend_from_slice(&payload_len.to_le_bytes());
    out.extend_from_slice(&crc32(&img.data).to_le_bytes());
    out.extend_from_slice(&payload);
    // Fail loudly if our own output would not pass our own header validation (e.g. impossible payload bound).
    parse_header_with_limit(&out, u64::MAX)?;
    Ok(out)
}

/// Decode a complete `.pkon2` file with the default resource limit.
pub fn decode(bytes: &[u8]) -> Result<Canonical, PkonError> {
    decode_with_limit(bytes, DEFAULT_MAX_DECODED_BYTES)
}

/// Decode a complete `.pkon2` file; `max_decoded_bytes` caps the reconstructed pixel buffer.
pub fn decode_with_limit(bytes: &[u8], max_decoded_bytes: u64) -> Result<Canonical, PkonError> {
    let hdr = parse_header_with_limit(bytes, max_decoded_bytes)?;
    let end = usize::try_from(HEADER_LEN as u64 + hdr.payload_len as u64)
        .map_err(|_| PkonError::InvalidPayloadLength { payload_len: hdr.payload_len })?;
    if bytes.len() < end {
        return Err(PkonError::Truncated { need: end, have: bytes.len() });
    }
    if bytes.len() > end {
        return Err(PkonError::TrailingBytes(bytes.len() - end));
    }
    let data = decode_payload_codec(&bytes[HEADER_LEN..end], &hdr.spec, hdr.codec)?;
    let actual = crc32(&data);
    if actual != hdr.crc32 {
        return Err(PkonError::ChecksumMismatch { expected: hdr.crc32, actual });
    }
    Canonical::new(hdr.spec, data).map_err(PkonError::UnsupportedImage)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pkon2x::x4_cfg;

    #[test]
    fn crc32_check_value() {
        assert_eq!(crc32(b"123456789"), 0xCBF4_3926);
        assert_eq!(crc32(b""), 0);
    }

    #[test]
    fn codec1_is_the_production_x4_config() {
        assert_eq!(format!("{:?}", codec1_cfg()), format!("{:?}", x4_cfg()));
    }

    #[test]
    fn codec2_is_the_production_x4p_config() {
        assert_eq!(format!("{:?}", codec2_cfg()), format!("{:?}", crate::pkon2x::x4p_cfg()));
    }
}
