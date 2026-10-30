# PKON2 milestone 5: activity x error-energy context

Baseline = Med/SubG/ActX from `results\20261027-041639Z-pkon2-m1`; baseline bytes identical to frozen M1 on every image: **true**. Coded bytes include the 11-byte header and all adaptive-model learning; the context swap adds no signalling. Timings: min of 3 runs, single-threaded. All configs exact on all images: **true**.

## Camera JPEG photos (primary, n=20)

| config | total bytes | bits/sample | vs baseline (aggregate) | mean per-image | smaller/larger | worst image | Hctx gap | coupled share | enc ms | dec ms |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| Act x prev (frozen baseline) | 16539865 | 2.1032 | +0.000% | +0.000% | 0/0 | +0.00% | +0.76% | 0.000 | 249 | 253 |
| Act x Err3 x prev (candidate, 96) | 16457692 | 2.0927 | -0.497% | -0.614% | 20/0 | -0.00% | +1.08% | 0.000 | 253 | 264 |
| Act x dummy3 x prev (dilution control, 96) | 16595621 | 2.1102 | +0.337% | +0.374% | 0/20 | +0.59% | +1.14% | 0.000 | 234 | 262 |

## Non-JPEG-origin (PNG) images, reported separately

| config | total bytes | bits/sample | vs baseline (aggregate) | mean per-image | smaller/larger | worst image | Hctx gap | coupled share | enc ms | dec ms |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| Act x prev (frozen baseline) | 1148082 | 0.7299 | +0.000% | +0.000% | 0/0 | +0.00% | +1.09% | 0.000 | 174 | 162 |
| Act x Err3 x prev (candidate, 96) | 1135110 | 0.7217 | -1.130% | -1.681% | 3/0 | -0.66% | +2.00% | 0.000 | 206 | 250 |
| Act x dummy3 x prev (dilution control, 96) | 1160458 | 0.7378 | +1.078% | +1.942% | 0/3 | +3.39% | +2.35% | 0.000 | 210 | 224 |

## Per image: bytes and change vs frozen baseline

| image | cat | src | baseline B | Act x Err3 x prev (candidate, 96) | Act x dummy3 x prev (dilution control, 96) |
|---|---|---|---:|---:|---:|
| outdoor_0 | outdoor | jpeg | 815272 | 813651 (-0.20%) | 817431 (+0.26%) |
| outdoor_1 | outdoor | jpeg | 547778 | 537120 (-1.95%) | 550913 (+0.57%) |
| indoor_0 | indoor | jpeg | 379149 | 375855 (-0.87%) | 381305 (+0.57%) |
| indoor_1 | indoor | jpeg | 646697 | 642519 (-0.65%) | 650149 (+0.53%) |
| portrait_0 | portrait | jpeg | 455539 | 447826 (-1.69%) | 457367 (+0.40%) |
| pet_0 | pet | jpeg | 793308 | 791476 (-0.23%) | 795415 (+0.27%) |
| pet_1 | pet | jpeg | 857767 | 856825 (-0.11%) | 859650 (+0.22%) |
| pet2_0 | pet | jpeg | 859192 | 854909 (-0.50%) | 861743 (+0.30%) |
| pet2_1 | pet | jpeg | 528629 | 523062 (-1.05%) | 531273 (+0.50%) |
| food_0 | food | jpeg | 1365488 | 1356300 (-0.67%) | 1369054 (+0.26%) |
| food_1 | food | jpeg | 795598 | 790448 (-0.65%) | 797782 (+0.27%) |
| foliage_0 | foliage | jpeg | 1280760 | 1274767 (-0.47%) | 1283569 (+0.22%) |
| foliage_1 | foliage | jpeg | 669030 | 663885 (-0.77%) | 671212 (+0.33%) |
| night_0 | night | jpeg | 1999783 | 1999760 (-0.00%) | 2005593 (+0.29%) |
| night_1 | night | jpeg | 670877 | 670669 (-0.03%) | 674337 (+0.52%) |
| blurry_0 | blurry | jpeg | 1133209 | 1130046 (-0.28%) | 1135296 (+0.18%) |
| blurry_1 | blurry | png | 785024 | 779832 (-0.66%) | 787208 (+0.28%) |
| text_0 | text | jpeg | 622632 | 614888 (-1.24%) | 626189 (+0.57%) |
| text_1 | text | jpeg | 1136704 | 1134705 (-0.18%) | 1140934 (+0.37%) |
| document_0 | document | jpeg | 548420 | 547456 (-0.18%) | 549814 (+0.25%) |
| document_1 | document | jpeg | 434033 | 431525 (-0.58%) | 436595 (+0.59%) |
| screen_0 | screen | png | 171930 | 166612 (-3.09%) | 175637 (+2.16%) |
| screen_1 | screen | png | 191128 | 188666 (-1.29%) | 197613 (+3.39%) |

## Predeclared verdict (JPEG n=20)

aggregate gain 0.497% (keep >= 1.0%, reject < 0.3%), smaller on 20/20 (keep >= 15), worst image -0.001% (keep <= +0.5%), enc time x1.01, dec time x1.04 (keep <= 1.25), exact: true. **Verdict: INVESTIGATE**

## Dilution control (JPEG n=20)

control (no-information 3-way split) vs baseline: +0.337% bytes (pure dilution cost). Candidate vs control: -0.831% bytes (information gain net of equal dilution), candidate smaller than control on 20/20 images.

