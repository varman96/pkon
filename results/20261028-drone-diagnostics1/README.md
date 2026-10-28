# Diagnostics: `parrot_anafi_dusk` (read-only, existing artifacts only)

Notation: **[R]** = recorded in an existing artifact; **[D]** = derived arithmetic (calculation shown). No causes are inferred.
n = 21,461,504 samples (5344 × 4016). Bayer planes: 4 × 5,365,376.

## 1. Source / sample-value statistics

| Item | Value | Source |
|---|---|---|
| Dimensions | 5344 × 4016 | [R] manifest |
| Declared / effective bit depth | 16 / 14 | [R] manifest (`declared_bits`, `bit_depth`) |
| Sample count | 21,461,504 | [R] mixer/cloud `n`; [D] 5344 × 4016 = 21,461,504 |
| Min / max sample | 816 / 16368 | [R] manifest |
| White level | 16383 | [R] manifest |
| Black level (raw list as recorded) | [262144, 256, 262144, 256, 262144, 256, 262144, 256] | [R] manifest; not interpreted here |
| CFA / compression / layout | [2,1,1,0], 2×2, origin (0,0); DNG compression NONE; Stage 2 `bayer_planes` | [R] |
| DNG bytes / canon (raw u16) bytes | 43,001,162 / 42,923,008 | [R] |
| Divisible by 16 | "every sample is a multiple of 16" | [R] bench1 report.md line 145 (statement only; no count or percentage stored) |
| Divisible by 8, 4, 2 | counts/% not recorded (a multiple of 16 is also a multiple of 8, 4, 2, so [D] 100% if the statement holds) | — |
| Value range used | "only 6% … of the 14-bit value range occurs" | [R] report.md line 145 (rounded) |
| Distinct sample values | not recorded as a number | — |

Derived cross-checks (not recorded as such):
- Multiples of 16 between min and max inclusive: (16368 − 816)/16 + 1 = 973. 973 / 16384 = 5.94% of the 14-bit range. This is an upper bound on the distinct count given the ÷16 statement; the actual count is unrecorded.
- Span: (max − min)/white level = 15552/16383 = 94.93%.

Per-Bayer-plane source statistics (min/max, distinct count, divisibility, range utilisation): **not recorded**. Only plane sample count is: 5,365,376 each [R] (`planes[].n`).

## 2. Stage 1 residual statistics (from `praw_results.json`)

| Item | Value | Source |
|---|---|---|
| Residual count | 21,461,504 | [R] `stage1.n` |
| Zero fraction | 0.0077243887 (0.772%) | [R] |
| Zero count | 165,777 | [D] 21,461,504 × 0.0077243887 |
| Mean zigzag residual | 559.5577 | [R] |
| Order-0 entropy | 10.1644 bits/sample | [R] `stage1.h0_bits` |
| Residual min / max | not recorded | — |
| Residual histogram | not recorded | — |
| Residuals ≥ 1024 | 3,534,982 = 16.471% | [R] mixer `big_residuals_ge_1024` (= cloud `escapes`); [D] 3,534,982 / 21,461,504 |
| Residuals > 255, > 4095, other thresholds | not recorded | — |

Whether the ≥1024 threshold is on the signed or zigzag scale is not stated in the artifacts reviewed.

Per Bayer plane (order as in `planes[]`; CFA colour labels not stored per plane):

| Plane | n | Zero % [R] | Zero count [D] (n × frac) | Mean zigzag [R] | H0 bits [R] |
|---|--:|--:|--:|--:|--:|
| 0 | 5,365,376 | 0.8253% | 44,282 | 440.21 | 9.7043 |
| 1 | 5,365,376 | 0.6860% | 36,805 | 876.57 | 10.7122 |
| 2 | 5,365,376 | 0.8362% | 44,866 | 273.22 | 9.3763 |
| 3 | 5,365,376 | 0.7422% | 39,824 | 648.22 | 10.0796 |

Per-plane min/max and large-residual percentages: **not recorded**.

## 3. Stage 3 activity classification (rule version 1)

| Class | n [R] | Fraction [R] | Class H0 bits [R] | Mean zigzag [R] |
|---|--:|--:|--:|--:|
| Low | 71,921 | 0.335% | 0.6567 | 24.78 |
| Moderate | 42,355 | 0.197% | 7.6775 | 116.80 |
| High | 1,602,424 | 7.467% | 7.8410 | 88.60 |
| VeryHigh | 19,744,804 | 92.001% | 10.3107 | 600.68 |

