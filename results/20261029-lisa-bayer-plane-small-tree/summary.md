all_ok = True  (25 images)

## Aggregate payload bytes (Stage 4 payload)

| group | n | table128 (prod) | planar+table128 | tree128 | tree64 | tree32 |
|---|---|---|---|---|---|---|
| ALL 25 | 25 | 459,359,578 | 397,011,489 (-13.573%) | 459,014,708 (-0.075%) | 459,440,168 (+0.018%) | 460,434,653 (+0.234%) |
| train | 13 | 270,482,657 | 238,006,467 (-12.007%) | 269,892,101 (-0.218%) | 270,131,631 (-0.130%) | 270,619,867 (+0.051%) |
| val | 4 | 79,467,975 | 68,461,848 (-13.850%) | 79,385,056 (-0.104%) | 79,458,163 (-0.012%) | 79,606,515 (+0.174%) |
| test | 8 | 109,408,946 | 90,543,174 (-17.243%) | 109,737,551 (+0.300%) | 109,850,374 (+0.403%) | 110,208,271 (+0.731%) |

## Tree variants vs tree128 (aggregate)

| group | tree64 vs tree128 | tree32 vs tree128 |
|---|---|---|
| ALL 25 | +0.093% | +0.309% |
| train | +0.089% | +0.270% |
| val | +0.092% | +0.279% |
| test | +0.103% | +0.429% |

## Per image (bytes; % vs production table128)

| image | split | bits | prod table128 | planar | tree128 | tree64 | tree32 |
|---|---|---|---|---|---|---|---|
| bmpcc4k_person | train | 12 | 9,313,387 | 7,517,904 (-19.28%) | 9,277,981 (-0.38%) | 9,296,208 (-0.18%) | 9,325,304 (+0.13%) |
| canon_sx40_snow | train | 12 | 12,984,352 | 11,954,171 (-7.93%) | 12,985,386 (+0.01%) | 12,988,214 (+0.03%) | 13,007,499 (+0.18%) |
| hasselblad_tallinn | train | 16 | 29,652,178 | 25,974,547 (-12.40%) | 29,565,678 (-0.29%) | 29,571,881 (-0.27%) | 29,592,236 (-0.20%) |
| leica_m8_house | train | 8 | 6,221,324 | 5,443,149 (-12.51%) | 6,245,183 (+0.38%) | 6,264,597 (+0.70%) | 6,281,292 (+0.96%) |
| leica_q_canoe | train | 14 | 30,243,521 | 27,966,309 (-7.53%) | 30,181,926 (-0.20%) | 30,213,541 (-0.10%) | 30,275,249 (+0.10%) |
| lg_h815_mountain | train | 10 | 11,701,527 | 9,588,201 (-18.06%) | 11,755,189 (+0.46%) | 11,770,863 (+0.59%) | 11,816,203 (+0.98%) |
| parrot_anafi_dusk | train | 14 | 25,655,767 | 23,137,314 (-9.82%) | 25,552,052 (-0.40%) | 25,575,350 (-0.31%) | 25,611,133 (-0.17%) |
| pentax_k1ii_station | train | 14 | 44,494,134 | 36,498,470 (-17.97%) | 44,247,852 (-0.55%) | 44,305,366 (-0.42%) | 44,438,181 (-0.13%) |
| pentax_k70_bulb | train | 14 | 23,836,850 | 20,396,871 (-14.43%) | 23,780,736 (-0.24%) | 23,783,522 (-0.22%) | 23,798,984 (-0.16%) |
| pixel6pro_valley | train | 14 | 12,573,085 | 11,299,890 (-10.13%) | 12,561,250 (-0.09%) | 12,563,979 (-0.07%) | 12,579,111 (+0.05%) |
| ricoh_gr3_dunes | train | 14 | 29,877,634 | 26,829,113 (-10.20%) | 29,829,941 (-0.16%) | 29,851,238 (-0.09%) | 29,903,524 (+0.09%) |
| samsung_gx20_berries | train | 12 | 14,195,250 | 11,666,880 (-17.81%) | 14,237,738 (+0.30%) | 14,252,582 (+0.40%) | 14,290,495 (+0.67%) |
| x100s_xtrans_chart | train | 14 | 19,733,648 | 19,733,648 (+0.00%) | 19,671,189 (-0.32%) | 19,694,290 (-0.20%) | 19,700,656 (-0.17%) |
| leica_cl_chateau | val | 14 | 31,007,703 | 25,248,944 (-18.57%) | 30,897,046 (-0.36%) | 30,946,538 (-0.20%) | 31,044,368 (+0.12%) |
| nexus6_courtyard | val | 10 | 9,835,629 | 8,006,667 (-18.60%) | 9,874,043 (+0.39%) | 9,884,462 (+0.50%) | 9,909,160 (+0.75%) |
| pentax_kx_lichen | val | 12 | 12,060,697 | 11,458,087 (-5.00%) | 12,064,363 (+0.03%) | 12,064,804 (+0.03%) | 12,066,315 (+0.05%) |
| sigma_fp_bike14 | val | 14 | 26,563,946 | 23,748,150 (-10.60%) | 26,549,604 (-0.05%) | 26,562,359 (-0.01%) | 26,586,672 (+0.09%) |
| dji_fc220_dusk | test | 14 | 14,125,601 | 12,030,903 (-14.83%) | 14,084,252 (-0.29%) | 14,091,186 (-0.24%) | 14,109,598 (-0.11%) |
| leica_m10_brick | test | 14 | 30,922,297 | 27,738,775 (-10.30%) | 30,878,027 (-0.14%) | 30,898,336 (-0.08%) | 30,941,065 (+0.06%) |
| leica_x2_lowlight | test | 12 | 12,100,174 | 8,727,947 (-27.87%) | 12,168,636 (+0.57%) | 12,193,232 (+0.77%) | 12,260,757 (+1.33%) |
| nikon_d80_piano | test | 12 | 8,535,478 | 7,416,588 (-13.11%) | 8,558,380 (+0.27%) | 8,558,492 (+0.27%) | 8,574,085 (+0.45%) |
| pentax_k30_frog | test | 12 | 15,269,777 | 11,836,232 (-22.49%) | 15,399,722 (+0.85%) | 15,413,603 (+0.94%) | 15,511,065 (+1.58%) |
| pentax_q7_cat | test | 12 | 10,533,683 | 9,288,949 (-11.82%) | 10,539,364 (+0.05%) | 10,541,375 (+0.07%) | 10,561,129 (+0.26%) |
| ricoh_gr_wood | test | 12 | 13,501,046 | 10,599,799 (-21.49%) | 13,612,189 (+0.82%) | 13,640,486 (+1.03%) | 13,695,381 (+1.44%) |
| sigma_fp_bike8 | test | 8 | 4,420,890 | 2,903,981 (-34.31%) | 4,496,981 (+1.72%) | 4,513,664 (+2.10%) | 4,555,191 (+3.04%) |

