//! REL6: joint sign context three-arm falsification trial. Frozen rules: historical/20261027-rel6-joint-sign/criteria.md (decision applied by make_report.py).
//!
//!   cargo run --release --bin rel6 -- --phase gates|time|diag|mem --out historical/20261027-rel6-joint-sign [--smoke]
//!
//! Arms (pkonrel::J_ARMS): A = production codec 2 (P only), B = P + DummyJointSign (card 15876, sign node, all planes), C = P + JointSign (sign node, all planes).
//! Loads ONLY the DEV (25) and VAL (10) images of historical/learn1-splits.json. The 7 discovery images that belong to the TEST split are NOT loaded.
use nanoscope::canon::{Canonical, Layout};
use nanoscope::pkon2::{center_crop, Planes};
use nanoscope::pkon2x::{BinDec, BinEnc, BitIo};
use nanoscope::pkonfile::{self, crc32, encode_with_codec, HEADER_LEN, CODEC_X4P};
use nanoscope::pkonlearn::prepared;
use nanoscope::pkonrel::*;
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;

const CROP: u32 = 1024;
const SPLITS: &str = "historical/learn1-splits.json";
const PROFILE_CSV: &str = "historical/20261027-212136Z-bitprof/per_image.csv";
const REL4_CSV: &str = "historical/20261027-rel4-x-on-p/per_image.csv";
const SMOKE: [&str; 3] = ["indoor_0", "food_1", "document_1"];
const STAGES: [&str; NCAT] = ["zero", "ge2", "class", "mantissa+tail", "sign"];
const PLANE_NAMES: [&str; 3] = ["G", "R-G", "B-G"];

fn sha_bytes(b: &[u8]) -> String {
    Sha256::digest(b).iter().map(|x| format!("{x:02x}")).collect()
}
fn pct(base: f64, x: f64) -> f64 {
    100.0 * (base - x) / base
}

struct Img {
    name: String,
    disc: bool,
    dev: bool,
    canon: Canonical,
    sub: Planes,
    orig: Vec<Vec<u8>>,
    src_file: String,
    src_fmt: String,
    src_w: String,
    src_h: String,
    crop_sha: String,
}
impl Img {
    fn set(&self) -> &'static str {
        if self.dev {
            "dev"
        } else {
            "val"
        }
    }
}

fn load_images(out: &Path, smoke: bool) -> Vec<Img> {
    let sj: serde_json::Value = serde_json::from_slice(&std::fs::read(SPLITS).unwrap()).unwrap();
    let get = |k: &str| sj[k].as_array().unwrap().iter().map(|s| s.as_str().unwrap().to_string()).collect::<Vec<_>>();
    let (dev_names, val_names) = (get("train_pool"), get("val"));
    let disc_names: Vec<String> = std::fs::read_to_string(PROFILE_CSV).unwrap().lines().skip(1).filter(|l| l.split(',').nth(1) == Some("P")).map(|l| l.split(',').next().unwrap().to_string()).collect();
    assert_eq!(disc_names.len(), 20);
    let man: Vec<serde_json::Value> = serde_json::from_str(&std::fs::read_to_string("data/dataset50.json").unwrap()).unwrap();
    let crop_hash: BTreeMap<String, String> = {
        let m: serde_json::Value = serde_json::from_str(&std::fs::read_to_string("benchmarks/pkon2/20261027-155433Z-bench50/dataset_manifest.json").unwrap()).unwrap();
        m["images"].as_array().unwrap().iter().map(|i| (i["name"].as_str().unwrap().to_string(), i["crop_sha256"].as_str().unwrap().to_string())).collect()
    };
    let mut imgs = vec![];
    for (names, dev) in [(&dev_names, true), (&val_names, false)] {
        for n in names {
            if smoke && !SMOKE.contains(&n.as_str()) {
                continue;
            }
            let m = man.iter().find(|m| m["name"].as_str() == Some(n)).unwrap();
            let file = m["file"].as_str().unwrap().to_string();
            let full = Canonical::load(&Path::new("data").join(&file)).unwrap();
            let crop = center_crop(&full, CROP);
            assert_eq!(crop.content_hash(), crop_hash[n], "crop hash mismatch for {n}");
            assert_eq!(crop.spec.layout, Layout::Rgb);
            let (sub, orig) = prepared(&crop);
            let crop_sha = sha_bytes(&crop.data);
            imgs.push(Img {
                name: n.clone(),
                disc: disc_names.contains(n),
                dev,
                canon: crop,
                sub,
                orig,
                src_file: file,
                src_fmt: m["source_format"].as_str().unwrap().to_string(),
                src_w: m["width"].as_str().unwrap().to_string(),
                src_h: m["height"].as_str().unwrap().to_string(),
                crop_sha,
            });
        }
    }
    // manifest (written in every phase that loads; identical content)
    let test = get("test");
    let mut csv = String::from("image,split,in_discovery20,source_file,source_format,source_w,source_h,crop_w,crop_h,decoded_representation,crop_data_sha256,crop_content_hash_matches_bench50_manifest\n");
    for im in &imgs {
        let _ = writeln!(csv, "{},{},{},{},{},{},{},{},{},{} {}-bit interleaved (G,R,B planes after SubG),{},true", im.name, im.set(), im.disc, im.src_file, im.src_fmt, im.src_w, im.src_h, im.canon.spec.width, im.canon.spec.height, format!("{:?}", im.canon.spec.layout), im.canon.spec.bit_depth, im.crop_sha);
    }
    if !smoke {
        std::fs::write(out.join("manifest_loaded.csv"), csv).unwrap();
        let mut nl = String::from("# Discovery-20 images that were NOT loaded because they belong to the TEST split of historical/learn1-splits.json\n");
        for n in disc_names.iter().filter(|n| test.contains(n)) {
            let _ = writeln!(nl, "{n}");
        }
        std::fs::write(out.join("manifest_discovery_not_loaded_test_split.txt"), nl).unwrap();
        // overlap report
        let mut ov = String::new();
        let _ = writeln!(ov, "DEV={} VAL={} DEV&VAL={} union={}", dev_names.len(), val_names.len(), dev_names.iter().filter(|n| val_names.contains(n)).count(), dev_names.len() + val_names.len());
        let _ = writeln!(ov, "discovery20 & DEV = {}", disc_names.iter().filter(|n| dev_names.contains(n)).count());
        let _ = writeln!(ov, "discovery20 & VAL = {}", disc_names.iter().filter(|n| val_names.contains(n)).count());
        let _ = writeln!(ov, "discovery20 & TEST (not loaded) = {}", disc_names.iter().filter(|n| test.contains(n)).count());
        std::fs::write(out.join("manifest_overlap.txt"), ov).unwrap();
    }
    for t in &test {
        assert!(!imgs.iter().any(|i| &i.name == t), "TEST image loaded: {t}");
    }
    imgs
}

