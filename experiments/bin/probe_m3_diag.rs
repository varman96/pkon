//! M3 diagnostic (not a candidate): where does GAP lose to MED? Codes the G plane and the SubG
//! chroma-difference plane (R-G+128) of each camera JPEG as standalone gray images, ctx = Act.
use nanoscope::canon::{Canonical, Layout, Spec};
use nanoscope::pkon2::*;

fn gray(w: u32, h: u32, d: Vec<u8>) -> Canonical {
    Canonical::new(Spec { width: w, height: h, layout: Layout::Gray, bit_depth: 8 }, d).unwrap()
}

fn main() {
    let manifest: Vec<serde_json::Value> = serde_json::from_str(&std::fs::read_to_string("data/manifest.json").unwrap()).unwrap();
    let (mut tot, mut n) = ([[0f64; 2]; 3], 0);
    println!("| image | G med | G gap | R-G med | R-G gap | B-G med | B-G gap |\n|---|---:|---:|---:|---:|---:|---:|");
    for m in manifest.iter().filter(|m| m["source_format"].as_str().unwrap().contains("jpeg")) {
        let img = center_crop(&Canonical::load(&std::path::Path::new("data").join(m["file"].as_str().unwrap())).unwrap(), 1024);
        let pl = to_planes(&img); // [G,R,B]
        let diff = |k: usize| -> Vec<u8> { (0..pl.w * pl.h).map(|i| pl.p[k][i].wrapping_sub(pl.p[0][i]).wrapping_add(128)).collect() };
        let planes = [pl.p[0].clone(), diff(1), diff(2)];
        print!("| {} |", m["name"].as_str().unwrap());
        for (j, p) in planes.iter().enumerate() {
            let g = gray(pl.w as u32, pl.h as u32, p.clone());
            for (q, pred) in [Pred::Med, Pred::Gap].iter().enumerate() {
                let o = measure(&g, Config { pred: *pred, color: Color::Raw, ctx: Ctx::Act });
                assert!(o.exact);
                tot[j][q] += o.coded_bytes as f64;
                print!(" {} |", o.coded_bytes);
            }
        }
        println!();
        n += 1;
    }
    for (j, nm) in ["G", "R-G", "B-G"].iter().enumerate() {
        println!("{nm}: MED {:.0} B, GAP {:.0} B, GAP vs MED {:+.2}% (n={n})", tot[j][0], tot[j][1], (tot[j][1] / tot[j][0] - 1.0) * 100.0);
    }
}
