# GDIR: gradient-adaptive directional G-plane predictor, one controlled trial

Criteria frozen before the run (sha256 5ae853a9cc52854a0ebccd049c93da69dd6416b18c6de64682ba7c3fa8d098b7). 35 DEV+VAL images, TEST never loaded. Sizes = complete file (24-byte header + payload); added metadata/framing overhead: **0 bytes** (everything is derived from decoded pixels; header differs only in the codec byte). Baseline `P` = production codec 2 (byte-identical to `pkonfile::encode` on every image). Timings single thread, best of 2.

**Correctness gates: PASS** (exact pixels + CRC from the file bytes on all 35 x 2 arms, enc/dec trace+state hashes equal, repeatable decode, identical re-encode, no over-read, P == production).

| group | images | baseline B | candidate B | change | improved | enc x | dec x |
|---|---|---|---|---|---|---|---|
| ALL | 35 | 26445058 | 26897062 | +1.709% | 0/35 | 1.037 | 1.162 |
| LOSE | 7 | 7334656 | 7385185 | +0.689% | 0/7 | 1.024 | 1.139 |
| CONTROL | 7 | 6982996 | 7077365 | +1.351% | 0/7 | 1.014 | 1.137 |
| OTHER | 21 | 12127406 | 12434512 | +2.532% | 0/21 | 1.055 | 1.188 |

## Per image

| image | set | group | baseline B | candidate B | change | enc ms base -> cand | dec ms base -> cand |
|---|---|---|---|---|---|---|---|
| text_0 | DEV | OTHER | 565493 | 581512 | +2.833% | 768 -> 862 | 729 -> 599 |
| clouds_n0 | DEV | OTHER | 448447 | 468046 | +4.370% | 517 -> 500 | 503 -> 477 |
| night_1 | DEV | OTHER | 617913 | 628210 | +1.666% | 546 -> 560 | 488 -> 537 |
| child_n0 | DEV | OTHER | 406096 | 428920 | +5.620% | 387 -> 446 | 371 -> 436 |
| forest_n0 | DEV | LOSE | 782934 | 785747 | +0.359% | 559 -> 633 | 642 -> 589 |
| bridge_n0 | DEV | OTHER | 492319 | 509270 | +3.443% | 427 -> 486 | 408 -> 470 |
| lake_n0 | DEV | CONTROL | 837460 | 851687 | +1.699% | 568 -> 628 | 554 -> 613 |
| street_n0 | DEV | LOSE | 912364 | 917899 | +0.607% | 602 -> 684 | 586 -> 665 |
| waterfall_n0 | DEV | CONTROL | 1031227 | 1043677 | +1.207% | 653 -> 725 | 623 -> 680 |
| document_0 | DEV | OTHER | 523638 | 530852 | +1.378% | 439 -> 504 | 431 -> 484 |
| river_n0 | DEV | LOSE | 1019443 | 1025455 | +0.590% | 648 -> 714 | 631 -> 705 |
| coast_n0 | DEV | OTHER | 444975 | 452781 | +1.754% | 418 -> 469 | 380 -> 441 |
| macro_n0 | DEV | OTHER | 804406 | 822778 | +2.284% | 535 -> 604 | 515 -> 605 |
| building_n0 | DEV | OTHER | 688393 | 709119 | +3.011% | 516 -> 565 | 483 -> 557 |
| night_0 | DEV | LOSE | 1917507 | 1929902 | +0.646% | 1016 -> 1110 | 1069 -> 1034 |
| indoor_1 | DEV | OTHER | 600800 | 613028 | +2.035% | 464 -> 586 | 470 -> 521 |
| peak_n0 | DEV | OTHER | 1207636 | 1220619 | +1.075% | 731 -> 796 | 709 -> 816 |
| bird_n0 | DEV | OTHER | 333092 | 354365 | +6.387% | 385 -> 431 | 360 -> 407 |
| snow_n0 | DEV | CONTROL | 1230807 | 1251144 | +1.652% | 723 -> 807 | 715 -> 781 |
| lighthouse_n0 | DEV | OTHER | 484986 | 497947 | +2.672% | 431 -> 489 | 411 -> 476 |
| indoor_0 | DEV | OTHER | 346721 | 354568 | +2.263% | 360 -> 425 | 347 -> 476 |
| desert_n0 | DEV | OTHER | 580105 | 599746 | +3.386% | 467 -> 651 | 447 -> 536 |
| document_1 | DEV | CONTROL | 402044 | 422626 | +5.119% | 385 -> 610 | 414 -> 513 |
| text_1 | DEV | LOSE | 1039180 | 1046516 | +0.706% | 1082 -> 1587 | 1431 -> 1531 |
| portrait_0 | DEV | LOSE | 397667 | 411864 | +3.570% | 870 -> 629 | 790 -> 927 |
| food_0 | VAL | LOSE | 1265561 | 1267802 | +0.177% | 1521 -> 1090 | 967 -> 1515 |
| food_1 | VAL | OTHER | 729719 | 743369 | +1.871% | 1130 -> 1522 | 770 -> 1365 |
| path_n0 | VAL | CONTROL | 1710962 | 1718127 | +0.419% | 2225 -> 1124 | 1203 -> 1192 |
| boat_n0 | VAL | CONTROL | 1004572 | 1019649 | +1.501% | 754 -> 740 | 706 -> 687 |
| flower_n0 | VAL | OTHER | 758220 | 764472 | +0.825% | 538 -> 616 | 523 -> 597 |
| beach_n0 | VAL | OTHER | 560047 | 574727 | +2.621% | 458 -> 589 | 449 -> 739 |
| woman_n0 | VAL | OTHER | 335672 | 347398 | +3.493% | 864 -> 508 | 501 -> 465 |
| outdoor_0 | VAL | CONTROL | 765924 | 770455 | +0.592% | 742 -> 1498 | 983 -> 1443 |
| outdoor_1 | VAL | OTHER | 495025 | 511292 | +3.286% | 995 -> 659 | 542 -> 975 |
| restaurant_n0 | VAL | OTHER | 703703 | 721493 | +2.528% | 1037 -> 836 | 807 -> 669 |

## Decision (criteria.md, predeclared)

### Candidate criteria (all required)
- bit-exact on every image: true
- aggregate (35) +1.709% (need <= -0.5%): false
- LOSE aggregate +0.689% (need <= -0.5%): false
- worst single image +6.387% (need <= +0.3%): false
- aggregate encode x1.037, decode x1.162 (need <= 1.15 each): false

### Reject conditions (any)
- reconstruction mismatch: false
- aggregate improvement < 0.2% (+1.709%): true
- LOSE group does not improve (+0.689%): true
- worst single image > +1.0% (+6.387%): true
- encode or decode time > 1.25x (x1.037 / x1.162): false

**Verdict: REJECT**
