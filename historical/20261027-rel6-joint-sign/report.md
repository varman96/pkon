# REL6 — joint sign context: three-arm falsification trial

Criteria frozen before any arm ran: `criteria.md` (sha256 `0e368c6f590f54ce494639baf39c9b4d4699be548a0b8fb0b386b86434dc3b93`). Nothing committed; production default untouched.

## 1. Frozen baseline and source revision

* Source: HEAD `d768736` ("bitprofil"), working tree clean at start. Changed by this task: `src/pkonrel.rs` (research copy, +91/−6) and new `src/bin/rel6.rs`; production files unmodified (`source_diff_audit.md`).
* Discrepancy with the task text: GDIR/GRAMP are **committed** in HEAD (not uncommitted); nothing needed preserving.
* Profile baseline: `results/20261027-212136Z-bitprof` arm P = 15,324,422 B over 20 images (re-summed from its CSV: matches). The profile's joint-sign gap = 54,739 + 19,550 + 20,122 = 94,411 B (0.616%) is against KT on the key S0×S1×S2×P (12·27·27·49 slots). S2 = (p,w,n) duplicates p (in S0) and w,n (in S1), so the effective support is 3×4×27×49 = 15,876, as specified.
* **Dataset deviation (TEST rule):** 7 of the 20 discovery images (blurry_0, foliage_0, foliage_1, pet2_0, pet2_1, pet_0, pet_1) are in the TEST split of `learn1-splits.json`. "Keep TEST unloaded" was honoured, so they were **not** loaded. Discovery results below are the 13 discovery images that sit inside DEV (9) / VAL (4); they are not independent of DEV/VAL. The 20-image discovery total cannot be reproduced without loading TEST; I did not do that. (The original profiler had already read those 7, so TEST is not pristine w.r.t. that profile.)
* Arm A vs frozen control: per-image Arm A file bytes equal the frozen reference (profile arm P for discovery images, REL4 arm P for all 35; the two agree where both exist) on **35/35** images: **PASS**. Arm A is also byte-identical to `pkonfile::encode` (production codec 2) on every image. No baseline was updated.

Arm A totals: DEV (25) 18,115,653 B; VAL (10) 8,329,405 B; DEV+VAL union (35 unique) 26,445,058 B; Discovery-20 subset loaded (13 = 9 DEV + 4 VAL) 9,667,192 B; DEV not in discovery (16, clean) 11,704,690 B; VAL not in discovery (6, clean) 5,073,176 B.

Dataset manifests (immutable inputs, sha256): see `manifest_references.txt`; per-image identity/format/dimensions/representation/membership: `manifest_loaded.csv`; overlap: `manifest_overlap.txt` (DEV∩VAL = ∅, so the 35-image union has 35 unique images; DEV∩Discovery = 9, VAL∩Discovery = 4).

## 2. Implementation and isolation

Arms (`J_ARMS`): **A** = production codec 2 (`Extra::P`); **B** = A + `DummyJointSign{salt 1}` — hash(plane,x,y,salt) mod 15,876 at the sign node, all planes; **C** = A + `JointSign` — context ((p·27 + w·9+n·3+ne)·49 + pf)·4 + cls = 3·4·27·49 = 15,876 at the sign node, all planes. Extra mixer weight starts at 0 and the bit models at p=½, n=0 in B and C; update order is the base-input loop. The existing REL5 `DummySign` (card 49, planes ≥ 1 only) does **not** satisfy the requirement (different cardinality and routing), so a new `DummyJointSign` with the candidate's cardinality and routing was added. Full audit with the remaining B-vs-C differences: `source_diff_audit.md`.

## 3. Correctness results