fn interleave_rgb(planes: &[Vec<u8>], n: usize) -> Vec<u8> {
    let mut d = vec![0u8; n * 3];
    for i in 0..n {
        d[i * 3 + 1] = planes[0][i];
        d[i * 3] = planes[1][i];
        d[i * 3 + 2] = planes[2][i];
    }
    d
}

// ------------------------------------------------------------------ encode / decode with the diagnostics flag
struct Enc {
    stream: Vec<u8>,
    bits: [[f64; NCAT]; MAXP],
    count: [[u64; NCAT]; MAXP],
    trace: u64,
    sign_trace: u64,
    ckpt: Vec<u64>,
    state: u64,
    extra_w: Vec<(usize, usize, f64, f64)>,
}
fn enc_diag(sub: &Planes, arm: &Arm, diag: bool) -> Enc {
    let mut work = Planes { w: sub.w, h: sub.h, p: sub.p.clone() };
    let mut coder = Coder::new(work.p.len(), arm);
    coder.diag = diag;
    let mut io = CeEnc::new();
    let trace = run_rel(&mut work, arm, &mut coder, &mut io, None);
    let (stream, bits, count) = io.finish();
    Enc { stream, bits, count, trace, sign_trace: coder.sign_trace, ckpt: coder.ckpt.clone(), state: coder.state_hash(), extra_w: coder.extra_weights(work.p.len(), arm.extras.len()) }
}
struct Dec {
    planes: Vec<Vec<u8>>,
    trace: u64,
    sign_trace: u64,
    ckpt: Vec<u64>,
    state: u64,
    consumed: usize,
}
fn dec_diag(stream: &[u8], arm: &Arm, w: usize, h: usize, np: usize) -> Dec {
    let mut pl = Planes { w, h, p: vec![vec![0; w * h]; np] };
    let mut coder = Coder::new(np, arm);
    coder.diag = true;
    let mut d = BinDec::new(stream);
    let trace = run_rel(&mut pl, arm, &mut coder, &mut d, None);
    Dec { planes: pl.p, trace, sign_trace: coder.sign_trace, ckpt: coder.ckpt.clone(), state: coder.state_hash(), consumed: d.consumed() }
}

// ------------------------------------------------------------------ gates
struct GateRes {
    file_bytes: usize,
    payload: usize,
    prod_file: Option<Vec<u8>>,
    enc: Enc,
    consumed: usize,
    why: Vec<String>,
}
fn gate_one(im: &Img, ai: usize, frozen: &BTreeMap<String, usize>) -> GateRes {
    let arm = &J_ARMS[ai];
    let e = enc_diag(&im.sub, arm, true);
    let e2 = enc_diag(&im.sub, arm, true);
    let plain = encode_rel(&im.sub, arm, None); // library path, diagnostics off
    let d = dec_diag(&e.stream, arm, im.sub.w, im.sub.h, im.sub.p.len());
    let mut why = vec![];
    let planes = invert(d.planes.clone(), im.sub.w, im.sub.h);
    if !(planes == im.orig && interleave_rgb(&planes, im.sub.w * im.sub.h) == im.canon.data) {
        why.push("G1 pixels differ (raw stream)".to_string());
    }
    // complete file with the production 24-byte header layout (codec byte 3 for B/C: not accepted by production parse_header)
    let mut file = encode(&im.canon).unwrap()[..HEADER_LEN].to_vec();
    if ai != 0 {
        file[6] = 3;
    }
    file[16..20].copy_from_slice(&(e.stream.len() as u32).to_le_bytes());
    file.extend_from_slice(&e.stream);
    let plen = u32::from_le_bytes(file[16..20].try_into().unwrap()) as usize;
    let fd = dec_diag(&file[HEADER_LEN..HEADER_LEN + plen], arm, im.sub.w, im.sub.h, im.sub.p.len());
    let fpl = invert(fd.planes, im.sub.w, im.sub.h);
    let crc_ok = u32::from_le_bytes(file[20..24].try_into().unwrap()) == crc32(&interleave_rgb(&fpl, im.sub.w * im.sub.h)) && file.len() == HEADER_LEN + plen;
    if !(fpl == im.orig && crc_ok) {
        why.push("G1 pixels/CRC differ (decoded from file bytes)".into());
    }
    if (im.canon.spec.width as usize, im.canon.spec.height as usize) != (im.sub.w, im.sub.h) {
        why.push("G1 dimensions".into());
    }
    if d.consumed != e.stream.len() {
        why.push(format!("G3 stream termination: decoder consumed {} of {} bytes", d.consumed, e.stream.len()));
    }
    if e.stream != e2.stream || e.trace != e2.trace || e.sign_trace != e2.sign_trace || e.ckpt != e2.ckpt {
        why.push("G4 re-encode differs".into());
    }
    if d.trace != e.trace || d.sign_trace != e.sign_trace {
        why.push(format!("G5 context sequence differs (trace {} sign_trace {})", d.trace != e.trace, d.sign_trace != e.sign_trace));
    }
    if d.ckpt != e.ckpt || d.state != e.state {
        let first = e.ckpt.iter().zip(&d.ckpt).position(|(a, b)| a != b);
        why.push(format!("G6 adaptive-state hash differs (first differing plane checkpoint: {first:?}, final {})", d.state != e.state));
    }
    if plain.stream != e.stream || plain.trace != e.trace || plain.state != e.state {
        why.push("G7 diagnostics flag changed the stream/trace/state".into());
    }
    let mut prod_file = None;
    if ai == 0 {
        let pf = encode(&im.canon).unwrap();
        if pf != file {
            why.push("G7 Arm A != production codec 2 file (byte-identical required)".into());
        }
        let back = pkonfile::decode(&pf).unwrap();
        if back.data != im.canon.data {
            why.push("G1 production decoder not exact on Arm A file".into());
        }
        if let Some(&f) = frozen.get(&im.name) {
            if f != file.len() {
                why.push(format!("G7 Arm A bytes {} != frozen reference {}", file.len(), f));
            }
        } else {
            why.push("no frozen reference for Arm A".into());
        }
        prod_file = Some(pf);
    }
    if file.len() != HEADER_LEN + e.stream.len() {
        why.push("file bytes != 24 + payload".into());
    }
    GateRes { file_bytes: file.len(), payload: e.stream.len(), prod_file, enc: e, consumed: d.consumed, why }
}

