# OR-128 tree: signal / entropy / occupancy diagnostic

Measurement only. Tree = frozen `Stage3Rule::Or128Tree` (128 leaves). No stage, tree, split or default was touched. Only new files: `src/bin/diag_or128_signal.rs` and this directory.
Data: all 25 corpus images, grouped by the existing `results/20261029-lisa-or128-tree/split.json` (train 13 / val 4 / test 8). Samples per split: train 244,520,800, val 74,699,104, test 115,444,116.
Run: `diag_or128_signal results/20261028-praw-bench1/dataset_manifest.json results/20261029-lisa-or128-tree/split.json <this dir>`. Residuals and leaf ids come from the production path (Stage 1 -> 2 -> `classify_stage3_with(Or128Tree)`).

## Representation (all entropies)
Zigzag residual v, x = v+1, e = bit length of x, m1 = bit below the leading one. Modelled symbol s = (e, m1), which is exactly what Stage 4 adapts. The remaining max(e-2,0) bits are raw (p = 1/2).
bits/residual = H(s | context) + mean(max(e-2,0)). Static plug-in entropy, computed per image (Stage 4 models are fresh per image), then summed over images in a split and divided by the split's sample count.
Plug-in bias is negligible: at most about 128*67/(2*N*ln2) = 0.001 bit for N ~ 10^7. The alphabet is 68 symbols, so there is no sparsity problem as there would be with the raw 16-bit value.

## 1. Signaling ratio
The artifacts do not define this statistic, so I fixed the definition below. If you meant something else, tell me which. I did not substitute another statistic.

SR = I(R; leaf) / H(R) = (H(R) − H(R | leaf)) / H(R)
- Numerator: bits/residual saved by knowing the tree leaf (the leaf is a function of the W, N, NW residuals only).
- Denominator: order-0 entropy of the same Stage 1 residuals.
- Per-image fit, summed over a split.

| split | n | H(R) order-0 | H(R\|leaf) | **SR, total-residual denominator** | SR, modelled symbol only (raw bits excluded from the denominator) |
|---|---|---|---|---|---|
| train | 244,520,800 | 9.5898 | 9.0483 | **5.65%** | 13.31% |
| val | 74,699,104 | 9.2969 | 8.6460 | **7.00%** | 16.54% |
| test | 115,444,116 | 8.3517 | 7.8004 | **6.60%** | 13.71% |

Per image (per_image.csv): total-denominator SR ranges 0.9% to 14.6%. Modelled-only SR exceeds 20% on 4 of 25 images (bmpcc4k_person 21.4%, pentax_k1ii_station 22.6%, sigma_fp_bike8 21.4%, leica_cl_chateau 35.2%) and exceeds 30% on 1 (leica_cl_chateau).

Out-of-sample variant: per-leaf distributions fitted on pooled TRAIN, add-1/2 smoothing, evaluated on each split, against pooled-TRAIN order-0:

| split | SR total | SR modelled-only |
|---|---|---|
| train (in-sample) | 7.75% | 17.41% |
| val | 9.25% | 20.37% |
| test | 10.78% | 20.96% |

This variant also counts between-image distribution shift, because the pooled order-0 baseline fits any single image worse than that image's own order-0. It is therefore not a cleaner estimate. I give it only for the held-out vs in-sample comparison.

**Against the 20–30% reference:** the primary figure (per-image fit, total-residual denominator, 5.7–7.0%) is below the range. The modelled-only figure (13–17%) is below it. The pooled held-out modelled-only figure (20.4% val, 21.0% test) touches the lower edge. Whether the reference is crossed therefore depends on the denominator and on pooling. The primary definition does not cross it.

Evidence for a λ·R or minimum-block constraint (per_leaf.csv). Gain is the leaf's bits saved versus the train order-0 model, using the train-fitted leaf distribution.
- Held-out gain is negative for 13 of 128 leaves on val (total −0.51 Mbit, against +67.9 Mbit overall; those leaves carry 1.8% of val visits). On test, 0 of 128 leaves are negative.
- Gain per visit by train-occupancy quartile (lowest quartile first): val 0.75 / 1.67 / 2.15 / 0.67 bits; test 1.19 / 1.03 / 1.25 / 0.92 bits. Low-occupancy leaves do not show lower held-out gain per visit on test.
- The negative-gain val leaves are not only tiny leaves. The three smallest have 5,011, 5,154 and 5,351 pooled val visits.
- **Missing measurement:** gain per internal node (parent model versus its two children, on held-out images). That is the quantity a λ·R node penalty would act on. The leaf table is relative to order-0, not to the parent. The smallest addition is to build the same histograms at each internal node of `OR128_TREE_NODES`. I did not do this, to stay in scope.

