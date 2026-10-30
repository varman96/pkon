# PKON2 50-image compression test

Results dir: `results/20261027-155433Z-bench50`. Images: 50 (21 retained from the earlier 23, 29 new; the 2 earlier screenshots were dropped). Images with all five codecs valid: 50. Invalid/failed records: 0.

**What is measured:** lossless compression of the *decoded pixels* (1024x1024 centre crop, RGB8) of each photo, not of the original JPEG file bytes. Every size is the complete file/stream the codec emits (`.pkon2` = 24-byte header + payload). Ratio = raw pixel bytes / stored bytes, all against the same decoded buffer. Every output was decoded from its own bytes and compared byte-for-byte with that buffer; invalid outputs are excluded from all statistics below.

## Aggregate (sum of stored bytes over the images valid for all codecs)

Raw: 158334976 B over 50 images (52778325 px).

| codec | total bytes | bpp (24 raw) | aggregate ratio | vs pkon2-file | mean per-image ratio | median per-image ratio |
|---|---|---|---|---|---|---|
| pkon2-file | 38914941 | 5.899 | 4.069 | +0.00% | 4.837 | 4.265 |
| jxl-e7-t1 | 39519208 | 5.990 | 4.007 | +1.55% | 4.649 | 4.260 |
| jxl-e9-t1 | 39118943 | 5.930 | 4.048 | +0.52% | 4.718 | 4.322 |
| webp-m6-t1 | 44695086 | 6.775 | 3.543 | +14.85% | 4.125 | 3.468 |
| png-rs | 65492478 | 9.927 | 2.418 | +68.30% | 2.672 | 2.485 |

## Per-image compression-ratio distribution

| codec | min | p10 | p25 | median | p75 | p90 | max |
|---|---|---|---|---|---|---|---|
| pkon2-file | 1.62 | 2.60 | 3.17 | 4.26 | 6.21 | 7.66 | 9.42 |
| jxl-e7-t1 | 1.67 | 2.65 | 3.20 | 4.26 | 5.88 | 7.40 | 9.12 |
| jxl-e9-t1 | 1.66 | 2.66 | 3.23 | 4.32 | 5.93 | 7.61 | 9.39 |
| webp-m6-t1 | 1.63 | 2.34 | 2.75 | 3.47 | 5.22 | 6.49 | 8.00 |
| png-rs | 1.29 | 1.75 | 2.07 | 2.49 | 3.27 | 3.94 | 5.27 |

## Smallest output per image (ties impossible at byte level here)

| codec | images smallest | share |
|---|---|---|
| pkon2-file | 20 | 40% |
| jxl-e7-t1 | 2 | 4% |
| jxl-e9-t1 | 28 | 56% |
| webp-m6-t1 | 0 | 0% |
| png-rs | 0 | 0% |

Head-to-head among the three leaders only: pkon2-file 20, jxl-e7-t1 2, jxl-e9-t1 28.

- pkon2-file smaller than jxl-e7-t1 on 27/50 images.
- pkon2-file smaller than jxl-e9-t1 on 20/50 images.

## PKON2 vs jxl-e7-t1: per-image size difference (negative = PKON2 smaller)

Distribution of (pkon2/jxl-e7-t1 - 1): min -28.07%, median -0.35%, max +5.76%.

Largest PKON2 wins:

| image | category | set | pkon2 B | jxl-e7-t1 B | diff | MED MAE (G) |
|---|---|---|---|---|---|---|
| skyline_n0 | skyline | new | 333939 | 464276 | -28.07% | 0.97 |
| beach_n0 | beach | new | 567989 | 752301 | -24.50% | 2.74 |
| clouds_n0 | clouds | new | 457070 | 570294 | -19.85% | 2.25 |
| desert_n0 | desert | new | 589935 | 727834 | -18.95% | 2.01 |
| coast_n0 | coast | new | 449720 | 523100 | -14.03% | 3.12 |
| night_1 | night | retained | 636406 | 712137 | -10.63% | 2.14 |

Largest PKON2 losses:

