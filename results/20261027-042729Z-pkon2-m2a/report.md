# PKON2 milestone 2A: adaptive SubG/Coupled selection

Baseline = Med/SubG/ActX from `results\20261027-041639Z-pkon2-m1`; baseline bytes identical to frozen M1 on every image: **true**. Coded bytes include the 11-byte header and all adaptive-model learning; the adaptive rule has zero signalling bits. Timings: min of 2 runs, single-threaded. All configs exact on all images: **true**.

## Camera JPEG photos (primary, n=20)

| config | total bytes | bits/sample | vs baseline (aggregate) | mean per-image | smaller/larger | worst image | Hctx gap | coupled share | enc ms | dec ms |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| baseline SubG (frozen) | 16539865 | 2.1032 | +0.000% | +0.000% | 0/0 | +0.00% | +0.76% | 0.000 | 211 | 232 |
| fixed Coupled | 18170793 | 2.3105 | +9.861% | +11.812% | 1/19 | +21.04% | +0.55% | 0.000 | 243 | 265 |
| Adaptive win4 (primary) | 16682513 | 2.1213 | +0.862% | +1.080% | 2/18 | +2.38% | +0.76% | 0.053 | 317 | 327 |
| Adaptive win2 (sens.) | 16685048 | 2.1216 | +0.878% | +1.070% | 2/18 | +2.28% | +0.76% | 0.047 | 304 | 322 |
| Adaptive win6 (sens.) | 16695909 | 2.1230 | +0.943% | +1.187% | 2/18 | +2.42% | +0.76% | 0.057 | 360 | 383 |

## Non-JPEG-origin (PNG) images, reported separately

| config | total bytes | bits/sample | vs baseline (aggregate) | mean per-image | smaller/larger | worst image | Hctx gap | coupled share | enc ms | dec ms |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| baseline SubG (frozen) | 1148082 | 0.7299 | +0.000% | +0.000% | 0/0 | +0.00% | +1.09% | 0.000 | 193 | 205 |
| fixed Coupled | 1188721 | 0.7558 | +3.540% | -0.348% | 2/1 | +7.18% | +1.11% | 0.000 | 177 | 199 |
| Adaptive win4 (primary) | 1143546 | 0.7270 | -0.395% | -1.691% | 2/1 | +0.81% | +1.14% | 0.020 | 219 | 216 |
| Adaptive win2 (sens.) | 1145602 | 0.7284 | -0.216% | -1.141% | 2/1 | +0.64% | +1.13% | 0.014 | 200 | 187 |
| Adaptive win6 (sens.) | 1144591 | 0.7277 | -0.304% | -1.652% | 2/1 | +0.95% | +1.14% | 0.022 | 252 | 207 |

## Per image: bytes and change vs frozen baseline

| image | cat | src | baseline B | fixed Coupled | Adaptive win4 (primary) | Adaptive win2 (sens.) | Adaptive win6 (sens.) |
|---|---|---|---:|---:|---:|---:|---:|
| outdoor_0 | outdoor | jpeg | 815272 | 919147 (+12.74%) | 829362 (+1.73%) | 827854 (+1.54%) | 830952 (+1.92%) |
| outdoor_1 | outdoor | jpeg | 547778 | 634311 (+15.80%) | 554025 (+1.14%) | 553767 (+1.09%) | 554841 (+1.29%) |
| indoor_0 | indoor | jpeg | 379149 | 436653 (+15.17%) | 383969 (+1.27%) | 383987 (+1.28%) | 384613 (+1.44%) |
| indoor_1 | indoor | jpeg | 646697 | 745679 (+15.31%) | 655520 (+1.36%) | 654659 (+1.23%) | 656010 (+1.44%) |
| portrait_0 | portrait | jpeg | 455539 | 551392 (+21.04%) | 461748 (+1.36%) | 462357 (+1.50%) | 462710 (+1.57%) |
| pet_0 | pet | jpeg | 793308 | 879790 (+10.90%) | 804932 (+1.47%) | 802448 (+1.15%) | 807424 (+1.78%) |
| pet_1 | pet | jpeg | 857767 | 953400 (+11.15%) | 873174 (+1.80%) | 869868 (+1.41%) | 875780 (+2.10%) |
| pet2_0 | pet | jpeg | 859192 | 984461 (+14.58%) | 879648 (+2.38%) | 878812 (+2.28%) | 879956 (+2.42%) |
| pet2_1 | pet | jpeg | 528629 | 624664 (+18.17%) | 536415 (+1.47%) | 536806 (+1.55%) | 537149 (+1.61%) |
| food_0 | food | jpeg | 1365488 | 1446257 (+5.92%) | 1370302 (+0.35%) | 1371547 (+0.44%) | 1370286 (+0.35%) |
| food_1 | food | jpeg | 795598 | 919098 (+15.52%) | 811265 (+1.97%) | 810888 (+1.92%) | 811645 (+2.02%) |
| foliage_0 | foliage | jpeg | 1280760 | 1327836 (+3.68%) | 1262524 (-1.42%) | 1272447 (-0.65%) | 1262349 (-1.44%) |
| foliage_1 | foliage | jpeg | 669030 | 785141 (+17.36%) | 680941 (+1.78%) | 681229 (+1.82%) | 681861 (+1.92%) |
| night_0 | night | jpeg | 1999783 | 1986408 (-0.67%) | 1979953 (-0.99%) | 1984782 (-0.75%) | 1979621 (-1.01%) |
| night_1 | night | jpeg | 670877 | 754252 (+12.43%) | 681077 (+1.52%) | 679346 (+1.26%) | 682823 (+1.78%) |
| blurry_0 | blurry | jpeg | 1133209 | 1227435 (+8.31%) | 1155044 (+1.93%) | 1152516 (+1.70%) | 1153118 (+1.76%) |
| blurry_1 | blurry | png | 785024 | 841350 (+7.18%) | 791382 (+0.81%) | 790083 (+0.64%) | 792458 (+0.95%) |
| text_0 | text | jpeg | 622632 | 709766 (+13.99%) | 629054 (+1.03%) | 627685 (+0.81%) | 629148 (+1.05%) |
| text_1 | text | jpeg | 1136704 | 1214329 (+6.83%) | 1149102 (+1.09%) | 1147363 (+0.94%) | 1150283 (+1.19%) |
| document_0 | document | jpeg | 548420 | 596624 (+8.79%) | 550377 (+0.36%) | 550842 (+0.44%) | 550967 (+0.46%) |
| document_1 | document | jpeg | 434033 | 474150 (+9.24%) | 434081 (+0.01%) | 435845 (+0.42%) | 434373 (+0.08%) |
| screen_0 | screen | png | 171930 | 171715 (-0.13%) | 168820 (-1.81%) | 169834 (-1.22%) | 168744 (-1.85%) |
| screen_1 | screen | png | 191128 | 175656 (-8.10%) | 183344 (-4.07%) | 185685 (-2.85%) | 183389 (-4.05%) |

## Predeclared verdict (primary = Adaptive win4, JPEG n=20)

aggregate gain -0.862% (keep >= 1.0%, reject < 0.3%), smaller on 2/20 (keep >= 15), worst image +2.381% (keep <= +0.3%), exact: true. **Verdict: REJECT**

