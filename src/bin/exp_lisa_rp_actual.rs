//! Experiment runner: actual range-coded size of RP1 vs production Stage 4, with lossless verification.
//! Usage: exp_lisa_rp_actual <dataset_manifest.json> <out.json> <id> [<id>...]
use nanoscope::lisa::*;
use nanoscope::lisa_rp::*;
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

/// Best-of-`reps` wall time in seconds, plus the last result.
fn timed<T>(reps: usize, mut f: impl FnMut() -> T) -> (T, f64) {
    let mut best = f64::MAX;
    let mut out = None;
    for _ in 0..reps {
        let t = Instant::now();
        out = Some(f());
        best = best.min(t.elapsed().as_secs_f64());
    }
    (out.unwrap(), best)
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let manifest: Value = serde_json::from_slice(&std::fs::read(&args[1]).unwrap()).unwrap();
    let (mut tot_p, mut tot_r, mut tot_n) = (0usize, 0usize, 0usize);
    let mut out = Vec::new();
    for id in &args[3..] {
        let m = manifest.as_array().unwrap().iter().find(|m| m["id"].as_str() == Some(id)).expect("id not in manifest");
        let (w, h) = (m["width"].as_u64().unwrap() as usize, m["height"].as_u64().unwrap() as usize);
        let bd = m["bit_depth"].as_u64().unwrap() as u8;
        let bytes = std::fs::read(m["canon_path"].as_str().unwrap()).unwrap();
        let samples: Vec<u16> = bytes.chunks_exact(2).map(|c| u16::from_le_bytes([c[0], c[1]])).collect();
        let n = samples.len();
        let s1 = encode_with_metadata(&samples, w, h, bd, meta_of(m, w, h)).unwrap();
        let s3 = classify_stage3_with(organize_stage2(s1).unwrap(), Stage3Rule::Legacy4).unwrap();

        // Production: real encode_stage4 / decode_stage4, then the full inverse back to samples.
        let (s4, prod_enc_t) = timed(3, || encode_stage4(s3.clone()).unwrap());
        let (d3, prod_dec_t) = timed(3, || decode_stage4(&s4).unwrap());
        let (s1b, _) = restore_stage1(&d3.into_stage2()).unwrap();
        let prod_ok = decode_with_metadata(&s1b).unwrap() == samples;
        let prod_bytes = s4.payload().len();

        // Control: production estimator through the experimental generic path must reproduce production bytes.
        let ctrl = encode_actual_base(&s3);
        let ctrl_same = ctrl == s4.payload() && decode_actual_base(&ctrl, &s3) == *s3.data();

        // RP1: actual coding. Decode must reproduce the Stage 3 residuals exactly; the rest of the inverse chain is
        // the unchanged production path verified above.
        let (rp, rp_enc_t) = timed(3, || encode_actual_rp(&s3));
        let (rp_dec, rp_dec_t) = timed(3, || decode_actual_rp(&rp, &s3));
        let rp_ok = rp_dec == *s3.data();

        // Integrated path (lisa.rs): explicit selection, bytes must equal the experiment; full pixel round trip.
        let default_same = encode_stage4_with(s3.clone(), Stage4Estimator::Counted).unwrap() == s4;
        let i4 = encode_stage4_with(s3.clone(), Stage4Estimator::RewardPenalty).unwrap();
        let int_same = i4.payload() == &rp[..];
        let i3 = decode_stage4(&i4).unwrap();
        let int_res_ok = i3.data() == s3.data();
        let (i1, _) = restore_stage1(&i3.into_stage2()).unwrap();
        let int_pixels_ok = decode_with_metadata(&i1).unwrap() == samples;
        println!("{id}: integrated: counted_default_identical={default_same} rp1_bytes_match_experiment={int_same} residuals_ok={int_res_ok} pixels_ok={int_pixels_ok}");

        let (ideal_b, ideal_c, _) = compare(&s3, n);
        let pct = 100.0 * (rp.len() as f64 - prod_bytes as f64) / prod_bytes as f64;
        let ideal_pct = 100.0 * (ideal_c - ideal_b) / ideal_b;
        let (pb, rb) = (prod_bytes as f64 * 8.0 / n as f64, rp.len() as f64 * 8.0 / n as f64);
        println!(
            "{id}: n={n} prod {prod_bytes} B ({pb:.5} bps) rp1 {} B ({rb:.5} bps) change {pct:+.4}% | ideal change {ideal_pct:+.4}% | \
             lossless prod={prod_ok} rp1={rp_ok} control_identical={ctrl_same} | enc s prod {prod_enc_t:.3} rp1 {rp_enc_t:.3} dec s prod {prod_dec_t:.3} rp1 {rp_dec_t:.3}",
            rp.len()
        );
        tot_p += prod_bytes;
        tot_r += rp.len();
        tot_n += n;
        out.push(json!({"id": id, "n": n, "prod_bytes": prod_bytes, "rp1_bytes": rp.len(), "pct_change": pct,
            "prod_bps": pb, "rp1_bps": rb, "ideal_prod_bps": ideal_b / n as f64, "ideal_rp1_bps": ideal_c / n as f64, "ideal_pct_change": ideal_pct,
            "prod_lossless": prod_ok, "rp1_lossless": rp_ok, "control_identical": ctrl_same,
            "enc_s": {"prod": prod_enc_t, "rp1": rp_enc_t}, "dec_s": {"prod": prod_dec_t, "rp1": rp_dec_t}}));
    }
    let agg = 100.0 * (tot_r as f64 - tot_p as f64) / tot_p as f64;
    println!("aggregate: prod {tot_p} B rp1 {tot_r} B change {agg:+.4}% ({:.5} -> {:.5} bps)", tot_p as f64 * 8.0 / tot_n as f64, tot_r as f64 * 8.0 / tot_n as f64);
    std::fs::write(&args[2], serde_json::to_vec_pretty(&json!({"images": out, "agg_prod_bytes": tot_p, "agg_rp1_bytes": tot_r, "agg_pct_change": agg})).unwrap()).unwrap();
}
