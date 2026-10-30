# PKON2 milestone 1 probe

23 images, centre-cropped to at most 1024x1024 (exact pixels, no resampling). Metric: **bits per sample = coded bytes*8 / (w*h*channels)** (lower is better); coded bytes include the 11-byte header and all adaptive-model learning cost (no side tables). Config label = predictor/colour/context. **All 11 configs x images decoded bit-exactly: true.**

## Dataset

| image | cat | src | camera | orig | crop | uniq colours/px | simple 16x16 blocks | px repeating W/N | edge px (act>40) | raw B | png-rs B | best PKON2 B |
|---|---|---|---|---|---|---:|---:|---:|---:|---:|---:|---:|
| outdoor_0 | outdoor | jpeg | NIKON COOLPIX P300 | 4000x3000 | 1024x1024 | 0.036 | 0.000 | 0.127 | 0.054 | 3145728 | 1417416 | 815272 |
| outdoor_1 | outdoor | jpeg | SONY ILCE-7S | 4240x2832 | 1024x1024 | 0.068 | 0.281 | 0.491 | 0.077 | 3145728 | 932530 | 547778 |
| indoor_0 | indoor | jpeg | Panasonic DMC-G80 | 4592x3448 | 1024x1024 | 0.028 | 0.263 | 0.601 | 0.053 | 3145728 | 775015 | 379149 |
| indoor_1 | indoor | jpeg | Xiaomi 23030RAC7Y | 3000x4000 | 1024x1024 | 0.104 | 0.024 | 0.270 | 0.089 | 3145728 | 1168705 | 646697 |
| portrait_0 | portrait | jpeg | Apple iPhone 15 Pro | 2034x2717 | 1024x1024 | 0.052 | 0.002 | 0.315 | 0.005 | 3145728 | 880271 | 455539 |
| pet_0 | pet | jpeg | NIKON CORPORATION NIKON D300 | 3607x2234 | 1024x1024 | 0.031 | 0.000 | 0.166 | 0.009 | 3145728 | 1279408 | 793308 |
| pet_1 | pet | jpeg | Panasonic DMC-FZ1000 | 3850x3283 | 1024x1024 | 0.040 | 0.000 | 0.133 | 0.011 | 3145728 | 1332222 | 857767 |
| pet2_0 | pet | jpeg | Canon Canon EOS 700D | 2877x3456 | 1024x1024 | 0.107 | 0.000 | 0.139 | 0.090 | 3145728 | 1479400 | 859192 |
| pet2_1 | pet | jpeg | Xiaomi M2103K19I | 4000x2992 | 1024x1024 | 0.066 | 0.026 | 0.391 | 0.019 | 3145728 | 983672 | 528629 |
| food_0 | food | jpeg | SONY DSC-W530 | 1572x1078 | 1024x1024 | 0.245 | 0.000 | 0.047 | 0.187 | 3145728 | 1996090 | 1365488 |
| food_1 | food | jpeg | SONY DSC-W530 | 2274x1666 | 1024x1024 | 0.074 | 0.000 | 0.170 | 0.077 | 3145728 | 1450115 | 795598 |
| foliage_0 | foliage | jpeg | SONY DSC-RX100 | 1831x1831 | 1024x1024 | 0.101 | 0.001 | 0.062 | 0.290 | 3145728 | 2016330 | 1280760 |
| foliage_1 | foliage | jpeg | Xiaomi 220233L2G | 4160x3120 | 1024x1024 | 0.105 | 0.000 | 0.219 | 0.031 | 3145728 | 1237201 | 669030 |
| night_0 | night | jpeg | NIKON CORPORATION NIKON D300 | 2000x1329 | 1024x1024 | 0.160 | 0.001 | 0.016 | 0.277 | 3145728 | 2290163 | 1986408 |
| night_1 | night | jpeg | Panasonic DMC-FZ1000 | 4949x3140 | 1024x1024 | 0.009 | 0.000 | 0.215 | 0.002 | 3145728 | 1046156 | 670877 |
| blurry_0 | blurry | jpeg | Canon Canon DIGITAL IXUS 40 | 1600x1200 | 1024x1024 | 0.071 | 0.000 | 0.058 | 0.000 | 3145728 | 1554190 | 1133209 |
| blurry_1 | blurry | png | - | 2160x1620 | 1024x1024 | 0.069 | 0.000 | 0.168 | 0.032 | 4194304 | 1280963 | 785024 |
| text_0 | text | jpeg | samsung SM-A127F | 4000x3000 | 1024x1024 | 0.073 | 0.238 | 0.381 | 0.066 | 3145728 | 1046410 | 622632 |
| text_1 | text | jpeg | Apple iPhone XS | 2573x1959 | 1024x1024 | 0.046 | 0.000 | 0.057 | 0.308 | 3145728 | 1716246 | 1136704 |
| document_0 | document | jpeg | motorola moto g pure | 4160x3120 | 1024x1024 | 0.007 | 0.000 | 0.208 | 0.056 | 3145728 | 957594 | 548420 |
| document_1 | document | jpeg | Xiaomi 2312CRAD3C | 3060x4080 | 1024x1024 | 0.007 | 0.210 | 0.518 | 0.065 | 3145728 | 796743 | 434033 |
| screen_0 | screen | png | - | 2880x1628 | 1024x1024 | 0.002 | 0.664 | 0.950 | 0.126 | 4194304 | 229481 | 171715 |
| screen_1 | screen | png | - | 1907x1360 | 1024x1024 | 0.004 | 0.738 | 0.947 | 0.086 | 4194304 | 153347 | 175656 |

