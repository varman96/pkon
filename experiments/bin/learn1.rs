//! PKON2 relationship-learning experiment (stage runner). Research only: production X4 and the file format are untouched;
//! `pkonlearn` is a parameterised copy of X4 that is byte-identical at the default `Theta`.
//!
//!   cargo run --release --bin learn1 -- splits                      # fixed, hash-derived train/val/test split + dedup check
//!   cargo run --release --bin learn1 -- train --n 5 --out DIR       # sees ONLY the first n training images; writes criteria.md first
//!   cargo run --release --bin learn1 -- eval --out DIR              # sees ONLY the validation images; test split is never loaded
//!
//! What is learned: a model of how the cross-entropy of an image depends on the JOINT setting of five X4 constants
//! (`pkonlearn::PARAMS`). Order 1 (additive main effects) = "each parameter independently"; order 2 adds pairwise
//! interaction terms = "entanglement". Fit by ridge regression on measured (image, theta) cross-entropies.
use nanoscope::canon::Canonical;
use nanoscope::pkon2::center_crop;
use nanoscope::pkon2::Planes;
use nanoscope::pkonlearn::*;
use serde_json::json;
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

const CROP: u32 = 1024;
const SPLITS: &str = "results/learn1-splits.json";
const TEST_MIN: usize = 15;
const VAL_MIN: usize = 10;
const SAMPLE: usize = 100; // theta points measured per training image (default + one-at-a-time + random)
const OVERHEAD: usize = NP; // bytes to signal theta per file: one level index per parameter

fn sha(s: &str) -> String {
    Sha256::digest(s.as_bytes()).iter().map(|x| format!("{x:02x}")).collect()
}
fn group_of(name: &str) -> String {
    name.split('_').next().unwrap().trim_end_matches(|c: char| c.is_ascii_digit()).to_string()
}

struct Splits {
    train: Vec<String>, // ordered: stage n uses the first n
    val: Vec<String>,
    test: Vec<String>,
}
fn make_splits(names: &[String]) -> Splits {
    let mut groups: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for n in names {
        groups.entry(group_of(n)).or_default().push(n.clone());
    }
    let mut order: Vec<(String, Vec<String>)> = groups.into_iter().collect();
    order.sort_by_key(|(g, _)| sha(&format!("group:{g}")));
    let (mut test, mut val, mut train) = (vec![], vec![], vec![]);
    for (_, ns) in order {
        if test.len() < TEST_MIN {
            test.extend(ns);
        } else if val.len() < VAL_MIN {
            val.extend(ns);
        } else {
            train.extend(ns);
        }
    }
    train.sort_by_key(|n| sha(&format!("img:{n}")));
    Splits { train, val, test }
}

fn manifest() -> Vec<serde_json::Value> {
    serde_json::from_str(&std::fs::read_to_string("data/dataset50.json").expect("data/dataset50.json")).unwrap()
}
fn load_crop(m: &serde_json::Value) -> Canonical {
    let full = Canonical::load(&Path::new("data").join(m["file"].as_str().unwrap())).unwrap();
    center_crop(&full, CROP)
}

fn thumb(img: &Canonical) -> Vec<f64> {
    let (w, h, c) = (img.spec.width as usize, img.spec.height as usize, img.spec.layout.channels());
    let g = 16;
    let mut t = vec![0f64; g * g];
    for y in 0..h {
        for x in 0..w {
            let i = (y * w + x) * c;
            t[(y * g / h) * g + x * g / w] += (img.data[i] as f64 + img.data[i + 1] as f64 + img.data[i + 2] as f64) / 3.0;
        }
    }
    let n = (w * h / (g * g)) as f64;
    t.iter_mut().for_each(|v| *v /= n);
    let mean = t.iter().sum::<f64>() / t.len() as f64;
    let sd = (t.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / t.len() as f64).sqrt().max(1e-9);
    t.iter().map(|v| (v - mean) / sd).collect()
}

