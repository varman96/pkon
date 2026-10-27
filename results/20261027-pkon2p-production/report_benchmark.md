# PKON2P (codec 2) production benchmark: 50 images, complete `.pkon2` files

Same decoded pixels, 1024-px centre crops (crop sha256 checked against the frozen bench50 manifest), `pkonfile::encode`/`decode` entry points, every size = 24-byte header + payload. Baseline = the **pre-change production code** (git HEAD 7833411, built separately, codec 1); new = working tree, codec 2. Held-out TEST images (15) are evaluated here for the first time in this investigation; DEV/VAL were used by REL1/REL2.

## Integrity of the comparison

- pre-change build's file sizes equal the frozen bench50 `pkon2-file` sizes on all 50 images: **True**
- working-tree codec 1 sizes equal the frozen baseline on all 50 images (existing format output unchanged): **True**
- every image, every run (6 runs): exact pixel round trip from file bytes, header spec/CRC/length verified, re-encode deterministic: **True**; header codec byte as expected: **True**

## Aggregate

| set | images | baseline bytes | PKON2P bytes | change | ratio before | ratio after | images smaller |
|---|---|---|---|---|---|---|---|
| ALL | 50 | 38914941 | 38282733 | -1.625% | 4.0687 | 4.1359 | 50/50 |
| DEV | 25 | 18435945 | 18115653 | -1.737% | 4.2658 | 4.3412 | 25/25 |
| VAL | 10 | 8437245 | 8329405 | -1.278% | 3.7284 | 3.7767 | 10/10 |
| TEST (held out) | 15 | 12041751 | 11837675 | -1.695% | 4.0056 | 4.0747 | 15/15 |

Mean per-image ratio 4.8366 -> 4.9273; median 4.2645 -> 4.3444.

Per-image saving: min +0.510% (path_n0), median +1.670%, max +3.746% (bird_n0). Images that got larger: none.

## Per image

| image | split | baseline B | PKON2P B | saving | ratio after | enc ms old -> new | dec ms old -> new |
|---|---|---|---|---|---|---|---|
| path_n0 | VAL | 1719726 | 1710962 | +0.510% | 1.839 | 804 -> 863 | 869 -> 1116 |
| outdoor_1 | VAL | 498289 | 495025 | +0.655% | 6.355 | 207 -> 276 | 212 -> 306 |
| snow_n0 | DEV | 1239618 | 1230807 | +0.711% | 2.556 | 604 -> 795 | 677 -> 811 |
| waterfall_n0 | DEV | 1040807 | 1031227 | +0.920% | 3.050 | 507 -> 421 | 321 -> 420 |
| bicycle_n0 | TEST | 1107010 | 1096143 | +0.982% | 2.870 | 586 -> 712 | 618 -> 705 |
| peak_n0 | DEV | 1219834 | 1207636 | +1.000% | 2.605 | 359 -> 882 | 349 -> 923 |
| coast_n0 | DEV | 449720 | 444975 | +1.055% | 7.069 | 390 -> 484 | 414 -> 548 |
| indoor_1 | DEV | 607405 | 600800 | +1.087% | 5.236 | 239 -> 425 | 235 -> 450 |
| blurry_0 | TEST | 1076424 | 1064089 | +1.146% | 2.956 | 539 -> 755 | 573 -> 788 |
| stairs_n0 | TEST | 1109421 | 1096140 | +1.197% | 2.870 | 559 -> 735 | 594 -> 781 |
| restaurant_n0 | VAL | 712598 | 703703 | +1.248% | 4.470 | 262 -> 749 | 250 -> 664 |
| lighthouse_n0 | DEV | 491252 | 484986 | +1.276% | 6.486 | 452 -> 563 | 379 -> 521 |
| boat_n0 | VAL | 1017568 | 1004572 | +1.277% | 3.131 | 535 -> 758 | 510 -> 794 |
| street_n0 | DEV | 924782 | 912364 | +1.343% | 3.448 | 494 -> 645 | 513 -> 679 |
| night_0 | DEV | 1943649 | 1917507 | +1.345% | 1.641 | 736 -> 1098 | 803 -> 996 |
| food_0 | VAL | 1283285 | 1265561 | +1.381% | 2.486 | 500 -> 761 | 538 -> 784 |
| beach_n0 | VAL | 567989 | 560047 | +1.398% | 5.617 | 334 -> 570 | 447 -> 598 |
| building_n0 | DEV | 698352 | 688393 | +1.426% | 4.570 | 478 -> 485 | 462 -> 578 |
| blurry_1 | TEST | 687310 | 677268 | +1.461% | 6.193 | 563 -> 737 | 494 -> 695 |
| foliage_0 | TEST | 1208203 | 1190491 | +1.466% | 2.642 | 658 -> 566 | 622 -> 641 |
| document_0 | DEV | 531813 | 523638 | +1.537% | 6.007 | 317 -> 468 | 279 -> 519 |
| station_n0 | TEST | 544009 | 535534 | +1.558% | 5.874 | 282 -> 582 | 351 -> 565 |
| woman_n0 | VAL | 341159 | 335672 | +1.608% | 9.371 | 341 -> 300 | 304 -> 300 |
| foliage_1 | TEST | 630064 | 619630 | +1.656% | 5.077 | 382 -> 716 | 377 -> 597 |
| desert_n0 | DEV | 589935 | 580105 | +1.666% | 5.423 | 426 -> 566 | 420 -> 627 |
| sunset_n0 | TEST | 912421 | 897153 | +1.673% | 3.506 | 572 -> 730 | 564 -> 671 |
| flower_n0 | VAL | 771175 | 758220 | +1.680% | 4.149 | 482 -> 601 | 490 -> 705 |
| macro_n0 | DEV | 818302 | 804406 | +1.698% | 3.911 | 496 -> 382 | 492 -> 423 |
| church_n0 | TEST | 731117 | 718552 | +1.719% | 4.378 | 473 -> 459 | 498 -> 544 |
| text_1 | DEV | 1057473 | 1039180 | +1.730% | 3.027 | 469 -> 785 | 461 -> 806 |
| pet_0 | TEST | 744800 | 731467 | +1.790% | 4.301 | 325 -> 472 | 345 -> 562 |
| skyline_n0 | TEST | 333939 | 327725 | +1.861% | 9.599 | 338 -> 451 | 300 -> 493 |
| clouds_n0 | DEV | 457070 | 448447 | +1.887% | 7.015 | 384 -> 476 | 348 -> 445 |
| outdoor_0 | VAL | 781152 | 765924 | +1.949% | 4.107 | 252 -> 342 | 253 -> 343 |
| portrait_0 | DEV | 405580 | 397667 | +1.951% | 7.910 | 196 -> 384 | 194 -> 446 |
| food_1 | VAL | 744304 | 729719 | +1.960% | 4.311 | 418 -> 328 | 371 -> 378 |
| lake_n0 | DEV | 854491 | 837460 | +1.993% | 3.756 | 496 -> 386 | 451 -> 374 |
| text_0 | DEV | 577448 | 565493 | +2.070% | 5.563 | 246 -> 471 | 272 -> 556 |
| pet2_1 | TEST | 492984 | 482753 | +2.075% | 6.516 | 328 -> 438 | 290 -> 472 |
| indoor_0 | DEV | 354417 | 346721 | +2.171% | 9.073 | 179 -> 430 | 176 -> 413 |
| graffiti_n0 | TEST | 850605 | 831930 | +2.195% | 3.781 | 361 -> 673 | 367 -> 723 |
| document_1 | DEV | 411166 | 402044 | +2.219% | 7.824 | 266 -> 495 | 270 -> 427 |
| bridge_n0 | DEV | 503531 | 492319 | +2.227% | 6.390 | 358 -> 497 | 395 -> 465 |
| pet2_0 | TEST | 798916 | 781117 | +2.228% | 4.027 | 460 -> 546 | 409 -> 572 |
| forest_n0 | DEV | 802555 | 782934 | +2.445% | 4.018 | 568 -> 720 | 489 -> 623 |
| night_1 | DEV | 636406 | 617913 | +2.906% | 5.091 | 325 -> 473 | 380 -> 486 |
| child_n0 | DEV | 419342 | 406096 | +3.159% | 7.746 | 356 -> 473 | 330 -> 435 |
| pet_1 | TEST | 814528 | 787683 | +3.296% | 3.994 | 423 -> 457 | 400 -> 516 |
| river_n0 | DEV | 1054942 | 1019443 | +3.365% | 3.086 | 564 -> 734 | 627 -> 838 |
| bird_n0 | DEV | 346055 | 333092 | +3.746% | 9.444 | 326 -> 507 | 340 -> 457 |