## Configs, mean bits/sample by source group

Delta is vs `Med/Raw/None` (index 4): mean of per-image relative change; `better/worse` counts images. H0 = order-0 residual entropy, Hctx = context-conditional entropy (both static, no learning cost, optimistic); coded = real bytes.

### camera JPEG photos (decoded) (n=20)

| config | coded bps | H0 bps | Hctx bps | delta vs Med/Raw/None | better/worse | zero resid | \|r\|<=2 | enc ms | dec ms |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| W/Raw/None | 4.289 | 4.328 | 4.328 | +12.34% | 0/20 | 0.189 | 0.557 | 134 | 155 |
| N/Raw/None | 4.326 | 4.377 | 4.377 | +13.43% | 0/20 | 0.182 | 0.548 | 128 | 158 |
| Avg/Raw/None | 4.009 | 4.055 | 4.055 | +4.95% | 3/17 | 0.207 | 0.611 | 141 | 153 |
| Grad/Raw/None | 4.000 | 4.041 | 4.041 | +4.42% | 1/19 | 0.214 | 0.595 | 139 | 156 |
| Med/Raw/None | 3.834 | 3.876 | 3.876 | +0.00% | 0/0 | 0.231 | 0.635 | 155 | 176 |
| Med/Raw/Act | 3.621 | 3.876 | 3.608 | -5.94% | 20/0 | 0.231 | 0.635 | 188 | 210 |
| Med/SubG/Act | 2.165 | 2.344 | 2.152 | -45.47% | 20/0 | 0.557 | 0.821 | 143 | 158 |
| Med/Coupled/Act | 2.377 | 2.503 | 2.370 | -39.61% | 20/0 | 0.521 | 0.814 | 169 | 190 |
| Med/Raw/ActX | 3.083 | 3.876 | 3.067 | -20.93% | 20/0 | 0.231 | 0.635 | 197 | 217 |
| Med/SubG/ActX | 2.103 | 2.344 | 2.087 | -47.12% | 20/0 | 0.557 | 0.821 | 154 | 169 |
| Med/Coupled/ActX | 2.311 | 2.503 | 2.298 | -41.40% | 20/0 | 0.521 | 0.814 | 181 | 199 |

Reference: png-rs mean bps 3.351; png-rs vs last config (Med/Coupled/ActX): +49.6% bytes on average, PKON2-probe smaller on 20/20 images.

### PNG (screenshots / other) (n=3)