- [D] 71,921 + 42,355 + 1,602,424 + 19,744,804 = 21,461,504 = n.
- Existing diagnostic on saturation: bench1 report.md states "on 14/16-bit images 86–99% of samples land in VeryHigh (fixed zigzag-score threshold 48)"; this image is 92.0% [R]. report.md labels the thresholds "mis-scaled" as a hypothesis with effect size untested. The `[3, 12, 48]` values are not echoed in this image's artifacts (only "48" appears in report.md).
- Static class-conditioned order-0 cost: 27,064,949.54 B [R] (`class_static_h0_bytes`); [D] ×8 / 21,461,504 = 10.0885 b/sample (report table shows 10.089).
- Per-plane class distributions: **not recorded**. Nothing was recomputed.

## 4. Mixer and cloud coding costs

### Mixer (`results/20261028-praw-mixer1/results.json`)

| Item | Value | Source |
|---|--:|---|
| Payload | 25,092,263 B | [R] |
| Header overhead | 26 B | [R] |
| Total | 25,092,289 B | [R] |
| Mixture ideal cost | 25,092,259.75 B | [R] |
| Payload − ideal | 3.25 B | [R] `payload_minus_mixture_ideal_bytes` |
| Bits/sample (payload) | 9.3534 | [R]; [D] 25,092,263 × 8 / 21,461,504 |

Per-model ideal cost (whole image):

| Model | Ideal bytes [R] | vs mixture [D] (model − 25,092,259.75) |
|---|--:|--:|
| global | 27,273,606.30 | +2,181,346.55 (+8.69%) |
| recent-history | 25,191,767.17 | +99,507.42 (+0.40%) |
| spatial | 26,100,302.16 | +1,008,042.41 (+4.02%) |
| activity-conditioned | 25,956,768.89 | +864,509.14 (+3.45%) |

Final weights [global, recent, spatial, activity] = [0.003320, 0.965552, 0.012786, 0.018342] [R].

Large residuals ≥1024 in the mixture [R]: count 3,534,982 (16.471%); ideal cost 5,177,068.76 B = 20.632% of the mixture. [D] remainder: 25,092,259.75 − 5,177,068.76 = 19,915,190.99 B over 21,461,504 − 3,534,982 = 17,926,522 residuals = 8.887 bits each.

Sampled segments: 16 segments of 1,341,344 samples each, in recorded order. Mean weights [glob, recent, spatial, activity]; bps = bits/sample.

| Seg | Mean weights | Mixture bps | Model bps (glob, recent, spatial, activity) |
|--:|---|--:|---|
| 1 | .092 .558 .033 .317 | 10.466 | 10.795 10.520 10.787 10.600 |
| 2 | .022 .758 .064 .157 | 10.050 | 10.650 10.099 10.329 10.249 |
| 3 | .006 .714 .021 .258 | 8.092 | 8.914 8.122 8.354 8.162 |
| 4 | .008 .327 .035 .630 | 7.922 | 8.491 7.967 8.061 7.952 |
| 5 | .008 .822 .129 .042 | 11.569 | 13.271 11.620 12.220 12.190 |
| 6 | .014 .812 .138 .037 | 10.936 | 11.802 10.993 11.406 11.460 |
| 7 | .009 .908 .056 .027 | 8.756 | 9.361 8.775 9.060 9.096 |
| 8 | .009 .829 .110 .052 | 8.504 | 9.094 8.529 8.714 8.710 |
| 9 | .014 .950 .016 .021 | 10.022 | 10.557 10.037 10.414 10.423 |
| 10 | .014 .822 .064 .101 | 9.764 | 10.269 9.809 10.026 10.003 |
| 11 | .008 .413 .048 .532 | 8.115 | 8.700 8.159 8.229 8.150 |
| 12 | .008 .420 .049 .524 | 7.969 | 8.549 8.011 8.085 8.004 |
| 13 | .003 .989 .004 .005 | 10.809 | 12.672 10.816 11.963 11.940 |
| 14 | .005 .901 .059 .035 | 10.292 | 11.675 10.346 11.043 10.996 |
| 15 | .006 .912 .031 .051 | 8.248 | 9.026 8.273 8.593 8.553 |
| 16 | .007 .854 .043 .096 | 8.140 | 8.837 8.173 8.382 8.321 |