fn frozen_reference(imgs: &[Img]) -> BTreeMap<String, usize> {
    // discovery images: bit-cost profile arm P file bytes; every image: REL4 arm P file bytes (both must agree where both exist)
    let mut prof: BTreeMap<String, usize> = BTreeMap::new();
    for l in std::fs::read_to_string(PROFILE_CSV).unwrap().lines().skip(1) {
        let f: Vec<&str> = l.split(',').collect();
        if f[1] == "P" {
            prof.insert(f[0].to_string(), f[2].parse().unwrap());
        }
    }
    let mut rel4: BTreeMap<String, usize> = BTreeMap::new();
    for l in std::fs::read_to_string(REL4_CSV).unwrap().lines().skip(1) {
        let f: Vec<&str> = l.split(',').collect();
        if f[2] == "P" {
            rel4.insert(f[0].to_string(), f[3].parse().unwrap());
        }
    }
    let mut out = BTreeMap::new();
    for im in imgs {
        let r4 = rel4[&im.name];
        if let Some(&p) = prof.get(&im.name) {
            assert_eq!(p, r4, "frozen references disagree for {} (profile {p} vs REL4 {r4}); not silently picking one", im.name);
        }
        out.insert(im.name.clone(), r4);
    }
    out
}

fn phase_gates(out: &Path, smoke: bool) {
    let imgs = load_images(out, smoke);
    let frozen = frozen_reference(&imgs);
    println!("loaded {} images (smoke={smoke})", imgs.len());
    let na = J_ARMS.len();
    let tasks: Vec<(usize, usize)> = (0..imgs.len()).flat_map(|i| (0..na).map(move |a| (i, a))).collect();
    let next = AtomicUsize::new(0);
    let results: Mutex<Vec<Vec<Option<GateRes>>>> = Mutex::new((0..imgs.len()).map(|_| (0..na).map(|_| None).collect()).collect());
    std::thread::scope(|s| {
        for _ in 0..std::thread::available_parallelism().map(|n| n.get()).unwrap_or(4) {
            s.spawn(|| loop {
                let t = next.fetch_add(1, Ordering::Relaxed);
                if t >= tasks.len() {
                    break;
                }
                let (i, ai) = tasks[t];
                let r = gate_one(&imgs[i], ai, &frozen);
                eprintln!("{} {} file {} {}", imgs[i].name, J_ARMS[ai].name, r.file_bytes, if r.why.is_empty() { "ok".to_string() } else { r.why.join("; ") });
                results.lock().unwrap()[i][ai] = Some(r);
            });
        }
    });
    let results: Vec<Vec<GateRes>> = results.into_inner().unwrap().into_iter().map(|v| v.into_iter().map(|x| x.unwrap()).collect()).collect();
    let mut fails: Vec<String> = vec![];
    for (i, im) in imgs.iter().enumerate() {
        for ai in 0..na {
            for w in &results[i][ai].why {
                fails.push(format!("{} / {}: {}", im.name, J_ARMS[ai].name, w));
            }
        }
        // isolation: non-sign cross-entropy of B and C exactly equal (f64 ==) to A's, decisions too
        for ai in 1..na {
            for k in 0..MAXP {
                for c in 0..4 {
                    if results[i][ai].enc.bits[k][c] != results[i][0].enc.bits[k][c] || results[i][ai].enc.count[k][c] != results[i][0].enc.count[k][c] {
                        fails.push(format!("{} / {} plane {k} stage {}: non-sign CE/count differs from A ({} vs {})", im.name, J_ARMS[ai].name, STAGES[c], results[i][ai].enc.bits[k][c], results[i][0].enc.bits[k][c]));
                    }
                }
                if results[i][ai].enc.count[k][4] != results[i][0].enc.count[k][4] {
                    fails.push(format!("{} / {} plane {k}: sign decision count differs from A", im.name, J_ARMS[ai].name));
                }
            }
        }
    }
    // model counts B == C
    let mc = |ai: usize| Coder::new(3, &J_ARMS[ai]).extra_model_counts(3, &J_ARMS[ai]);
    let (mb, mcc) = (mc(1), mc(2));
    if mb != mcc || mcc.2 != 3 * JCARD {
        fails.push(format!("model counts differ or unexpected: B {mb:?} C {mcc:?}"));
    }
    let tag = if smoke { "smoke_" } else { "" };
    if !fails.is_empty() {
        std::fs::write(out.join(format!("{tag}gate_failure.txt")), fails.join("\n")).unwrap();
        eprintln!("CORRECTNESS GATE FAILED:\n{}", fails.join("\n"));
        std::process::exit(2);
    }
    println!("ALL CORRECTNESS GATES PASS on {} images x {} arms; model counts (alloc, bytes, reachable) B {mb:?} = C {mcc:?}", imgs.len(), na);
    let mut sizes = String::from("image,split,in_discovery20,arm,file_bytes,payload_bytes,ideal_bytes_ce,exact,gates_ok,consumed,sign_trace,state_hash,ckpt_g,ckpt_rg,ckpt_bg\n");
    let mut ce = String::from("image,arm,plane,stage,decisions,bits\n");
    for (i, im) in imgs.iter().enumerate() {
        for ai in 0..na {
            let r = &results[i][ai];
            let ideal: f64 = r.enc.bits.iter().flatten().sum::<f64>() / 8.0;
            let _ = writeln!(sizes, "{},{},{},{},{},{},{:.1},true,true,{},{:016x},{:016x},{:016x},{:016x},{:016x}", im.name, im.set(), im.disc, J_ARMS[ai].name, r.file_bytes, r.payload, ideal, r.consumed, r.enc.sign_trace, r.enc.state, r.enc.ckpt[0], r.enc.ckpt[1], r.enc.ckpt[2]);
            for k in 0..3 {
                for c in 0..NCAT {
                    let _ = writeln!(ce, "{},{},{},{},{},{:.4}", im.name, J_ARMS[ai].name, PLANE_NAMES[k], STAGES[c], r.enc.count[k][c], r.enc.bits[k][c]);
                }
            }
        }
        let _ = results[i][0].prod_file.as_ref();
    }
    std::fs::write(out.join(format!("{tag}sizes.csv")), sizes).unwrap();
    std::fs::write(out.join(format!("{tag}ce_by_plane_stage.csv")), ce).unwrap();
    let mut fz = String::from("image,split,frozen_reference_file_bytes,armA_file_bytes\n");
    for (i, im) in imgs.iter().enumerate() {
        let _ = writeln!(fz, "{},{},{},{}", im.name, im.set(), frozen[&im.name], results[i][0].file_bytes);
    }
    std::fs::write(out.join(format!("{tag}armA_frozen_control.csv")), fz).unwrap();
}

