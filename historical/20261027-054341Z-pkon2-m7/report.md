# PKON2 milestone 7: adaptive bias correction in front of X4

Control = X4 (frozen M6, `results\20261027-051350Z-pkon2-m6`); control bytes identical to frozen M6 on every image: **true**. All configs use the identical decoded pixels, SubG transform, X4 coder and 11-byte header; only the prediction differs. Timings: min of 3 runs (single-threaded). All configs exact on all images: **true**.

## Camera JPEG photos (primary), n=20

| config | total bytes | bits/sample | vs X4 (aggregate) | mean per-image | smaller/larger | worst image | enc ms/img | dec ms/img | state KiB |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| X4 control (no bias correction) | 15687437 | 1.9948 | +0.000% | +0.000% | 0/0 | +0.00% | 642 | 639 | 124 |
| P1 bias: W,N signed errors | 16101874 | 2.0475 | +2.642% | +2.680% | 0/20 | +6.97% | 715 | 741 | 3205 |
| P2 bias: + previous plane (R<-G, B<-R) | 16062871 | 2.0425 | +2.393% | +2.501% | 0/20 | +5.58% | 813 | 814 | 3205 |
| P3 bias: + B<-R and G (candidate) | 16043387 | 2.0400 | +2.269% | +2.483% | 1/19 | +5.57% | 813 | 814 | 3205 |
| D1 diag: P1 with dead-zone (trunc) | 15815840 | 2.0111 | +0.819% | +0.824% | 0/20 | +2.00% | 783 | 761 | 3205 |
| D3 diag: P3 with dead-zone (trunc) | 15823357 | 2.0120 | +0.866% | +1.045% | 1/19 | +2.08% | 773 | 745 | 3205 |

## Non-JPEG-origin (PNG) images, reported separately, n=3

| config | total bytes | bits/sample | vs X4 (aggregate) | mean per-image | smaller/larger | worst image | enc ms/img | dec ms/img | state KiB |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| X4 control (no bias correction) | 1018250 | 0.6474 | +0.000% | +0.000% | 0/0 | +0.00% | 568 | 584 | 165 |
| P1 bias: W,N signed errors | 1184557 | 0.7531 | +16.333% | +30.834% | 0/3 | +52.75% | 896 | 895 | 4270 |
| P2 bias: + previous plane (R<-G, B<-R) | 1146309 | 0.7288 | +12.576% | +22.554% | 0/3 | +37.96% | 773 | 796 | 4270 |
| P3 bias: + B<-R and G (candidate) | 1141864 | 0.7260 | +12.140% | +21.520% | 0/3 | +36.00% | 867 | 773 | 4270 |
| D1 diag: P1 with dead-zone (trunc) | 1138178 | 0.7236 | +11.778% | +25.053% | 1/2 | +43.34% | 702 | 750 | 4270 |
| D3 diag: P3 with dead-zone (trunc) | 1100704 | 0.6998 | +8.098% | +17.350% | 1/2 | +31.23% | 908 | 958 | 4270 |

## Per image: bytes and change vs X4 control

