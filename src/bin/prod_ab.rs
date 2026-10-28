//! REL6 production A/B validation. A = production codec 2 (`pkonfile::encode`, the default), B = production codec 3 (`encode_with_codec(.., CODEC_X4PJ)`).
//! Only the public production API (`pkonfile`) is used for encode/decode; nothing from the research harness.
//!
//!   cargo run --release --bin prod_ab -- --phase 1|2 --out <dir>
//!
//! Phase 1 loads ONLY the DEV (25) + VAL (10) images; phase 2 loads ONLY the 15 TEST images (split into the 8 lightly touched and the 7 used in
//! development/profiling). Rules, thresholds and timing protocol are the task's; nothing here is tuned.
use nanoscope::canon::Canonical;
use nanoscope::pkon2::center_crop;
use nanoscope::pkonfile::{self, decode, encode, encode_with_codec, parse_header, CODEC_X4P, CODEC_X4PJ, HEADER_LEN};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::time::Instant;

const CROP: u32 = 1024;
const SPLITS: &str = "results/learn1-splits.json";
const PROFILE_CSV: &str = "results/20261027-212136Z-bitprof/per_image.csv";
const FROZEN_A: &str = "results/20261027-rel6-joint-sign/armA_frozen_control.csv";
const REPS: usize = 3;

fn sha(b: &[u8]) -> String {
    Sha256::digest(b).iter().map(|x| format!("{x:02x}")).collect()
}

struct Img {
    name: String,
    split: &'static str, // dev | val | test_light | test_used
    canon: Canonical,
    file: String,
    file_sha: String,
    crop_sha: String,
    content_hash: String,
}