## 2. Residual entropy: tree vs simple spatial predictor
Tree = Stage 1 residual conditioned on the leaf, static per-image entropy. MED = LOCO-I / JPEG-LS median edge detector on the same samples. Edges: pixel (0,0) is predicted as 2^(bd−1), the first row as W, the first column as N. Same zigzag and same bits/residual definition. MED is unconditioned order-0, with no context partition.
Two MED variants. The "raster" variant uses the 1-pixel neighbours of the raw mosaic, as Stage 1 does. The "plane" variant uses same-colour neighbours 2 pixels apart, the fairer baseline for Bayer; for the non-Bayer image (X-Trans) it equals the raster variant.

| split | n | tree | MED raster | MED plane | tree − raster | tree − plane |
|---|---|---|---|---|---|---|
| train | 244,520,800 | 9.0483 | 11.1281 | 8.0281 | −2.080 | **+1.020** |
| val | 74,699,104 | 8.6460 | 11.2393 | 7.5202 | −2.593 | **+1.126** |
| test | 115,444,116 | 7.8004 | 10.3719 | 6.6301 | −2.572 | **+1.170** |

- Against MED on the raw raster the tree entropy is **lower** on all 25 images.
- Against MED on same-colour planes the tree entropy is **higher** on 24 of 25 images, by about 1.0–1.2 bits/residual. The exception is x100s_xtrans_chart, where the plane variant equals the raster variant and the tree is lower.
- Order-0 entropy of the Stage 1 residuals themselves is already above plane-MED (9.59 / 9.30 / 8.35), so the gap comes from the Stage 1 residuals on the mosaic rather than from the tree partition. The tree partition itself saves 0.54 / 0.65 / 0.55 bits.
- Sanity check against real coding: the test tree total at 7.8004 bits/residual would be 112.56 MB static. The recorded adaptive Stage 4 payload is 109,737,551 B, 2.5% lower, which is plausible since adaptive coding follows within-image drift.
- **Not measured:** MED residuals conditioned on their own context partition. A like-for-like context-modelled MED baseline would need a separate diagnostic implementation.

## 3. Leaf occupancy (visits per leaf = residuals whose context id is that leaf)
Pooled per split:

| split | total visits | leaves | min | p25 | median | p75 | max | **bottom-50% share** | top-10 leaves share |
|---|---|---|---|---|---|---|---|---|---|
| train | 244,520,800 | 128 | 8,835 | 267,698 | 692,744 | 1,530,892 | 34,329,700 | **7.71%** | 48.3% |
| val | 74,699,104 | 128 | 827 | 30,970 | 121,549 | 422,109 | 15,047,357 | **3.38%** | 60.5% |
| test | 115,444,116 | 128 | 4,666 | 69,580 | 198,572 | 515,307 | 14,196,082 | **4.17%** | 64.7% |

No leaf is empty in any split (pooled).

Leaves below a visit threshold, pooled (number of leaves; share of visits). No threshold was proposed; this is a grid.

| visits < | train | val | test |
|---|---|---|---|
| 1,000 | 0; 0% | 2; 0.002% | 0; 0% |
| 10,000 | 2; 0.007% | 16; 0.11% | 8; 0.05% |
| 100,000 | 12; 0.14% | 61; 2.95% | 44; 1.80% |
| 1,000,000 | 78; 12.5% | 110; 25.4% | 107; 20.7% |

**Against the 5% reference (bottom 50% of leaves < 5% of visits):** crossed on val (3.38%) and test (4.17%); not crossed on train (7.71%).

Per image, which is the actual dilution unit because Stage 4 models start fresh for each image: bottom-50% share is below 5% on 19 of 25 images. The per-image minimum is 0.0000, the maximum 14.0% (hasselblad_tallinn). Per image, the number of leaves with fewer than 1,000 visits ranges from 2 to 115, and with fewer than 100 visits from 0 to 110 (per_image.csv). For example sigma_fp_bike8 has 18 leaves with 100 or more visits and leica_m8_house has 21. The occupancy data for assessing a hard minimum-visit threshold is in per_image_leaf_visits.csv (25 images x 128 leaves).

## Missing / limits
- The "signaling ratio" definition is my reading (see §1). If you intended a different quantity, it is not measured here.
- Per-internal-node held-out gain (for λ·R) was not measured; see §1.
- A context-conditioned MED baseline was not built; see §2.
- No real-coder (adaptive Stage 4) per-leaf costs were measured; all entropies are static plug-in.

## Files
`diag.json` (aggregates, occupancy, out-of-sample, per-image), `per_image.csv`, `per_leaf.csv`, `per_image_leaf_visits.csv`, `stdout.txt`, `stderr.txt`, code `src/bin/diag_or128_signal.rs`.
