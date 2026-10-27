# GBLEND: adaptive G-plane predictor, one controlled trial

Criteria frozen before the run (sha256 67df6a76b79df05df609da28649812ab1dfb0af9f47a7bab174cd1536b4091e9). 35 DEV+VAL images, TEST never loaded. Sizes = complete file (24-byte header + payload); added metadata/framing overhead: **0 bytes** (everything is derived from decoded pixels; header differs only in the codec byte). Baseline `P` = production codec 2 (byte-identical to `pkonfile::encode` on every image). Timings single thread, best of 2.

**Correctness gates: PASS** (exact pixels + CRC from the file bytes on all 35 x 2 arms, enc/dec trace+state hashes equal, repeatable decode, identical re-encode, no over-read, P == production).

| group | images | baseline B | candidate B | change | improved | enc x | dec x |
|---|---|---|---|---|---|---|---|
| ALL | 35 | 26445058 | 26236304 | -0.789% | 28/35 | 1.117 | 1.109 |
| LOSE | 7 | 7334656 | 7246509 | -1.202% | 6/7 | 1.069 | 1.020 |
| CONTROL | 7 | 6982996 | 6955926 | -0.388% | 5/7 | 1.022 | 1.076 |
| OTHER | 21 | 12127406 | 12033869 | -0.771% | 17/21 | 1.177 | 1.159 |

## Per image

| image | set | group | baseline B | candidate B | change | enc ms base -> cand | dec ms base -> cand |
|---|---|---|---|---|---|---|---|
| text_0 | DEV | OTHER | 565493 | 571569 | +1.074% | 715 -> 910 | 674 -> 923 |
| clouds_n0 | DEV | OTHER | 448447 | 445758 | -0.600% | 685 -> 750 | 595 -> 691 |
| night_1 | DEV | OTHER | 617913 | 610212 | -1.246% | 784 -> 964 | 861 -> 862 |
| child_n0 | DEV | OTHER | 406096 | 405296 | -0.197% | 640 -> 815 | 514 -> 730 |
| forest_n0 | DEV | LOSE | 782934 | 770628 | -1.572% | 792 -> 634 | 488 -> 896 |
| bridge_n0 | DEV | OTHER | 492319 | 486442 | -1.194% | 670 -> 810 | 686 -> 788 |
| lake_n0 | DEV | CONTROL | 837460 | 822897 | -1.739% | 955 -> 981 | 990 -> 1011 |
| street_n0 | DEV | LOSE | 912364 | 904135 | -0.902% | 1043 -> 1115 | 1148 -> 1146 |
| waterfall_n0 | DEV | CONTROL | 1031227 | 1027816 | -0.331% | 1147 -> 1223 | 902 -> 1177 |
| document_0 | DEV | OTHER | 523638 | 522165 | -0.281% | 758 -> 816 | 728 -> 870 |
| river_n0 | DEV | LOSE | 1019443 | 1013837 | -0.550% | 1086 -> 1279 | 992 -> 723 |
| coast_n0 | DEV | OTHER | 444975 | 443987 | -0.222% | 850 -> 771 | 600 -> 747 |
| macro_n0 | DEV | OTHER | 804406 | 801651 | -0.342% | 848 -> 1055 | 918 -> 922 |
| building_n0 | DEV | OTHER | 688393 | 692635 | +0.616% | 876 -> 991 | 724 -> 1129 |
| night_0 | DEV | LOSE | 1917507 | 1918674 | +0.061% | 1752 -> 1780 | 1821 -> 1686 |
| indoor_1 | DEV | OTHER | 600800 | 597485 | -0.552% | 797 -> 876 | 785 -> 772 |
| peak_n0 | DEV | OTHER | 1207636 | 1198886 | -0.725% | 1300 -> 1407 | 1248 -> 1116 |
| bird_n0 | DEV | OTHER | 333092 | 327739 | -1.607% | 577 -> 791 | 570 -> 601 |
| snow_n0 | DEV | CONTROL | 1230807 | 1227428 | -0.275% | 1133 -> 1312 | 1189 -> 1260 |
| lighthouse_n0 | DEV | OTHER | 484986 | 468594 | -3.380% | 722 -> 854 | 631 -> 758 |
| indoor_0 | DEV | OTHER | 346721 | 347037 | +0.091% | 617 -> 723 | 589 -> 381 |
| desert_n0 | DEV | OTHER | 580105 | 573162 | -1.197% | 489 -> 651 | 505 -> 918 |
| document_1 | DEV | CONTROL | 402044 | 409270 | +1.797% | 602 -> 386 | 620 -> 368 |
| text_1 | DEV | LOSE | 1039180 | 1000752 | -3.698% | 578 -> 773 | 716 -> 720 |
| portrait_0 | DEV | LOSE | 397667 | 385255 | -3.121% | 416 -> 467 | 394 -> 452 |
| food_0 | VAL | LOSE | 1265561 | 1253228 | -0.975% | 788 -> 850 | 752 -> 815 |
| food_1 | VAL | OTHER | 729719 | 719664 | -1.378% | 535 -> 609 | 520 -> 802 |
| path_n0 | VAL | CONTROL | 1710962 | 1701993 | -0.524% | 1216 -> 1029 | 908 -> 972 |
| boat_n0 | VAL | CONTROL | 1004572 | 1006784 | +0.220% | 649 -> 724 | 635 -> 728 |
| flower_n0 | VAL | OTHER | 758220 | 756724 | -0.197% | 626 -> 1125 | 993 -> 1210 |
| beach_n0 | VAL | OTHER | 560047 | 548358 | -2.087% | 1101 -> 867 | 930 -> 911 |
| woman_n0 | VAL | OTHER | 335672 | 338873 | +0.954% | 388 -> 806 | 545 -> 644 |
| outdoor_0 | VAL | CONTROL | 765924 | 759738 | -0.808% | 910 -> 1101 | 730 -> 916 |
| outdoor_1 | VAL | OTHER | 495025 | 481799 | -2.672% | 809 -> 855 | 689 -> 815 |
| restaurant_n0 | VAL | OTHER | 703703 | 695833 | -1.118% | 834 -> 938 | 822 -> 947 |

