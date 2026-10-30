//! Bit-cost profiler runner (instrumentation only; no compression experiment). Profiles the relational coder `pkonrel` (arm `control` = codec 1 = X4 = the frozen
//! M6 reference; arm `P` = codec 2 = current production default) on the 20 JPEG-origin M6 images (1024-px centre crops, the exp_n3_full corpus).
//!
//!   cargo run --release --bin bitprof -- --out historical/<stamp>-bitprof
use nanoscope::canon::Canonical;
use nanoscope::pkon2::{center_crop, Planes};
use nanoscope::pkon2x::BinEnc;
use nanoscope::pkonfile::{self, HEADER_LEN};
use nanoscope::pkonlearn::prepared;
use nanoscope::pkonrel::*;
use nanoscope::profile::*;
use std::fmt::Write as _;
use std::path::{Path, PathBuf};

fn prof_encode(sub: &Planes, arm: &Arm) -> (Vec<u8>, ProfData, f64, u64, u64) {
    let mut work = Planes { w: sub.w, h: sub.h, p: sub.p.clone() };
    let mut coder = Coder::new(work.p.len(), arm);
    let mut io = ProfEnc::new();
    let t = std::time::Instant::now();
    let trace = run_rel(&mut work, arm, &mut coder, &mut io, None);
    let (stream, data) = io.finish();
    let ms = t.elapsed().as_secs_f64() * 1e3;
    (stream, data, ms, trace, coder.state_hash())
}

fn slot_name(node: usize, slot: usize) -> &'static str {
    if node == SIGN {
        ["S0 p*4+cls", "S1 w,n,ne", "S2 p,w,n", "-", "P", "-"][slot]
    } else {
        ["A", "B", "C", "D", "P", "-"][slot]
    }
}
fn grp_name(g: usize) -> &'static str {
    CATP_NAMES[g]
}
const PLANES: [&str; NP] = ["G", "R-G", "B-G"];

struct ImgRes {
    name: String,
    arm: &'static str,
    file_bytes: usize,
    payload: usize,
    model_bits: f64,
    cat: Vec<([f64; NCATP], [u64; NCATP])>,
    enc_plain_ms: f64,
    enc_prof_ms: f64,
}