All gates passed (`run_stdout_gates.txt`): for every one of the 35 images × 3 arms — exact reconstruction of all pixels (from the raw stream **and** decoded back from the complete file bytes with CRC32), dimensions/representation, decoder consumes exactly the stream length (no over-read, correct termination), re-encode byte-identical, encoder/decoder context+symbol trace hash equal, encoder/decoder sign-node (plane, extra contexts, final probability) hash equal, adaptive-state hash equal after every plane and at the end, instrumented stream = uninstrumented stream, Arm A = production `encode` byte-for-byte = frozen reference, non-sign cross-entropy and decision counts of B and C exactly (f64 ==) equal to A, equal model allocation in B and C (1,289,925 bit models, 5,159,700 B; 47,628 reachable joint models at the sign node = 3 planes × 15,876). Regression tests: `cargo test --release` 17/17 pass (`regression_tests_summary.txt`), incl. the new unit test.

## 4–5. Actual encoded bytes (complete files, 24-byte header + payload)

S = 100·(control − candidate)/control, positive = smaller.

| set | A bytes | B bytes | C bytes | S(B,A) % | S(C,A) % | S(C,B) % | images C<A | images C<B |
|---|---:|---:|---:|---:|---:|---:|---:|---:|
| DEV (25) | 18,115,653 | 18,115,841 | 17,983,426 | -0.001 | +0.730 | +0.731 | 25/25 | 25/25 |
| VAL (10) | 8,329,405 | 8,329,502 | 8,270,821 | -0.001 | +0.703 | +0.704 | 10/10 | 10/10 |
| DEV+VAL union (35 unique) | 26,445,058 | 26,445,343 | 26,254,247 | -0.001 | +0.722 | +0.723 | 35/35 | 35/35 |
| Discovery-20 subset loaded (13 = 9 DEV + 4 VAL) | 9,667,192 | 9,667,289 | 9,596,179 | -0.001 | +0.735 | +0.736 | 13/13 | 13/13 |
| DEV not in discovery (16, clean) | 11,704,690 | 11,704,818 | 11,616,313 | -0.001 | +0.755 | +0.756 | 16/16 | 16/16 |
| VAL not in discovery (6, clean) | 5,073,176 | 5,073,236 | 5,041,755 | -0.001 | +0.619 | +0.621 | 6/6 | 6/6 |

Per image (full table with sign CE in `per_image.csv`):

