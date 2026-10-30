# GBLEND frozen criteria (written before the runner exists; the runner refuses to start if this file's sha256 differs)

Candidate (single, no variants): `P+GBlend` = production codec 2 (X4P) with plane 0 (G) predicted by `src/gblend.rs`:
backward-adaptive inverse-squared-local-error blend (weights (1<<28)/(s^2+1), s = sum |err| at W,N,NW,NE) of MED and a strictly causal
10-tap+bias linear predictor whose coefficients are refitted by least squares, for every 128-px tile column, from the 128x128 tile directly above
at each tile-row start (y>=128; first tile row = MED). MED is used exactly when its local error sum is 0 and wherever no linear predictor exists.
R-G / B-G prediction, SubG, residual mapping, all contexts, mixer and coder are unchanged. No side information (0 bytes added); header
identical to codec 2 except codec byte 3 (research only).

Baseline: unchanged production PKON2P (arm `P`, must equal `pkonfile::encode` byte-for-byte on every image). Data: the 35 DEV+VAL images
(25+10, results/learn1-splits.json re-derived, crop hashes checked); the 15 TEST images are never loaded. Groups from
results/20261027-diag1-residual-maps/groups.json: LOSE (7 JPEG XL e9 wins), CONTROL (7 matched), OTHER (21).
Size = complete file (24-byte header + payload). Timing: single thread, best of 2, encode and decode of the same arm in the same harness.

Correctness gates (any failure => FAIL, no performance interpretation): exact pixels decoded from the FILE bytes and CRC32 match on all 35;
encoder/decoder trace and model-state hashes equal; re-encode identical; decoder consumed <= stream+4; P file == production encode.

Decision (applied mechanically, no tuning):
PASS iff all hold:
 1. exact reconstruction on all 35 images;
 2. aggregate bytes (35 images) decrease >= 0.5%;
 3. LOSE group improves in aggregate (sum candidate bytes < sum baseline bytes);
 4. no image grows by more than 0.3%;
 5. aggregate encode time and aggregate decode time each <= 1.15x baseline;
 6. format compatibility preserved (codec 1 and codec 2 streams unchanged and still decodable; new behaviour uses a new codec id; decoder state is
    bounded by image size, no input-dependent allocation or panic).  Criterion 6 is judged in the report from the code and the focused tests.
FAIL iff any gate fails, or aggregate decrease < 0.2%, or LOSE does not improve in aggregate, or timing > 1.15x materially (> 1.30x), or an image grows > 1.0%.
Otherwise INCONCLUSIVE (do not ship, no follow-up variants).
If PASS: integrate exactly this implementation as a new codec id (codec 3 = X4P + GBlend, written by default only after the regression tests pass),
keep codecs 1 and 2 decodable, run focused tests, re-run the benchmark on the production build.
If FAIL/INCONCLUSIVE: production unchanged; stop pursuing this predictor.
