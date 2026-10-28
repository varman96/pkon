# PKON2 25-image diverse compression benchmark

Results dir: `results/20261028-015857Z-bench25-diversity`. Production default under test: PKON2 REL6, codec id 3, written by `pkonfile::encode` (24-byte header + payload; all stored sizes include the header). Images: 25. Images with all five configurations valid and exactly reconstructed: 25. Invalid/failed records: 0.

## Input representation

Every source file was decoded once (Rust `image` crate, `Canonical::load`) to interleaved 8-bit samples and centre-cropped to at most 1024x1024 (the same crop pipeline used by every earlier PKON2 report). All codecs received the identical crop buffer (checked by SHA-256 in the manifest; the pre-encode copy is `manifest_pre_encode.json`). RGB sources stay 3-channel (raw 24 bpp); PNG-origin RGBA sources stay 4-channel (raw 32 bpp) for every codec. Ratio = canonical raw bytes / stored bytes; bpp = stored bits / pixels. **The measurement is of the decoded 1024x1024 crops, not of the full-resolution photographs.**

## 1. Manifest (25 images)

| # | name | set | category | source fmt | source px | crop | layout | raw B | MED MAE (G) | zero-resid share | mean G | clipped (0/255) | selection note |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| 1 | outdoor_0 | retained23 | outdoor | jpeg | 4000x3000 | 1024x1024 | rgb8 | 3,145,728 | 4.04 | 0.11 | 118 | 0.04% | mountain landscape, high-frequency detail |
| 2 | indoor_0 | retained23 | indoor | jpeg | 4592x3448 | 1024x1024 | rgb8 | 3,145,728 | 2.15 | 0.55 | 164 | 0.00% | bright indoor scene, smooth surfaces |
| 3 | portrait_0 | retained23 | portrait | jpeg | 2034x2717 | 1024x1024 | rgb8 | 3,145,728 | 1.30 | 0.33 | 92 | 0.35% | people: portrait |
| 4 | pet_0 | retained23 | pet | jpeg | 3607x2234 | 1024x1024 | rgb8 | 3,145,728 | 2.15 | 0.18 | 87 | 0.19% | animal: fur detail |
| 5 | pet2_1 | retained23 | pet2 | jpeg | 4000x2992 | 1024x1024 | rgb8 | 3,145,728 | 1.71 | 0.37 | 212 | 7.79% | animal: heavily over-exposed regions |
| 6 | food_0 | retained23 | food | jpeg | 1572x1078 | 1024x1024 | rgb8 | 3,145,728 | 5.93 | 0.12 | 100 | 0.56% | mixed detail, close range |
| 7 | foliage_0 | retained23 | foliage | jpeg | 1831x1831 | 1024x1024 | rgb8 | 3,145,728 | 7.56 | 0.10 | 106 | 3.35% | dense foliage, fine texture |
| 8 | night_0 | retained23 | night | jpeg | 2000x1329 | 1024x1024 | rgb8 | 3,145,728 | 10.07 | 0.09 | 44 | 2.83% | low-light, sensor noise |
| 9 | night_1 | retained23 | night | jpeg | 4949x3140 | 1024x1024 | rgb8 | 3,145,728 | 2.14 | 0.17 | 71 | 0.00% | low-light, comparatively clean |
| 10 | blurry_1 | retained23 | blurry | png | 2160x1620 | 1024x1024 | rgba8 | 4,194,304 | 2.03 | 0.28 | 147 | 0.14% | PNG origin, RGBA, defocus blur |
| 11 | screen_0 | retained23 | screen | png | 2880x1628 | 1024x1024 | rgba8 | 4,194,304 | 3.63 | 0.93 | 226 | 56.94% | PNG origin, RGBA, UI screenshot (graphic content) |
| 12 | text_1 | retained23 | text | jpeg | 2573x1959 | 1024x1024 | rgb8 | 3,145,728 | 5.77 | 0.09 | 79 | 0.58% | sharp edges: printed text |
| 13 | document_1 | retained23 | document | jpeg | 3060x4080 | 1024x1024 | rgb8 | 3,145,728 | 2.16 | 0.44 | 165 | 5.08% | document: flat paper, clipped highlights |
| 14 | clouds_n0 | added | clouds | jpeg | 4000x3000 | 1024x1024 | rgb8 | 3,145,728 | 2.25 | 0.33 | 142 | 0.19% | sky / clouds |
| 15 | skyline_n0 | added | skyline | jpeg | 3648x2736 | 1024x1024 | rgb8 | 3,145,728 | 0.97 | 0.44 | 187 | 0.00% | sky + architecture, very smooth |
| 16 | sunset_n0 | added | sunset | jpeg | 1980x1320 | 1024x1024 | rgb8 | 3,145,728 | 1.91 | 0.32 | 70 | 0.05% | dusk sky, colour gradients |
| 17 | snow_n0 | added | snow | jpeg | 2048x1365 | 1024x1024 | rgb8 | 3,145,728 | 5.56 | 0.26 | 150 | 0.21% | snow, bright low-contrast texture |
| 18 | path_n0 | added | path | jpeg | 2048x1366 | 1024x1024 | rgb8 | 3,145,728 | 13.82 | 0.04 | 88 | 0.24% | extreme fine texture (gravel/ground) |
| 19 | building_n0 | added | building | jpeg | 2400x1957 | 1024x1024 | rgb8 | 3,145,728 | 1.48 | 0.40 | 149 | 0.00% | architecture: facade |
| 20 | church_n0 | added | church | jpeg | 3456x4608 | 1024x1024 | rgb8 | 3,145,728 | 4.03 | 0.11 | 90 | 0.07% | architecture: detailed stonework |
| 21 | child_n0 | added | child | jpeg | 3152x2337 | 1024x1024 | rgb8 | 3,145,728 | 0.70 | 0.45 | 106 | 0.00% | people: smooth skin, shallow depth of field |
| 22 | woman_n0 | added | woman | jpeg | 1366x2048 | 1024x1024 | rgb8 | 3,145,728 | 1.20 | 0.72 | 235 | 2.05% | people: high-key portrait |
| 23 | bird_n0 | added | bird | jpeg | 3000x4000 | 1024x1024 | rgb8 | 3,145,728 | 1.05 | 0.44 | 132 | 3.91% | animal: bokeh / background blur |
| 24 | waterfall_n0 | added | waterfall | jpeg | 2048x1365 | 1024x1024 | rgb8 | 3,145,728 | 3.87 | 0.29 | 94 | 1.00% | moving water, mixed detail |
| 25 | lake_n0 | added | lake | jpeg | 4608x2566 | 1024x1024 | rgb8 | 3,145,728 | 4.09 | 0.25 | 149 | 0.01% | water + reflections |

