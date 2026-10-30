//! PKON2 milestone-1 probe: measures what statistical structure real photographs expose.
//!
//! usage: probe [--data data] [--crop 1024] [--out historical]
//! Reads data/manifest.json (tools/fetch_dataset.py), writes historical/<stamp>-pkon2-m1/{report.md,results.json}.
//! Images are centre-cropped to --crop x --crop (exact pixels, no resampling) to keep iteration fast.
use nanoscope::canon::{first_mismatch, Canonical};
use nanoscope::codec::{Codec, PngCodec};
use nanoscope::pkon2::*;
use serde::Serialize;
use std::collections::HashSet;
use std::fmt::Write as _;
use std::path::PathBuf;

#[derive(Serialize, Clone)]
struct Run {
    cfg: String,
    coded: usize,
    h0: f64,
    hctx: f64,
    zero: f64,
    small: f64,
    enc_ms: f64,
    dec_ms: f64,
    exact: bool,
}

#[derive(Serialize, Clone)]
struct Bright {
    frac: f64,
    mean_abs: f64,
    h0_bps: f64,
}

#[derive(Serialize, Clone)]
struct Img {
    name: String,
    category: String,
    src: String,
    camera: String,
    orig_dims: String,
    dims: String,
    layout: String,
    samples: usize,
    raw_bytes: usize,
    png_bytes: usize,
    png_exact: bool,
    pix_h0_bps: f64,
    uniq_color_frac: f64,
    simple_block_frac: f64,
    repeat_frac: f64,
    edge_frac: f64,
    runs: Vec<Run>,
    bright: Vec<Bright>,
    dir_wins: Vec<f64>,
    dir_med_bps: f64,
    dir_oracle_bps: f64,
}

fn configs() -> Vec<Config> {
    let mut v: Vec<Config> = PREDS.iter().map(|&pred| Config { pred, color: Color::Raw, ctx: Ctx::None }).collect();
    for (color, ctx) in [
        (Color::Raw, Ctx::Act),
        (Color::SubG, Ctx::Act),
        (Color::Coupled, Ctx::Act),
        (Color::Raw, Ctx::ActX),
        (Color::SubG, Ctx::ActX),
        (Color::Coupled, Ctx::ActX),
    ] {
        v.push(Config { pred: Pred::Med, color, ctx });
    }
    v
}

fn utc_stamp() -> String {
    let secs = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs() as i64;
    let (days, rem) = (secs.div_euclid(86400), secs.rem_euclid(86400));
    let z = days + 719468;
    let era = z.div_euclid(146097);
    let doe = z.rem_euclid(146097);
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + (m <= 2) as i64;
    format!("{y:04}{m:02}{d:02}-{:02}{:02}{:02}Z", rem / 3600, rem % 3600 / 60, rem % 60)
}

fn h0_bits(hist: &[u64; 256]) -> f64 {
    let n: u64 = hist.iter().sum();
    if n == 0 {
        return 0.0;
    }
    hist.iter().filter(|&&v| v > 0).map(|&v| -(v as f64) * (v as f64 / n as f64).log2()).sum()
}

/// Cheap indicators for "graphic-like" content, on the cropped RGB(A) pixels.
fn content_indicators(img: &Canonical) -> (f64, f64, f64, f64) {
    let (w, h, ch) = (img.spec.width as usize, img.spec.height as usize, img.spec.layout.channels());
    let px = |i: usize| -> u32 {
        let mut v = 0u32;
        for c in 0..ch.min(4) {
            v = (v << 8) | img.data[i * ch + c] as u32;
        }
        v
    };
    let uniq: HashSet<u32> = (0..w * h).map(px).collect();
    let (mut simple, mut blocks) = (0usize, 0usize);
    for by in (0..h.saturating_sub(15)).step_by(16) {
        for bx in (0..w.saturating_sub(15)).step_by(16) {
            let mut s = HashSet::new();
            for y in by..by + 16 {
                for x in bx..bx + 16 {
                    s.insert(px(y * w + x));
                }
            }
            blocks += 1;
            simple += (s.len() <= 8) as usize;
        }
    }
    let pl = to_planes(img);
    let (mut rep, mut edge) = (0usize, 0usize);
    for y in 0..h {
        for x in 0..w {
            let i = y * w + x;
            let same_w = x > 0 && (0..pl.p.len()).all(|k| pl.p[k][i] == pl.p[k][i - 1]);
            let same_n = y > 0 && (0..pl.p.len()).all(|k| pl.p[k][i] == pl.p[k][i - w]);
            rep += (same_w || same_n) as usize;
            edge += (activity(neighbors(&pl.p[0], w, x, y)) > 40) as usize;
        }
    }
    let n = (w * h) as f64;
    (uniq.len() as f64 / n, simple as f64 / blocks.max(1) as f64, rep as f64 / n, edge as f64 / n)
}

