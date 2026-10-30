# Success criteria (frozen BEFORE any validation image is encoded)

Stage = N training images (N=5 first). The learned candidate is chosen on TRAINING data only: the order-2 (pairwise
interaction) model if its 5-fold held-out-theta CV error is < 0.90 x the order-1 (additive) model's, else order-1.
Candidates evaluated on the fixed validation set (10 images, never used for fitting or selection):
baseline (unchanged X4, theta = defaults), indep (coordinate-wise tuning: each parameter tuned alone, others at
default), o1 (argmin of the additive model), o2 (argmin of the pairwise model), bestmeasured (best theta among the
points actually measured on the training images). Every non-baseline candidate is charged 5 bytes per file to signal
theta (one level index per parameter). Sizes are real coded bytes (stream + 11-byte header), not cross-entropy.

Stage SUCCESS requires ALL of, for the selected candidate:
- C1 every validation image decodes byte-exactly (all candidates; any failure aborts the stage).
- C2 validation aggregate bytes (incl. signalling) <= 99.70% of baseline (>= 0.30% smaller).
- C3 at most 3 of 10 validation images are larger than baseline by > 0.30%, and none by > 1.5%.
- C4 summed single-thread encode time and decode time (best of 2) each within +10% of baseline.
Relationship CLAIM (reported separately; the stage can succeed without it):
- C5 the selected candidate is o2 AND its validation bytes beat BOTH indep and o1 by >= 0.10% of baseline.
- C6 (entanglement evidence, diagnostic) on the validation full grid, o2 has lower held-out RMSE than o1 by >= 10%.
Sanity (diagnostic): |cross-entropy/8 - coded stream bytes| < 0.05% on every encode.

If the stage fails: report why; do not enlarge the data or the model automatically.
Continuation to 10 images is considered only if C1-C4 pass.