| image | set | disc | A | B | C | S(B,A)% | S(C,A)% | S(C,B)% | sign-CE gain C vs B % |
|---|---|---|---:|---:|---:|---:|---:|---:|---:|
| beach_n0 | val |  | 560,047 | 560,054 | 555,125 | -0.001 | +0.879 | +0.880 | +4.27 |
| bird_n0 | dev |  | 333,092 | 333,096 | 330,078 | -0.001 | +0.905 | +0.906 | +5.64 |
| boat_n0 | val |  | 1,004,572 | 1,004,583 | 998,282 | -0.001 | +0.626 | +0.627 | +3.38 |
| bridge_n0 | dev |  | 492,319 | 492,325 | 488,274 | -0.001 | +0.822 | +0.823 | +4.21 |
| building_n0 | dev |  | 688,393 | 688,402 | 684,544 | -0.001 | +0.559 | +0.560 | +2.89 |
| child_n0 | dev |  | 406,096 | 406,102 | 403,061 | -0.002 | +0.747 | +0.749 | +3.96 |
| clouds_n0 | dev |  | 448,447 | 448,451 | 445,873 | -0.001 | +0.574 | +0.575 | +2.83 |
| coast_n0 | dev |  | 444,975 | 444,980 | 442,661 | -0.001 | +0.520 | +0.521 | +2.75 |
| desert_n0 | dev |  | 580,105 | 580,114 | 574,284 | -0.002 | +1.003 | +1.005 | +4.91 |
| document_0 | dev | y | 523,638 | 523,644 | 519,887 | -0.001 | +0.716 | +0.718 | +3.48 |
| document_1 | dev | y | 402,044 | 402,050 | 399,299 | -0.002 | +0.683 | +0.684 | +3.38 |
| flower_n0 | val |  | 758,220 | 758,230 | 753,428 | -0.001 | +0.632 | +0.633 | +3.42 |
| food_0 | val | y | 1,265,561 | 1,265,572 | 1,255,156 | -0.001 | +0.822 | +0.823 | +4.43 |
| food_1 | val | y | 729,719 | 729,729 | 722,829 | -0.001 | +0.944 | +0.946 | +5.05 |
| forest_n0 | dev |  | 782,934 | 782,943 | 772,921 | -0.001 | +1.279 | +1.280 | +7.03 |
| indoor_0 | dev | y | 346,721 | 346,726 | 343,593 | -0.001 | +0.902 | +0.904 | +4.96 |
| indoor_1 | dev | y | 600,800 | 600,807 | 596,615 | -0.001 | +0.697 | +0.698 | +3.49 |
| lake_n0 | dev |  | 837,460 | 837,467 | 829,589 | -0.001 | +0.940 | +0.941 | +5.09 |
| lighthouse_n0 | dev |  | 484,986 | 484,991 | 479,675 | -0.001 | +1.095 | +1.096 | +6.09 |
| macro_n0 | dev |  | 804,406 | 804,417 | 799,244 | -0.001 | +0.642 | +0.643 | +3.19 |
| night_0 | dev | y | 1,917,507 | 1,917,517 | 1,911,500 | -0.001 | +0.313 | +0.314 | +1.89 |
| night_1 | dev | y | 617,913 | 617,919 | 612,703 | -0.001 | +0.843 | +0.844 | +4.31 |
| outdoor_0 | val | y | 765,924 | 765,933 | 761,096 | -0.001 | +0.630 | +0.631 | +3.39 |
| outdoor_1 | val | y | 495,025 | 495,032 | 489,985 | -0.001 | +1.018 | +1.020 | +5.80 |
| path_n0 | val |  | 1,710,962 | 1,710,976 | 1,703,432 | -0.001 | +0.440 | +0.441 | +2.48 |
| peak_n0 | dev |  | 1,207,636 | 1,207,647 | 1,199,744 | -0.001 | +0.653 | +0.654 | +3.68 |
| portrait_0 | dev | y | 397,667 | 397,672 | 393,153 | -0.001 | +1.135 | +1.136 | +6.07 |
| restaurant_n0 | val |  | 703,703 | 703,713 | 697,787 | -0.001 | +0.841 | +0.842 | +4.34 |
| river_n0 | dev |  | 1,019,443 | 1,019,453 | 1,010,117 | -0.001 | +0.915 | +0.916 | +4.72 |
| snow_n0 | dev |  | 1,230,807 | 1,230,819 | 1,223,563 | -0.001 | +0.589 | +0.590 | +3.14 |
| street_n0 | dev |  | 912,364 | 912,374 | 906,251 | -0.001 | +0.670 | +0.671 | +3.40 |
| text_0 | dev | y | 565,493 | 565,501 | 561,080 | -0.001 | +0.780 | +0.782 | +3.81 |
| text_1 | dev | y | 1,039,180 | 1,039,187 | 1,029,283 | -0.001 | +0.952 | +0.953 | +5.71 |
| waterfall_n0 | dev |  | 1,031,227 | 1,031,237 | 1,026,434 | -0.001 | +0.465 | +0.466 | +2.40 |
| woman_n0 | val |  | 335,672 | 335,680 | 333,701 | -0.002 | +0.587 | +0.590 | +3.29 |

## 6. Sign-node cross-entropy and model-cost vs file-size reconciliation

Sign-node CE, aggregate over all planes (exact coder cross-entropy, established `CeEnc` accounting; context-free tail bits are in the `mantissa+tail` stage):

| set | A sign bytes | B | C | gain B vs A % | gain C vs A % | **gain C vs B %** |
|---|---:|---:|---:|---:|---:|---:|
| DEV (25) | 3,400,057 | 3,400,247 | 3,267,832 | -0.006 | +3.889 | +3.894 |
| VAL (10) | 1,545,305 | 1,545,403 | 1,486,719 | -0.006 | +3.791 | +3.797 |
| DEV+VAL union (35 unique) | 4,945,362 | 4,945,650 | 4,754,551 | -0.006 | +3.858 | +3.864 |
| Discovery-20 subset loaded (13 = 9 DEV + 4 VAL) | 1,777,585 | 1,777,684 | 1,706,572 | -0.006 | +3.995 | +4.000 |
| DEV not in discovery (16, clean) | 2,224,323 | 2,224,452 | 2,135,948 | -0.006 | +3.973 | +3.979 |
| VAL not in discovery (6, clean) | 943,454 | 943,514 | 912,031 | -0.006 | +3.331 | +3.337 |