// ------------------------------------------------------------------ timing (single thread, uninstrumented sinks, sequential)
fn time_enc(sub: &Planes, arm: &Arm) -> (f64, Vec<u8>) {
    let mut work = Planes { w: sub.w, h: sub.h, p: sub.p.clone() };
    let t = std::time::Instant::now();
    let mut coder = Coder::new(work.p.len(), arm);
    let mut io = BinEnc::new();
    run_rel(&mut work, arm, &mut coder, &mut io, None);
    let s = io.finish();
    (t.elapsed().as_secs_f64() * 1e3, s)
}
fn time_dec(stream: &[u8], arm: &Arm, w: usize, h: usize, np: usize) -> (f64, Vec<Vec<u8>>) {
    let t = std::time::Instant::now();
    let mut pl = Planes { w, h, p: vec![vec![0; w * h]; np] };
    let mut coder = Coder::new(np, arm);
    let mut d = BinDec::new(stream);
    run_rel(&mut pl, arm, &mut coder, &mut d, None);
    (t.elapsed().as_secs_f64() * 1e3, pl.p)
}

fn phase_time(out: &Path, smoke: bool) {
    let imgs = load_images(out, smoke);
    let reps = 3usize;
    let na = J_ARMS.len();
    // arms: 0..na research harness; na = production codec 2 (pkonfile) as an additional reference
    let mut csv = String::from("image,arm,rep,enc_ms,dec_ms,order_in_rep\n");
    // per-arm init cost
    let mut init = String::from("arm,coder_new_ms_median_of_21\n");
    for (ai, arm) in J_ARMS.iter().enumerate() {
        let mut v: Vec<f64> = (0..21)
            .map(|_| {
                let t = std::time::Instant::now();
                let c = Coder::new(3, arm);
                std::hint::black_box(&c);
                t.elapsed().as_secs_f64() * 1e3
            })
            .collect();
        v.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let _ = writeln!(init, "{},{:.3}", J_ARMS[ai].name, v[10]);
    }
    std::fs::write(out.join(if smoke { "smoke_timings_init.csv" } else { "timings_init.csv" }), init).unwrap();
    // warm-up: full enc+dec of the first image for every arm (untimed)
    for arm in J_ARMS.iter() {
        let (_, s) = time_enc(&imgs[0].sub, arm);
        time_dec(&s, arm, imgs[0].sub.w, imgs[0].sub.h, 3);
    }
    let _ = pkonfile::decode(&encode(&imgs[0].canon).unwrap());
    let t_all = std::time::Instant::now();
    for rep in 0..reps {
        for (ii, im) in imgs.iter().enumerate() {
            // arm order rotates per rep and per image so no arm always runs first
            for pos in 0..(na + 1) {
                let a = (pos + rep + ii) % (na + 1);
                if a < na {
                    let arm = &J_ARMS[a];
                    let (e, s) = time_enc(&im.sub, arm);
                    let (d, pl) = time_dec(&s, arm, im.sub.w, im.sub.h, 3);
                    assert_eq!(invert(pl, im.sub.w, im.sub.h), im.orig, "timing run decoded wrong pixels: {} {}", im.name, arm.name);
                    let _ = writeln!(csv, "{},{},{},{:.2},{:.2},{}", im.name, arm.name, rep, e, d, pos);
                } else {
                    let t = std::time::Instant::now();
                    let f = encode(&im.canon).unwrap();
                    let e = t.elapsed().as_secs_f64() * 1e3;
                    let t = std::time::Instant::now();
                    let b = pkonfile::decode(&f).unwrap();
                    let d = t.elapsed().as_secs_f64() * 1e3;
                    assert_eq!(b.data, im.canon.data);
                    let _ = writeln!(csv, "{},prod_codec2,{},{:.2},{:.2},{}", im.name, rep, e, d, pos);
                }
            }
        }
        eprintln!("timing rep {} done, elapsed {:.0}s", rep + 1, t_all.elapsed().as_secs_f64());
    }
    std::fs::write(out.join(if smoke { "smoke_timings_raw.csv" } else { "timings_raw.csv" }), csv).unwrap();
}

