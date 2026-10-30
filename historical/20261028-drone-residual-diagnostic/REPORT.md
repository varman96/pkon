# PRAW drone residual diagnostic: `parrot_anafi_dusk`

Tags: **[R]** recorded in an existing artifact or source; **[D]** derived by arithmetic from [R] values; **[U]** unavailable.
Nothing was run, modified or recomputed. Sources read: `src/praw.rs`, `src/praw_mixer.rs`, `src/praw_cloud.rs`, `src/bin/bench_praw1.rs`, and the existing `results/20261028-drone-diagnostics1/README.md` (which summarises the bench1/mixer/cloud artifacts).

## 1. Stage 1: predictor and residual mapping (from `src/praw.rs`)

- **Traversal [R]**: Morton (Z-order) over the full interleaved mosaic (`morton_visit`, `encode`). The traversal does not look at the CFA, so W/N neighbours are usually a different colour channel than the current pixel.
- **Predictor [R]**: per sample, every pattern whose neighbours are all already reconstructed is eligible. The `PATTERNS` list includes W, N, 2W−WW, 2N−NN, a gradient-weighted W/N blend, W/E and N/S interpolation (E and S are available in Morton order), med3(W,N,NE) and NE.
  - Each eligible pattern is scored by replaying it on 9 nearby reconstructed positions (`TRAIN`), with cost `log2_q4(zigzag(err)+1)`. The lowest cost wins.
  - A per-(pattern, activity-ctx) bias is added. The context is built from W/N/NW/NE gradients, shifted by `bit_depth−8`, with 8 buckets. The bias is bounded to 32 counts by halving.
  - The result is clamped to `[0, maxv]`. With no eligible pattern, the mid-range value is used.
- **Mapping [R]**: signed `r = v − pred` goes to `z = 2r` if r ≥ 0, else `−2r−1`. This gives 0, −1, 1, −2, 2 → 0, 1, 2, 3, 4. Residuals are kept in Morton order.
- **Stage 2 [R]**: `organize_stage2` splits the Morton-order residuals into 4 Bayer-parity planes (row-major, `i = offset_y*2 + offset_x`). Image CFA is `[2,1,1,0]`, so the plane colours are B, G, G, R in image-parity order. This is only a relabelling of residuals; the predictor never saw planes.
- **Stage 3 [R]**: `classify_activity` takes the mean zigzag value of W, N and NW *within the same Bayer plane* of the residual array. Thresholds are `[3, 12, 48]` on the zigzag scale: Low < 3 ≤ Moderate < 12 ≤ High < 48 ≤ VeryHigh.
- **Scale of the existing ">= 1024" counter [R]**: `BIG = 1024` in `praw_mixer.rs`, applied to `z`, so it is on the **zigzag** scale. This resolves the open question in the earlier diagnostics README.
  - [D] z ≥ 1024 means |r| ≥ 512 (positive r ≥ 512, or negative r ≤ −512).
  - [D] Stage 3's VeryHigh cut of 48 means a mean |r| of about 24, against a 14-bit range of 16383.

## 2. What we know (all [R] unless marked)

| Fact | Value |
|---|---|
| Samples | 21,461,504, four Bayer planes of 5,365,376 each |
| Source values | min 816, max 16368, "every sample a multiple of 16", "~6% of range used" (report.md statement only) |
| Stage 1 zero fraction / mean z / H0 | 0.772% / 559.56 / 10.164 b/sample |
| z ≥ 1024 | 3,534,982 = 16.471% [D]; the mixer spends 20.632% [D] of its ideal bytes on them |
| Per-plane (0..3) H0 | 9.704 / 10.712 / 9.376 / 10.080 |
| Per-plane mean z | 440.2 / 876.6 / 273.2 / 648.2 |
| Per-plane zero fraction | 0.825% / 0.686% / 0.836% / 0.742% |
| Stage 3 classes (Low / Mod / High / VeryHigh) | 0.335% / 0.197% / 7.467% / 92.001% |
| Class H0 (same order) | 0.657 / 7.678 / 7.841 / 10.311 |
| Class mean z (same order) | 24.8 / 116.8 / 88.6 / 600.7 |
| Class-conditioned static H0 | 10.0885 b/sample [D], against 10.1644 unconditioned |
| Coded cost (b/sample) | Stage 4 9.633, mixer 9.353, JXL 5.142 [D] |

[D] Reading of the class table: the class split is real in mean z and H0 (Low 24.8 and 0.66 b, VeryHigh 600.7 and 10.31 b). But 92% of samples fall in one class, so the split buys only 10.1644 − 10.0885 = 0.076 b/sample of static order-0 entropy. In this image the classes separate a tiny low-activity minority from the bulk; they do not stratify the bulk.

[D] Within the 92% VeryHigh class, the mean z of 600.7 shows the class lumps residuals from about 24 up to thousands.

[D] Per-plane H0 spans 9.38 to 10.71 b (1.34 b). Plane 1 has about 3.2 times the mean z of plane 2 (876.6 / 273.2). Plane roles come from the CFA `[2,1,1,0]`; the artifacts store no colour label per plane.

