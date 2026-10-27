# Stage report: 5 training images -> validation (10 images)

Training images: ["text_0","clouds_n0","night_1","child_n0","forest_n0"]

Selected on training CV: **o1** (CV MSE order1 1.478e-6, order2 1.383e-6).

## Candidates on the validation set (real coded bytes; non-baseline includes 5 B signalling per file)

| candidate | theta | total bytes | vs baseline | enc ms (sum, best of 2) | dec ms (sum) | exact |
|---|---|---|---|---|---|---|
| baseline | limit=255 mixlr=6 mixinit=1250 errcut=8 actscale=4 | 8437115 | +0.000% | 2861 | 2870 | true |
| indep | limit=128 mixlr=10 mixinit=1250 errcut=14 actscale=6 | 8434669 | -0.029% | 4154 | 3584 | true |
| o1 | limit=128 mixlr=10 mixinit=1250 errcut=14 actscale=6 | 8434669 | -0.029% | 3322 | 3250 | true |
| o2 | limit=255 mixlr=10 mixinit=1250 errcut=14 actscale=6 | 8430902 | -0.074% | 4559 | 4652 | true |
| bestmeasured | limit=255 mixlr=10 mixinit=1700 errcut=14 actscale=6 | 8432135 | -0.059% | 4549 | 4342 | true |

## Per-image change for the selected candidate (o1)

| image | baseline B | o1 B | change |
|---|---|---|---|
| food_0 | 1283272 | 1284211 | +0.073% |
| food_1 | 744291 | 743219 | -0.144% |
| path_n0 | 1719713 | 1722089 | +0.138% |
| boat_n0 | 1017555 | 1017825 | +0.027% |
| flower_n0 | 771162 | 770042 | -0.145% |
| beach_n0 | 567976 | 567650 | -0.057% |
| woman_n0 | 341146 | 340314 | -0.244% |
| outdoor_0 | 781139 | 780452 | -0.088% |
| outdoor_1 | 498276 | 497695 | -0.117% |
| restaurant_n0 | 712585 | 711172 | -0.198% |

Per-image change for the other candidates (percent vs baseline):

| image | indep | o1 | o2 | bestmeasured |
|---|---|---|---|---|
| food_0 | +0.073 | +0.073 | -0.010 | +0.001 |
| food_1 | -0.144 | -0.144 | -0.189 | -0.176 |
| path_n0 | +0.138 | +0.138 | +0.053 | +0.062 |
| boat_n0 | +0.027 | +0.027 | -0.031 | -0.017 |
| flower_n0 | -0.145 | -0.145 | -0.127 | -0.110 |
| beach_n0 | -0.057 | -0.057 | -0.070 | -0.051 |
| woman_n0 | -0.244 | -0.244 | -0.186 | -0.149 |
| outdoor_0 | -0.088 | -0.088 | -0.197 | -0.184 |
| outdoor_1 | -0.117 | -0.117 | -0.086 | -0.059 |
| restaurant_n0 | -0.198 | -0.198 | -0.182 | -0.168 |

## Held-out prediction on the validation full grid (432 theta points x 10 images; evaluation only)

| model | RMSE of predicted ln cost-ratio | Spearman vs actual | regret of its argmin vs best global theta |
|---|---|---|---|
| constant 0 | 0.00268 | - | - |
| order 1 (additive) | 0.00110 | 0.896 | 0.047% |
| order 2 (pairwise) | 0.00106 | 0.914 | 0.002% |

Diagnostic headroom (peeks at validation, NOT a candidate): best single global theta on the grid = limit=255 mixlr=10 mixinit=800 errcut=14 actscale=6 at -0.076%; per-image oracle theta (needs 432 trial encodes per image) = -0.105% before signalling.

## Entanglement

Share of validation-surface variance that is NOT additive: actual 0.9%, model-o2-predicted 1.9% (train-fitted order-2 model predicts 1.9% of its own surface).

| pair | interaction RMS (actual, val) | interaction RMS (o2 model from train) |
|---|---|---|
| limit x mixlr | 0.00014 | 0.00025 |
| limit x mixinit | 0.00001 | 0.00002 |
| limit x errcut | 0.00001 | 0.00002 |
| limit x actscale | 0.00003 | 0.00006 |
| mixlr x mixinit | 0.00008 | 0.00007 |
| mixlr x errcut | 0.00001 | 0.00001 |
| mixlr x actscale | 0.00000 | 0.00002 |
| mixinit x errcut | 0.00000 | 0.00000 |
| mixinit x actscale | 0.00000 | 0.00000 |
| errcut x actscale | 0.00000 | 0.00001 |

Main-effect RMS (actual val): limit 0.00160, mixlr 0.00016, mixinit 0.00016, errcut 0.00006, actscale 0.00081. Spearman agreement of the pair-interaction ranking (model vs actual): 0.89 (n=10 pairs).

Sanity: max |CE/8 - stream bytes| over candidate encodes = 0.0011%.

## Criteria (from criteria.md)

- C1 byte-exact: PASS
- C2 >=0.30% smaller (o1: -0.029%): FAIL
- C3 regressions (0 images >+0.30%, worst +0.138%): PASS
- C4 time (enc +16.1%, dec +13.2%): FAIL
- C5 relationship claim (o2 beats indep by +0.045% and o1 by +0.045% of baseline; selected=o1): FAIL
- C6 held-out RMSE o2/o1 = 0.97: FAIL

**Stage verdict: FAIL**