Paths, source-file SHA-256 and crop-buffer SHA-256 for every image are in `manifest.csv` (and `manifest_pre_encode.json`). Descriptors (MED MAE etc.) are computed on the G plane of the crop by the harness and are used only for grouping; they are not part of any codec. Category labels come from the existing corpus manifests; they were not re-verified visually here.

### Diversity summary

- From the existing 23-image corpus: 13; added from the extra 29-image pool (`data/new50`): 12. Selection was by content/statistic diversity, made before any encoding.
- Layouts: rgb8: 23, rgba8: 2. Source formats: image/jpeg: 23, image/png: 2.
- Alpha: the RGBA inputs are PNG-origin; their alpha plane is constant 255 in the crops used (measured below), so alpha reconstruction is tested for layout and exactness but the plane carries no information.
- MED MAE (G) spans 0.70 to 13.82 (median 2.16); zero-residual share 0.04-0.93; mean G 44-235; clipped share up to 56.9%.
- Categories: bird (1), blurry (1), building (1), child (1), church (1), clouds (1), document (1), foliage (1), food (1), indoor (1), lake (1), night (2), outdoor (1), path (1), pet (1), pet2 (1), portrait (1), screen (1), skyline (1), snow (1), sunset (1), text (1), waterfall (1), woman (1).
- Categories NOT represented as such in this set (no image selected for them): none claimed beyond the list above; e.g. there is no synthetic noise image and no pure-gradient image — smooth content is represented only by the lowest-MAE photographs.

## 2. Encoders, versions, settings

| config | implementation | settings actually passed | lossless confirmation |
|---|---|---|---|
| PKON2 | in-process Rust, `nanoscope::pkonfile::encode/decode`, header codec byte read from every output (must be 3) | none (production default, `CODEC_CURRENT` = 3) | exact decode compare of every rep; header CRC-32 matches the input pixels; flipped payload bit rejected |
| JPEG XL e7 | `ffmpeg version 9.0.2-full_build-www.gyan.dev Copyright (c) 2000-2026 the FFmpeg developers` -> libjxl wrapper, one process per call | `-c:v libjxl -distance 0 -effort 7 -pix_fmt <rgb24\|rgba> -threads 1` | `distance 0` = lossless (ffmpeg doc: 'zero = lossless'); exact decode compare |
| JPEG XL e9 | same | `-distance 0 -effort 9 -threads 1` | same |
| WebP m6 | same ffmpeg, libwebp wrapper | `-c:v libwebp -lossless 1 -compression_level 6 -quality 100 -pix_fmt bgra -threads 1` | output contains a `VP8L` chunk (payload_bytes>0 in results.json); exact decode compare |
| PNG | in-process Rust: image 0.25.10 / png 0.18.1 | image::PngEncoder CompressionType::Best, FilterType::Adaptive, no metadata | PNG is lossless by definition; exact decode compare |

- libjxl / libwebp library versions: not reported by the ffmpeg build (version strings not found in binary); only ffmpeg 9.0.2-full_build-www.gyan.dev is known

Thread counts: PKON2 and PNG are single-threaded; ffmpeg encoders/decoders were run with `-threads 1`. Machine: 8 logical CPUs (Intel i7-1185G7 laptop, Windows 11). Timing procedure: per (image, codec) 1 untimed warm-up round trip, then 3 timed rounds; each round = encode -> decode from the encoded bytes -> exact compare (all rounds, including warm-up, are compared). Timed regions are exactly `Codec::encode` / `Codec::decode`. Reported timing = median of the reps; min/max are in `timings.csv`. Codecs run sequentially, one at a time, with no other workload started by the harness.

**Timing caveat:** JXL and WebP are driven through the `ffmpeg` CLI, so their encode and decode times include process spawn, raw temp-file write/read and ffmpeg's container/demux layer. Measured no-op floor on a 33x17 image (best of 5, ms): pkon2-file enc 0.4 / dec 0.4; jxl-e7-t1 enc 41.3 / dec 42.8; jxl-e9-t1 enc 44.4 / dec 44.1; webp-m6-t1 enc 57.9 / dec 42.1; png-rs enc 0.2 / dec 0.0. PKON2 and PNG are in-process and carry no such overhead; timings are therefore not like-for-like across those two groups. The PNG encoder is the Rust `png` crate at its highest compression setting; no stronger PNG optimiser (zopfli/oxipng) was available, so 'best available' means that.

## 3. Exact reconstruction

| image | PKON2 (.pkon2, codec 3) | JXL lossless e7 | JXL lossless e9 | WebP lossless m6 | PNG (png-rs best) |
|---|---|---|---|---|---|
| outdoor_0 | exact | exact | exact | exact | exact |
| indoor_0 | exact | exact | exact | exact | exact |
| portrait_0 | exact | exact | exact | exact | exact |
| pet_0 | exact | exact | exact | exact | exact |
| pet2_1 | exact | exact | exact | exact | exact |
| food_0 | exact | exact | exact | exact | exact |
| foliage_0 | exact | exact | exact | exact | exact |
| night_0 | exact | exact | exact | exact | exact |
| night_1 | exact | exact | exact | exact | exact |
| blurry_1 | exact | exact | exact | exact | exact |
| screen_0 | exact | exact | exact | exact | exact |
| text_1 | exact | exact | exact | exact | exact |
| document_1 | exact | exact | exact | exact | exact |
| clouds_n0 | exact | exact | exact | exact | exact |
| skyline_n0 | exact | exact | exact | exact | exact |
| sunset_n0 | exact | exact | exact | exact | exact |
| snow_n0 | exact | exact | exact | exact | exact |
| path_n0 | exact | exact | exact | exact | exact |
| building_n0 | exact | exact | exact | exact | exact |
| church_n0 | exact | exact | exact | exact | exact |
| child_n0 | exact | exact | exact | exact | exact |
| woman_n0 | exact | exact | exact | exact | exact |
| bird_n0 | exact | exact | exact | exact | exact |
| waterfall_n0 | exact | exact | exact | exact | exact |
| lake_n0 | exact | exact | exact | exact | exact |

