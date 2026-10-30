# External codec benchmark: PKON2 X4 vs JPEG XL / WebP / PNG (lossless)

Inputs: 23 images, each the exact decoded RGB8 pixel buffer centre-cropped to at most 1024x1024 (identical crops/pipeline to every PKON2 report). The 20 JPEG-origin inputs are rgb8 and the 3 PNG-origin inputs are rgba8 (alpha is constant 255 in all three, but the 4-channel layout is preserved for every codec); interleaved, no row padding. bpp = bits per pixel of the canonical buffer (RGB = 24 raw, RGBA = 32 raw); ratio = canonical raw bytes / stored bytes. Every stream was decoded from its bytes and compared byte-for-byte with the canonical buffer. **All exact: true**. Timing: best of 2 (encode and decode timed independently per round trip), single process, nothing else running.

## Aggregate size (sum of actual stored bytes; bpp = stored bits / pixel; raw = 24 bpp RGB, 32 bpp RGBA)

### 20 JPEG-origin (20 images, raw 62914560 B, 20971520 px)

| codec | total bytes | payload bytes | bpp | ratio | % of raw | vs PKON2 X4 | exact |
|---|---:|---:|---:|---:|---:|---:|---|
| pkon2-x4 | 15598046 | 15597826 | 5.950 | 4.033 | 24.79% | +0.00% | true |
| jxl-e7-t1 | 15510651 | 15510651 | 5.917 | 4.056 | 24.65% | -0.56% | true |
| jxl-e9-t1 | 15368919 | 15368919 | 5.863 | 4.094 | 24.43% | -1.47% | true |
| webp-m6-t1 | 17966130 | 17965719 | 6.854 | 3.502 | 28.56% | +15.18% | true |
| png-rs | 26355877 | 26354737 | 10.054 | 2.387 | 41.89% | +68.97% | true |

### 3 PNG-origin (3 images, raw 12582912 B, 3145728 px)

| codec | total bytes | payload bytes | bpp | ratio | % of raw | vs PKON2 X4 | exact |
|---|---:|---:|---:|---:|---:|---:|---|
| pkon2-x4 | 1012208 | 1012175 | 2.574 | 12.431 | 8.04% | +0.00% | true |
| jxl-e7-t1 | 966310 | 966310 | 2.457 | 13.022 | 7.68% | -4.53% | true |
| jxl-e9-t1 | 929272 | 929272 | 2.363 | 13.541 | 7.39% | -8.19% | true |
| webp-m6-t1 | 949222 | 949162 | 2.414 | 13.256 | 7.54% | -6.22% | true |
| png-rs | 1663791 | 1663620 | 4.231 | 7.563 | 13.22% | +64.37% | true |

### All 23 combined (23 images, raw 75497472 B, 24117248 px)

| codec | total bytes | payload bytes | bpp | ratio | % of raw | vs PKON2 X4 | exact |
|---|---:|---:|---:|---:|---:|---:|---|
| pkon2-x4 | 16610254 | 16610001 | 5.510 | 4.545 | 22.00% | +0.00% | true |
| jxl-e7-t1 | 16476961 | 16476961 | 5.466 | 4.582 | 21.82% | -0.80% | true |
| jxl-e9-t1 | 16298191 | 16298191 | 5.406 | 4.632 | 21.59% | -1.88% | true |
| webp-m6-t1 | 18915352 | 18914881 | 6.274 | 3.991 | 25.05% | +13.88% | true |
| png-rs | 28019668 | 28018357 | 9.294 | 2.694 | 37.11% | +68.69% | true |

"vs PKON2 X4" = (codec total bytes / PKON2 X4 total bytes - 1); positive means the codec is larger than PKON2. Computed from summed bytes, not averaged per-image percentages.

## Timing (sum over images of best-of-N, ms; MP/s = raw megapixels per second)

ffmpeg-driven codecs (JXL, WebP) are measured through a process spawn plus temp-file I/O; PNG and PKON2 are in-process. `net` subtracts the per-call floor measured on a 33x17 image through the same path (see calibration). The `-t1` rows use `-threads 1` and are the like-for-like comparison with single-threaded PKON2; `-mt` rows (library default threads) are timing-only.