| config | coded bps | H0 bps | Hctx bps | delta vs Med/Raw/None | better/worse | zero resid | \|r\|<=2 | enc ms | dec ms |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| W/Raw/None | 1.396 | 1.520 | 1.520 | +17.14% | 0/3 | 0.759 | 0.867 | 117 | 155 |
| N/Raw/None | 1.380 | 1.526 | 1.526 | +8.25% | 2/1 | 0.760 | 0.862 | 125 | 151 |
| Avg/Raw/None | 1.421 | 1.581 | 1.581 | +31.27% | 0/3 | 0.755 | 0.867 | 118 | 139 |
| Grad/Raw/None | 1.184 | 1.285 | 1.285 | +2.79% | 1/2 | 0.783 | 0.904 | 114 | 134 |
| Med/Raw/None | 1.174 | 1.276 | 1.276 | +0.00% | 0/0 | 0.786 | 0.904 | 124 | 141 |
| Med/Raw/Act | 1.087 | 1.276 | 1.091 | -12.24% | 3/0 | 0.786 | 0.904 | 155 | 172 |
| Med/SubG/Act | 0.755 | 0.888 | 0.751 | -31.28% | 3/0 | 0.843 | 0.954 | 133 | 161 |
| Med/Coupled/Act | 0.780 | 0.889 | 0.776 | -32.59% | 3/0 | 0.839 | 0.956 | 163 | 196 |
| Med/Raw/ActX | 0.944 | 1.276 | 0.937 | -22.76% | 3/0 | 0.786 | 0.904 | 200 | 234 |
| Med/SubG/ActX | 0.730 | 0.888 | 0.722 | -35.34% | 3/0 | 0.843 | 0.954 | 203 | 206 |
| Med/Coupled/ActX | 0.756 | 0.889 | 0.747 | -36.13% | 3/0 | 0.839 | 0.956 | 182 | 200 |

Reference: png-rs mean bps 1.058; png-rs vs last config (Med/Coupled/ActX): +24.4% bytes on average, PKON2-probe smaller on 2/3 images.

## Per image: coded bps for key configs

| image | cat | pixel H0 (no prediction) | Med/Raw/None | Med/Raw/Act | Med/Coupled/Act | Med/Raw/ActX | Med/Coupled/ActX | png-rs | best config |
|---|---|---:|---:|---:|---:|---:|---:|---|
| outdoor_0 | outdoor | 7.062 | 4.421 | 4.286 | 2.428 | 3.568 | 2.338 | 3.605 | Med/SubG/ActX |
| outdoor_1 | outdoor | 7.542 | 2.895 | 2.616 | 1.670 | 2.186 | 1.613 | 2.372 | Med/SubG/ActX |
| indoor_0 | indoor | 6.864 | 2.653 | 2.164 | 1.162 | 1.676 | 1.110 | 1.971 | Med/SubG/ActX |
| indoor_1 | indoor | 7.693 | 3.676 | 3.451 | 1.973 | 2.798 | 1.896 | 2.972 | Med/SubG/ActX |
| portrait_0 | portrait | 7.373 | 2.851 | 2.693 | 1.463 | 2.019 | 1.402 | 2.239 | Med/SubG/ActX |
| pet_0 | pet | 7.135 | 3.578 | 3.478 | 2.341 | 2.986 | 2.237 | 3.254 | Med/SubG/ActX |
| pet_1 | pet | 7.397 | 3.958 | 3.914 | 2.530 | 3.343 | 2.425 | 3.388 | Med/SubG/ActX |
| pet2_0 | pet | 7.548 | 4.067 | 3.894 | 2.605 | 3.363 | 2.504 | 3.762 | Med/SubG/ActX |
| pet2_1 | pet | 6.788 | 3.113 | 2.801 | 1.653 | 2.214 | 1.589 | 2.502 | Med/SubG/ActX |
| food_0 | food | 7.577 | 4.932 | 4.664 | 3.705 | 4.420 | 3.678 | 5.076 | Med/SubG/ActX |
| food_1 | food | 7.265 | 4.035 | 3.722 | 2.409 | 3.143 | 2.337 | 3.688 | Med/SubG/ActX |
| foliage_0 | foliage | 7.127 | 5.295 | 4.939 | 3.439 | 4.508 | 3.377 | 5.128 | Med/SubG/ActX |
| foliage_1 | foliage | 7.493 | 3.735 | 3.515 | 2.072 | 2.814 | 1.997 | 3.146 | Med/SubG/ActX |
| night_0 | night | 6.554 | 5.470 | 5.148 | 5.070 | 5.106 | 5.052 | 5.824 | Med/Coupled/ActX |
| night_1 | night | 5.669 | 3.555 | 3.514 | 2.000 | 2.839 | 1.918 | 2.661 | Med/SubG/ActX |
| blurry_0 | blurry | 7.265 | 3.604 | 3.578 | 3.159 | 3.407 | 3.122 | 3.953 | Med/SubG/ActX |
| blurry_1 | blurry | 5.448 | 2.396 | 2.331 | 1.622 | 2.017 | 1.605 | 2.443 | Med/SubG/ActX |
| text_0 | text | 5.754 | 3.093 | 2.837 | 1.866 | 2.307 | 1.805 | 2.661 | Med/SubG/ActX |
| text_1 | text | 7.247 | 4.933 | 4.804 | 3.193 | 4.324 | 3.088 | 4.365 | Med/SubG/ActX |
| document_0 | document | 6.864 | 4.029 | 3.875 | 1.554 | 2.822 | 1.517 | 2.435 | Med/SubG/ActX |
| document_1 | document | 4.750 | 2.794 | 2.530 | 1.240 | 1.825 | 1.206 | 2.026 | Med/SubG/ActX |
| screen_0 | screen | 1.732 | 0.694 | 0.564 | 0.361 | 0.471 | 0.328 | 0.438 | Med/Coupled/ActX |
| screen_1 | screen | 0.485 | 0.433 | 0.367 | 0.357 | 0.345 | 0.335 | 0.292 | Med/Coupled/ActX |

