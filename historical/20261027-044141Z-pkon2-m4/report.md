# PKON2 milestone 4: error-energy context vs activity context

Baseline = Med/SubG/ActX (and Act-alone) from `results\20261027-041639Z-pkon2-m1`; baseline bytes identical to frozen M1 on every image: **true**. Coded bytes include the 11-byte header and all adaptive-model learning; the context swap adds no signalling. Timings: min of 3 runs, single-threaded. All configs exact on all images: **true**.

## Camera JPEG photos (primary, n=20)

| config | total bytes | bits/sample | vs baseline (aggregate) | mean per-image | smaller/larger | worst image | Hctx gap | coupled share | enc ms | dec ms |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| Act x prev (frozen baseline) | 16539865 | 2.1032 | +0.000% | +0.000% | 0/0 | +0.00% | +0.76% | 0.000 | 224 | 253 |
| Err x prev (primary) | 16686987 | 2.1219 | +0.889% | +1.104% | 3/17 | +4.11% | +0.80% | 0.000 | 224 | 231 |
| Act alone (frozen M1) | 17022609 | 2.1645 | +2.919% | +3.315% | 0/20 | +5.28% | +0.56% | 0.000 | 206 | 246 |
| Err alone (ablation) | 17229202 | 2.1908 | +4.168% | +4.890% | 0/20 | +9.91% | +0.54% | 0.000 | 204 | 222 |

## Non-JPEG-origin (PNG) images, reported separately

| config | total bytes | bits/sample | vs baseline (aggregate) | mean per-image | smaller/larger | worst image | Hctx gap | coupled share | enc ms | dec ms |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| Act x prev (frozen baseline) | 1148082 | 0.7299 | +0.000% | +0.000% | 0/0 | +0.00% | +1.09% | 0.000 | 228 | 236 |
| Err x prev (primary) | 1148024 | 0.7299 | -0.005% | +0.625% | 1/2 | +1.24% | +1.24% | 0.000 | 212 | 248 |
| Act alone (frozen M1) | 1187728 | 0.7551 | +3.453% | +6.492% | 0/3 | +11.02% | +0.53% | 0.000 | 175 | 212 |
| Err alone (ablation) | 1189891 | 0.7565 | +3.642% | +7.536% | 0/3 | +13.32% | +0.60% | 0.000 | 244 | 259 |

## Per image: bytes and change vs frozen baseline

| image | cat | src | baseline B | Err x prev (primary) | Act alone (frozen M1) | Err alone (ablation) |
|---|---|---|---:|---:|---:|---:|
| outdoor_0 | outdoor | jpeg | 815272 | 828835 (+1.66%) | 848409 (+4.06%) | 865330 (+6.14%) |
| outdoor_1 | outdoor | jpeg | 547778 | 543774 (-0.73%) | 564745 (+3.10%) | 560394 (+2.30%) |
| indoor_0 | indoor | jpeg | 379149 | 394736 (+4.11%) | 395202 (+4.23%) | 416715 (+9.91%) |
| indoor_1 | indoor | jpeg | 646697 | 652382 (+0.88%) | 669198 (+3.48%) | 679059 (+5.00%) |
| portrait_0 | portrait | jpeg | 455539 | 455578 (+0.01%) | 474437 (+4.15%) | 477811 (+4.89%) |
| pet_0 | pet | jpeg | 793308 | 806216 (+1.63%) | 835191 (+5.28%) | 850795 (+7.25%) |
| pet_1 | pet | jpeg | 857767 | 874260 (+1.92%) | 900757 (+5.01%) | 920381 (+7.30%) |
| pet2_0 | pet | jpeg | 859192 | 866439 (+0.84%) | 898112 (+4.53%) | 909326 (+5.84%) |
| pet2_1 | pet | jpeg | 528629 | 539863 (+2.13%) | 552653 (+4.54%) | 568726 (+7.59%) |
| food_0 | food | jpeg | 1365488 | 1355364 (-0.74%) | 1376766 (+0.83%) | 1366574 (+0.08%) |
| food_1 | food | jpeg | 795598 | 806650 (+1.39%) | 824208 (+3.60%) | 841695 (+5.79%) |
| foliage_0 | foliage | jpeg | 1280760 | 1285551 (+0.37%) | 1302808 (+1.72%) | 1309661 (+2.26%) |
| foliage_1 | foliage | jpeg | 669030 | 680165 (+1.66%) | 695516 (+3.96%) | 711969 (+6.42%) |
| night_0 | night | jpeg | 1999783 | 2013287 (+0.68%) | 2007705 (+0.40%) | 2022334 (+1.13%) |
| night_1 | night | jpeg | 670877 | 683283 (+1.85%) | 703415 (+4.85%) | 718988 (+7.17%) |
| blurry_0 | blurry | jpeg | 1133209 | 1135797 (+0.23%) | 1155949 (+2.01%) | 1159360 (+2.31%) |
| blurry_1 | blurry | png | 785024 | 780518 (-0.57%) | 791019 (+0.76%) | 786550 (+0.19%) |
| text_0 | text | jpeg | 622632 | 621116 (-0.24%) | 644872 (+3.57%) | 645325 (+3.64%) |
| text_1 | text | jpeg | 1136704 | 1142960 (+0.55%) | 1171489 (+3.06%) | 1179716 (+3.78%) |
| document_0 | document | jpeg | 548420 | 555241 (+1.24%) | 556470 (+1.47%) | 565384 (+3.09%) |
| document_1 | document | jpeg | 434033 | 445490 (+2.64%) | 444707 (+2.46%) | 459659 (+5.90%) |
| screen_0 | screen | png | 171930 | 174017 (+1.21%) | 190875 (+11.02%) | 194833 (+13.32%) |
| screen_1 | screen | png | 191128 | 193489 (+1.24%) | 205834 (+7.69%) | 208508 (+9.09%) |

## Predeclared verdict (JPEG n=20)

aggregate gain -0.889% (keep >= 1.0%, reject < 0.3%), smaller on 3/20 (keep >= 15), worst image +4.111% (keep <= +0.5%), enc time x1.00, dec time x0.91 (keep <= 1.25), exact: true. **Verdict: REJECT**

## Ablation (JPEG n=20): Err alone vs Act alone (no previous-plane term)

aggregate +1.214% bytes, Err smaller on 2/20 images.