| codec | set | enc total ms | enc net ms | enc MP/s (net) | dec total ms | dec net ms | dec MP/s (net) |
|---|---|---:|---:|---:|---:|---:|---:|
| pkon2-x4 | 20 JPEG-origin | 5707 | 5707 | 3.67 | 6014 | 6014 | 3.49 |
| pkon2-x4 | 3 PNG-origin | 1028 | 1028 | 3.06 | 1027 | 1027 | 3.06 |
| pkon2-x4 | All 23 combined | 6735 | 6735 | 3.58 | 7041 | 7041 | 3.43 |
| jxl-e7-t1 | 20 JPEG-origin | 20392 | 19575 | 1.07 | 8838 | 7989 | 2.63 |
| jxl-e7-t1 | 3 PNG-origin | 2340 | 2217 | 1.42 | 1170 | 1042 | 3.02 |
| jxl-e7-t1 | All 23 combined | 22732 | 21793 | 1.11 | 10008 | 9031 | 2.67 |
| jxl-e9-t1 | 20 JPEG-origin | 75564 | 74674 | 0.28 | 9427 | 8598 | 2.44 |
| jxl-e9-t1 | 3 PNG-origin | 9822 | 9688 | 0.32 | 1169 | 1045 | 3.01 |
| jxl-e9-t1 | All 23 combined | 85386 | 84362 | 0.29 | 10596 | 9642 | 2.50 |
| webp-m6-t1 | 20 JPEG-origin | 171049 | 169925 | 0.12 | 2436 | 1589 | 13.20 |
| webp-m6-t1 | 3 PNG-origin | 14998 | 14830 | 0.21 | 295 | 168 | 18.77 |
| webp-m6-t1 | All 23 combined | 186047 | 184754 | 0.13 | 2731 | 1757 | 13.73 |
| jxl-e7-mt | 20 JPEG-origin | 7546 | 6694 | 3.13 | 8051 | 7156 | 2.93 |
| jxl-e7-mt | 3 PNG-origin | 1062 | 935 | 3.37 | 1094 | 960 | 3.28 |
| jxl-e7-mt | All 23 combined | 8608 | 7628 | 3.16 | 9145 | 8116 | 2.97 |
| jxl-e9-mt | 20 JPEG-origin | 32383 | 31475 | 0.67 | 9097 | 8241 | 2.54 |
| jxl-e9-mt | 3 PNG-origin | 3928 | 3792 | 0.83 | 1195 | 1067 | 2.95 |
| jxl-e9-mt | All 23 combined | 36311 | 35267 | 0.68 | 10293 | 9308 | 2.59 |
| png-rs | 20 JPEG-origin | 9791 | 9788 | 2.14 | 224 | 224 | 93.69 |
| png-rs | 3 PNG-origin | 1685 | 1685 | 1.87 | 33 | 33 | 94.31 |
| png-rs | All 23 combined | 11476 | 11473 | 2.10 | 258 | 257 | 93.77 |

Calibration (floor per call, best of 5 on 33x17): jxl-e7-t1 enc 40.8 / dec 42.5 ms; jxl-e9-t1 enc 44.5 / dec 41.5 ms; webp-m6-t1 enc 56.2 / dec 42.4 ms; jxl-e7-mt enc 42.6 / dec 44.7 ms; jxl-e9-mt enc 45.4 / dec 42.8 ms; png-rs enc 0.2 / dec 0.0 ms

## Per-image results

Columns: total bytes | bpp | ratio | enc ms | dec ms (exact in every row unless marked FAIL).