| image | category | set | pkon2 B | jxl-e7-t1 B | diff | MED MAE (G) |
|---|---|---|---|---|---|---|
| pet_1 | pet | retained | 814528 | 770192 | +5.76% | 2.86 |
| forest_n0 | forest | new | 802555 | 764239 | +5.01% | 3.20 |
| pet_0 | pet | retained | 744800 | 709641 | +4.95% | 2.15 |
| text_1 | text | retained | 1057473 | 1008865 | +4.82% | 5.77 |
| river_n0 | river | new | 1054942 | 1009270 | +4.53% | 2.63 |
| night_0 | night | retained | 1943649 | 1881297 | +3.31% | 10.07 |

## PKON2 vs jxl-e9-t1: per-image size difference (negative = PKON2 smaller)

Distribution of (pkon2/jxl-e9-t1 - 1): min -26.57%, median +0.51%, max +6.63%.

Largest PKON2 wins:

| image | category | set | pkon2 B | jxl-e9-t1 B | diff | MED MAE (G) |
|---|---|---|---|---|---|---|
| skyline_n0 | skyline | new | 333939 | 454777 | -26.57% | 0.97 |
| beach_n0 | beach | new | 567989 | 730930 | -22.29% | 2.74 |
| desert_n0 | desert | new | 589935 | 721862 | -18.28% | 2.01 |
| clouds_n0 | clouds | new | 457070 | 557863 | -18.07% | 2.25 |
| coast_n0 | coast | new | 449720 | 509527 | -11.74% | 3.12 |
| night_1 | night | retained | 636406 | 701535 | -9.28% | 2.14 |

Largest PKON2 losses:

| image | category | set | pkon2 B | jxl-e9-t1 B | diff | MED MAE (G) |
|---|---|---|---|---|---|---|
| pet_0 | pet | retained | 744800 | 698480 | +6.63% | 2.15 |
| pet_1 | pet | retained | 814528 | 765105 | +6.46% | 2.86 |
| forest_n0 | forest | new | 802555 | 756965 | +6.02% | 3.20 |
| text_1 | text | retained | 1057473 | 998577 | +5.90% | 5.77 |
| river_n0 | river | new | 1054942 | 1002276 | +5.25% | 2.63 |
| blurry_1 | blurry | retained | 687310 | 655667 | +4.83% | 2.03 |

## Timing (sum over images of best-of-2, ms; ffmpeg codecs include process spawn + temp-file I/O, `net` subtracts the per-call floor measured on a 33x17 image)

| codec | enc total | enc net | enc MP/s net | dec total | dec net | dec MP/s net | enc net ms/img | dec net ms/img |
|---|---|---|---|---|---|---|---|---|
| pkon2-file | 17046 | 17033 | 3.08 | 17424 | 17410 | 3.01 | 341 | 348 |
| jxl-e7-t1 | 50131 | 48083 | 1.09 | 22355 | 20042 | 2.62 | 962 | 401 |
| jxl-e9-t1 | 216439 | 213726 | 0.25 | 23575 | 20808 | 2.52 | 4275 | 416 |
| webp-m6-t1 | 506675 | 503354 | 0.10 | 5986 | 3485 | 15.04 | 10067 | 70 |
| png-rs | 29914 | 29893 | 1.75 | 629 | 627 | 83.56 | 598 | 13 |

## Results by image characteristic (aggregate bytes in the group; groups with n<3 omitted)

`X vs pkon2` = codec bytes / pkon2 bytes - 1 over the group (positive: codec larger). Groups are descriptive buckets, not controlled experiments.

### Origin set

| group | n | pkon2 ratio | e7 vs pkon2 | e9 vs pkon2 | webp vs pkon2 | pkon2 beats e9 (n) |
|---|---|---|---|---|---|---|
| new | 29 | 4.03 | +3.1% | +2.1% | +14.5% | 14 |
| retained | 21 | 4.12 | -0.7% | -1.6% | +15.4% | 6 |

### Residual activity (G-plane MED mean |residual|; tercile cuts 2.14 / 3.20)