125/125 (image, codec) outputs reconstructed every sample exactly in all reps (including alpha where present) and encoded deterministically across reps. Failures: 0.

PKON2 additionally: header codec id = 3 on 25/25 files, header CRC/dimensions/layout match the input on 25/25.

## 4. Stored size

### Per-image stored bytes (complete file; `.pkon2` includes its 24-byte header)

| image | layout | PKON2 (.pkon2, codec 3) | JXL lossless e7 | JXL lossless e9 | WebP lossless m6 | PNG (png-rs best) | smallest |
|---|---|---|---|---|---|---|---|
| outdoor_0 | rgb8 | **761,096** | 824,676 | 821,091 | 989,404 | 1,417,416 | PKON2 (.pkon2, codec 3) |
| indoor_0 | rgb8 | **343,593** | 365,779 | 359,179 | 447,900 | 775,015 | PKON2 (.pkon2, codec 3) |
| portrait_0 | rgb8 | 393,153 | 398,485 | **390,589** | 483,336 | 880,271 | JXL lossless e9 |
| pet_0 | rgb8 | 724,469 | 709,641 | **698,480** | 911,822 | 1,279,408 | JXL lossless e9 |
| pet2_1 | rgb8 | **479,628** | 495,526 | 488,803 | 603,644 | 983,672 | PKON2 (.pkon2, codec 3) |
| food_0 | rgb8 | 1,255,156 | 1,264,540 | **1,255,067** | 1,400,906 | 1,996,090 | JXL lossless e9 |
| foliage_0 | rgb8 | **1,178,415** | 1,184,936 | 1,180,499 | 1,357,336 | 2,016,330 | PKON2 (.pkon2, codec 3) |
| night_0 | rgb8 | 1,911,500 | **1,881,297** | 1,891,057 | 1,933,568 | 2,290,163 | JXL lossless e7 |
| night_1 | rgb8 | **612,703** | 712,137 | 701,535 | 705,974 | 1,046,156 | PKON2 (.pkon2, codec 3) |
| blurry_1 | rgba8 | 669,903 | 667,721 | **655,667** | 828,564 | 1,280,963 | JXL lossless e9 |
| screen_0 | rgba8 | 136,850 | 121,247 | 112,699 | **70,464** | 229,481 | WebP lossless m6 |
| text_1 | rgb8 | 1,029,283 | 1,008,865 | **998,577** | 1,239,866 | 1,716,246 | JXL lossless e9 |
| document_1 | rgb8 | **399,299** | 426,214 | 413,808 | 485,044 | 796,743 | PKON2 (.pkon2, codec 3) |
| clouds_n0 | rgb8 | **445,873** | 570,294 | 557,863 | 534,082 | 915,485 | PKON2 (.pkon2, codec 3) |
| skyline_n0 | rgb8 | **323,207** | 464,276 | 454,777 | 403,066 | 691,009 | PKON2 (.pkon2, codec 3) |
| sunset_n0 | rgb8 | **892,162** | 913,743 | 903,717 | 1,038,204 | 1,347,491 | PKON2 (.pkon2, codec 3) |
| snow_n0 | rgb8 | **1,223,563** | 1,252,009 | 1,248,716 | 1,340,534 | 1,776,436 | PKON2 (.pkon2, codec 3) |
| path_n0 | rgb8 | **1,703,432** | 1,713,272 | 1,717,184 | 1,811,926 | 2,439,490 | PKON2 (.pkon2, codec 3) |
| building_n0 | rgb8 | **684,544** | 717,273 | 708,820 | 820,972 | 1,145,058 | PKON2 (.pkon2, codec 3) |
| church_n0 | rgb8 | **714,230** | 729,588 | 724,747 | 841,722 | 1,420,194 | PKON2 (.pkon2, codec 3) |
| child_n0 | rgb8 | **403,061** | 416,191 | 412,043 | 509,778 | 857,943 | PKON2 (.pkon2, codec 3) |
| woman_n0 | rgb8 | **333,701** | 367,893 | 361,013 | 393,378 | 597,287 | PKON2 (.pkon2, codec 3) |
| bird_n0 | rgb8 | **330,078** | 344,952 | 334,976 | 406,096 | 752,782 | PKON2 (.pkon2, codec 3) |
| waterfall_n0 | rgb8 | **1,026,434** | 1,061,597 | 1,055,859 | 1,175,322 | 1,518,762 | PKON2 (.pkon2, codec 3) |
| lake_n0 | rgb8 | **829,589** | 852,564 | 846,434 | 972,500 | 1,491,870 | PKON2 (.pkon2, codec 3) |

### Aggregate (25 images, raw 80,740,352 B, 26,214,400 px)

| codec | total bytes | vs PKON2 | bpp (stored bits / px) | aggregate ratio | mean per-image ratio | median per-image ratio |
|---|---|---|---|---|---|---|
| PKON2 (.pkon2, codec 3) | 18,804,922 | +0.00% | 5.739 | 4.294 | 6.374 | 4.595 |
| JXL lossless e7 | 19,464,716 | +3.51% | 5.940 | 4.148 | 6.174 | 4.417 |
| JXL lossless e9 | 19,293,200 | +2.60% | 5.888 | 4.185 | 6.355 | 4.484 |
| WebP lossless m6 | 21,705,408 | +15.42% | 6.624 | 3.720 | 6.633 | 3.832 |
| PNG (png-rs best) | 31,661,761 | +68.37% | 9.662 | 2.550 | 3.440 | 2.747 |