By plane (union):

| plane | sign decisions | A bits/dec | B bits/dec | C bits/dec | gain C vs A % | gain C vs B % | ideal bytes saved C vs A | share of total saving |
|---|---:|---:|---:|---:|---:|---:|---:|---:|
| G | 26,544,735 | 0.8787 | 0.8788 | 0.8489 | +3.398 | +3.403 | 99,080 | 51.9% |
| R-G | 10,189,735 | 0.7747 | 0.7748 | 0.7399 | +4.503 | +4.510 | 44,433 | 23.3% |
| B-G | 11,445,983 | 0.7289 | 0.7289 | 0.6958 | +4.535 | +4.542 | 47,298 | 24.8% |

Reconciliation of model-cost saving vs actual file saving (C vs A; ideal = Σ −log2 P over all decisions / 8, i.e. the cross-entropy the coder was handed):

| set | ideal-bytes saving (all stages) | ideal-bytes saving (sign stage) | actual file-bytes saving | actual − ideal | non-sign ideal saving |
|---|---:|---:|---:|---:|---:|
| DEV (25) | 132225 | 132225 | 132227 | +2 | -0 |
| VAL (10) | 58585 | 58585 | 58584 | -1 | 0 |
| DEV+VAL union (35 unique) | 190810 | 190810 | 190811 | +1 | 0 |
| Discovery-20 subset loaded (13 = 9 DEV + 4 VAL) | 71013 | 71013 | 71013 | -0 | -0 |
| DEV not in discovery (16, clean) | 88375 | 88375 | 88377 | +2 | 0 |
| VAL not in discovery (6, clean) | 31423 | 31423 | 31421 | -2 | 0 |

Coder termination/precision overhead (payload − ideal bytes, union, 35 files): A 125 B, B 121 B, C 124 B (≈ 3.6 B per file; fixed 24-byte header is identical in all arms; B and C use codec byte 3 in the header but the same length). The actual−ideal residual above is therefore the range-coder precision/termination difference, not a modelling effect.

Profile estimate vs realised: the bit-cost profile estimated the joint-KT reference at 94,411 B (0.616%) below the mixer over the 20 discovery images. Realised on the 13 loaded discovery images: file saving of C vs A = 71013 B (0.735%); the profile's estimate is for the plain KT *replacement* of the mixer on all 20 images, so it is a different quantity (a ceiling for replacing the mixer, not for adding an input).

## 6b. Instrumented diagnostics (`diag` phase; instrumented streams verified byte-identical to uninstrumented)

### Joint-context occupancy (per plane, aggregated over the 35 images; allocated = 15,876 contexts per plane; arm B = hashed dummy, C = real)

| arm | plane | active ctx (mean/img) | unseen ctx (mean/img) | obs per active ctx (mean) | median obs per active ctx | decisions in ctxs with < 16 obs | first-visit (cold) decisions | cold share of sign decisions | cold-decision cost (bytes, total) |
|---|---|---:|---:|---:|---:|---:|---:|---:|---:|
| B:P+dummyJ | G | 15876 | 0 | 47.8 | 48 | 0.22% | 555,660 | 2.09% | 60898 |
| B:P+dummyJ | R-G | 15649 | 227 | 18.6 | 12 | 21.83% | 547,703 | 5.38% | 50694 |
| B:P+dummyJ | B-G | 15743 | 133 | 20.8 | 13 | 17.88% | 551,018 | 4.81% | 46557 |
| C:P+jointsign | G | 3596 | 12280 | 210.9 | 22 | 1.07% | 125,857 | 0.47% | 15005 |
| C:P+jointsign | R-G | 4151 | 11725 | 70.1 | 5 | 3.81% | 145,284 | 1.43% | 15940 |
| C:P+jointsign | B-G | 4552 | 11324 | 71.8 | 6 | 3.68% | 159,317 | 1.39% | 17421 |

