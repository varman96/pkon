# Source diff audit — REL6 production port (codec id 3)

Full patch of the production-path files: `production_port.patch` (src/pkon2x.rs, src/pkonfile.rs, tests/pkonfile.rs, docs/pkon2-format-v1.md).
New files (additive): `src/bin/prod_ab.rs` (benchmark, uses only the public `pkonfile` API), `tests/pkonfile_codec3.rs`.
Pre-existing uncommitted work (`src/pkonrel.rs` research copy, `src/bin/rel6.rs`, `results/20261027-rel6-joint-sign/`) is untouched by this task; the production path does not call into it (only the new test `codec3_payload_equals_independent_research_arm_c` uses `pkonrel` as an independent cross-check).

## Functional changes (src/pkon2x.rs)
1. `In::J` enum variant: a sign-node-only input; legal only as the 6th and last entry after `In::P` (asserted in `Coder::new`).
2. `x4pj_cfg()` and `JCARD = 3*27*49*4 = 15,876`.
3. `Coder`: `has_j`, `joint: Vec<Bit>` (planes x JCARD, empty unless `In::J`), counted in `state_bytes`.
4. `Coder::sign_bit_j`: the sign decision with the 4 codec-2 sign inputs + the joint input; same stretch/squash/mixer-update/bit-update code as `Coder::bit`; mixer weight 4 starts at 0 (`mixw` init unchanged).
5. `code_sym`: if `has_j` use `sign_bit_j` with context `((p*27+w*9+n*3+ne)*49+pf)*4+cls`; else the unchanged `bit(...)` call.
6. `run_x`: the input-context loop is `.take(NI)` (so `In::J`, the 6th entry, is never evaluated as a magnitude input) plus an `unreachable!` arm for `In::J`.
Not touched: MED/SubG/residual mapping, magnitude contexts and mixing, range coder, tables, all codec-1/2 paths (`nmag`, `has_p`, `bit`, `tail_bit` unchanged; for codec 1/2 `has_j` is false so the new branch is never taken).

## Format plumbing (src/pkonfile.rs, docs)
* `CODEC_X4PJ = 3`, `codec3_cfg()` (spelled out, with a test that it equals `x4pj_cfg()`), `cfg_for`, `parse_header` and `encode_with_codec` accept 3. `CODEC_CURRENT` stays 2; `encode()` still writes codec 2. Header layout, version, flags, CRC unchanged.
* Docs: codec-id row and a new "Codec id 3" section.

## Test change required by the new id (tests/pkonfile.rs, 2 lines)
Two existing tests used id 3 as their example of an *unsupported* codec; they now use 4 (`UnsupportedCodec(4)`). This is the only edit to an existing test and does not weaken any assertion (unknown ids are still rejected).

## Checks
* `cargo test --release`: all existing tests pass (lib 18, pkon2 7, pkonfile 19 incl. golden codec-1 files and pinned codec-1/2 payloads) plus the 2 new codec-3 tests.
* Production codec-3 payload == independent research arm C stream; production codec-2 payload == research arm A (test above).
* No change to the production default, no commit.
