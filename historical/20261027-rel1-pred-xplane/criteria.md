# REL1 frozen criteria (written before the experiment code exists; the runner refuses to start if this file's sha256 differs from the one it records)

## Data
- Development (DEV) = the 25-image training pool of `results/learn1-splits.json`; validation (VAL) = its 10 validation images.
  The test list in that file is never loaded. Split file and every crop hash (`Canonical::content_hash` vs
  `results/20261027-155433Z-bench50/dataset_manifest.json`) are verified at start; any mismatch aborts.
- Pixels: the same 1024-px centre crops as every earlier report. Size of an arm on an image = range-coder stream + 11-byte header
  (same accounting as every earlier research report). No arm has per-file signalling; adopting an arm would need a new codec id inside the
  existing header, i.e. 0 extra bytes. The container's fixed 24-byte file header is identical for all arms and excluded.

## Definitions
- bytes(A,i) = complete coded bytes of arm A on image i. Set sums are over the images of that set (DEV: 25, VAL: 10).
- S(A, set) = 100 * (sum_i bytes(control,i) - sum_i bytes(A,i)) / sum_i bytes(control,i). Positive = smaller. Aggregate, byte-weighted.
- "improves on an image" = bytes(A,i) < bytes(control,i), strictly. Fraction = count / images in the set.
- E(set) = S(PxX joint, set) - S(P+X separate, set), in percentage points.
- Capacity-adjusted entanglement E_cap(set) = [S(PxX) - S(DummyJ)] - [S(P+X) - S(Dummy2)], where DummyJ is one meaningless input of the
  joint's cardinality (2401) and Dummy2 is two meaningless inputs of 49 (matching P+X separate). These two extra dummy shapes are not feature
  arms; they exist only because one dummy cannot match both capacity shapes, and the spec requires the capacity confound to be addressed.
- The primary dummy (`Dummy`) is one meaningless input of cardinality 49 (matches P and X individually).

## Correctness gates (any failure: stop, keep artifacts, no performance interpretation)
G1 exact pixel reconstruction, every arm, every image. G2 control stream byte-identical to production `pkonfile::encode_payload` (and
`measure_x` size) on every image. G3 encoder/decoder agreement on a running hash of every pixel's contexts and decoded symbols, and on the
final model state hash. G4 unit tests: tiny/odd sizes, constant, checkerboard 0/255, noise, ramp, gray + RGB; every arm. G5 truncated/corrupted
streams never panic and are flagged by decoder over-read where the stream is too short (research streams have no container, so
malformed-stream rejection is limited to the existing decoder guarantees). G6 repeat encode gives identical bytes.

## Decision rules (applied mechanically; not relaxed after seeing results)
Let VAL support for an arm mean: S(arm, VAL) >= 0.25 and improves on >= 7 of 10 VAL images.
- P promising: S(P,DEV) >= 0.5 AND improves on >= 80% of DEV images (>= 20/25) AND S(P,DEV) >= 2 * max(S(X,DEV), 0) AND VAL support.
- X promising: S(X,DEV) >= 0.5 AND S(X,DEV) > S(P,DEV) AND VAL support.
- Joint promising: E(DEV) >= 0.3 AND E_cap(DEV) >= 0.15 AND E(VAL) > 0 AND E_cap(VAL) > 0.
- Little practical signal: S(P,DEV) < 0.2 AND S(X,DEV) < 0.2 AND neither has VAL support.
- Confounded (overrides any "promising"): S(Dummy, DEV) >= +0.10 (extra capacity alone helps), or any gate fails.
- Ambiguous: everything else (e.g. dev passes but VAL does not, 0.2 <= S < 0.5, inconsistent sign DEV vs VAL); the report then names the
  smallest next diagnostic.
Secondary, non-decisive: TEXTURED = {forest_n0, text_1, river_n0, text_0} (DEV members of the earlier loss table / text) vs SMOOTH = the other DEV
images; per-plane and per-decision-type cross-entropy; the static information estimates (I(sign;F), I(sign;F|S1), I(zero;F|Ab); plug-in with
Miller-Madow correction, in-sample, per plane, bits per pixel). These estimates are diagnostics, not achievable bounds.

## Arm implementation fixed in advance
Extra inputs are added to the logistic mixer for BOTH the magnitude decisions and the sign decision; base inputs and their initial weights are
unchanged from X4 (1.25/n_base each); every extra input starts at weight 0 so each arm begins with the control's prediction. MIXLR and LIMIT
are X4's. P = qd(GAP-MED)*7 + qd(Grad-MED), qd cuts {<=-9,-8..-3,-2..-1,0,1..2,3..8,>=9}; X = q7(e_{k-1}(i))*7 + q7(e_{k-2}(i)) (k=1: second
term is the zero class; planes are G,R-G,B-G, so R sees G's residual, B sees R's and G's), q7 = the M7 quantiser; X is absent on plane 0.
These cuts are a-priori choices, not tuned.