Planar Stage 1 applies to 24/25 images (Bayer metadata); the others are byte-identical to production by construction.
Planar smaller on 24/24 Bayer images; tree64 smaller than table on 13/25, tree32 on 6/25.

## Context occupancy (per image, then pooled)

Per-image visit counts. bottom50 = share of visits in the least-visited half of the leaves. low = leaves with < 2,048 visits in that image.

- **table128_prod** (128 contexts): bottom-50% share median 6.79%, <5% on 10/25 images; leaves with <2,048 visits per image: median 7, max 93; visit share in those leaves: median 0.031%, max 0.268%; unused contexts (0 visits) median 0
- **tree128** (128 contexts): bottom-50% share median 0.58%, <5% on 19/25 images; leaves with <2,048 visits per image: median 49, max 115; visit share in those leaves: median 0.076%, max 0.456%; unused contexts (0 visits) median 0
- **tree64** (64 contexts): bottom-50% share median 1.60%, <5% on 17/25 images; leaves with <2,048 visits per image: median 8, max 54; visit share in those leaves: median 0.018%, max 0.148%; unused contexts (0 visits) median 0
- **tree32** (32 contexts): bottom-50% share median 2.87%, <5% on 15/25 images; leaves with <2,048 visits per image: median 2, max 25; visit share in those leaves: median 0.003%, max 0.069%; unused contexts (0 visits) median 0
- **planar_table128** (128 contexts): bottom-50% share median 0.70%, <5% on 19/25 images; leaves with <2,048 visits per image: median 49, max 115; visit share in those leaves: median 0.097%, max 0.421%; unused contexts (0 visits) median 0

Pooled per split (sum of per-image visit vectors; bottom-50% share of pooled visits):

| variant | split | bottom-50% share | min leaf visits | max leaf visits |
|---|---|---|---|---|
| table128_prod | train | 23.50% | 58,701 | 9,523,344 |
| table128_prod | val | 13.83% | 6,131 | 5,242,299 |
| table128_prod | test | 17.51% | 18,791 | 5,129,110 |
| tree128 | train | 7.71% | 8,835 | 34,329,700 |
| tree128 | val | 3.38% | 827 | 15,047,357 |
| tree128 | test | 4.17% | 4,666 | 14,196,082 |
| tree64 | train | 12.41% | 18,658 | 34,379,527 |
| tree64 | val | 6.89% | 827 | 15,068,812 |
| tree64 | test | 6.95% | 6,479 | 16,865,721 |
| tree32 | train | 17.47% | 147,516 | 42,373,256 |
| tree32 | val | 11.88% | 5,927 | 17,522,917 |
| tree32 | test | 10.68% | 32,758 | 20,276,965 |
