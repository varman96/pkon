# REL6 frozen operationalisation (written before any arm was run; thresholds are the task's, copied unaltered)

Arms (src/pkonrel.rs `J_ARMS`): A = production codec 2 (`Extra::P`); B = A + `DummyJointSign{salt 1}` (hash(plane,x,y,salt) mod 15876, sign node only, ALL planes);
C = A + `JointSign` (ctx = ((p*27 + w*9+n*3+ne)*49 + pf)*4 + cls, = 3x4x27x49 = 15,876, sign node only, ALL planes). Initial extra mixer weight 0, same bit-model init, same update order.

Sets: DEV = learn1 train_pool (25), VAL = learn1 val (10), union = 35 unique (DEV & VAL = empty, so the union is meaningful).
Discovery-20: 13 of its images are inside DEV/VAL (9 DEV, 4 VAL); the other 7 are TEST-split images and are NOT loaded (the "keep TEST unloaded" rule wins). Discovery is therefore reported as the 13-image subset; it is not independent of DEV/VAL.
Informational splits: DEV/VAL images outside discovery ("clean": 16 + 6).

Measures (all complete-file bytes, 24-byte header + payload; S = 100*(control-candidate)/control, positive = smaller):
 S(C,A,DEV), S(C,A,VAL), S(C,A,union) aggregate over summed bytes. Sign CE = sum of exact coder cross-entropy at the sign node over all planes, aggregated over the 35 images.
 CE gain C vs B = 100*(CE_B - CE_C)/CE_B on the union.
 Timing: research-harness arms A/B/C (same code path, plain BinEnc/BinDec sink, single thread, sequential, 1 warm-up + 3 timed reps, arm order rotated). Ratio R_enc/R_dec = (sum over images of per-image median-of-3 time of C)/(same for A), and also the median over images of the per-image ratio; BOTH must be <= 1.15 to pass; "reproducible violation" = both aggregates > 1.15 in each of the 3 reps.
 Memory: peak working set in a fresh child process (raw planes loaded from disk first), increment = peak(C) - peak(A), at 128/256/512/1024-px crops. "Bounded" = increment <= 16 MiB and |incr(1024) - incr(128)| <= 2 MiB. "Unacceptable/unbounded" = either violated.

Candidate for further validation: all of: exact on every image (all gates); S(C,A,DEV) >= 0.30; S(C,A,VAL) >= 0.15; S(C,A,union) >= 0.25; CE gain C vs B >= 2.0;
 no DEV/VAL image with C bytes > 1.001 * A bytes; enc and dec timing ratios <= 1.15; memory measured and bounded; source-diff audit confirms isolation.
Reject: any reconstruction/gate failure; CE gain C vs B < 0.8; S(C,A) < 0.1 on DEV AND on VAL ("less than 0.1% on both"); reproducible runtime violation; unbounded/unacceptable memory; a source change outside the experiment.
Inconclusive: everything else; the report names the exact criterion that prevented a decision.
