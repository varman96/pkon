# REL6 production A/B validation (codec 2 vs new codec 3)

Written for: the project owner, deciding whether REL6 goes forward. Nothing committed; production default unchanged (`encode()` still writes codec 2).

## Verdict: **INCONCLUSIVE**. Strong positive on DEV/VAL, but promotion is not justified because no genuinely independent holdout exists

* All DEV/VAL promotion criteria (1-6, 10) hold, and no reject condition fires on any evidence.
* TEST criteria 7-9 are numerically met (15/15 smaller, +0.851% aggregate, no growth), but TEST is **contaminated** (section 7). Under the agreed handling (option 1), TEST evidence cannot upgrade the verdict to "candidate for promotion".
* **Single next action:** obtain or authorise a genuinely new, never-viewed holdout (new images, hashes recorded before running) and run this exact frozen A/B (`prod_ab`) on it, with the code unchanged.

## 1. Production change and codec-ID behaviour
Codec id **3** (`CODEC_X4PJ`) = codec 2 + one extra sign-node mixer input (`In::J`). Context: `((prevSign*27 + W*9 + N*3 + NE)*49 + P)*4 + magClass`, 15,876 per plane, mixer weight starts at 0. Header layout, version and CRC are unchanged. Readers accept 1, 2, 3; unknown ids still give `UnsupportedCodec`; writers still default to 2. Codec 3 is produced only by `encode_with_codec(img, CODEC_X4PJ)`. Docs updated (`docs/pkon2-format-v1.md`). Full diff: `production_port.patch`; audit: `source_diff_audit.md`.

Datasets: DEV = `train_pool` (25), VAL = `val` (10), TEST = `test` (15) of `results/learn1-splits.json` (sha256 `90ea95aa...3bb`, see `manifest_hashes.txt`); 1024-px centre crops whose content hashes were asserted equal to the bench50 manifest for every image; per-image hashes in `dataset_manifest.csv`. DEV/VAL were loaded only in Phase 1 and TEST only in Phase 2, after the implementation was frozen and Phase 1 artifacts saved.

## 2. Source-diff audit (criterion 12: **met**)
Only the sign-conditioning feature plus codec-ID plumbing changed on the production path; codec 1/2 never enter the new code (`has_j` is false). One existing-test edit was forced by the new id: two tests used id 3 as their "unsupported codec" example and now use 4. Research-only code (diagnostic hashes, dummy control) was not ported; the `pkonrel.rs` research copy and `rel6.rs` are untouched.

## 3a. Correctness and compatibility (all **pass**)
* All 50 images x both arms (production `encode_with_codec` then production `decode`): pixels, spec (dimensions, layout/channel order, 8-bit) identical; header codec byte as intended (A=2, B=3); payload length consistent; encoding deterministic; CRC passes. `run_stdout.txt`: "GATES PASS (35 images)" and "GATES PASS (15 images)".
* Arm A is byte-identical to the default `encode()` on every image, and to the frozen codec-2 reference size on all 35 DEV/VAL images.
* `cargo test --release`: all pre-existing tests pass (golden codec-1 files, pinned codec-1/2 payloads, mutation/no-panic) with the two-line id edit. New tests: codec 3 round trips on all layouts and edge sizes, is accepted by the production decoder, a patched codec byte is rejected where payloads differ, and the production codec-3 payload equals the independent research arm C stream.
* Production bytes equal the research result exactly (DEV 17,983,426; VAL 8,270,821).

## 3b. Actual encoded bytes (complete files)

| group | n | A bytes | B bytes | B-A | aggregate reduction % | smaller/equal/larger | min img % | median img % | max growth % | exact |
|---|---:|---:|---:|---:|---:|---|---:|---:|---:|---|
| DEV | 25 | 18,115,653 | 17,983,426 | -132,227 | 0.7299 | 25/0/0 | 0.3133 | 0.7474 | 0.0000 | True |
| VAL | 10 | 8,329,405 | 8,270,821 | -58,584 | 0.7033 | 10/0/0 | 0.4401 | 0.7271 | 0.0000 | True |
| DEV+VAL | 35 | 26,445,058 | 26,254,247 | -190,811 | 0.7215 | 35/0/0 | 0.3133 | 0.7474 | 0.0000 | True |
| TEST_light8 | 8 | 6,180,445 | 6,129,028 | -51,417 | 0.8319 | 8/0/0 | 0.5563 | 0.8304 | 0.0000 | True |
| TEST_used7 | 7 | 5,657,230 | 5,607,896 | -49,334 | 0.8721 | 7/0/0 | 0.6473 | 0.8844 | 0.0000 | True |
| TEST_all15 | 15 | 11,837,675 | 11,736,924 | -100,751 | 0.8511 | 15/0/0 | 0.5563 | 0.8844 | 0.0000 | True |