fn cmd_splits() {
    let man = manifest();
    let names: Vec<String> = man.iter().map(|m| m["name"].as_str().unwrap().to_string()).collect();
    let sp = make_splits(&names);
    let split_of = |n: &str| if sp.train.iter().any(|x| x == n) { "train" } else if sp.val.iter().any(|x| x == n) { "val" } else { "test" };
    // duplicate / near-duplicate screen on everything that is going to be used
    let crops: Vec<Canonical> = man.iter().map(load_crop).collect();
    let hashes: Vec<String> = crops.iter().map(|c| c.content_hash()).collect();
    let th: Vec<Vec<f64>> = crops.iter().map(thumb).collect();
    let mut pairs = vec![];
    for i in 0..names.len() {
        for j in i + 1..names.len() {
            let d = (th[i].iter().zip(&th[j]).map(|(a, b)| (a - b).powi(2)).sum::<f64>() / 256.0).sqrt();
            pairs.push((d, i, j));
        }
    }
    pairs.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
    let mut urls: Vec<&str> = man.iter().map(|m| m["url"].as_str().unwrap_or("")).collect();
    urls.sort();
    let dup_url = urls.windows(2).any(|w| w[0] == w[1] && !w[0].is_empty());
    let mut hs = hashes.clone();
    hs.sort();
    let dup_hash = hs.windows(2).any(|w| w[0] == w[1]);
    println!("images {} | duplicate crop hash: {dup_hash} | duplicate source url: {dup_url}", names.len());
    println!("nearest thumbnail pairs (RMS of z-scored 16x16 luma; 0 = identical, ~1.4 = unrelated):");
    let mut near = vec![];
    for (d, i, j) in pairs.iter().take(6) {
        println!("  {:.3}  {} ({}) ~ {} ({})", d, names[*i], split_of(&names[*i]), names[*j], split_of(&names[*j]));
        near.push(json!({"rms": d, "a": names[*i], "a_split": split_of(&names[*i]), "b": names[*j], "b_split": split_of(&names[*j])}));
    }
    let cross_min = pairs.iter().find(|(_, i, j)| split_of(&names[*i]) != split_of(&names[*j])).unwrap();
    println!("nearest CROSS-split pair: {:.3} {} ~ {}", cross_min.0, names[cross_min.1], names[cross_min.2]);
    let gsplit: BTreeMap<String, Vec<&str>> = names.iter().fold(BTreeMap::new(), |mut m, n| {
        m.entry(group_of(n)).or_default().push(split_of(n));
        m
    });
    let leak = gsplit.values().any(|v| v.iter().any(|s| *s != v[0]));
    println!("groups straddling splits: {leak}");
    println!("train pool {} (stage order): {:?}\nval {}: {:?}\ntest {}: {:?}", sp.train.len(), sp.train, sp.val.len(), sp.val, sp.test.len(), sp.test);
    assert!(!dup_hash && !dup_url && !leak);
    std::fs::create_dir_all("results").unwrap();
    let j = json!({"rule": "group = name before '_' without trailing digits; groups sorted by sha256('group:'+g); first groups fill test (>=15 imgs), then val (>=10), rest train pool; train pool ordered by sha256('img:'+name); stage n uses first n",
        "train_pool": sp.train, "val": sp.val, "test": sp.test, "nearest_pairs": near,
        "nearest_cross_split_rms": cross_min.0, "nearest_cross_split": [names[cross_min.1], names[cross_min.2]], "dup_crop_hash": dup_hash, "dup_url": dup_url});
    std::fs::write(SPLITS, serde_json::to_string_pretty(&j).unwrap()).unwrap();
}

fn read_splits() -> Splits {
    let j: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(SPLITS).expect("run `learn1 splits` first")).unwrap();
    let v = |k: &str| j[k].as_array().unwrap().iter().map(|s| s.as_str().unwrap().to_string()).collect::<Vec<_>>();
    Splits { train: v("train_pool"), val: v("val"), test: v("test") }
}

struct Img {
    name: String,
    work: Planes,
    orig: Vec<Vec<u8>>,
}
fn load_set(names: &[String]) -> Vec<Img> {
    let man = manifest();
    names
        .iter()
        .map(|n| {
            let m = man.iter().find(|m| m["name"].as_str() == Some(n)).unwrap();
            let (work, orig) = prepared(&load_crop(m));
            Img { name: n.clone(), work, orig }
        })
        .collect()
}

/// (cross-entropy bits, actual coded bytes incl. 11-byte header) for every (image, grid index) task, in parallel.
fn measure(imgs: &[Img], idxs: &[usize]) -> Vec<Vec<(f64, usize)>> {
    let tasks: Vec<(usize, usize)> = (0..imgs.len()).flat_map(|i| (0..idxs.len()).map(move |g| (i, g))).collect();
    let next = AtomicUsize::new(0);
    let out = std::sync::Mutex::new(vec![vec![(0.0, 0usize); idxs.len()]; imgs.len()]);
    let nt = std::thread::available_parallelism().map(|n| n.get()).unwrap_or(4);
    std::thread::scope(|s| {
        for _ in 0..nt {
            s.spawn(|| loop {
                let t = next.fetch_add(1, Ordering::Relaxed);
                if t >= tasks.len() {
                    break;
                }
                let (i, g) = tasks[t];
                let e = encode(&imgs[i].work, Theta::from_levels(&grid_levels(idxs[g])));
                out.lock().unwrap()[i][g] = (e.ce_bits, e.coded_bytes);
            });
        }
    });
    out.into_inner().unwrap()
}

