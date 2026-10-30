//! Production benchmark of the real `.pkon2` file path on the 50-image set (same 1024-px centre crops as bench50), one codec per process
//! so peak memory is attributable. Measurement only.
//!
//!   cargo run --release --bin bench_p -- --codec 2 --out results/20261027-pkon2p-production/run_c2_a.csv
//!
//! Per image: encode (best of 2) -> complete file -> decode from the file bytes (best of 2) -> byte compare with the crop's pixels, header checks,
//! determinism. Crop hashes are verified against results/20261027-155433Z-bench50/dataset_manifest.json. Run through the pre-change tree it is
//! built with `encode_with_codec(img, codec)` replaced by `encode(img)` (the only API difference).
use nanoscope::canon::Canonical;
use nanoscope::pkon2::center_crop;
use nanoscope::pkonfile::{self, crc32, HEADER_LEN};
use std::collections::BTreeMap;
use std::path::Path;
use std::time::Instant;

fn main() {
    let a: Vec<String> = std::env::args().collect();
    let arg = |k: &str| a.iter().position(|x| x == k).map(|i| a[i + 1].clone());
    let codec: u8 = arg("--codec").unwrap().parse().unwrap();
    let out = arg("--out").unwrap();
    let man: Vec<serde_json::Value> = serde_json::from_str(&std::fs::read_to_string("data/dataset50.json").unwrap()).unwrap();
    let crop_hash: BTreeMap<String, String> = {
        let m: serde_json::Value = serde_json::from_str(&std::fs::read_to_string("results/20261027-155433Z-bench50/dataset_manifest.json").unwrap()).unwrap();
        m["images"].as_array().unwrap().iter().map(|i| (i["name"].as_str().unwrap().to_string(), i["crop_sha256"].as_str().unwrap().to_string())).collect()
    };
    let mut csv = String::from("image,file_bytes,payload_bytes,raw_bytes,exact,header_ok,deterministic,enc_ms,dec_ms,header_codec\n");
    let (mut tf, mut te, mut td) = (0usize, 0f64, 0f64);
    for m in &man {
        let name = m["name"].as_str().unwrap();
        let full = Canonical::load(&Path::new("data").join(m["file"].as_str().unwrap())).unwrap();
        let img = center_crop(&full, 1024);
        assert_eq!(img.content_hash(), crop_hash[name], "crop hash mismatch for {name}");
        drop(full);
        let mut best_e = f64::MAX;
        let mut file = vec![];
        for _ in 0..2 {
            let t = Instant::now();
            #[allow(unused_mut)]
            let f = pkonfile::encode_with_codec(&img, codec).unwrap();
            best_e = best_e.min(t.elapsed().as_secs_f64() * 1e3);
            file = f;
        }
        let mut best_d = f64::MAX;
        let mut dec = None;
        for _ in 0..2 {
            let t = Instant::now();
            let d = pkonfile::decode(&file).unwrap();
            best_d = best_d.min(t.elapsed().as_secs_f64() * 1e3);
            dec = Some(d);
        }
        let dec = dec.unwrap();
        let exact = dec.spec == img.spec && dec.data == img.data;
        let h = pkonfile::parse_header(&file).unwrap();
        let header_ok = h.spec == img.spec && h.codec == codec && h.crc32 == crc32(&img.data) && file.len() == HEADER_LEN + h.payload_len as usize;
        let det = pkonfile::encode_with_codec(&img, codec).unwrap() == file;
        assert!(exact && header_ok && det, "{name}: exact {exact} header {header_ok} deterministic {det}");
        tf += file.len();
        te += best_e;
        td += best_d;
        csv += &format!("{name},{},{},{},{exact},{header_ok},{det},{best_e:.1},{best_d:.1},{}\n", file.len(), file.len() - HEADER_LEN, img.data.len(), h.codec);
        println!("{name}: {} B enc {best_e:.0} ms dec {best_d:.0} ms", file.len());
    }
    let pid = std::process::id();
    let peak = std::process::Command::new("powershell")
        .args(["-NoProfile", "-Command", &format!("(Get-Process -Id {pid}).PeakWorkingSet64")])
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_default();
    csv += &format!("#total,{tf},,,,,,{te:.1},{td:.1},{codec}\n#peak_working_set_bytes,{peak}\n");
    std::fs::write(&out, csv).unwrap();
    println!("total {tf} B, enc {te:.0} ms, dec {td:.0} ms, peak WS {peak}");
}
