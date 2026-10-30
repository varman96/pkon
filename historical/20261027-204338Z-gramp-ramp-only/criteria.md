# GRAMP frozen specification and criteria (written before the predictor or runner exist; the runner refuses to start if this file's sha256 differs)

Candidate (single, no variants): `P+GRamp` = production codec 2 (X4P) where plane 0 (G) is predicted by a planar predictor ONLY at samples
accepted by a ramp detector; MED everywhere else (including all of planes 1,2). Same prediction point as production MED, same residual mapping,
contexts, mixer, coder, framing; 0 side-information bytes; header identical to codec 2 except codec byte 3 (research only).
Control = unchanged `P`, must equal `pkonfile::encode` byte-for-byte.

## Detector (integer, causal; all inputs are already reconstructed G samples)
Pixels: W(x-1,y) WW(x-2,y) N(x,y-1) NW(x-1,y-1) NE(x+1,y-1) NWW(x-2,y-1) NN(x,y-2) NNW(x-1,y-2) NNE(x+1,y-2).
Boundary: the detector is evaluated only when x>=2, y>=2 and x+1<w (all nine pixels inside the image). Otherwise MED. No substitution values.
Horizontal gradient measurements (4):  h1=W-WW  h2=N-NW  h3=NE-N  h4=NW-NWW
Vertical   gradient measurements (4):  v1=W-NW  v2=N-NN  v3=NE-NNE v4=NW-NNW
Accept (ramp) iff ALL hold (constants fixed now, never tuned):
  (a) consistency: max(h1..h4)-min(h1..h4) <= 2  AND  max(v1..v4)-min(v1..v4) <= 2      (rejects texture / noise)
  (b) no edge:     |h_i| <= 8 and |v_i| <= 8 for all i                                  (rejects abrupt edges)
  (c) evidence of a slope: |Sh|+|Sv| >= 4 where Sh=h1+h2+h3+h4, Sv=v1+v2+v3+v4          (flat regions stay MED)
Anything else -> MED.

## Planar predictor (i32 only, no overflow possible: |terms| < 2^12)
Estimate the gradients as the means of the measurements (Hhat=Sh/4, Vhat=Sv/4) and average the four plane extrapolations
 W+Hhat, N+Vhat, NW+Hhat+Vhat, NE-Hhat+Vhat:
  pred = clamp( (4*(W+N+NW+NE) + Sh + 3*Sv + 8) >> 4 , 0, 255 )     (arithmetic shift = floor, +8 gives round half up)

## Diagnostics (research instrumentation, NOT coded-bit savings)
Per image, over the original G plane (decoder-identical state): ramp-classified samples and %, samples where planar != MED ("actually predicted differently"),
and over ramp samples: zero-residual fraction and mean |residual| under MED vs under planar. Per-region coded bits are not measurable without modifying the coder
and are therefore not reported; actual bytes (whole file) are the only compression evidence.

## Data
Same 35 DEV+VAL images as the GBlend/GDIR reports (results/learn1-splits.json, re-derived, crop hashes checked); TEST never loaded. Groups from
results/20261027-diag1-residual-maps/groups.json: LOSE (7 JPEG XL wins), CONTROL (7 matched), OTHER (21).
Size = complete file (24-byte header + payload). Timing: single thread, best of 2, encode and decode in the same harness.

## Correctness gates (any failure => no performance interpretation)
exact pixels from the FILE bytes + CRC32 on all 35; encoder/decoder trace and model-state hashes equal; re-encode identical; decoder consumed <= stream+4;
P file == production encode; unit tests pass (decoder-view causality with poisoned future pixels, identical ramp decisions, boundaries).

## Decision (applied mechanically, no tuning, no variants)
CANDIDATE iff: exact on all images; aggregate (35) bytes smaller >= 0.5%; LOSE (7) aggregate smaller >= 0.5%; no image grows > 0.3%; aggregate encode and decode time each <= 1.15x.
REJECT iff any: reconstruction mismatch; aggregate improvement < 0.2%; LOSE group does not improve in aggregate; any image grows > 1.0%; aggregate encode or decode time > 1.25x.
Otherwise INCONCLUSIVE. No parameter sweeps, no alternative predictors.
