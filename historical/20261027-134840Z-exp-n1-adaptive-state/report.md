# Experiment N1: adaptive-state cost of X4

Cold control bytes identical to frozen M6 X4 (`"results\\20261027-051350Z-pkon2-m6"`) on every test image: **true**. Priming set (disjoint from test, 4 images, 1024x1024 centre crops, coded back to back): ["portrait_0", "food_1", "text_1", "blurry_0"]; priming took 1.5s; total adaptive state 124 KiB. Timings: min of 2 runs. All variants decoded bit-exact on all images: **true**.

## Per image (bytes; change vs cold)

| image | bps (cold) | V0 cold (X4 as shipped) | V1 primed, all state, counts kept | V2 primed, all state, counts capped at 30 | V3 primed, mixer weights only | V4 primed, counters+tail only (cap 30), mixer cold | V5 SAME-IMAGE prime, all state (contaminated upper bound) |
|---|---:|---:|---:|---:|---:|---:|---:|
| indoor_0 | 0.909 | 357324 | 357616 (+0.082%) | 357344 (+0.006%) | 357388 (+0.018%) | 357291 (-0.009%) | 356976 (-0.097%) |
| document_1 | 1.052 | 413671 | 414438 (+0.185%) | 413884 (+0.051%) | 413758 (+0.021%) | 413811 (+0.034%) | 413429 (-0.059%) |
| outdoor_1 | 1.272 | 500260 | 500849 (+0.118%) | 500341 (+0.016%) | 500236 (-0.005%) | 500388 (+0.026%) | 499890 (-0.074%) |
| pet_1 | 2.086 | 820140 | 820278 (+0.017%) | 820024 (-0.014%) | 820126 (-0.002%) | 820045 (-0.012%) | 819727 (-0.050%) |
| foliage_0 | 3.089 | 1214450 | 1214663 (+0.018%) | 1214355 (-0.008%) | 1214419 (-0.003%) | 1214402 (-0.004%) | 1213918 (-0.044%) |
| night_0 | 4.950 | 1946369 | 1946792 (+0.022%) | 1946423 (+0.003%) | 1946425 (+0.003%) | 1946390 (+0.001%) | 1945557 (-0.042%) |

## Aggregate (6 images)

| variant | total bytes | vs cold | bytes saved | enc ms/img | dec ms/img |
|---|---:|---:|---:|---:|---:|
| V0 cold (X4 as shipped) | 5252214 | +0.000% | 0 | 483 | 608 |
| V1 primed, all state, counts kept | 5254636 | +0.046% | -2422 | 601 | 593 |
| V2 primed, all state, counts capped at 30 | 5252371 | +0.003% | -157 | 734 | 610 |
| V3 primed, mixer weights only | 5252352 | +0.003% | -138 | 554 | 565 |
| V4 primed, counters+tail only (cap 30), mixer cold | 5252327 | +0.002% | -113 | 566 | 603 |
| V5 SAME-IMAGE prime, all state (contaminated upper bound) | 5249497 | -0.052% | 2717 | 523 | 630 |
