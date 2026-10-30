# LISA Stage 3 research: Predictive Context Quantizer (PCQ)

Revision: HEAD 86cd186 plus an uncommitted working tree (RP1 work in src/lisa.rs and src/lisa_rp.rs, untouched by this task).
New files: src/lisa_pcq.rs, src/lisa_pcq_table.rs, src/bin/exp_lisa_pcq.rs, tests/lisa_pcq.rs; one line added to src/lib.rs.
Dataset: results/20261028-praw-bench1/dataset_manifest.json (25 RAW images, 8 to 16 bit). rustc 1.99.1, release profile.
No commits made. The whole test suite was NOT run (only tests/lisa_pcq.rs and tests/lisa_stage4.rs).

## 1. Question answered
Can a deterministic, universal, frozen Stage 3 context built from the W/N/NW neighborhood (without first averaging) make the
unchanged production Stage 4 compress better on held-out images? **Yes: -2.87% aggregate bytes on 8 held-out images, every image smaller, all exactly lossless.**

## 2. Source findings
- classify_activity (src/lisa.rs:919) matches the spec: W, N, NW zigzag residuals, rounded mean, thresholds [3,12,48], n==0 gives Low.
  Contexts are recomputed by the decoder; there is no label stream. Bayer mode uses row-major plane neighbors, identity mode uses spatial neighbors in Morton order.
- Stage 4 (S4Model, lisa.rs:1091) has ActivityClass::COUNT = 4 independent models, each with 34 unary and 34 mantissa `Bit` states, fresh per image and never shared between planes.
  The context count is a compile-time constant in the production encoder, decoder and Stage4Record.model_count, and STAGE3_RULE_VERSION is checked on decode.
- Production integration would need a Stage 3 rule/version bump, a variable model_count and a new coding version. This is not done: PCQ lives in an isolated module in the same style as lisa_rp, and production is byte-identical (control below).

## 3. Algorithm designed
Read W, N, NW (same neighbors as Stage 3). Analyze: map each neighbor to a half-octave level (22 levels: 0,1,2,3 exact, then 4-5, 6-7, 8-11, 12-15, 16-23, ...; capped at 21).
A missing neighbor is imputed with the rounded mean of the available ones (all missing gives 0), so edges reuse interior contexts.
Key = ordered level triple (22^3 = 10,648). Quantize: a frozen table key -> context id (K=128). Send: id selects the production Stage 4 model.

## 4. Feature representation and rationale
The ordered triple is the unreduced representation; no average is taken before quantization. Half-octave levels align with the old thresholds (3, 12, 48 are level boundaries).
Held-out cross-entropy of the target (bit length e of zigzag+1; bits/sample, pooled static model, 0.5 smoothing; train = 13 images, held-out = 4 val images):

| feature | contexts | train | held-out |
|---|---:|---:|---:|
| baseline 4-class (production) | 4 | 3.1842 | 3.2543 |
| average, half-octave levels | 22 | 2.8303 | 2.7261 |
| max level only | 22 | 2.8528 | 2.7683 |
| min level only | 22 | 2.9820 | 2.8737 |
| W only | 22 | 3.0245 | 2.9293 |
| W,N | 484 | 2.7744 | 2.6566 |
| mean of levels | 22 | 2.8603 | 2.7719 |
| (max, min) | 253 | 2.7598 | 2.6469 |
| (mean, range) | 330 | 2.7726 | 2.6552 |
| (max level, argmax position) | 64 | 2.8352 | 2.7457 |
| sorted triple (order-free) | 2024 | 2.7446 | 2.6279 |
| ordered triple W,N,NW | 10648 | 2.7244 | 2.6145 |

Reading: (a) most of the proxy gain comes from finer magnitude resolution (22 levels vs 4 classes; the old VeryHigh class lumps everything >= 48);
(b) structure adds a further ~0.11 bps on the proxy (22-level average 2.726 vs ordered triple 2.615);
(c) NW adds little beyond W,N (2.657 vs 2.615 includes it, 0.04 bps); order (sorted vs ordered) is worth ~0.013 bps;
(d) range and argmax position add little over magnitude summaries. Redundancy judged only on this proxy plus the actual-coding controls in section 9; I did not prove any feature useless.
Collapse example (train): zigzag (0,0,12) and (4,4,4) both score 4 (Moderate) but P(e=1) is 0.276 vs 0.098 and mean e is 2.91 vs 3.34, so the averaging does merge neighborhoods with different target distributions.
Population: all 10,648 keys occur in training; 7,842 keys hold 99% of mass; 1,708 keys with <1000 samples hold 0.36% of mass (sparse keys do not dominate).

