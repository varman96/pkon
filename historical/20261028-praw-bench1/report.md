# PRAW Benchmark 1: diverse RAW compression (25 DNGs)

Date 2026-10-28. PRAW = existing four-stage pipeline (`src/praw.rs`, Stage 4 coding_version 1, Stage 3 rule v1), **unmodified**.

## Method
- **Corpus**: 25 genuine DNGs from raw.pixls.us (CC0), chosen from a 49-file candidate pool after inspecting metadata and low-res previews. Full SHA-256 in `dataset_manifest.json` (DNG and canonical bytes). Rejected as non-mosaic: Leica M Monochrom (mono LinearRaw), Samsung S22 Ultra (3-channel LinearRaw).
- **Extraction** (`tools/praw_dng.py`; tifffile 2026.10.15, imagecodecs 2026.10.10): the full stored raster of the raw IFD, with no crop, black/white-level handling, linearization, WB or demosaic. Canonical form = row-major little-endian uint16, 2 bytes/sample for every image (8/10/12/14/16-bit all widened to 16). The lossless-JPEG DNG decode was **not cross-validated** against a second decoder (previews looked sane; all codecs saw the same array).
- **PRAW bit depth** = min(declared BitsPerSample, bits(max(WhiteLevel, observed max))), asserting every sample fits. CFA origin follows the DNG ActiveArea rule. Valid 2x2 Bayer -> `BayerCfa`; X-Trans -> `OtherCfa` (Stage 2 identity/Morton fallback).
- **PRAW size = Stage 4 payload only.** No serialized PRAW container exists, so reconstruction metadata (width, height, bit depth, layout, CFA pattern+origin; a few dozen bytes) is excluded rather than invented; even 64 B would change no ratio by more than 0.0002%. The other codecs are complete files/frames including headers.
- **PNG**: libpng libpng 1.6.59 via imagecodecs, 16-bit gray, zlib 9. **JPEG XL**: libjxl libjxl 0.12.0, `lossless=True`, effort 7, single thread, 16-bit gray. **Zstd**: zstd zstd 1.5.7, level 19, on raw canonical LE16 bytes (no byte shuffle). One fixed configuration each; JXL effort 9 and zstd shuffle/long mode were not tried.
- **Gate**: every output decoded and compared sample-for-sample (shape, dtype, values; PRAW decoded from the Stage 4 output only, metadata compared). 25/25 exact for all four codecs.
- **Timing** (secondary): PRAW is single-threaded Rust, and the baseline job ran concurrently on another core, so times are indicative only. PRAW encode roughly 15-60 s and decode 15-70 s per image (about 3-4 µs/sample).
- **Reproduce**: `tools/praw_bench1_prepare.py` -> `cargo run --release --bin bench_praw1` -> `tools/praw_bench1_baselines.py` -> `tools/praw_bench1_report.py` (Python run with `-I`; manifest paths as in this directory).

