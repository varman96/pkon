//! Experiment runner: LISA adaptive probability mixer (src/praw_mixer.rs) on a fixed list of images from the frozen manifest.
//! Usage: exp_praw_mixer <dataset_manifest.json> <praw_results.json> <baseline_results.json> <cloud_results.json> <out.json> <id> [<id>...]
//! Baseline byte counts are read from the frozen artifacts, never recomputed.
use nanoscope::praw::*;
use nanoscope::praw_mixer::*;
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

fn load(p: &str) -> Value {
    serde_json::from_slice(&std::fs::read(p).unwrap_or_else(|e| panic!("cannot read {p}: {e}"))).unwrap()
}
fn by_id<'a>(v: &'a Value, id: &str) -> &'a Value {
    v.as_array().unwrap().iter().find(|e| e["id"].as_str() == Some(id)).unwrap_or_else(|| panic!("{id} missing in baseline artifact"))
}
fn pct(a: f64, b: f64) -> f64 {
    100.0 * (a - b) / b
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let manifest = load(&args[1]);
    let praw_r = load(&args[2]);
    let base_r = load(&args[3]);
    let cloud_r = load(&args[4]);
    let mut out = Vec::new();
    let (mut tot_mix, mut tot_praw, mut tot_cloud, mut tot_jxl) = (0u64, 0u64, 0u64, 0u64);
    for (ii, id) in args[6..].iter().enumerate() {
        let m = manifest.as_array().unwrap().iter().find(|m| m["id"].as_str() == Some(id)).expect("id not in manifest");
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
        let (stream, hdr, st) = encode_mixer(&s2).unwrap();
        let t_enc = t.elapsed().as_secs_f64();

        let t = Instant::now();
        let s1b = decode_mixer(&stream, &meta, bd).unwrap();
        let t_dec_mix = t.elapsed().as_secs_f64();
        let t = Instant::now();
        let dec = decode_with_metadata(&s1b).unwrap();
        let t_inv = t.elapsed().as_secs_f64();
        let exact = dec == samples && s1b.metadata == meta;

        let mut rejects = Vec::new();
        if ii == 0 {
            let plen = stream.len() - hdr;
            let mut c = stream.clone();
            c.truncate(stream.len() - 1);
            rejects.push(("payload_truncated_1_byte_header_unchanged", decode_mixer(&c, &meta, bd).is_err()));
            let mut c = stream.clone();
            c.truncate(hdr + plen / 2);
            c[18..26].copy_from_slice(&((plen / 2) as u64).to_le_bytes());
            rejects.push(("payload_halved_header_fixed_up", decode_mixer(&c, &meta, bd).is_err()));
            let mut c = stream.clone();
            c[0] ^= 0xFF;
            rejects.push(("bad_magic", decode_mixer(&c, &meta, bd).is_err()));
            let mut c = stream.clone();
            c[5] = 2;
            rejects.push(("bad_mode", decode_mixer(&c, &meta, bd).is_err()));
            let mut c = stream.clone();
            c[6] ^= 1;
            rejects.push(("bad_eta", decode_mixer(&c, &meta, bd).is_err()));
            let mut c = stream.clone();
            c[10] ^= 1;
            rejects.push(("bad_count", decode_mixer(&c, &meta, bd).is_err()));
            rejects.push(("shorter_than_header", decode_mixer(&stream[..10], &meta, bd).is_err()));
            let mut c = stream.clone();
            c.push(0);
            rejects.push(("trailing_byte", decode_mixer(&c, &meta, bd).is_err()));
        }

        let payload = (stream.len() - hdr) as u64;
        let total = stream.len() as u64;
        let praw_b = by_id(&praw_r, id)["payload_bytes"].as_u64().unwrap();
        let cloud = by_id(&cloud_r, id);
        let (cloud_p, cloud_t) = (cloud["payload_bytes"].as_u64().unwrap(), cloud["total_bytes"].as_u64().unwrap());
        let jxl = base_r["images"][id.as_str()]["jxl"]["bytes"].as_u64().unwrap();
        tot_mix += total;
        tot_praw += praw_b;
        tot_cloud += cloud_t;
        tot_jxl += jxl;
        let n = st.n as f64;
        let bps = |b: u64| b as f64 * 8.0 / n;
        println!(
            "{id}: payload {payload} B hdr {hdr} B total {total} exact={exact} | LISA {praw_b} ({:+.3}% payload-vs-payload) cloud {cloud_t} ({:+.3}% total-vs-total) JXL {jxl} ({:+.3}% total-vs-file) | bps mix {:.4} praw {:.4} | ideal {:.0} B | enc {t_enc:.1}s dec {t_dec_mix:.1}s w={:?} rejects={:?}",
            pct(payload as f64, praw_b as f64),
            pct(total as f64, cloud_t as f64),
            pct(total as f64, jxl as f64),
            bps(payload),
            bps(praw_b),
            st.mix_bits / 8.0,
            st.final_weights.map(|x| (x * 1e4).round() / 1e4),
            rejects
        );
        out.push(json!({"id": id, "width": w, "height": h, "bit_depth": bd, "n": st.n, "exact": exact,
            "mixer": {"payload_bytes": payload, "overhead_bytes": hdr, "total_bytes": total},
            "baselines": {"praw_payload_bytes": praw_b, "cloud_payload_bytes": cloud_p, "cloud_total_bytes": cloud_t, "jxl_file_bytes": jxl},
            "pct": {"mixer_payload_vs_praw_payload": pct(payload as f64, praw_b as f64),
                    "mixer_total_vs_cloud_total": pct(total as f64, cloud_t as f64),
                    "mixer_payload_vs_cloud_payload": pct(payload as f64, cloud_p as f64),
                    "mixer_total_vs_jxl_file": pct(total as f64, jxl as f64)},
            "bits_per_sample": {"mixer_payload": bps(payload), "praw_payload": bps(praw_b), "cloud_payload": bps(cloud_p)},
            "ideal": {"mixture_bytes": st.mix_bits / 8.0, "model_bytes": st.model_bits.map(|b| b / 8.0), "model_names": MODEL_NAMES,
                      "payload_minus_mixture_ideal_bytes": payload as f64 - st.mix_bits / 8.0,
                      "big_residuals_ge_1024": {"count": st.big_n, "fraction": st.big_n as f64 / n, "ideal_bytes": st.big_bits / 8.0, "share_of_mixture": st.big_bits / st.mix_bits}},
            "final_weights": st.final_weights,
            "segments": st.segments.iter().map(|s| json!({"n": s.n, "mixture_bps": s.mix_bits / s.n as f64,
                "model_bps": s.model_bits.map(|b| b / s.n as f64), "mean_weights": s.w_sum.map(|x| x / s.n as f64)})).collect::<Vec<_>>(),
            "time_s": {"stage1_2": t_pre, "mixer_encode": t_enc, "mixer_decode": t_dec_mix, "stage1_inverse": t_inv},
            "malformed_rejected": rejects.iter().map(|(k, v)| json!({"case": k, "rejected": v})).collect::<Vec<_>>()}));
    }
    println!(
        "AGGREGATE (3 images): mixer total {tot_mix} | LISA payload {tot_praw} ({:+.3}%, mixer total vs LISA payload) | cloud total {tot_cloud} ({:+.3}%) | JXL files {tot_jxl} ({:+.3}%)",
        pct(tot_mix as f64, tot_praw as f64),
        pct(tot_mix as f64, tot_cloud as f64),
        pct(tot_mix as f64, tot_jxl as f64)
    );
    let agg = json!({"mixer_total": tot_mix, "praw_payload": tot_praw, "cloud_total": tot_cloud, "jxl_files": tot_jxl,
        "pct_vs_praw": pct(tot_mix as f64, tot_praw as f64), "pct_vs_cloud": pct(tot_mix as f64, tot_cloud as f64), "pct_vs_jxl": pct(tot_mix as f64, tot_jxl as f64)});
    std::fs::write(&args[5], serde_json::to_vec_pretty(&json!({"images": out, "aggregate": agg})).unwrap()).unwrap();
}
