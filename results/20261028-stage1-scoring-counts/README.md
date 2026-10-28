# PRAW Stage 1: candidate scoring counts (static analysis of `src/praw.rs`)

No code was run or modified. All numbers are **hard upper bounds** read from loop bounds, not measured or typical counts.
"Hard max" assumes every neighbour is available at every position, which may not be reachable simultaneously. No averages are given.

## Call graph (per output sample)

`encode`/`decode` (L352 / L371, one call per Morton-visited pixel) -> `State::predict` (L298):
1. L301 `for p in 0..PATTERNS.len()` (14 patterns, L158-204): eligibility check `needs.iter().any(fetch)` (L302).
2. For each eligible `p`: `score` (L306, L274) loops `TRAIN` (9 offsets, L207): `fetch(actual)` (L278), then `predict_at(p, qx, qy)` (L280) which runs `needs` fetches (L266) and, if all present, `eval` through `g` -> `fetch` (L269).
3. Winner: `predict_at(p, x, y)` (L316), `context` (L317, 4 fetches, L289).

Loop bounds: patterns = 14, TRAIN = 9, no inner loop other than `needs` / `eval` (n_p fetches each).

## Per-pattern counts (hard max per sample)

n = `needs.len()` = number of `g()` calls in `eval`.

| p | pattern | n | eligibility fetches (L302) | score: actual fetches (9) | `predict_at` calls (9) | `needs` fetches (9n) | `eval` fetches (9n) |
|--:|---|--:|--:|--:|--:|--:|--:|
| 0 | MED | 3 | 3 | 9 | 9 | 27 | 27 |
| 1 | W | 1 | 1 | 9 | 9 | 9 | 9 |
| 2 | N | 1 | 1 | 9 | 9 | 9 | 9 |
| 3 | planar | 3 | 3 | 9 | 9 | 27 | 27 |
| 4 | (W+N)/2 | 2 | 2 | 9 | 9 | 18 | 18 |
| 5 | (W+NE)/2 | 2 | 2 | 9 | 9 | 18 | 18 |
| 6 | 2W-WW | 2 | 2 | 9 | 9 | 18 | 18 |
| 7 | 2N-NN | 2 | 2 | 9 | 9 | 18 | 18 |
| 8 | grad blend | 7 | 7 | 9 | 9 | 63 | 63 |
| 9 | (W+E)/2 | 2 | 2 | 9 | 9 | 18 | 18 |
| 10 | (N+S)/2 | 2 | 2 | 9 | 9 | 18 | 18 |
| 11 | 4-avg | 4 | 4 | 9 | 9 | 36 | 36 |
| 12 | med3 | 3 | 3 | 9 | 9 | 27 | 27 |
| 13 | NE | 1 | 1 | 9 | 9 | 9 | 9 |
| **sum** | | **35** | **35** | **126** | **126** | **315** | **315** |

Sum of n: 3+1+1+3+2+2+2+2+7+2+2+4+3+1 = 35.

## Totals per sample (hard max)

| Quantity | Arithmetic | Max |
|---|---|--:|
| Candidate patterns considered | 14 | 14 |
| Candidate-pattern evaluations (replays inside `score`) | 14 x 9 | 126 |
| `predict_at` calls in scoring | 14 x 9 | 126 |
| `predict_at` calls, final (L316) | 1 (selected pattern only) | 1 |
| **`predict_at` total** | 126 + 1 | **127** |
| Per pattern | 9 in `score`, +1 only if selected | 9 (10 if selected) |
| `fetch`, eligibility (`needs`, L302) | sum n | 35 |
| `fetch`, score actual (L278) | 14 x 9 | 126 |
| `fetch`, `needs` inside `predict_at` in scoring (L266) | 9 x 35 | 315 |
| `fetch`, `eval` inside `predict_at` in scoring (L269) | 9 x 35 | 315 |
| `fetch`, final `predict_at` needs + eval | n_sel + n_sel, max n_sel = 7 (p=8) | 14 |
| `fetch`, `context` (L289) | 4 (tuple evaluates all four, no short-circuit) | 4 |
| **`fetch` total** | 35 + 126 + 315 + 315 + 14 + 4 | **809** |

By `needs` vs `eval`: `needs` = 35 (eligibility) + 315 (scoring) + 7 (final, max) = 357. `eval` = 315 (scoring) + 7 (final, max) = 322. Other: 126 (training actuals) + 4 (context) = 130. Sum 357 + 322 + 130 = 809.

## Hard bound vs guaranteed

- Short-circuit: `any(..is_none())` stops at the first missing neighbour, and `predict_at` returns before `eval` on failure. So `needs` fetches per call are between 1 and n, and `eval` fetches are 0 or n. The 809 and 127 figures are therefore ceilings.
- Guaranteed per sample: the eligibility loop runs all 14 patterns, with at least 1 fetch each (>= 14). `context` is exactly 4 fetches whenever a pattern is selected. Each eligible pattern does exactly 9 training-actual fetches and exactly 9 `predict_at` calls, because the loop has no early exit (L277). If no pattern is eligible, `score`, `predict_at` and `context` are not called and the sample costs only the eligibility fetches.
- The fetches in `score` are not bounded below beyond this. How many of the 9 training positions are known depends on Morton order, which was not analysed.

## Repeated work

- Across candidate patterns, the 9 training-actual fetches (L278) are identical for every pattern (same positions, 14 times: 126 fetches versus 9 distinct).
- Across candidate patterns, `predict_at` reads neighbours of each training pixel (qx, qy). The same `fetch(q, offset)` cells are read by many patterns, e.g. W is needed by 10 of 14 patterns. The same availability checks and reads are redone per pattern.
- Eligibility `needs` fetches at the centre pixel (L302) are repeated again for the selected pattern in the final `predict_at` (L266), and again for the `needs` of `context` (W, N, NW, NE).
- Across training pixels, neighbouring training pixels have overlapping neighbourhoods (e.g. the prediction for W's W is fetched at the same cell as the prediction for WW's neighbours), so the same reconstructed cells are fetched multiple times within one `score` call.
- Across samples (consecutive pixels), training pixels at sample (x, y) were centre pixels or training pixels for earlier samples, so their `predict_at` results for the same pattern were possibly computed before. These are not cached, but the bias state may have changed. Only the `predict_at` value itself is a pure function of `rec`/`known` at that location.
- `eval` through `g` calls `fetch` again (L269) for cells that `needs` (L266) just fetched in the same `predict_at` call: 35 + 35 per full replay set, i.e. each needed cell is fetched twice.
