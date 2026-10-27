# REL4: X on top of production codec 2

Criteria frozen before the run (sha256 c311a5b312ebe5355cda710367437f29b28151a48e1bca0b2569068ba90612b6). DEV 25 / VAL 10, test list never loaded. Sizes = complete file (24-byte header + payload). Baseline `P` = production codec 2.

**Correctness gates G1-G6: PASS** (exact pixels from the file bytes + CRC, P file == production `encode` output on every image, enc/dec trace+state hashes equal, re-encode identical, no over-read, P and P+X bytes equal REL1 per-image bytes + 13).

| arm | DEV bytes | S(DEV) % vs P | improves | VAL bytes | S(VAL) % | improves | enc ms (VAL, 1 thr) | dec ms |
|---|---|---|---|---|---|---|---|---|
| P | 18115653 | +0.000 | 0/25 | 8329405 | +0.000 | 0/10 | 5713 | 5464 |
| P+X | 18027939 | +0.484 | 25/25 | 8313242 | +0.194 | 10/10 | 6124 | 5708 |
| P+dummy | 18071862 | +0.242 | 25/25 | 8309049 | +0.244 | 10/10 | 6314 | 5989 |

## Cross-entropy by decision type (DEV, % fewer bits vs P)

| arm | zero | ge2 | class | mantissa | sign | total |
|---|---|---|---|---|---|---|
| P+X | +0.342 | +0.260 | +0.346 | +0.055 | +1.488 | +0.484 |
| P+dummy | +0.402 | +0.281 | +0.397 | +0.082 | +0.016 | +0.242 |

## Decision (criteria.md)

- (1) S(P+X,DEV) = +0.484 (>=0.5), improves 25/25 (>=80%): false
- (2) S(P+X,VAL) = +0.194 (>=0.25), improves 10/10 (>=7): false
- (3) S(P+X,DEV) >= 2 x max(S(P+dummy,DEV)=+0.242, 0): true

**Verdict: MODIFY / AMBIGUOUS (see failed criteria).**

## Analysis (post-run; the decision above was computed mechanically from the frozen criteria)

- Reproduction: P+X payloads are byte-for-byte equal to REL1's `P+X` on all 35 images (gate G6), so there is no implementation discrepancy; X's
  incremental gain over P is DEV +0.484% / VAL +0.194% of P's bytes (REL1 control-relative: +0.476 / +0.192 pp).
- Capacity control: a meaningless 49-class second input on top of P saves +0.242% DEV and +0.244% VAL. On VAL it matches or beats X
  (X +0.194%). X beats the dummy on only 11 of 35 images.
- Where X's gain lives: X's zero/ge2/class/mantissa cross-entropy gain (+0.34/+0.26/+0.35/+0.06%) is the same size as, or smaller than, the dummy's
  (+0.40/+0.28/+0.40/+0.08%), i.e. the magnitude side is mostly a capacity/bias effect. X's sign gain is +1.49% against +0.02% for the dummy; that is the only
  clearly informative part.
- Concentration: X's DEV total comes mainly from two images, night_0 (+1.61%) and macro_n0 (+2.21%), about 55% of the DEV bytes X saves. The median image
  gain is about +0.2%, the same as the dummy. All images improve (25/25, 10/10), but mostly by capacity-sized amounts.
- Cost: enc +7%, dec +4% (VAL, single thread, noise-level in this harness). Memory: one more 49-class context table, 3 planes x 49 x 27 nodes = 3,969 bit
  models (about 16 KB if 4 B each) plus the existing spare mixer-weight slot; no per-file signalling.
