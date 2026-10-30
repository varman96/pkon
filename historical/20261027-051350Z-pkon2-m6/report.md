# PKON2 milestone 6: binarized adaptive coder + logistic mixing

Baseline = Med/SubG/ActX from `results\20261027-041639Z-pkon2-m1`; baseline bytes identical to frozen M1 on every image: **true**. All configs use the identical decoded pixels, MED predictor, SubG transform and 11-byte header; only the entropy coder differs. Timings: single run each (single-threaded). All configs exact on all images: **true**.

## Camera JPEG photos (primary), n=20

| config | total bytes | bits/sample | vs baseline (aggregate) | mean per-image | smaller/larger | worst image | enc ms/img | dec ms/img | state KiB |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| Act x prev, 256-ary (frozen baseline) | 16539865 | 2.1032 | +0.000% | +0.000% | 0/0 | +0.00% | 277 | 316 | 48 |
| Act x Err3 x prev, 256-ary (M5, 96) | 16457692 | 2.0927 | -0.497% | -0.614% | 20/0 | -0.00% | 337 | 341 | 144 |
| X1 bin, ctx A | 16370845 | 2.0817 | -1.022% | -1.160% | 20/0 | -0.71% | 549 | 522 | 124 |
| X1s bin, ctx A, sign ctx | 15968743 | 2.0305 | -3.453% | -3.584% | 20/0 | -1.78% | 564 | 617 | 124 |
| X2 mix A+C, sign ctx | 15870643 | 2.0181 | -4.046% | -4.316% | 20/0 | -1.82% | 779 | 734 | 124 |
| X4 mix A+B+C+D, sign ctx (candidate) | 15687437 | 1.9948 | -5.154% | -5.473% | 20/0 | -2.67% | 1069 | 1080 | 124 |

## Non-JPEG-origin (PNG) images, reported separately, n=3

| config | total bytes | bits/sample | vs baseline (aggregate) | mean per-image | smaller/larger | worst image | enc ms/img | dec ms/img | state KiB |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| Act x prev, 256-ary (frozen baseline) | 1148082 | 0.7299 | +0.000% | +0.000% | 0/0 | +0.00% | 257 | 237 | 64 |
| Act x Err3 x prev, 256-ary (M5, 96) | 1135110 | 0.7217 | -1.130% | -1.681% | 3/0 | -0.66% | 236 | 297 | 192 |
| X1 bin, ctx A | 1135473 | 0.7219 | -1.098% | -1.311% | 2/1 | +3.10% | 429 | 451 | 165 |
| X1s bin, ctx A, sign ctx | 1053220 | 0.6696 | -8.263% | -6.171% | 2/1 | +1.20% | 455 | 436 | 165 |
| X2 mix A+C, sign ctx | 1040225 | 0.6614 | -9.395% | -7.832% | 3/0 | -0.01% | 684 | 657 | 165 |
| X4 mix A+B+C+D, sign ctx (candidate) | 1018250 | 0.6474 | -11.309% | -10.377% | 3/0 | -2.22% | 920 | 958 | 165 |

## Per image: bytes and change vs frozen baseline