/// Residual behaviour by brightness (MED, plane 0 = G or Y). Is prediction still useful in the dark?
fn brightness_bins(pl: &Planes) -> Vec<Bright> {
    let (w, h) = (pl.w, pl.h);
    let mut hist = vec![[0u64; 256]; 4];
    let mut sum_abs = [0u64; 4];
    for y in 0..h {
        for x in 0..w {
            let v = pl.p[0][y * w + x];
            let r = (v as i32 - predict(Pred::Med, neighbors(&pl.p[0], w, x, y))) as u8;
            let b = (v >> 6) as usize;
            hist[b][zigzag(r) as usize] += 1;
            sum_abs[b] += (r as i8).unsigned_abs() as u64;
        }
    }
    let n = (w * h) as f64;
    (0..4)
        .map(|b| {
            let c: u64 = hist[b].iter().sum();
            Bright {
                frac: c as f64 / n,
                mean_abs: if c > 0 { sum_abs[b] as f64 / c as f64 } else { 0.0 },
                h0_bps: if c > 0 { h0_bits(&hist[b]) / c as f64 } else { 0.0 },
            }
        })
        .collect()
}

/// Per 8x8 block (plane 0): which fixed predictor is cheapest under an Elias-gamma cost proxy, and how
/// much would a per-block oracle choice save (side info charged at log2(5) bits/block)? Screening only.
fn directional(pl: &Planes) -> (Vec<f64>, f64, f64) {
    let (w, h) = (pl.w, pl.h);
    let cost = |r: u8| -> f64 { (2 * (31 - (zigzag(r) as u32 + 1).leading_zeros()) + 1) as f64 };
    let (mut wins, mut blocks) = (vec![0usize; PREDS.len()], 0usize);
    let (mut med_total, mut oracle_total) = (0.0, 0.0);
    for by in (0..h.saturating_sub(7)).step_by(8) {
        for bx in (0..w.saturating_sub(7)).step_by(8) {
            let mut c = vec![0.0f64; PREDS.len()];
            for y in by..by + 8 {
                for x in bx..bx + 8 {
                    let nb = neighbors(&pl.p[0], w, x, y);
                    let v = pl.p[0][y * w + x] as i32;
                    for (j, &p) in PREDS.iter().enumerate() {
                        c[j] += cost((v - predict(p, nb)) as u8);
                    }
                }
            }
            let best = (0..PREDS.len()).min_by(|&a, &b| c[a].partial_cmp(&c[b]).unwrap()).unwrap();
            wins[best] += 1;
            blocks += 1;
            med_total += c[4];
            oracle_total += c[best] + (PREDS.len() as f64).log2();
        }
    }
    let samples = (blocks * 64).max(1) as f64;
    (wins.iter().map(|&v| v as f64 / blocks.max(1) as f64).collect(), med_total / samples, oracle_total / samples)
}

/// Order-0 entropy of the raw sample values (no prediction), bits/sample averaged over planes.
fn pixel_h0(pl: &Planes) -> f64 {
    mean(pl.p.iter().map(|p| {
        let mut h = [0u64; 256];
        p.iter().for_each(|&v| h[v as usize] += 1);
        h0_bits(&h) / p.len() as f64
    }))
}

fn mean(v: impl Iterator<Item = f64>) -> f64 {
    let (mut s, mut n) = (0.0, 0);
    for x in v {
        s += x;
        n += 1;
    }
    if n == 0 {
        f64::NAN
    } else {
        s / n as f64
    }
}

