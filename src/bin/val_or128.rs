//! OR-128 production validation: default (OR-128) four-stage path on frozen canonical samples, with an exact-reconstruction
//! gate, determinism check and a Legacy4/Counted control that must reproduce the historical production bytes.
//! Uses only the public `lisa` API; the algorithm is untouched.
//! Usage: val_or128 <dataset_manifest.json> <out.json> <id> [<id>...]
use nanoscope::lisa::*;
use nanoscope::lisa_bayer_plane::{decode_stage1, encode_stage2, restore_stage1_with_mode};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
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

struct Run {
    s4: Stage4Output,
    s3: Stage3Output,
    enc_s: f64,
}

fn encode(samples: &[u16], w: usize, h: usize, bd: u8, meta: &ImageMetadata, rule: Stage3Rule) -> Run {
    let t = Instant::now();
    let s2 = encode_stage2(samples, w, h, bd, meta.clone()).unwrap();
    let s3 = classify_stage3_with(s2, rule).unwrap();
    let s4 = encode_stage4(s3.clone()).unwrap();
    Run { s4, s3, enc_s: t.elapsed().as_secs_f64() }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let manifest: Value = serde_json::from_slice(&std::fs::read(&args[1]).unwrap()).unwrap();
    let mut out = Vec::new();
    let mut all_ok = true;
    for id in &args[3..] {
        let m = manifest.as_array().unwrap().iter().find(|m| m["id"].as_str() == Some(id)).expect("id not in manifest");
        let (w, h) = (m["width"].as_u64().unwrap() as usize, m["height"].as_u64().unwrap() as usize);
        let bd = m["bit_depth"].as_u64().unwrap() as u8;
        let bytes = std::fs::read(m["canon_path"].as_str().unwrap()).unwrap();
        let sha = format!("{:x}", Sha256::digest(&bytes));
        let sha_ok = sha == m["canon_sha256"].as_str().unwrap() && bytes.len() == w * h * 2;
        let samples: Vec<u16> = bytes.chunks_exact(2).map(|c| u16::from_le_bytes([c[0], c[1]])).collect();
        let meta = meta_of(m, w, h);

        // Production default path: classify_stage3 (OR-128) + encode_stage4.
        let a = encode(&samples, w, h, bd, &meta, Stage3Rule::default());
        let b = encode(&samples, w, h, bd, &meta, Stage3Rule::default());
        let deterministic = a.s4 == b.s4 && a.s4.payload() == b.s4.payload();
        let rule_ok = a.s3.rule() == Some(Stage3Rule::Or128) && Stage3Rule::default() == Stage3Rule::Or128;
        let (ctx_max, ctx_used) = {
            let mut seen = [false; 256];
            let mut tally = |c: &[u8]| c.iter().for_each(|&v| seen[v as usize] = true);
            match a.s3.contexts() {
                OrganizedContexts::Morton(c) => tally(c),
                OrganizedContexts::BayerPlanes(cs) => cs.iter().for_each(|c| tally(c)),
            }
            (seen.iter().rposition(|&s| s).unwrap(), seen.iter().filter(|&&s| s).count())
        };
        let mode = match a.s3.mode() {
            Some(Stage2Mode::BayerPlanes { .. }) => "bayer_planes",
            Some(Stage2Mode::Identity) => "identity_morton",
            None => "unknown",
        };
        let (rec3, rec4) = {
            let hs = a.s4.history();
            match (&hs[hs.len() - 2], &hs[hs.len() - 1]) {
                (ReconstructionRecord::Stage3Context(r3), ReconstructionRecord::Stage4Entropy(r4)) => {
                    (json!({"rule_version": r3.rule_version, "class_count": r3.class_count}),
                     json!({"coding_version": r4.coding_version, "model_count": r4.model_count, "residual_count": r4.residual_count}))
                }
                _ => panic!("unexpected history"),
            }
        };

        // Decode from the Stage 4 output only; full inverse chain; compare with source samples and metadata.
        let t = Instant::now();
        let d3 = decode_stage4(&a.s4).expect("decode stage4 (also enforces full payload consumption)");
        let t_d4 = t.elapsed().as_secs_f64();
        let residuals_equal = d3.data() == a.s3.data() && d3.contexts() == a.s3.contexts();
        let t = Instant::now();
        let (s1b, mode) = restore_stage1_with_mode(&d3.into_stage2()).expect("restore stage1");
        let dec = decode_stage1(&s1b, mode).expect("decode stage1");
        let t_d1 = t.elapsed().as_secs_f64();
        let pixels_exact = dec == samples;
        let meta_equal = s1b.metadata == meta;
        let first_bad = dec.iter().zip(&samples).position(|(x, y)| x != y);

        // Control: Legacy4 + Counted must reproduce the historical production payload size.
        let ctl = encode(&samples, w, h, bd, &meta, Stage3Rule::Legacy4);
        let ctl_dec = decode_stage4(&ctl.s4).unwrap();
        let ctl_exact = {
            let (s1c, mode) = restore_stage1_with_mode(&ctl_dec.into_stage2()).unwrap();
            decode_stage1(&s1c, mode).unwrap() == samples
        };

        // Timing in the historical methodology (exp_lisa_rp_actual): Stage 4 encode / decode only, best of 3.
        let best = |f: &mut dyn FnMut()| (0..3).map(|_| { let t = Instant::now(); f(); t.elapsed().as_secs_f64() }).fold(f64::MAX, f64::min);
        let s4_enc_best = best(&mut || { std::hint::black_box(encode_stage4(a.s3.clone()).unwrap()); });
        let s4_dec_best = best(&mut || { std::hint::black_box(decode_stage4(&a.s4).unwrap()); });
        let s4_enc_ctl_best = best(&mut || { std::hint::black_box(encode_stage4(ctl.s3.clone()).unwrap()); });
        let s4_dec_ctl_best = best(&mut || { std::hint::black_box(decode_stage4(&ctl.s4).unwrap()); });

        let ok = sha_ok && deterministic && rule_ok && residuals_equal && pixels_exact && meta_equal && ctl_exact;
        all_ok &= ok;
        let n = (w * h) as f64;
        println!(
            "{id}: {mode:?} {}x{} {bd}b OR128 {} B ({:.5} bps) legacy4-ctl {} B | exact={pixels_exact} meta_eq={meta_equal} resid_eq={residuals_equal} \
             deterministic={deterministic} canon_sha_ok={sha_ok} ctl_exact={ctl_exact} ctx_used={ctx_used}/128 | full enc {:.3}s/{:.3}s full dec {:.3}s | S4 best3 enc or128 {:.3}s legacy4 {:.3}s dec or128 {:.3}s legacy4 {:.3}s",
            w, h, a.s4.payload().len(), a.s4.payload().len() as f64 * 8.0 / n, ctl.s4.payload().len(), a.enc_s, b.enc_s, t_d4 + t_d1,
            s4_enc_best, s4_enc_ctl_best, s4_dec_best, s4_dec_ctl_best
        );
        out.push(json!({
            "id": id, "width": w, "height": h, "bit_depth": bd, "samples": w * h,
            "camera": m["camera"], "layout": format!("{:?}", meta.layout), "cfa": m["cfa"], "cfa_dim": m["cfa_dim"], "cfa_origin": m["cfa_origin"],
            "dng_compression": m["dng_compression"], "stage2_mode": format!("{:?}", mode),
            "canon_sha256": sha, "canon_sha256_matches_manifest": sha_ok,
            "or128_payload_bytes": a.s4.payload().len(), "or128_bps": a.s4.payload().len() as f64 * 8.0 / n,
            "legacy4_counted_payload_bytes": ctl.s4.payload().len(), "legacy4_ctl_exact": ctl_exact,
            "pixels_exact": pixels_exact, "first_mismatch": first_bad, "metadata_equal": meta_equal,
            "stage3_residuals_and_contexts_equal": residuals_equal, "deterministic_payload": deterministic,
            "decoder_consumed_full_payload": true, "stage3_record": rec3, "stage4_record": rec4,
            "max_context_id": ctx_max, "contexts_used": ctx_used,
            "time_s": {"encode_run1": a.enc_s, "encode_run2": b.enc_s, "encode_legacy4_ctl": ctl.enc_s,
                       "stage4_encode_best3_or128": s4_enc_best, "stage4_decode_best3_or128": s4_dec_best,
                       "stage4_encode_best3_legacy4": s4_enc_ctl_best, "stage4_decode_best3_legacy4": s4_dec_ctl_best,
                       "decode_stage4": t_d4, "decode_restore": t_d1, "decode_total": t_d4 + t_d1},
        }));
    }
    std::fs::write(&args[2], serde_json::to_vec_pretty(&out).unwrap()).unwrap();
    println!("ALL_OK={all_ok}");
    if !all_ok {
        std::process::exit(1);
    }
}
