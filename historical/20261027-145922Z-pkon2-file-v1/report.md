# `.pkon2` v1 verification on the 23-image corpus

Encode -> complete file -> decode from file bytes only (no external spec) -> byte comparison. "historical B" is the production X4 byte count in `results/20261027-x4-promotion-verify/report.md`, which is stream + 11 notional header bytes that were never serialized.

| image | layout | WxH | payload B | header B | file B | overhead B | overhead % of payload | payload == historical B-11 | metadata | pixels exact | deterministic | enc ms | dec ms |
|---|---|---|---:|---:|---:|---:|---:|---|---|---|---|---:|---:|
| outdoor_0 | rgb | 1024x1024 | 781128 | 24 | 781152 | 24 | 0.00307% | true | true | true | true | 275 | 263 |
| outdoor_1 | rgb | 1024x1024 | 498265 | 24 | 498289 | 24 | 0.00482% | true | true | true | true | 216 | 215 |
| indoor_0 | rgb | 1024x1024 | 354393 | 24 | 354417 | 24 | 0.00677% | true | true | true | true | 249 | 233 |
| indoor_1 | rgb | 1024x1024 | 607381 | 24 | 607405 | 24 | 0.00395% | true | true | true | true | 541 | 474 |
| portrait_0 | rgb | 1024x1024 | 405556 | 24 | 405580 | 24 | 0.00592% | true | true | true | true | 431 | 271 |
| pet_0 | rgb | 1024x1024 | 744776 | 24 | 744800 | 24 | 0.00322% | true | true | true | true | 541 | 486 |
| pet_1 | rgb | 1024x1024 | 814504 | 24 | 814528 | 24 | 0.00295% | true | true | true | true | 1022 | 797 |
| pet2_0 | rgb | 1024x1024 | 798892 | 24 | 798916 | 24 | 0.00300% | true | true | true | true | 840 | 764 |
| pet2_1 | rgb | 1024x1024 | 492960 | 24 | 492984 | 24 | 0.00487% | true | true | true | true | 452 | 466 |
| food_0 | rgb | 1024x1024 | 1283261 | 24 | 1283285 | 24 | 0.00187% | true | true | true | true | 620 | 620 |
| food_1 | rgb | 1024x1024 | 744280 | 24 | 744304 | 24 | 0.00322% | true | true | true | true | 369 | 352 |
| foliage_0 | rgb | 1024x1024 | 1208179 | 24 | 1208203 | 24 | 0.00199% | true | true | true | true | 600 | 602 |
| foliage_1 | rgb | 1024x1024 | 630040 | 24 | 630064 | 24 | 0.00381% | true | true | true | true | 327 | 381 |
| night_0 | rgb | 1024x1024 | 1943625 | 24 | 1943649 | 24 | 0.00123% | true | true | true | true | 765 | 856 |
| night_1 | rgb | 1024x1024 | 636382 | 24 | 636406 | 24 | 0.00377% | true | true | true | true | 358 | 315 |
| blurry_0 | rgb | 1024x1024 | 1076400 | 24 | 1076424 | 24 | 0.00223% | true | true | true | true | 437 | 529 |
| text_0 | rgb | 1024x1024 | 577424 | 24 | 577448 | 24 | 0.00416% | true | true | true | true | 353 | 295 |
| text_1 | rgb | 1024x1024 | 1057449 | 24 | 1057473 | 24 | 0.00227% | true | true | true | true | 527 | 483 |
| document_0 | rgb | 1024x1024 | 531789 | 24 | 531813 | 24 | 0.00451% | true | true | true | true | 318 | 327 |
| document_1 | rgb | 1024x1024 | 411142 | 24 | 411166 | 24 | 0.00584% | true | true | true | true | 255 | 279 |
| blurry_1 | rgba | 1024x1024 | 687286 | 24 | 687310 | 24 | 0.00349% | true | true | true | true | 439 | 401 |
| screen_0 | rgba | 1024x1024 | 141776 | 24 | 141800 | 24 | 0.01693% | true | true | true | true | 356 | 258 |
| screen_1 | rgba | 1024x1024 | 183113 | 24 | 183137 | 24 | 0.01311% | true | true | true | true | 319 | 308 |

Totals: payload 16610001 B, files 16610553 B, container overhead 552 B (0.00332% of payload).
All images exact, metadata recovered, payload unchanged, output deterministic: true
