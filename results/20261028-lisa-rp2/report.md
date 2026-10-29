# LISA RP2: second-difference-driven adaptation rate (ideal coding cost only)

Code: `src/lisa_rp2.rs`, `src/bin/exp_lisa_rp2.rs` (new; `lisa_rp.rs` and production untouched).
Data: `results.json` (all diagnostics, machine-readable), `stdout.txt` (full text).
These are ideal prequential bit costs, not byte-level compression results.

## Integrity
- All three estimators run in one loop over identical decisions. Each carries a trace (decision count, ones count, order-sensitive hash); all matched on every image, with one shared reset (model creation) per image. The run asserts this.
- Fixed-rate numbers reproduce `lisa-rp1` exactly (7.01054 / 7.85351 / 9.57974 bps).
- Update order verified by a unit test that re-derives R/P step by step: surprise from pre-update p, d, a, alpha, then a single update.
- Mapping tests (zero, +, -, large, strictly in (0,1)) and the update-order test pass (6/6). Only focused tests were run.
- Warm-up: first two observations per (class, kind, slot) stream use 1/32 (about 200 decisions per image), as specified.

## Results (ideal bits per sample)

| image | production | fixed 1/32 | second-diff | fixed vs prod | dyn vs prod | dyn vs fixed |
|---|---|---|---|---|---|---|
| pentax_q7_cat | 7.05538 | 7.01054 | 7.86062 | -0.636% | +11.413% | +12.126% |
| pentax_kx_lichen | 7.78884 | 7.85351 | 8.81658 | +0.830% | +13.195% | +12.263% |
| parrot_anafi_dusk | 9.63275 | 9.57974 | 10.72579 | -0.550% | +11.347% | +11.963% |

Total ideal bits are in `results.json` (`total_bits`).

## Adaptation-rate distribution (dynamic decisions only)
Reporting bins on u = (alpha - 1/255)/(1/4 - 1/255): near-min u<0.1, mid 0.1..0.9, near-max u>0.9. These bins are for reporting only.

| image | mean | min | max | p50 | p90 | near-min | mid | near-max |
|---|---|---|---|---|---|---|---|---|
| q7 cat | 0.0273 | 0.0039 | 0.238 | ~0.005 | ~0.094 | 77.2% | 22.8% | 0.01% |
| kx lichen | 0.0260 | 0.0039 | 0.238 | ~0.005 | ~0.086 | 78.6% | 21.4% | 0.00% |
| parrot dusk | 0.0221 | 0.0039 | 0.238 | ~0.005 | ~0.067 | 83.2% | 16.8% | 0.02% |

Quantiles come from a 100-bin histogram, so they are accurate to about 0.0025. The rate never gets near 1/4, because |a| > 9 is rare.

## Where the difference sits
Numbers below are KX lichen, in bps of the whole image (dyn minus fixed). The Q7 cat pattern is the same.

- **Classes:** the loss grows with decision count. Class 3 is +0.689, class 2 +0.265, class 1 +0.009, class 0 +0.0002. All four classes lose on every image.
- **Bayer planes:** uniform (+0.233..+0.246), so there is no plane effect.
- **Segments:** uniform across all 16 (about +0.060 each). The loss is stationary, not tied to a drift region.
- **Unary vs mantissa:** unary +0.939, mantissa +0.024. Unary slots 1-5 carry nearly all of it (+0.15..+0.19 each).
- **By |a|** (the relationship between second-difference magnitude and selected rate is in `results.json` → `by_abs_second_diff`):

| \|a\| bin | mean alpha | dyn-fixed (bps) |
|---|---|---|
| < 0.01 | 0.004 | +1.170 |
| 0.01-0.1 | 0.016 | +0.207 |
| 0.1-0.5 | 0.047 | -0.069 |
| 0.5-1 | 0.106 | -0.057 |
| 1-2 | 0.150 | -0.184 |
| 2-5 | 0.186 | -0.105 |
| >= 5 | >= 0.21 | about 0 |

  The same sign pattern appears on Q7.
- **By production probability of the observed outcome:** the dynamic rate wins on surprising outcomes (p < 0.3: -0.35 and -0.33 bps vs fixed on KX), and loses on the likely outcomes (p 0.7-0.99: +0.58 and +0.97 bps vs fixed).

## Answers
1. **KX lichen vs fixed:** no. Dynamic is +12.26% worse than the fixed candidate and +13.20% worse than production. It does not reduce the stable-distribution penalty. It makes it much larger.
2. **Q7 and Parrot gains:** not preserved. Both flip from about -0.6% and -0.55% vs production to +11.4% and +11.3%.
3. **Concentration:** the loss is in stable, high-probability unary decisions (slots 1-5, classes 2-3), across all planes and segments. This is the opposite of the intent. Large |a| does help on the changing decisions, but only modestly, and the loss on stable ones outweighs it by roughly a factor of 5 or more.
4. **Evidence on the design:** the second-difference signal as specified is not useful. Two features of the specified form could explain the result (a mechanism hypothesis, which I did not test):
   - a_t includes the current symbol's surprise, so one rare outcome raises alpha for that same update;
   - 78-83% of decisions sit at about 1/255, which is far slower than the 1/32 that already beat production on Q7 and Parrot, so those decisions track too slowly.

## Observed results and next decision
Observed: second-difference RP is about 12% worse than fixed-rate RP on all three images, with identical symbol sequences verified. The fixed 1/32 candidate remains the only RP variant with any gain over production (Q7, Parrot) and it still regresses on KX.

The data supports not continuing this exact specification. I did not tune or change anything, per the constraints. The decision is yours. Options that would change the specification, which I have not pursued:
- a different signal or mapping;
- a higher rate floor;
- applying the rate to the next update rather than the current one.

The investigation is not closed.
