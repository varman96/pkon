# LISA RP3: Kalman-gain adaptation rate (ideal coding cost only)

Code: `src/lisa_rp3.rs`, `src/bin/exp_lisa_rp3.rs` (new; `lisa_rp.rs`, `lisa_rp2.rs` and production untouched; one `pub mod` line added to `lib.rs`).
Data: `results.json`, `stdout.txt`. These are ideal prequential bit costs, not byte-level compression results.

## Equations implemented
Per decision stream (class, kind, slot), reset once per image. `p_t` is the RP probability `(r+eps)/(r+p+2eps)`, eps = 1/1024, r = p = 0.5 initially, read before bit `b_t` is seen:

- a_t = p_t - 2 p_{t-1} + p_{t-2}  (a_t = 0 for the first two observations of a stream)
- Q_t = a_t^2;  R_t = p_t (1 - p_t);  V^-_t = V_{t-1} + Q_t;  V_{-1} = 0.5
- alpha_t = V^-_t / (V^-_t + R_t)
- r <- (1-alpha) r + alpha b;  p <- (1-alpha) p + alpha (1-b)  (the RP components; the next p_{t+1} is read from them)
- V_t = (1 - alpha_t) V^-_t

Interpretation made explicit: the update `p_{t+1} = p_t + alpha (b - p_t)` is realized by the RP components, so the probability state is the RP state (no separate state). Because r+p stays 1, this equals the stated update up to the eps offset.
Safeguard: if V^- + R is exactly 0, alpha = 0. Never triggered (R > 0 always; zero-alpha count = 0). No alpha clamp.

## Integrity
- Production and fixed-rate numbers reproduce `lisa-rp1` exactly (7.05538/7.01054, 7.78884/7.85351, 9.63275/9.57974).
- All three estimators run in one loop over identical decisions (count, ones, hash asserted equal); one reset per image.
- Raw p=1/2 mantissa bits are included identically in all three costs.
- Focused tests (4/4 pass): update order vs independent hand derivation (bit-exact), alpha independent of current bit (causality), finite/valid state over 5 adversarial 200k-bit streams (all-0, all-1, alternating, blocks, pseudo-random), zero-denominator safety. Initial gain = 2/3 as the specified initialization implies.
- Invalid or non-finite states: 0 on all three images.

## Results (ideal bits per sample)

| image | production | fixed 1/32 | Kalman | Kalman vs fixed | Kalman vs production | fixed vs production |
|---|---|---|---|---|---|---|
| pentax_q7_cat | 7.05538 | 7.01054 | 10.23089 | +45.936% | +45.008% | -0.636% |
| pentax_kx_lichen | 7.78884 | 7.85351 | 11.54823 | +47.045% | +48.266% | +0.830% |
| parrot_anafi_dusk | 9.63275 | 9.57974 | 13.00968 | +35.804% | +35.057% | -0.550% |
| aggregate (total bits / total samples) | 8.45278 | 8.43376 | 11.87995 | +40.862% | +40.545% | -0.23% |

## Adaptation rate (all decisions, including warm-up)

| image | mean | min | max | mean if Q=0 | zero |
|---|---|---|---|---|---|
| q7 cat | 0.2977 | 9.2e-6 | 0.99967 | 0.2174 | 0 |
| kx lichen | 0.2993 | 8.6e-6 | 0.99967 | 0.2179 | 0 |
| parrot dusk | 0.2235 | 1.1e-6 | 0.99967 | 0.1638 | 0 |

Distribution (q7 cat; others similar, see `stdout.txt` for octave histograms): 24% of decisions have alpha >= 1/2, 39.7% >= 1/4, 54.7% >= 1/8, 78.2% >= 1/32 (the fixed rate); only ~3.6% below 2^-9.

## Does the rate respond to the probability trajectory?
Yes, strongly and monotonically. Mean alpha rises with |a|: about 0.055 (|a| < 1e-4), 0.16, 0.27, 0.39, 0.35-0.37, 0.45, 0.75 (0.3 <= |a| < 1), 0.974 (|a| >= 1), nearly identical on all three images. Q contributes materially: it raises mean alpha about 36-37% over the Q=0 counterfactual, and nearly doubles it for |a| >= 0.3.
The rate also decays with stream age (mean 0.43 at age 1e3 down to ~0.2-0.25 beyond 1e6) as V shrinks, but the decay is shallow.

## Where the loss sits
Kalman minus fixed is positive in every one of the 16 image segments on every image (about +0.16..+0.25 bps each), so the loss is stationary, not tied to a region. The rate is far above 1/32 for most decisions (the fixed rate is a stronger prior for these nearly stationary unary decisions), so the estimator tracks noise. This matches the direction of the `lisa-rp2` second-difference result but is larger.

## Conclusions and next decision
1. The specified controller is implemented faithfully; no instability or invalid state occurred.
2. It is much worse than both the fixed-rate RP (+36% to +47%) and production (+35% to +48%) on all three images; no gain on any image.
3. With V_0 = 0.5 and R <= 0.25, the specified recursion yields large gains whenever Q is non-negligible, and Q = a^2 is large because p_t itself moves with the large alpha, a feedback loop. This is a property of the specified equations; I did not tune them.
4. Supported decision: reject this Kalman-gain controller as specified. Ideal costs say nothing about byte-level results, and nothing here supports further tuning of this form. Any next candidate should be a decision for you (for example, a different volatility signal than the second difference of the estimator's own output).