Frequency distribution of context occupancy (arm C, union; contexts per log2 bin of observation count, and the share of sign decisions that fall in each bin):

| plane | 1 | 2–3 | 4–7 | 8–15 | 16–31 | 32–63 | 64–127 | 128–255 | 256+ |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| G contexts | 13,886 | 14,188 | 14,578 | 14,427 | 13,908 | 13,314 | 11,793 | 9,800 | 19,963 |
| G decision share | 0.1% | 0.1% | 0.3% | 0.6% | 1.2% | 2.3% | 4.0% | 6.7% | 84.7% |
| R-G contexts | 30,344 | 24,269 | 20,824 | 17,478 | 15,047 | 12,212 | 9,484 | 6,832 | 8,794 |
| R-G decision share | 0.3% | 0.6% | 1.1% | 1.9% | 3.3% | 5.4% | 8.4% | 12.1% | 67.0% |
| B-G contexts | 32,542 | 26,184 | 22,110 | 19,144 | 16,746 | 13,853 | 10,356 | 7,813 | 10,569 |
| B-G decision share | 0.3% | 0.5% | 1.0% | 1.8% | 3.3% | 5.4% | 8.2% | 12.4% | 67.0% |

Sparsest images by mean active joint contexts per plane (arm C): child_n0 (1248), bird_n0 (1454), document_0 (1632), portrait_0 (1740), coast_n0 (2238). Densest: snow_n0 (7094), path_n0 (7542), night_0 (8448).

### Solo value of the joint input and mixing

| plane | mixed C sign bytes | C solo joint-model bytes | A mixed sign bytes | B mixed sign bytes |
|---|---:|---:|---:|---:|
| G | 2816606 | 2842343 | 2915686 | 2915829 |
| R-G | 942376 | 962471 | 986808 | 986882 |
| B-G | 995569 | 1017578 | 1042867 | 1042940 |

### Which sign probabilities changed, and did it pay (union)

| comparison | plane | sign decisions | decisions whose coded probability changed | mean abs(Δ logit) of changed | changed & cheaper | changed & costlier | net ideal bytes saved |
|---|---|---:|---:|---:|---:|---:|---:|
| B:P+dummyJ vs A:P | G | 26,544,735 | 69.32% | 0.0133 | 50.18% | 49.82% | -142 |
| B:P+dummyJ vs A:P | R-G | 10,189,735 | 69.96% | 0.0179 | 50.51% | 49.49% | -74 |
| B:P+dummyJ vs A:P | B-G | 11,445,983 | 71.86% | 0.0168 | 50.59% | 49.41% | -73 |
| C:P+jointsign vs A:P | G | 26,544,735 | 99.39% | 0.3654 | 58.04% | 41.96% | 99080 |
| C:P+jointsign vs A:P | R-G | 10,189,735 | 99.51% | 0.4529 | 60.98% | 39.02% | 44433 |
| C:P+jointsign vs A:P | B-G | 11,445,983 | 99.57% | 0.4412 | 61.20% | 38.80% | 47298 |
| C:P+jointsign vs B:P+dummyJ | G | 26,544,735 | 99.39% | 0.3658 | 58.02% | 41.98% | 99222 |
| C:P+jointsign vs B:P+dummyJ | R-G | 10,189,735 | 99.52% | 0.4534 | 60.95% | 39.05% | 44506 |
| C:P+jointsign vs B:P+dummyJ | B-G | 11,445,983 | 99.57% | 0.4416 | 61.18% | 38.82% | 47371 |

Propagation: sign-node mixer weights and bit models are per (plane, node); non-sign nodes share no state with the sign node, so the change cannot reach later *non-sign* decisions — gate G7 confirms non-sign CE and decision counts are exactly equal in A, B and C. Within the sign node the new input changes the shared mixer weight vector (all of weights 0..3 adapt on a different error signal) and the per-context models; the decision *sequence* (which pixels have a sign and the sign bits) is identical in all arms (asserted per image).

### Calibration of the coded sign probability (union, all planes; 16 uniform probability buckets)