| image | cat | src | X4 B | P1 bias: W,N signed errors | P2 bias: + previous plane (R<-G, B<-R) | P3 bias: + B<-R and G (candidate) | D1 diag: P1 with dead-zone (trunc) | D3 diag: P3 with dead-zone (trunc) |
|---|---|---|---:|---:|---:|---:|---:|---:|
| outdoor_0 | outdoor | jpeg | 786614 | 802392 (+2.01%) | 801757 (+1.93%) | 802300 (+1.99%) | 788037 (+0.18%) | 792854 (+0.79%) |
| outdoor_1 | outdoor | jpeg | 500260 | 511712 (+2.29%) | 509634 (+1.87%) | 510116 (+1.97%) | 502856 (+0.52%) | 502301 (+0.41%) |
| indoor_0 | indoor | jpeg | 357324 | 366118 (+2.46%) | 367648 (+2.89%) | 368048 (+3.00%) | 363099 (+1.62%) | 364439 (+1.99%) |
| indoor_1 | indoor | jpeg | 612023 | 621979 (+1.63%) | 624652 (+2.06%) | 624896 (+2.10%) | 616608 (+0.75%) | 618046 (+0.98%) |
| portrait_0 | portrait | jpeg | 408582 | 411938 (+0.82%) | 412936 (+1.07%) | 413169 (+1.12%) | 409020 (+0.11%) | 410323 (+0.43%) |
| pet_0 | pet | jpeg | 750519 | 802840 (+6.97%) | 792407 (+5.58%) | 792352 (+5.57%) | 756192 (+0.76%) | 763427 (+1.72%) |
| pet_1 | pet | jpeg | 820140 | 872681 (+6.41%) | 860002 (+4.86%) | 860791 (+4.96%) | 825116 (+0.61%) | 835199 (+1.84%) |
| pet2_0 | pet | jpeg | 809270 | 823754 (+1.79%) | 826919 (+2.18%) | 827685 (+2.28%) | 815380 (+0.76%) | 820237 (+1.36%) |
| pet2_1 | pet | jpeg | 496339 | 503431 (+1.43%) | 505585 (+1.86%) | 505979 (+1.94%) | 498637 (+0.46%) | 500882 (+0.92%) |
| food_0 | food | jpeg | 1287571 | 1301585 (+1.09%) | 1303874 (+1.27%) | 1304876 (+1.34%) | 1293765 (+0.48%) | 1294963 (+0.57%) |
| food_1 | food | jpeg | 751684 | 761473 (+1.30%) | 762645 (+1.46%) | 763569 (+1.58%) | 755574 (+0.52%) | 756985 (+0.71%) |
| foliage_0 | foliage | jpeg | 1214450 | 1245157 (+2.53%) | 1242676 (+2.32%) | 1245397 (+2.55%) | 1229994 (+1.28%) | 1230343 (+1.31%) |
| foliage_1 | foliage | jpeg | 635099 | 643110 (+1.26%) | 643882 (+1.38%) | 644523 (+1.48%) | 638079 (+0.47%) | 640155 (+0.80%) |
| night_0 | night | jpeg | 1946369 | 1968437 (+1.13%) | 1963721 (+0.89%) | 1934615 (-0.60%) | 1960872 (+0.75%) | 1928481 (-0.92%) |
| night_1 | night | jpeg | 639193 | 667726 (+4.46%) | 663238 (+3.76%) | 664272 (+3.92%) | 642889 (+0.58%) | 650052 (+1.70%) |
| blurry_0 | blurry | jpeg | 1081484 | 1105455 (+2.22%) | 1104672 (+2.14%) | 1104569 (+2.13%) | 1088343 (+0.63%) | 1089300 (+0.72%) |
| blurry_1 | blurry | png | 687861 | 710268 (+3.26%) | 712683 (+3.61%) | 713366 (+3.71%) | 685484 (-0.35%) | 686112 (-0.25%) |
| text_0 | text | jpeg | 581186 | 600384 (+3.30%) | 600672 (+3.35%) | 600698 (+3.36%) | 592757 (+1.99%) | 592608 (+1.97%) |
| text_1 | text | jpeg | 1063018 | 1125835 (+5.91%) | 1109037 (+4.33%) | 1109526 (+4.38%) | 1083448 (+1.92%) | 1076348 (+1.25%) |
| document_0 | document | jpeg | 532641 | 535006 (+0.44%) | 535827 (+0.60%) | 535909 (+0.61%) | 533226 (+0.11%) | 534136 (+0.28%) |
| document_1 | document | jpeg | 413671 | 430861 (+4.16%) | 431087 (+4.21%) | 430097 (+3.97%) | 421948 (+2.00%) | 422278 (+2.08%) |
| screen_0 | screen | png | 143504 | 219207 (+52.75%) | 197980 (+37.96%) | 195164 (+36.00%) | 205704 (+43.34%) | 188321 (+31.23%) |
| screen_1 | screen | png | 186885 | 255082 (+36.49%) | 235646 (+26.09%) | 233334 (+24.85%) | 246990 (+32.16%) | 226271 (+21.07%) |

## Predeclared verdict (JPEG, candidate = P3)

aggregate gain -2.269% (keep >= 1.0%, reject < 0.3%), smaller on 1/20 (keep >= 15/20), worst image +5.574% (keep <= +0.5%), exact: true; enc time x1.27, dec time x1.27 vs X4 (keep <= 1.25). **Verdict: REJECT**

