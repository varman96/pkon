# OR-128 production validation (3-image smoke benchmark)

Date 2026-10-29. HEAD 8b3d034. Working tree was clean at start (OR-128 integration is already committed in HEAD, not uncommitted as the task brief said). Nothing committed, reset, stashed or deleted. Only additions: `src/bin/val_or128.rs` and this directory.

## Corpus (approved set, all present; hashes match `results/20261028-praw-bench1/dataset_manifest.json`)
| id | camera | dims | bits | CFA | DNG compression | Stage 2 path | DNG sha256 | canon sha256 |
|---|---|---|---|---|---|---|---|---|
| pentax_q7_cat | PENTAX Q7 | 4064x3000 | 12 | RGGB-class 2x2 `[0,1,1,2]`, origin 0,0 | none | Bayer planes (BayerCfa) | 577eac8d... | 6dbbb396... |
| pentax_kx_lichen | PENTAX K-x | 4352x2868 | 12 | 2x2 `[2,1,1,0]` | JPEG | Bayer planes | 7a594b66... | d759e7b5... |
| parrot_anafi_dusk | Parrot Anafi | 5344x4016 | 14 | 2x2 `[2,1,1,0]` | none | Bayer planes | a257a296... | 1f05c5bb... |

All three exercise the Bayer path; the non-Bayer/Morton path was not exercised on real images (synthetic tests only). Canonical samples are the frozen full stored raster as little-endian u16 (`data/praw1/canon/*.u16`); the round trip is checked against those, per the project methodology. I did not re-extract canon from the DNGs: `tifffile`/`imagecodecs` are not installed here. Both DNG and canon sha256 were re-verified against the manifest.

## Table verification
sha256 of `OR128_TABLE` (parsed from `src/lisa_or128_table.rs`) = `PCQ_TABLE` (`src/lisa_pcq_table.rs`) = `results/20261029-lisa-pcq1/train/map_K128.bin` = `e6dd165df4b46dc8eccbe0c63dac4bae1f6850c895859efd3cf7c630e43f1c51` (10648 bytes, byte-identical). Not the scalar-22 hash.

## Commands
```
cargo build --release --bin val_or128
./target/release/val_or128.exe results/20261028-praw-bench1/dataset_manifest.json results/20261029-lisa-or128-validation/val_or128.json pentax_q7_cat pentax_kx_lichen parrot_anafi_dusk
```
Production default path: `encode_with_metadata -> organize_stage2 -> classify_stage3 (Or128) -> encode_stage4 (Counted estimator)`; decode `decode_stage4 -> restore_stage1 -> decode_with_metadata`. Control: same with `Stage3Rule::Legacy4`.

## Correctness gate: PASSED for all 3
Per image: pixels exactly equal to canon (no first mismatch); metadata equal; Stage 3 residuals and contexts equal after decode; payload deterministic (two independent encodes byte-identical); Stage 3 record rule_version 2 / class_count 128, Stage 4 model_count 128, residual_count = W*H; all 128 contexts used, max id 127. `decode_stage4` enforces full payload consumption (`consumed() == payload.len()`). Legacy4 control reproduces the historical production bytes exactly and also round-trips.

Malformed/truncated data: the decoder check above and the in-crate tests (`truncated_or_extended_payload_is_rejected`, `or128_stream_tests`) cover rejection, but `Stage4Output.payload` is private, so I did NOT tamper with real-image payloads. Deviation, noted.

## Sizes (Stage 4 payload bytes; PNG/JXL/Zstd are complete files, percent = (OR-128 - X)/X, negative = OR-128 smaller)
| image | raw u16 | OR-128 | old prod | vs old | PNG9 | vs PNG | JXL e7 | vs JXL | Zstd19 | vs Zstd | raw/OR-128 |
|---|---|---|---|---|---|---|---|---|---|---|---|
| pentax_q7_cat | 24,384,000 | 10,533,683 | 10,752,399 | -2.034% | 15,000,668 | -29.78% | 10,362,059 | +1.66% | 12,967,357 | -18.77% | 2.3149 |
| pentax_kx_lichen | 24,963,072 | 12,060,697 | 12,152,091 | -0.752% | 17,283,444 | -30.22% | 12,647,502 | -4.64% | 14,316,715 | -15.76% | 2.0698 |
| parrot_anafi_dusk | 42,923,008 | 25,655,767 | 25,841,668 | -0.719% | 20,387,867 | +25.84% | 13,794,940 | +85.98% | 15,912,764 | +61.23% | 1.6730 |
| **aggregate** | 92,270,080 | 48,250,147 | 48,746,158 | **-1.018%** | 52,671,979 | -8.40% | 36,804,501 | +31.10% | 43,196,836 | +11.70% | 1.9123 |

Reference settings (from `baseline_results.json` / `tools/lisa_bench1_baselines.py`, reused, not rerun): PNG level 9, 16-bit gray; JXL lossless effort 7, 1 thread; Zstd level 19 on raw LE16 bytes; libs imagecodecs 2026.10.10, libjxl 0.12.0, zstd 1.5.7, libpng 1.6.59. LISA size = Stage 4 payload only (no container exists; metadata of a few dozen bytes excluded, as in the historical methodology). Aggregate raw/OR-128 ratio is the byte-weighted aggregate.

Reading: OR-128 beats the old production on all three (-1.02% aggregate), beats PNG and Zstd on the two 12-bit images and JXL on kx_lichen, but trails JXL by 1.66% on q7_cat. parrot_anafi_dusk remains far behind all three references; the historical report attributes this to sparse values (14-bit container, every sample a multiple of 16), which is unchanged by this rule. No tuning was done on these images.

## Timings (this machine; quiet run; `val_or128_stdout.txt`)
Historical method (Stage 4 only, best of 3):
| image | S4 enc OR-128 | S4 enc Legacy4 | S4 dec OR-128 | S4 dec Legacy4 |
|---|---|---|---|---|
| pentax_q7_cat | 0.907 s | 0.909 s | 1.310 s | 1.075 s |
| pentax_kx_lichen | 1.032 s | 1.109 s | 1.416 s | 1.247 s |
| parrot_anafi_dusk | 2.085 s | 2.070 s | 2.683 s | 2.576 s |

OR-128 Stage 4 encode is on par with Legacy4; decode is about 2-22% slower (more context-table lookups). Absolute numbers are 1.4-1.7x the historical `prod` timings (e.g. q7_cat enc 0.550 s, dec 0.684 s in the rp1-integrated report), and the Legacy4 control on the same run is equally slower, so that gap is machine/load state, not OR-128. Full pipeline (Stage 1-4 enc, one run) took 9-15 s per image and full decode 9-16 s; Stage 1-3 dominate. That was not profiled and the historical reports have no comparable figure. `run1_contended_*` is the first run, which was slowed (20-40 s) by a stray filesystem `find` I had started; I stopped it and reran. Sizes and correctness were identical in both runs. Timings are single-machine and unreplicated.

## Files in this directory
report.md, comparison.json (table above), val_or128.json + val_or128_stdout.txt (authoritative run), run1_contended_correctness.json / run1_contended_stdout.txt (first run, kept).

## Deviations / blockers
- Brief said changes were uncommitted; the tree was clean and the integration is in HEAD.
- Canon samples were not re-derived from DNG (python deps missing); hashes verified instead.
- No real-image payload tampering (private field).
- Tests: no full test suite rerun; no library code changed. Only `cargo build --release --bin val_or128` was run after adding the new bin.