**Per image: DEV/VAL (35)**

| image | split | A bytes | B bytes | B-A | reduction % | exact A/B | enc A/B ms (med) | dec A/B ms (med) |
|---|---|---:|---:|---:|---:|---|---|---|
| text_0 | dev | 565,493 | 561,080 | -4,413 | 0.7804 | true/true | 463.41/467.70 | 428.16/576.17 |
| clouds_n0 | dev | 448,447 | 445,873 | -2,574 | 0.5740 | true/true | 497.92/558.22 | 476.67/535.54 |
| night_1 | dev | 617,913 | 612,703 | -5,210 | 0.8432 | true/true | 612.42/582.88 | 555.88/661.58 |
| child_n0 | dev | 406,096 | 403,061 | -3,035 | 0.7474 | true/true | 779.25/643.83 | 866.42/712.02 |
| forest_n0 | dev | 782,934 | 772,921 | -10,013 | 1.2789 | true/true | 549.39/726.89 | 692.76/626.26 |
| bridge_n0 | dev | 492,319 | 488,274 | -4,045 | 0.8216 | true/true | 344.39/368.98 | 344.14/365.18 |
| lake_n0 | dev | 837,460 | 829,589 | -7,871 | 0.9399 | true/true | 481.59/519.42 | 483.59/544.66 |
| street_n0 | dev | 912,364 | 906,251 | -6,113 | 0.6700 | true/true | 571.85/650.90 | 558.54/767.26 |
| waterfall_n0 | dev | 1,031,227 | 1,026,434 | -4,793 | 0.4648 | true/true | 955.90/1165.40 | 976.85/1104.96 |
| document_0 | dev | 523,638 | 519,887 | -3,751 | 0.7163 | true/true | 435.71/497.59 | 457.19/475.64 |
| river_n0 | dev | 1,019,443 | 1,010,117 | -9,326 | 0.9148 | true/true | 1183.41/1285.77 | 1171.89/900.80 |
| coast_n0 | dev | 444,975 | 442,661 | -2,314 | 0.5200 | true/true | 421.21/403.70 | 429.20/366.18 |
| macro_n0 | dev | 804,406 | 799,244 | -5,162 | 0.6417 | true/true | 583.37/600.59 | 607.71/636.10 |
| building_n0 | dev | 688,393 | 684,544 | -3,849 | 0.5591 | true/true | 561.70/578.60 | 592.44/540.76 |
| night_0 | dev | 1,917,507 | 1,911,500 | -6,007 | 0.3133 | true/true | 1265.68/1605.27 | 1149.56/2231.77 |
| indoor_1 | dev | 600,800 | 596,615 | -4,185 | 0.6966 | true/true | 828.79/640.34 | 771.65/844.21 |
| peak_n0 | dev | 1,207,636 | 1,199,744 | -7,892 | 0.6535 | true/true | 733.33/829.33 | 807.98/757.36 |
| bird_n0 | dev | 333,092 | 330,078 | -3,014 | 0.9049 | true/true | 670.48/683.57 | 599.66/687.98 |
| snow_n0 | dev | 1,230,807 | 1,223,563 | -7,244 | 0.5886 | true/true | 1086.57/954.16 | 865.09/909.51 |
| lighthouse_n0 | dev | 484,986 | 479,675 | -5,311 | 1.0951 | true/true | 479.51/489.49 | 456.18/599.35 |
| indoor_0 | dev | 346,721 | 343,593 | -3,128 | 0.9022 | true/true | 470.67/474.98 | 474.33/414.49 |
| desert_n0 | dev | 580,105 | 574,284 | -5,821 | 1.0034 | true/true | 817.59/805.63 | 780.82/782.41 |
| document_1 | dev | 402,044 | 399,299 | -2,745 | 0.6828 | true/true | 510.95/483.23 | 524.63/490.98 |
| text_1 | dev | 1,039,180 | 1,029,283 | -9,897 | 0.9524 | true/true | 706.16/766.84 | 659.10/693.21 |
| portrait_0 | dev | 397,667 | 393,153 | -4,514 | 1.1351 | true/true | 444.41/487.35 | 466.74/461.72 |
| food_0 | val | 1,265,561 | 1,255,156 | -10,405 | 0.8222 | true/true | 912.99/768.30 | 1099.43/696.95 |
| food_1 | val | 729,719 | 722,829 | -6,890 | 0.9442 | true/true | 586.78/674.59 | 620.65/614.91 |
| path_n0 | val | 1,710,962 | 1,703,432 | -7,530 | 0.4401 | true/true | 768.80/854.44 | 780.76/825.63 |
| boat_n0 | val | 1,004,572 | 998,282 | -6,290 | 0.6261 | true/true | 714.83/728.86 | 705.49/739.89 |
| flower_n0 | val | 758,220 | 753,428 | -4,792 | 0.6320 | true/true | 646.13/683.91 | 583.88/657.30 |
| beach_n0 | val | 560,047 | 555,125 | -4,922 | 0.8789 | true/true | 484.03/643.56 | 530.32/604.68 |
| woman_n0 | val | 335,672 | 333,701 | -1,971 | 0.5872 | true/true | 621.72/637.21 | 580.12/530.74 |
| outdoor_0 | val | 765,924 | 761,096 | -4,828 | 0.6303 | true/true | 816.37/824.60 | 849.40/736.31 |
| outdoor_1 | val | 495,025 | 489,985 | -5,040 | 1.0181 | true/true | 508.42/466.44 | 525.90/512.32 |
| restaurant_n0 | val | 703,703 | 697,787 | -5,916 | 0.8407 | true/true | 518.06/573.97 | 593.84/604.73 |

