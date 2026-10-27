# REL5 frozen criteria (written before the runner or the new arms exist; the runner refuses to start if this file's sha256 differs)

## Question
Does X, supplied ONLY to the sign decision (mixer node SIGN = 26), on top of production codec 2 (X4P), carry enough genuine sign-prediction information
to justify shipping a sign-only version?

## Inspected facts (src/pkonrel.rs, results/*rel1..rel4/criteria.md, read before writing this file)
- In the research coder every decision is a mixer `bit()` call; extras are appended after the base inputs (`nin = nb + ex.n`); the sign decision is node
  `SIGN` (26) with 3 base inputs; all other nodes (0..25: zero, ge2, class, mantissa) are non-sign. Mixer weights and bit-model tables are per
  (plane, node), so a sign-only extra cannot touch any non-sign probability.
- P = `Extra::P` (all decisions). X = `x_feature` = q7(e[k-1])*7 + q7(e[k-2]), unchanged from REL1/REL4; plane 0 has no X.
- REL4 criteria sha256 c311a5b312ebe5355cda710367437f29b28151a48e1bca0b2569068ba90612b6 (frozen thresholds below are copied from it, unaltered).

## Arms (research harness `pkonrel`, `XS_ARMS`)
- `P`: production codec 2, MUST be byte-identical to `pkonfile::encode(img)` on every image.
- `P+Xsign`: [P, XSign]. XSign = the X feature (49 classes, bit-model slot 5, initial mixer weight 0) mixed in ONLY at node SIGN, ONLY on planes k>=1.
  Zero, ge2, class, mantissa and tail decisions use the original P-only inputs.
- `P+dummysign`: [P, DummySign{salt 1, card 49}]: a hash of (plane, x, y, salt) mod 49, same slot, same cardinality, same initial weight, same decision routing
  (ONLY node SIGN, ONLY planes k>=1), same update schedule. It differs from X only in information content.
Model counts: both extra arms add the same table (49 classes x 27 nodes x planes allocated; reachable bit models = 2 planes x 49 classes at the sign node).
The runner asserts equal allocation and equal reachable-model count.

## Data and accounting
25 DEV + 10 VAL images of results/learn1-splits.json; split derivation and every crop hash re-verified; 15 TEST images never loaded or inspected.
Size = complete file: 24-byte header + payload. P uses the production header (codec 2); the two extra arms use the same header with codec byte 3 (not accepted by production
`parse_header`), payload_len and CRC fields filled, and are decoded back from the file bytes.

## Definitions
S(A,set) = 100 * (sum bytes(P) - sum bytes(A)) / sum bytes(P), aggregate, positive = smaller. "improves" = strictly fewer bytes on that image.

## Correctness gates (any failure: verdict INVALID, no performance interpretation, nothing shipped)
G1 exact pixels from the file bytes + CRC32 match, every arm, every image. G2 P file == production `encode` output. G3 encoder/decoder context+symbol
trace hash and final model-state hash equal. G4 re-encode identical (stream and trace). G5 decoder consumed bytes <= stream length + 4 (no over-read).
G6 for every arm and image, file bytes == 24 + payload bytes counted independently from the encoder's stream, and P bytes equal REL4's per-image P file
bytes. G7 non-sign isolation: for P+Xsign and P+dummysign, the cross-entropy of every non-sign category (zero, ge2, class, mantissa) per plane is exactly
equal (f64 ==) to P's. G8 equal model counts/allocation between the X-sign and dummy-sign arms (see above).

## Decision rules (REL4's thresholds, unaltered; applied mechanically)
PASS iff G1-G8 pass and ALL of:
 1. S(P+Xsign, DEV) >= 0.5 and improves >= 80% of DEV images (>= 20/25).
 2. S(P+Xsign, VAL) >= 0.25 and improves >= 7 of 10 VAL images.
 3. S(P+Xsign, DEV) >= 2 * max(S(P+dummysign, DEV), 0).
FAIL iff gates pass and (S(P+Xsign, DEV) < 0.2 or S(P+Xsign, VAL) <= 0).
AMBIGUOUS otherwise (gates pass, no FAIL condition, but at least one of 1-3 not met); the report names each failed criterion, its margin, and the smallest
defensible next decision. No threshold is changed after the run.
Informational only (not criteria): X vs dummy per-image counts, sign/non-sign split, time (single thread), memory.

## Triggered by PASS (and only by PASS)
Integrate X-sign into production codec 2 with NO format change (same codec id 2, same header, X is derived from already-decoded residuals, zero side
information); codec 1 and non-sign decisions untouched. Then focused verification: round trips + CRC, determinism, enc/dec agreement, codec 1
compatibility and golden fixtures, malformed/truncation checks on the changed path, byte-for-byte equality of production output with the REL5 P+Xsign
stream on DEV+VAL, complete-file DEV/VAL benchmark reproducing the experiment. Post-integration results are implementation verification only, not a
second adoption vote. Write `production_integration.md`. TEST is not touched. No commit.
If this is FAIL or AMBIGUOUS nothing is integrated.