### Per-image bits per pixel and compression ratio (vs canonical raw)

| image | raw bpp | PKON2 (.pkon2, codec 3) bpp / ratio | JXL lossless e7 bpp / ratio | JXL lossless e9 bpp / ratio | WebP lossless m6 bpp / ratio | PNG (png-rs best) bpp / ratio |
|---|---|---|---|---|---|---|
| outdoor_0 | 24 | 5.807 / 4.133 | 6.292 / 3.815 | 6.264 / 3.831 | 7.549 / 3.179 | 10.814 / 2.219 |
| indoor_0 | 24 | 2.621 / 9.155 | 2.791 / 8.600 | 2.740 / 8.758 | 3.417 / 7.023 | 5.913 / 4.059 |
| portrait_0 | 24 | 3.000 / 8.001 | 3.040 / 7.894 | 2.980 / 8.054 | 3.688 / 6.508 | 6.716 / 3.574 |
| pet_0 | 24 | 5.527 / 4.342 | 5.414 / 4.433 | 5.329 / 4.504 | 6.957 / 3.450 | 9.761 / 2.459 |
| pet2_1 | 24 | 3.659 / 6.559 | 3.781 / 6.348 | 3.729 / 6.436 | 4.605 / 5.211 | 7.505 / 3.198 |
| food_0 | 24 | 9.576 / 2.506 | 9.648 / 2.488 | 9.575 / 2.506 | 10.688 / 2.245 | 15.229 / 1.576 |
| foliage_0 | 24 | 8.991 / 2.669 | 9.040 / 2.655 | 9.006 / 2.665 | 10.356 / 2.318 | 15.383 / 1.560 |
| night_0 | 24 | 14.584 / 1.646 | 14.353 / 1.672 | 14.428 / 1.663 | 14.752 / 1.627 | 17.473 / 1.374 |
| night_1 | 24 | 4.675 / 5.134 | 5.433 / 4.417 | 5.352 / 4.484 | 5.386 / 4.456 | 7.982 / 3.007 |
| blurry_1 | 32 | 5.111 / 6.261 | 5.094 / 6.282 | 5.002 / 6.397 | 6.321 / 5.062 | 9.773 / 3.274 |
| screen_0 | 32 | 1.044 / 30.649 | 0.925 / 34.593 | 0.860 / 37.217 | 0.538 / 59.524 | 1.751 / 18.277 |
| text_1 | 24 | 7.853 / 3.056 | 7.697 / 3.118 | 7.619 / 3.150 | 9.459 / 2.537 | 13.094 / 1.833 |
| document_1 | 24 | 3.046 / 7.878 | 3.252 / 7.381 | 3.157 / 7.602 | 3.701 / 6.485 | 6.079 / 3.948 |
| clouds_n0 | 24 | 3.402 / 7.055 | 4.351 / 5.516 | 4.256 / 5.639 | 4.075 / 5.890 | 6.985 / 3.436 |
| skyline_n0 | 24 | 2.466 / 9.733 | 3.542 / 6.776 | 3.470 / 6.917 | 3.075 / 7.804 | 5.272 / 4.552 |
| sunset_n0 | 24 | 6.807 / 3.526 | 6.971 / 3.443 | 6.895 / 3.481 | 7.921 / 3.030 | 10.281 / 2.335 |
| snow_n0 | 24 | 9.335 / 2.571 | 9.552 / 2.513 | 9.527 / 2.519 | 10.227 / 2.347 | 13.553 / 1.771 |
| path_n0 | 24 | 12.996 / 1.847 | 13.071 / 1.836 | 13.101 / 1.832 | 13.824 / 1.736 | 18.612 / 1.290 |
| building_n0 | 24 | 5.223 / 4.595 | 5.472 / 4.386 | 5.408 / 4.438 | 6.264 / 3.832 | 8.736 / 2.747 |
| church_n0 | 24 | 5.449 / 4.404 | 5.566 / 4.312 | 5.529 / 4.340 | 6.422 / 3.737 | 10.835 / 2.215 |
| child_n0 | 24 | 3.075 / 7.805 | 3.175 / 7.558 | 3.144 / 7.634 | 3.889 / 6.171 | 6.546 / 3.667 |
| woman_n0 | 24 | 2.546 / 9.427 | 2.807 / 8.551 | 2.754 / 8.714 | 3.001 / 7.997 | 4.557 / 5.267 |
| bird_n0 | 24 | 2.518 / 9.530 | 2.632 / 9.119 | 2.556 / 9.391 | 3.098 / 7.746 | 5.743 / 4.179 |
| waterfall_n0 | 24 | 7.831 / 3.065 | 8.099 / 2.963 | 8.056 / 2.979 | 8.967 / 2.676 | 11.587 / 2.071 |
| lake_n0 | 24 | 6.329 / 3.792 | 6.505 / 3.690 | 6.458 / 3.716 | 7.420 / 3.235 | 11.382 / 2.109 |

### Per-image size difference vs PKON2 (positive = codec larger than PKON2)