Per-plane mixer byte breakdown: **not recorded**.

### Original cloud (`results/20261028-praw-cloud1/results.json`)

| Item | Value | Source |
|---|---|---|
| Payload / total | 27,203,831 / 27,203,857 B (header 26) | [R] |
| Escapes | 3,534,982 | [R] |
| Escape rate | 16.471% | [D] 3,534,982 / 21,461,504 |
| Ideal tree cost | 22,101,330.61 B | [R] |
| Ideal escape-tail cost | 5,102,496.81 B | [R] |
| Tree + tail | 27,203,827.43 B | [D] 22,101,330.61 + 5,102,496.81 (payload − this = 3.57 B) |
| Bits/sample (payload) | 10.1405 | [R] |
| Tail share of payload | 18.757% | [D] 5,102,496.81 / 27,203,831 |

## 5. Existing codec comparison (frozen benchmark `results/20261028-praw-bench1/`)

| Codec | Bytes [R] | Ratio vs canon 42,923,008 [R] | Bits/sample [D] (bytes × 8 / 21,461,504) |
|---|--:|--:|--:|
| PRAW Stage 4 | 25,841,668 | 1.661 | 9.6328 |
| Cloud (total) | 27,203,857 | — | 10.1405 (payload) |
| Mixer (total) | 25,092,289 | — ([D] 42,923,008 / 25,092,289 = 1.711) | 9.3534 (payload) |
| JPEG XL (libjxl 0.12.0) | 13,794,940 | 3.112 | 5.1422 |
| Zstd-19 (1.5.7) | 15,912,764 | 2.697 | 5.9313 |
| PNG | 20,387,867 | 2.105 | 7.5996 |

All codecs round-trip exact [R].

Differences [D]:
- Mixer payload − PRAW: 25,092,263 − 25,841,668 = −749,405 B (−2.900% [R]).
- PRAW − JXL: 25,841,668 − 13,794,940 = +12,046,728 B (PRAW = 1.873 × JXL).
- Mixer total − JXL: 25,092,289 − 13,794,940 = +11,297,349 B (+81.895% [R]).
- Share of the PRAW–JXL gap covered by the mixer: 749,405 / 12,046,728 = 6.22%.
- bench1 report: JXL is −46.6% vs PRAW [R].
- Recorded metrics for this image: Stage 1 zero 0.8%, mean zigzag 559.6, H0 10.164 b/s, VeryHigh 92.0%, class-H0 10.089 b/s, payload 9.633 b/s. PNG and Zstd are both smaller than PRAW. The only value-set metric is the report's "multiple of 16 / ~6% of range" statement; report.md labels its link to the gap a hypothesis ("not proven").

## 6. Missing diagnostics

See the list below; none was generated.

## Unavailable requested numbers
- Counts/percentages of samples divisible by 16/8/4/2 (only a statement that all are multiples of 16).
- Distinct-value count and exact range-utilisation fraction (only "~6%").
- All per-plane source statistics (min/max, distinct count, divisibility, utilisation).
- Residual min/max, histogram, thresholds >255 and >4095 (≥1024 only), globally and per plane.
- Per-plane large-residual percentages.
- Per-plane activity-class distributions.
- Per-plane or per-segment mixer byte costs (only per-segment bits/sample over 16 segments).
- `[3, 12, 48]` thresholds echoed in this image's artifacts (only "48" in report.md).
- Signed-vs-zigzag scale of the ≥1024 field.

## Files inspected
- `results/20261028-praw-bench1/`: `dataset_manifest.json`, `per_image.csv`, `praw_results.json`, `baseline_results.json`, `baseline_stdout.txt`, `praw_stdout.txt`, `report.md`, `tables.md`
- `results/20261028-praw-cloud1/`: `README.md`, `results.json`
- `results/20261028-praw-mixer1/`: `README.md`, `results.json`, `stdout.txt`
- `tools/praw_bench1_selection.json`

## Confirmation
No source code or existing artifact was modified, and no compression, decoding, or benchmark was run. The only additions are this directory and this file. Arithmetic used inline calculator commands on the recorded numbers only.