## 1. Dataset manifest
| id | camera | WxH | declared→used bits | CFA | DNG storage | stored dtype | file bytes | sha256 (DNG, prefix) | scene (visual, from low-res preview) | source (raw.pixls.us/data/) |
|---|---|---|---|---|---|---|--:|---|---|---|
| pentax_k1ii_station | RICOH IMAGING COMPANY, LTD. PENTAX K-1 Mark II | 7392x4950 | 14→14 | Bayer [0, 1, 1, 2] org [0, 0] | JPEG (code 7, predictor 1, strips) | uint16 | 46,069,312 | `190d5878675f4de0…` | daylight overcast; architecture + trains; high detail; large (36 MP) | Pentax/K-1%20Mark%20II/_IMG2387.DNG |
| pentax_k30_frog | PENTAX PENTAX K-30 | 4960x3300 | 12→12 | Bayer [0, 1, 1, 2] org [0, 0] | JPEG (code 7, predictor 1, strips) | uint16 | 14,234,631 | `63eaa1ff93ad7c89…` | water + wildlife (frog on lily pads); specular water | Pentax/K-30/_IGP4042.DNG |
| pentax_kx_lichen | PENTAX PENTAX K-x | 4352x2868 | 12→12 | Bayer [2, 1, 1, 0] org [0, 0] | JPEG (code 7, predictor 1, strips) | uint16 | 14,359,791 | `7a594b66b54f99bb…` | fine organic texture (lichen on fabric); daylight | Pentax/K-x/IMGP6431.DNG |
| pentax_q7_cat | PENTAX PENTAX Q7 | 4064x3000 | 12→12 | Bayer [0, 1, 1, 2] org [0, 0] | NONE (code 1, predictor 1, strips) | uint16 | 20,880,795 | `577eac8d03419bb8…` | animal fur (black cat); dark subject, small sensor | Pentax/Q7/IMGP1691.DNG |
| pentax_k70_bulb | RICOH IMAGING COMPANY, LTD. PENTAX K-70 | 6080x4064 | 14→14 | Bayer [0, 1, 1, 2] org [0, 0] | JPEG (code 7, predictor 1, strips) | uint16 | 28,786,390 | `69a65be065c5e6c4…` | night / low-light (lit bulb in dark room); noise-dominated | Pentax/K-70/IMGP6855.DNG |
| leica_cl_chateau | LEICA CAMERA AG LEICA CL | 6120x4016 | 14→14 | Bayer [1, 2, 0, 1] org [0, 0] | NONE (code 1, predictor 1, strips) | uint16 | 44,546,560 | `1f4cfa565d176d85…` | overcast landscape; chateau, grass, large smooth sky | Leica/CL/L1080510_2018_CL-35-135-Chateau_Thy__Chateau_Ham.DNG |
| leica_m10_brick | Leica Camera AG LEICA M10 | 5984x3992 | 16→14 | Bayer [0, 1, 1, 2] org [0, 0] | JPEG (code 7, predictor 1, strips) | uint16 | 34,209,792 | `26be5e06dfb53a29…` | daylight architecture (brick building); fine regular structure | Leica/M10/f5381888.dng |
| leica_q_canoe | LEICA CAMERA AG LEICA Q (Typ 116) | 6120x4016 | 14→14 | Bayer [2, 1, 1, 0] org [0, 0] | NONE (code 1, predictor 1, strips) | uint16 | 45,100,032 | `f7b2a6b6d7684536…` | water + foliage (canoe on swamp channel) | Leica/Q%20(Typ%20116)/L1070048.DNG |
| leica_x2_lowlight | LEICA CAMERA AG LEICA X2 | 4944x3272 | 12→12 | Bayer [0, 1, 1, 2] org [0, 0] | NONE (code 1, predictor 1, strips) | uint16 | 24,453,726 | `a67eca6271afb105…` | low-light interior; defocused, smooth and noisy | Leica/X2/L1021324.DNG |
| leica_m8_house | Leica Camera AG M8 Digital Camera | 3920x2638 | 8→8 | Bayer [0, 1, 1, 2] org [0, 0] | NONE (code 1, predictor 1, strips) | uint8 | 10,575,296 | `7fdcca2371bb07fb…` | daylight architecture + foliage; 8-bit stored samples | Leica/M8/L1030132.DNG |
| pixel6pro_valley | Google Pixel 6 Pro | 4080x3072 | 16→14 | Bayer [1, 2, 0, 1] org [0, 0] | JPEG (code 7, predictor 1, tiled) | uint16 | 11,932,956 | `3a859e917a91698a…` | mountain landscape + sky; phone sensor | Google/Pixel%206%20Pro/PXL_20220910_093206982.dng |
| dji_fc220_dusk | DJI FC220 | 4000x3000 | 16→14 | Bayer [0, 1, 1, 2] org [0, 0] | NONE (code 1, predictor 1, strips) | uint16 | 24,331,468 | `85a4fc27cca3c135…` | dusk skyline + water + sky; drone | DJI/FC220/DJI_0004.DNG |
| parrot_anafi_dusk | None None | 5344x4016 | 16→14 | Bayer [2, 1, 1, 0] org [0, 0] | NONE (code 1, predictor 1, strips) | uint16 | 43,001,162 | `a257a296047ff2d3…` | dusk / heavy overcast; town from above; dark | Parrot/Anafi/100000190019.DNG |
| hasselblad_tallinn | Hasselblad L1D-20c | 5568x3648 | 16→16 | Bayer [0, 1, 1, 2] org [0, 0] | NONE (code 1, predictor 1, strips) | uint16 | 41,369,400 | `7afecae0db80d832…` | daylight aerial of dense old town (roofs, many small features); true 16-bit range | Hasselblad/L1D-20c/DJI_0017.DNG |
| sigma_fp_bike14 | SIGMA SIGMA fp | 6064x4042 | 14→14 | Bayer [0, 1, 1, 2] org [0, 0] | JPEG (code 7, predictor 1, tiled) | uint16 | 34,676,310 | `0844bcf23a49e21b…` | machinery/vehicle detail (bicycle handlebar close-up); shallow depth of field | Sigma/fp/SDIM4591.DNG |
| sigma_fp_bike8 | SIGMA SIGMA fp | 3856x2170 | 8→8 | Bayer [0, 1, 1, 2] org [0, 0] | NONE (code 1, predictor 1, strips) | uint8 | 8,447,488 | `b7e44ca41c5c5a01…` | same bicycle scene, genuine 8-bit uncompressed cinema DNG | Sigma/fp/A002_650_20240229_000001_4k_8bit.DNG |
| nexus6_courtyard | motorola Nexus 6 | 4208x3120 | 16→10 | Bayer [2, 1, 1, 0] org [0, 0] | NONE (code 1, predictor 1, strips) | uint16 | 26,288,408 | `c92496d31742e619…` | bright daylight courtyard, sky and clouds; 10-bit phone | motorola/Nexus%206/DSC_0017.DNG |
| lg_h815_mountain | LG Mobile LG-H815 | 5312x2988 | 10→10 | Bayer [2, 1, 1, 0] org [0, 0] | NONE (code 1, predictor 1, strips) | uint16 | 20,027,632 | `6719693cddbbe348…` | snow mountain landscape + sky; 10-bit phone | LG/H815/20161227_150823.dng |
| ricoh_gr_wood | RICOH IMAGING COMPANY, LTD. GR | 4960x3280 | 12→12 | Bayer [0, 1, 1, 2] org [0, 0] | JPEG (code 7, predictor 1, strips) | uint16 | 12,129,815 | `7ceec1ea31c56b6e…` | fine texture (weathered painted wood, dry grass) | Ricoh/GR/R0003611.DNG |
| ricoh_gr3_dunes | RICOH IMAGING COMPANY, LTD. RICOH GR III | 6112x4064 | 14→14 | Bayer [0, 1, 1, 2] org [0, 0] | JPEG (code 7, predictor 1, strips) | uint16 | 33,374,940 | `05513ee72f7cac41…` | daylight beach landscape, dune grass, clouds | Ricoh/GR%20III/R0000357.DNG |
| samsung_gx20_berries | SAMSUNG TECHWIN Co. SAMSUNG GX20 | 4864x3136 | 12→12 | Bayer [2, 1, 1, 0] org [0, 0] | NONE (code 1, predictor 1, strips) | uint16 | 24,061,568 | `5557016b2712b621…` | foliage + fruit close-up; shallow depth of field | Samsung/GX20/_G205027.DNG |
| nikon_d80_piano | NIKON CORPORATION NIKON D80 | 3900x2611 | 16→12 | Bayer [1, 2, 0, 1] org [0, 0] | JPEG (code 7, predictor 1, tiled) | uint16 | 8,283,990 | `5095ca264e763624…` | indoor, dim-light (piano by a window); converted Nikon DNG | Nikon/D80/DSC_0143.dng |
| bmpcc4k_person | None None | 4128x2176 | 12→12 | Bayer [0, 1, 1, 2] org [0, 0] | JPEG (code 7, predictor 1, tiled) | uint16 | 8,069,760 | `a90b542deab59409…` | individual person (portrait, workshop) indoors; cinema DNG | Blackmagic/Blackmagic%20Pocket%20Cinema%20Camera%204k/BMPCC_4k_DCI_lossless.dng |
| x100s_xtrans_chart | FUJIFILM X100S | 4936x3296 | 16→14 | X-Trans 6x6 [0, 1, 0, 2, 1, 2, 1, 2, 1, 1, 0, 1, 1, 0, 1, 1, 2, 1, 2, 1, 2, 0, 1, 0, 1, 0, 1, 1, 2, 1, 1, 2, 1, 1, 0, 1] org [0, 0] | JPEG (code 7, predictor 1, tiled) | uint16 | 20,729,906 | `7adec564bbf74858…` | X-Trans 6x6 CFA (non-Bayer); colour chart on dark ground | Fujifilm/X100S/fujifilm-x100s-daylight-DSCF9505.dng |
| canon_sx40_snow | Canon Canon PowerShot SX40 HS | 4176x3060 | 12→12 | Bayer [0, 1, 1, 2] org [0, 0] | NONE (code 1, predictor 1, strips) | uint16 | 19,206,240 | `dab360e60b94a3f0…` | overcast winter street (bare trees, parked cars, snow); CHDK uncompressed | Canon/PowerShot%20SX40%20HS/CRW_6037.DNG |