**Per image: TEST lightly touched (8)**

| image | split | A bytes | B bytes | B-A | reduction % | exact A/B | enc A/B ms (med) | dec A/B ms (med) |
|---|---|---:|---:|---:|---:|---|---|---|
| graffiti_n0 | test_light | 831,930 | 823,354 | -8,576 | 1.0309 | true/true | 742.67/666.78 | 795.99/790.46 |
| blurry_1 | test_light | 677,268 | 669,903 | -7,365 | 1.0875 | true/true | 687.68/780.96 | 757.98/759.58 |
| skyline_n0 | test_light | 327,725 | 323,207 | -4,518 | 1.3786 | true/true | 702.54/427.26 | 572.52/394.20 |
| station_n0 | test_light | 535,534 | 532,231 | -3,303 | 0.6168 | true/true | 631.79/573.82 | 610.98/567.23 |
| stairs_n0 | test_light | 1,096,140 | 1,084,703 | -11,437 | 1.0434 | true/true | 813.03/807.68 | 886.25/839.12 |
| bicycle_n0 | test_light | 1,096,143 | 1,089,238 | -6,905 | 0.6299 | true/true | 975.33/937.48 | 984.92/989.09 |
| church_n0 | test_light | 718,552 | 714,230 | -4,322 | 0.6015 | true/true | 599.84/805.66 | 652.10/813.75 |
| sunset_n0 | test_light | 897,153 | 892,162 | -4,991 | 0.5563 | true/true | 811.61/716.96 | 750.76/848.42 |

**Per image: TEST previously used in development/profiling (7)**

| image | split | A bytes | B bytes | B-A | reduction % | exact A/B | enc A/B ms (med) | dec A/B ms (med) |
|---|---|---:|---:|---:|---:|---|---|---|
| blurry_0 | test_used | 1,064,089 | 1,054,239 | -9,850 | 0.9257 | true/true | 819.17/949.82 | 820.91/843.01 |
| pet_0 | test_used | 731,467 | 724,469 | -6,998 | 0.9567 | true/true | 623.96/725.47 | 660.90/756.04 |
| pet_1 | test_used | 787,683 | 781,968 | -5,715 | 0.7255 | true/true | 684.39/643.77 | 752.42/731.07 |
| pet2_0 | test_used | 781,117 | 774,209 | -6,908 | 0.8844 | true/true | 711.31/843.73 | 753.46/730.70 |
| pet2_1 | test_used | 482,753 | 479,628 | -3,125 | 0.6473 | true/true | 571.29/573.63 | 560.11/606.63 |
| foliage_0 | test_used | 1,190,491 | 1,178,415 | -12,076 | 1.0144 | true/true | 952.56/878.69 | 933.05/968.16 |
| foliage_1 | test_used | 619,630 | 614,968 | -4,662 | 0.7524 | true/true | 690.04/620.36 | 628.97/741.86 |

## 4. Timing (production path, 1 warm-up + 3 reps per image and arm, order alternated; all reps kept in `timings.csv`)
Primary estimator (fixed before looking): median over images of (median-of-3 B)/(median-of-3 A).

| set | enc ratio | dec ratio | enc, aggregate sum | dec, aggregate sum | enc, ratio of median times | dec, ratio of median times |
|---|---:|---:|---:|---:|---:|---:|
| DEV/VAL (35) | 1.030 | 1.047 | 1.047 | 1.050 | 1.097 | 1.071 |
| TEST (15) | 0.961 | 1.004 | 0.994 | 1.023 | n/a | n/a |