// ------------------------------------------------------------------ the learner

fn pair_list() -> Vec<(usize, usize)> {
    let mut v = vec![];
    for a in 0..NP {
        for b in a + 1..NP {
            v.push((a, b));
        }
    }
    v
}
fn feat_dim(order: usize) -> usize {
    let main: usize = PARAMS.iter().map(|p| p.levels.len()).sum();
    let pair: usize = pair_list().iter().map(|&(a, b)| PARAMS[a].levels.len() * PARAMS[b].levels.len()).sum();
    1 + main + if order >= 2 { pair } else { 0 }
}
fn features(l: &[usize; NP], order: usize) -> Vec<f64> {
    let mut f = vec![0.0; feat_dim(order)];
    f[0] = 1.0;
    let mut off = 1;
    let mut moff = [0usize; NP];
    for p in 0..NP {
        moff[p] = off;
        f[off + l[p]] = 1.0;
        off += PARAMS[p].levels.len();
    }
    if order >= 2 {
        for (a, b) in pair_list() {
            f[off + l[a] * PARAMS[b].levels.len() + l[b]] = 1.0;
            off += PARAMS[a].levels.len() * PARAMS[b].levels.len();
        }
    }
    f
}
/// Weighted ridge regression (intercept unpenalised). Rows are (features, y, weight).
fn ridge(rows: &[(Vec<f64>, f64, f64)], lambda: f64) -> Vec<f64> {
    let d = rows[0].0.len();
    let mut a = vec![vec![0.0; d + 1]; d];
    for (f, y, w) in rows {
        for i in 0..d {
            if f[i] == 0.0 {
                continue;
            }
            for j in 0..d {
                a[i][j] += w * f[i] * f[j];
            }
            a[i][d] += w * f[i] * y;
        }
    }
    for i in 1..d {
        a[i][i] += lambda;
    }
    a[0][0] += 1e-9;
    for c in 0..d {
        let piv = (c..d).max_by(|&x, &y| a[x][c].abs().partial_cmp(&a[y][c].abs()).unwrap()).unwrap();
        a.swap(c, piv);
        let pv = a[c][c];
        for j in c..=d {
            a[c][j] /= pv;
        }
        for r in 0..d {
            if r != c && a[r][c] != 0.0 {
                let fct = a[r][c];
                for j in c..=d {
                    a[r][j] -= fct * a[c][j];
                }
            }
        }
    }
    (0..d).map(|i| a[i][d]).collect()
}
fn predict_y(beta: &[f64], l: &[usize; NP], order: usize) -> f64 {
    features(l, order).iter().zip(beta).map(|(f, b)| f * b).sum()
}

struct Fit {
    order: usize,
    lambda: f64,
    cv_mse: f64,
    beta: Vec<f64>,
    surface: Vec<f64>, // predicted y over the whole grid
}
fn fit_order(rows: &[(usize, Vec<f64>, f64, f64)], order: usize) -> Fit {
    // rows: (grid idx, features, y, weight). Cross-validate by holding out whole theta points (all images at once).
    let mut gidx: Vec<usize> = rows.iter().map(|r| r.0).collect();
    gidx.sort();
    gidx.dedup();
    let fold_of = |g: usize| (sha(&format!("fold:{g}")).as_bytes()[0] as usize) % 5;
    let mut best = (f64::INFINITY, 0.0);
    for &lam in &[0.01, 0.1, 1.0, 10.0] {
        let (mut se, mut sw) = (0.0, 0.0);
        for f in 0..5 {
            let tr: Vec<_> = rows.iter().filter(|r| fold_of(r.0) != f).map(|r| (r.1.clone(), r.2, r.3)).collect();
            let beta = ridge(&tr, lam);
            for r in rows.iter().filter(|r| fold_of(r.0) == f) {
                let p: f64 = r.1.iter().zip(&beta).map(|(a, b)| a * b).sum();
                se += r.3 * (p - r.2).powi(2);
                sw += r.3;
            }
        }
        if se / sw < best.0 {
            best = (se / sw, lam);
        }
    }
    let all: Vec<_> = rows.iter().map(|r| (r.1.clone(), r.2, r.3)).collect();
    let beta = ridge(&all, best.1);
    let surface = (0..GRID).map(|g| predict_y(&beta, &grid_levels(g), order)).collect();
    Fit { order, lambda: best.1, cv_mse: best.0, beta, surface }
}
fn argmin(s: &[f64]) -> usize {
    (0..s.len()).min_by(|&a, &b| s[a].partial_cmp(&s[b]).unwrap()).unwrap()
}

