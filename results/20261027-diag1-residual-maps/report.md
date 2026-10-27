# DIAG1: where PKON2P's bits go, and where JPEG XL beats it

Diagnosis only. No codec code was changed, nothing committed, TEST (15 images) never loaded. Browse: `index.html` (contact sheets in `sheets/`, full-res maps in `maps/`).

## 1. What the repository actually says (differs from the brief)

- Source revision `bf0e6d6` plus the pre-existing uncommitted `src/pkonrel.rs` edit (untouched). New files only: `src/bin/diag1.rs`, `tools/diag1_*.py` (copies in `repro/`).
- **The "30 losing images" premise no longer holds.** Using the frozen 50-image benchmark (`results/20261027-155433Z-bench50`, JXL e9/e7) against PKON2P (codec 2, `results/20261027-pkon2p-production/per_image.csv`): JXL e9 is smaller on **15/50** images overall (e7: 13/50); the 30 was the old pre-P state. Inside the allowed DEV+VAL pool (35 images) it is **7**: text_1 (+4.07%), forest_n0 (+3.43), portrait_0 (+1.81), river_n0 (+1.71), night_0 (+1.40), street_n0 (+1.14), food_0 (+0.84). PKON2P beats e9 by up to 23% elsewhere.
- Split re-derived from the rule and checked against `results/learn1-splits.json` (sha256 90ea95aa…): 25 DEV / 10 VAL / 15 TEST; every crop sha256 matched the bench50 manifest (assertions in `diag1.rs`). Grouping rule fixed before any residual statistic was computed: **LOSE** = e9 smaller (7); **CONTROL** = 7 winners matched one-to-one to losers by overall PKON2P bits/pixel (waterfall_n0, outdoor_0, document_1, boat_n0, path_n0, lake_n0, snow_n0); **OTHER_WIN** = remaining 21. Group sizes are tiny, so group means are leads, not proof.
- Production pipeline (read from `pkon2.rs`, `pkon2x.rs`): planes coded in order G, R-G+128, B-G+128 (SubG); MED on each plane; residual = (pixel − MED) mod 256, zigzagged; X4P coder = binarised class/mantissa/sign decisions, 4 magnitude contexts + P (gap−MED, grad−MED, 49 classes) mixed logistically; contexts use only W/N/NW/NE/NN-type neighbours' residual magnitudes, the previous plane's residual at the *same* pixel, and the signs of W/N/NE/previous plane. **Position (x,y) is never used.**

## 2. Correctness of the diagnostic

- `diag1` runs the research coder (`pkonrel::run_rel`, arms X4 and X4+P) with a cost-recording sink: for all 35 images stream bytes **equal production codec 1 and codec 2 payloads exactly**, and the per-pixel cost map sums to the stream size within 4 bytes (`meta.json`, `analyze_stdout.txt`). Per-pixel decision counts were asserted to match the coder's binarisation.
- Exported MED residuals were re-derived by an independent numpy implementation of `neighbors()/predict()` (incl. boundary rules) from the source crop: identical on every pixel of every plane of all 35 images (asserted in `diag1_analyze.py`; `groups.json: convention_check`).

## 3. Conventions

Residual `r` = signed int8, the exact value the codec zigzags. "Absolute error" = |r| (0..128). Abs maps: `min(|r|*8,255)` (saturates at 32), signed maps: `clip(4r+128)` (zero = grey 128, brighter = positive), cost maps: `min(bits*32,255)` (saturates at 8 bits), all identical for every image/group. Contact sheets are 4x block-mean downscales (display only); every number is from full-res arrays. Full-res PNG maps + lossless int8 `.npz` residuals exist for the 14 LOSE+CONTROL images; sheets for all 35. P is shown only as a *bit-saving map* (X4 bits − X4P bits per pixel); it does not alter the MED prediction. The SubG transform is why RGB-domain MED errors (R,B ≈ 4.2–5.5 MAE) are much larger than the coded chroma residuals (≈1.2–1.9): the coded representation is G, R-G, B-G, shown in all maps; RGB-domain stats are in the CSV (`mae_rgb`).

(mb/px = millibits per pixel.)

## 4. Measurements (per-image CSVs: `per_image_plane_stats.csv`, group means below; unweighted over images)