Typical added cost is about 4-5% (roughly 0.59 s to 0.64 s per 1024x1024 RGB encode). **Variability is large:** the spread across the 3 reps of one image/arm is a median of 23-27% of its median time (max about 119%), and per-image ratios range from 0.63 to 1.94. Every estimator is below 1.15 (largest 1.097) and none approaches the 1.25 reject line, so criterion 10 is met on the data as collected. The right reading is "no evidence of a cost above about 10%", not a precise figure. No rerun was done.

## 5. Memory (criterion 11: **met**)
Extra coder state is a fixed `planes x 15,876 x 4 B` = 190,512 B for RGB (348,516 vs 158,004 B coder state; 254 KB extra for RGBA), independent of image size, allocated once per encode/decode (`memory_state_bytes.txt`). Process peak working set for a 1024x1024 encode+decode (`memory_peak.csv`, polled, noisy): +4 KB (forest_n0) and +0.77 MB (food_0) vs codec 2, against a ~50 MB peak dominated by source-image decoding. No per-image or per-context growth.

## 6. Decision criteria applied
| # | criterion | result |
|---|---|---|
| 1 | exact + format/compat gates | **met** |
| 2 | DEV/VAL >= 0.25% | **met** (0.7215%) |
| 3 | DEV >= 0.25% | **met** (0.7299%) |
| 4 | VAL >= 0.25% | **met** (0.7033%) |
| 5 | >= 80% of DEV/VAL images smaller | **met** (35/35) |
| 6 | no DEV/VAL image > 0.10% larger | **met** (none larger; smallest gain 0.313%) |
| 7 | TEST >= 0.25% | numerically 0.8511%; **cannot be credited as independent** |
| 8 | >= 80% of TEST images smaller | numerically 15/15; **cannot be credited as independent** |
| 9 | no TEST image > 0.10% larger | numerically none larger; **cannot be credited as independent** |
| 10 | median enc/dec time <= 1.15x | **met** (<= 1.097 on every estimator; noisy) |
| 11 | bounded memory | **met** |
| 12 | isolated change | **met** |

Reject conditions: none triggered (no correctness failure; DEV/VAL 0.72% >= 0.10%; TEST 0.85% >= 0.10%; neither DEV nor VAL grows; 0% of images grow; no image grows > 0.5%; timing <= 1.097 < 1.25; memory bounded; change isolated). Because the promotion list is not fully creditable (7-9), the result is **inconclusive**.

## 7. What the evidence does and does not justify
**Justifies:** REL6 survives production integration. The complete-file gain on DEV/VAL matches the research figure (0.72%, all 35 images improved), decoding is exact, existing formats are untouched, and the cost is small and bounded. On all 15 TEST images the gain is 0.85% with every image smaller. The 8 lightly touched images (0.832%) and the 7 previously used ones (0.872%) agree closely, so there is no sign that the result depends on the contaminated images.

**Does not justify:** calling any TEST result pristine or fully independent, or declaring criteria 7-9 satisfied. Seven TEST images (blurry_0, pet_0, pet_1, pet2_0, pet2_1, foliage_0, foliage_1) were used throughout development and in the profile that motivated REL6. The other eight (graffiti_n0, blurry_1, skyline_n0, station_n0, stairs_n0, bicycle_n0, church_n0, sunset_n0) appear only in size-benchmark reports (`bench50`, `pkon2p-production`) and not in REL6's design, but they were seen, so they are "lightly touched", not independent. Also not justified: a precise performance figure (timing is noisy), generalisation beyond 1024-px centre crops of this 50-image corpus, or switching the default.

Methodology notes: the harness first asserted RGB-only and panicked on the RGBA TEST image (blurry_1) before any result was produced; the assertion was removed (harness-only, logged in `build_config.txt`), and no codec code changed after Phase 1. The memory-measurement mode was added to the harness after Phase 2 and does not affect codec output.

## Artifacts (this directory)
`report.md`, `per_image.csv`, `aggregate.csv`, `timings.csv`, `dataset_manifest.csv`, `manifest_hashes.txt`, `source_diff_audit.md`, `production_port.patch`, `run_stdout.txt` (+ per-phase stdout/stderr), `build_config.txt` (toolchain, profile, exact commands), `decision_inputs.txt`, `memory_state_bytes.txt`, `memory_peak.csv`, `aggregate.py`, per-phase raw CSVs. The original REL6 research results are untouched.
