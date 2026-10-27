//! Focused gate for the `.pkon2` v1 container: exact round trips, header/limit/corruption policy, determinism,
//! payload equivalence with the unchanged X4 codec, and no-panic behaviour on malformed input.
use nanoscope::canon::{Canonical, Layout, Spec};
use nanoscope::corpus::synthetic;
use nanoscope::pkon2::HEADER_BYTES;
use nanoscope::pkon2x::{measure_x, x4_cfg};
use nanoscope::pkonfile::*;

fn img(w: u32, h: u32, layout: Layout, seed: u32) -> Canonical {
    let mut x = seed;
    let n = w as usize * h as usize * layout.channels();
    let data = (0..n)
        .map(|i| {
            x = x.wrapping_mul(1664525).wrapping_add(1013904223);
            ((i / layout.channels()) as u8).wrapping_add((x >> 24) as u8 & 15)
        })
        .collect();
    Canonical::new(Spec { width: w, height: h, layout, bit_depth: 8 }, data).unwrap()
}

fn sample_file() -> (Canonical, Vec<u8>) {
    let i = img(13, 7, Layout::Rgba, 7);
    let f = encode(&i).unwrap();
    (i, f)
}

fn derr(b: &[u8]) -> Option<PkonError> {
    decode(b).err()
}

/// Rewrite a header field in a copy of a valid file.
fn patched(f: &[u8], at: usize, bytes: &[u8]) -> Vec<u8> {
    let mut v = f.to_vec();
    v[at..at + bytes.len()].copy_from_slice(bytes);
    v
}

#[test]
fn all_layouts_round_trip_and_metadata_is_recovered() {
    for (w, h) in [(1, 1), (1, 9), (9, 1), (33, 17)] {
        for l in [Layout::Gray, Layout::GrayAlpha, Layout::Rgb, Layout::Rgba] {
            let i = img(w, h, l, w * 31 + h);
            let f = encode(&i).unwrap();
            let hdr = parse_header(&f).unwrap();
            assert_eq!(hdr.spec, i.spec);
            assert_eq!(hdr.payload_len as usize + HEADER_LEN, f.len());
            let d = decode(&f).unwrap();
            assert_eq!(d.spec, i.spec, "{w}x{h} {l:?}");
            assert_eq!(d.data, i.data, "{w}x{h} {l:?}");
        }
    }
}

#[test]
fn synthetic_corpus_exact_incl_hidden_alpha_and_payload_matches_x4() {
    let mut n = 0;
    for e in synthetic().iter().filter(|e| e.image.spec.bit_depth == 8) {
        let f = encode(&e.image).unwrap();
        let d = decode(&f).unwrap();
        assert_eq!(d.spec, e.image.spec, "{}", e.name);
        assert_eq!(d.data, e.image.data, "{} not byte-identical", e.name);
        // The payload is exactly what the existing codec counts (measure_x = stream + notional 11-byte header).
        let m = measure_x(&e.image, &x4_cfg());
        assert_eq!(f.len() - HEADER_LEN + HEADER_BYTES, m.coded_bytes, "{} payload size changed", e.name);
        assert_eq!(encode_payload(&e.image), &f[HEADER_LEN..], "{}", e.name);
        n += 1;
    }
    assert!(n >= 12);
}

#[test]
fn encoding_is_deterministic() {
    let i = img(40, 30, Layout::Rgb, 3);
    assert_eq!(encode(&i).unwrap(), encode(&i).unwrap());
}

/// Pins the codec-1 payload bytes: if the X4 codec ever changes, this fails and forces a new codec id.
#[test]
fn codec1_payload_is_pinned() {
    let p = encode_payload(&img(33, 17, Layout::Rgb, 12345));
    assert_eq!((p.len(), crc32(&p)), PINNED_33X17_RGB);
}
const PINNED_33X17_RGB: (usize, u32) = (1176, 2208450786);

#[test]
fn header_layout_is_byte_exact() {
    let (i, f) = sample_file();
    assert_eq!(&f[0..4], &[0x89, b'P', b'K', b'2']);
    assert_eq!(&f[4..8], &[1, 0, 1, 3]); // version, flags, codec, rgba
    assert_eq!(&f[8..12], &13u32.to_le_bytes());
    assert_eq!(&f[12..16], &7u32.to_le_bytes());
    assert_eq!(&f[16..20], &((f.len() - 24) as u32).to_le_bytes());
    assert_eq!(&f[20..24], &crc32(&i.data).to_le_bytes());
}

