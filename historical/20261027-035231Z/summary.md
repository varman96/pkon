# Nanoscope sanity benchmark

**Corpus: 15 images, all SYNTHETIC (diagnostic, not natural images).** Initial machinery check only: tiny sample, no statistical claims, and nothing here says any future codec is competitive.

- Build: rustc 1.99.1 (58cf8f9c3 2026-10-13) release opt=3 debug_assertions=false target=x86_64-pc-windows-gnu
- Host: windows Microsoft Windows [Version 10.0.26200.9807] | Intel64 Family 6 Model 140 Stepping 1, GenuineIntel | 8 logical CPUs | git no commits yet
- Compiled features: ["sse2"]; CPU runtime features: ["sse2", "sse4.2", "popcnt", "avx", "avx2", "bmi1", "bmi2", "fma", "avx512f", "avx512bw", "avx512vl"]
- ffmpeg: ffmpeg version 9.0.2-full_build-www.gyan.dev Copyright (c) 2000-2026 the FFmpeg developers (no-op spawn median 122 ms)
- Ratio = canonical sample bytes / whole encoded bytes (higher is better). Byte counts are the primary result.
- Timing: min of 3 runs of encode/decode only. **ffmpeg-based timings include process spawn + temp-file I/O (floor ≈ the spawn figure above) and are not comparable to in-process png-rs/raw.** Peak memory not measured.

## Encoded bytes per image (ratio)

| image | format | canonical B | raw | png-rs | jxl-e7(ffmpeg) | jxl-e9(ffmpeg) | webp-l6(ffmpeg) |
|---|---|---:|---:|---:|---:|---:|---:|
| flat_rgb | rgb8 | 12288 | 12298 (1.00x) | 140 (87.77x) | 34 (361.41x) | 34 (361.41x) | 32 (384.00x) |
| checker_rgb | rgb8 | 12288 | 12298 (1.00x) | 189 (65.02x) | 71 (173.07x) | 80 (153.60x) | 64 (192.00x) |
| tile_rgb | rgb8 | 12288 | 12298 (1.00x) | 235 (52.29x) | 498 (24.67x) | 172 (71.44x) | 132 (93.09x) |
| gradient_rgb | rgb8 | 36864 | 36874 (1.00x) | 691 (53.35x) | 182 (202.55x) | 107 (344.52x) | 336 (109.71x) |
| gradient_gray_odd | gray8 | 5917 | 5927 (1.00x) | 302 (19.59x) | 63 (93.92x) | 69 (85.75x) | unsupported |
| shapes_rgb | rgb8 | 49152 | 49162 (1.00x) | 918 (53.54x) | 305 (161.15x) | 274 (179.39x) | 280 (175.54x) |
| noise_rgb | rgb8 | 12288 | 12298 (1.00x) | 12420 (0.99x) | 13523 (0.91x) | 13587 (0.90x) | 12410 (0.99x) |
| correlated_rgb | rgb8 | 27648 | 27658 (1.00x) | 13118 (2.11x) | 9736 (2.84x) | 9651 (2.86x) | 10248 (2.70x) |
| odd_rgb | rgb8 | 1683 | 1693 (0.99x) | 180 (9.35x) | 257 (6.55x) | 185 (9.10x) | 114 (14.76x) |
| alpha_rgba | rgba8 | 25600 | 25610 (1.00x) | 1269 (20.17x) | 642 (39.88x) | 500 (51.20x) | **MISMATCH** (938 B) |
| alpha_hidden_rgba | rgba8 | 16384 | 16394 (1.00x) | 7458 (2.20x) | 7283 (2.25x) | 7206 (2.27x) | **MISMATCH** (72 B) |
| noise_rgba | rgba8 | 9216 | 9226 (1.00x) | 9332 (0.99x) | 10173 (0.91x) | 10227 (0.90x) | **MISMATCH** (9358 B) |
| graya_gradient | graya8 | 2400 | 2410 (1.00x) | 98 (24.49x) | 79 (30.38x) | 49 (48.98x) | unsupported |
| gray16_gradient | gray16 | 8192 | 8202 (1.00x) | 2245 (3.65x) | 1835 (4.46x) | 1730 (4.74x) | unsupported |
| rgb16_smooth | rgb16 | 13824 | 13834 (1.00x) | 8169 (1.69x) | 5417 (2.55x) | 5139 (2.69x) | unsupported |

## Totals (only images every listed codec reproduced exactly)

### All baselines: 8 of 15 images exact for all of [raw, png-rs, jxl-e7(ffmpeg), jxl-e9(ffmpeg), webp-l6(ffmpeg)]

| codec | encoded B | ratio | enc ms (sum of bests) | dec ms (sum of bests) |
|---|---:|---:|---:|---:|
| raw | 164579 | 1.000x | 0.0 | 0.1 |
| png-rs | 27891 | 5.898x | 15.7 | 1.9 |
| jxl-e7(ffmpeg) | 24606 | 6.685x | 910.9 | 948.6 |
| jxl-e9(ffmpeg) | 24090 | 6.829x | 1460.3 | 1086.7 |
| webp-l6(ffmpeg) | 23616 | 6.966x | 2465.9 | 776.4 |

### PNG vs JPEG XL: 15 of 15 images exact for all of [png-rs, jxl-e7(ffmpeg), jxl-e9(ffmpeg)]

| codec | encoded B | ratio | enc ms (sum of bests) | dec ms (sum of bests) |
|---|---:|---:|---:|---:|
| png-rs | 56764 | 4.334x | 32.0 | 3.2 |
| jxl-e7(ffmpeg) | 50098 | 4.911x | 1838.2 | 1869.2 |
| jxl-e9(ffmpeg) | 49010 | 5.020x | 2418.1 | 1857.0 |

## Correctness findings

- **webp-l6(ffmpeg)** on `alpha_rgba` → mismatch: first mismatch at (x=0, y=0) channel B: expected 128, got 0 (1081 differing samples total)
- **webp-l6(ffmpeg)** on `alpha_hidden_rgba` → mismatch: first mismatch at (x=0, y=0) channel R: expected 202, got 0 (6118 differing samples total)
- **webp-l6(ffmpeg)** on `noise_rgba` → mismatch: first mismatch at (x=0, y=8) channel R: expected 109, got 0 (27 differing samples total)