fn main() {
    let (mut data, mut crop, mut out) = (PathBuf::from("data"), 1024u32, PathBuf::from("historical"));
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--data" => data = args[{ i += 1; i }].clone().into(),
            "--crop" => crop = args[{ i += 1; i }].parse().expect("--crop N"),
            "--out" => out = args[{ i += 1; i }].clone().into(),
            a => panic!("unknown argument {a}\nusage: probe [--data data] [--crop 1024] [--out historical]"),
        }
        i += 1;
    }
    if cfg!(debug_assertions) {
        panic!("run with --release (timings and speed)");
    }
    let manifest: Vec<serde_json::Value> =
        serde_json::from_str(&std::fs::read_to_string(data.join("manifest.json")).expect("manifest.json; run tools/fetch_dataset.py"))
            .unwrap();
    let cfgs = configs();
    let mut imgs: Vec<Img> = vec![];
    for m in &manifest {
        let s = |k: &str| m[k].as_str().unwrap_or("").to_string();
        let name = s("name");
        let full = match Canonical::load(&data.join(s("file"))) {
            Ok(c) => c,
            Err(e) => {
                eprintln!("{name}: skipped, {e}");
                continue;
            }
        };
        if !supported(&full.spec) {
            eprintln!("{name}: skipped, {} unsupported by probe", full.spec.label());
            continue;
        }
        let img = center_crop(&full, crop);
        let png = PngCodec;
        let png_enc = png.encode(&img).unwrap();
        let png_exact = first_mismatch(&img, &png.decode(&png_enc, &img.spec).unwrap()).is_none();
        let pl = to_planes(&img);
        let (uniq, simple, rep, edge) = content_indicators(&img);
        let (wins, dmed, dor) = directional(&pl);
        let runs: Vec<Run> = cfgs
            .iter()
            .map(|&c| {
                let o = measure(&img, c);
                Run {
                    cfg: c.label(),
                    coded: o.coded_bytes,
                    h0: o.h0_bytes,
                    hctx: o.hctx_bytes,
                    zero: o.zero_frac,
                    small: o.small_frac,
                    enc_ms: o.enc_ms,
                    dec_ms: o.dec_ms,
                    exact: o.exact,
                }
            })
            .collect();
        let src = if s("source_format").contains("jpeg") { "jpeg" } else { "png" };
        let cat = s("category").trim_end_matches('2').to_string();
        eprintln!(
            "{name}: {}x{} {} med/raw/none {} B, med/coupled/actx {} B, png {} B, all exact: {}",
            img.spec.width,
            img.spec.height,
            img.spec.label(),
            runs[4].coded,
            runs[runs.len() - 1].coded,
            png_enc.len(),
            runs.iter().all(|r| r.exact)
        );
        imgs.push(Img {
            name,
            category: cat,
            src: src.into(),
            camera: s("camera"),
            orig_dims: format!("{}x{}", full.spec.width, full.spec.height),
            dims: format!("{}x{}", img.spec.width, img.spec.height),
            layout: img.spec.label(),
            samples: img.spec.width as usize * img.spec.height as usize * img.spec.layout.channels(),
            raw_bytes: img.data.len(),
            png_bytes: png_enc.len(),
            png_exact,
            pix_h0_bps: pixel_h0(&pl),
            uniq_color_frac: uniq,
            simple_block_frac: simple,
            repeat_frac: rep,
            edge_frac: edge,
            runs,
            bright: brightness_bins(&pl),
            dir_wins: wins,
            dir_med_bps: dmed,
            dir_oracle_bps: dor,
        });
    }

    let report = report(&imgs, &cfgs, crop);
    let dir = out.join(format!("{}-pkon2-m1", utc_stamp()));
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("report.md"), &report).unwrap();
    std::fs::write(dir.join("results.json"), serde_json::to_string_pretty(&imgs).unwrap()).unwrap();
    println!("{report}\nwritten to {}", dir.display());
}

fn bps(bytes: f64, im: &Img) -> f64 {
    bytes * 8.0 / im.samples as f64
}