| group | n | pkon2 ratio | e7 vs pkon2 | e9 vs pkon2 | webp vs pkon2 | pkon2 beats e9 (n) |
|---|---|---|---|---|---|---|
| high | 17 | 2.86 | -0.7% | -1.2% | +12.2% | 4 |
| low | 16 | 5.43 | +3.9% | +2.4% | +17.3% | 8 |
| mid | 17 | 4.97 | +3.4% | +1.8% | +17.3% | 8 |

### Brightness (mean G level)

| group | n | pkon2 ratio | e7 vs pkon2 | e9 vs pkon2 | webp vs pkon2 | pkon2 beats e9 (n) |
|---|---|---|---|---|---|---|
| bright (>=150) | 11 | 5.11 | +3.4% | +2.2% | +14.9% | 8 |
| mid | 38 | 3.99 | +1.4% | +0.4% | +15.8% | 12 |

### Original resolution

| group | n | pkon2 ratio | e7 vs pkon2 | e9 vs pkon2 | webp vs pkon2 | pkon2 beats e9 (n) |
|---|---|---|---|---|---|---|
| 4-9 MP | 8 | 4.29 | -0.8% | -1.9% | +17.6% | 2 |
| <4 MP | 15 | 3.07 | +0.4% | -0.1% | +10.8% | 5 |
| >=9 MP | 27 | 4.90 | +3.3% | +1.9% | +17.6% | 13 |

### Original JPEG bits/pixel (source file bytes*8/pixels; detail + quality proxy)

| group | n | pkon2 ratio | e7 vs pkon2 | e9 vs pkon2 | webp vs pkon2 | pkon2 beats e9 (n) |
|---|---|---|---|---|---|---|
| 1.5-3 | 28 | 4.16 | +2.1% | +0.9% | +15.1% | 11 |
| <1.5 | 8 | 6.88 | +9.4% | +7.6% | +20.9% | 6 |
| >=3 | 14 | 3.20 | -1.3% | -1.9% | +12.9% | 3 |

### Orientation of original

| group | n | pkon2 ratio | e7 vs pkon2 | e9 vs pkon2 | webp vs pkon2 | pkon2 beats e9 (n) |
|---|---|---|---|---|---|---|
| landscape | 36 | 3.75 | +1.5% | +0.5% | +14.5% | 15 |
| portrait | 14 | 5.20 | +1.9% | +0.5% | +16.1% | 5 |

### Query category (1-2 images each, mostly omitted by n>=3 rule)

| group | n | pkon2 ratio | e7 vs pkon2 | e9 vs pkon2 | webp vs pkon2 | pkon2 beats e9 (n) |
|---|---|---|---|---|---|---|


### Rank correlation of PKON2-vs-JXL-e9 size difference with image descriptors (Spearman, n=50)

| descriptor | Spearman rho |
|---|---|
| MED mean |residual| | +0.18 |
| MED zero-residual fraction | -0.36 |
| mean G level | -0.28 |
| clipped fraction | +0.18 |
| original megapixels | -0.17 |
| source JPEG bits/px | +0.39 |
| pkon2 bpp (size proxy) | +0.43 |

With n=50, |rho| below ~0.28 is not distinguishable from zero at p=0.05.

## Per-image results

Columns: pkon2 B (ratio) | e7 B | e9 B | webp B | png B. Full detail incl. timing in `per_image.csv`.