## Noise / lighting: MED residuals by brightness of plane 0 (G or Y), JPEG photos

| brightness bin | share of px | mean abs resid | H0 bits/sample |
|---|---:|---:|---:|
| 0-63 | 0.222 | 3.863 | 4.030 |
| 64-127 | 0.297 | 4.164 | 4.179 |
| 128-191 | 0.319 | 3.281 | 3.746 |
| 192-255 | 0.162 | 2.035 | 2.855 |

Per-image dark-bin (0-63) vs mid-bin (64-191) H0, images with >=2% dark pixels:

| image | dark share | dark H0 | mid H0 |
|---|---:|---:|---:|
| outdoor_0 | 0.113 | 4.322 | 4.407 |
| outdoor_1 | 0.088 | 4.340 | 3.700 |
| indoor_0 | 0.185 | 3.566 | 3.662 |
| indoor_1 | 0.283 | 3.394 | 3.938 |
| portrait_0 | 0.273 | 2.587 | 2.956 |
| pet_0 | 0.339 | 3.880 | 3.279 |
| pet_1 | 0.168 | 3.580 | 3.994 |
| pet2_0 | 0.180 | 3.998 | 4.335 |
| food_0 | 0.328 | 4.615 | 5.084 |
| food_1 | 0.085 | 4.935 | 3.829 |
| foliage_0 | 0.339 | 4.817 | 5.424 |
| foliage_1 | 0.080 | 3.879 | 3.743 |
| night_0 | 0.789 | 4.938 | 6.789 |
| night_1 | 0.149 | 3.263 | 3.558 |
| blurry_0 | 0.328 | 3.407 | 3.548 |
| blurry_1 | 0.095 | 4.108 | 3.937 |
| text_0 | 0.326 | 3.294 | 3.904 |
| text_1 | 0.303 | 4.182 | 5.136 |
| document_1 | 0.072 | 4.005 | 2.428 |
| screen_0 | 0.046 | 2.857 | 2.607 |

## Directional structure: per-8x8-block best fixed predictor (plane 0), Elias-gamma cost proxy

