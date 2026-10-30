# REL1: predictor disagreement (P) and cross-plane residual (X) as extra mixer inputs

Criteria frozen before the run (sha256 f7eec0082c88eaa2ad705f31a24b8e4e803eaa6aa047f3b93371da8e82420c22); split file sha256 90ea95aa3220276181b57be6f4217414010451af39308a1f0c9aa188534da3bb; DEV = 25 images, VAL = 10, test list never loaded. Sizes = stream + 11-byte header (no per-arm signalling; the fixed container header is identical for all arms).

**Correctness gates: PASS** (every arm x every image: exact pixels incl. interleaved compare to the canonical crop; control stream == `pkonfile::encode_payload` and size == `measure_x`; encoder/decoder context+symbol trace hash and final model-state hash equal; re-encode identical; no decoder over-read). Unit tests (edge sizes, constant/checkerboard/noise/ramp, Gray+RGB, truncation/corruption, causality of P, feature ranges) are in `src/pkonrel.rs`.

## Aggregate actual encoded size

| arm | DEV bytes | S(DEV) % | improves DEV | VAL bytes | S(VAL) % | improves VAL | enc ms (VAL sum, 1 thread) | dec ms | extra inputs |
|---|---|---|---|---|---|---|---|---|---|
| control | 18435620 | +0.000 | 0/25 | 8437115 | +0.000 | 0/10 | 7383 | 6784 | 0 |
| dummy | 18414588 | +0.114 | 25/25 | 8426488 | +0.126 | 10/10 | 7423 | 7073 | 1 |
| P | 18115328 | +1.737 | 25/25 | 8329275 | +1.278 | 10/10 | 9886 | 10090 | 1 |
| X | 18345763 | +0.487 | 25/25 | 8422194 | +0.177 | 10/10 | 8423 | 7369 | 1 |
| P+X | 18027614 | +2.213 | 25/25 | 8313112 | +1.470 | 10/10 | 9522 | 8622 | 2 |
| PxX | 18006052 | +2.330 | 25/25 | 8310021 | +1.506 | 10/10 | 8004 | 8483 | 1 |
| dummy2 | 18411975 | +0.128 | 25/25 | 8425607 | +0.136 | 10/10 | 7757 | 7868 | 2 |
| dummyJ | 18421534 | +0.076 | 25/25 | 8429427 | +0.091 | 10/10 | 6076 | 5896 | 1 |

Time ratios vs control (VAL, best of 2, single thread): dummy enc x1.01 dec x1.04; P enc x1.34 dec x1.49; X enc x1.14 dec x1.09; P+X enc x1.29 dec x1.27; PxX enc x1.08 dec x1.25; dummy2 enc x1.05 dec x1.16; dummyJ enc x0.82 dec x0.87

## Entanglement