/// Main-effect and pairwise-interaction strength (RMS, in ln-cost units) of any surface over the full grid, plus the
/// share of the surface variance that is not additive.
struct Entangle {
    main: Vec<f64>,
    pair: Vec<((usize, usize), f64)>,
    nonadditive_var_share: f64,
}
fn entangle(s: &[f64]) -> Entangle {
    let n = s.len() as f64;
    let m = s.iter().sum::<f64>() / n;
    let marg = |ps: &[usize]| -> BTreeMap<Vec<usize>, f64> {
        let mut acc: BTreeMap<Vec<usize>, (f64, f64)> = BTreeMap::new();
        for g in 0..GRID {
            let l = grid_levels(g);
            let e = acc.entry(ps.iter().map(|&p| l[p]).collect()).or_default();
            e.0 += s[g];
            e.1 += 1.0;
        }
        acc.into_iter().map(|(k, (a, c))| (k, a / c)).collect()
    };
    let rms = |v: Vec<f64>| (v.iter().map(|x| x * x).sum::<f64>() / v.len() as f64).sqrt();
    let mains: Vec<BTreeMap<Vec<usize>, f64>> = (0..NP).map(|p| marg(&[p])).collect();
    let main: Vec<f64> = mains.iter().map(|mp| rms(mp.values().map(|v| v - m).collect())).collect();
    let pair = pair_list()
        .into_iter()
        .map(|(a, b)| {
            let mab = marg(&[a, b]);
            let v: Vec<f64> = mab.iter().map(|(k, v)| v - mains[a][&vec![k[0]]] - mains[b][&vec![k[1]]] + m).collect();
            ((a, b), rms(v))
        })
        .collect();
    let var = s.iter().map(|v| (v - m).powi(2)).sum::<f64>() / n;
    let resid = (0..GRID)
        .map(|g| {
            let l = grid_levels(g);
            let add = m + (0..NP).map(|p| mains[p][&vec![l[p]]] - m).sum::<f64>();
            (s[g] - add).powi(2)
        })
        .sum::<f64>()
        / n;
    Entangle { main, pair, nonadditive_var_share: if var > 0.0 { resid / var } else { 0.0 } }
}
fn spearman(a: &[f64], b: &[f64]) -> f64 {
    let rank = |v: &[f64]| -> Vec<f64> {
        let mut o: Vec<usize> = (0..v.len()).collect();
        o.sort_by(|&x, &y| v[x].partial_cmp(&v[y]).unwrap());
        let mut r = vec![0.0; v.len()];
        for (k, &i) in o.iter().enumerate() {
            r[i] = k as f64;
        }
        r
    };
    let (ra, rb) = (rank(a), rank(b));
    let n = a.len() as f64;
    let (ma, mb) = (ra.iter().sum::<f64>() / n, rb.iter().sum::<f64>() / n);
    let cov: f64 = ra.iter().zip(&rb).map(|(x, y)| (x - ma) * (y - mb)).sum();
    let (va, vb): (f64, f64) = (ra.iter().map(|x| (x - ma).powi(2)).sum(), rb.iter().map(|y| (y - mb).powi(2)).sum());
    cov / (va * vb).sqrt()
}

fn theta_json(g: usize) -> serde_json::Value {
    let l = grid_levels(g);
    let mut m = serde_json::Map::new();
    for p in 0..NP {
        m.insert(PARAMS[p].name.into(), json!(PARAMS[p].levels[l[p]]));
    }
    serde_json::Value::Object(m)
}
fn theta_str(g: usize) -> String {
    let l = grid_levels(g);
    (0..NP).map(|p| format!("{}={}", PARAMS[p].name, PARAMS[p].levels[l[p]])).collect::<Vec<_>>().join(" ")
}

const CRITERIA: &str = "# Success criteria (frozen BEFORE any validation image is encoded)

Stage = N training images (N=5 first). The learned candidate is chosen on TRAINING data only: the order-2 (pairwise
interaction) model if its 5-fold held-out-theta CV error is < 0.90 x the order-1 (additive) model's, else order-1.
Candidates evaluated on the fixed validation set (10 images, never used for fitting or selection):
baseline (unchanged X4, theta = defaults), indep (coordinate-wise tuning: each parameter tuned alone, others at
default), o1 (argmin of the additive model), o2 (argmin of the pairwise model), bestmeasured (best theta among the
points actually measured on the training images). Every non-baseline candidate is charged 5 bytes per file to signal
theta (one level index per parameter). Sizes are real coded bytes (stream + 11-byte header), not cross-entropy.