| image | category | orig WxH | pkon2 | jxl-e7 | jxl-e9 | webp-m6 | png | smallest |
|---|---|---|---|---|---|---|---|---|
| outdoor_0 | outdoor | 4000x3000 | 781152 (4.03x) | 824676 | 821091 | 989404 | 1417416 | pkon2-file |
| outdoor_1 | outdoor | 4240x2832 | 498289 (6.31x) | 520781 | 510641 | 594604 | 932530 | pkon2-file |
| indoor_0 | indoor | 4592x3448 | 354417 (8.88x) | 365779 | 359179 | 447900 | 775015 | pkon2-file |
| indoor_1 | indoor | 3000x4000 | 607405 (5.18x) | 612689 | 605142 | 721894 | 1168705 | jxl-e9-t1 |
| portrait_0 | portrait | 2034x2717 | 405580 (7.76x) | 398485 | 390589 | 483336 | 880271 | jxl-e9-t1 |
| pet_0 | pet | 3607x2234 | 744800 (4.22x) | 709641 | 698480 | 911822 | 1279408 | jxl-e9-t1 |
| pet_1 | pet | 3850x3283 | 814528 (3.86x) | 770192 | 765105 | 955806 | 1332222 | jxl-e9-t1 |
| pet2_0 | pet2 | 2877x3456 | 798916 (3.94x) | 780721 | 773313 | 980572 | 1479400 | jxl-e9-t1 |
| pet2_1 | pet2 | 4000x2992 | 492984 (6.38x) | 495526 | 488803 | 603644 | 983672 | jxl-e9-t1 |
| food_0 | food | 1572x1078 | 1283285 (2.45x) | 1264540 | 1255067 | 1400906 | 1996090 | jxl-e9-t1 |
| food_1 | food | 2274x1666 | 744304 (4.23x) | 740780 | 734683 | 941478 | 1450115 | jxl-e9-t1 |
| foliage_0 | foliage | 1831x1831 | 1208203 (2.60x) | 1184936 | 1180499 | 1357336 | 2016330 | jxl-e9-t1 |
| foliage_1 | foliage | 4160x3120 | 630064 (4.99x) | 630027 | 622554 | 759766 | 1237201 | jxl-e9-t1 |
| night_0 | night | 2000x1329 | 1943649 (1.62x) | 1881297 | 1891057 | 1933568 | 2290163 | jxl-e7-t1 |
| night_1 | night | 4949x3140 | 636406 (4.94x) | 712137 | 701535 | 705974 | 1046156 | pkon2-file |
| blurry_0 | blurry | 1600x1200 | 1076424 (2.92x) | 1063542 | 1050998 | 1193420 | 1554190 | jxl-e9-t1 |
| blurry_1 | blurry | 2160x1620 | 687310 (6.10x) | 667721 | 655667 | 828564 | 1280963 | jxl-e9-t1 |
| text_0 | text | 3000x4000 | 577448 (5.45x) | 586370 | 578009 | 675746 | 1046410 | pkon2-file |
| text_1 | text | 2573x1959 | 1057473 (2.97x) | 1008865 | 998577 | 1239866 | 1716246 | jxl-e9-t1 |
| document_0 | document | 3120x4160 | 531813 (5.92x) | 533453 | 529789 | 584044 | 957594 | jxl-e9-t1 |
| document_1 | document | 3060x4080 | 411166 (7.65x) | 426214 | 413808 | 485044 | 796743 | pkon2-file |
| beach_n0 | beach | 4000x2250 | 567989 (5.54x) | 752301 | 730930 | 622632 | 959418 | pkon2-file |
| street_n0 | street | 3648x2736 | 924782 (3.40x) | 908644 | 902117 | 1079950 | 1522031 | jxl-e9-t1 |
| building_n0 | building | 2400x1957 | 698352 (4.50x) | 717273 | 708820 | 820972 | 1145058 | pkon2-file |
| forest_n0 | forest | 4048x3040 | 802555 (3.92x) | 764239 | 756965 | 968484 | 1495690 | jxl-e9-t1 |
| flower_n0 | flower | 2448x3264 | 771175 (4.08x) | 774581 | 767292 | 914978 | 1404606 | jxl-e9-t1 |
| sunset_n0 | sunset | 1980x1320 | 912421 (3.45x) | 913743 | 903717 | 1038204 | 1347491 | jxl-e9-t1 |
| snow_n0 | snow | 2048x1365 | 1239618 (2.54x) | 1252009 | 1248716 | 1340534 | 1776436 | pkon2-file |
| bird_n0 | bird | 3000x4000 | 346055 (9.09x) | 344952 | 334976 | 406096 | 752782 | jxl-e9-t1 |
| bridge_n0 | bridge | 4608x3468 | 503531 (6.25x) | 540970 | 530820 | 602420 | 967167 | pkon2-file |
| child_n0 | child | 3152x2337 | 419342 (7.50x) | 416191 | 412043 | 509778 | 857943 | jxl-e9-t1 |
| lake_n0 | lake | 4608x2566 | 854491 (3.68x) | 852564 | 846434 | 972500 | 1491870 | jxl-e9-t1 |
| macro_n0 | macro | 1365x2048 | 818302 (3.84x) | 824244 | 814185 | 916848 | 1210602 | jxl-e9-t1 |
| church_n0 | church | 3456x4608 | 731117 (4.30x) | 729588 | 724747 | 841722 | 1420194 | jxl-e9-t1 |
| boat_n0 | boat | 2048x1536 | 1017568 (3.09x) | 1032017 | 1028975 | 1166400 | 1580787 | pkon2-file |
| desert_n0 | desert | 1615x2048 | 589935 (5.33x) | 727834 | 721862 | 683242 | 1063023 | pkon2-file |
| stairs_n0 | stairs | 3648x2736 | 1109421 (2.84x) | 1101662 | 1083329 | 1253852 | 1728009 | jxl-e9-t1 |
| woman_n0 | woman | 1366x2048 | 341159 (9.22x) | 367893 | 361013 | 393378 | 597287 | pkon2-file |
| waterfall_n0 | waterfall | 2048x1365 | 1040807 (3.02x) | 1061597 | 1055859 | 1175322 | 1518762 | pkon2-file |
| peak_n0 | peak | 2592x1944 | 1219834 (2.58x) | 1224688 | 1216205 | 1372922 | 1985072 | jxl-e9-t1 |
| restaurant_n0 | restaurant | 4000x3000 | 712598 (4.41x) | 736167 | 724298 | 902414 | 1252548 | pkon2-file |
| graffiti_n0 | graffiti | 4000x2667 | 850605 (3.70x) | 829804 | 821745 | 1018978 | 1516070 | jxl-e9-t1 |
| station_n0 | station | 3264x2448 | 544009 (5.78x) | 566806 | 557920 | 636384 | 1028440 | pkon2-file |
| clouds_n0 | clouds | 4000x3000 | 457070 (6.88x) | 570294 | 557863 | 534082 | 915485 | pkon2-file |
| coast_n0 | coast | 4080x3060 | 449720 (6.99x) | 523100 | 509527 | 512628 | 797532 | pkon2-file |
| skyline_n0 | skyline | 3648x2736 | 333939 (9.42x) | 464276 | 454777 | 403066 | 691009 | pkon2-file |
| path_n0 | path | 2048x1366 | 1719726 (1.83x) | 1713272 | 1717184 | 1811926 | 2439490 | jxl-e7-t1 |
| river_n0 | river | 2587x3883 | 1054942 (2.98x) | 1009270 | 1002276 | 1165178 | 1600724 | jxl-e9-t1 |
| bicycle_n0 | bicycle | 2048x1366 | 1107010 (2.84x) | 1102964 | 1097034 | 1247610 | 1757880 | jxl-e9-t1 |
| lighthouse_n0 | lighthouse | 3000x4000 | 491252 (6.40x) | 517893 | 502728 | 587892 | 1032232 | pkon2-file |