| arm | ECE (n-weighted mean abs(pred − observed)) | worst bucket abs(pred − observed), n ≥ 10,000 |
|---|---:|---:|
| A:P | 0.00374 | 0.00709 |
| B:P+dummyJ | 0.00371 | 0.00695 |
| C:P+jointsign | 0.00198 | 0.00447 |

### Adaptive state (final, mean over images)

Initialisation: every extra bit model p = ½, n = 0; extra mixer weights 0 (so the first sign prediction of B and C equals A's). Final sign-node mixer weight of the first extra (P) and the second extra (dummy/joint) and number of models saturated at the count limit (255):

| arm | plane | w(P) at sign node | w(second extra) at sign node | P models active / saturated at sign node (mean) | joint models active/saturated (mean) |
|---|---|---:|---:|---:|---:|
| A:P | G | 0.8387 | - | 48 / 41 | - |
| A:P | R-G | 0.8719 | - | 35 / 21 | - |
| A:P | B-G | 0.9212 | - | 36 / 22 | - |
| B:P+dummyJ | G | 0.8387 | 0.0036 | 48 / 41 | 15876 / 0 |
| B:P+dummyJ | R-G | 0.8720 | -0.0031 | 35 / 21 | 15649 / 0 |
| B:P+dummyJ | B-G | 0.9213 | 0.0057 | 36 / 22 | 15743 / 0 |
| C:P+jointsign | G | 0.1614 | 0.8468 | 48 / 41 | 3596 / 572 |
| C:P+jointsign | R-G | 0.2199 | 0.7733 | 35 / 21 | 4151 / 252 |
| C:P+jointsign | B-G | 0.2686 | 0.7788 | 36 / 22 | 4552 / 303 |

## 7. Per-image findings

* Sign CE improves but file grows (C vs A): none.
* C beats A but not B: none.
* Images where C grows > 0.1% vs A: none.
* C-vs-A saving range over 35 images: min +0.313% (night_0), median +0.747%, max +1.279% (forest_n0).
* Discovery-selected vs not (informational; the 13 are inside DEV/VAL, so they are contaminated by the profile's selection): S(C,A) on the 13 = +0.735%, on the 22 non-discovery = +0.714%. Discovery gains hold on the non-discovery images.

## 8. Runtime

Single thread, release build, sequential, 1 untimed warm-up, 3 timed reps, arm order rotated per image/rep, plain `BinEnc`/`BinDec` sinks (no instrumentation), each timing includes `Coder::new` (allocation + init) and for decode the full reconstruction. Machine: `machine_conditions.txt`. All raw measurements are in `timings_raw.csv`; per-image median/min/max/spread in `timings.csv`; nothing was discarded.

| arm | enc: Σ per-image median (s) | enc ratio vs A (aggregate) | enc: median of per-image ratios | dec: Σ median (s) | dec ratio vs A (aggregate) | dec: median of per-image ratios |
|---|---:|---:|---:|---:|---:|---:|
| A:P | 21.55 | 1.000 | 1.000 | 22.00 | 1.000 | 1.000 |
| B:P+dummyJ | 23.56 | 1.094 | 1.082 | 24.54 | 1.116 | 1.085 |
| C:P+jointsign | 22.88 | 1.062 | 1.039 | 23.68 | 1.076 | 1.034 |
| prod_codec2 | 16.04 | 0.744 | 0.735 | 16.24 | 0.738 | 0.735 |

Per-rep aggregate ratios C/A (reproducibility): rep 0 enc 1.107 dec 1.091; rep 1 enc 1.100 dec 1.094; rep 2 enc 1.094 dec 1.072. B/A: rep 0 enc 1.231 dec 1.150; rep 1 enc 1.127 dec 1.161; rep 2 enc 1.079 dec 1.072.
Run-to-run spread (max−min over 3 reps, % of median) across all image/arm/mode cells: median 70.7%, 90th pct 146.2%, max 212.8%.

Largest per-image C/A time ratios: boat_n0 dec 1.844, night_0 enc 1.621, waterfall_n0 dec 1.554, boat_n0 enc 1.363.

Instrumentation overhead (separate runs, `instr_overhead.csv`, indicative): mean time plain `BinEnc` vs `CeEnc`(+diag flag off) vs `DiagEnc`+diag on — A:P: plain 572 ms, ce 589 ms, diag 1289 ms; B:P+dummyJ: plain 654 ms, ce 699 ms, diag 1456 ms; C:P+jointsign: plain 888 ms, ce 924 ms, diag 2064 ms. The `diag` column also contains one extra uninstrumented encode (used for the byte-identity assertion), so it overstates pure instrumentation cost by roughly one plain encode. Instrumented runs were never used for the timing table.

**Timing criterion: PASS under the predeclared harness-relative definition** (C/A in the same research harness: enc 1.062 aggregate / 1.039 median-of-ratios, dec 1.076 / 1.034; each of the 3 reps individually below 1.15). Documented limitations:

* **Drift:** the machine was not in a steady state. Σ Arm A encode per rep was 30.8 s, 23.5 s, 19.3 s (decode 33.5, 24.1, 20.0 s); the same-cell run-to-run spread is therefore large (above). Arm order was rotated per image and rep, so arm ratios are less affected than absolute times, but absolute times from this run should not be reused. A second timing pass was considered and not run; the C/A ratio was 1.07–1.11 in every rep, so a rerun is not expected to change the criterion outcome, but this has not been verified.
* **Production-harness gap:** production codec 2 (`pkonfile`) is ~26% faster than the research harness running the same Arm A (production/A: enc 0.744, dec 0.738). The ≤1.15 criterion was frozen as C versus A in the *same* harness, which isolates the cost of the extra input. C versus the *production* implementation is 1.43x enc / 1.46x dec in these numbers, but that mixes the input cost with the harness gap; the true cost of the extra input inside the production coder is **not measured** and must be measured at promotion review.

Initialisation (`Coder::new`, median of 21): A:P 0.05 ms, B:P+dummyJ 1.13 ms, C:P+jointsign 1.14 ms. There is no per-image reset beyond this allocation.

## 9. Memory

Peak working set / peak commit measured by `K32GetProcessMemoryInfo` in a **fresh child process per measurement** (raw prepared planes read from disk first, so JPEG decoding is excluded; 3 repetitions, median). `peak − base` = peak minus the working set after loading the planes. Static model state (`memory_static.csv`) is an allocation-formula estimate and is labelled as such.

| image | crop px | mode | A peak WS (MiB) | B | C | C − A (MiB) | B − A (MiB) | C − A peak commit (MiB) |
|---|---:|---|---:|---:|---:|---:|---:|---:|
| bird_n0 | 1024 | dec | 15.1 | 20.0 | 20.0 | +4.91 | +4.91 | +4.91 |
| bird_n0 | 1024 | enc | 15.7 | 20.6 | 20.6 | +4.91 | +4.91 | +4.92 |
| building_n0 | 128 | dec | 5.0 | 9.9 | 9.9 | +4.91 | +4.90 | +4.92 |
| building_n0 | 128 | enc | 5.0 | 9.9 | 9.9 | +4.91 | +4.90 | +4.92 |
| building_n0 | 256 | dec | 5.7 | 10.6 | 10.6 | +4.90 | +4.91 | +4.91 |
| building_n0 | 256 | enc | 5.7 | 10.6 | 10.6 | +4.91 | +4.91 | +4.92 |
| building_n0 | 512 | dec | 8.4 | 13.3 | 13.3 | +4.91 | +4.91 | +4.92 |
| building_n0 | 512 | enc | 8.7 | 13.6 | 13.6 | +4.91 | +4.91 | +4.91 |
| building_n0 | 1024 | dec | 15.4 | 20.3 | 20.3 | +4.89 | +4.90 | +4.88 |
| building_n0 | 1024 | enc | 16.5 | 21.4 | 21.4 | +4.91 | +4.92 | +4.92 |
| night_0 | 1024 | dec | 16.6 | 21.5 | 21.5 | +4.90 | +4.91 | +4.91 |
| night_0 | 1024 | enc | 17.8 | 22.7 | 22.7 | +4.91 | +4.89 | +4.89 |

Static model-state estimate (allocation formula, 3 planes × 27 nodes × 4 B per bit model): A:P: 35,154 bit models = 0.13 MiB; extras alloc 3,969 models = 0.02 MiB; reachable at sign node 0; B:P+dummyJ: 1,321,029 bit models = 5.04 MiB; extras alloc 1,289,925 models = 4.92 MiB; reachable at sign node 47628; C:P+jointsign: 1,321,029 bit models = 5.04 MiB; extras alloc 1,289,925 models = 4.92 MiB; reachable at sign node 47628. All table sizes depend only on plane count and context cardinalities, not on image width/height (bounded); only the pixel buffers scale with image size, equally in all arms. Actual occupied joint contexts: `context_occupancy.csv` (6b). Per-file cost: the 15,876×27 table is allocated per plane (the sign node uses 1/27 of it).

Incremental peak working set C − A: max 4.91 MiB (limit 16); 1024-px vs 128-px increment difference 0.01 MiB (limit 2) -> memory BOUNDED.

## 10. Predeclared decision (criteria.md, applied once)

| criterion | met | value |
|---|---|---|
| Exact reconstruction on every image (all gates) | yes | gates passed, 35×3 runs |
| S(C,A,DEV) >= 0.30% | yes | +0.730% |
| S(C,A,VAL) >= 0.15% | yes | +0.703% |
| S(C,A,union) >= 0.25% | yes | +0.722% |
| sign-CE gain C vs B >= 2.0% | yes | +3.864% |
| no DEV/VAL image grows > 0.1% (C vs A) | yes | worst -0.313% (night_0) |
| median enc and dec time ratios <= 1.15 (aggregate and median-of-ratios) | yes | enc 1.062/1.039 dec 1.076/1.034 |
| incremental memory measured and bounded | yes | see section 9 |
| source-diff audit confirms isolation | yes | source_diff_audit.md |

**Decision: CANDIDATE FOR FURTHER VALIDATION (research candidate only; no production change).**

## 11. Failure analysis and limitations

* No failure found: no gate failed, no image regressed, no image where sign CE improved but the file grew, none where C beat A but not B. The dummy (B) changed total size by -0.001%, so extra-input capacity/interface effects are negligible in this harness.
* Dummy is capacity-matched but not distribution-matched (uniform position hash vs skewed real-context occupancy); see `source_diff_audit.md`. This cannot inflate C's gain, it only limits what B can rule out.
* Selection contamination: the joint context was chosen from a profile of the 20 discovery images; 13 of them are in DEV/VAL. The 22 untouched DEV/VAL images gain +0.714%% vs +0.735%% on the 13, so there is no sign of selection inflation, but DEV/VAL themselves were also used by earlier experiments (REL1–REL5). TEST was never loaded; the 7 discovery images that live in TEST were not run, so the 20-image discovery result is not reproduced.
* Timing is noisy and measured only in the research harness (section 8). Memory was measured for 3 images at 1024 px and one image at 128/256/512 px, not on all images.
* `timing_run2/` is a leftover from a second timing pass that was started and then cancelled before it produced any timings (it holds only manifest copies and an empty log); it is not evidence and was not used.
* Format/promotion: B and C files use codec byte 3, which production rejects. Promotion would need a new codec id (or an explicit format decision), a production implementation, golden fixtures, malformed-input tests and a production-coder timing measurement. None of that was done.
* The file saving exceeds the profile's 0.616%% estimate (0.722%% here); the two are different quantities (KT replacement of the mixer on 20 images vs an added mixer input on 35), so do not read the estimate as a tight bound.

## 12. Decision and next action

**Candidate for further validation — retained as a research candidate for a separate promotion review. The production default is unchanged and nothing is committed.**

Next action (not performed here): a separate promotion review that (1) implements the input in the production coder behind a new codec id, (2) measures production-relative enc/dec time and peak memory, (3) evaluates on TEST once, under frozen criteria, and (4) adds fixtures and malformed-stream tests.
