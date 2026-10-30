Baseline e0c7484 vs optimized f382c27, 3 runs each, medians in seconds, same driver/release profile.

| image | residual sha256 equal (6 runs) | payload bytes / sha equal | exact | stage1 | full encode | decode |
|---|:-:|:-:|:-:|---|---|---|
| leica_m8_house | True (406eec14978b) | True (6333715) | True | 14.7 -> 6.6 (2.24x) | 15.4 -> 7.5 (2.05x) | 19.9 -> 8.0 (2.49x) |
| nexus6_courtyard | True (a3044ed7b22c) | True (10006901) | True | 24.6 -> 10.0 (2.46x) | 25.3 -> 11.3 (2.25x) | 21.1 -> 10.5 (2.00x) |
| hasselblad_tallinn | True (854cf73ec790) | True (30161380) | True | 31.0 -> 10.0 (3.11x) | 33.0 -> 12.5 (2.64x) | 37.3 -> 12.4 (3.01x) |

Aggregate (sum of per-image medians): stage1_s 70.3 -> 26.5 = 2.65x; encode_s 73.7 -> 31.2 = 2.36x; decode_s 78.3 -> 31.0 = 2.53x

Raw per-run values:
- leica_m8_house stage1_s: base [14.7, 14.8, 14.7] opt [6.6, 10.1, 5.3]
- leica_m8_house encode_s: base [15.4, 15.5, 15.4] opt [7.5, 11.2, 5.9]
- leica_m8_house decode_s: base [21.6, 19.9, 15.6] opt [8.2, 8.0, 7.9]
- nexus6_courtyard stage1_s: base [25.4, 24.6, 17.1] opt [10.2, 10.0, 8.8]
- nexus6_courtyard encode_s: base [26.3, 25.3, 18.4] opt [12.3, 11.3, 10.3]
- nexus6_courtyard decode_s: base [20.8, 23.2, 21.1] opt [12.8, 8.0, 10.5]
- hasselblad_tallinn stage1_s: base [35.9, 31.0, 20.9] opt [10.0, 9.3, 10.0]
- hasselblad_tallinn encode_s: base [40.1, 33.0, 23.2] opt [12.5, 11.9, 12.5]
- hasselblad_tallinn decode_s: base [40.0, 23.3, 37.3] opt [14.0, 12.4, 10.7]