#[test]
fn bad_magic_version_flags_codec_format() {
    let (_, f) = sample_file();
    assert_eq!(derr(&patched(&f, 0, b"X")), Some(PkonError::BadMagic));
    assert_eq!(derr(&patched(&f, 3, b"3")), Some(PkonError::BadMagic));
    assert_eq!(derr(b"\x89PNG\r\n\x1a\n and more bytes to exceed the header length"), Some(PkonError::BadMagic));
    for v in [0u8, 2, 255] {
        assert_eq!(derr(&patched(&f, 4, &[v])), Some(PkonError::UnsupportedVersion(v)));
    }
    for fl in [1u8, 0x80, 0xFF] {
        assert_eq!(derr(&patched(&f, 5, &[fl])), Some(PkonError::ReservedFlags(fl)));
    }
    for c in [0u8, 2, 255] {
        assert_eq!(derr(&patched(&f, 6, &[c])), Some(PkonError::UnsupportedCodec(c)));
    }
    for p in [4u8, 5, 255] {
        assert_eq!(derr(&patched(&f, 7, &[p])), Some(PkonError::UnsupportedPixelFormat(p)));
    }
}

#[test]
fn sixteen_bit_and_inconsistent_images_are_rejected_on_encode() {
    let e = synthetic().into_iter().find(|e| e.image.spec.bit_depth == 16).unwrap();
    assert!(matches!(encode(&e.image), Err(PkonError::UnsupportedImage(_))));
    let mut bad = img(4, 4, Layout::Rgb, 1);
    bad.data.pop();
    assert!(matches!(encode(&bad), Err(PkonError::UnsupportedImage(_))));
}

#[test]
fn every_prefix_is_rejected_and_boundaries_classify() {
    let (_, f) = sample_file();
    for n in 0..f.len() {
        let r = derr(&f[..n]);
        assert!(r.is_some(), "prefix of {n} bytes decoded");
        if n == 0 {
            assert!(r.is_some());
        } else if n < HEADER_LEN {
            assert_eq!(r, Some(PkonError::Truncated { need: HEADER_LEN, have: n }), "n={n}");
        } else {
            assert_eq!(r, Some(PkonError::Truncated { need: f.len(), have: n }), "n={n}");
        }
        assert!(n >= HEADER_LEN || parse_header(&f[..n]).is_err());
    }
    assert!(parse_header(&f[..HEADER_LEN]).is_ok(), "header alone parses (payload presence is decode's job)");
}

#[test]
fn trailing_and_concatenated_data_is_rejected() {
    let (_, f) = sample_file();
    let mut t = f.clone();
    t.push(0);
    assert_eq!(derr(&t), Some(PkonError::TrailingBytes(1)));
    let mut c = f.clone();
    c.extend_from_slice(&f);
    assert_eq!(derr(&c), Some(PkonError::TrailingBytes(f.len())));
}

#[test]
fn invalid_dimensions_and_payload_lengths() {
    let (_, f) = sample_file();
    assert_eq!(derr(&patched(&f, 8, &0u32.to_le_bytes())), Some(PkonError::InvalidDimensions { width: 0, height: 7 }));
    assert_eq!(derr(&patched(&f, 12, &0u32.to_le_bytes())), Some(PkonError::InvalidDimensions { width: 13, height: 0 }));
    assert_eq!(derr(&patched(&f, 16, &0u32.to_le_bytes())), Some(PkonError::InvalidPayloadLength { payload_len: 0 }));
    // declared longer than the file -> truncated; shorter -> trailing bytes
    let pl = (f.len() - HEADER_LEN) as u32;
    assert!(matches!(decode(&patched(&f, 16, &(pl + 1).to_le_bytes())), Err(PkonError::Truncated { .. })));
    assert!(matches!(decode(&patched(&f, 16, &u32::MAX.to_le_bytes())), Err(PkonError::Truncated { .. })));
    assert!(matches!(decode(&patched(&f, 16, &(pl - 1).to_le_bytes())), Err(PkonError::TrailingBytes(1))));
}

#[test]
fn overflow_and_oversize_are_rejected_before_allocation() {
    let (_, f) = sample_file();
    let max = u32::MAX.to_le_bytes();
    // u32::MAX x u32::MAX x 4 overflows u64 products; both must be TooLarge, not a panic or an allocation attempt.
    let both = patched(&patched(&f, 8, &max), 12, &max);
    assert!(matches!(decode(&both), Err(PkonError::TooLarge { .. })));
    assert!(matches!(parse_header(&both), Err(PkonError::TooLarge { .. })));
    // exactly at / over the configured limit (13x7 RGBA = 364 bytes)
    assert!(parse_header_with_limit(&f, 364).is_ok());
    assert!(matches!(parse_header_with_limit(&f, 363), Err(PkonError::TooLarge { .. })));
    assert!(matches!(decode_with_limit(&f, 363), Err(PkonError::TooLarge { .. })));
    // within the limit but impossible for a 40-byte payload: rejected without decoding
    let big = patched(&patched(&f, 8, &8000u32.to_le_bytes()), 12, &8000u32.to_le_bytes());
    assert!(matches!(parse_header(&big), Err(PkonError::InvalidPayloadLength { .. })));
}

