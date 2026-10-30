# Stage 1 code-level performance investigation (read-only)

Source read: `src/praw.rs` only (no tests, benchmarks, compression or decoding were run; nothing was modified).
**[V]** = verified by reading the code. **[H]** = hypothesis; no profiling data exists, so no bottleneck is claimed as measured.
Operation counts below are derived by reading the code (upper bounds for an interior pixel whose neighbours are all known).

## 1. Execution path

Entry: `encode_with_metadata` (praw.rs:553) -> `encode` (praw.rs:341). Callers: bench_praw1.rs:67, praw_mixer.rs:383, exp_praw_mixer.rs:48, exp_praw_cloud.rs:33. `decode` (praw.rs:363) runs the identical `predict`/`update` per sample, so decode cost mirrors encode cost [V].

Per `encode`:
1. `validate` + a linear range scan of `samples` (praw.rs:342-349): O(n), once. [V]
2. `State::new` (praw.rs:238): allocates `rec: Vec<u16>` and `known: Vec<bool>`, n entries each. [V]
3. `morton_visit` (praw.rs:93) -> recursive `visit` (praw.rs:98-117): one call per quadrant, closure `f` called at size 1. [V]
4. Per pixel, in the closure (praw.rs:352-358):
   - `st.predict(x, y)` (praw.rs:298-321)
     - loop over the 14 `PATTERNS` (praw.rs:158-204): eligibility check = `fetch` of every offset in `needs` (praw.rs:302).
     - for each eligible pattern, `score(p, x, y)` (praw.rs:274-285): loops the 9 `TRAIN` offsets (praw.rs:207). Each does one `fetch` of the training pixel, then `predict_at(p, qx, qy)` (praw.rs:264) which re-checks all `needs` via `fetch` and then calls `eval` through a `&dyn Fn` closure that calls `fetch` again per neighbour access, then clamps. Cost accumulates via `log2_q4(zigzag(..)+1)`.
     - pick the lowest score (ties to the lower index), `predict_at` once more for the winner (praw.rs:316).
     - `context` (praw.rs:288): 4 fetches, `leading_zeros`.
     - bias correction: `div_euclid` by the count (praw.rs:319), then clamp to `[0, maxv]` (praw.rs:320).
   - residual: `zigzag(v - pred.value)` pushed to `out` (praw.rs:355), preallocated.
   - `st.update` (praw.rs:323): writes `rec[i]` and `known[i]`, updates and halves the bias tables (`BIAS_LIMIT` 32).

## 2. Likely hot spots (ranked)

The score loop (praw.rs:274-285 and 264-271) is almost certainly the dominant cost, then the fetch primitive it is built on. Everything else is O(1) per sample with small constants.

### Rank 1: candidate scoring replays every pattern on 9 training pixels [V structure, H on share]
Per sample: up to 14 patterns x 9 training positions = **up to 126 `predict_at` calls** (fewer in practice: a pattern needing E/S/NE is ineligible when those pixels are not yet reconstructed, and training pixels near unknown neighbours are skipped). Each call includes a `needs` re-check, an indirect `eval` call through `fn(&dyn Fn)`, and a clamp.

### Rank 2: `fetch` is called a very large number of times [V counts, H on cost]
Counting `needs` lengths from the table (sum over 14 patterns = 35: 3,1,1,3,2,2,2,2,7,2,2,4,3,1), and noting each `eval` calls `g()` once per needed offset (the closures read exactly their needed offsets; `eval` for pattern 8 does 7):
- eligibility pass (praw.rs:302): 35 fetches
- per training position, per pattern: 1 (praw.rs:278) + k (needs check, praw.rs:266) + k (eval, praw.rs:269) = 1 + 2k; summed over patterns = 14 + 70 = 84; times 9 training positions = **~756 fetches**
- final `predict_at` (70) plus `context` (4)

So **roughly 800+ `fetch` calls per sample at the upper bound**, about 17 G for the 21.46 M-sample image if every neighbour were available. Real counts are lower for the reasons above; this is a ceiling, not a measurement.
Each `fetch` (praw.rs:253-261) does i64 add, 4 bounds comparisons, a multiply, then reads `known[i]` and `rec[i]` from two separate arrays. [H] The bounds-check and two-array access are paid on every call.

### Rank 3: pattern 8 (gradient-weighted blend) [V]
It has 7 neighbours, two i64 muls and an i64 **division** (praw.rs:193) per evaluation, at most 10 times a sample (9 training + 1 final), plus 7 fetches each. The only division-heavy pattern. [H] A visible fraction of the score cost because of the division latency.