fn main() {
    let a: Vec<String> = std::env::args().collect();
    let out = PathBuf::from(a.iter().position(|x| x == "--out").map(|i| a[i + 1].clone()).expect("--out"));
    std::fs::create_dir_all(&out).unwrap();
    // ---- corpus: the 20 JPEG-origin M6 images, exactly as exp_n3_full
    let m6: Vec<serde_json::Value> = {
        let mut d: Vec<PathBuf> = std::fs::read_dir("historical").unwrap().filter_map(|e| e.ok()).map(|e| e.path()).filter(|p| p.to_string_lossy().ends_with("-pkon2-m6")).collect();
        d.sort();
        serde_json::from_str(&std::fs::read_to_string(d.pop().unwrap().join("results.json")).unwrap()).unwrap()
    };
    let manifest: Vec<serde_json::Value> = serde_json::from_str(&std::fs::read_to_string("data/manifest.json").unwrap()).unwrap();
    // production X4 (N3-promoted In::Ab) reference bytes per image, from historical/20261027-x4-promotion-verify/report.md (columns: image | src | frozen M6 | production)
    let promo: std::collections::BTreeMap<String, usize> = std::fs::read_to_string("historical/20261027-x4-promotion-verify/report.md")
        .unwrap()
        .lines()
        .filter(|l| l.contains("| jpeg |"))
        .map(|l| {
            let c: Vec<&str> = l.split('|').map(|s| s.trim()).collect();
            (c[1].to_string(), c[4].parse::<usize>().unwrap())
        })
        .collect();
    assert_eq!(promo.len(), 20);
    struct Img {
        name: String,
        canon: Canonical,
        sub: Planes,
        frozen_x4: usize,
        promoted_x4: usize,
    }
    let mut imgs = vec![];
    for rec in m6.iter().filter(|x| x["src"] == "jpeg") {
        let name = rec["name"].as_str().unwrap().to_string();
        let m = manifest.iter().find(|m| m["name"] == name.as_str()).unwrap();
        let canon = center_crop(&Canonical::load(&Path::new("data").join(m["file"].as_str().unwrap())).unwrap(), 1024);
        let (sub, _orig) = prepared(&canon);
        let promoted_x4 = promo[&name];
        imgs.push(Img { name, canon, sub, frozen_x4: rec["runs"][5]["coded"].as_u64().unwrap() as usize, promoted_x4 });
    }
    println!("loaded {} images; profiler state {} B per image, aggregate {} B", imgs.len(), ProfEnc::state_bytes(), Agg::state_bytes());
    let arms: [(&Arm, u8); 2] = [(&ARMS[0], pkonfile::CODEC_X4), (&ARMS[2], pkonfile::CODEC_X4P)];
    let mut problems: Vec<String> = vec![];
    let mut all: Vec<(Agg, Vec<ImgRes>)> = vec![];
    let mut csv_lines: Vec<String> = vec![];
    let mut gap_rows: Vec<(String, &str, Vec<f64>)> = vec![];
    for (arm, codec) in arms {
        let mut agg = Agg::new();
        let mut res: Vec<ImgRes> = vec![];
        for im in &imgs {
            let (w, h) = (im.sub.w, im.sub.h);
            // production output (established path)
            let file = pkonfile::encode_with_codec(&im.canon, codec).unwrap();
            let payload_prod = &file[HEADER_LEN..];
            // profiling disabled: the research copy with the CeEnc sink (also gives the existing per-category cross-entropy)
            let e1 = encode_rel(&im.sub, arm, None);
            let e2 = encode_rel(&im.sub, arm, None);
            // profiling enabled
            let (s1, d1, ms1, trace1, state1) = prof_encode(&im.sub, arm);
            let (s2, _d2, ms2, trace2, _) = prof_encode(&im.sub, arm);
            let mut why = vec![];
            if e1.stream != payload_prod {
                why.push("profiling-disabled stream != production payload");
            }
            if s1 != payload_prod {
                why.push("profiling-enabled stream != production payload");
            }
            if s1 != s2 || trace1 != trace2 {
                why.push("profiled encode not deterministic");
            }
            if trace1 != e1.trace || state1 != e1.state {
                why.push("profiled trace/model state != plain");
            }
            // decode the production FILE with the production decoder, bit-exact
            let dec = pkonfile::decode(&file).unwrap();
            if dec.data != im.canon.data {
                why.push("decode not bit-exact");
            }
            // reconciliation: decision counts and cost totals
            if d1.dropped != 0 {
                why.push("profiler dropped observations");
            }
            if (d1.obs_bits - d1.code_bits).abs() > 1e-6 * d1.code_bits.max(1.0) {
                why.push("observed cost != coded cost");
            }
            let ncount: u64 = d1.node_n.iter().flatten().sum::<u64>() + d1.tail.iter().map(|t| t.n).sum::<u64>();
            if ncount != d1.code_n || d1.obs_n != d1.code_n {
                why.push("decision counts do not reconcile");
            }
            let ce_total: f64 = e1.bits.iter().flatten().sum();
            if (ce_total - d1.code_bits).abs() > 1e-6 * ce_total.max(1.0) {
                why.push("total != CeEnc cross-entropy");
            }
            for p in 0..NP {
                let (cb, cn) = d1.cat(p);
                // CeEnc categories: zero, ge2, class, mantissa (incl. tail), sign
                let ce_b = |c: usize| e1.bits[p][c];
                let ce_n = |c: usize| e1.count[p][c];
                let pairs = [(cb[0], ce_b(0)), (cb[1], ce_b(1)), (cb[2], ce_b(2)), (cb[3] + cb[5], ce_b(3)), (cb[4], ce_b(4))];
                for (x, y) in pairs {
                    if (x - y).abs() > 1e-6 * y.max(1.0) {
                        why.push("per-category cost != CeEnc");
                    }
                }
                if cn[0] != ce_n(0) || cn[1] != ce_n(1) || cn[2] != ce_n(2) || cn[3] + cn[5] != ce_n(3) || cn[4] != ce_n(4) {
                    why.push("per-category counts != CeEnc");
                }
            }
            if *arm.name.as_bytes() == *b"control" && e1.coded_bytes != im.promoted_x4 {
                why.push("X4 control != promotion-verified production X4 reference");
            }
            if !why.is_empty() {
                problems.push(format!("{} / {}: {}", im.name, arm.name, why.join("; ")));
            }
            let _ = (w, h);
            let cat: Vec<_> = (0..NP).map(|p| d1.cat(p)).collect();
            let model_bits = d1.code_bits;
            let one = {
                let mut o = AggExt::new();
                o.add(&d1.ext);
                o
            };
            let gaps: Vec<f64> = (0..NP).map(|p| (d1.node_bits[p][SIGN] - one.sign4[p].kt) / 8.0).chain((0..NP).map(|p| (d1.node_bits[p][0] - one.pair[(p * PAIR_NODES) * 10 + 3].kt) / 8.0)).collect();
            agg.add(&d1);
            let r = ImgRes { name: im.name.clone(), arm: arm.name, file_bytes: file.len(), payload: payload_prod.len(), model_bits, cat, enc_plain_ms: e1.ms.min(e2.ms), enc_prof_ms: ms1.min(ms2) };
            let mut line = format!("{},{},{},{},{:.0},{:.4}", r.name, r.arm, r.file_bytes, r.payload, r.model_bits, r.model_bits / 8.0 / r.payload as f64);
            for p in 0..NP {
                for c in 0..NCATP {
                    let _ = write!(line, ",{:.0},{}", r.cat[p].0[c], r.cat[p].1[c]);
                }
            }
            for g in &gaps {
                let _ = write!(line, ",{g:.0}");
            }
            gap_rows.push((r.name.clone(), arm.name, gaps.clone()));
            csv_lines.push(line);
            println!("{} [{}] file {} model {:.0} B ({:.4}x payload) enc plain {:.0} ms prof {:.0} ms {}", r.name, r.arm, r.file_bytes, r.model_bits / 8.0, r.model_bits / 8.0 / r.payload as f64, r.enc_plain_ms, r.enc_prof_ms, if why_ok(&problems, &im.name, arm.name) { "ok" } else { "PROBLEM" });
            res.push(r);
        }
        all.push((agg, res));
    }
    let mut hdr = String::from("image,arm,file_bytes,payload_bytes,model_bits,model_bytes_over_payload");
    for p in PLANES {
        for c in CATP_NAMES {
            let _ = write!(hdr, ",{p} | {c} bits,{p} | {c} decisions");
        }
    }
    hdr += ",sign_gap_vs_joint_S0S1S2P_G_bytes,sign_gap_R-G,sign_gap_B-G,node0_gap_vs_joint_AxP_G_bytes,node0_gap_R-G,node0_gap_B-G";
    std::fs::write(out.join("per_image.csv"), format!("{hdr}\n{}\n", csv_lines.join("\n"))).unwrap();
    if !problems.is_empty() {
        std::fs::write(out.join("gate_failure.txt"), problems.join("\n")).unwrap();
        eprintln!("GATE FAILED (profiler or reconciliation):\n{}", problems.join("\n"));
        std::process::exit(2);
    }
    // ---- reports
    let samples_per_plane: u64 = imgs.iter().map(|i| (i.sub.w * i.sub.h) as u64).sum();
    let mut md = String::from("# PKON2 bit-cost profile\n\n");
    md += &format!(
        "Corpus: the 20 JPEG-origin M6 images (1024-px centre crops, same order/representation as `exp_n3_full`). Arms: `control` = codec 1 = X4 (the frozen M6 reference), `P` = codec 2 = X4 + the P mixer input = the current production default.\n\nGates: **PASS**. For every image and arm: the profiling-disabled stream and the profiling-enabled stream are byte-identical to the production payload (`pkonfile::encode_with_codec`), the production decoder reproduces the pixels exactly, the profiled encode is deterministic and ends in the same model-state/trace hash, the observed cost equals the sum of costs over every `code` call, decision counts reconcile (mixed + tail decisions == coder calls), every per-(plane, category) cost and count equals the existing `CeEnc` cross-entropy, and the X4 arm equals the production-X4 byte counts recorded by `x4-promotion-verify` (the *frozen M6* X4 used context `In::A`; production X4 is the N3-promoted `In::Ab`, which saves 0.570% on these 20 images: 15,687,437 -> 15,598,046 payload+11 B).\n\nPlane samples per plane (denominator for bits/sample): {samples_per_plane} (= {} images x 1024 x 1024). Total samples (3 planes): {}.\n\n",
        imgs.len(),
        samples_per_plane * 3
    );
    md += "**Estimator definitions** (see `src/profile.rs`): *model* = sum of ideal costs `-log2 P` of the probabilities the coder was given (ideal bytes, not the physical range-coder bits); *solo* = ideal cost of one mixer input's own stored probability on the same decisions; *H* = static two-pass empirical entropy of the (plane, node, family) partition per image (optimistic: fits the image, ignores parameter cost); *KT* = sequential Krichevsky-Trofimov code length of the same partition per image (a plain adaptive count per context, includes its learning cost). Gap = model - reference on the same decisions. Only gaps against KT are treated as evidence of avoidable cost; H is shown as the optimistic bound.\n\n";
    for (ai, (agg, res)) in all.iter().enumerate() {
        let arm = arms[ai].0.name;
        let title = if ai == 0 { "X4 control (codec 1, frozen M6 reference)" } else { "P (codec 2, current production)" };
        md += &format!("\n---\n\n# Arm `{arm}`: {title}\n\n");
        let tot_file: usize = res.iter().map(|r| r.file_bytes).sum();
        let tot_payload: usize = res.iter().map(|r| r.payload).sum();
        let tot_model: f64 = res.iter().map(|r| r.model_bits).sum();
        md += &format!("Actual encoded: {tot_file} file bytes ({tot_payload} payload + {} x 24 header). Ideal model cost: {:.0} bytes = {:.5}x payload (the ~0.0x% difference is coder termination/precision overhead; categories are NOT forced to sum to the file size). Bits per sample: {:.4} (file, 3 planes), {:.4} (model).\n\n", res.len(), tot_model / 8.0, tot_model / 8.0 / tot_payload as f64, tot_file as f64 * 8.0 / (samples_per_plane * 3) as f64, tot_model / (samples_per_plane * 3) as f64);
        // --- category by plane
        md += "## 1. Cost by plane and coding stage (aggregate)\n\n| plane | stage | decisions | ideal bytes | share of total | bits/decision | bits/plane-sample |\n|---|---|---:|---:|---:|---:|---:|\n";
        let mut cat_tot = [[0f64; NCATP]; NP];
        let mut cat_n = [[0u64; NCATP]; NP];
        for r in res {
            for p in 0..NP {
                for c in 0..NCATP {
                    cat_tot[p][c] += r.cat[p].0[c];
                    cat_n[p][c] += r.cat[p].1[c];
                }
            }
        }
        for p in 0..NP {
            let pt: f64 = cat_tot[p].iter().sum();
            for c in 0..NCATP {
                md += &format!("| {} | {} | {} | {:.0} | {:.2}% | {:.4} | {:.4} |\n", PLANES[p], CATP_NAMES[c], cat_n[p][c], cat_tot[p][c] / 8.0, 100.0 * cat_tot[p][c] / tot_model, cat_tot[p][c] / cat_n[p][c].max(1) as f64, cat_tot[p][c] / samples_per_plane as f64);
            }
            md += &format!("| **{}** | **all** | {} | {:.0} | {:.2}% | {:.4} | {:.4} |\n", PLANES[p], cat_n[p].iter().sum::<u64>(), pt / 8.0, 100.0 * pt / tot_model, pt / cat_n[p].iter().sum::<u64>() as f64, pt / samples_per_plane as f64);
        }
        md += "\nAll planes by stage:\n\n| stage | decisions | ideal bytes | share | bits/decision |\n|---|---:|---:|---:|---:|\n";
        for c in 0..NCATP {
            let (b, n): (f64, u64) = (0..NP).map(|p| (cat_tot[p][c], cat_n[p][c])).fold((0.0, 0), |x, y| (x.0 + y.0, x.1 + y.1));
            md += &format!("| {} | {} | {:.0} | {:.2}% | {:.4} |\n", CATP_NAMES[c], n, b / 8.0, 100.0 * b / tot_model, b / n.max(1) as f64);
        }
        // --- node table
        md += "\n## 2. Unary/mantissa/sign decision nodes (aggregate over planes)\n\nObserved P(bit=1) is the empirical frequency of 1; `H uncond` is the per-image static entropy of the node's bit with no context at all, `KT uncond` the sequential version. `model` vs these shows how much context modelling already gains at that node.\n\n| plane | node | meaning | decisions | P(1) | model bytes | model b/dec | H uncond b/dec | KT uncond b/dec | model/H |\n|---|---|---|---:|---:|---:|---:|---:|---:|---:|\n";
        let meaning = |n: usize| -> String {
            match n {
                0 => "class>0 (zero?)".into(),
                1 => "class>1".into(),
                2..=7 => format!("class>{n}"),
                8..=15 => format!("mantissa bit 1 (class {})", n - 8),
                16..=25 => format!("mantissa bit 2 (class {}, prev bit {})", 3 + (n - 16) / 2, (n - 16) % 2),
                _ => "sign (negative?)".into(),
            }
        };
        for p in 0..NP {
            for node in 0..NODES {
                let a = &agg.nodes[p * NODES + node];
                if a.n == 0 {
                    continue;
                }
                md += &format!("| {} | {} | {} | {} | {:.4} | {:.0} | {:.4} | {:.4} | {:.4} | {:.3} |\n", PLANES[p], node, meaning(node), a.n, a.n1 as f64 / a.n as f64, a.bits / 8.0, a.bits / a.n as f64, a.h / a.n as f64, a.kt / a.n as f64, a.bits / a.h.max(1.0));
            }
        }
        // --- tail
        md += "\n## 3. Context-free tail bits\n\nSource semantics: `tail_bit` codes each low mantissa bit (positions below the two modeled bits) with ONE adaptive probability per (plane, class, bit position), no mixing and no other context; it is not an equiprobable raw bit. Observed P(1) and the empirical entropy of that (plane, class, pos) partition:\n\n| plane | class | pos | decisions | P(1) | model bytes | model b/bit | H b/bit | KT b/bit | model - KT (bytes) |\n|---|---|---|---:|---:|---:|---:|---:|---:|---:|\n";
        let (mut tb, mut tn, mut th, mut tk) = (0.0, 0u64, 0.0, 0.0);
        for p in 0..NP {
            for class in 0..9 {
                for pos in 0..8 {
                    let t = &agg.tail[(p * 9 + class) * 8 + pos];
                    if t.n == 0 {
                        continue;
                    }
                    tb += t.bits;
                    tn += t.n;
                    th += t.h;
                    tk += t.kt;
                    md += &format!("| {} | {} | {} | {} | {:.4} | {:.0} | {:.4} | {:.4} | {:.4} | {:.0} |\n", PLANES[p], class, pos, t.n, t.n1 as f64 / t.n as f64, t.bits / 8.0, t.bits / t.n as f64, t.h / t.n as f64, t.kt / t.n as f64, (t.bits - t.kt) / 8.0);
                }
            }
        }
        md += &format!("\nTail total: {tn} bits coded, model {:.0} bytes ({:.4} b/bit), H {:.0} bytes, KT {:.0} bytes; model - KT = {:+.0} bytes.\n", tb / 8.0, tb / tn.max(1) as f64, th / 8.0, tk / 8.0, (tb - tk) / 8.0);
        // --- family rollups
        md += "\n## 4. Model vs empirical reference by context family\n\nFor each (plane, stage, family slot): the decisions at those nodes partitioned by that family's context value. `mix` = final model cost; `solo` = that family's own probability alone; H/KT = references for the same partition. `mix-KT` > 0 means the full mixed model pays MORE than a plain adaptive count on this single family; < 0 means mixing beats that single family (expected, since the model also uses the other families). `solo-KT` isolates the cost of the production bit-state estimator (rate 1/(n+1.5), limit 255) against KT on the same partition.\n\nFamilies for non-sign nodes: A = (activity*4 + prev-plane residual class)*3 + MED branch; B = (class of W residual * 4 + class of N residual)*4 + prev class; C = (activity*3 + local error level)*4 + prev class; D = activity bucket of the extended residual energy*8 + activity of prev-plane residual; P = predictor-disagreement feature (49 classes). For sign nodes: S0 = prev sign*4 + class, S1 = W,N,NE signs, S2 = prev,W,N signs, P as above.\n\n| plane | stage | family | decisions | ctx used | obs/ctx | mix bytes | solo bytes | H bytes | KT bytes | mix-KT | solo-KT | mix-H |\n|---|---|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|\n";
        let mut fam_csv = String::from("plane,stage,family,decisions,ctx_used,mix_bytes,solo_bytes,h_bytes,kt_bytes,mix_minus_kt_bytes,solo_minus_kt_bytes,mix_minus_h_bytes\n");
        for p in 0..NP {
            for g in 0..5 {
                for slot in 0..NSLOT {
                    let (mut obs, mut used, mut mix, mut solo, mut h, mut kt) = (0u64, 0usize, 0.0, 0.0, 0.0, 0.0);
                    let mut nm = "";
                    for node in 0..NODES {
                        if node_cat(node) != g {
                            continue;
                        }
                        nm = slot_name(node, slot);
                        for ctx in 0..CAP {
                            let c = &agg.cells[((p * NODES + node) * NSLOT + slot) * CAP + ctx];
                            if c.n0 + c.n1 == 0 {
                                continue;
                            }
                            obs += c.n0 + c.n1;
                            used += 1;
                            mix += c.mix;
                            solo += c.solo;
                            h += c.h;
                            kt += c.kt;
                        }
                    }
                    if obs == 0 {
                        continue;
                    }
                    md += &format!("| {} | {} | {} | {} | {} | {:.0} | {:.0} | {:.0} | {:.0} | {:.0} | {:+.0} | {:+.0} | {:+.0} |\n", PLANES[p], grp_name(g), nm, obs, used, obs as f64 / used as f64, mix / 8.0, solo / 8.0, h / 8.0, kt / 8.0, (mix - kt) / 8.0, (solo - kt) / 8.0, (mix - h) / 8.0);
                    let _ = writeln!(fam_csv, "{},{},{},{},{},{:.1},{:.1},{:.1},{:.1},{:.1},{:.1},{:.1}", PLANES[p], grp_name(g), nm, obs, used, mix / 8.0, solo / 8.0, h / 8.0, kt / 8.0, (mix - kt) / 8.0, (solo - kt) / 8.0, (mix - h) / 8.0);
                }
            }
        }
        std::fs::write(out.join(format!("families_{arm}.csv")), fam_csv).unwrap();
        // --- top contexts by excess over KT
        let mut rows: Vec<(f64, usize)> = vec![];
        for (i, c) in agg.cells.iter().enumerate() {
            if c.n0 + c.n1 >= 1000 {
                rows.push(((c.mix - c.kt) / 8.0, i));
            }
        }
        rows.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap());
        md += "\n## 5. Contexts where the model pays most over a plain adaptive count (mix - KT, bytes; support >= 1000 decisions)\n\n| plane | node | family | ctx | decisions | P(1) | mix bytes | solo bytes | KT bytes | mix-KT | mix b/dec | KT b/dec |\n|---|---|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|\n";
        let mut ctx_csv = String::from("plane,node,family,ctx,decisions,p1,mix_bytes,solo_bytes,h_bytes,kt_bytes,mix_minus_kt\n");
        for (rank, (ex, i)) in rows.iter().enumerate() {
            let c = &agg.cells[*i];
            let ctx = i % CAP;
            let slot = (i / CAP) % NSLOT;
            let node = (i / CAP / NSLOT) % NODES;
            let p = i / CAP / NSLOT / NODES;
            let n = (c.n0 + c.n1) as f64;
            if rank < 30 {
                md += &format!("| {} | {} | {} | {} | {} | {:.4} | {:.0} | {:.0} | {:.0} | {:+.0} | {:.4} | {:.4} |\n", PLANES[p], node, slot_name(node, slot), ctx, n as u64, c.n1 as f64 / n, c.mix / 8.0, c.solo / 8.0, c.kt / 8.0, ex, c.mix / n, c.kt / n);
            }
            let _ = writeln!(ctx_csv, "{},{},{},{},{},{:.5},{:.1},{:.1},{:.1},{:.1},{:.1}", PLANES[p], node, slot_name(node, slot), ctx, n as u64, c.n1 as f64 / n, c.mix / 8.0, c.solo / 8.0, c.h / 8.0, c.kt / 8.0, ex);
        }
        std::fs::write(out.join(format!("contexts_{arm}.csv")), ctx_csv).unwrap();
        let pos_sum: f64 = rows.iter().filter(|r| r.0 > 0.0).map(|r| r.0).sum();
        md += &format!("\n{} contexts have support >= 1000; {} of them have mix > KT, summing to {:.0} bytes of single-context excess (not additive savings: the contexts overlap across families).\n", rows.len(), rows.iter().filter(|r| r.0 > 0.0).count(), pos_sum);
        // --- state buckets
        md += "\n## 6. Cost by model-confidence bucket (count of the least-seen base input at the decision; tail uses its own count)\n\n| stage | bucket | decisions | share of decisions | ideal bytes | b/decision |\n|---|---|---:|---:|---:|---:|\n";
        for c in 0..NCATP {
            let tot_n: u64 = (0..NP).map(|p| (0..NB).map(|b| agg.st[p][c][b].n).sum::<u64>()).sum();
            for b in 0..NB {
                let (bits, n) = (0..NP).map(|p| (agg.st[p][c][b].bits, agg.st[p][c][b].n)).fold((0.0, 0u64), |x, y| (x.0 + y.0, x.1 + y.1));
                if n == 0 {
                    continue;
                }
                md += &format!("| {} | {} | {} | {:.3}% | {:.0} | {:.4} |\n", CATP_NAMES[c], BUCKET_NAMES[b], n, 100.0 * n as f64 / tot_n as f64, bits / 8.0, bits / n as f64);
            }
        }
        // --- calibration of the final mixed probability (what an ideal SSE/APM on that partition could at best recover)
        md += "\n## 6b. Calibration of the final mixed probability (per plane and stage)\n\nThe decisions are partitioned by the model's own final probability (32 logit buckets; `cal2`: 16 buckets x the activity bucket of family A). If an output-recalibration stage could help, the model's cost would exceed the reference cost of that partition. `model - KT` is the fair (sequential, includes learning) estimate of what a perfect recalibration on that partition could save; `model - H` is the optimistic static bound.\n\n| plane | stage | model bytes | cal1 H | cal1 KT | model-KT(cal1) | % of stage | cal2 H | cal2 KT | model-KT(cal2) | % of stage |\n|---|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|\n";
        let (mut tot_c1, mut tot_c2) = (0.0, 0.0);
        for p in 0..NP {
            for g in 0..5 {
                let (mut mb, mut h1, mut k1, mut h2, mut k2) = (0.0, 0.0, 0.0, 0.0, 0.0);
                for node in 0..NODES {
                    if node_cat(node) != g {
                        continue;
                    }
                    for b in 0..NCAL1 {
                        let c = &agg.ext.cal1[(p * NODES + node) * NCAL1 + b];
                        mb += c.bits;
                        h1 += c.h;
                        k1 += c.kt;
                    }
                    for b in 0..NCAL2 {
                        let c = &agg.ext.cal2[(p * NODES + node) * NCAL2 + b];
                        h2 += c.h;
                        k2 += c.kt;
                    }
                }
                let has2 = g != 4;
                tot_c1 += mb - k1;
                if has2 {
                    tot_c2 += mb - k2;
                }
                md += &format!(
                    "| {} | {} | {:.0} | {:.0} | {:.0} | {:+.0} | {:+.2}% | {} | {} | {} | {} |\n",
                    PLANES[p], grp_name(g), mb / 8.0, h1 / 8.0, k1 / 8.0, (mb - k1) / 8.0, 100.0 * (mb - k1) / mb,
                    if has2 { format!("{:.0}", h2 / 8.0) } else { "-".into() }, if has2 { format!("{:.0}", k2 / 8.0) } else { "-".into() }, if has2 { format!("{:+.0}", (mb - k2) / 8.0) } else { "-".into() }, if has2 { format!("{:+.2}%", 100.0 * (mb - k2) / mb) } else { "-".into() }
                );
            }
        }
        md += &format!("\nSum of (model - KT) over all plane x stage rows: cal1 {:+.0} bytes ({:+.3}% of model), cal2 (non-sign rows only) {:+.0} bytes. Negative = the model is already better than that recalibration reference.\n", tot_c1 / 8.0, 100.0 * tot_c1 / tot_model, tot_c2 / 8.0);
        md += "\nCalibration detail for node 0 (zero flag): mean predicted P(1) vs observed frequency per probability bucket (buckets with >= 1000 decisions):\n\n| plane | bucket | decisions | mean predicted P(1) | observed P(1) | model bytes | H bytes | KT bytes | model-KT |\n|---|---:|---:|---:|---:|---:|---:|---:|---:|\n";
        for p in 0..NP {
            for b in 0..NCAL1 {
                let c = &agg.ext.cal1[(p * NODES) * NCAL1 + b];
                let n = (c.n[0] + c.n[1]) as f64;
                if n >= 1000.0 {
                    md += &format!("| {} | {} | {} | {:.4} | {:.4} | {:.0} | {:.0} | {:.0} | {:+.0} |\n", PLANES[p], b, n as u64, c.psum / n, c.n[1] as f64 / n, c.bits / 8.0, c.h / 8.0, c.kt / 8.0, (c.bits - c.kt) / 8.0);
                }
            }
        }
        // --- joint contexts
        md += "\n## 6c. Joint-context references (does the mixer leave interaction information unused?)\n\nFor nodes 0 and 1 (the two most expensive non-sign decisions) the decisions are partitioned by the JOINT context of two families; for the sign node by the joint (S0,S1,S2) and (S0,S1,S2,P). KT = sequential adaptive count on that joint context (includes learning cost for the larger partition, so it is a fair reference). `best single` is the lowest KT among the single-family partitions at that node. `model - KT(joint)` > 0 means a plain adaptive count on that joint context would have beaten the whole mixer.\n\n| plane | node | partition | decisions | model bytes | best single KT | joint H | joint KT | model-KT(joint) | % of node |\n|---|---|---|---:|---:|---:|---:|---:|---:|---:|\n";
        let fam = ["A", "B", "C", "D", "P"];
        let mut best_joint: Vec<(f64, String)> = vec![];
        for p in 0..NP {
            for node in 0..PAIR_NODES {
                let mb = agg.nodes[p * NODES + node].bits;
                let best_single = (0..5).map(|s| (0..CAP).map(|c| agg.cells[((p * NODES + node) * NSLOT + s) * CAP + c].kt).sum::<f64>()).fold(f64::MAX, f64::min);
                for (pi, &(a, b)) in PAIRS.iter().enumerate() {
                    let r = &agg.ext.pair[(p * PAIR_NODES + node) * 10 + pi];
                    md += &format!("| {} | {} | {}x{} | {} | {:.0} | {:.0} | {:.0} | {:.0} | {:+.0} | {:+.2}% |\n", PLANES[p], node, fam[a], fam[b], r.n, mb / 8.0, best_single / 8.0, r.h / 8.0, r.kt / 8.0, (mb - r.kt) / 8.0, 100.0 * (mb - r.kt) / mb);
                    best_joint.push(((mb - r.kt) / 8.0, format!("{} node {} {}x{}", PLANES[p], node, fam[a], fam[b])));
                }
            }
            let mb = agg.nodes[p * NODES + SIGN].bits;
            let best_single = (0..5).filter(|s| *s != 3).map(|s| (0..CAP).map(|c| agg.cells[((p * NODES + SIGN) * NSLOT + s) * CAP + c].kt).sum::<f64>()).fold(f64::MAX, f64::min);
            for (nm, r) in [("S0xS1xS2", &agg.ext.sign3[p]), ("S0xS1xS2xP", &agg.ext.sign4[p])] {
                md += &format!("| {} | 26 | {} | {} | {:.0} | {:.0} | {:.0} | {:.0} | {:+.0} | {:+.2}% |\n", PLANES[p], nm, r.n, mb / 8.0, best_single / 8.0, r.h / 8.0, r.kt / 8.0, (mb - r.kt) / 8.0, 100.0 * (mb - r.kt) / mb);
                best_joint.push(((mb - r.kt) / 8.0, format!("{} sign {}", PLANES[p], nm)));
            }
        }
        best_joint.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap());
        md += &format!("\nLargest model-KT(joint) over all rows: {} = {:+.0} bytes. Rows with model-KT(joint) > 0: {}.\n", best_joint[0].1, best_joint[0].0, best_joint.iter().filter(|x| x.0 > 0.0).count());
        // --- per-image consistency of the two largest joint-context gaps
        md += "\n## 6d. Per-image consistency of the joint-context gaps (model bytes minus KT of the joint context; > 0 means the plain joint count beats the mixer on that image)\n\n| image | sign vs S0xS1xS2xP: G | R-G | B-G | node 0 vs AxP: G | R-G | B-G |\n|---|---:|---:|---:|---:|---:|---:|\n";
        let rows: Vec<&(String, &str, Vec<f64>)> = gap_rows.iter().filter(|r| r.1 == arm).collect();
        for r in &rows {
            md += &format!("| {} | {:+.0} | {:+.0} | {:+.0} | {:+.0} | {:+.0} | {:+.0} |\n", r.0, r.2[0], r.2[1], r.2[2], r.2[3], r.2[4], r.2[5]);
        }
        let pos = |k: usize| rows.iter().filter(|r| r.2[k] > 0.0).count();
        md += &format!("\nImages where the joint count beats the mixer (of {}): sign G {}, R-G {}, B-G {}; node 0 AxP: G {}, R-G {}, B-G {}.\n", rows.len(), pos(0), pos(1), pos(2), pos(3), pos(4), pos(5));
        // --- per image
        md += "\n## 7. Per image (actual bytes vs ideal model cost; encode time plain vs profiled)\n\n| image | file bytes | payload | model bytes | model/payload | G | R-G | B-G (ideal bytes) | sign share | enc plain ms | enc profiled ms |\n|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|\n";
        let (mut tp, mut tq) = (0.0, 0.0);
        for r in res {
            let pb: Vec<f64> = (0..NP).map(|p| r.cat[p].0.iter().sum::<f64>() / 8.0).collect();
            let sign: f64 = (0..NP).map(|p| r.cat[p].0[4]).sum::<f64>();
            md += &format!("| {} | {} | {} | {:.0} | {:.4} | {:.0} | {:.0} | {:.0} | {:.2}% | {:.0} | {:.0} |\n", r.name, r.file_bytes, r.payload, r.model_bits / 8.0, r.model_bits / 8.0 / r.payload as f64, pb[0], pb[1], pb[2], 100.0 * sign / r.model_bits, r.enc_plain_ms, r.enc_prof_ms);
            tp += r.enc_plain_ms;
            tq += r.enc_prof_ms;
        }
        md += &format!("\nEncode time (single thread, best of 2, run_rel + range-coder finish; same scope for both): plain {tp:.0} ms, profiled {tq:.0} ms, **x{:.2}**.\n", tq / tp);
    }
    md += &format!("\n---\n\n## Profiler overhead\n\n- Time: see the x-factor at the end of each arm's section (encode only; the profiler is encoder-side, decoding is unchanged and does not call it).\n- Memory: per-image tables {} B (cells {} B), aggregate {} B; both are allocated once per image/run and bounded by planes x nodes x slots x 96 contexts, independent of image size. Process peak memory was not instrumented.\n- Production path: the observation hooks are guarded by the associated constant `RelIo::OBSERVE` (false for every production sink), so production sinks compile without them.\n", ProfEnc::state_bytes(), NP * NODES * NSLOT * CAP * std::mem::size_of::<Cell>(), Agg::state_bytes());
    std::fs::write(out.join("report.md"), md).unwrap();
    println!("done");
}

fn why_ok(problems: &[String], name: &str, arm: &str) -> bool {
    !problems.iter().any(|p| p.starts_with(&format!("{name} / {arm}:")))
}
#[allow(dead_code)]
fn _unused(_: BinEnc) {}
