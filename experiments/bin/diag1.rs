//! DIAG1: residual exporter (measurement only; no codec change). Loads ONLY the 25 DEV + 10 VAL images of results/learn1-splits.json
//! (never the test list), re-derives the split and re-checks every crop hash, then writes raw per-image arrays for `tools/diag1_analyze.py`:
//!   rgb.png            the 1024-px centre crop actually coded
//!   med_sub.i8         3 planes (G, R-G+128, B-G+128 = coded SubG planes) x N : signed MED residual (pixel - MED) mod 256 as i8, raster order
//!   med_rgb.i8         same MED on the untransformed planes (G, R, B)
//!   pcls.u8            3 x N : production P class (0..48) of the coded planes
//!   cost_x4p.f32       3 x N : exact bits spent on each pixel by production codec 2 (X4P)
//!   cost_x4.f32        3 x N : same for codec 1 (X4, no P)
//!   meta.json          sizes + consistency checks
//!   cargo run --release --bin diag1 -- --raw <dir>
use nanoscope::canon::{Canonical, Layout};
use nanoscope::pkon2::{center_crop, neighbors, predict, zigzag, Pred};
use nanoscope::pkon2x::{p_class, BinEnc, BitIo};
use nanoscope::pkonfile::{encode_payload_codec, CODEC_X4, CODEC_X4P};
use nanoscope::pkonlearn::prepared;
use nanoscope::pkonrel::*;
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

const CROP: u32 = 1024;
fn sha(s: &str) -> String {
    Sha256::digest(s.as_bytes()).iter().map(|x| format!("{x:02x}")).collect()
}
fn group_of(name: &str) -> String {
    name.split('_').next().unwrap().trim_end_matches(|c: char| c.is_ascii_digit()).to_string()
}
fn rederive(names: &[String]) -> (Vec<String>, Vec<String>, Vec<String>) {
    let mut groups: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for n in names {
        groups.entry(group_of(n)).or_default().push(n.clone());
    }
    let mut order: Vec<(String, Vec<String>)> = groups.into_iter().collect();
    order.sort_by_key(|(g, _)| sha(&format!("group:{g}")));
    let (mut test, mut val, mut train) = (vec![], vec![], vec![]);
    for (_, ns) in order {
        if test.len() < 15 {
            test.extend(ns);
        } else if val.len() < 10 {
            val.extend(ns);
        } else {
            train.extend(ns);
        }
    }
    train.sort_by_key(|n| sha(&format!("img:{n}")));
    (train, val, test)
}

/// Encoder that records the exact cost (bits) of every binary decision, in coding order.
struct CostIo {
    inner: BinEnc,
    costs: Vec<f32>,
}
impl BitIo for CostIo {
    fn code(&mut self, p1: u32, bit: u32) -> u32 {
        let p = if bit == 1 { p1 } else { 65536 - p1 };
        self.costs.push(-((p as f64 / 65536.0).log2()) as f32);
        self.inner.code(p1, bit)
    }
}
impl RelIo for CostIo {}

/// Number of binary decisions code_sym spends on zigzag symbol z (mirrors pkonrel::Coder::code_sym).
fn n_decisions(z: u8) -> usize {
    let m = (z as u32 + 1) >> 1;
    let c = if m == 0 { 0 } else { 32 - m.leading_zeros() as usize };
    let class = if c < 8 { c + 1 } else { 8 };
    let mant = if (2..8).contains(&c) { c - 1 } else { 0 };
    class + mant + (m != 0) as usize
}

fn med_resid(p: &[u8], w: usize, h: usize) -> Vec<i8> {
    let mut r = vec![0i8; w * h];
    for y in 0..h {
        for x in 0..w {
            let pred = predict(Pred::Med, neighbors(p, w, x, y));
            r[y * w + x] = p[y * w + x].wrapping_sub(pred as u8) as i8;
        }
    }
    r
}

fn per_pixel_cost(sub: &nanoscope::pkon2::Planes, arm: &Arm) -> (Vec<Vec<f32>>, usize) {
    let (w, h, np) = (sub.w, sub.h, sub.p.len());
    let mut work = nanoscope::pkon2::Planes { w, h, p: sub.p.clone() };
    let mut coder = Coder::new(np, arm);
    let mut io = CostIo { inner: BinEnc::new(), costs: vec![] };
    run_rel(&mut work, arm, &mut coder, &mut io, None);
    let CostIo { inner, costs } = io;
    let stream = inner.finish();
    let mut out = vec![vec![0f32; w * h]; np];
    let mut at = 0usize;
    for k in 0..np {
        let med = med_resid(&sub.p[k], w, h);
        for i in 0..w * h {
            let n = n_decisions(zigzag(med[i] as u8));
            out[k][i] = costs[at..at + n].iter().sum();
            at += n;
        }
    }
    assert_eq!(at, costs.len(), "decision count mismatch");
    (out, stream.len())
}

fn wr<T: Copy>(path: &Path, planes: &[Vec<T>], f: impl Fn(T, &mut Vec<u8>)) {
    let mut b = vec![];
    for p in planes {
        for &v in p {
            f(v, &mut b);
        }
    }
    std::fs::write(path, b).unwrap();
}

