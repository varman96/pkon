//! ENTMAP: crop exporter for the entropy-map analysis (measurement input only; no codec is run, nothing is encoded or decoded
//! except the source files themselves). Loads every image of a bench25 results dir exactly as the benchmark did
//! (`Canonical::load` + `center_crop(1024)`), asserts the crop SHA-256 against that run's `manifest_pre_encode.json`, and
//! writes the interleaved 8-bit crop bytes so `tools/entmap_analyze.py` sees the identical buffers every codec received.
//!   <out>/<name>.raw   interleaved samples, row-major (R,G,B[,A])
//!   <out>/index.json   name, w, h, channels, crop_sha256 (verified)
//!   cargo run --release --bin entmap_export -- <bench25 results dir> <out dir>
use nanoscope::canon::Canonical;
use nanoscope::pkon2::center_crop;
use std::path::{Path, PathBuf};

const CROP: u32 = 1024;

fn main() {
    let a: Vec<String> = std::env::args().collect();
    let (bench, out) = (PathBuf::from(&a[1]), PathBuf::from(&a[2]));
    let man: Vec<serde_json::Value> = serde_json::from_str(&std::fs::read_to_string(bench.join("manifest_pre_encode.json")).unwrap()).unwrap();
    std::fs::create_dir_all(&out).unwrap();
    let mut index = vec![];
    for m in &man {
        let name = m["name"].as_str().unwrap();
        let full = Canonical::load(Path::new(m["source_path"].as_str().unwrap())).unwrap();
        let crop = center_crop(&full, CROP);
        let want = m["crop_sha256"].as_str().unwrap();
        assert_eq!(crop.content_hash(), want, "crop hash mismatch for {name}");
        std::fs::write(out.join(format!("{name}.raw")), &crop.data).unwrap();
        index.push(serde_json::json!({"name": name, "w": crop.spec.width, "h": crop.spec.height,
            "channels": crop.spec.layout.channels(), "crop_sha256": want,
            "full_w": full.spec.width, "full_h": full.spec.height}));
        println!("{name:14} {}x{} ch {} sha256 ok", crop.spec.width, crop.spec.height, crop.spec.layout.channels());
    }
    std::fs::write(out.join("index.json"), serde_json::to_string_pretty(&index).unwrap()).unwrap();
}
