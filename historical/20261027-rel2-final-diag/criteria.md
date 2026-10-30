# REL2 final diagnostic: criteria (written before the runner exists; the runner refuses to start if this file's sha256 differs)

Purpose: REL1 said P saves 1.737% DEV / 1.278% VAL (25/25 and 10/10 images) but a meaningless dummy input saved 0.114% DEV, above the frozen
0.10% confound limit, so REL1's overall verdict was CONFOUNDED. REL1's frozen criteria.md (sha256 f7eec008...) is unchanged and its verdict stands as
recorded. This diagnostic asks one question: how much of P's gain survives once a pure adaptive-bias input is already present?

## Data and method (identical to REL1)
Same 25 DEV + 10 VAL images, same split file (results/learn1-splits.json, sha256 90ea95aa...), same crop-hash checks, same size accounting
(stream + 11-byte header; container header is identical for all arms and excluded). The 15 test images are never loaded. P's definition and
quantisation are REL1's, untouched (qd cuts, GAP/Grad minus MED, 49 classes). No tuning of anything.

## Arms (pkonrel::DIAG_ARMS)
- control: production X4 (must be byte-identical to pkonfile::encode_payload on every image).
- const: X4 + ONE extra mixer input with ONE class (context always 0): a per-(plane, decision node) adaptive probability, count-based rate,
  fed to the mixer through the same stretch/weight path as every other input (initial weight 0). Applies to magnitude AND sign decisions.
- dummy: REL1's hash input, 49 classes (re-run, must reproduce REL1 bytes exactly).
- P: REL1's P alone (re-run, must reproduce REL1 bytes exactly).
- P+const: X4 + P + const (two extras).

## What each comparison can and cannot establish
- const vs control = S(const): the gain of an extra mixer input that carries NO information about the pixel's neighbourhood, only the running
  per-node bit frequency. It is the pure "adaptive bias / extra mixer input" effect. Limitation: it is not the same capacity as a 49-class input
  (one class vs 49 classes: fewer parameters, faster learning, different adaptation), so S(const) is a floor-type estimate of the bias effect, not an exact
  subtraction term for P.
- dummy vs control = S(dummy): REL1's number, re-measured. A 49-class input whose class is independent of the image: it matches P's capacity and
  context-dilution shape but also injects noise (random class => slow, diluted counters). It is both an upper-capacity and a noisier control.
- P+const vs const = the additional value of P given a bias input is already present. This is the primary attribution comparison:
  it removes the constant-bias effect by construction (both arms have the bias input) and leaves what the 49 predictor-disagreement classes add.
  Limitation: it does not remove capacity/dilution effects of having 49 classes, nor interactions between the bias input and P in the mixer.
- P vs P+const: whether P's gain was relying on bias (if P+const ~ P, the bias input is redundant once P is present).
- None of these alone proves P's gain is "predictor-disagreement information". The conclusion is judged by the joint pattern below, plus REL1's
  existing evidence (sign cross-entropy -7.3%, sign-weight ~0.9 vs dummy ~ -0.1, static I(sign;F|S1) > 0 on all planes, textured gains > dummy gains).

## Decision rule (applied mechanically; ONLY frozen REL1 numbers are reused)
Let Sc(A) = 100*(sum bytes(const) - sum bytes(A)) / sum bytes(const), improves = strictly smaller than const on that image.
P is SUPPORTED as useful information iff ALL of:
 1. Correctness gates pass (below).
 2. Sc(P+const, DEV) >= 0.5 and improves on >= 80% of DEV images (REL1's P thresholds, now measured against the bias-controlled baseline).
 3. VAL: Sc(P+const, VAL) >= 0.25 and improves on >= 7 of 10 VAL images (REL1's VAL-support thresholds).
 4. S(P,DEV) >= 2 * max(S(const,DEV), S(dummy,DEV)) (REL1's 2x rule, with the larger of the two controls in place of X).
P is CONTRADICTED iff Sc(P+const, DEV) < 0.2 or it improves < 50% of DEV images. Otherwise AMBIGUOUS (the report states precisely why).
The REL1 limit "S(dummy,DEV) >= 0.10 => confounded" is NOT relaxed or replaced: it stays true (0.114) and REL1's recorded verdict stays CONFOUNDED.
This diagnostic does not overturn it; it decides whether the remaining evidence justifies production integration despite it.

## Correctness gates (any failure: stop, keep artifacts, no performance interpretation)
G1 exact pixel reconstruction, every arm, every image (also interleaved vs the canonical crop). G2 control stream byte-identical to
pkonfile::encode_payload and size == measure_x. G3 encoder/decoder agreement on the per-pixel context/symbol trace hash and final model-state hash.
G4 re-encode identical (repeatability). G5 no decoder over-read. G6 control, dummy and P bytes on every image equal REL1's per_image.csv values
(cross-run repeatability / harness unchanged). Unit tests in src/pkonrel.rs cover edge sizes and all DIAG_ARMS.

## Reported but non-decisive
Per-image and aggregate bytes and %; per-decision-type cross-entropy (zero, ge2, class, mantissa, sign); single-thread VAL timings (best of 2).
