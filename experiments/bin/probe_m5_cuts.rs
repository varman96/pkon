//! M5 step 1: measure the terciles of error energy E over the 20 camera JPEGs (before any benchmark).
use nanoscope::canon::Canonical;
use nanoscope::pkon2::*;

fn main() {
    let manifest: Vec<serde_json::Value> = serde_json::from_str(&std::fs::read_to_string("data/manifest.json").unwrap()).unwrap();
    let mut hist = vec![0u64; 1021];
    for m in manifest.iter().filter(|m| m["source_format"].as_str().unwrap().contains("jpeg")) {
        let img = center_crop(&Canonical::load(&std::path::Path::new("data").join(m["file"].as_str().unwrap())).unwrap(), 1024);
        for (a, b) in hist.iter_mut().zip(energy_histogram(&img)) {
            *a += b;
        }
    }
    let total: u64 = hist.iter().sum();
    let mut cum = 0u64;
    let mut cuts = vec![];
    for (e, &c) in hist.iter().enumerate() {
        cum += c;
        for q in [1u64, 2] {
            if cuts.len() as u64 == q - 1 && cum * 3 >= total * q {
                cuts.push(e);
            }
        }
    }
    println!("pooled samples {total}; terciles of E (level 0: E<=c0, 1: E<=c1, 2: above): {cuts:?}");
    for e in 0..12 {
        println!("E={e}: share {:.4}, cdf {:.4}", hist[e] as f64 / total as f64, hist[..=e].iter().sum::<u64>() as f64 / total as f64);
    }
}
