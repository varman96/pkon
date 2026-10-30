# LISA Stage 3 final comparison: trained 22-context scalar quantizer vs frozen K=128 PCQ

Original evidence preserved unchanged: results/20261029-lisa-pcq1/. Nothing committed. src/lisa.rs, Stage 1/2/4, the range coder and the PCQ table are untouched.
Code added (experiment only): `collect_score`, `score_bin`, `partition_dp` appended to src/lisa_pcq.rs; `sstats`/`strain` subcommands and a `scalar:<file>` map type in src/bin/exp_lisa_pcq.rs. Focused checks only (the experiment's own eval); no test suite was run.

## 1. Scalar quantizer design (frozen)
Feature: the production Stage 3 score, i.e. the rounded mean of the available W/N/NW zigzag residuals (same as `classify_activity`; none available -> 0), clamped to 0..1023.
Quantizer: 22 contiguous intervals of that score, chosen by exact dynamic programming to minimize total ideal code length of the target (bit length of zigzag+1) pooled over the 13 TRAIN images. No hand-picked thresholds.
Context starts at score >= 0, 1, 3, 6, 10, 15, 21, 29, 39, 54, 72, 94, 118, 150, 193, 239, 306, 387, 477, 614, 782, 1023.
Frozen table: results/20261029-lisa-scalar1/train/scalar_K22.bin (1024 B), sha256 f4e44dca9512a5eaf4a18adc045fc325a17900009388f7477c5eb94f689e0f33. Nothing is stored in the stream.
Proxy cross-entropy: train 2.8411 bps, val 2.7536 bps.

## 2. Separation
Same deterministic split as the PCQ study (results/20261029-lisa-pcq1/split.json): 13 TRAIN, 4 VAL, 8 TEST.
Score statistics were gathered for TRAIN and VAL images only (the `sstats` command never loads TEST images); training used TRAIN only.
K=22 was fixed by the task, so no parameter was selected on any data; VAL was evaluated for reporting only. The mapping was frozen (hash above) before the first TEST run, and TEST was run once. Test data influenced no threshold, count or other parameter.

## 3. Actual range-coded bytes (production Stage 4, same binary, corpus and procedure for all)
VAL aggregate (81,117,121 B baseline): scalar-22 -1.387%, PCQ-128 -2.033%.

TEST:

| image | baseline B | scalar-22 B | change | PCQ-128 B | change |
|---|--:|--:|--:|--:|--:|
| dji_fc220_dusk | 14,376,023 | 14,237,533 | -0.963% | 14,125,601 | -1.742% |
| leica_m10_brick | 31,680,502 | 31,155,748 | -1.656% | 30,922,297 | -2.393% |
| leica_x2_lowlight | 12,590,596 | 12,304,565 | -2.272% | 12,100,174 | -3.895% |
| nikon_d80_piano | 8,607,207 | 8,573,818 | -0.388% | 8,535,478 | -0.833% |
| pentax_k30_frog | 16,052,117 | 15,645,855 | -2.531% | 15,269,777 | -4.874% |
| pentax_q7_cat | 10,752,399 | 10,581,764 | -1.587% | 10,533,683 | -2.034% |
| ricoh_gr_wood | 13,998,332 | 13,765,780 | -1.661% | 13,501,046 | -3.552% |
| sigma_fp_bike8 | 4,579,439 | 4,480,137 | -2.168% | 4,420,890 | -3.462% |
| **aggregate** | **112,636,615** | **110,745,200** | **-1.679%** | **109,408,946** | **-2.866%** |

PCQ beats scalar-22 on all 8 held-out images (by 0.38 to 2.34 pp); aggregate gap 1,336,254 B = 1.19 pp of baseline (1.2% of the scalar-22 size). PCQ also wins on VAL (0.65 pp).
The PCQ numbers reproduce the earlier report exactly (same bytes).

## 4. Losslessness and agreement
All 8 test and 4 val images, both variants: residual decode exact and payload fully consumed (lossless=true), encoder and decoder context-id vectors identical, repeat encode byte-identical, production round trip ok, and the baseline-classifier control reproduces production bytes (control_ok=true).
Limitation unchanged from the PCQ study: this is exact equality at the residual level; a PCQ-specific full pixel round trip is not part of this experiment and is on the production-integration list.

## 5. Runtime (indicative only)
TEST totals, best-of-3 per image, same procedure and process, single machine with uncontrolled other load: encode baseline 12.8 s, scalar-22 16.8 s, PCQ-128 20.5 s; decode baseline 15.0 s, scalar-22 17.2 s, PCQ-128 18.8 s. The baseline itself ran ~33% slower than in the earlier PCQ run (9.6 s), so absolute values and small differences are noisy; no timing conclusion is drawn and none is needed for the decision.

## 6. Decision (predefined rule)
PCQ has a meaningful byte-size advantage (-2.87% vs -1.68%, every held-out image, same sign on validation), not a negligible one. **Retain PCQ (K=128).** The scalar quantizer is not preferred.
Attribution update: a trained 22-context scalar on the average gets 59% of PCQ's test gain; the W/N/NW structure is worth about 1.19 pp more, which is larger than the earlier untrained-control estimate of 0.85 pp.

## 7. Next step
Stage 3 research stops here. Proceed to production engineering: integrate PCQ K=128 as the Stage 3 rule/version, support 128 contexts in `Stage4Record`, add a PCQ-specific full pixel round trip, run focused regression tests, then the required release suite and the frozen JPEG XL benchmark.