## 5. Offline optimization method
Greedy agglomerative merging of the observed keys, minimizing at every step the increase in total ideal code length of the target histogram
(weighted Jensen-Shannon cost, the agglomerative information-bottleneck rule; no ordering constraint on the groups, unlike Forchhammer et al.'s ordered DP).
Training histograms are pooled raw counts (large images weigh more). Context ids are ordered by mean target bit length.
Handling of the stated concerns: predictive benefit = code-length cost of each merge; context count = the K sweep; sparse contexts = they merge first because their merge cost is small;
unseen keys (none occurred here) would take the context of the nearest observed key by L1 distance on levels (ties to lowest key), computed offline and frozen;
model/code-length overhead = measured by actual coding (each context costs fresh adaptive states), not modeled; generalization = K chosen on validation images, tested on separate test images.
K selection rule (declared after seeing the validation sweep, before any test run): smallest K within 0.05 pp of the best validation aggregate. K=128 (-2.033% vs K=256 -2.040%; K=64 -1.958% fails the rule).

## 6. Frozen universal mapping
src/lisa_pcq_table.rs: `PCQ_TABLE: [u8; 10648]`, `PCQ_K = 128`, compiled into the crate (sha256 of the source .bin e6dd165d...). One table for all images, bit depths and planes; nothing is stored in the stream.
Source binary: results/20261029-lisa-pcq1/train/map_K128.bin. Trained on the 13 train images only.

## 7. Files changed
Added: src/lisa_pcq.rs, src/lisa_pcq_table.rs, src/bin/exp_lisa_pcq.rs, tests/lisa_pcq.rs. Modified: src/lib.rs (two `pub mod` lines). No change to lisa.rs, lisa_rp.rs, Stage 1/2/4 code.
Results: results/20261029-lisa-pcq1/ (train/ maps and analysis.json, val_sweep*, test_*, trainval_*, split.json, this report). Per-image statistics were kept in the session scratchpad, not committed (regenerate with `exp_lisa_pcq stats`).

## 8. Training/validation split
Deterministic by sorted image id (rule in `split()`), not hand-picked: index%3==2 is TEST (8); of the rest, index%4==3 is VAL (4); the other 13 are TRAIN.
- TRAIN: bmpcc4k_person, canon_sx40_snow, hasselblad_tallinn, leica_m8_house, leica_q_canoe, lg_h815_mountain, parrot_anafi_dusk, pentax_k1ii_station, pentax_k70_bulb, pixel6pro_valley, ricoh_gr3_dunes, samsung_gx20_berries, x100s_xtrans_chart
- VAL: leica_cl_chateau, nexus6_courtyard, pentax_kx_lichen, sigma_fp_bike14
- TEST: dji_fc220_dusk, leica_m10_brick, leica_x2_lowlight, nikon_d80_piano, pentax_k30_frog, pentax_q7_cat, ricoh_gr_wood, sigma_fp_bike8
Test covers 8, 12 and 14 bit; 10 and 16 bit appear only in train/val. The only non-Bayer (identity/Morton) image, x100s_xtrans_chart, is in TRAIN, so Morton generalization is untested on held-out data.

## 9. Results (actual range-coded bytes, production Stage 4 unchanged)
Baseline = production encode_stage4 re-measured on this working tree (needed because the tree is dirty; timings are best of 3).

Validation sweep (4 val images, aggregate vs baseline 81,117,121 B): max-level-22 control -1.453%, mean-of-levels-22 control -1.138%, K=16 -1.512%, K=32 -1.837%, K=64 -1.958%, K=128 -2.033%, K=256 -2.040%.

TEST (held out), frozen K=128:

| image | bits | baseline B | PCQ B | change | bps base -> PCQ | max-level-22 ctl | K=64 |
|---|--:|--:|--:|--:|---|--:|--:|
| dji_fc220_dusk | 14 | 14,376,023 | 14,125,601 | -1.742% | 9.5840 -> 9.4171 | -1.232% | -1.738% |
| leica_m10_brick | 14 | 31,680,502 | 30,922,297 | -2.393% | 10.6096 -> 10.3557 | -1.772% | -2.321% |
| leica_x2_lowlight | 12 | 12,590,596 | 12,100,174 | -3.895% | 6.2265 -> 5.9840 | -3.103% | -3.402% |
| nikon_d80_piano | 12 | 8,607,207 | 8,535,478 | -0.833% | 6.7621 -> 6.7057 | -0.317% | -0.658% |
| pentax_k30_frog | 12 | 16,052,117 | 15,269,777 | -4.874% | 7.8456 -> 7.4632 | -3.172% | -4.457% |
| pentax_q7_cat | 12 | 10,752,399 | 10,533,683 | -2.034% | 7.0554 -> 6.9119 | -1.681% | -2.013% |
| ricoh_gr_wood | 12 | 13,998,332 | 13,501,046 | -3.552% | 6.8835 -> 6.6390 | -2.279% | -3.105% |
| sigma_fp_bike8 | 8 | 4,579,439 | 4,420,890 | -3.462% | 4.3783 -> 4.2267 | -2.299% | -2.690% |
| **aggregate (size-weighted)** | | **112,636,615** | **109,408,946** | **-2.866%** | 7.8055 -> 7.5818 | -2.016% | -2.628% |

Unweighted mean of per-image percentages: -2.848% (range -0.83% to -4.87%). The aggregate is the byte-weighted figure.
In-sample check with the same frozen table: TRAIN aggregate -2.111% (12 of 13 images smaller; x100s_xtrans_chart, the Morton image, +0.007%), VAL aggregate -2.033%. Held-out is not worse than in-sample, so no sign of overfitting.
Per-image train/val numbers: trainval_stdout.txt and trainval_results.json.
Attribution: the fine-resolution scalar control (max level, 22 contexts, no structure) gets -2.02% of the -2.87% on test; the trained structure contributes about 0.85 pp more.
The mean-of-levels scalar is weaker than max level in the real coder (-1.14% vs -1.45% on val), so the specific summary matters.
The pooled cross-entropy proxy predicted about -0.62 bps for K>=64 but actual coding gives about -0.2 bps (2 to 3% of 7.8 bps). The proxy overstates the gain because production adapts per image; only the actual-size numbers should be used.

## 10. Lossless and determinism verification
Measured, per image, in every eval run: (a) production encode/decode/restore reproduces samples (prod_ok), (b) the baseline classifier through the experimental coder reproduces production payload bytes exactly and decodes to the same residuals and ids (control_ok), (c) PCQ decode reproduces the Stage 3 residuals exactly and consumes the whole payload, (d) encoder and decoder context-id vectors are identical, (e) a repeat encode gives identical bytes and ids. All true for all 25 images. Context assignment in the decoder uses only already-decoded values (shared traversal, decoder never reads the source), which (d) exercises.
tests/lisa_pcq.rs (passes): synthetic Bayer odd-size 12/14-bit, monochrome 8/16-bit round trips for all of the above plus production-chain pixel recovery; key and frozen table are total over 17^3 neighbor combinations including None and u32::MAX; every table entry is in range and every context id is used. tests/lisa_stage4.rs (production) passes.
Not done: the PCQ-decoded residuals are not fed through a PCQ-specific restore to pixels, because Stage2Output fields are private. Pixel recovery follows from exact residual equality plus the unchanged, separately verified production inverse. The full test suite was not run.

## 11. Runtime and complexity
Test aggregate, best of 3 (single machine, other load not controlled): encode 9.63 s -> 13.56 s (+41%), decode 10.20 s -> 12.07 s (+18%); per-image ratios range from about 0.7x to 2.8x, so timing is noisy and only indicative.
Overhead: one table lookup plus three level computations per sample. Model state: 128 contexts x 68 `Bit` states vs 4 x 68. Frozen table 10,648 B in the binary, 0 B in the stream.

## 12. What worked
Finer, structure-preserving contexts reduced real coded size on every held-out image, with a gain that grows to K~128 and then saturates. About 70% of the test gain comes from finer magnitude resolution, about 30% from W/N/NW structure.

## 13. What failed or remains uncertain
- Morton/identity path: one image only (in train), +0.007%, i.e. no gain there; held-out behavior unknown.
- 10-bit and 16-bit are not in the test set. Only 8 test images; K was chosen on 4 validation images.
- The attribution of structure vs resolution uses single-scalar controls, not a tuned ~22-context scalar quantizer, so the structure share may be overstated. A trained K=22 on the average alone is the clean control.
- Feature redundancy (NW, order, range) is judged on a proxy that is known to overstate gains; not confirmed by actual coding except through the K=64/128 comparison.
- Timing overhead unmeasured under controlled conditions.
- Not integrated into production: needs a version bump and variable context count in Stage4Record.

## 14. Decision and next step
Held-out compression improves (-2.87%) with exact lossless results and no overfitting signal, so the evidence supports proceeding rather than rejecting PCQ; adopting it into production is the user's call (it changes the Stage 3 rule and stream version).
Smallest next experiment: a resolution-only control, an optimally trained scalar quantizer (K~22 on the average or max level, same train/val/test split), to say how much of the gain really needs the three-neighbor structure, and a 10/16-bit and Morton held-out check by moving those images into TEST in a second split.