fn load(phase: u32, out: &Path) -> Vec<Img> {
    let sj: serde_json::Value = serde_json::from_slice(&std::fs::read(SPLITS).unwrap()).unwrap();
    let get = |k: &str| sj[k].as_array().unwrap().iter().map(|s| s.as_str().unwrap().to_string()).collect::<Vec<_>>();
    let used: Vec<String> = std::fs::read_to_string(PROFILE_CSV).unwrap().lines().skip(1).filter(|l| l.split(',').nth(1) == Some("P")).map(|l| l.split(',').next().unwrap().to_string()).collect();
    let man: Vec<serde_json::Value> = serde_json::from_str(&std::fs::read_to_string("data/dataset50.json").unwrap()).unwrap();
    let crop_hash: BTreeMap<String, String> = {
        let m: serde_json::Value = serde_json::from_str(&std::fs::read_to_string("results/20261027-155433Z-bench50/dataset_manifest.json").unwrap()).unwrap();
        m["images"].as_array().unwrap().iter().map(|i| (i["name"].as_str().unwrap().to_string(), i["crop_sha256"].as_str().unwrap().to_string())).collect()
    };
    let mut groups: Vec<(&'static str, Vec<String>)> = vec![];
    if phase == 1 {
        groups.push(("dev", get("train_pool")));
        groups.push(("val", get("val")));
    } else {
        let test = get("test");
        groups.push(("test_light", test.iter().filter(|n| !used.contains(n)).cloned().collect()));
        groups.push(("test_used", test.iter().filter(|n| used.contains(n)).cloned().collect()));
    }
    let mut imgs = vec![];
    for (split, names) in &groups {
        for n in names {
            let m = man.iter().find(|m| m["name"].as_str() == Some(n)).unwrap();
            let file = m["file"].as_str().unwrap().to_string();
            let path = Path::new("data").join(&file);
            let file_sha = sha(&std::fs::read(&path).unwrap());
            let full = Canonical::load(&path).unwrap();
            let crop = center_crop(&full, CROP);
            let content_hash = crop.content_hash();
            assert_eq!(content_hash, crop_hash[n], "crop hash mismatch for {n}");
            assert_eq!(crop.spec.bit_depth, 8);
            imgs.push(Img { name: n.clone(), split, file, file_sha, crop_sha: sha(&crop.data), content_hash, canon: crop });
        }
    }
    let mut csv = String::from("phase,split,image,source_file,source_file_sha256,crop_w,crop_h,layout,bit_depth,crop_data_sha256,crop_content_hash_bench50\n");
    for im in &imgs {
        let _ = writeln!(csv, "{phase},{},{},{},{},{},{},{:?},{},{},{}", im.split, im.name, im.file, im.file_sha, im.canon.spec.width, im.canon.spec.height, im.canon.spec.layout, im.canon.spec.bit_depth, im.crop_sha, im.content_hash);
    }
    std::fs::write(out.join(format!("manifest_phase{phase}.csv")), csv).unwrap();
    imgs
}

fn ms(t: Instant) -> f64 {
    t.elapsed().as_secs_f64() * 1e3
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let arg = |k: &str| args.iter().position(|a| a == k).map(|i| args[i + 1].clone());
    if let Some(c) = arg("--mem-run") {
        // child process for peak-working-set measurement: load one DEV image (already-public phase-1 data), encode + decode with codec `c`, exit.
        let n = arg("--image").unwrap();
        let man: Vec<serde_json::Value> = serde_json::from_str(&std::fs::read_to_string("data/dataset50.json").unwrap()).unwrap();
        let m = man.iter().find(|m| m["name"].as_str() == Some(&n)).unwrap();
        let full = Canonical::load(&Path::new("data").join(m["file"].as_str().unwrap())).unwrap();
        let crop = center_crop(&full, CROP);
        drop(full);
        if c != "none" {
            let f = encode_with_codec(&crop, c.parse().unwrap()).unwrap();
            assert_eq!(decode(&f).unwrap().data, crop.data);
        }
        println!("ok {}", n);
        return;
    }
    if args.iter().any(|a| a == "--state-bytes") {
        for (name, cfg) in [("codec2", nanoscope::pkon2x::x4p_cfg()), ("codec3", nanoscope::pkon2x::x4pj_cfg())] {
            for planes in [1usize, 3, 4] {
                println!("{name} planes={planes} coder_state_bytes={}", nanoscope::pkon2x::Coder::new(planes, &cfg).state_bytes());
            }
        }
        return;
    }
    let phase: u32 = arg("--phase").unwrap().parse().unwrap();
    let out = PathBuf::from(arg("--out").unwrap());
    std::fs::create_dir_all(&out).unwrap();
    let frozen: BTreeMap<String, usize> = if phase == 1 {
        std::fs::read_to_string(FROZEN_A).unwrap().lines().skip(1).map(|l| {
            let f: Vec<&str> = l.split(',').collect();
            (f[0].to_string(), f[2].parse().unwrap())
        }).collect()
    } else {
        BTreeMap::new()
    };
    let imgs = load(phase, &out);
    eprintln!("phase {phase}: {} images loaded", imgs.len());

    let codecs = [("A", CODEC_X4P), ("B", CODEC_X4PJ)];
    let mut per = String::from("image,split,A_bytes,B_bytes,diff_B_minus_A,pct_change,exact_A,exact_B,codec_byte_A,codec_byte_B,A_equals_default_encode,A_equals_frozen_reference,deterministic\n");
    let mut tim = String::from("image,split,arm,rep,order_in_rep,encode_ms,decode_ms\n");
    let mut failures = vec![];
    for im in &imgs {
        // ---- correctness gates (one pass per arm, untimed) ----
        let mut bytes = [vec![], vec![]];
        let mut exact = [false; 2];
        let mut cb = [0u8; 2];
        for (ai, (_, codec)) in codecs.iter().enumerate() {
            let f = encode_with_codec(&im.canon, *codec).unwrap();
            let hdr = parse_header(&f).unwrap();
            cb[ai] = hdr.codec;
            let d = decode(&f).unwrap();
            exact[ai] = d.spec == im.canon.spec && d.data == im.canon.data && hdr.codec == *codec && hdr.spec == im.canon.spec && f.len() == HEADER_LEN + hdr.payload_len as usize;
            let f2 = encode_with_codec(&im.canon, *codec).unwrap();
            if f2 != f {
                failures.push(format!("{} arm {}: nondeterministic encode", im.name, codecs[ai].0));
            }
            bytes[ai] = f;
        }
        let a_default = encode(&im.canon).unwrap() == bytes[1]; // since promotion: the default writer is codec 3 (arm B); arm A is explicit codec 2
        let a_frozen = frozen.get(&im.name).map(|&v| v == bytes[0].len());
        for ai in 0..2 {
            if !exact[ai] {
                failures.push(format!("{} arm {}: reconstruction/format mismatch", im.name, codecs[ai].0));
            }
        }
        if !a_default {
            failures.push(format!("{}: arm A differs from default encode", im.name));
        }
        if a_frozen == Some(false) {
            failures.push(format!("{}: arm A differs from frozen reference size", im.name));
        }
        let (a, b) = (bytes[0].len() as i64, bytes[1].len() as i64);
        let _ = writeln!(per, "{},{},{},{},{},{:.4},{},{},{},{},{},{},true", im.name, im.split, a, b, b - a, 100.0 * (a - b) as f64 / a as f64, exact[0], exact[1], cb[0], cb[1], a_default, a_frozen.map(|x| x.to_string()).unwrap_or("n/a".into()));
        if failures.iter().any(|f| f.starts_with(&im.name)) {
            eprintln!("GATE FAILURE on {}: stopping", im.name);
            break;
        }
        // ---- timing: 1 warm-up (both arms) + 3 reps, arm order rotated ----
        for (ai, (_, codec)) in codecs.iter().enumerate() {
            let f = encode_with_codec(&im.canon, *codec).unwrap();
            let _ = decode(&f).unwrap();
        }
        for rep in 0..REPS {
            let order: [usize; 2] = if rep % 2 == 0 { [0, 1] } else { [1, 0] };
            for (pos, &ai) in order.iter().enumerate() {
                let t = Instant::now();
                let f = encode_with_codec(&im.canon, codecs[ai].1).unwrap();
                let e = ms(t);
                let t = Instant::now();
                let d = decode(&f).unwrap();
                let dm = ms(t);
                assert_eq!(d.data, im.canon.data, "timing-run reconstruction mismatch {}", im.name);
                assert_eq!(f, bytes[ai], "timing-run bytes changed {}", im.name);
                let _ = writeln!(tim, "{},{},{},{},{},{:.3},{:.3}", im.name, im.split, codecs[ai].0, rep + 1, pos + 1, e, dm);
            }
        }
        eprintln!("{:<14} {:<10} A={} B={} ({:+.3}%)", im.name, im.split, a, b, 100.0 * (a - b) as f64 / a as f64);
    }
    std::fs::write(out.join(format!("per_image_phase{phase}.csv")), per).unwrap();
    std::fs::write(out.join(format!("timings_phase{phase}.csv")), tim).unwrap();
    let _ = pkonfile::HEADER_LEN;
    if failures.is_empty() {
        println!("GATES PASS ({} images)", imgs.len());
    } else {
        for f in &failures {
            println!("GATE FAIL: {f}");
        }
        std::process::exit(2);
    }
}