## Previously difficult images (fur / foliage / text / texture)

| image | baseline B | PKON2P B | saving |
|---|---|---|---|
| foliage_0 | 1208203 | 1190491 | +1.466% |
| foliage_1 | 630064 | 619630 | +1.656% |
| forest_n0 | 802555 | 782934 | +2.445% |
| text_0 | 577448 | 565493 | +2.070% |
| text_1 | 1057473 | 1039180 | +1.730% |
| document_0 | 531813 | 523638 | +1.537% |
| document_1 | 411166 | 402044 | +2.219% |
| pet_0 | 744800 | 731467 | +1.790% |
| pet_1 | 814528 | 787683 | +3.296% |
| pet2_0 | 798916 | 781117 | +2.228% |
| pet2_1 | 492984 | 482753 | +2.075% |
| river_n0 | 1054942 | 1019443 | +3.365% |
| bird_n0 | 346055 | 333092 | +3.746% |
| graffiti_n0 | 850605 | 831930 | +2.195% |
| night_0 | 1943649 | 1917507 | +1.345% |

## Time and memory (single thread, best of 2 repetitions x 2 passes per image, sequential on one machine)

| build | encode total ms | decode total ms | enc vs old | dec vs old | peak working set (best of 2 runs) |
|---|---|---|---|---|---|
| pre-change production (codec 1) | 21216 | 21124 | x1.000 | x1.000 | 63.5 MB |
| working tree, codec 1 | 22385 | 22443 | x1.055 | x1.062 | 63.1 MB |
| working tree, codec 2 (PKON2P) | 28624 | 29580 | x1.349 | x1.400 | 63.2 MB |

Throughput (codec 2): encode 5.53 MB/s, decode 5.35 MB/s of raw pixels (old: 7.46 / 7.50). Per-image time ratio spread new2/old (enc): min 0.77, max 2.86.
Model state: the mixer weight tables grow from [i32;4] to [i32;5] per (plane,node) and one more 96-context probability table per plane is allocated (3 planes x 96 x 27 x 4 B = 31 KB): negligible against the decoded image buffers.