Oracle chooses the cheapest of W,N,Avg,Grad,MED per block, charged log2(5)=2.32 bits/block side info. A proxy upper-bound screen, not a coded result.

| image | cat | win W | win N | win Avg | win Grad | win MED | MED proxy bps | oracle proxy bps | oracle gain |
|---|---|---:|---:|---:|---:|---:|---:|---:|---:|
| outdoor_0 | outdoor | 0.12 | 0.04 | 0.57 | 0.02 | 0.26 | 5.295 | 5.139 | -2.9% |
| outdoor_1 | outdoor | 0.18 | 0.06 | 0.07 | 0.36 | 0.33 | 3.448 | 3.378 | -2.0% |
| indoor_0 | indoor | 0.25 | 0.12 | 0.16 | 0.08 | 0.39 | 2.782 | 2.732 | -1.8% |
| indoor_1 | indoor | 0.12 | 0.06 | 0.21 | 0.11 | 0.50 | 4.067 | 4.011 | -1.4% |
| portrait_0 | portrait | 0.01 | 0.02 | 0.10 | 0.53 | 0.35 | 3.090 | 2.933 | -5.1% |
| pet_0 | pet | 0.13 | 0.01 | 0.30 | 0.15 | 0.42 | 4.079 | 3.994 | -2.1% |
| pet_1 | pet | 0.05 | 0.15 | 0.63 | 0.03 | 0.15 | 4.810 | 4.588 | -4.6% |
| pet2_0 | pet | 0.09 | 0.12 | 0.23 | 0.17 | 0.39 | 4.729 | 4.617 | -2.4% |
| pet2_1 | pet | 0.12 | 0.13 | 0.16 | 0.15 | 0.43 | 3.231 | 3.189 | -1.3% |
| food_0 | food | 0.07 | 0.03 | 0.29 | 0.19 | 0.42 | 5.825 | 5.708 | -2.0% |
| food_1 | food | 0.02 | 0.02 | 0.24 | 0.28 | 0.44 | 4.419 | 4.344 | -1.7% |
| foliage_0 | foliage | 0.08 | 0.06 | 0.27 | 0.18 | 0.41 | 6.435 | 6.302 | -2.1% |
| foliage_1 | foliage | 0.06 | 0.05 | 0.25 | 0.17 | 0.47 | 4.088 | 4.023 | -1.6% |
| night_0 | night | 0.17 | 0.15 | 0.36 | 0.03 | 0.29 | 6.534 | 6.361 | -2.7% |
| night_1 | night | 0.09 | 0.05 | 0.73 | 0.00 | 0.14 | 4.178 | 3.987 | -4.6% |
| blurry_0 | blurry | 0.04 | 0.06 | 0.44 | 0.07 | 0.39 | 4.094 | 4.015 | -1.9% |
| blurry_1 | blurry | 0.01 | 0.01 | 0.08 | 0.49 | 0.42 | 3.710 | 3.601 | -2.9% |
| text_0 | text | 0.13 | 0.09 | 0.37 | 0.03 | 0.39 | 3.225 | 3.159 | -2.0% |
| text_1 | text | 0.01 | 0.03 | 0.17 | 0.30 | 0.49 | 6.100 | 6.017 | -1.4% |
| document_0 | document | 0.10 | 0.09 | 0.46 | 0.04 | 0.31 | 4.668 | 4.533 | -2.9% |
| document_1 | document | 0.14 | 0.12 | 0.22 | 0.08 | 0.44 | 2.920 | 2.861 | -2.0% |
| screen_0 | screen | 0.79 | 0.13 | 0.00 | 0.03 | 0.05 | 1.800 | 1.704 | -5.3% |
| screen_1 | screen | 0.86 | 0.08 | 0.00 | 0.03 | 0.03 | 1.645 | 1.625 | -1.2% |

## Learning cost: real bytes vs static entropy (config `Med/Coupled/ActX`)

Sum over images: coded 19359514 B vs Hctx 19246427 B (coded/Hctx = 1.0059). Static Hctx ignores model-table cost, so the gap is the adaptive-learning overhead, not a gain.