## What the results tell us

### Observed (measured here)
- **Aggregate: PKON2 and JPEG XL e9 are within 0.5%.** PKON2 38,914,941 B vs e9 39,118,943 B (e9 +0.52%) and e7 39,519,208 B (e7 +1.55%); WebP m6 is +14.9% and PNG +68.3%. On the earlier 23-image set e9 was 1.9% *smaller* than PKON2; on this 50-image set PKON2 is ahead in aggregate, so the earlier gap does not generalise as a fixed number.
- **The two sets disagree.** On the 21 retained images e9 is 1.6% smaller than PKON2 (PKON2 smaller on 6/21). On the 29 new images PKON2 is 2.1% smaller than e9 (PKON2 smaller on 14/29). Per-image, e9 is smallest on 28/50, PKON2 on 20/50, e7 on 2/50.
- **The aggregate win is driven by a few images.** PKON2 beats e9 by 26.6% (skyline_n0), 22.3% (beach_n0), 18.3% (desert_n0), 18.1% (clouds_n0), 11.7% (coast_n0), 9.3% (night_1). The median per-image difference is +0.51% in favour of e9, and PKON2's worst loss is only 6.6% (pet_0). So PKON2 is the more variable codec: bigger wins, smaller losses, and typically slightly behind e9.
- **Losses concentrate on textured subjects:** pet_0/pet_1 (+6.6%/+6.5%), forest_n0 (+6.0%), text_1 (+5.9%), river_n0 (+5.3%), blurry_1 (+4.8%).
- **Descriptors (Spearman vs PKON2/e9 size difference, n=50):** source JPEG bits/px +0.39, MED zero-residual fraction -0.36, mean brightness -0.28. These are weak-to-moderate; MED mean |residual| (+0.18), clipping and resolution are not distinguishable from zero. Bucketed: images with high-quality source JPEGs (>=3 bpp) favour e9 by 1.9% over PKON2's size, while low-bpp (<1.5, n=8) sources favour PKON2 by 7.6%.
- **Speed:** PKON2 3.1 MP/s encode and 3.0 MP/s decode (~340 ms per 1 MP image, symmetric). That is 2.8x faster to encode than e7, 12.5x faster than e9, 29x faster than WebP m6; decode is ~15% faster than JXL (~2.5 MP/s), but PNG (84 MP/s) and WebP (15 MP/s) decode far faster.

