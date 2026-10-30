# REL5: X on the sign decision only, on top of production codec 2

Criteria frozen before the run (sha256 a47ceb43de35c3260344d02f9e87de4ffc00f0540f73835dde42187c7ba8c0be). DEV 25 / VAL 10, test list never loaded. Sizes = complete file (24-byte header + payload). Baseline `P` = production codec 2.

**Correctness gates G1-G8: PASS** (exact pixels from the file bytes + CRC, P file == production `encode` output on every image, enc/dec trace+state hashes equal, re-encode identical, no over-read, P bytes equal REL4 per-image P bytes, file = 24 + payload, non-sign cross-entropy of both extra arms exactly equal to P, equal model counts).

| arm | DEV bytes | S(DEV) % vs P | improves | VAL bytes | S(VAL) % | improves | enc ms (VAL, 1 thr) | dec ms |
|---|---|---|---|---|---|---|---|---|
| P | 18115653 | +0.000 | 0/25 | 8329405 | +0.000 | 0/10 | 9555 | 8965 |
| P+Xsign | 18065057 | +0.279 | 25/25 | 8324535 | +0.058 | 10/10 | 9988 | 9076 |
| P+dummysign | 18115227 | +0.002 | 24/25 | 8329263 | +0.002 | 10/10 | 10126 | 9264 |

## Cross-entropy by decision type (DEV, % fewer bits vs P)

| arm | zero | ge2 | class | mantissa | sign | total |
|---|---|---|---|---|---|---|
| P+Xsign | +0.000 | +0.000 | +0.000 | +0.000 | +1.488 | +0.279 |
| P+dummysign | +0.000 | +0.000 | +0.000 | +0.000 | +0.012 | +0.002 |

## Sign vs non-sign decisions (bytes saved vs P, from exact coder cross-entropy)

| arm | set | sign bytes saved | non-sign bytes saved | sign share of P's bits |
|---|---|---|---|---|
| P+Xsign | DEV | 50593 | 0 | 18.77% |
| P+Xsign | VAL | 4869 | 0 | 18.55% |
| P+dummysign | DEV | 424 | 0 | 18.77% |
| P+dummysign | VAL | 141 | 0 | 18.55% |

P+Xsign beats P+dummysign on 24/25 DEV and 10/10 VAL images (informational). Per-image saving vs P, all 35: Xsign min +0.000% / median +0.056% / max +1.462%; dummysign min +0.000% / median +0.001% / max +0.019%.

Gates G7/G8 passed: non-sign cross-entropy of both extra arms is exactly equal to P's on every image and plane; extra-input model counts (allocated models, bytes, reachable sign-node models) Xsign (7938, 31752, 98) = dummysign (7938, 31752, 98). Encode/decode times above are single-thread best-of-2 on VAL; peak process memory was not measured (the added state is the allocated-model figure above plus no extra mixer storage).

## Decision (criteria.md)

- (1) S(P+Xsign,DEV) = +0.279 (>=0.5), improves 25/25 (>=80%): false
- (2) S(P+Xsign,VAL) = +0.058 (>=0.25), improves 10/10 (>=7): false
- (3) S(P+Xsign,DEV) >= 2 x max(S(P+dummysign,DEV)=+0.002, 0): true

**Verdict: AMBIGUOUS.**

## Interpretation (post-verdict; the verdict above is the frozen mechanical one)

- X carries real sign information: P+Xsign saves 1.488% of sign bits (identical to what X saved on sign decisions in REL4), the dummy saves 0.012%, and X-sign
  beats the dummy on 24/25 DEV and 10/10 VAL images. Non-sign decisions are bit-exactly unchanged (G7).
- It is small in file terms because sign decisions are only ~18.7% of coded bits: +0.279% DEV, +0.058% VAL, both below the frozen minima (0.5 / 0.25).
  Criterion 1 misses by 0.221 pp, criterion 2 by 0.192 pp; criterion 3 (dummy rule) passes. Every image improves (35/35) but median gain is +0.056%.
- Gain is concentrated: macro_n0 (+1.46%) and night_0 (+1.14%) dominate; the median image gains ~0.06%. VAL has no comparable image, hence +0.058%.
- REL4's extra ~0.2 pp over X-sign was a generic capacity/bias effect on magnitude decisions that a dummy also gets (+0.24%), not X information.
- Decision: **AMBIGUOUS -> not integrated.** Production, codec 1 and all earlier artifacts are untouched; no production_integration.md exists because PASS was not reached.
  The smallest defensible next decision is to stop pursuing X as a stand-alone feature (gain ceiling ~0.28% DEV even if the sign signal is perfect to use),
  and treat "extra sign-node input" as at most a component of a different, richer sign model. Lowering the thresholds to ship it would be a policy change, not
  something this run supports.