Coverage gaps (observed): no crowds/groups, no feathers/birds, one portrait (BMPCC), fur via one black cat. "Night" is a lit bulb in a dark room plus dusk drone shots. Scene labels are my visual classification of low-res previews. Leica M8 and Sigma fp 8-bit are genuinely 8-bit stored samples. No decode failures occurred among the 25 selected files.

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

## Observed facts
- Aggregate: JPEG XL 1.972 > PRAW 1.849 > Zstd-19 1.672 > PNG 1.471. JXL output is 6.2% smaller than PRAW in total (440.8 MB vs 470.1 MB); PRAW is 20.5% smaller than PNG and 9.6% smaller than Zstd. JXL beats PRAW on 20/25 images.
- PRAW beats JXL on 5/25 images, always narrowly (+0.2% to +4.1%): pentax_kx_lichen, leica_m10_brick, pixel6pro_valley, hasselblad_tallinn, sigma_fp_bike14. Those are textured or high-entropy images (7.8-11.9 bits/sample).
- JXL's largest wins: (a) **sparse-valued data**: parrot_anafi_dusk (-46.6%) and dji_fc220_dusk (-25.2%). Measured: every sample is a multiple of 16 and 4 respectively, and only 6% and 11% of the 14-bit value range occurs (about 10/12-bit data shifted into a 14-bit container). Even PNG (2.105) and Zstd (2.697) beat PRAW (1.661) on parrot. (b) **low-entropy / 8-bit files**: sigma_fp_bike8 -16.5%, leica_m8_house -6.4%, leica_x2_lowlight -8.9%. (c) the noisy low-light pentax_k70_bulb (-13.5%; 52% of the value range occurs). Elsewhere JXL leads by 0.7-7.4%.
- Stage 3 classes are degenerate on high-bit-depth data: on 14/16-bit images 86-99% of samples land in VeryHigh (fixed zigzag-score threshold 48), e.g. hasselblad 98.8%, leica_cl 97.0%. They spread out only on 8-12-bit images. Static class-conditioned order-0 entropy beats plain Stage 1 order-0 entropy by 0.01-0.4 bits/sample, most on the spread-out images (x2 0.37, nexus6 0.34, lg 0.31).
- The one non-Bayer file (X-Trans, identity/Morton path) lost to JXL by 7.4%, but there is no Bayer-plane counterpart of it, so this does not isolate the organization path. The other 24 used Bayer planes.
- Stage 4 payloads are at or below Stage 1 and class order-0 entropy on every image (e.g. k1ii 10.07 vs 10.84 bits/sample), so the adaptive coder captures real local structure beyond static class statistics.