// ------------------------------------------------------------------ instrumented diagnostics
const NHIST: usize = 9;
#[derive(Clone, Default)]
struct PlaneDiag {
    n_sign: u64,
    bits: f64,
    cal_n: [u64; 16],
    cal_n1: [u64; 16],
    cal_psum: [f64; 16],
    cal_bits: [f64; 16],
    solo_bits: f64,
    cold_n: u64,
    cold_bits: f64,
    jctx: Vec<u32>,
}
struct DiagEnc {
    inner: BinEnc,
    cur: (usize, usize),
    bits: [[f64; NCAT]; MAXP],
    count: [[u64; NCAT]; MAXP],
    sign_p: Vec<u16>,
    sign_bit: Vec<u8>,
    sign_plane: Vec<u8>,
    pd: Vec<PlaneDiag>,
}
impl DiagEnc {
    fn new() -> Self {
        DiagEnc { inner: BinEnc::new(), cur: (0, 0), bits: [[0.0; NCAT]; MAXP], count: [[0; NCAT]; MAXP], sign_p: vec![], sign_bit: vec![], sign_plane: vec![], pd: (0..MAXP).map(|_| PlaneDiag { jctx: vec![0; JCARD], ..Default::default() }).collect() }
    }
}
impl BitIo for DiagEnc {
    #[inline]
    fn code(&mut self, p1: u32, bit: u32) -> u32 {
        let c = bit_cost(p1, bit) as f64;
        self.bits[self.cur.0][self.cur.1] += c;
        self.count[self.cur.0][self.cur.1] += 1;
        self.inner.code(p1, bit)
    }
}
impl RelIo for DiagEnc {
    const OBSERVE: bool = true;
    fn tag(&mut self, cat: usize, plane: usize) {
        self.cur = (plane, cat);
    }
    fn observe(&mut self, plane: usize, node: usize, slots: &[usize], ctxs: &[usize], inputs: &[(u16, u8)], p: u32, bit: u32) {
        if node != SIGN {
            return;
        }
        self.sign_p.push(p as u16);
        self.sign_bit.push(bit as u8);
        self.sign_plane.push(plane as u8);
        let c = bit_cost(p, bit) as f64;
        let d = &mut self.pd[plane];
        d.n_sign += 1;
        d.bits += c;
        let b = (p >> 12) as usize;
        d.cal_n[b] += 1;
        d.cal_n1[b] += bit as usize as u64;
        d.cal_psum[b] += p as f64 / 65536.0;
        d.cal_bits[b] += c;
        let last = slots.len() - 1;
        if slots[last] == 5 {
            d.jctx[ctxs[last]] += 1;
            d.solo_bits += bit_cost(inputs[last].0 as u32, bit) as f64;
            if inputs[last].1 == 0 {
                d.cold_n += 1;
                d.cold_bits += c;
            }
        }
    }
}
struct DiagRun {
    enc: Enc,
    d: DiagEnc,
    sat: Vec<(usize, usize)>, // per plane: (active contexts, saturated contexts) of slot 5 at the sign node
    sat_p: Vec<(usize, usize)>,
}
fn diag_arm(im: &Img, ai: usize) -> DiagRun {
    let arm = &J_ARMS[ai];
    let mut work = Planes { w: im.sub.w, h: im.sub.h, p: im.sub.p.clone() };
    let mut coder = Coder::new(3, arm);
    coder.diag = true;
    let mut io = DiagEnc::new();
    let trace = run_rel(&mut work, arm, &mut coder, &mut io, None);
    let stream = std::mem::replace(&mut io.inner, BinEnc::new()).finish();
    let plain = encode_rel(&im.sub, arm, None);
    assert_eq!(stream, plain.stream, "instrumented stream != uninstrumented stream ({} {})", im.name, arm.name);
    assert_eq!(trace, plain.trace);
    let sat: Vec<(usize, usize)> = (0..3).map(|k| if arm.extras.len() > 1 { coder.slot_node_stats(k, 5, SIGN) } else { (0, 0) }).collect();
    let sat_p: Vec<(usize, usize)> = (0..3).map(|k| coder.slot_node_stats(k, 4, SIGN)).collect();
    let enc = Enc { stream, bits: io.bits, count: io.count, trace, sign_trace: coder.sign_trace, ckpt: coder.ckpt.clone(), state: coder.state_hash(), extra_w: coder.extra_weights(3, arm.extras.len()) };
    DiagRun { enc, d: io, sat, sat_p }
}