Stage SUCCESS requires ALL of, for the selected candidate:
- C1 every validation image decodes byte-exactly (all candidates; any failure aborts the stage).
- C2 validation aggregate bytes (incl. signalling) <= 99.70% of baseline (>= 0.30% smaller).
- C3 at most 3 of 10 validation images are larger than baseline by > 0.30%, and none by > 1.5%.
- C4 summed single-thread encode time and decode time (best of 2) each within +10% of baseline.
Relationship CLAIM (reported separately; the stage can succeed without it):
- C5 the selected candidate is o2 AND its validation bytes beat BOTH indep and o1 by >= 0.10% of baseline.
- C6 (entanglement evidence, diagnostic) on the validation full grid, o2 has lower held-out RMSE than o1 by >= 10%.
Sanity (diagnostic): |cross-entropy/8 - coded stream bytes| < 0.05% on every encode.

If the stage fails: report why; do not enlarge the data or the model automatically.
Continuation to 10 images is considered only if C1-C4 pass.
";

fn cmd_train(n: usize, out: &Path) {
    let sp = read_splits();
    std::fs::create_dir_all(out).unwrap();
    std::fs::write(out.join("criteria.md"), CRITERIA).unwrap();
    let names: Vec<String> = sp.train[..n].to_vec();
    println!("training images ({n}): {names:?}");
    let imgs = load_set(&names);

    // measured points: default, every one-at-a-time move, then a deterministic pseudo-random fill to SAMPLE
    let d0 = default_index();
    let mut pts = vec![d0];
    for p in 0..NP {
        for lv in 0..PARAMS[p].levels.len() {
            if lv != PARAMS[p].default {
                let mut l = grid_levels(d0);
                l[p] = lv;
                pts.push(grid_index(&l));
            }
        }
    }
    let mut rest: Vec<usize> = (0..GRID).filter(|g| !pts.contains(g)).collect();
    rest.sort_by_key(|g| sha(&format!("sample:{g}")));
    pts.extend(rest.into_iter().take(SAMPLE - pts.len()));
    let t0 = std::time::Instant::now();
    let m = measure(&imgs, &pts);
    println!("measured {} points x {} images in {:.1}s", pts.len(), n, t0.elapsed().as_secs_f64());

    let base: Vec<f64> = (0..n).map(|i| m[i][0].0).collect();
    let wsum: f64 = base.iter().sum();
    // CSV of the raw training measurements
    let mut csv = String::from("image,grid_idx,theta,ce_bits,coded_bytes,ce_bytes_gap_pct\n");
    let mut rows: Vec<(usize, Vec<f64>, f64, f64)> = vec![];
    let mut max_gap: f64 = 0.0;
    for i in 0..n {
        for (k, &g) in pts.iter().enumerate() {
            let (ce, by) = m[i][k];
            let gap = ((ce / 8.0) - (by - 11) as f64) / (by - 11) as f64 * 100.0;
            max_gap = max_gap.max(gap.abs());
            csv += &format!("{},{},\"{}\",{:.1},{},{:.4}\n", imgs[i].name, g, theta_str(g), ce, by, gap);
            rows.push((g, vec![], (ce / base[i]).ln(), base[i] / wsum * n as f64));
        }
    }
    std::fs::write(out.join("train_measurements.csv"), csv).unwrap();
    let fits: Vec<Fit> = [1usize, 2]
        .iter()
        .map(|&o| {
            let r: Vec<_> = rows.iter().map(|r| (r.0, features(&grid_levels(r.0), o), r.2, r.3)).collect();
            fit_order(&r, o)
        })
        .collect();

    // training-set mean y per measured point (bytes-weighted)
    let ymean = |k: usize| (0..n).map(|i| base[i] / wsum * (m[i][k].0 / base[i]).ln()).sum::<f64>();
    let ymeas: Vec<(usize, f64)> = (0..pts.len()).map(|k| (pts[k], ymean(k))).collect();
    // independent tuning: each parameter alone from the defaults
    let mut l_ind = grid_levels(d0);
    for p in 0..NP {
        let mut best = (0.0, PARAMS[p].default);
        for (g, y) in &ymeas {
            let l = grid_levels(*g);
            if (0..NP).all(|q| q == p || l[q] == PARAMS[q].default) && *y < best.0 {
                best = (*y, l[p]);
            }
        }
        l_ind[p] = best.1;
    }
    let g_ind = grid_index(&l_ind);
    let g_best = ymeas.iter().min_by(|a, b| a.1.partial_cmp(&b.1).unwrap()).unwrap().0;
    let g_o1 = argmin(&fits[0].surface);
    let g_o2 = argmin(&fits[1].surface);
    let selected = if fits[1].cv_mse < 0.90 * fits[0].cv_mse { "o2" } else { "o1" };
    let ent2 = entangle(&fits[1].surface);
    let ent1 = entangle(&fits[0].surface);
    let names_p: Vec<&str> = PARAMS.iter().map(|p| p.name).collect();
    println!("CV (held-out theta) weighted MSE: order1 {:.3e} (lambda {}), order2 {:.3e} (lambda {}) -> selected {selected}", fits[0].cv_mse, fits[0].lambda, fits[1].cv_mse, fits[1].lambda);
    println!("max |CE/8 - stream bytes| = {max_gap:.4}%");
    for (nm, g) in [("indep", g_ind), ("o1", g_o1), ("o2", g_o2), ("bestmeasured", g_best)] {
        let measured = pts.iter().position(|&x| x == g).map(|k| ymeas[k].1);
        println!("  {nm:13} {}  pred_o2 {:+.3}%  measured-on-train {}", theta_str(g), fits[1].surface[g] * 100.0, measured.map(|v| format!("{:+.3}%", v * 100.0)).unwrap_or("(not measured)".into()));
    }
    let model = json!({
        "n_train": n, "train_images": names, "points_measured": pts.len(),
        "cv": {"order1_mse": fits[0].cv_mse, "order1_lambda": fits[0].lambda, "order2_mse": fits[1].cv_mse, "order2_lambda": fits[1].lambda},
        "selected": selected, "max_ce_vs_bytes_gap_pct": max_gap,
        "candidates": {"indep": {"grid_idx": g_ind, "theta": theta_json(g_ind)}, "o1": {"grid_idx": g_o1, "theta": theta_json(g_o1)},
                       "o2": {"grid_idx": g_o2, "theta": theta_json(g_o2)}, "bestmeasured": {"grid_idx": g_best, "theta": theta_json(g_best)}},
        "o1_surface": fits[0].surface, "o2_surface": fits[1].surface,
        "train_entanglement_o2": {"main_rms": ent2.main, "pair_rms": ent2.pair.iter().map(|((a, b), v)| json!({"pair": format!("{}x{}", names_p[*a], names_p[*b]), "rms": v})).collect::<Vec<_>>(), "nonadditive_var_share": ent2.nonadditive_var_share},
        "train_entanglement_o1_nonadditive_var_share": ent1.nonadditive_var_share,
    });
    std::fs::write(out.join("model.json"), serde_json::to_string_pretty(&model).unwrap()).unwrap();
    println!("wrote {}", out.display());
}

