//! Builds .pkon fixtures with the production encoder, plus `.raw` reference pixels from the production decoder, and
//! malformed variants. Usage: mkfixture <input.png|jpg> <out_dir> [crop_size]
use image::GenericImageView;
use nanoscope::canon::{Canonical, Layout, Spec};
use nanoscope::pkonfile::{decode, encode_with_codec};
use std::fs;
use std::path::Path;

fn emit(dir: &Path, name: &str, img: &Canonical, codec: u8) {
    let pk = encode_with_codec(img, codec).unwrap();
    let back = decode(&pk).unwrap();
    assert_eq!(back.data, img.data, "encoder/decoder roundtrip");
    fs::write(dir.join(format!("{name}.pkon")), &pk).unwrap();
    fs::write(dir.join(format!("{name}.raw")), &back.data).unwrap();
    println!("{name}: {}x{} {} bytes -> {}", img.spec.width, img.spec.height, img.spec.label(), pk.len());
}

fn main() {
    let a: Vec<String> = std::env::args().collect();
    let dir = Path::new(&a[2]);
    fs::create_dir_all(dir).unwrap();
    let crop: u32 = a.get(3).map(|s| s.parse().unwrap()).unwrap_or(192);
    let dynimg = image::open(&a[1]).unwrap();
    let (w, h) = dynimg.dimensions();
    let (cw, ch) = (crop.min(w), crop.min(h));
    let rgb = Canonical::from_dynamic(dynimg.crop_imm(0, 0, cw, ch)).unwrap();
    assert_eq!(rgb.spec.layout, Layout::Rgb);
    for c in 1..=3u8 {
        emit(dir, &format!("rgb_c{c}"), &rgb, c);
    }
    // Derived gray / rgba (synthetic alpha) / gray+alpha images from the same pixels.
    let n = (cw * ch) as usize;
    let gray: Vec<u8> = (0..n).map(|i| rgb.data[i * 3 + 1]).collect();
    let rgba: Vec<u8> = (0..n).flat_map(|i| [rgb.data[i * 3], rgb.data[i * 3 + 1], rgb.data[i * 3 + 2], (i % 251) as u8]).collect();
    let ga: Vec<u8> = (0..n).flat_map(|i| [gray[i], (i % 253) as u8]).collect();
    let mk = |layout, data| Canonical::new(Spec { width: cw, height: ch, layout, bit_depth: 8 }, data).unwrap();
    emit(dir, "gray_c3", &mk(Layout::Gray, gray), 3);
    emit(dir, "rgba_c3", &mk(Layout::Rgba, rgba), 3);
    emit(dir, "graya_c3", &mk(Layout::GrayAlpha, ga), 3);

    // Malformed variants of rgb_c3.
    let good = fs::read(dir.join("rgb_c3.pkon")).unwrap();
    let w = |n: &str, b: &[u8]| fs::write(dir.join(format!("bad_{n}.pkon")), b).unwrap();
    w("empty", &[]);
    w("badmagic", &{ let mut b = good.clone(); b[1] = b'X'; b });
    w("truncated_header", &good[..10]);
    w("truncated_payload", &good[..good.len() / 2]);
    w("trailing", &{ let mut b = good.clone(); b.push(0); b });
    w("codec9", &{ let mut b = good.clone(); b[6] = 9; b });
    w("zero_width", &{ let mut b = good.clone(); b[8..12].fill(0); b });
    w("huge_dims", &{ let mut b = good.clone(); b[8..12].copy_from_slice(&0xFFFF_FFFFu32.to_le_bytes()); b[12..16].copy_from_slice(&0xFFFF_FFFFu32.to_le_bytes()); b });
    w("flipped_payload", &{ let mut b = good.clone(); let m = b.len() / 2; b[m] ^= 0x55; b });
}