fn main() {
    let a: Vec<String> = std::env::args().collect();
    let raw = PathBuf::from(a.iter().position(|x| x == "--raw").map(|i| a[i + 1].clone()).expect("--raw"));
    let sj: serde_json::Value = serde_json::from_slice(&std::fs::read("results/learn1-splits.json").unwrap()).unwrap();
    let get = |k: &str| sj[k].as_array().unwrap().iter().map(|s| s.as_str().unwrap().to_string()).collect::<Vec<_>>();
    let (dev, val, test) = (get("train_pool"), get("val"), get("test"));
    let man: Vec<serde_json::Value> = serde_json::from_str(&std::fs::read_to_string("data/dataset50.json").unwrap()).unwrap();
    let all: Vec<String> = man.iter().map(|m| m["name"].as_str().unwrap().to_string()).collect();
    let (rd, rv, rt) = rederive(&all);
    assert!(rd == dev && rv == val && rt == test, "split file does not match its derivation rule");
    assert_eq!((dev.len(), val.len(), test.len()), (25, 10, 15));
    let m: serde_json::Value = serde_json::from_str(&std::fs::read_to_string("results/20261027-155433Z-bench50/dataset_manifest.json").unwrap()).unwrap();
    let crop_hash: BTreeMap<String, String> = m["images"].as_array().unwrap().iter().map(|i| (i["name"].as_str().unwrap().to_string(), i["crop_sha256"].as_str().unwrap().to_string())).collect();
    let arm_p = Arm { name: "X4P", extras: &[Extra::P] };
    let arm_x = Arm { name: "X4", extras: &[] };
    for (names, set) in [(&dev, "DEV"), (&val, "VAL")] {
        for n in names {
            let me = man.iter().find(|m| m["name"].as_str() == Some(n)).unwrap();
            let full = Canonical::load(&Path::new("data").join(me["file"].as_str().unwrap())).unwrap();
            let crop = center_crop(&full, CROP);
            assert_eq!(crop.content_hash(), crop_hash[n], "crop hash mismatch for {n}");
            assert_eq!(crop.spec.layout, Layout::Rgb);
            let (sub, orig) = prepared(&crop);
            let (w, h) = (sub.w, sub.h);
            let d = raw.join(n);
            std::fs::create_dir_all(&d).unwrap();
            image::save_buffer(d.join("rgb.png"), &crop.data, w as u32, h as u32, image::ColorType::Rgb8).unwrap();
            let ms: Vec<Vec<i8>> = sub.p.iter().map(|p| med_resid(p, w, h)).collect();
            let mr: Vec<Vec<i8>> = orig.iter().map(|p| med_resid(p, w, h)).collect();
            let pc: Vec<Vec<u8>> = sub
                .p
                .iter()
                .map(|p| {
                    (0..w * h)
                        .map(|i| {
                            let (x, y) = (i % w, i / w);
                            let nb = neighbors(p, w, x, y);
                            p_class(p, w, x, y, nb, predict(Pred::Med, nb)) as u8
                        })
                        .collect()
                })
                .collect();
            let (cp, lp) = per_pixel_cost(&sub, &arm_p);
            let (cx, lx) = per_pixel_cost(&sub, &arm_x);
            let prod_p = encode_payload_codec(&crop, CODEC_X4P).len();
            let prod_x = encode_payload_codec(&crop, CODEC_X4).len();
            assert_eq!(lp, prod_p, "research X4P stream != production codec 2 payload for {n}");
            assert_eq!(lx, prod_x, "research X4 stream != production codec 1 payload for {n}");
            wr(&d.join("med_sub.i8"), &ms, |v, b| b.push(v as u8));
            wr(&d.join("med_rgb.i8"), &mr, |v, b| b.push(v as u8));
            wr(&d.join("pcls.u8"), &pc, |v, b| b.push(v));
            wr(&d.join("cost_x4p.f32"), &cp, |v, b| b.extend_from_slice(&v.to_le_bytes()));
            wr(&d.join("cost_x4.f32"), &cx, |v, b| b.extend_from_slice(&v.to_le_bytes()));
            let tot = |c: &Vec<Vec<f32>>| c.iter().map(|p| p.iter().map(|&v| v as f64).sum::<f64>()).sum::<f64>() / 8.0;
            let meta = serde_json::json!({"name": n, "set": set, "w": w, "h": h, "crop_sha256": crop.content_hash(),
                "x4p_stream_bytes": lp, "x4_stream_bytes": lx, "x4p_cost_sum_bytes": tot(&cp), "x4_cost_sum_bytes": tot(&cx),
                "production_x4p_payload": prod_p, "production_x4_payload": prod_x});
            std::fs::write(d.join("meta.json"), serde_json::to_string_pretty(&meta).unwrap()).unwrap();
            println!("{set} {n}: x4p {lp} (prod {prod_p}) x4 {lx} (prod {prod_x}) costsum {:.0}/{:.0}", tot(&cp), tot(&cx));
        }
    }
}