## 3. Can residual distributions be recovered from existing artifacts? No

Residual arrays are not persisted; only scalar summaries are. The artifacts hold per-plane and per-class `n`, `mean_zz`, `zero_frac` and `h0_bits`. `bench_praw1.rs` builds full histograms in memory (`stats`, `class_hist`) and discards them after printing the scalars.

[U] (not recorded, not derivable):
- Residual min/max, globally or per plane.
- Any histogram.
- Counts above zigzag 255 or 4095. Only ≥ 1024 exists, and only globally.
- Per-plane large-residual rates.
- Per-plane activity-class distributions.
- Per-class tail rates (share of z ≥ 1024 inside each class).
- Per-class and per-plane percentiles.

The H0 values and the class table cannot be inverted into a distribution. The per-segment mixer rows (16 segments) give bits/sample only, not tail rates.

**Not recoverable from the DNG without recomputing Stage 1.** The residual depends on the adaptive predictor state, so any independent reconstruction would effectively re-run Stage 1.

## 4. Cheapest reliable way to get the missing measurements

Stage 1, 2 and 3 are deterministic and cheap compared with Stage 4 and the decode round-trip. `bench_praw1.rs` already contains the loader and the Stage 1→3 pipeline and the histogram code, but it also runs Stage 4 encode and the full decode gate. Skipping those is what makes this cheap.

The measurement is one pass over the Stage 3 output (`OrganizedResiduals::BayerPlanes` plus `OrganizedContexts::BayerPlanes`) after `encode_with_metadata` → `organize_stage2` → `classify_stage3`, for this single image (`--only parrot_anafi_dusk` pattern). It needs:
- a per-plane histogram of `z` (cap `2^(bd+1)` like `stats`);
- per-(plane, class) counts of `z` above the three thresholds, plus min/max.

From this single pass, every requested item follows:
- min/max and histogram: direct;
- counts above 255, 1024, 4095: sums over histogram bins;
- per-plane large-residual rates: tail counts per plane;
- whether Stage 3 classes separate residuals meaningfully: per-class histogram and tail share, e.g. the fraction of z ≥ 1024 residuals that sit in VeryHigh.

Cost: one Stage 1 pass (the dominant cost, it replays up to 14 patterns per sample on 9 positions), no entropy coding, no decode. A tool that writes only a new results directory touches no existing artifact.

## 5. Single best next experiment

**Per-plane and per-activity-class residual tail profile for `parrot_anafi_dusk`.**

Measure, for each Bayer plane and each Stage 3 class: n, min/max z, the full z histogram, and the counts and rates of z > 255, ≥ 1024 and > 4095. Also record the fraction of all z ≥ 1024 residuals falling in each class and plane.

Decision criterion (fixed before running):

1. **Tail-dominated** if z ≥ 1024 is ≥ 15% of residuals in at least 3 of the 4 planes **and** those residuals carry ≥ 20% of the mixer's ideal bytes (the global 16.471% and 20.632% are already known). Then Stage 1 prediction quality on the mosaic is the target. Next step is a predictor change (for example CFA-aware same-colour neighbours) evaluated on Stage 1 residual statistics first, before any Stage 4 work.
2. **Plane-driven** if one or two planes hold ≥ 60% of the z ≥ 1024 residuals while the rest have tail rate < 8%. Then per-plane modelling (a Stage 4 context or a per-plane predictor choice) is the target, not the global predictor.
3. **Class resolution** if ≥ 95% of z ≥ 1024 residuals fall in VeryHigh **and** the VeryHigh class spans a z range from below 48 to above 4095 with no internal separation. Then the Stage 3 thresholds `[3, 12, 48]` cannot discriminate this image's tail, and the next step is a threshold re-scale test (a quantity report.md calls "mis-scaled" but never tested).
4. **Bounded tail** if z > 4095 is < 1% of residuals and the z ≥ 1024 mass is concentrated in 1024..4095. Then the tail is a moderate-magnitude noise floor, and the gap to JXL is not an escape-coding problem. Stop work on escape or tail coding.

The result must be assigned to exactly one branch. The thresholds above are decision bounds chosen for this report, not recorded facts. If none fits cleanly, report the measured values rather than force a branch.

## 6. Observations, not tested

- [D] Samples are all multiples of 16 (source-image statement, [R] in report.md), but the Stage 1 predictor averages and bias-corrects. So `pred` is generally not a multiple of 16, and `r = v − pred` carries the low 4 bits of the prediction. Whether this inflates residual entropy is **untested** and not supported by any stored statistic. The tail-profile run above can include a count of `(v − pred) mod 16` as a free by-product only if the signed residual or the prediction is kept, which the `Stage1Output` already holds (z maps back to r exactly). It is a secondary diagnostic, not a precondition for the decision above.
- The Stage 1 predictor works on the mosaic and ignores CFA colour, while Stage 2 and 3 treat planes as the unit. This is a structural fact from the code, not a measured cause of cost.

## Confirmation

No source code or existing artifact was modified. No compression, decoding, or benchmark was run. The only addition is this file.