### Hypotheses (not established by these measurements)
- The big PKON2 wins have low or very low G-plane MED residuals (skyline_n0 0.97). They may be smooth or heavily in-camera-processed images where PKON2's adaptive context modelling beats JXL's predictor/context set; this needs inspection of those images and the residual statistics per context, not just a correlation.
- The weak positive correlation with source JPEG bits/px suggests that on detailed, high-quality photos JXL's richer prediction (weighted/gradient predictors, MA-tree contexts) wins, and the pet/forest/text losses fit "fine texture, edges or fur", but sample sizes per type (1-2) cannot confirm this.
- JPEG-origin structure (8x8 DCT blocks, chroma subsampling in the source) could be exploited by one codec's predictors more than another's; nothing here tests it.

### Evidence-backed opportunities to improve PKON2 (in order of support)
1. **Close the textured-image gap.** Losses are consistently 5-7% on fur/foliage/text photos. Investigate on pet_0, pet_1, forest_n0, text_1, river_n0: per-context residual cost and whether adding a stronger predictor (e.g. a weighted blend of neighbours, or a second-pass bias/error-feedback stage) lowers their cost. Success criterion: reduce the 20 e9-win images' gap without losing the smooth-image wins.
2. **Understand and keep the smooth-image advantage.** Verify on skyline_n0/beach_n0/desert_n0/clouds_n0 what PKON2 is exploiting; this protects against regressions when changing the model.
3. **Low-light/noisy images:** night_0 (1.62x, the hardest image) is smallest under e7 (1,881,297 B), with e9 at 1,891,057 B and PKON2 at 1,943,649 B (PKON2 3.3% / 2.8% larger). night_1 is a large PKON2 win, so "night" is not a uniform category and this is not an actionable pattern on its own.
4. **Speed is not the issue:** at ~340 ms/MP PKON2 is already faster than JXL e7 to encode, so spending more time per pixel on a stronger model is affordable.

### Limitations that affect interpretation
- 50 images, 1-2 per query for most new categories; groups are small and the correlations are weak. Differences under ~1-2% in aggregate should not be treated as settled.
- All new images come from one source (Wikimedia Commons, CC0) and skew to enthusiast/older compact cameras and several modern phones; not a random camera roll. 14 of 50 are portrait-orientation originals.
- Pixels are the decoded JPEG output at 1024x1024 centre crops (EXIF orientation not applied, as in prior reports), not full images and not the original JPEG bytes. Crops of high-resolution images are smoother per pixel than the same scene downsized.
- JXL/WebP via ffmpeg 9.0.2 (libjxl/libwebp versions not exposed, so unverified); effort 10 and `cjxl` were not tested. JXL/WebP timings include process spawn and file I/O (floor subtracted). PNG is the `image` crate, not an optimising encoder.
- Retained images reflect two selection rounds; the 2 screenshots from the earlier 23 were removed per the brief, which changes aggregates versus the old report.
