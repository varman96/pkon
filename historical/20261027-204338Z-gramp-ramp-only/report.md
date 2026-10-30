# GRAMP: ramp-only planar G-plane predictor, one controlled trial

Criteria frozen before the run (sha256 ea7005e1f3688fd177f27da40358eef4b353812ce42b7814e73fd46871efa4d8). 35 DEV+VAL images, TEST never loaded. Sizes = complete file (24-byte header + payload); added metadata/framing overhead: **0 bytes** (everything is derived from decoded pixels; header differs only in the codec byte). Baseline `P` = production codec 2 (byte-identical to `pkonfile::encode` on every image). Timings single thread, best of 2.

**Correctness gates: PASS** (exact pixels + CRC from the file bytes on all 35 x 2 arms, enc/dec trace+state hashes equal, repeatable decode, identical re-encode, no over-read, P == production).

| group | images | baseline B | candidate B | change | improved | enc x | dec x |
|---|---|---|---|---|---|---|---|
| ALL | 35 | 26445058 | 26582432 | +0.519% | 0/35 | 1.060 | 1.037 |
| LOSE | 7 | 7334656 | 7343607 | +0.122% | 0/7 | 1.085 | 1.137 |
| CONTROL | 7 | 6982996 | 7013341 | +0.435% | 0/7 | 0.832 | 0.964 |
| OTHER | 21 | 12127406 | 12225484 | +0.809% | 0/21 | 1.168 | 1.030 |

## Per image

| image | set | group | baseline B | candidate B | change | enc ms base -> cand | dec ms base -> cand | ramp samples (%) |
|---|---|---|---|---|---|---|---|---|
| text_0 | DEV | OTHER | 565493 | 570193 | +0.831% | 514 -> 541 | 506 -> 748 | 142064 (13.55%) |
| clouds_n0 | DEV | OTHER | 448447 | 453436 | +1.113% | 406 -> 416 | 384 -> 406 | 160868 (15.34%) |
| night_1 | DEV | OTHER | 617913 | 618552 | +0.103% | 485 -> 848 | 461 -> 496 | 23428 (2.23%) |
| child_n0 | DEV | OTHER | 406096 | 417127 | +2.716% | 387 -> 405 | 379 -> 389 | 459252 (43.80%) |
| forest_n0 | DEV | LOSE | 782934 | 784145 | +0.155% | 553 -> 629 | 535 -> 553 | 123673 (11.79%) |
| bridge_n0 | DEV | OTHER | 492319 | 497163 | +0.984% | 421 -> 510 | 435 -> 429 | 149193 (14.23%) |
| lake_n0 | DEV | CONTROL | 837460 | 841860 | +0.525% | 589 -> 596 | 554 -> 570 | 79681 (7.60%) |
| street_n0 | DEV | LOSE | 912364 | 912843 | +0.053% | 593 -> 609 | 700 -> 599 | 38304 (3.65%) |
| waterfall_n0 | DEV | CONTROL | 1031227 | 1034824 | +0.349% | 640 -> 656 | 621 -> 674 | 106150 (10.12%) |
| document_0 | DEV | OTHER | 523638 | 523902 | +0.050% | 430 -> 456 | 425 -> 493 | 11833 (1.13%) |
| river_n0 | DEV | LOSE | 1019443 | 1019609 | +0.016% | 649 -> 670 | 638 -> 649 | 10373 (0.99%) |
| coast_n0 | DEV | OTHER | 444975 | 450274 | +1.191% | 393 -> 480 | 416 -> 403 | 70558 (6.73%) |
| macro_n0 | DEV | OTHER | 804406 | 808440 | +0.501% | 533 -> 614 | 520 -> 553 | 170848 (16.29%) |
| building_n0 | DEV | OTHER | 688393 | 695049 | +0.967% | 514 -> 543 | 497 -> 519 | 183017 (17.45%) |
| night_0 | DEV | LOSE | 1917507 | 1917647 | +0.007% | 1017 -> 1033 | 1013 -> 1003 | 6673 (0.64%) |
| indoor_1 | DEV | OTHER | 600800 | 603868 | +0.511% | 515 -> 912 | 924 -> 683 | 93324 (8.90%) |
| peak_n0 | DEV | OTHER | 1207636 | 1208084 | +0.037% | 1063 -> 1109 | 1107 -> 1250 | 22817 (2.18%) |
| bird_n0 | DEV | OTHER | 333092 | 351404 | +5.498% | 622 -> 1001 | 1066 -> 666 | 392153 (37.40%) |
| snow_n0 | DEV | CONTROL | 1230807 | 1240176 | +0.761% | 1703 -> 1233 | 1347 -> 1156 | 114392 (10.91%) |
| lighthouse_n0 | DEV | OTHER | 484986 | 488768 | +0.780% | 675 -> 782 | 622 -> 1324 | 133154 (12.70%) |
| indoor_0 | DEV | OTHER | 346721 | 353936 | +2.081% | 879 -> 846 | 918 -> 579 | 118700 (11.32%) |
| desert_n0 | DEV | OTHER | 580105 | 580774 | +0.115% | 861 -> 1021 | 1026 -> 723 | 38554 (3.68%) |
| document_1 | DEV | CONTROL | 402044 | 409921 | +1.959% | 564 -> 558 | 569 -> 546 | 171316 (16.34%) |
| text_1 | DEV | LOSE | 1039180 | 1039309 | +0.012% | 1041 -> 1488 | 1025 -> 1785 | 14873 (1.42%) |
| portrait_0 | DEV | LOSE | 397667 | 403868 | +1.559% | 896 -> 798 | 577 -> 586 | 273963 (26.13%) |
| food_0 | VAL | LOSE | 1265561 | 1266186 | +0.049% | 1142 -> 1166 | 1051 -> 1122 | 29894 (2.85%) |
| food_1 | VAL | OTHER | 729719 | 731241 | +0.209% | 729 -> 1045 | 701 -> 799 | 96235 (9.18%) |
| path_n0 | VAL | CONTROL | 1710962 | 1710971 | +0.001% | 1404 -> 809 | 1394 -> 1114 | 817 (0.08%) |
| boat_n0 | VAL | CONTROL | 1004572 | 1009560 | +0.497% | 1248 -> 1011 | 976 -> 1007 | 126913 (12.10%) |
| flower_n0 | VAL | OTHER | 758220 | 760022 | +0.238% | 839 -> 1055 | 802 -> 1286 | 85722 (8.18%) |
| beach_n0 | VAL | OTHER | 560047 | 560237 | +0.034% | 853 -> 684 | 715 -> 750 | 11611 (1.11%) |
| woman_n0 | VAL | OTHER | 335672 | 344338 | +2.582% | 517 -> 608 | 539 -> 535 | 134219 (12.80%) |
| outdoor_0 | VAL | CONTROL | 765924 | 766029 | +0.014% | 828 -> 943 | 743 -> 914 | 7775 (0.74%) |
| outdoor_1 | VAL | OTHER | 495025 | 501584 | +1.325% | 821 -> 800 | 1007 -> 742 | 105486 (10.06%) |
| restaurant_n0 | VAL | OTHER | 703703 | 707092 | +0.482% | 832 -> 844 | 734 -> 839 | 202734 (19.33%) |