fn phase_diag(out: &Path) {
    let imgs = load_images(out, false);
    let next = AtomicUsize::new(0);
    let rows: Mutex<Vec<(usize, String, String, String, String, String)>> = Mutex::new(vec![]);
    std::thread::scope(|s| {
        for _ in 0..std::thread::available_parallelism().map(|n| n.get()).unwrap_or(4) {
            s.spawn(|| loop {
                let i = next.fetch_add(1, Ordering::Relaxed);
                if i >= imgs.len() {
                    break;
                }
                let im = &imgs[i];
                let runs: Vec<DiagRun> = (0..3).map(|ai| diag_arm(im, ai)).collect();
                let (mut occ, mut cal, mut chg, mut st, mut stage) = (String::new(), String::new(), String::new(), String::new(), String::new());
                // alignment: the same sign decisions in the same order in every arm
                for ai in 1..3 {
                    assert!(runs[ai].d.sign_bit == runs[0].d.sign_bit && runs[ai].d.sign_plane == runs[0].d.sign_plane, "sign decision sequence differs between arms on {}", im.name);
                }
                for ai in 0..3 {
                    let r = &runs[ai];
                    for k in 0..3 {
                        for c in 0..NCAT {
                            let _ = writeln!(stage, "{},{},{},{},{},{:.4}", im.name, J_ARMS[ai].name, PLANE_NAMES[k], STAGES[c], r.enc.count[k][c], r.enc.bits[k][c]);
                        }
                        let d = &r.d.pd[k];
                        for b in 0..16 {
                            let _ = writeln!(cal, "{},{},{},{},{},{},{:.4},{:.4}", im.name, J_ARMS[ai].name, PLANE_NAMES[k], b, d.cal_n[b], d.cal_n1[b], d.cal_psum[b], d.cal_bits[b]);
                        }
                        // adaptive state (final): extras' mean mixer weights at node 0 and the sign node, saturation of the extra tables
                        let wts: Vec<String> = r.enc.extra_w.iter().filter(|w| w.0 == k).map(|w| format!("{:.5}", w.3)).collect();
                        let ckpt_k = r.enc.ckpt[k];
                        let _ = writeln!(st, "{},{},{},{},{},{},{},{:016x},{}", im.name, J_ARMS[ai].name, PLANE_NAMES[k], wts.get(0).cloned().unwrap_or_default(), wts.get(1).cloned().unwrap_or_default(), r.sat_p[k].0, r.sat_p[k].1, ckpt_k, if ai == 0 { "-".to_string() } else { format!("{}/{}", r.sat[k].0, r.sat[k].1) });
                        if ai >= 1 {
                            let hist = &d.jctx;
                            let active = hist.iter().filter(|&&v| v > 0).count();
                            let total: u64 = hist.iter().map(|&v| v as u64).sum();
                            let mut hc = [0u64; NHIST];
                            let mut hd = [0u64; NHIST];
                            for &v in hist.iter().filter(|&&v| v > 0) {
                                let b = (31 - (v as u32).leading_zeros() as usize).min(NHIST - 1);
                                hc[b] += 1;
                                hd[b] += v as u64;
                            }
                            let mut sorted: Vec<u32> = hist.iter().copied().filter(|&v| v > 0).collect();
                            sorted.sort_unstable();
                            let med = sorted.get(sorted.len() / 2).copied().unwrap_or(0);
                            let _ = writeln!(
                                occ,
                                "{},{},{},{},{},{},{},{},{:.2},{},{},{},{:.4},{:.4},{:.4},{},{},{},{},{}",
                                im.name,
                                J_ARMS[ai].name,
                                PLANE_NAMES[k],
                                JCARD,
                                active,
                                JCARD - active,
                                total,
                                total,
                                if active > 0 { total as f64 / active as f64 } else { 0.0 },
                                med,
                                d.cold_n,
                                d.n_sign,
                                d.cold_bits / 8.0,
                                d.solo_bits / 8.0,
                                d.bits / 8.0,
                                hc.iter().map(|v| v.to_string()).collect::<Vec<_>>().join(";"),
                                hd.iter().map(|v| v.to_string()).collect::<Vec<_>>().join(";"),
                                r.sat[k].1,
                                r.sat[k].0,
                                0
                            );
                        }
                    }
                }
                // sign-probability changes relative to A and C relative to B
                for (xi, yi) in [(1usize, 0usize), (2, 0), (2, 1)] {
                    let (px, py) = (&runs[xi].d.sign_p, &runs[yi].d.sign_p);
                    let bits = &runs[0].d.sign_bit;
                    let pl = &runs[0].d.sign_plane;
                    for k in 0..3usize {
                        let (mut n, mut changed, mut better, mut worse) = (0u64, 0u64, 0u64, 0u64);
                        let (mut dl, mut dbits) = (0.0f64, 0.0f64);
                        for j in 0..px.len() {
                            if pl[j] as usize != k {
                                continue;
                            }
                            n += 1;
                            if px[j] != py[j] {
                                changed += 1;
                                let lo = |p: u16| (p as f64 / (65536.0 - p as f64)).ln();
                                dl += (lo(px[j]) - lo(py[j])).abs();
                                let (cx, cy) = (bit_cost(px[j] as u32, bits[j] as u32) as f64, bit_cost(py[j] as u32, bits[j] as u32) as f64);
                                dbits += cy - cx;
                                if cx < cy {
                                    better += 1;
                                } else if cx > cy {
                                    worse += 1;
                                }
                            }
                        }
                        let _ = writeln!(chg, "{},{} vs {},{},{},{},{:.5},{},{},{:.4}", im.name, J_ARMS[xi].name, J_ARMS[yi].name, PLANE_NAMES[k], n, changed, if changed > 0 { dl / changed as f64 } else { 0.0 }, better, worse, dbits);
                    }
                }
                eprintln!("diag {} done", im.name);
                rows.lock().unwrap().push((i, occ, cal, chg, st, stage));
            });
        }
    });
    let mut rows = rows.into_inner().unwrap();
    rows.sort_by_key(|r| r.0);
    let mut occ = String::from("image,arm,plane,allocated_ctx,active_ctx,unseen_ctx,total_obs,sum_check,obs_per_active_ctx,median_obs_active,cold_decisions_n0,sign_decisions,cold_bytes,solo_joint_model_bytes,mixed_sign_bytes,ctx_hist_log2bin_counts_1;2-3;4-7;8-15;16-31;32-63;64-127;128-255;256+,decisions_in_hist_bins,saturated_joint_models_at_sign_node,joint_models_n_gt_0_at_sign_node,pad\n");
    let mut cal = String::from("image,arm,plane,p_bucket16,n,n_ones,sum_pred_p,bits\n");
    let mut chg = String::from("image,comparison,plane,sign_decisions,decisions_p_changed,mean_abs_logit_change,changed_and_cheaper,changed_and_costlier,net_bits_saved\n");
    let mut st = String::from("image,arm,plane,w_P_at_sign_node(first extra),w_second_extra_at_sign_node,P_models_active,P_models_saturated,state_hash_after_plane,joint_models_active/saturated\n");
    let mut stage = String::from("image,arm,plane,stage,decisions,bits\n");
    for r in rows {
        occ += &r.1;
        cal += &r.2;
        chg += &r.3;
        st += &r.4;
        stage += &r.5;
    }
    std::fs::write(out.join("context_occupancy.csv"), occ).unwrap();
    std::fs::write(out.join("calibration.csv"), cal).unwrap();
    std::fs::write(out.join("sign_probability_changes.csv"), chg).unwrap();
    std::fs::write(out.join("adaptive_state.csv"), st).unwrap();
    std::fs::write(out.join("per_plane_stage.csv"), stage).unwrap();
    println!("diagnostics written");
}