| image | JXL lossless e7 | JXL lossless e9 | WebP lossless m6 | PNG (png-rs best) |
|---|---|---|---|---|
| outdoor_0 | +8.35% | +7.88% | +30.00% | +86.23% |
| indoor_0 | +6.46% | +4.54% | +30.36% | +125.56% |
| portrait_0 | +1.36% | -0.65% | +22.94% | +123.90% |
| pet_0 | -2.05% | -3.59% | +25.86% | +76.60% |
| pet2_1 | +3.31% | +1.91% | +25.86% | +105.09% |
| food_0 | +0.75% | -0.01% | +11.61% | +59.03% |
| foliage_0 | +0.55% | +0.18% | +15.18% | +71.11% |
| night_0 | -1.58% | -1.07% | +1.15% | +19.81% |
| night_1 | +16.23% | +14.50% | +15.22% | +70.74% |
| blurry_1 | -0.33% | -2.13% | +23.68% | +91.22% |
| screen_0 | -11.40% | -17.65% | -48.51% | +67.69% |
| text_1 | -1.98% | -2.98% | +20.46% | +66.74% |
| document_1 | +6.74% | +3.63% | +21.47% | +99.54% |
| clouds_n0 | +27.91% | +25.12% | +19.78% | +105.32% |
| skyline_n0 | +43.65% | +40.71% | +24.71% | +113.80% |
| sunset_n0 | +2.42% | +1.30% | +16.37% | +51.04% |
| snow_n0 | +2.32% | +2.06% | +9.56% | +45.19% |
| path_n0 | +0.58% | +0.81% | +6.37% | +43.21% |
| building_n0 | +4.78% | +3.55% | +19.93% | +67.27% |
| church_n0 | +2.15% | +1.47% | +17.85% | +98.84% |
| child_n0 | +3.26% | +2.23% | +26.48% | +112.86% |
| woman_n0 | +10.25% | +8.18% | +17.88% | +78.99% |
| bird_n0 | +4.51% | +1.48% | +23.03% | +128.06% |
| waterfall_n0 | +3.43% | +2.87% | +14.51% | +47.96% |
| lake_n0 | +2.77% | +2.03% | +17.23% | +79.83% |

- JXL lossless e7 vs PKON2 per-image: min -11.40%, median +2.77%, max +43.65%; PKON2 smaller on 20/25 images, codec smaller on 5/25.

- JXL lossless e9 vs PKON2 per-image: min -17.65%, median +1.91%, max +40.71%; PKON2 smaller on 18/25 images, codec smaller on 7/25.

- WebP lossless m6 vs PKON2 per-image: min -48.51%, median +19.78%, max +30.36%; PKON2 smaller on 24/25 images, codec smaller on 1/25.

- PNG (png-rs best) vs PKON2 per-image: min +19.81%, median +78.99%, max +128.06%; PKON2 smaller on 25/25 images, codec smaller on 0/25.

### Wins by stored size

| codec | images with smallest file | which |
|---|---|---|
| PKON2 (.pkon2, codec 3) | 18 | outdoor_0, indoor_0, pet2_1, foliage_0, night_1, document_1, clouds_n0, skyline_n0, sunset_n0, snow_n0, path_n0, building_n0, church_n0, child_n0, woman_n0, bird_n0, waterfall_n0, lake_n0 |
| JXL lossless e7 | 1 | night_0 |
| JXL lossless e9 | 5 | portrait_0, pet_0, food_0, blurry_1, text_1 |
| WebP lossless m6 | 1 | screen_0 |
| PNG (png-rs best) | 0 | - |

Head-to-head among PKON2, JXL e7, JXL e9 only: PKON2 (.pkon2, codec 3) 18, JXL lossless e7 1, JXL lossless e9 6.

## 5. Results by image characteristic

Groups are assigned from the manifest (layout, source format, measured descriptors, category label). Sizes are summed stored bytes within the group; percentages are vs PKON2 in the same group. Group sizes are small; treat differences within a few percent with care.

**Smooth (lowest-MAE third by MED MAE)** (8: child_n0, skyline_n0, bird_n0, woman_n0, portrait_0, building_n0, pet2_1, sunset_n0)

| PKON2 (.pkon2, codec 3) | JXL lossless e7 | JXL lossless e9 | WebP lossless m6 | PNG (png-rs best) |
|---|---|---|---|---|
| 3,839,534 (+0.00%); wins 7 | 4,118,339 (+7.26%); wins 0 | 4,054,738 (+5.60%); wins 1 | 4,658,474 (+21.33%); wins 0 | 7,255,513 (+88.97%); wins 0 |

**Mid detail (middle third)** (8: blurry_1, night_1, indoor_0, pet_0, document_1, clouds_n0, screen_0, waterfall_n0)

| PKON2 (.pkon2, codec 3) | JXL lossless e7 | JXL lossless e9 | WebP lossless m6 | PNG (png-rs best) |
|---|---|---|---|---|
| 4,359,124 (+0.00%); wins 5 | 4,634,630 (+6.32%); wins 0 | 4,555,090 (+4.50%); wins 2 | 5,159,172 (+18.35%); wins 1 | 7,842,013 (+79.90%); wins 0 |

**Textured (highest-MAE third)** (9: church_n0, outdoor_0, lake_n0, snow_n0, text_1, food_0, foliage_0, night_0, path_n0)

| PKON2 (.pkon2, codec 3) | JXL lossless e7 | JXL lossless e9 | WebP lossless m6 | PNG (png-rs best) |
|---|---|---|---|---|
| 10,606,264 (+0.00%); wins 6 | 10,711,747 (+0.99%); wins 1 | 10,683,372 (+0.73%); wins 2 | 11,887,762 (+12.08%); wins 0 | 16,564,235 (+56.17%); wins 0 |

**Dark (mean G < 80)** (4: night_0, night_1, text_1, sunset_n0)

| PKON2 (.pkon2, codec 3) | JXL lossless e7 | JXL lossless e9 | WebP lossless m6 | PNG (png-rs best) |
|---|---|---|---|---|
| 4,445,648 (+0.00%); wins 2 | 4,516,042 (+1.58%); wins 1 | 4,494,886 (+1.11%); wins 1 | 4,917,612 (+10.62%); wins 0 | 6,400,056 (+43.96%); wins 0 |

**Bright (mean G >= 150)** (7: indoor_0, pet2_1, screen_0, document_1, skyline_n0, snow_n0, woman_n0)

| PKON2 (.pkon2, codec 3) | JXL lossless e7 | JXL lossless e9 | WebP lossless m6 | PNG (png-rs best) |
|---|---|---|---|---|
| 3,239,841 (+0.00%); wins 6 | 3,492,944 (+7.81%); wins 0 | 3,438,995 (+6.15%); wins 0 | 3,744,030 (+15.56%); wins 1 | 5,849,643 (+80.55%); wins 0 |