fn report(imgs: &[Img], cfgs: &[Config], crop: u32) -> String {
    let mut s = String::new();
    let nruns = cfgs.len();
    let cname = |j: usize| cfgs[j].label();
    let groups: [(&str, Vec<&Img>); 2] = [
        ("camera JPEG photos (decoded)", imgs.iter().filter(|i| i.src == "jpeg").collect()),
        ("PNG (screenshots / other)", imgs.iter().filter(|i| i.src == "png").collect()),
    ];
    let all_exact = imgs.iter().all(|i| i.runs.iter().all(|r| r.exact) && i.png_exact);
    writeln!(s, "# PKON2 milestone 1 probe\n").unwrap();
    writeln!(s, "{} images, centre-cropped to at most {crop}x{crop} (exact pixels, no resampling). Metric: **bits per sample = coded bytes*8 / (w*h*channels)** (lower is better); coded bytes include the {HEADER_BYTES}-byte header and all adaptive-model learning cost (no side tables). Config label = predictor/colour/context. **All {} configs x images decoded bit-exactly: {all_exact}.**\n", imgs.len(), nruns).unwrap();

    writeln!(s, "## Dataset\n").unwrap();
    writeln!(s, "| image | cat | src | camera | orig | crop | uniq colours/px | simple 16x16 blocks | px repeating W/N | edge px (act>40) | raw B | png-rs B | best PKON2 B |").unwrap();
    writeln!(s, "|---|---|---|---|---|---|---:|---:|---:|---:|---:|---:|---:|").unwrap();
    for im in imgs {
        let best = im.runs.iter().map(|r| r.coded).min().unwrap();
        writeln!(s, "| {} | {} | {} | {} | {} | {} | {:.3} | {:.3} | {:.3} | {:.3} | {} | {} | {} |", im.name, im.category, im.src, if im.camera.is_empty() { "-" } else { &im.camera }, im.orig_dims, im.dims, im.uniq_color_frac, im.simple_block_frac, im.repeat_frac, im.edge_frac, im.raw_bytes, im.png_bytes, best).unwrap();
    }

    writeln!(s, "\n## Configs, mean bits/sample by source group\n").unwrap();
    writeln!(s, "Delta is vs `Med/Raw/None` (index 4): mean of per-image relative change; `better/worse` counts images. H0 = order-0 residual entropy, Hctx = context-conditional entropy (both static, no learning cost, optimistic); coded = real bytes.\n").unwrap();
    for (gname, g) in &groups {
        if g.is_empty() {
            continue;
        }
        writeln!(s, "### {gname} (n={})\n", g.len()).unwrap();
        writeln!(s, "| config | coded bps | H0 bps | Hctx bps | delta vs Med/Raw/None | better/worse | zero resid | \\|r\\|<=2 | enc ms | dec ms |").unwrap();
        writeln!(s, "|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|").unwrap();
        for j in 0..nruns {
            let d: Vec<f64> = g.iter().map(|i| i.runs[j].coded as f64 / i.runs[4].coded as f64 - 1.0).collect();
            writeln!(s, "| {} | {:.3} | {:.3} | {:.3} | {:+.2}% | {}/{} | {:.3} | {:.3} | {:.0} | {:.0} |", cname(j),
                mean(g.iter().map(|i| bps(i.runs[j].coded as f64, i))), mean(g.iter().map(|i| bps(i.runs[j].h0, i))), mean(g.iter().map(|i| bps(i.runs[j].hctx, i))),
                mean(d.iter().copied()) * 100.0, d.iter().filter(|&&x| x < -0.0005).count(), d.iter().filter(|&&x| x > 0.0005).count(),
                mean(g.iter().map(|i| i.runs[j].zero)), mean(g.iter().map(|i| i.runs[j].small)),
                mean(g.iter().map(|i| i.runs[j].enc_ms)), mean(g.iter().map(|i| i.runs[j].dec_ms))).unwrap();
        }
        let png_d: Vec<f64> = g.iter().map(|i| i.png_bytes as f64 / i.runs[nruns - 1].coded as f64 - 1.0).collect();
        writeln!(s, "\nReference: png-rs mean bps {:.3}; png-rs vs last config ({}): {:+.1}% bytes on average, PKON2-probe smaller on {}/{} images.\n", mean(g.iter().map(|i| bps(i.png_bytes as f64, i))), cname(nruns - 1), mean(png_d.iter().copied()) * 100.0, png_d.iter().filter(|&&x| x > 0.0).count(), g.len()).unwrap();
    }

    writeln!(s, "## Per image: coded bps for key configs\n").unwrap();
    let key = [4usize, 5, 7, 8, 10];
    write!(s, "| image | cat | pixel H0 (no prediction) |").unwrap();
    for &j in &key {
        write!(s, " {} |", cname(j)).unwrap();
    }
    writeln!(s, " png-rs | best config |\n|---|---|{}---:|---|", "---:|".repeat(key.len())).unwrap();
    for im in imgs {
        write!(s, "| {} | {} | {:.3} |", im.name, im.category, im.pix_h0_bps).unwrap();
        for &j in &key {
            write!(s, " {:.3} |", bps(im.runs[j].coded as f64, im)).unwrap();
        }
        let b = (0..nruns).min_by_key(|&j| im.runs[j].coded).unwrap();
        writeln!(s, " {:.3} | {} |", bps(im.png_bytes as f64, im), cname(b)).unwrap();
    }

    writeln!(s, "\n## Noise / lighting: MED residuals by brightness of plane 0 (G or Y), JPEG photos\n").unwrap();
    writeln!(s, "| brightness bin | share of px | mean abs resid | H0 bits/sample |\n|---|---:|---:|---:|").unwrap();
    let jp: Vec<&Img> = imgs.iter().filter(|i| i.src == "jpeg").collect();
    for (b, lab) in ["0-63", "64-127", "128-191", "192-255"].iter().enumerate() {
        // weight bins by pixels so images with an empty bin do not skew the mean
        let tot: f64 = jp.iter().map(|i| i.bright[b].frac).sum();
        let w = |f: &dyn Fn(&Bright) -> f64| jp.iter().map(|i| i.bright[b].frac * f(&i.bright[b])).sum::<f64>() / tot;
        writeln!(s, "| {lab} | {:.3} | {:.3} | {:.3} |", mean(jp.iter().map(|i| i.bright[b].frac)), w(&|b| b.mean_abs), w(&|b| b.h0_bps)).unwrap();
    }
    writeln!(s, "\nPer-image dark-bin (0-63) vs mid-bin (64-191) H0, images with >=2% dark pixels:\n").unwrap();
    writeln!(s, "| image | dark share | dark H0 | mid H0 |\n|---|---:|---:|---:|").unwrap();
    for im in imgs.iter().filter(|i| i.bright[0].frac >= 0.02) {
        let mid_w = im.bright[1].frac + im.bright[2].frac;
        let mid = (im.bright[1].frac * im.bright[1].h0_bps + im.bright[2].frac * im.bright[2].h0_bps) / mid_w.max(1e-9);
        writeln!(s, "| {} | {:.3} | {:.3} | {:.3} |", im.name, im.bright[0].frac, im.bright[0].h0_bps, mid).unwrap();
    }

    writeln!(s, "\n## Directional structure: per-8x8-block best fixed predictor (plane 0), Elias-gamma cost proxy\n").unwrap();
    writeln!(s, "Oracle chooses the cheapest of W,N,Avg,Grad,MED per block, charged log2(5)=2.32 bits/block side info. A proxy upper-bound screen, not a coded result.\n").unwrap();
    writeln!(s, "| image | cat | win W | win N | win Avg | win Grad | win MED | MED proxy bps | oracle proxy bps | oracle gain |\n|---|---|---:|---:|---:|---:|---:|---:|---:|---:|").unwrap();
    for im in imgs {
        writeln!(s, "| {} | {} | {:.2} | {:.2} | {:.2} | {:.2} | {:.2} | {:.3} | {:.3} | {:+.1}% |", im.name, im.category, im.dir_wins[0], im.dir_wins[1], im.dir_wins[2], im.dir_wins[3], im.dir_wins[4], im.dir_med_bps, im.dir_oracle_bps, (im.dir_oracle_bps / im.dir_med_bps - 1.0) * 100.0).unwrap();
    }

    writeln!(s, "\n## Learning cost: real bytes vs static entropy (config `{}`)\n", cname(nruns - 1)).unwrap();
    let (c, h) = imgs.iter().fold((0.0, 0.0), |a, i| (a.0 + i.runs[nruns - 1].coded as f64, a.1 + i.runs[nruns - 1].hctx));
    writeln!(s, "Sum over images: coded {c:.0} B vs Hctx {h:.0} B (coded/Hctx = {:.4}). Static Hctx ignores model-table cost, so the gap is the adaptive-learning overhead, not a gain.\n", c / h).unwrap();
    s
}