fn cmd_eval(out: &Path) {
    let sp = read_splits();
    let model: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(out.join("model.json")).expect("run train first")).unwrap();
    let imgs = load_set(&sp.val);
    let nv = imgs.len();
    let cand_names = ["baseline", "indep", "o1", "o2", "bestmeasured"];
    let d0 = default_index();
    let cand_g: Vec<usize> = cand_names.iter().map(|c| if *c == "baseline" { d0 } else { model["candidates"][*c]["grid_idx"].as_u64().unwrap() as usize }).collect();
    let selected = model["selected"].as_str().unwrap().to_string();

    // 1. real encode + decode of every candidate (single thread, best of 2 for time)
    let mut cres: Vec<Vec<(usize, f64, f64, bool, f64)>> = vec![]; // [cand][img] = (bytes, enc_ms, dec_ms, exact, ce_gap_pct)
    for (c, &g) in cand_g.iter().enumerate() {
        let th = Theta::from_levels(&grid_levels(g));
        let mut v = vec![];
        for im in &imgs {
            let e1 = encode(&im.work, th);
            let e2 = encode(&im.work, th);
            let (ok, d1) = decode_check(&e1.stream, th, im.work.w, im.work.h, &im.orig);
            let (_, d2) = decode_check(&e1.stream, th, im.work.w, im.work.h, &im.orig);
            assert!(ok, "C1 FAILED: {} not byte-exact for {}", im.name, cand_names[c]);
            let payload = e1.stream.len() as f64;
            v.push((e1.coded_bytes + if c == 0 { 0 } else { OVERHEAD }, e1.ms.min(e2.ms), d1.min(d2), ok, (e1.ce_bits / 8.0 - payload) / payload * 100.0));
        }
        println!("candidate {:13} {}", cand_names[c], theta_str(g));
        cres.push(v);
    }
    let total = |c: usize| cres[c].iter().map(|r| r.0 as f64).sum::<f64>();
    let ttime = |c: usize, k: usize| cres[c].iter().map(|r| if k == 0 { r.1 } else { r.2 }).sum::<f64>();
    let max_gap = cres.iter().flatten().map(|r| r.4.abs()).fold(0.0, f64::max);

    // 2. full-grid measurement on validation (evaluation of the learned models only; never fed back)
    let all: Vec<usize> = (0..GRID).collect();
    let t0 = std::time::Instant::now();
    let fg = measure(&imgs, &all);
    println!("validation full grid {} x {} in {:.0}s", GRID, nv, t0.elapsed().as_secs_f64());
    let basece: Vec<f64> = (0..nv).map(|i| fg[i][d0].0).collect();
    let bsum: f64 = basece.iter().sum();
    let yval: Vec<f64> = (0..GRID).map(|g| (0..nv).map(|i| basece[i] / bsum * (fg[i][g].0 / basece[i]).ln()).sum()).collect();
    let s1: Vec<f64> = serde_json::from_value(model["o1_surface"].clone()).unwrap();
    let s2: Vec<f64> = serde_json::from_value(model["o2_surface"].clone()).unwrap();
    let rmse = |s: &[f64]| ((0..GRID).map(|g| (s[g] - yval[g]).powi(2)).sum::<f64>() / GRID as f64).sqrt();
    let (r1, r2, r0) = (rmse(&s1), rmse(&s2), rmse(&vec![0.0; GRID]));
    let (sp1, sp2) = (spearman(&s1, &yval), spearman(&s2, &yval));
    let oracle_global = argmin(&yval);
    let oracle_per_img: f64 = (0..nv).map(|i| (0..GRID).map(|g| fg[i][g].1).min().unwrap() as f64).sum();
    let base_total = total(0);
    let regret = |s: &[f64]| (yval[argmin(s)] - yval[oracle_global]) * 100.0;
    let ev = entangle(&yval);
    let ep = entangle(&s2);
    let names_p: Vec<&str> = PARAMS.iter().map(|p| p.name).collect();
    let pa: Vec<f64> = ep.pair.iter().map(|x| x.1).collect();
    let pb: Vec<f64> = ev.pair.iter().map(|x| x.1).collect();
    let pair_rank_rho = spearman(&pa, &pb);

    // 3. criteria
    let sel_i = cand_names.iter().position(|c| *c == selected).unwrap();
    let rel = |c: usize| total(c) / base_total - 1.0;
    let c1 = true;
    let c2 = total(sel_i) <= 0.9970 * base_total;
    let regs: Vec<f64> = (0..nv).map(|i| cres[sel_i][i].0 as f64 / cres[0][i].0 as f64 - 1.0).collect();
    let c3 = regs.iter().filter(|&&r| r > 0.003).count() <= 3 && regs.iter().all(|&r| r <= 0.015);
    let c4 = ttime(sel_i, 0) <= 1.10 * ttime(0, 0) && ttime(sel_i, 1) <= 1.10 * ttime(0, 1);
    let c5 = selected == "o2" && total(3) <= total(1) - 0.001 * base_total && total(3) <= total(2) - 0.001 * base_total;
    let c6 = r2 <= 0.90 * r1;
    let stage = c1 && c2 && c3 && c4;

    let mut md = format!("# Stage report: {} training images -> validation ({} images)\n\n", model["n_train"], nv);
    md += &format!("Training images: {}\n\nSelected on training CV: **{selected}** (CV MSE order1 {:.3e}, order2 {:.3e}).\n\n", model["train_images"], model["cv"]["order1_mse"].as_f64().unwrap(), model["cv"]["order2_mse"].as_f64().unwrap());
    md += "## Candidates on the validation set (real coded bytes; non-baseline includes 5 B signalling per file)\n\n| candidate | theta | total bytes | vs baseline | enc ms (sum, best of 2) | dec ms (sum) | exact |\n|---|---|---|---|---|---|---|\n";
    for c in 0..cand_names.len() {
        md += &format!("| {} | {} | {} | {:+.3}% | {:.0} | {:.0} | {} |\n", cand_names[c], theta_str(cand_g[c]), total(c) as u64, rel(c) * 100.0, ttime(c, 0), ttime(c, 1), cres[c].iter().all(|r| r.3));
    }
    md += &format!("\n## Per-image change for the selected candidate ({selected})\n\n| image | baseline B | {selected} B | change |\n|---|---|---|---|\n");
    for i in 0..nv {
        md += &format!("| {} | {} | {} | {:+.3}% |\n", imgs[i].name, cres[0][i].0, cres[sel_i][i].0, regs[i] * 100.0);
    }
    md += "\nPer-image change for the other candidates (percent vs baseline):\n\n| image | indep | o1 | o2 | bestmeasured |\n|---|---|---|---|---|\n";
    for i in 0..nv {
        md += &format!("| {} | {:+.3} | {:+.3} | {:+.3} | {:+.3} |\n", imgs[i].name, (cres[1][i].0 as f64 / cres[0][i].0 as f64 - 1.0) * 100.0, (cres[2][i].0 as f64 / cres[0][i].0 as f64 - 1.0) * 100.0, (cres[3][i].0 as f64 / cres[0][i].0 as f64 - 1.0) * 100.0, (cres[4][i].0 as f64 / cres[0][i].0 as f64 - 1.0) * 100.0);
    }
    md += &format!("\n## Held-out prediction on the validation full grid ({} theta points x {} images; evaluation only)\n\n| model | RMSE of predicted ln cost-ratio | Spearman vs actual | regret of its argmin vs best global theta |\n|---|---|---|---|\n| constant 0 | {:.5} | - | - |\n| order 1 (additive) | {:.5} | {:.3} | {:.3}% |\n| order 2 (pairwise) | {:.5} | {:.3} | {:.3}% |\n", GRID, nv, r0, r1, sp1, regret(&s1), r2, sp2, regret(&s2));
    md += &format!("\nDiagnostic headroom (peeks at validation, NOT a candidate): best single global theta on the grid = {} at {:+.3}%; per-image oracle theta (needs {} trial encodes per image) = {:+.3}% before signalling.\n", theta_str(oracle_global), yval[oracle_global] * 100.0, GRID, (oracle_per_img / (0..nv).map(|i| fg[i][d0].1 as f64).sum::<f64>() - 1.0) * 100.0);
    md += &format!("\n## Entanglement\n\nShare of validation-surface variance that is NOT additive: actual {:.1}%, model-o2-predicted {:.1}% (train-fitted order-2 model predicts {:.1}% of its own surface).\n\n| pair | interaction RMS (actual, val) | interaction RMS (o2 model from train) |\n|---|---|---|\n", ev.nonadditive_var_share * 100.0, ep.nonadditive_var_share * 100.0, ep.nonadditive_var_share * 100.0);
    for (k, ((a, b), v)) in ev.pair.iter().enumerate() {
        md += &format!("| {} x {} | {:.5} | {:.5} |\n", names_p[*a], names_p[*b], v, ep.pair[k].1);
    }
    md += &format!("\nMain-effect RMS (actual val): {}. Spearman agreement of the pair-interaction ranking (model vs actual): {:.2} (n=10 pairs).\n", names_p.iter().zip(&ev.main).map(|(n, v)| format!("{n} {v:.5}")).collect::<Vec<_>>().join(", "), pair_rank_rho);
    md += &format!("\nSanity: max |CE/8 - stream bytes| over candidate encodes = {:.4}%.\n", max_gap);
    md += &format!("\n## Criteria (from criteria.md)\n\n- C1 byte-exact: PASS\n- C2 >=0.30% smaller ({selected}: {:+.3}%): {}\n- C3 regressions ({} images >+0.30%, worst {:+.3}%): {}\n- C4 time (enc {:+.1}%, dec {:+.1}%): {}\n- C5 relationship claim (o2 beats indep by {:+.3}% and o1 by {:+.3}% of baseline; selected={selected}): {}\n- C6 held-out RMSE o2/o1 = {:.2}: {}\n\n**Stage verdict: {}**\n", rel(sel_i) * 100.0, pf(c2), regs.iter().filter(|&&r| r > 0.003).count(), regs.iter().cloned().fold(f64::MIN, f64::max) * 100.0, pf(c3), (ttime(sel_i, 0) / ttime(0, 0) - 1.0) * 100.0, (ttime(sel_i, 1) / ttime(0, 1) - 1.0) * 100.0, pf(c4), (total(1) - total(3)) / base_total * 100.0, (total(2) - total(3)) / base_total * 100.0, pf(c5), r2 / r1, pf(c6), if stage { "SUCCESS" } else { "FAIL" });
    std::fs::write(out.join("report.md"), &md).unwrap();
    let mut csv = String::from("image,grid_idx,theta,ce_bits,coded_bytes\n");
    for i in 0..nv {
        for g in 0..GRID {
            csv += &format!("{},{},\"{}\",{:.1},{}\n", imgs[i].name, g, theta_str(g), fg[i][g].0, fg[i][g].1);
        }
    }
    std::fs::write(out.join("val_fullgrid.csv"), csv).unwrap();
    println!("{md}");
}
fn pf(b: bool) -> &'static str {
    if b { "PASS" } else { "FAIL" }
}

fn main() {
    let a: Vec<String> = std::env::args().collect();
    let arg = |k: &str| a.iter().position(|x| x == k).map(|i| a[i + 1].clone());
    match a.get(1).map(|s| s.as_str()) {
        Some("splits") => cmd_splits(),
        Some("train") => cmd_train(arg("--n").expect("--n").parse().unwrap(), &PathBuf::from(arg("--out").expect("--out"))),
        Some("eval") => cmd_eval(&PathBuf::from(arg("--out").expect("--out"))),
        _ => eprintln!("usage: learn1 splits | train --n N --out DIR | eval --out DIR"),
    }
}