## Interpretation (hypotheses, not established)
1. PRAW is a credible lossless RAW coder: it beats PNG and Zstd on nearly every image and is within about 1-5% of JXL e7 on typical 12-14-bit camera files, but it does not beat JXL in aggregate.
2. The largest losses (dusk drone files) look like **value-set sparsity** that spatial prediction plus zigzag residuals cannot exploit and JXL can. Support: GCD/distinct-value counts, and PNG/Zstd also beating PRAW there. Not proven: lichen has only 33% of values populated and PRAW still wins, so sparsity alone does not predict loss.
3. Stage 3 activity thresholds appear mis-scaled for 14-bit and 16-bit data (classes collapse into one), so context selection contributes little there. This might explain relative weakness on low-entropy content, but it does not explain the sparse-value losses and its effect size is untested.
4. No stage can be credited for PRAW's wins from this data: no ablations were run.

## Next experiments (each tied to an observation)
1. **Value-set diagnostic** (dusk files; sigma8/m8/k70). Without changing PRAW, losslessly divide by the sample GCD (and separately try a histogram-rank remap) in the harness and rerun PRAW on the 6-8 affected images. If the dusk gaps close from -25..-47% to a few percent, a small value-set pre-stage is the next algorithm item; if not, the loss is in prediction/context.
2. **Stage 3 threshold-scaling ablation** (class saturation at 14/16 bits). Offline, scale `STAGE3_THRESHOLDS` by 2^(bit_depth-12) on about 8 images spanning 10-16 bits and compare payloads; accept only if gains are broad. This is an algorithm change and was deliberately not made here.

## Files
`dataset_manifest.json`, `praw_results.json` (all PRAW diagnostics incl. per-plane and per-class stats), `baseline_results.json`, `per_image.csv`, `tables.md`, stdout logs. Tools: `tools/praw_*`, runner `src/bin/bench_praw1.rs`. Canonical samples live in gitignored `data/praw1/`.