| image | cat | src | baseline B | Act x Err3 x prev, 256-ary (M5, 96) | X1 bin, ctx A | X1s bin, ctx A, sign ctx | X2 mix A+C, sign ctx | X4 mix A+B+C+D, sign ctx (candidate) |
|---|---|---|---:|---:|---:|---:|---:|---:|
| outdoor_0 | outdoor | jpeg | 815272 | 813651 (-0.20%) | 807363 (-0.97%) | 797676 (-2.16%) | 795193 (-2.46%) | 786614 (-3.52%) |
| outdoor_1 | outdoor | jpeg | 547778 | 537120 (-1.95%) | 539287 (-1.55%) | 521715 (-4.76%) | 509892 (-6.92%) | 500260 (-8.67%) |
| indoor_0 | indoor | jpeg | 379149 | 375855 (-0.87%) | 371923 (-1.91%) | 365565 (-3.58%) | 361546 (-4.64%) | 357324 (-5.76%) |
| indoor_1 | indoor | jpeg | 646697 | 642519 (-0.65%) | 637907 (-1.36%) | 627350 (-2.99%) | 622059 (-3.81%) | 612023 (-5.36%) |
| portrait_0 | portrait | jpeg | 455539 | 447826 (-1.69%) | 448269 (-1.60%) | 424268 (-6.86%) | 416131 (-8.65%) | 408582 (-10.31%) |
| pet_0 | pet | jpeg | 793308 | 791476 (-0.23%) | 786140 (-0.90%) | 761061 (-4.06%) | 758396 (-4.40%) | 750519 (-5.39%) |
| pet_1 | pet | jpeg | 857767 | 856825 (-0.11%) | 850665 (-0.83%) | 826922 (-3.60%) | 825035 (-3.82%) | 820140 (-4.39%) |
| pet2_0 | pet | jpeg | 859192 | 854909 (-0.50%) | 850775 (-0.98%) | 825349 (-3.94%) | 820351 (-4.52%) | 809270 (-5.81%) |
| pet2_1 | pet | jpeg | 528629 | 523062 (-1.05%) | 520683 (-1.50%) | 508349 (-3.84%) | 501831 (-5.07%) | 496339 (-6.11%) |
| food_0 | food | jpeg | 1365488 | 1356300 (-0.67%) | 1355846 (-0.71%) | 1314591 (-3.73%) | 1305062 (-4.43%) | 1287571 (-5.71%) |
| food_1 | food | jpeg | 795598 | 790448 (-0.65%) | 787314 (-1.04%) | 767103 (-3.58%) | 761389 (-4.30%) | 751684 (-5.52%) |
| foliage_0 | foliage | jpeg | 1280760 | 1274767 (-0.47%) | 1271289 (-0.74%) | 1233753 (-3.67%) | 1227434 (-4.16%) | 1214450 (-5.18%) |
| foliage_1 | foliage | jpeg | 669030 | 663885 (-0.77%) | 661198 (-1.17%) | 648110 (-3.13%) | 642117 (-4.02%) | 635099 (-5.07%) |
| night_0 | night | jpeg | 1999783 | 1999760 (-0.00%) | 1984592 (-0.76%) | 1964115 (-1.78%) | 1963427 (-1.82%) | 1946369 (-2.67%) |
| night_1 | night | jpeg | 670877 | 670669 (-0.03%) | 663656 (-1.08%) | 646548 (-3.63%) | 644766 (-3.89%) | 639193 (-4.72%) |
| blurry_0 | blurry | jpeg | 1133209 | 1130046 (-0.28%) | 1125057 (-0.72%) | 1093989 (-3.46%) | 1090294 (-3.79%) | 1081484 (-4.56%) |
| blurry_1 | blurry | png | 785024 | 779832 (-0.66%) | 776777 (-1.05%) | 704021 (-10.32%) | 698644 (-11.00%) | 687861 (-12.38%) |
| text_0 | text | jpeg | 622632 | 614888 (-1.24%) | 613698 (-1.43%) | 602753 (-3.19%) | 594048 (-4.59%) | 581186 (-6.66%) |
| text_1 | text | jpeg | 1136704 | 1134705 (-0.18%) | 1128204 (-0.75%) | 1079528 (-5.03%) | 1076563 (-5.29%) | 1063018 (-6.48%) |
| document_0 | document | jpeg | 548420 | 547456 (-0.18%) | 540971 (-1.36%) | 538370 (-1.83%) | 536973 (-2.09%) | 532641 (-2.88%) |
| document_1 | document | jpeg | 434033 | 431525 (-0.58%) | 426008 (-1.85%) | 421628 (-2.86%) | 418136 (-3.66%) | 413671 (-4.69%) |
| screen_0 | screen | png | 171930 | 166612 (-3.09%) | 161646 (-5.98%) | 155775 (-9.40%) | 150463 (-12.49%) | 143504 (-16.53%) |
| screen_1 | screen | png | 191128 | 188666 (-1.29%) | 197050 (+3.10%) | 193424 (+1.20%) | 191118 (-0.01%) | 186885 (-2.22%) |

## Predeclared verdict (JPEG, candidate = last config)

aggregate gain 5.154% (keep >= 1.0%, reject < 0.3%), smaller on 20/20 (keep >= 75%), worst image -2.671% (keep <= +0.5%), exact: true; enc time x3.86, dec time x3.42 vs baseline (reported, not gated). **Verdict: KEEP**

