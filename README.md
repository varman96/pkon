# nanoscope

Research rig for lossless image compression experiments. Goal (long-term, unproven): beat JPEG XL
lossless on diverse images. This repo currently contains **only the measurement rig**, no novel codec.

```
cargo test --release                 # correctness gate (~25 s; ffmpeg tests skip loudly if ffmpeg is missing)
cargo run --release --bin bench      # synthetic corpus, ~1 min -> results/<UTC timestamp>/{results.json,results.csv,summary.md}
cargo run --release --bin bench -- --dir path/to/pngs   # add external PNGs (same harness)
```

- **Canonical form** (`src/canon.rs`): interleaved 8/16-bit (LE) gray / gray+alpha / RGB / RGBA samples, no padding. Every codec is compared on exactly these bytes.
- **Adding a codec**: implement `Codec` (`src/codec.rs`), add it to the list in `src/bin/bench.rs`. The harness diffs every sample and rejects decoders that ignore their input.
- **External tools**: JXL/WebP currently go through `ffmpeg` (override with `NANOSCOPE_FFMPEG`). Reference `cjxl`/`djxl`/`cwebp` are not installed; install libjxl (e.g. `winget install libjxl`) before quoting JXL numbers as "the" baseline.
- Results directories are never overwritten. Synthetic images are diagnostic only.

## PKON2 milestone 1 (statistics probe)

```
python tools/fetch_dataset.py        # CC0 camera JPEG/PNG originals from Wikimedia Commons -> data/ (gitignored), fixed selection rule
cargo run --release --bin probe      # ~3 min -> results/<stamp>-pkon2-m1/{report.md,results.json}
cargo test --release --test pkon2    # every predictor/colour/context config round-trips bit-exactly
```
`src/pkon2.rs` is a small config-switchable coder (predictor x colour handling x context, adaptive range coder); every
number in the report is real coded bytes with a verified exact decode, plus entropy screens. Not a production codec.

Milestone 2A (adaptive SubG/Coupled selection, rejected): `cargo run --release --bin probe_m2` -> `results/<stamp>-pkon2-m2a/`.
