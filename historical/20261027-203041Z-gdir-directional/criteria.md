# GDIR frozen specification and criteria (written before the predictor or runner exist; the runner refuses to start if this file's sha256 differs)

Candidate (single, no variants): `P+GDir` = production codec 2 (X4P) with plane 0 (G, SubG domain) predicted by `src/gdir.rs` at the exact point
where production applies MED. Everything else (residual/zigzag, contexts, mixer, coder, R-G/B-G, SubG, framing) unchanged; 0 side-information bytes;
header identical to codec 2 except codec byte 3 (research only). Control = unchanged `P` (must equal `pkonfile::encode` byte-for-byte).

## Predictor (integer only, raster order, plane 0)
Neighbours (reconstructed): W(x-1,y) N(x,y-1) NW(x-1,y-1) NE(x+1,y-1) WW(x-2,y) NN(x,y-2) NNE(x+1,y-2) NNW(x-1,y-2) NWW(x-2,y-1).
A pixel is *available* iff inside the image (strictly causal in raster order; no substitution values).
Candidates c in {W,N,NW,NE} (index order W=0,N=1,NW=2,NE=3), P_c = value of that neighbour (already in 0..255, so the clamp is the identity);
candidate available iff its own neighbour is available. If fewer than 2 candidates are available -> MED (production `predict(Pred::Med, neighbors())`).

Directional gradient g_c = sum of three absolute differences; a term is skipped (contributes 0) if either of its two pixels is unavailable:
  g_W  (horizontal) = |W-WW|  + |N-NW|   + |NE-N|
  g_N  (vertical)   = |W-NW|  + |N-NN|   + |NE-NNE|
  g_NW (diagonal \) = |W-NWW| + |N-NNW|  + |NE-NN|
  g_NE (diagonal /) = |W-N|   + |N-NNE|  + |NW-NN|

History estimate E_c (u32, Q4 fixed point = 16 x mean abs error). Initialised to 64 (=4.0) for every c at every plane start (encoder and decoder).
After the pixel v is reconstructed, for every candidate that was available at that pixel (unavailable ones unchanged):
  E_c <- E_c + (((|v - P_c|) << 4) - E_c) >> 3      (arithmetic shift on i32; rate 1/8)

Score (lower is better):  S_c = 16*g_c + E_c + 1      (>= 1, so no division by zero; max ~16.3k so no overflow)
Select the two available candidates with the smallest S (ties -> lower index); a = best, b = second.
Weights: w_c = floor(2^24 / S_c)   (lower score -> larger weight; w_c >= 1000 > 0)
den = w_a + w_b (never 0; if it were, MED is used).
Prediction = floor((2*(w_a*P_a + w_b*P_b) + den) / (2*den))   (round-half-up, u64 arithmetic; a convex combination of 0..255 values, hence in 0..255).

## Baseline / data
Same 35 DEV+VAL images as the GBlend report (results/learn1-splits.json, re-derived, crop hashes checked); TEST never loaded. Groups from
results/20261027-diag1-residual-maps/groups.json: LOSE (7 JPEG XL wins), CONTROL (7 matched), OTHER (21).
Size = complete file (24-byte header + payload). Timing: single thread, best of 2, encode and decode in the same harness.

## Correctness gates (any failure => no performance interpretation)
exact pixels decoded from the FILE bytes + CRC32 on all 35; encoder/decoder trace and model-state hashes equal; re-encode identical; decoder consumed <= stream+4;
P file == production encode; focused unit tests pass (decoder-view causality with poisoned future pixels, boundaries, state order).

## Decision (applied mechanically, no tuning, no variants)
CANDIDATE iff all hold:
 - bit-exact reconstruction on every image
 - aggregate bytes over all 35 images smaller by >= 0.5%
 - aggregate bytes over the 7 LOSE images smaller by >= 0.5%
 - no individual image grows by more than 0.3%
 - aggregate encode time and aggregate decode time each <= 1.15x control
REJECT iff any:
 - any reconstruction mismatch
 - aggregate improvement over 35 images < 0.2%
 - LOSE group does not improve in aggregate
 - any individual image grows by more than 1.0%
 - aggregate encode or decode time > 1.25x control
Otherwise INCONCLUSIVE (no parameter tuning, no alternative predictors in this run).