### Rank 4: indirect call overhead
`eval: fn(&dyn Fn(Off) -> i32) -> i32` (praw.rs:150) and `g` as a `&dyn Fn` (praw.rs:269) prevent inlining of the pattern bodies, and the `PATTERNS` slice is walked at runtime. [H] This costs call overhead and blocks vectorisation / constant folding of `fetch`; the compiler cannot specialise per pattern. Release profile is opt-level 3, thin LTO, 1 codegen unit (Cargo.toml:15-19), so inlining across `fetch` is likely but the dyn indirection stays.

### Lower: Morton traversal, bias, residual output
- `visit` recursion (praw.rs:98): about 4/3 calls per pixel amortised, plus pruned out-of-bounds quadrants (side = 8192 for 5344x4016, praw.rs:94). [V] Small next to ~800 fetches. [H] Negligible.
- Bias correction: one `div_euclid` and a small table access per sample. [H] Negligible.
- Residual push to a preallocated `Vec`: negligible.

## 3. Potential waste

1. **Redundant recomputation across samples [V structure, H on safety].** `score` at pixel (x,y) evaluates `predict_at(p, q)` for 9 pixels q near (x,y); the same q is used as a training pixel by several later pixels (e.g. W of x is later N-type/WW of its own neighbours). The value `predict_at(p, q)` and `actual - pred` depend only on q and p provided q's required neighbours' availability has not changed since the last time. For patterns that read only W/N/NW/WW/NN offsets this is likely stable, because Morton order is monotone in both x and y, so W and N are always visited before the pixel [H, from reasoning; not tested]. Patterns reading NE/E/S can see availability change over time, so a naive cache could differ and break encoder/decoder parity or the bit-exact output. This is the main caveat on the optimisation below.
2. **Double neighbour reads:** `needs` check followed by `eval`'s own reads of the same offsets (praw.rs:266 and 269), and again for the winner (praw.rs:316). Eligibility is also checked a third time in `predict` (praw.rs:302) before `score`. [V]
3. **`known` as a separate `Vec<bool>`** (praw.rs:231): every `fetch` reads two arrays. [H] extra cache/pressure; a sentinel in `rec` could avoid this but would change layout only, not behaviour.
4. **Allocations:** only two n-sized arrays in `State::new` and one preallocated output. No per-sample allocation was found. [V]
5. **Branches:** `Option` chains in `fetch`/`predict_at`/`score` give many data-dependent branches. In the image interior most are predictable; near the Morton boundaries and for E/S availability they are not. [H]

## 4. Morton traversal and locality

- Verified: reconstructed samples are stored row-major in `rec` (praw.rs:259: `ny * w + nx`) but visited in Morton order. [V]
- Neighbour access is within +/-2 rows, so each neighbour access touches a different row of 5344 u16 (10.7 KB), i.e. 3-5 distinct cache lines per fetch group in row-major storage. Morton visiting means consecutive pixels share neighbour lines inside a block, which helps reuse. [H] The working set per local area is small (a few rows x a few tiles), so the L1/L2 hit rate is likely fine; the dominant cost is probably instruction count rather than misses, but that needs cache-miss counters to confirm.
- `known` is a second array with 1-byte entries, a separate line per access (praw.rs:260).
- Out-of-bounds handling in `fetch` runs every call (praw.rs:256), including for interior pixels.

## 5. Best next step

**Single candidate: stop recomputing scoring terms that were already computed for the same training pixel.** Precompute (or cache per pixel) the per-pattern training cost `log2_q4(zigzag(actual - pred) + 1)` once per reconstructed pixel, for patterns whose inputs do not change after the pixel is visited, and sum cached terms in `score` instead of replaying `predict_at`. This targets Rank 1 and 2 directly (most of the ~126 `predict_at` calls and ~750 fetches).

Do not implement before measuring. Evidence needed:
- **Confirm:** a sampling profile or instrumented counters (not yet taken) showing that `score` / `predict_at` / `fetch` account for the large majority of Stage 1 time; counts of actual (not upper-bound) `predict_at` calls per sample; and a check that for the chosen patterns the cached term equals the recomputed term for every pixel (cache invalidation cases: E/S/NE dependent patterns and `TRAIN` pixels that were unknown when visited).
- **Reject:** profile shows time spread across traversal/memory (e.g. cache misses dominate) or `score` is a small share; or the per-pixel cache cannot reproduce identical scores (then the residuals would differ from the current stream, so the idea only works if it is bit-exact).

Any change must keep `encode` output bit-identical (residual stream) since Stage 2-4 and the decoder depend on it.