**Clipped share >= 1% of G samples at 0/255** (7: pet2_1, foliage_0, night_0, screen_0, document_1, woman_n0, bird_n0)

| PKON2 (.pkon2, codec 3) | JXL lossless e7 | JXL lossless e9 | WebP lossless m6 | PNG (png-rs best) |
|---|---|---|---|---|
| 4,769,471 (+0.00%); wins 5 | 4,822,065 (+1.10%); wins 1 | 4,782,855 (+0.28%); wins 0 | 5,249,530 (+10.07%); wins 1 | 7,666,458 (+60.74%); wins 0 |

**RGB inputs** (23: outdoor_0, indoor_0, portrait_0, pet_0, pet2_1, food_0, foliage_0, night_0, night_1, text_1, document_1, clouds_n0, skyline_n0, sunset_n0, snow_n0, path_n0, building_n0, church_n0, child_n0, woman_n0, bird_n0, waterfall_n0, lake_n0)

| PKON2 (.pkon2, codec 3) | JXL lossless e7 | JXL lossless e9 | WebP lossless m6 | PNG (png-rs best) |
|---|---|---|---|---|
| 17,998,169 (+0.00%); wins 18 | 18,675,748 (+3.76%); wins 1 | 18,524,834 (+2.93%); wins 4 | 20,806,380 (+15.60%); wins 0 | 30,151,317 (+67.52%); wins 0 |

**RGBA inputs (PNG origin)** (2: blurry_1, screen_0)

| PKON2 (.pkon2, codec 3) | JXL lossless e7 | JXL lossless e9 | WebP lossless m6 | PNG (png-rs best) |
|---|---|---|---|---|
| 806,753 (+0.00%); wins 0 | 788,968 (-2.20%); wins 0 | 768,366 (-4.76%); wins 1 | 899,028 (+11.44%); wins 1 | 1,510,444 (+87.23%); wins 0 |

**JPEG-origin** (23: outdoor_0, indoor_0, portrait_0, pet_0, pet2_1, food_0, foliage_0, night_0, night_1, text_1, document_1, clouds_n0, skyline_n0, sunset_n0, snow_n0, path_n0, building_n0, church_n0, child_n0, woman_n0, bird_n0, waterfall_n0, lake_n0)

| PKON2 (.pkon2, codec 3) | JXL lossless e7 | JXL lossless e9 | WebP lossless m6 | PNG (png-rs best) |
|---|---|---|---|---|
| 17,998,169 (+0.00%); wins 18 | 18,675,748 (+3.76%); wins 1 | 18,524,834 (+2.93%); wins 4 | 20,806,380 (+15.60%); wins 0 | 30,151,317 (+67.52%); wins 0 |

**PNG-origin** (2: blurry_1, screen_0)

| PKON2 (.pkon2, codec 3) | JXL lossless e7 | JXL lossless e9 | WebP lossless m6 | PNG (png-rs best) |
|---|---|---|---|---|
| 806,753 (+0.00%); wins 0 | 788,968 (-2.20%); wins 0 | 768,366 (-4.76%); wins 1 | 899,028 (+11.44%); wins 1 | 1,510,444 (+87.23%); wins 0 |

**Category: people** (3: portrait_0, child_n0, woman_n0)

| PKON2 (.pkon2, codec 3) | JXL lossless e7 | JXL lossless e9 | WebP lossless m6 | PNG (png-rs best) |
|---|---|---|---|---|
| 1,129,915 (+0.00%); wins 2 | 1,182,569 (+4.66%); wins 0 | 1,163,645 (+2.99%); wins 1 | 1,386,492 (+22.71%); wins 0 | 2,335,501 (+106.70%); wins 0 |

**Category: animals** (3: pet_0, pet2_1, bird_n0)

| PKON2 (.pkon2, codec 3) | JXL lossless e7 | JXL lossless e9 | WebP lossless m6 | PNG (png-rs best) |
|---|---|---|---|---|
| 1,534,175 (+0.00%); wins 2 | 1,550,119 (+1.04%); wins 0 | 1,522,259 (-0.78%); wins 1 | 1,921,562 (+25.25%); wins 0 | 3,015,862 (+96.58%); wins 0 |

**Category: sky / clouds / dusk** (3: clouds_n0, skyline_n0, sunset_n0)

| PKON2 (.pkon2, codec 3) | JXL lossless e7 | JXL lossless e9 | WebP lossless m6 | PNG (png-rs best) |
|---|---|---|---|---|
| 1,661,242 (+0.00%); wins 3 | 1,948,313 (+17.28%); wins 0 | 1,916,357 (+15.36%); wins 0 | 1,975,352 (+18.91%); wins 0 | 2,953,985 (+77.82%); wins 0 |

**Category: architecture** (3: indoor_0, building_n0, church_n0)

| PKON2 (.pkon2, codec 3) | JXL lossless e7 | JXL lossless e9 | WebP lossless m6 | PNG (png-rs best) |
|---|---|---|---|---|
| 1,742,367 (+0.00%); wins 3 | 1,812,640 (+4.03%); wins 0 | 1,792,746 (+2.89%); wins 0 | 2,110,594 (+21.13%); wins 0 | 3,340,267 (+91.71%); wins 0 |

**Category: water** (2: waterfall_n0, lake_n0)

| PKON2 (.pkon2, codec 3) | JXL lossless e7 | JXL lossless e9 | WebP lossless m6 | PNG (png-rs best) |
|---|---|---|---|---|
| 1,856,023 (+0.00%); wins 2 | 1,914,161 (+3.13%); wins 0 | 1,902,293 (+2.49%); wins 0 | 2,147,822 (+15.72%); wins 0 | 3,010,632 (+62.21%); wins 0 |

**Category: graphic / text / document** (4: blurry_1, screen_0, text_1, document_1)

