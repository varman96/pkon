# REL3: mixer weight-set selection by activity (vs production codec 2)

Criteria frozen before the run (sha256 a91d271aa23ae048fd4670fb2ed7402ca690f9f2c65ad3f282e3efcae886044a). DEV 25 / VAL 10, test list never loaded. Sizes = stream + 11 B header. Baseline `P` = production codec 2.

**Correctness gates G1-G6: PASS** (exact pixels incl. interleaved compare, P == production codec-2 payload on every image, enc/dec trace+state hashes equal, re-encode identical, no over-read, P bytes equal REL1 per-image bytes).

| arm | DEV bytes | S(DEV) % vs P | improves | VAL bytes | S(VAL) % | improves | enc ms (VAL, 1 thr) | dec ms |
|---|---|---|---|---|---|---|---|---|
| P | 18115328 | +0.000 | 0/25 | 8329275 | +0.000 | 0/10 | 5832 | 5502 |
| P+selAct | 18066349 | +0.270 | 25/25 | 8307961 | +0.256 | 10/10 | 5835 | 5565 |
| P+selDummy | 18178518 | -0.349 | 0/25 | 8352193 | -0.275 | 0/10 | 7219 | 7243 |

## Cross-entropy by decision type (DEV, % fewer bits vs P)

| arm | zero | ge2 | class | mantissa | sign | total |
|---|---|---|---|---|---|---|
| P+selAct | +0.373 | +0.215 | +0.181 | +0.008 | +0.533 | +0.270 |
| P+selDummy | -0.347 | -0.333 | -0.379 | -0.199 | -0.506 | -0.349 |

## Decision (criteria.md)

- (2) S(selAct,DEV) = +0.270 (>=0.5), improves 25/25 (>=80%): false
- (3) S(selAct,VAL) = +0.256 (>=0.25), improves 10/10 (>=7): true
- (4) S(selAct,DEV) >= 2 x max(S(selDummy,DEV)=-0.349, 0): true

**Verdict: AMBIGUOUS.**
