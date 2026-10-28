## Aggregate (total canonical bytes / total encoded bytes)

| codec | total encoded B | ratio | saved % | mean per-image ratio | all exact |
|---|---:|---:|---:|---:|:--:|
| PRAW | 470,069,239 | 1.8494 | 45.93 | 2.0678 | True |
| PNG | 591,085,501 | 1.4707 | 32.01 | 1.6084 | True |
| JPEG XL | 440,816,057 | 1.9721 | 49.29 | 2.2570 | True |
| Zstd-19 | 519,793,942 | 1.6724 | 40.21 | 1.9071 | True |

Total canonical bytes: 869,328,040 across 25 images.

## Per-image (ratio = canonical bytes / encoded bytes; best per row in bold)

| id | camera | WxH | bits | DNG | PRAW | PNG | JXL | Zstd | PRAW vs JXL |
|---|---|---|--:|---|--:|--:|--:|--:|--:|
| pentax_k1ii_station | RICOH IMAGING COMPANY, LTD. PENTAX K-1 Mark II | 7392x4950 | 14 | JPEG | 1.589 | 1.261 | **1.617** | 1.327 | -1.7% |
| pentax_k30_frog | PENTAX PENTAX K-30 | 4960x3300 | 12 | JPEG | 2.039 | 1.506 | **2.138** | 1.768 | -4.6% |
| pentax_kx_lichen | PENTAX PENTAX K-x | 4352x2868 | 12 | JPEG | **2.054** | 1.444 | 1.974 | 1.744 | +4.1% |
| pentax_q7_cat | PENTAX PENTAX Q7 | 4064x3000 | 12 | NONE | 2.268 | 1.626 | **2.353** | 1.880 | -3.6% |
| pentax_k70_bulb | RICOH IMAGING COMPANY, LTD. PENTAX K-70 | 6080x4064 | 14 | JPEG | 2.043 | 1.796 | **2.361** | 2.263 | -13.5% |
| leica_cl_chateau | LEICA CAMERA AG LEICA CL | 6120x4016 | 14 | NONE | 1.543 | 1.264 | **1.590** | 1.400 | -3.0% |
| leica_m10_brick | Leica Camera AG LEICA M10 | 5984x3992 | 14 | JPEG | **1.508** | 1.216 | 1.483 | 1.266 | +1.7% |
| leica_q_canoe | LEICA CAMERA AG LEICA Q (Typ 116) | 6120x4016 | 14 | NONE | 1.569 | 1.252 | **1.609** | 1.375 | -2.5% |
| leica_x2_lowlight | LEICA CAMERA AG LEICA X2 | 4944x3272 | 12 | NONE | 2.570 | 1.851 | **2.819** | 2.343 | -8.9% |
| leica_m8_house | Leica Camera AG M8 Digital Camera | 3920x2638 | 8 | NONE | 3.265 | 2.492 | **3.488** | 2.919 | -6.4% |
| pixel6pro_valley | Google Pixel 6 Pro | 4080x3072 | 14 | JPEG | **1.960** | 1.399 | 1.935 | 1.575 | +1.3% |
| dji_fc220_dusk | DJI FC220 | 4000x3000 | 14 | NONE | 1.669 | 1.560 | **2.231** | 1.920 | -25.2% |
| parrot_anafi_dusk | None None | 5344x4016 | 14 | NONE | 1.661 | 2.105 | **3.112** | 2.697 | -46.6% |
| hasselblad_tallinn | Hasselblad L1D-20c | 5568x3648 | 16 | NONE | **1.347** | 1.187 | 1.333 | 1.304 | +1.0% |
| sigma_fp_bike14 | SIGMA SIGMA fp | 6064x4042 | 14 | JPEG | **1.809** | 1.315 | 1.805 | 1.471 | +0.2% |
| sigma_fp_bike8 | SIGMA SIGMA fp | 3856x2170 | 8 | NONE | 3.654 | 2.859 | **4.377** | 3.796 | -16.5% |
| nexus6_courtyard | motorola Nexus 6 | 4208x3120 | 10 | NONE | 2.624 | 1.934 | **2.833** | 2.566 | -7.4% |
| lg_h815_mountain | LG Mobile LG-H815 | 5312x2988 | 10 | NONE | 2.655 | 1.886 | **2.840** | 2.277 | -6.5% |
| ricoh_gr_wood | RICOH IMAGING COMPANY, LTD. GR | 4960x3280 | 12 | JPEG | 2.324 | 1.627 | **2.562** | 2.009 | -9.3% |
| ricoh_gr3_dunes | RICOH IMAGING COMPANY, LTD. RICOH GR III | 6112x4064 | 14 | JPEG | 1.625 | 1.262 | **1.636** | 1.354 | -0.7% |
| samsung_gx20_berries | SAMSUNG TECHWIN Co. SAMSUNG GX20 | 4864x3136 | 12 | NONE | 2.097 | 1.467 | **2.163** | 1.744 | -3.1% |
| nikon_d80_piano | NIKON CORPORATION NIKON D80 | 3900x2611 | 12 | JPEG | 2.366 | 1.687 | **2.435** | 1.982 | -2.8% |
| bmpcc4k_person | None None | 4128x2176 | 12 | JPEG | 1.868 | 1.450 | **1.961** | 1.693 | -4.7% |
| x100s_xtrans_chart | FUJIFILM X100S | 4936x3296 | 14 | JPEG | 1.649 | 1.343 | **1.781** | 1.454 | -7.4% |
| canon_sx40_snow | Canon Canon PowerShot SX40 HS | 4176x3060 | 12 | NONE | 1.937 | 1.422 | **1.988** | 1.551 | -2.6% |

