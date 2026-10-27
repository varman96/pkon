# PKON2 milestone 3: GAP vs MED

Baseline = Med/SubG/ActX from `results\20261027-041639Z-pkon2-m1`; baseline bytes identical to frozen M1 on every image: **true**. Coded bytes include the 11-byte header and all adaptive-model learning; GAP adds no signalling. Timings: min of 3 runs, single-threaded. All configs exact on all images: **true**.

## Camera JPEG photos (primary, n=20)

| config | total bytes | bits/sample | vs baseline (aggregate) | mean per-image | smaller/larger | worst image | Hctx gap | coupled share | enc ms | dec ms |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| MED baseline (frozen) | 16539865 | 2.1032 | +0.000% | +0.000% | 0/0 | +0.00% | +0.76% | 0.000 | 147 | 163 |
| GAP | 17838630 | 2.2683 | +7.852% | +9.941% | 1/19 | +22.17% | +0.61% | 0.000 | 163 | 183 |

## Non-JPEG-origin (PNG) images, reported separately

| config | total bytes | bits/sample | vs baseline (aggregate) | mean per-image | smaller/larger | worst image | Hctx gap | coupled share | enc ms | dec ms |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| MED baseline (frozen) | 1148082 | 0.7299 | +0.000% | +0.000% | 0/0 | +0.00% | +1.09% | 0.000 | 118 | 135 |
| GAP | 1192201 | 0.7580 | +3.843% | +9.280% | 1/2 | +14.77% | +1.21% | 0.000 | 143 | 160 |

## Per image: bytes and change vs frozen baseline

| image | cat | src | baseline B | GAP |
|---|---|---|---:|---:|
| outdoor_0 | outdoor | jpeg | 815272 | 900642 (+10.47%) |
| outdoor_1 | outdoor | jpeg | 547778 | 646574 (+18.04%) |
| indoor_0 | indoor | jpeg | 379149 | 439351 (+15.88%) |
| indoor_1 | indoor | jpeg | 646697 | 756023 (+16.91%) |
| portrait_0 | portrait | jpeg | 455539 | 556518 (+22.17%) |
| pet_0 | pet | jpeg | 793308 | 865261 (+9.07%) |
| pet_1 | pet | jpeg | 857767 | 896351 (+4.50%) |
| pet2_0 | pet | jpeg | 859192 | 986846 (+14.86%) |
| pet2_1 | pet | jpeg | 528629 | 609608 (+15.32%) |
| food_0 | food | jpeg | 1365488 | 1402781 (+2.73%) |
| food_1 | food | jpeg | 795598 | 901383 (+13.30%) |
| foliage_0 | foliage | jpeg | 1280760 | 1331034 (+3.93%) |
| foliage_1 | foliage | jpeg | 669030 | 770746 (+15.20%) |
| night_0 | night | jpeg | 1999783 | 1968975 (-1.54%) |
| night_1 | night | jpeg | 670877 | 702069 (+4.65%) |
| blurry_0 | blurry | jpeg | 1133209 | 1211157 (+6.88%) |
| blurry_1 | blurry | png | 785024 | 776758 (-1.05%) |
| text_0 | text | jpeg | 622632 | 700028 (+12.43%) |
| text_1 | text | jpeg | 1136704 | 1148544 (+1.04%) |
| document_0 | document | jpeg | 548420 | 576404 (+5.10%) |
| document_1 | document | jpeg | 434033 | 468335 (+7.90%) |
| screen_0 | screen | png | 171930 | 197330 (+14.77%) |
| screen_1 | screen | png | 191128 | 218113 (+14.12%) |

## Predeclared verdict (JPEG n=20)

aggregate gain -7.852% (keep >= 0.5%, reject < 0.1%), smaller on 1/20 (keep >= 14), worst image +22.167% (keep <= +1.0%), enc time x1.11, dec time x1.12 (keep <= 1.5), exact: true. **Verdict: REJECT**