## Ramp diagnostics (residual statistics only: these are NOT coded-bit savings)

| group | G samples | ramp samples | ramp % | planar != MED | zero-resid MED | zero-resid planar | mean abs MED | mean abs planar |
|---|---|---|---|---|---|---|---|---|
| ALL | 36700160 | 3910567 | 10.655% | 1939952 (49.6% of ramp) | 0.4776 | 0.4542 | 0.6722 | 0.6668 |
| LOSE | 7340032 | 497753 | 6.781% | 257846 (51.8% of ramp) | 0.4258 | 0.4535 | 0.7753 | 0.6947 |
| CONTROL | 7340032 | 607044 | 8.270% | 280441 (46.2% of ramp) | 0.5331 | 0.4479 | 0.5589 | 0.6671 |
| OTHER | 22020096 | 2805770 | 12.742% | 1401665 (50.0% of ramp) | 0.4748 | 0.4557 | 0.6784 | 0.6618 |

Per-region coded bits are not reported: the coder attributes bits per node category, not per pixel, so an ramp-region cost cannot be measured without changing the coder. Whole-file bytes are the only compression evidence.

## Decision (criteria.md, predeclared)

### Candidate criteria (all required)
- bit-exact on every image: true
- aggregate (35) +0.519% (need <= -0.5%): false
- LOSE aggregate +0.122% (need <= -0.5%): false
- worst single image +5.498% (need <= +0.3%): false
- aggregate encode x1.060, decode x1.037 (need <= 1.15 each): true

### Reject conditions (any)
- reconstruction mismatch: false
- aggregate improvement < 0.2% (+0.519%): true
- LOSE group does not improve (+0.122%): true
- worst single image > +1.0% (+5.498%): true
- encode or decode time > 1.25x (x1.060 / x1.037): false

**Verdict: REJECT**