| PKON2 (.pkon2, codec 3) | JXL lossless e7 | JXL lossless e9 | WebP lossless m6 | PNG (png-rs best) |
|---|---|---|---|---|
| 2,235,335 (+0.00%); wins 1 | 2,224,047 (-0.50%); wins 0 | 2,180,751 (-2.44%); wins 2 | 2,623,938 (+17.38%); wins 1 | 4,023,433 (+79.99%); wins 0 |

**Category: foliage / ground texture** (3: outdoor_0, foliage_0, path_n0)

| PKON2 (.pkon2, codec 3) | JXL lossless e7 | JXL lossless e9 | WebP lossless m6 | PNG (png-rs best) |
|---|---|---|---|---|
| 3,642,943 (+0.00%); wins 3 | 3,722,884 (+2.19%); wins 0 | 3,718,774 (+2.08%); wins 0 | 4,158,666 (+14.16%); wins 0 | 5,873,236 (+61.22%); wins 0 |

**Category: low light** (2: night_0, night_1)

| PKON2 (.pkon2, codec 3) | JXL lossless e7 | JXL lossless e9 | WebP lossless m6 | PNG (png-rs best) |
|---|---|---|---|---|
| 2,524,203 (+0.00%); wins 1 | 2,593,434 (+2.74%); wins 1 | 2,592,592 (+2.71%); wins 0 | 2,639,542 (+4.57%); wins 0 | 3,336,319 (+32.17%); wins 0 |

## 6. Timings

Per-image medians (ms) over the timed reps. `spread` = (max-min)/median over reps; it is the within-run noise indicator.

### Encode time per image (median ms [min-max])

| image | PKON2 (.pkon2, codec 3) | JXL lossless e7 | JXL lossless e9 | WebP lossless m6 | PNG (png-rs best) |
|---|---|---|---|---|---|
| outdoor_0 | 338 [331-345] | 1,156 [1,110-1,334] | 4,147 [4,124-4,417] | 8,975 [8,685-10,661] | 434 [426-451] |
| indoor_0 | 281 [278-293] | 806 [758-824] | 3,262 [3,258-3,284] | 6,756 [6,748-6,784] | 586 [525-653] |
| portrait_0 | 298 [298-299] | 781 [778-793] | 3,156 [3,149-3,214] | 11,976 [10,312-12,876] | 693 [690-728] |
| pet_0 | 392 [389-393] | 965 [950-1,097] | 3,400 [3,379-3,466] | 8,755 [8,530-9,281] | 552 [552-567] |
| pet2_1 | 317 [316-321] | 860 [854-864] | 3,451 [3,426-4,928] | 11,871 [10,792-12,528] | 1,123 [924-1,237] |
| food_0 | 1,313 [1,241-1,536] | 1,724 [1,699-2,405] | 7,845 [7,027-9,778] | 12,296 [9,131-24,954] | 211 [201-213] |
| foliage_0 | 553 [552-615] | 1,488 [1,445-1,497] | 5,162 [5,139-5,238] | 12,850 [8,588-17,603] | 302 [301-308] |
| night_0 | 1,354 [1,273-1,466] | 2,712 [2,189-3,453] | 12,538 [11,704-12,798] | 15,973 [15,955-16,862] | 232 [229-277] |
| night_1 | 773 [731-817] | 1,035 [1,034-1,061] | 4,761 [4,751-4,795] | 10,184 [10,112-14,086] | 998 [987-1,306] |
| blurry_1 | 949 [748-957] | 1,226 [1,216-1,597] | 5,847 [5,557-6,848] | 11,623 [10,869-12,771] | 1,319 [1,308-1,341] |
| screen_0 | 339 [321-368] | 363 [358-387] | 1,538 [1,503-1,547] | 3,483 [3,467-3,932] | 164 [151-197] |
| text_1 | 538 [537-593] | 1,213 [1,114-1,231] | 4,890 [4,771-4,999] | 8,350 [8,067-8,670] | 343 [329-389] |
| document_1 | 384 [356-402] | 709 [692-923] | 3,499 [3,416-3,731] | 7,067 [6,831-7,368] | 903 [892-984] |
| clouds_n0 | 340 [337-414] | 903 [781-919] | 3,873 [3,633-4,202] | 9,594 [9,476-11,435] | 937 [879-938] |
| skyline_n0 | 367 [316-368] | 854 [748-953] | 5,372 [4,754-8,018] | 7,859 [7,676-8,110] | 953 [945-965] |
| sunset_n0 | 608 [603-934] | 1,204 [1,198-1,345] | 4,542 [4,276-5,137] | 9,347 [8,455-12,343] | 1,150 [1,008-1,301] |
| snow_n0 | 1,158 [841-1,371] | 1,292 [1,278-1,296] | 5,576 [5,551-6,257] | 10,406 [9,761-11,043] | 616 [597-633] |
| path_n0 | 815 [784-875] | 1,572 [1,538-1,723] | 7,206 [6,946-8,407] | 9,604 [8,428-10,484] | 135 [130-147] |
| building_n0 | 469 [415-477] | 1,068 [934-1,082] | 4,540 [4,509-4,840] | 11,077 [11,060-13,569] | 1,115 [1,028-1,338] |
| church_n0 | 586 [575-888] | 1,096 [1,088-1,128] | 4,807 [4,656-4,870] | 12,110 [12,084-13,894] | 602 [542-754] |
| child_n0 | 331 [309-335] | 901 [872-1,044] | 3,827 [3,724-4,226] | 9,646 [8,611-11,689] | 1,454 [1,210-1,535] |
| woman_n0 | 466 [316-473] | 961 [824-1,735] | 5,311 [4,260-5,477] | 6,492 [6,492-9,229] | 421 [417-431] |
| bird_n0 | 350 [331-380] | 866 [722-872] | 3,885 [3,846-4,157] | 11,710 [10,968-12,015] | 945 [879-1,248] |
| waterfall_n0 | 612 [551-654] | 1,309 [1,233-1,333] | 5,607 [5,558-6,114] | 12,717 [10,384-12,870] | 598 [551-611] |
| lake_n0 | 545 [496-572] | 1,130 [1,097-1,146] | 4,811 [4,810-5,201] | 11,679 [10,327-13,030] | 498 [467-514] |