('PRAW vs JXL' = how much smaller PRAW is than JXL: +x% means PRAW output is x% smaller.)

## Win/loss of each codec against PRAW (encoded bytes, per image)

| codec | smaller than PRAW | larger than PRAW |
|---|--:|--:|
| PNG | 1 | 24 |
| JPEG XL | 20 | 5 |
| Zstd-19 | 4 | 21 |

## PRAW best / worst by ratio

Worst: hasselblad_tallinn (1.347), leica_m10_brick (1.508), leica_cl_chateau (1.543), leica_q_canoe (1.569)

Best: sigma_fp_bike8 (3.654), leica_m8_house (3.265), lg_h815_mountain (2.655), nexus6_courtyard (2.624)

## PRAW diagnostics

| id | Stage 2 | bits/sample (payload) | S1 zero% | S1 mean zz | S1 H0 b/s | Low% | Mod% | High% | VHigh% | class-H0 b/s | enc s | dec s |
|---|---|--:|--:|--:|--:|--:|--:|--:|--:|--:|--:|--:|
| pentax_k1ii_station | bayer_planes | 10.067 | 0.4 | 840.7 | 10.840 | 0.1 | 0.3 | 5.5 | 94.2 | 10.772 | 51 | 70 |
| pentax_k30_frog | bayer_planes | 7.846 | 1.7 | 193.2 | 8.418 | 0.4 | 7.6 | 27.3 | 64.6 | 8.186 | 32 | 37 |
| pentax_kx_lichen | bayer_planes | 7.789 | 1.0 | 100.7 | 7.800 | 0.0 | 0.9 | 28.3 | 70.7 | 7.787 | 26 | 27 |
| pentax_q7_cat | bayer_planes | 7.055 | 2.6 | 117.3 | 7.527 | 0.5 | 17.5 | 47.0 | 35.0 | 7.145 | 26 | 31 |
| pentax_k70_bulb | bayer_planes | 7.833 | 2.0 | 137.9 | 8.198 | 1.1 | 1.4 | 21.9 | 75.6 | 8.076 | 49 | 58 |
| leica_cl_chateau | bayer_planes | 10.372 | 0.2 | 1445.9 | 11.298 | 0.0 | 0.1 | 2.9 | 97.0 | 11.247 | 48 | 50 |
| leica_m10_brick | bayer_planes | 10.610 | 3.6 | 999.8 | 11.075 | 3.3 | 0.2 | 1.4 | 95.1 | 10.865 | 45 | 46 |
| leica_q_canoe | bayer_planes | 10.195 | 2.2 | 848.3 | 10.549 | 1.3 | 0.4 | 7.2 | 91.1 | 10.398 | 56 | 63 |
| leica_x2_lowlight | bayer_planes | 6.227 | 5.2 | 111.2 | 6.942 | 5.1 | 30.1 | 33.7 | 31.1 | 6.574 | 35 | 45 |
| leica_m8_house | bayer_planes | 4.900 | 11.6 | 14.3 | 5.263 | 12.7 | 41.2 | 42.8 | 3.3 | 5.031 | 22 | 27 |
| pixel6pro_valley | bayer_planes | 8.163 | 1.2 | 213.4 | 8.561 | 0.1 | 3.8 | 32.5 | 63.6 | 8.383 | 38 | 26 |
| dji_fc220_dusk | bayer_planes | 9.584 | 0.4 | 441.0 | 10.003 | 0.0 | 0.1 | 5.1 | 94.8 | 9.978 | 27 | 25 |
| parrot_anafi_dusk | bayer_planes | 9.633 | 0.8 | 559.6 | 10.164 | 0.3 | 0.2 | 7.5 | 92.0 | 10.089 | 53 | 52 |
| hasselblad_tallinn | bayer_planes | 11.879 | 0.1 | 2419.9 | 12.367 | 0.0 | 0.1 | 1.1 | 98.8 | 12.329 | 46 | 49 |
| sigma_fp_bike14 | bayer_planes | 8.842 | 0.6 | 252.3 | 8.992 | 0.0 | 0.4 | 13.1 | 86.5 | 8.944 | 56 | 54 |
| sigma_fp_bike8 | bayer_planes | 4.378 | 12.7 | 17.7 | 5.097 | 25.8 | 39.0 | 22.8 | 12.3 | 4.478 | 23 | 19 |
| nexus6_courtyard | bayer_planes | 6.098 | 4.4 | 45.8 | 6.626 | 2.9 | 31.2 | 34.2 | 31.8 | 6.288 | 31 | 27 |
| lg_h815_mountain | bayer_planes | 6.026 | 5.2 | 42.7 | 6.593 | 5.3 | 29.4 | 35.0 | 30.3 | 6.281 | 38 | 37 |
| ricoh_gr_wood | bayer_planes | 6.884 | 3.9 | 81.8 | 7.386 | 3.4 | 21.0 | 34.9 | 40.7 | 7.022 | 39 | 40 |
| ricoh_gr3_dunes | bayer_planes | 9.848 | 0.7 | 499.1 | 10.234 | 0.6 | 1.1 | 5.2 | 93.1 | 10.136 | 61 | 43 |
| samsung_gx20_berries | bayer_planes | 7.631 | 3.1 | 103.5 | 8.026 | 2.0 | 4.3 | 32.4 | 61.3 | 7.803 | 26 | 27 |
| nikon_d80_piano | bayer_planes | 6.762 | 2.2 | 46.4 | 6.948 | 0.4 | 10.3 | 52.7 | 36.6 | 6.821 | 18 | 18 |
| bmpcc4k_person | bayer_planes | 8.566 | 0.7 | 282.2 | 9.305 | 0.1 | 1.3 | 9.3 | 89.4 | 9.201 | 15 | 15 |
| x100s_xtrans_chart | identity_morton | 9.703 | 0.4 | 510.6 | 10.214 | 0.0 | 0.5 | 10.8 | 88.7 | 9.941 | 29 | 29 |
| canon_sx40_snow | bayer_planes | 8.262 | 1.5 | 173.8 | 8.683 | 0.8 | 5.5 | 22.1 | 71.6 | 8.451 | 22 | 21 |

## Correctness gate

| codec | exact / total |
|---|--:|
| PRAW | 25 / 25 |
| PNG | 25 / 25 |
| JPEG XL | 25 / 25 |
| Zstd-19 | 25 / 25 |