## Decision (criteria.md)

1. exact reconstruction on 35/35: true
2. aggregate change -0.789% (need <= -0.5%): true
3. LOSE group aggregate -1.202% (need < 0): true
4. worst single image +1.797% (need <= +0.3%): false
5. aggregate encode x1.117, decode x1.109 (need <= 1.15): true
6. format compatibility: judged separately in the final report

**Verdict: FAIL**

## Notes

- Fail trigger actually hit: criterion 4 (worst image document_1 +1.797%, also woman_n0 +0.954%, building_n0 +0.616%, boat_n0 +0.220%, indoor_0 +0.091%, night_0 +0.061%) and the frozen "an image grows > 1.0%" FAIL clause. Seven images grew; 28/35 shrank. Criteria 1-3 and 5 were met; criterion 6 is moot after the FAIL.
- Reference, unchanged: the diagnostic's causal tile-above LS predictor (G plane, static cost model) gave LOSE forest +4.4, portrait +9.7, text_1 +8.1, river +3.5, food_0 +1.9, street +1.8, night_0 -3.2% G-plane cost (`results/20261027-diag1-residual-maps/causal_ls_G.csv`). Actual file-size change in this trial for the same images: forest -1.57, portrait -3.12, text_1 -3.70, river -0.55, food_0 -0.98, street -0.90, night_0 +0.06 (negative = smaller).
- Overhead: 0 bytes signalled. Production behaviour is unchanged: `pkonfile`/`pkon2x` untouched; arm `P` reproduced `pkonfile::encode` byte-for-byte on all 35 images. Research-only additions: `src/gblend.rs`, `Extra::GBlend` + `GB_ARMS` hooks in `src/pkonrel.rs` (additive, other arms unaffected), `src/bin/gblend.rs`.
- Implementation note for criterion 6 (moot): the 11x11 normal-equation solve used f64 add/sub/mul/div (deterministic IEEE, no FMA), per-pixel prediction integer-only.
- Per the protocol, no variants, no threshold changes, and no further work on this predictor.