**Encode aggregate (sum of per-image medians over 25 images)**

| codec | total s | mean ms/image | MB/s (raw px bytes) | vs PKON2 | median per-image spread | max per-image spread | images where all reps of codec < all reps of PKON2 | images where all reps of codec > all reps of PKON2 |
|---|---|---|---|---|---|---|---|---|
| PKON2 (.pkon2, codec 3) | 14.5 | 579 | 5.58 | 1.00x | 14% | 54% | - | - |
| JXL lossless e7 | 28.2 | 1,128 | 2.86 | 1.95x | 12% | 95% | 0 | 23 |
| JXL lossless e9 | 122.9 | 4,914 | 0.66 | 8.49x | 9% | 61% | 0 | 25 |
| WebP lossless m6 | 252.4 | 10,096 | 0.32 | 17.44x | 20% | 129% | 0 | 25 |
| PNG (png-rs best) | 17.3 | 691 | 4.67 | 1.19x | 10% | 39% | 7 | 14 |

### Decode time per image (median ms [min-max])

| image | PKON2 (.pkon2, codec 3) | JXL lossless e7 | JXL lossless e9 | WebP lossless m6 | PNG (png-rs best) |
|---|---|---|---|---|---|
| outdoor_0 | 354 [343-365] | 466 [466-552] | 491 [483-503] | 145 [144-148] | 10 [9-10] |
| indoor_0 | 280 [279-289] | 387 [383-411] | 404 [403-408] | 115 [109-117] | 10 [9-14] |
| portrait_0 | 302 [300-307] | 398 [397-401] | 424 [417-431] | 137 [109-167] | 9 [9-10] |
| pet_0 | 392 [391-396] | 418 [412-420] | 461 [453-464] | 132 [127-133] | 10 [10-11] |
| pet2_1 | 329 [320-330] | 425 [415-431] | 457 [446-585] | 122 [118-137] | 18 [17-18] |
| food_0 | 1,385 [1,126-1,830] | 705 [627-947] | 604 [460-872] | 182 [153-1,348] | 15 [15-16] |
| foliage_0 | 632 [558-638] | 552 [539-556] | 593 [588-600] | 140 [125-149] | 25 [21-30] |
| night_0 | 1,401 [1,311-1,434] | 802 [672-1,519] | 641 [610-916] | 138 [129-140] | 37 [28-41] |
| night_1 | 717 [697-743] | 449 [439-449] | 366 [357-374] | 115 [111-157] | 13 [11-13] |
| blurry_1 | 859 [708-903] | 592 [573-631] | 474 [457-513] | 122 [107-123] | 13 [13-22] |
| screen_0 | 372 [348-384] | 181 [176-199] | 226 [218-239] | 76 [67-81] | 7 [7-8] |
| text_1 | 619 [546-787] | 501 [497-513] | 461 [438-482] | 113 [107-119] | 18 [16-18] |
| document_1 | 330 [323-398] | 426 [343-449] | 391 [354-392] | 107 [87-108] | 11 [10-11] |
| clouds_n0 | 403 [345-447] | 373 [362-377] | 329 [326-396] | 101 [97-107] | 11 [11-15] |
| skyline_n0 | 348 [313-351] | 398 [361-405] | 377 [350-681] | 94 [94-103] | 10 [9-12] |
| sunset_n0 | 663 [559-849] | 547 [529-557] | 448 [436-482] | 114 [109-139] | 34 [31-34] |
| snow_n0 | 1,077 [899-1,328] | 512 [507-516] | 477 [451-530] | 125 [122-132] | 15 [15-18] |
| path_n0 | 845 [807-981] | 599 [584-696] | 513 [501-526] | 135 [131-157] | 17 [16-31] |
| building_n0 | 497 [463-587] | 494 [489-496] | 432 [424-434] | 112 [102-143] | 15 [13-26] |
| church_n0 | 929 [579-991] | 500 [468-538] | 427 [421-544] | 100 [99-174] | 14 [13-15] |
| child_n0 | 326 [309-337] | 473 [454-519] | 518 [490-622] | 119 [113-134] | 15 [11-19] |
| woman_n0 | 406 [361-412] | 477 [464-804] | 421 [413-510] | 95 [94-96] | 9 [9-11] |
| bird_n0 | 367 [351-393] | 451 [408-477] | 387 [382-478] | 95 [95-95] | 27 [14-29] |
| waterfall_n0 | 569 [568-645] | 588 [542-594] | 471 [469-473] | 120 [118-132] | 14 [14-15] |
| lake_n0 | 600 [543-600] | 464 [463-480] | 403 [401-405] | 112 [111-120] | 16 [15-18] |

**Decode aggregate (sum of per-image medians over 25 images)**

| codec | total s | mean ms/image | MB/s (raw px bytes) | vs PKON2 | median per-image spread | max per-image spread | images where all reps of codec < all reps of PKON2 | images where all reps of codec > all reps of PKON2 |
|---|---|---|---|---|---|---|---|---|
| PKON2 (.pkon2, codec 3) | 15.0 | 600 | 5.38 | 1.00x | 12% | 51% | - | - |
| JXL lossless e7 | 12.2 | 487 | 6.63 | 0.81x | 9% | 106% | 11 | 9 |
| JXL lossless e9 | 11.2 | 448 | 7.21 | 0.75x | 10% | 88% | 13 | 7 |
| WebP lossless m6 | 3.0 | 119 | 27.22 | 0.20x | 13% | 657% | 24 | 0 |
| PNG (png-rs best) | 0.4 | 16 | 206.19 | 0.03x | 20% | 88% | 25 | 0 |

Reading the table: 'vs PKON2' is the ratio of summed medians (>1 = slower than PKON2). The last two columns count images where the two codecs' rep ranges do not overlap, i.e. where the ordering was consistent across every repetition. ffmpeg-driven codecs include the spawn/file overhead noted in section 2.