#[test]
fn corrupted_checksum_and_payload_are_detected() {
    let (_, f) = sample_file();
    for b in 20..24 {
        let mut c = f.clone();
        c[b] ^= 1;
        assert!(matches!(decode(&c), Err(PkonError::ChecksumMismatch { .. })), "crc byte {b}");
    }
    // The last 4 payload bytes are the range coder's flush: any value inside the final interval decodes identically, so a
    // flip there may be harmless. The invariant is therefore "never wrong pixels": an Ok result must equal the original.
    // Every flip before that tail must be rejected.
    let (orig, _) = sample_file();
    let tail = f.len() - 4;
    let mut benign = 0;
    for b in HEADER_LEN..f.len() {
        for bit in 0..8 {
            let mut c = f.clone();
            c[b] ^= 1 << bit;
            match decode(&c) {
                Ok(d) => {
                    assert!(b >= tail, "flip at payload byte {b} bit {bit} (before the flush tail) was accepted");
                    assert_eq!(d.data, orig.data, "accepted corrupt file with wrong pixels");
                    benign += 1;
                }
                Err(e) => assert!(matches!(e, PkonError::ChecksumMismatch { .. } | PkonError::CorruptPayload), "{e}"),
            }
        }
    }
    println!("{benign} of {} tail-bit flips were pixel-neutral", 4 * 8);
}

#[test]
fn width_height_format_corruption_is_never_accepted() {
    // Dimension / layout edits keep a structurally valid header; the pixel checksum (or stream-end check) must catch them.
    let (_, f) = sample_file();
    for (at, v) in [(8usize, 12u32), (8, 14), (12, 6), (12, 8), (8, 7), (12, 13)] {
        assert!(decode(&patched(&f, at, &v.to_le_bytes())).is_err(), "dim edit at {at} -> {v}");
    }
    for p in 0..3u8 {
        assert!(decode(&patched(&f, 7, &[p])).is_err(), "layout edit -> {p}");
    }
}

#[test]
fn mutated_inputs_never_panic() {
    // Deterministic xorshift mutator over valid files of every layout: random byte/bit edits, splices, truncations.
    let mut s = 0x9E37_79B9_7F4A_7C15u64;
    let mut next = move || {
        s ^= s << 13;
        s ^= s >> 7;
        s ^= s << 17;
        s
    };
    let files: Vec<Vec<u8>> = [Layout::Gray, Layout::GrayAlpha, Layout::Rgb, Layout::Rgba].iter().map(|&l| encode(&img(9, 5, l, 5)).unwrap()).collect();
    let mut ok = 0;
    for _ in 0..3000 {
        let mut f = files[(next() % 4) as usize].clone();
        for _ in 0..1 + next() % 4 {
            let i = (next() % f.len() as u64) as usize;
            match next() % 4 {
                0 => f[i] ^= 1 << (next() % 8),
                1 => f[i] = next() as u8,
                2 => f.truncate(i),
                _ => f.insert(i, next() as u8),
            }
            if f.is_empty() {
                break;
            }
        }
        // Small limit keeps mutated dimensions from turning a fuzz case into a long decode.
        if decode_with_limit(&f, 1 << 16).is_ok() {
            ok += 1;
        }
    }
    let _ = ok;
    // Arbitrary garbage with a valid header and a random payload must also return an error (or a checksum-clean image), not panic.
    let (_, base) = sample_file();
    for _ in 0..300 {
        let mut f = base.clone();
        for b in f[HEADER_LEN..].iter_mut() {
            *b = next() as u8;
        }
        assert!(decode(&f).is_err());
    }
}

#[test]
fn flat_image_stays_within_the_payload_bound() {
    // The densest realistic stream: validates MAX_SAMPLES_PER_PAYLOAD_BYTE does not reject legitimate output.
    for l in [Layout::Gray, Layout::Rgb, Layout::Rgba] {
        let i = Canonical::new(Spec { width: 1024, height: 1024, layout: l, bit_depth: 8 }, vec![77; 1024 * 1024 * l.channels()]).unwrap();
        let f = encode(&i).unwrap();
        println!("flat 1024x1024 {l:?}: {} samples -> {} payload bytes", i.data.len(), f.len() - HEADER_LEN);
        assert_eq!(decode(&f).unwrap().data, i.data);
    }
}