| image | src | pkon2-x4 | jxl-e7-t1 | jxl-e9-t1 | webp-m6-t1 | png-rs |
|---|---|---|---|---|---|---|
| outdoor_0 | jpeg | 781139 B / 5.96 / 4.03x / 255 / 281 | 824676 B / 6.29 / 3.81x / 1055 / 444 | 821091 B / 6.26 / 3.83x / 3930 / 471 | 989404 B / 7.55 / 3.18x / 8177 / 127 | 1417416 B / 10.81 / 2.22x / 452 / 10 |
| outdoor_1 | jpeg | 498276 B / 3.80 / 6.31x / 222 / 232 | 520781 B / 3.97 / 6.04x / 846 / 377 | 510641 B / 3.90 / 6.16x / 3418 / 407 | 594604 B / 4.54 / 5.29x / 6828 / 109 | 932530 B / 7.11 / 3.37x / 296 / 10 |
| indoor_0 | jpeg | 354404 B / 2.70 / 8.88x / 201 / 200 | 365779 B / 2.79 / 8.60x / 745 / 397 | 359179 B / 2.74 / 8.76x / 3354 / 410 | 447900 B / 3.42 / 7.02x / 6838 / 112 | 775015 B / 5.91 / 4.06x / 502 / 9 |
| indoor_1 | jpeg | 607392 B / 4.63 / 5.18x / 255 / 259 | 612689 B / 4.67 / 5.13x / 971 / 445 | 605142 B / 4.62 / 5.20x / 3751 / 467 | 721894 B / 5.51 / 4.36x / 13269 / 114 | 1168705 B / 8.92 / 2.69x / 714 / 12 |
| portrait_0 | jpeg | 405567 B / 3.09 / 7.76x / 256 / 275 | 398485 B / 3.04 / 7.89x / 791 / 407 | 390589 B / 2.98 / 8.05x / 3185 / 428 | 483336 B / 3.69 / 6.51x / 10332 / 108 | 880271 B / 6.72 / 3.57x / 702 / 9 |
| pet_0 | jpeg | 744787 B / 5.68 / 4.22x / 271 / 285 | 709641 B / 5.41 / 4.43x / 951 / 411 | 698480 B / 5.33 / 4.50x / 3411 / 449 | 911822 B / 6.96 / 3.45x / 8551 / 127 | 1279408 B / 9.76 / 2.46x / 599 / 11 |
| pet_1 | jpeg | 814515 B / 6.21 / 3.86x / 291 / 305 | 770192 B / 5.88 / 4.08x / 978 / 401 | 765105 B / 5.84 / 4.11x / 3374 / 448 | 955806 B / 7.29 / 3.29x / 10350 / 124 | 1332222 B / 10.16 / 2.36x / 495 / 9 |
| pet2_0 | jpeg | 798903 B / 6.10 / 3.94x / 283 / 309 | 780721 B / 5.96 / 4.03x / 1068 / 448 | 773313 B / 5.90 / 4.07x / 3595 / 479 | 980572 B / 7.48 / 3.21x / 9996 / 133 | 1479400 B / 11.29 / 2.13x / 466 / 12 |
| pet2_1 | jpeg | 492971 B / 3.76 / 6.38x / 223 / 235 | 495526 B / 3.78 / 6.35x / 890 / 424 | 488803 B / 3.73 / 6.44x / 3419 / 445 | 603644 B / 4.61 / 5.21x / 10530 / 117 | 983672 B / 7.50 / 3.20x / 635 / 10 |
| food_0 | jpeg | 1283272 B / 9.79 / 2.45x / 383 / 418 | 1264540 B / 9.65 / 2.49x / 1320 / 506 | 1255067 B / 9.58 / 2.51x / 4522 / 531 | 1400906 B / 10.69 / 2.25x / 7728 / 137 | 1996090 B / 15.23 / 1.58x / 208 / 15 |
| food_1 | jpeg | 744291 B / 5.68 / 4.23x / 298 / 303 | 740780 B / 5.65 / 4.25x / 1022 / 447 | 734683 B / 5.61 / 4.28x / 3765 / 479 | 941478 B / 7.18 / 3.34x / 7908 / 135 | 1450115 B / 11.06 / 2.17x / 524 / 12 |
| foliage_0 | jpeg | 1208190 B / 9.22 / 2.60x / 369 / 397 | 1184936 B / 9.04 / 2.65x / 1376 / 509 | 1180499 B / 9.01 / 2.66x / 4749 / 554 | 1357336 B / 10.36 / 2.32x / 7804 / 135 | 2016330 B / 15.38 / 1.56x / 218 / 16 |
| foliage_1 | jpeg | 630051 B / 4.81 / 4.99x / 265 / 266 | 630027 B / 4.81 / 4.99x / 923 / 433 | 622554 B / 4.75 / 5.05x / 3546 / 460 | 759766 B / 5.80 / 4.14x / 10672 / 116 | 1237201 B / 9.44 / 2.54x / 589 / 11 |
| night_0 | jpeg | 1943636 B / 14.83 / 1.62x / 523 / 556 | 1881297 B / 14.35 / 1.67x / 1683 / 620 | 1891057 B / 14.43 / 1.66x / 6484 / 647 | 1933568 B / 14.75 / 1.63x / 10168 / 140 | 2290163 B / 17.47 / 1.37x / 137 / 17 |
| night_1 | jpeg | 636393 B / 4.86 / 4.94x / 263 / 267 | 712137 B / 5.43 / 4.42x / 907 / 390 | 701535 B / 5.35 / 4.48x / 3449 / 418 | 705974 B / 5.39 / 4.46x / 6250 / 111 | 1046156 B / 7.98 / 3.01x / 623 / 8 |
| blurry_0 | jpeg | 1076411 B / 8.21 / 2.92x / 332 / 356 | 1063542 B / 8.11 / 2.96x / 1131 / 470 | 1050998 B / 8.02 / 2.99x / 3644 / 511 | 1193420 B / 9.11 / 2.64x / 7664 / 132 | 1554190 B / 11.86 / 2.02x / 311 / 12 |
| text_0 | jpeg | 577435 B / 4.41 / 5.45x / 249 / 257 | 586370 B / 4.47 / 5.36x / 911 / 438 | 578009 B / 4.41 / 5.44x / 3653 / 460 | 675746 B / 5.16 / 4.66x / 7885 / 109 | 1046410 B / 7.98 / 3.01x / 731 / 10 |
| text_1 | jpeg | 1057460 B / 8.07 / 2.97x / 341 / 369 | 1008865 B / 7.70 / 3.12x / 1175 / 488 | 998577 B / 7.62 / 3.15x / 4126 / 553 | 1239866 B / 9.46 / 2.54x / 6922 / 131 | 1716246 B / 13.09 / 1.83x / 299 / 14 |
| document_0 | jpeg | 531800 B / 4.06 / 5.92x / 233 / 238 | 533453 B / 4.07 / 5.90x / 858 / 391 | 529789 B / 4.04 / 5.94x / 3115 / 404 | 584044 B / 4.46 / 5.39x / 7248 / 109 | 957594 B / 7.31 / 3.29x / 602 / 9 |
| document_1 | jpeg | 411153 B / 3.14 / 7.65x / 193 / 203 | 426214 B / 3.25 / 7.38x / 789 / 394 | 413808 B / 3.16 / 7.60x / 3076 / 405 | 485044 B / 3.70 / 6.49x / 5928 / 111 | 796743 B / 6.08 / 3.95x / 689 / 9 |
| blurry_1 | png | 687297 B / 5.24 / 6.10x / 302 / 317 | 667721 B / 5.09 / 6.28x / 1060 / 490 | 655667 B / 5.00 / 6.40x / 3880 / 568 | 828564 B / 6.32 / 5.06x / 8629 / 140 | 1280963 B / 9.77 / 3.27x / 1267 / 20 |
| screen_0 | png | 141787 B / 1.08 / 29.58x / 256 / 230 | 121247 B / 0.93 / 34.59x / 374 / 176 | 112699 B / 0.86 / 37.22x / 1462 / 242 | 70464 B / 0.54 / 59.52x / 3466 / 71 | 229481 B / 1.75 / 18.28x / 221 / 10 |
| screen_1 | png | 183124 B / 1.40 / 22.90x / 470 / 480 | 177342 B / 1.35 / 23.65x / 907 / 503 | 160906 B / 1.23 / 26.07x / 4480 / 358 | 50194 B / 0.38 / 83.56x / 2904 / 83 | 153347 B / 1.17 / 27.35x / 197 / 4 |