E = S(PxX) - S(P+X): DEV +0.117 pp, VAL +0.037 pp. Capacity-adjusted E_cap = [S(PxX)-S(dummyJ)] - [S(P+X)-S(dummy2)]: DEV +0.169 pp, VAL +0.082 pp. (dummyJ = 1 meaningless input with the joint's 2401 classes; dummy2 = 2 meaningless inputs of 49.) Per-image E is in `per_image.csv` (`e_joint_minus_sep_pp`).

## Secondary: textured vs smooth (DEV; not decisive)

| arm | textured S % | smooth S % |
|---|---|---|
| dummy | +0.112 | +0.115 |
| P | +2.444 | +1.572 |
| X | +0.220 | +0.550 |
| P+X | +2.675 | +2.105 |
| PxX | +2.851 | +2.208 |
| dummy2 | +0.121 | +0.130 |
| dummyJ | +0.074 | +0.077 |

## Cross-entropy by decision type (DEV, % change in bits vs control, positive = fewer bits)

| arm | zero | ge2 | class | mantissa | sign | total |
|---|---|---|---|---|---|---|
| dummy | +0.175 | +0.139 | +0.219 | +0.036 | +0.007 | +0.114 |
| P | +0.445 | +0.485 | +0.447 | +0.065 | +7.295 | +1.737 |
| X | +0.321 | +0.239 | +0.335 | +0.049 | +1.495 | +0.487 |
| P+X | +0.785 | +0.744 | +0.791 | +0.120 | +8.674 | +2.213 |
| PxX | +0.814 | +0.972 | +0.809 | +0.269 | +8.860 | +2.330 |
| dummy2 | +0.201 | +0.149 | +0.241 | +0.043 | +0.011 | +0.128 |
| dummyJ | +0.152 | +0.082 | +0.128 | +0.007 | -0.001 | +0.076 |

Sanity: max |cross-entropy/8 - stream bytes| over all runs = 0.0011%.

## Static information carried by the features (bits per pixel of the plane; DEV and VAL)

Estimator: plug-in conditional entropy with Miller-Madow correction, computed in-sample from the exact features/contexts the coder used, on the images' final pass (static distribution per plane over the whole image set; no adaptation, no learning cost). F = feature; S1 = existing sign context #2 (W,N,NE residual signs, 27 states); Ab = existing magnitude context A/b (96 states); zero = (residual == 0). It measures how much the feature tells about the outcome *beyond one existing context*; it does NOT measure what the other inputs already capture jointly, is optimistic for the 2401-class PxX (finite-sample), and is not an achievable size bound. The last two columns are the coder's realised cross-entropy savings of the matching arm (P->P, X->X, PxX->PxX), per pixel, for comparison.

| set | plane | feature | I(sign;F) | I(sign;F\|S1) | I(zero;F\|Ab) | realised sign saving (arm CE) | realised zero+ge2 saving (arm CE) |
|---|---|---|---|---|---|---|---|
| DEV | 0 | P | 0.03920 | 0.03942 | 0.00509 | 0.03605 | 0.00271 |
| DEV | 1 | P | 0.02910 | 0.02715 | 0.01186 | 0.02422 | 0.00533 |
| DEV | 1 | X | 0.00976 | 0.00994 | 0.00189 | 0.00049 | 0.00335 |
| DEV | 1 | PxX | 0.03875 | 0.03629 | 0.01504 | 0.02524 | 0.00567 |
| DEV | 2 | P | 0.02828 | 0.02580 | 0.01360 | 0.02138 | 0.00345 |
| DEV | 2 | X | 0.04232 | 0.04562 | 0.00453 | 0.01624 | 0.00389 |
| DEV | 2 | PxX | 0.07400 | 0.07189 | 0.02049 | 0.03788 | 0.01347 |
| VAL | 0 | P | 0.03291 | 0.04042 | 0.00315 | 0.03049 | 0.00071 |
| VAL | 1 | P | 0.03050 | 0.02443 | 0.01395 | 0.01995 | 0.00389 |
| VAL | 1 | X | 0.00991 | 0.01000 | 0.00118 | 0.00019 | 0.00453 |
| VAL | 1 | PxX | 0.04139 | 0.03430 | 0.01636 | 0.02097 | 0.00396 |
| VAL | 2 | P | 0.03555 | 0.02705 | 0.01732 | 0.02108 | 0.00315 |
| VAL | 2 | X | 0.02958 | 0.03487 | 0.00365 | 0.00339 | 0.00183 |
| VAL | 2 | PxX | 0.07168 | 0.06425 | 0.02304 | 0.02751 | 0.00816 |

Context usage (DEV):

| feature | plane | distinct classes used | entropy of class usage (bits) | max possible (bits) |
|---|---|---|---|---|
| P | 0 | 49 | 3.903 | 5.615 |
| P | 1 | 49 | 2.170 | 5.615 |
| X | 1 | 7 | 2.697 | 5.615 |
| P | 2 | 49 | 2.278 | 5.615 |
| X | 2 | 49 | 4.074 | 5.615 |
| dummy (hash, uniform by construction) | all | 49 | ~5.615 | 5.615 |

## Adaptation of the extra inputs (mean final mixer weight, DEV images; weight 1.0 = 65536)

| arm | extra # | plane | node-0 weight | sign weight |
|---|---|---|---|---|
| dummy | 0 | 0 | -0.137 | -0.067 |
| dummy | 0 | 1 | -0.147 | -0.151 |
| dummy | 0 | 2 | -0.056 | -0.170 |
| P | 0 | 0 | 0.196 | 0.836 |
| P | 0 | 1 | 0.145 | 0.899 |
| P | 0 | 2 | 0.113 | 0.942 |
| X | 0 | 0 | 0.000 | 0.000 |
| X | 0 | 1 | -0.284 | 0.283 |
| X | 0 | 2 | -0.103 | 0.668 |
| P+X | 0 | 0 | 0.196 | 0.836 |
| P+X | 1 | 0 | 0.000 | 0.000 |
| P+X | 0 | 1 | 0.221 | 0.901 |
| P+X | 1 | 1 | -0.346 | 0.302 |
| P+X | 0 | 2 | 0.108 | 0.943 |
| P+X | 1 | 2 | -0.083 | 0.713 |
| PxX | 0 | 0 | 0.196 | 0.836 |
| PxX | 0 | 1 | 0.160 | 0.889 |
| PxX | 0 | 2 | 0.399 | 0.916 |
| dummy2 | 0 | 0 | -0.073 | -0.055 |
| dummy2 | 1 | 0 | -0.071 | -0.061 |
| dummy2 | 0 | 1 | -0.063 | -0.130 |
| dummy2 | 1 | 1 | -0.064 | -0.138 |
| dummy2 | 0 | 2 | -0.022 | -0.145 |
| dummy2 | 1 | 2 | -0.023 | -0.143 |
| dummyJ | 0 | 0 | -0.123 | -0.004 |
| dummyJ | 0 | 1 | -0.117 | -0.006 |
| dummyJ | 0 | 2 | -0.064 | -0.016 |

## Decision rules (criteria.md)

- S(Dummy,DEV) = +0.114 (confounded if >= +0.10): confounded = true
- P promising: S(P,DEV) +1.737 (>=0.5), improves 25/25 (>=20), S(P)>=2*max(S(X),0): true, VAL S +1.278 improves 10/10 -> **PASS**
- X promising: S(X,DEV) +0.487 (>=0.5), > S(P): false, VAL S +0.177 improves 10/10 -> **no**
- Joint promising: E(DEV) +0.117 (>=0.3), E_cap(DEV) +0.169 (>=0.15), E(VAL) +0.037 (>0), E_cap(VAL) +0.082 (>0) -> **no**
- Little practical signal: **no**

**Overall: CONFOUNDED (extra capacity alone helps)**
