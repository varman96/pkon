//! Experiment runner: PRAW probability-cloud model (src/praw_cloud.rs) on a fixed list of images from the frozen manifest.
//! Usage: exp_praw_cloud <dataset_manifest.json> <out.json> <id> [<id>...]
use nanoscope::praw::*;
use nanoscope::praw_cloud::*;
use serde_json::{json, Value};
use std::time::Instant;

fn meta_of(m: &Value, w: usize, h: usize) -> ImageMetadata {
    let Some(cfa) = m["cfa"].as_array().filter(|a| !a.is_empty()) else {
        return ImageMetadata::without_cfa(w, h, SampleLayout::Monochrome);
    };
    let dim: Vec<usize> = m["cfa_dim"].as_array().unwrap().iter().map(|v| v.as_u64().unwrap() as usize).collect();
    let org: Vec<usize> = m["cfa_origin"].as_array().unwrap().iter().map(|v| v.as_u64().unwrap() as usize).collect();
    let entries: Vec<ColorFilter> = cfa.iter().map(|v| ColorFilter::try_from(v.as_u64().unwrap() as u8).unwrap()).collect();
    let pat = CfaPattern { width: dim[1], height: dim[0], entries, origin_x: org[0], origin_y: org[1] };
    let bayer = ImageMetadata::with_cfa(w, h, SampleLayout::BayerCfa, pat.clone());
    if (dim[0], dim[1]) == (2, 2) && bayer.validate().is_ok() { bayer } else { ImageMetadata::with_cfa(w, h, SampleLayout::OtherCfa, pat) }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let manifest: Vec<Value> = serde_json::from_slice(&std::fs::read(&args[1]).unwrap()).unwrap();
    let mut out = Vec::new();
    for (ii, id) in args[3..].iter().enumerate() {
        let m = manifest.iter().find(|m| m["id"].as_str() == Some(id)).expect("id not in manifest");
        let (w, h) = (m["width"].as_u64().unwrap() as usize, m["height"].as_u64().unwrap() as usize);
        let bd = m["bit_depth"].as_u64().unwrap() as u8;
        let bytes = std::fs::read(m["canon_path"].as_str().unwrap()).unwrap();
        let samples: Vec<u16> = bytes.chunks_exact(2).map(|c| u16::from_le_bytes([c[0], c[1]])).collect();
        let meta = meta_of(m, w, h);

        let t = Instant::now();
        let s1 = encode_with_metadata(&samples, w, h, bd, meta.clone()).unwrap();
        let s2 = organize_stage2(s1).unwrap();
        let t_pre = t.elapsed().as_secs_f64();
        let t = Instant::now();
        let (stream, hdr, stats) = encode_cloud(&s2).unwrap();
        let t_enc = t.elapsed().as_secs_f64();

        let t = Instant::now();
        let s1b = decode_cloud(&stream, &meta, bd).unwrap();
        let dec = decode_with_metadata(&s1b).unwrap();
        let t_dec = t.elapsed().as_secs_f64();
        let exact = dec == samples && s1b.metadata == meta;

        // Malformed-input rejection (first image only).
        let mut rejects = Vec::new();
        if ii == 0 {
            let plen = stream.len() - hdr;
            let mut c = stream.clone(); c.truncate(stream.len() - 1);
            rejects.push(("payload_truncated_1_byte_header_unchanged", decode_cloud(&c, &meta, bd).is_err()));
            let mut c = stream.clone(); c.truncate(hdr + plen / 2); c[18..26].copy_from_slice(&((plen / 2) as u64).to_le_bytes());
            rejects.push(("payload_halved_header_fixed_up", decode_cloud(&c, &meta, bd).is_err()));
            let mut c = stream.clone(); c[0] ^= 0xFF;
            rejects.push(("bad_magic", decode_cloud(&c, &meta, bd).is_err()));
            let mut c = stream.clone(); c[5] = 2;
            rejects.push(("bad_mode", decode_cloud(&c, &meta, bd).is_err()));
            let mut c = stream.clone(); c[10] ^= 1;
            rejects.push(("bad_count", decode_cloud(&c, &meta, bd).is_err()));
            rejects.push(("shorter_than_header", decode_cloud(&stream[..10], &meta, bd).is_err()));
            let mut c = stream.clone(); c.push(0);
            rejects.push(("trailing_byte", decode_cloud(&c, &meta, bd).is_err()));
        }
        let payload = stream.len() - hdr;
        println!("{id}: payload {payload} B hdr {hdr} B total {} exact={exact} enc {t_enc:.1}s dec {t_dec:.1}s esc {} ({:.4}%) ideal {:.0} B tree + {:.0} B tail rejects={:?}",
            stream.len(), stats.escapes, 100.0 * stats.escapes as f64 / stats.n as f64, stats.tree_bits / 8.0, stats.tail_bits / 8.0, rejects);
        out.push(json!({"id": id, "width": w, "height": h, "bit_depth": bd, "n": stats.n,
            "payload_bytes": payload, "overhead_bytes": hdr, "total_bytes": stream.len(), "exact": exact,
            "escapes": stats.escapes, "ideal_tree_bytes": stats.tree_bits / 8.0, "ideal_tail_bytes": stats.tail_bits / 8.0,
            "time_s": {"stage1_2": t_pre, "encode": t_enc, "decode_incl_stage1_inverse": t_dec},
            "malformed_rejected": rejects.iter().map(|(k, v)| json!({"case": k, "rejected": v})).collect::<Vec<_>>()}));
    }
    std::fs::write(&args[2], serde_json::to_vec_pretty(&out).unwrap()).unwrap();
}
