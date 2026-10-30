# REL4 frozen criteria (written before the runner exists; the runner refuses to start if this file's sha256 differs)

## Question
Does adding X (cross-plane residual) as one extra logistic-mixer input, ON TOP OF production codec 2 (X4P = X4 + P), reduce complete encoded size?

## Verified definitions (read from src/pkonrel.rs, not from memory)
- Control `P` = production codec 2. MUST be byte-identical to `pkonfile::encode_payload_codec(img, CODEC_X4P)` on every image.
- X = `x_feature`: q7(e_{k-1}(i))*7 + q7(e_{k-2}(i)), e_j = unzigzag residual of plane j at the SAME index i (planes coded G, R-G, B-G; plane 1 sees
  plane 0 only, second term = zero class; plane 0 has no X, fixed class 24). 49 classes, one extra mixer input for magnitude and sign decisions,
  initial weight 0. Exactly REL1's `X` as used in REL1's `P+X` arm. Nothing is retuned.
- Arm `P+X` = [Extra::P, Extra::X] (REL1 arm "P+X"). Arm `P+dummy` = [Extra::P, Extra::Dummy{salt 1, card 49}] (capacity control: a meaningless
  second extra input of X's shape). Only these three arms are run.

## Data and accounting
Same 25 DEV + 10 VAL images (results/learn1-splits.json, derivation and crop hashes re-verified). The 15 TEST images are never loaded.
Size = COMPLETE `.pkon2`-style file: 24-byte header + payload. For P this is checked equal to `pkonfile::encode(img).len()`. P+X / P+dummy files use the
same 24-byte header with codec byte 3 (research-only, not accepted by production `parse_header`), same payload_len/CRC fields, and are decoded back from
the file bytes (header fields, payload_len, CRC32 of decoded pixels all checked). No per-arm signalling beyond the codec byte, so 0 extra bytes.

## Definitions
S(A,set) = 100 * (sum bytes(P) - sum bytes(A)) / sum bytes(P), aggregate, positive = smaller. "improves" = strictly fewer bytes on that image.

## Correctness gates (any failure: stop, no performance interpretation)
G1 exact pixels from the file bytes, every arm and image. G2 P file == production `encode` output. G3 encoder/decoder context+symbol trace hash and final
model-state hash equal. G4 re-encode identical. G5 decoder never over-reads the payload. G6 P and P+X payload sizes equal REL1's per-image bytes
(results/20261027-rel1-pred-xplane/per_image.csv, minus its 11-byte accounting header).

## Decision rule (REL3/REL1 thresholds, applied mechanically, not relaxed)
ADOPT iff G1-G6 pass and all of:
 1. S(P+X, DEV) >= 0.5 and improves >= 80% of DEV images (>= 20/25).
 2. S(P+X, VAL) >= 0.25 and improves >= 7 of 10 VAL images.
 3. S(P+X, DEV) >= 2 * max(S(P+dummy, DEV), 0).
ABANDON iff S(P+X, DEV) < 0.2 or S(P+X, VAL) <= 0.
Otherwise MODIFY/AMBIGUOUS: the report must say which criterion failed and by how much, and whether the miss points to implementation difference
(G6 mismatch), redundant information (P+X gain over P+dummy vs. weights/CE), or insufficient evidence (sample size).
Reproduction comparison: incremental X gain on P from REL1 = S(P+X) - S(P) (in REL1's control-relative points) and, converted, relative to P.
Time and memory are recorded for information only.
