# OR-128 learned context tree (Stage 3 rule v3): result

Date 2026-10-29. Algorithm name is OR-128 throughout. The tree is `Stage3Rule::Or128Tree` (rule version 3, opt-in); the fixed-table OR-128 (rule version 2) remains the production default and is unchanged. Stage 1, Stage 2, the PKON coder and the Stage 4 update equations were not modified.

## Verdict
On the 8 held-out images the learned tree is **0.30% LARGER** than the fixed-table OR-128 (109,737,551 B vs 109,408,946 B), better on 2 images and worse on 6. **The default stays at v2.** This reports one feature set, one objective and one training run; it does not show that decision trees cannot beat the table.

## Baseline check (before any tree work)
Split identical to `results/20261029-lisa-pcq1/split.json` (13 train / 4 val / 8 test). Production OR-128 payloads on the 8 test images equal the recorded PCQ-1 frozen-map bytes exactly (109,408,946 B), all exact/deterministic. On the 4 val images the local coder reproduced production payloads byte-for-byte (gate in `exp_or128_tree val`).

## Method
- Features (18, all from W, N, NW; missing neighbours imputed as in the table version): half-octave levels of W, N, NW, max, min, median, mean, |W-N|, |N-NW|, |W-NW|, W+N, max-min, |sW+sN-sNW|, |sW+sN|; zigzag parity of W, N, NW; count of missing neighbours. See `src/lisa_or128_tree.rs`.
- Learner: start from one leaf; best-first greedy growth. A candidate `feature <= threshold` is scored by the adaptive Stage 4 code length (counted estimator, models fresh per image, original coding order) of the two child subsequences on a 10% windowed sample of the 13 train images. Gain = parent cost - children cost. Minimum 4,000 sampled residuals per child, stop below 64 bits gain. Search approximation = windowed adaptive cost; all reported sizes below are real range-coder bytes.
- Complexity selection (rule declared before running): smallest leaf count within 0.05% of the best validation total, on the 4 val images with the real coder.
- Frozen tree: `src/lisa_or128_tree_data.rs`, sha256 `8df3a7e6...` (pinned in `tests/lisa_or128_tree.rs`). Compiled in; 0 B in the stream. Equivalent serialized size would be about 4 B per internal node, about 0.5 KB.

## Training
Greedy growth stopped itself at 203 leaves (no split above 64 bits). Estimated sampled train cost fell 2.53% (218,237,713 -> 212,710,783 bits). Load 287 s, greedy search 737 s (8 threads).

## Validation (real coder, 4 images, OR-128 table = 79,467,975 B)
| leaves | 1 | 8 | 16 | 32 | 64 | 96 | **128** | 160 | 192 |
|---|---|---|---|---|---|---|---|---|---|
| vs table | +2.446% | +0.818% | +0.423% | +0.174% | -0.012% | -0.067% | **-0.104%** | -0.125% | -0.138% |

Rule selected 128 leaves (the best, 192, is only 0.034 pp better). Validation gain of 0.10% was already much smaller than the 2.5% training estimate.

## Held-out test (production path, Stage 4 payload bytes)
| image | OR-128 table (v2) | OR-128 tree (v3) | change |
|---|---|---|---|
| dji_fc220_dusk | 14,125,601 | 14,084,252 | -0.293% |
| leica_m10_brick | 30,922,297 | 30,878,027 | -0.143% |
| leica_x2_lowlight | 12,100,174 | 12,168,636 | +0.566% |
| nikon_d80_piano | 8,535,478 | 8,558,380 | +0.268% |
| pentax_k30_frog | 15,269,777 | 15,399,722 | +0.851% |
| pentax_q7_cat | 10,533,683 | 10,539,364 | +0.054% |
| ricoh_gr_wood | 13,501,046 | 13,612,189 | +0.823% |
| sigma_fp_bike8 | 4,420,890 | 4,496,981 | +1.721% |
| **total** | **109,408,946** | **109,737,551** | **+0.300%** |

No tree/stream overhead applies (tree is fixed and versioned). Gains appeared on the two 14-bit images and losses on 12-bit and 8-bit images, but I did not test a bit-depth explanation.

## Tree
128 leaves, 127 internal nodes, max depth 12, mean leaf depth 7.9. Splits per feature: sign(W) 18, lvl(min) 15, lvl(max) 9, lvl(mean) 9, lvl(|sW+sN|) 9, lvl(N) 8, lvl(W) 8, sign(NW) 8, lvl(|W-NW|) 7, sign(N) 7, missing 6, lvl(med) 5, lvl(|W-N|) 5, and 2-3 each for lvl(|sW+sN-sNW|), lvl(max-min), lvl(NW), lvl(W+N), lvl(|N-NW|).

## Correctness
Both rules on all 8 test images: exact pixel reconstruction, metadata equal, deterministic repeated encode, decoded Stage 3 output (residuals and context ids) equal to the encoder's, full payload consumed. Tests: lib 31, lisa_or128 5, lisa_or128_tree 5 (new), lisa_stage4, lisa_pcq, pkon2, roundtrip 11 and golden codec tests all pass. One existing test was edited: `invalid_versions_and_counts_fail_safely` used rule version 3 as an invalid example, now valid, so it uses 4; `rule_cannot_be_reinterpreted` and `valid_streams_decode` were extended to cover v2<->v3. Legacy v1 byte-identity tests unchanged and passing.

## Timings (best of 3, seconds, single uncontended run; v2 / v3)
Stage 3 classify: 0.18-1.12 / 0.53-3.27. Stage 4 decode (includes context recomputation): 0.62-5.36 / 0.92-8.74. Stage 4 encode: roughly equal. Timings varied between repeated runs (the table's own Stage 4 encode ranged 1.38 to 2.57 s on similar work across runs), so no precise speed conclusion; the tree's per-sample evaluation (18 features plus a depth-8 walk) is clearly slower than a table lookup, about 2-3x for Stage 3 and decode.

## Limitations and next step
- The windowed 10% adaptive-cost objective overestimated the gain (2.5% train estimate, -0.10% val, +0.30% test). The tree overfit the 13 training images or the objective mismatches full-image adaptive coding. The table was also trained on the same 13 images with a different method, so the comparison is fair on data but not on search effort.
- Only causal W, N, NW information was used. Wider neighbours (WW, NN, NE) would need changing the Stage 3 call interface and are not valid on the Morton path without care; not tried.
- Next evidence-driven step: measure how much of the train-to-test gap is sampling versus image-specific overfitting, e.g. train with the real full-image coder cost on fewer candidates, or add per-split validation (stop growth when the validation images stop improving) rather than a post-hoc leaf count.
