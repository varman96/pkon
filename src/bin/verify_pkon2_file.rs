//! `.pkon2` v1 verification on the 23-image corpus (same 1024-px centre crops every PKON2 report used): encode a complete
//! file, decode it from bytes alone, compare every byte and the recovered metadata, and compare the payload with the
//! historical production X4 byte counts (`x4-promotion-verify` reports stream + the notional 11 header bytes).
//!
//!   cargo run --release --bin verify_pkon2_file      # -> results/<stamp>-pkon2-file-v1/report.md
use nanoscope::canon::{first_mismatch, Canonical};
use nanoscope::pkon2::{center_crop, HEADER_BYTES};
use nanoscope::pkonfile::*;
use std::path::PathBuf;
use std::time::Instant;

const CROP: u32 = 1024;

fn main() {
    let data = PathBuf::from("data");
    let manifest: Vec<serde_json::Value> = serde_json::from_str(&std::fs::read_to_string(data.join("manifest.json")).unwrap()).unwrap();
    let prior = std::fs::read_to_string("results/20261027-x4-promotion-verify/report.md").unwrap();
    let prior_b = |name: &str| -> usize {
        let l = prior.lines().find(|l| l.starts_with(&format!("| {name} |"))).unwrap_or_else(|| panic!("{name} missing in prior report"));
        l.split('|').nth(4).unwrap().trim().parse().unwrap()
    };
    let mut rows = String::from("| image | layout | WxH | payload B | header B | file B | overhead B | overhead % of payload | payload == historical B-11 | metadata | pixels exact | deterministic | enc ms | dec ms |\n|---|---|---|---:|---:|---:|---:|---:|---|---|---|---|---:|---:|\n");
    let (mut tp, mut tf, mut n, mut ok) = (0usize, 0usize, 0, true);
    for src in ["jpeg", "png"] {
        for m in manifest.iter().filter(|m| m["source_format"].as_str().unwrap().ends_with(src)) {
            let name = m["name"].as_str().unwrap();
            let full = Canonical::load(&data.join(m["file"].as_str().unwrap())).unwrap();
            let img = center_crop(&full, CROP);
            let t = Instant::now();
            let file = encode(&img).unwrap();
            let enc = t.elapsed().as_secs_f64() * 1e3;
            let t = Instant::now();
            let dec = decode(&file).unwrap();
            let decm = t.elapsed().as_secs_f64() * 1e3;
            let hdr = parse_header(&file).unwrap();
            let payload = file.len() - HEADER_LEN;
            let hist = prior_b(name);
            let same_payload = payload + HEADER_BYTES == hist && hdr.payload_len as usize == payload && encode_payload(&img) == file[HEADER_LEN..];
            let meta = dec.spec == img.spec && hdr.spec == img.spec;
            let exact = first_mismatch(&img, &dec).is_none() && dec.data == img.data;
            let det = encode(&img).unwrap() == file;
            ok &= same_payload && meta && exact && det;
            let over = file.len() - payload;
            tp += payload;
            tf += file.len();
            n += 1;
            rows += &format!(
                "| {name} | {} | {}x{} | {payload} | {HEADER_LEN} | {} | {over} | {:.5}% | {same_payload} | {meta} | {exact} | {det} | {enc:.0} | {decm:.0} |\n",
                img.spec.layout.name(), img.spec.width, img.spec.height, file.len(), over as f64 * 100.0 / payload as f64
            );
            println!("{name}: payload {payload} file {} exact {exact} same_payload {same_payload} meta {meta} det {det}", file.len());
        }
    }
    let rep = format!(
        "# `.pkon2` v1 verification on the {n}-image corpus\n\nEncode -> complete file -> decode from file bytes only (no external spec) -> byte comparison. \"historical B\" is the production X4 byte count in `results/20261027-x4-promotion-verify/report.md`, which is stream + {HEADER_BYTES} notional header bytes that were never serialized.\n\n{rows}\nTotals: payload {tp} B, files {tf} B, container overhead {} B ({:.5}% of payload).\nAll images exact, metadata recovered, payload unchanged, output deterministic: {ok}\n",
        tf - tp, (tf - tp) as f64 * 100.0 / tp as f64
    );
    let stamp = std::process::Command::new("powershell").args(["-NoProfile", "-Command", "(Get-Date).ToUniversalTime().ToString('yyyyMMdd-HHmmss')+'Z'"]).output().map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string()).unwrap();
    let dir = PathBuf::from("results").join(format!("{stamp}-pkon2-file-v1"));
    assert!(!dir.exists(), "results dir exists");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("report.md"), &rep).unwrap();
    println!("{rep}\nwritten to {}", dir.display());
    assert!(ok, "verification failed");
}