| | LOSE (7) | CONTROL (7) | OTHER_WIN (21) |
|---|---|---|---|
| coded bpp G / R-G / B-G | 3.85 / 2.11 / 2.03 | 3.80 / 1.93 / 1.88 | 2.85 / 0.78 / 0.77 |
| MAE (SubG) G / R-G / B-G | 4.5 / 1.7 / 1.9 | 5.5 / 1.2 / 1.3 | 2.4 / 0.3 / 0.4 |
| pixels smooth / texture / edge (3×3 G-range ≤4 / ≤32 / >32) | 15% / 60% / 25% | 33% / 40% / 27% | 39% / 50% / 11% |
| mean bits/px in smooth regions | 1.73 | 1.28 | 0.86 |
| corr of signed r with W / N neighbour (G plane) | 0.31 / 0.36 | 0.16 / 0.10 | 0.28 / 0.25 |
| bits saved by P (bpp, G/R-G/B-G) | .044/.058/.048 | .035/.028/.025 | .039/.019/.018 |

Findings, in order of strength:

1. **Losers are texture/noise-floor images, not edge images.** They have the fewest smooth pixels (15% vs 33–39%) and the *highest* cost in smooth regions (1.73 vs 1.28 bits/px): sensor noise and fine detail (foliage, bark, food, branches against sky, skin) rather than sharp edges. Edge pixels carry a smaller share of bits for losers (31%) than controls (42%). Across all 35 images, the gap to JXL correlates with chroma bits/px (Spearman 0.52, p=0.001) and chroma share of bits (0.45), and anti-correlates with fraction of smooth pixels (−0.44) – exploratory, many tests were looked at, n=35.
2. **MED leaves linearly predictable structure in losers' G plane.** Signed MED residuals correlate with their W/N neighbours at 0.2–0.5 in six of seven losers (night_0, 0.05, is the exception) versus 0.1–0.16 in controls. If MED were adequate, that correlation would be ≈0. Magnitude clustering (|r| correlation ≈ 0.3–0.5) is similar in all groups and is already what the contexts exploit.
3. **Oracle linear-predictor test** (`oracle_ls_predictor.csv`, `causal_ls_G.csv`). Held-out static cost model identical for both predictors (class|activity×residual-energy + mantissa + sign). A per-tile least-squares predictor from 10 causal same-plane taps (oracle, fitted on its own 128² tile) reduces G-plane cost by 5.8% on average for losers (forest 6.7, portrait 13.6, text_1 10.1, river 5.4, street 4.0, food_0 3.4, night_0 −2.7). Fitting each tile on the **tile above it only** (strictly causal, conservative) keeps most of it: forest +4.4, portrait +9.7, text_1 +8.1, river +3.5, food_0 +1.9, street +1.8, night_0 −3.2; losers mean +3.75%. The signed-neighbour correlation predicts which images benefit (Spearman 0.58 with causal gain; night_0 has none and no gain). With G being ≈53% of file bits that is ≈+2.4% of file bits on average for losers – the same size as JXL's lead (mean 2.1%, range 0.8–4.1%). The same predictor **loses** on 15 of 28 non-losers (e.g. building_n0 −9, document_1 −11, indoor_0 −14, boat_n0 −10 causal), so it only helps if blended/selected adaptively; taking max(0, gain) per image gives LOSE 2.6% / CONTROL 0.2% / OTHER 1.3% of file. Caveats: static cost model, not the coder; the real coder's mixing already recovers part of what a static model misses; selection-of-best is optimistic; one 10-tap family tested.
4. **Chroma: partial cross-plane signal, partly exploited.** Corr(r_B-G, r_R-G) ≈ +0.2 with sign agreement 0.69–0.78 (the coder's sign context uses it). A linear fit of B-G on R-G and G residuals explains only 7.5–9.7% of variance. The oracle LS with cross-plane gradient taps gains +4.4% (LOSE) / +3.8% (CONTROL) on the B-G plane but loses on R-G and for OTHER_WIN, so cross-channel *prediction* is image-specific. Static information of the previous plane's signed residual given production-like contexts is ≈ +56 mb/px for B-G (≈3% of B-G bits); how much the existing sign context already captures is **not resolved**. Plane-0 residual given plane-1 residual adds little (≈ +17 mb/px). Windows of the previous plane that include pixels east/south of the current one (a genuinely unused source) added ≈ 0 beyond existing contexts.
5. **Entropy-model feature calibration.** With a production-like base context, the held-out static gain of P is 33–42 mb/px (control), the real coder gain is 25–58 mb/px, so static ≈ real here and the method is usable for ranking. A dummy hash of the same cardinality sits at −70 to −130 mb/px (dilution), so features must beat that to be real. Texture direction (signs of W−NW, N−NW, NE−N, 27 classes) shows +27 to +41 mb/px, comparable to P, and on a small base context adds +1.1% on top of P (P 3.6%, T 3.4%, P+T 4.9% static; heavy overlap). Pixel parity (x%2,y%2), a feature the coder lacks, is worth up to 5–11% of chroma-plane bits on ~10 images (child_n0, document_0, bird_n0, portrait_0, outdoor_0, bridge_n0; consistent with 4:2:0 JPEG sources), ≈0 on the losers except portrait_0, and only +0.36% on top of P+T on average. Mod-8/16 phase gains are indistinguishable from dilution.

Not supported: edges as the main explanation, cross-plane window-energy contexts, DCT-grid (mod 8) position contexts, exact-context match models (text_1 repeats are not a factor: only 5.5% of loser bits sit on pixels a last-occurrence model predicts exactly).

## 5. Direct answers

1. Structured spatial residuals: yes in six of seven losers (neighbour correlation 0.2–0.5 vs ≈0.1 controls); magnitude clustering is not different.
2. Errors concentrate in fine texture / noise floor (60% of loser pixels, 59% of loser bits); not edges.
3. Transformed planes: G dominates (≈53% of bits); chroma planes of losers are 10–12% costlier than controls and carry the strongest correlation with the JXL gap.
4. Cross-channel: some unexploited signal on B-G (signed prev-plane residual), but small and image-specific; unresolved overlap with the sign context.
5. The maps point first to a **better, adaptively selected spatial predictor** for the G plane; second to texture-direction / parity contexts (small).
6. Strongest: causal-LS signal on losers' G plane aligned with the JXL gap and with signed-residual correlation. Inconclusive: how much survives a real adaptive predictor plus the existing mixer; cross-plane gain; JXL's own mechanism (not inspected).

## 6. Recommended next experiment (one)

**Add a second, adaptively weighted prediction stage to the G plane, with MED kept as the fallback**: a backward-adaptive linear (LS/RLS or JXL-style error-weighted) predictor over ~10 causal taps, combined with MED by a selection or weighting driven by local past error (the repo already contains `Ens::{Three,Select3,Snap3,Sq3}` and `probe_m8`, whose results were not recorded in `results/`; reuse it as a starting point rather than writing new infrastructure, and add the LS member). Feed the predictor disagreement into the mixer in the same way P was added. Judge on actual bytes on DEV then VAL with the frozen thresholds of earlier REL experiments.

- Motivation: finding 2–3 (signed-residual correlation, causal LS gain of 2–10% of G cost on six of seven losers, negative on night_0 and on many winners ⇒ must be adaptive).
- Continue if: losers' G cost drops ≥2% and no image grows by more than ~0.3% (the selection must protect winners); aggregate DEV gain ≥ 0.5%.
- Abandon if: the adaptive blend recovers <1/3 of the oracle/causal gain on losers or hurts winners like M3/M7 did – then conclude MED + context mixing is saturated for spatial prediction and the remaining gap is not spatial.
- Cheap side bet if the predictor path fails: texture-direction input (and pixel-parity for chroma) as extra mixer inputs, expected ≤0.5%.

## Files

`per_image_plane_stats.csv`, `static_information*.csv`, `phase_*.csv`, `overlap_small_base.csv`, `oracle_ls_predictor.csv`, `causal_ls_G.csv` (machine-readable); `manifest.csv/json` (image → split, group, bytes, crop sha, outputs); `groups.json`; `repro/` (exporter and analysis scripts; run `diag1` then `diag1_analyze.py`, `diag1_extra.py`, `diag1_phase.py`, `diag1_ls.py`, `diag1_ls_causal.py`, `diag1_overlap.py`). Raw per-pixel cost arrays were kept outside the results directory (≈1 GB) and regenerate with `diag1`.
