# REL3 frozen criteria (written before the code exists; the runner refuses to start if this file's sha256 differs)

## Question
Does choosing the logistic-mixer weight set by local activity reduce complete encoded size, beyond production codec 2 (X4P)?

## Hypothesis
Production mixes its inputs with ONE weight vector per (plane, decision node). The right trust in each input (A/b, B, C, D, P) differs between flat and
textured neighbourhoods (REL1: final weights differ strongly by plane and node). The activity bucket `act_bucket(activity(nb))` (8 values) is already a
causal quantity both sides compute. Supporting result: weight set selected by `act` gives a real gain over a single set. No new information source is added;
only how existing inputs are combined. The probability tables, contexts, binarisation, range coder and all initial weights are unchanged (each of the
8 sets starts from the same initial weights as the single set did).

## Arms (research harness `pkonrel`, `SEL_ARMS`)
- P: production codec 2. MUST be byte-identical to `pkonfile::encode_payload_codec(img, 2)` on every image (the control).
- P+selAct: same, mixer weight set chosen by act_bucket(activity) (8 sets) for every decision (magnitude and sign).
- P+selDummy: same, but the set is a hash of (plane, x, y) mod 8, carrying no image information. Isolates "more weight sets / dilution" from "informative selector".

## Data and accounting
Identical to REL1/REL2: same 25 DEV + 10 VAL images and split file, same crop-hash checks, size = stream + 11 B header (identical for all arms; no
signalling needed). The 15 test images are NOT loaded.

## Decision rule (REL1's own thresholds; applied mechanically)
S(A) = 100 * (bytes(P) - bytes(A)) / bytes(P), aggregate. SUPPORTED iff all of:
 1. Correctness gates pass: G1 exact pixels (every arm, image), G2 P == production codec-2 payload, G3 enc/dec context+symbol trace and final model-state
    hashes equal, G4 re-encode identical, G5 no decoder over-read, G6 P bytes equal REL1's per-image P bytes.
 2. S(selAct, DEV) >= 0.5 and improves >= 80% of DEV images.
 3. S(selAct, VAL) >= 0.25 and improves >= 7 of 10 VAL images.
 4. S(selAct, DEV) >= 2 * max(S(selDummy, DEV), 0).
CONTRADICTED iff S(selAct, DEV) < 0.2. Otherwise AMBIGUOUS (report says exactly why).
Runtime and memory are recorded for information only and are not a criterion.