## PKON2 X4 cross-checks

Replica pipeline bytes == `measure_x(x4_cfg())` bytes on every image (asserted). Compared with the prior `x4-promotion-verify` report's production bytes:

| image | bench bytes | measure_x bytes | prior report bytes | match | measure_x enc/dec ms (excl. plane split/colour) |
|---|---:|---:|---:|---|---|
| outdoor_0 | 781139 | 781139 | - | n/a | 297/310 |
| outdoor_1 | 498276 | 498276 | - | n/a | 233/244 |
| indoor_0 | 354404 | 354404 | - | n/a | 206/217 |
| indoor_1 | 607392 | 607392 | - | n/a | 262/283 |
| portrait_0 | 405567 | 405567 | - | n/a | 242/233 |
| pet_0 | 744787 | 744787 | - | n/a | 285/302 |
| pet_1 | 814515 | 814515 | - | n/a | 307/331 |
| pet2_0 | 798903 | 798903 | - | n/a | 310/345 |
| pet2_1 | 492971 | 492971 | - | n/a | 245/247 |
| food_0 | 1283272 | 1283272 | - | n/a | 400/423 |
| food_1 | 744291 | 744291 | - | n/a | 284/303 |
| foliage_0 | 1208190 | 1208190 | - | n/a | 381/406 |
| foliage_1 | 630051 | 630051 | - | n/a | 259/271 |
| night_0 | 1943636 | 1943636 | - | n/a | 551/571 |
| night_1 | 636393 | 636393 | - | n/a | 259/275 |
| blurry_0 | 1076411 | 1076411 | - | n/a | 349/378 |
| text_0 | 577435 | 577435 | - | n/a | 253/264 |
| text_1 | 1057460 | 1057460 | - | n/a | 362/375 |
| document_0 | 531800 | 531800 | - | n/a | 237/244 |
| document_1 | 411153 | 411153 | - | n/a | 208/216 |
| blurry_1 | 687297 | 687297 | - | n/a | 316/331 |
| screen_0 | 141787 | 141787 | - | n/a | 228/240 |
| screen_1 | 183124 | 183124 | - | n/a | 483/481 |

## Environment

- ffmpeg: ffmpeg version 9.0.2-full_build-www.gyan.dev Copyright (c) 2000-2026 the FFmpeg developers
- see `environment.md` for codec library versions and exact settings.