// ------------------------------------------------------------------ instrumentation overhead (separate from the timing table)
fn phase_instr(out: &Path) {
    let mut imgs = load_images(out, true);
    imgs.truncate(3);
    let mut csv = String::from("image,arm,sink,rep,ms
");
    for im in &imgs {
        for (ai, arm) in J_ARMS.iter().enumerate() {
            for rep in 0..3 {
                let (plain, _) = time_enc(&im.sub, arm);
                let t = std::time::Instant::now();
                let e = enc_diag(&im.sub, arm, false);
                std::hint::black_box(&e);
                let ce = t.elapsed().as_secs_f64() * 1e3;
                let t = std::time::Instant::now();
                let r = diag_arm(im, ai);
                std::hint::black_box(&r);
                let dg = t.elapsed().as_secs_f64() * 1e3;
                let _ = writeln!(csv, "{},{},plain,{rep},{plain:.1}
{},{},ce,{rep},{ce:.1}
{},{},diag,{rep},{dg:.1}", im.name, arm.name, im.name, arm.name, im.name, arm.name);
            }
        }
    }
    std::fs::write(out.join("instr_overhead.csv"), csv).unwrap();
}

// ------------------------------------------------------------------ memory
#[cfg(windows)]
mod winmem {
    #[repr(C)]
    #[derive(Default)]
    pub struct Pmc {
        pub cb: u32,
        pub page_fault_count: u32,
        pub peak_working_set: usize,
        pub working_set: usize,
        pub q1: usize,
        pub q2: usize,
        pub q3: usize,
        pub q4: usize,
        pub pagefile: usize,
        pub peak_pagefile: usize,
    }
    #[link(name = "kernel32")]
    extern "system" {
        fn GetCurrentProcess() -> isize;
        fn K32GetProcessMemoryInfo(h: isize, p: *mut Pmc, cb: u32) -> i32;
    }
    pub fn now() -> (usize, usize, usize) {
        let mut p = Pmc::default();
        p.cb = std::mem::size_of::<Pmc>() as u32;
        unsafe {
            K32GetProcessMemoryInfo(GetCurrentProcess(), &mut p, p.cb);
        }
        (p.working_set, p.peak_working_set, p.peak_pagefile)
    }
}
#[cfg(not(windows))]
mod winmem {
    pub fn now() -> (usize, usize, usize) {
        (0, 0, 0)
    }
}

/// child: `--phase memprobe --raw <planes.bin> --w W --h H --arm i --mode enc|dec [--stream file]`; prints "base_ws peak_ws_after peak_commit_after"
fn phase_memprobe(a: &[String]) {
    let g = |k: &str| a.iter().position(|x| x == k).map(|i| a[i + 1].clone()).unwrap();
    let (w, h): (usize, usize) = (g("--w").parse().unwrap(), g("--h").parse().unwrap());
    let ai: usize = g("--arm").parse().unwrap();
    let arm = &J_ARMS[ai];
    let raw = std::fs::read(g("--raw")).unwrap();
    let planes: Vec<Vec<u8>> = (0..3).map(|k| raw[k * w * h..(k + 1) * w * h].to_vec()).collect();
    drop(raw);
    let sub = Planes { w, h, p: planes };
    let (base_ws, _, _) = winmem::now();
    let mode = g("--mode");
    if mode == "enc" {
        let mut work = Planes { w, h, p: sub.p.clone() };
        let mut coder = Coder::new(3, arm);
        let mut io = BinEnc::new();
        run_rel(&mut work, arm, &mut coder, &mut io, None);
        let s = io.finish();
        std::hint::black_box(&s);
    } else {
        let stream = std::fs::read(g("--stream")).unwrap();
        let mut pl = Planes { w, h, p: vec![vec![0; w * h]; 3] };
        let mut coder = Coder::new(3, arm);
        let mut d = BinDec::new(&stream);
        run_rel(&mut pl, arm, &mut coder, &mut d, None);
        std::hint::black_box(&pl);
    }
    let (_, peak_ws, peak_commit) = winmem::now();
    println!("{base_ws} {peak_ws} {peak_commit}");
}

fn phase_mem(out: &Path) {
    let imgs = load_images(out, false);
    let scratch = std::env::temp_dir().join("rel6_mem");
    std::fs::create_dir_all(&scratch).unwrap();
    let exe = std::env::current_exe().unwrap();
    // images: smallest, median and largest by the frozen Arm A file size at 1024, plus three smaller crops of the median image
    let frozen = frozen_reference(&imgs);
    let mut by_size: Vec<(usize, usize)> = imgs.iter().enumerate().map(|(i, im)| (i, frozen[&im.name])).collect();
    by_size.sort_by_key(|x| x.1);
    let chosen = [by_size[0].0, by_size[by_size.len() / 2].0, by_size[by_size.len() - 1].0];
    let mut csv = String::from("image,crop_px,arm,mode,rep,base_working_set_bytes,peak_working_set_bytes,peak_commit_bytes,peak_minus_base_bytes\n");
    let mut jobs: Vec<(String, u32, Planes, Vec<Vec<u8>>)> = vec![];
    for &i in &chosen {
        jobs.push((imgs[i].name.clone(), 1024, Planes { w: imgs[i].sub.w, h: imgs[i].sub.h, p: imgs[i].sub.p.clone() }, vec![]));
    }
    let ii = chosen[1];
    for px in [128u32, 256, 512] {
        let c = center_crop(&imgs[ii].canon, px);
        let (sub, _) = prepared(&c);
        jobs.push((imgs[ii].name.clone(), px, sub, vec![]));
    }
    for (name, px, sub, _) in &jobs {
        let raw: Vec<u8> = sub.p.iter().flatten().copied().collect();
        let rawp = scratch.join(format!("{name}_{px}.raw"));
        std::fs::write(&rawp, &raw).unwrap();
        for ai in 0..J_ARMS.len() {
            let stream = encode_rel(sub, &J_ARMS[ai], None).stream;
            let sp = scratch.join(format!("{name}_{px}_{ai}.stream"));
            std::fs::write(&sp, &stream).unwrap();
            for mode in ["enc", "dec"] {
                for rep in 0..3 {
                    let o = std::process::Command::new(&exe)
                        .args(["--phase", "memprobe", "--raw", rawp.to_str().unwrap(), "--w", &sub.w.to_string(), "--h", &sub.h.to_string(), "--arm", &ai.to_string(), "--mode", mode, "--stream", sp.to_str().unwrap()])
                        .output()
                        .unwrap();
                    assert!(o.status.success(), "memprobe failed: {}", String::from_utf8_lossy(&o.stderr));
                    let v: Vec<usize> = String::from_utf8_lossy(&o.stdout).split_whitespace().map(|x| x.parse().unwrap()).collect();
                    let _ = writeln!(csv, "{},{},{},{},{},{},{},{},{}", name, px, J_ARMS[ai].name, mode, rep, v[0], v[1], v[2], v[1] as i64 - v[0] as i64);
                }
            }
        }
    }
    std::fs::write(out.join("memory_raw.csv"), csv).unwrap();
    // static model-state sizes (estimate from the allocation formulas; the measured peaks are in memory_raw.csv)
    let mut ms = String::from("arm,bit_models_total,bit_model_bytes_total,mixer_weight_bytes,tail_bytes,extra_models_alloc,extra_models_bytes,extra_models_reachable_sign_node,size_independent_of_image_dims
");
    for arm in J_ARMS.iter() {
        let c = Coder::new(3, arm);
        let (alloc, bytes, reach) = c.extra_model_counts(3, arm);
        let cards: Vec<usize> = (0..2).map(|j| arm.extras.get(j).map_or(1, |e| e.card())).collect();
        let models = 3 * 27 * (4 * 96 + cards[0] + cards[1]);
        let _ = writeln!(ms, "{},{},{},{},{},{},{},{},true", arm.name, models, models * 4, 3 * 27 * 8 * 6 * 4, 3 * 9 * 8 * 4, alloc, bytes, reach);
    }
    std::fs::write(out.join("memory_static.csv"), ms).unwrap();
    println!("memory phase done");
}

fn main() {
    let a: Vec<String> = std::env::args().collect();
    let arg = |k: &str| a.iter().position(|x| x == k).map(|i| a[i + 1].clone());
    let phase = arg("--phase").expect("--phase");
    if phase == "memprobe" {
        phase_memprobe(&a);
        return;
    }
    let out = PathBuf::from(arg("--out").expect("--out"));
    std::fs::create_dir_all(&out).unwrap();
    let smoke = a.iter().any(|x| x == "--smoke");
    match phase.as_str() {
        "gates" => phase_gates(&out, smoke),
        "time" => phase_time(&out, smoke),
        "diag" => phase_diag(&out),
        "mem" => phase_mem(&out),
        "instr" => phase_instr(&out),
        _ => panic!("unknown phase"),
    }
}

/// Codec 2 (the control/baseline these research runs were frozen against); the production default is now codec 3.
fn encode(c: &nanoscope::canon::Canonical) -> Result<Vec<u8>, nanoscope::pkonfile::PkonError> {
    encode_with_codec(c, CODEC_X4P)
}
