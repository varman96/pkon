# PKON

**I'm trying to build the best compression in the world.**

PKON is a lossless compression platform that I'm building independently in Rust, and I'm also trying to sell it. It has two complementary engines in one container format: **PKON2** for fast, practical everyday image compression, and **LISA** for high-efficiency lossless RAW/DNG compression. Every file decodes back to exactly the samples that went in.

> **PKON2 is built for speed. LISA is built to push compression further.**

## The two engines

| | **PKON2: Consumer** | **LISA: Pro** |
|---|---|---|
| Built for | Everyday image workflows | Demanding RAW/DNG imaging workflows |
| Formats | PNG and JPEG (a JPEG is encoded from its decoded pixels) | Lossless RAW/DNG |
| Priorities | Fast, practical compression and decompression; lightweight operation (about 63 MB peak working set, single thread, in the production benchmark); efficient storage | Maximum compression efficiency; exact preservation of every RAW sample; specialized prediction and adaptive entropy coding |
| Trade-off | Not the fastest decoder (PNG and WebP decode much faster) | Much slower than the codecs I compare it to; the extra computation buys smaller files |

They share the PKON container, local tooling and the same exact-reconstruction guarantee. They have different priorities and are not competing algorithms.

## Strongest measured results

PKON2 and LISA are benchmarked separately, on different data and against different baselines. Don't compare numbers across the two.

### PKON2: JPEG XL-class compression, with substantially faster encoding

On the 25-image benchmark, PKON2 stores 18,804,922 bytes: **2.60% smaller than JPEG XL effort 9**, with encoding **about 8.5× faster** than effort 9 and **about 2× faster** than effort 7, and **exact lossless reconstruction** of every image. These are the original JPEG/PNG-to-`.pkon2` converter benchmarks. The input is each photo's **decoded 8-bit pixels**, centre-cropped to at
most 1024×1024. It is not the original JPEG file bytes. Sizes are complete files (24-byte header plus payload), and every output was decoded and compared
byte-for-byte with the source pixels. All outputs were exact in every run below.

**25 diverse images** ([report](benchmarks/pkon2/20261028-015857Z-bench25-diversity/report.md); production default, codec 3):

| Codec | Total bytes | Size vs PKON2 | Encode (MB/s of raw pixels) | Decode (MB/s) |
|---|---:|---:|---:|---:|
| **PKON2** | **18,804,922** | | **5.58** | 5.38 |
| JPEG XL (effort 9) | 19,293,200 | +2.60% | 0.66 | 7.21 |
| JPEG XL (effort 7) | 19,464,716 | +3.51% | 2.86 | 6.63 |
| WebP lossless | 21,705,408 | +15.42% | 0.32 | 27.22 |
| PNG | 31,661,761 | +68.37% | 4.67 | 206.19 |

**50 images** ([report](benchmarks/pkon2/20261027-155433Z-bench50/report.md)): 38,914,941 bytes for PKON2 against 39,118,943 for
JPEG XL e9 (0.52% larger), 39,519,208 for e7 (1.55% larger), 44,695,086 for WebP (14.85% larger) and 65,492,478 for PNG
(68.30% larger). PKON2's biggest per-image wins over JPEG XL e9 were `skyline_n0` (−26.6%) and `beach_n0` (−22.3%).
In a later production run of the improved PKON2P coding mode (codec 2), 50 of 50 images came out smaller than the previous production
codec, 1.63% smaller in total ([report](benchmarks/pkon2/20261027-pkon2p-production/report_benchmark.md)).

How I read these honestly:

- The margin over JPEG XL is **thin**. In the 50-image run JPEG XL e9 produced the smallest file on 28 of 50 images and PKON2 on 20. PKON2's real advantage over JPEG XL is **speed at the same ballpark of size**: in the 25-image run it encoded about 8.5× faster than JPEG XL e9 and about 2× faster than e7, and in the 50-image run about 12× faster than e9.
- PKON2 is not the fastest at decoding. PNG and WebP decode much faster, and JPEG XL decoded somewhat faster than PKON2 in the 25-image run.
- Timings include ffmpeg process spawn for JPEG XL and WebP (calibrated and subtracted in the reports where noted). They come from one laptop (i7-1185G7); the reports describe the exact procedure.
- These benchmarks do not compare against the size of the original lossy JPEG file, because PKON2 stores the decoded pixels losslessly. That is a different job.

### LISA: 11.03% fewer bytes than JPEG XL on real RAW files

25 real DNG images, 434.7 megapixels, all represented as 16-bit samples. LISA used 11.03% fewer bytes than JPEG XL across the 25 images and 15.97% fewer on the eight held-out images. It compressed better on 22 of 25 images, and all 25 were reconstructed exactly. **Qualification: the LISA figures below are Stage 4 compressed payload sizes, not complete PKON container sizes.**

| Codec | Total encoded bytes (all 25) | Held-out 8 only |
|---|---:|---:|
| **LISA** | **397,011,489** | **90,543,174** |
| JPEG XL | 440,815,878 | 105,007,221 |
| PNG | 591,948,741 | 147,040,755 |

- LISA used **11.03% fewer bytes than JPEG XL** and **49.10% fewer bytes than PNG**.
- LISA compressed better than JPEG XL on **22 of 25 images**. It did not on the other three (see [the challenge below](#current-challenge-drone-imagery)).
- On the 8 held-out images, LISA used **15.97% fewer bytes than JPEG XL**.
- **All 25 images passed exact lossless round-trip verification**, including all 8 held-out images.

> **Measurement qualification:** LISA figures are **Stage 4 compressed payload sizes**, not complete PKON container sizes. JPEG XL and PNG were
> measured using the same 16-bit RAW sample representation. LISA is also much slower: about 2.1 MiB/s encode, against 8.9 for JPEG XL and
> 15.3 for PNG. Details: [benchmarks/lisa-vs-jxl-png/report.md](benchmarks/lisa-vs-jxl-png/report.md).

**Bayer-plane prediction** predicts each of the sensor's four colour planes separately.
- It gave a **13.57% smaller payload** across the full 25-image corpus and a **17.24% smaller payload** across the eight held-out images.
- Payloads were smaller on **all 24 Bayer images**. The X-Trans image used the existing fallback.
- Source: [historical/20261029-lisa-bayer-plane-small-tree/summary.md](historical/20261029-lisa-bayer-plane-small-tree/summary.md).

**OR-128** is a 128-context residual classifier.
- It was trained on 13 images and selected using a separate validation set of 4 images.
- It was evaluated on the 8 held-out images from the frozen 25-image corpus.
- Records: [lisa-pcq1](historical/20261029-lisa-pcq1/report.md), [lisa-or128-tree](historical/20261029-lisa-or128-tree/report.md).

## Current challenge: drone imagery

**I'm still struggling with drone imagery.** I'm not getting the compression performance I want on drone DNG files, and I haven't solved it.

- In the RAW benchmark, LISA was **larger than JPEG XL on the two drone dusk shots**: `parrot_anafi_dusk` by **67.7%** (the worst case in the corpus) and `dji_fc220_dusk` by **11.9%**. The third loss was an X-Trans chart (8.0% larger), which uses the fallback predictor.
- `dji_fc220_dusk` is one of the 8 held-out images, so the held-out result is not a clean sweep: LISA beat JPEG XL on 7 of those 8.
- I've written diagnostics on the Parrot file ([drone-diagnostics1](historical/20261028-drone-diagnostics1/REPORT.md), [drone-residual-diagnostic](historical/20261028-drone-residual-diagnostic/REPORT.md)). They record measurements, but they don't establish why it compresses badly, and I'm not going to guess. Finding out is open work.

## Why PKON

- **Smaller lossless storage.** PKON2 is ahead of JPEG XL, WebP and PNG in total size on the 25-image and 50-image photo benchmarks; LISA is ahead of JPEG XL and PNG in total size on the RAW benchmark.
- **Faster encoding for everyday images.** PKON2 encodes several times faster than JPEG XL at comparable size.
- **Specialized RAW compression.** LISA works on the sensor's own samples, per colour plane for Bayer sensors, with no demosaic or other processing.
- **Exact sample reconstruction.** Every benchmark output was decoded and compared with its source, and the tooling reports integrity hashes on every conversion.
- **One container, local tooling.** A single `.pkon` format carries both engines, with a local converter UI and a Windows decoder. Nothing leaves your machine.

PKON is under active development. This repository doesn't set licensing terms, pricing or availability, so I'm not stating any here. If you have demanding image-storage needs, [open an issue](https://github.com/varman96/pkon/issues) to start a conversation.

## Quick start: the local converter

**Prerequisites:** a [Rust toolchain](https://rustup.rs/). For DNG files you also need Python 3 with `numpy` and `tifffile`. Run the commands from the repository root.

```
cargo run --release --bin serve
```

Then open **http://127.0.0.1:8080/** in a browser. The server prints `PKON converter: http://127.0.0.1:8080/   (Ctrl+C to stop)`. Press **Ctrl+C** in the terminal to stop it. To use another port, pass it as an argument: `cargo run --release --bin serve -- 9000`. The server listens on localhost only, and nothing is stored on disk.

What the page does:

| You provide | The page produces |
|---|---|
| PNG, JPEG (8-bit) | `.pkon` using PKON2 |
| DNG | `.pkon` using LISA |
| `.pkon` / `.pkon2` (standard image) | PNG |
| `.pkon` (LISA RAW) | lossless `.u16` or `.u8` sample export, plus a `.json` metadata file |

There is also a viewer for opening a standard-image `.pkon` directly in a new tab without converting it. Each conversion shows an integrity report (sizes, ratio and SHA-256 hashes).

## Supported formats and capabilities

| Input | Engine | Notes |
|---|---|---|
| PNG | PKON2 | 8-bit gray, gray+alpha, RGB, RGBA; lossless |
| JPEG | PKON2 | Encoded from decoded pixels; original JPEG bytes are not preserved |
| DNG (RAW) | LISA | 1–16 bit single-channel raster with CFA metadata; exact sample preservation; Bayer-plane prediction, with a fallback for other layouts such as X-Trans |

Output: `.pkon` (v2 container) and legacy `.pkon2`, decodable back to PNG or raw u16/u8 samples.

## Technical architecture

### LISA: the four-stage pipeline

| Stage | What it does | Code |
|---|---|---|
| 1. Prediction / decorrelation | Predicts each sample from already-reconstructed neighbours, choosing among candidate patterns by replaying them on nearby reconstructed samples, with a bounded bias term. It emits exact zigzag-mapped residuals and needs no side information. For Bayer data this runs per colour plane (the production default); other layouts use the mosaic version. | `src/lisa.rs`, `src/lisa_bayer_plane.rs` |
| 2. Organization | A reversible permutation of the residuals. Values are never altered, and a Bayer mosaic is separated into its four CFA-position planes. | `src/lisa.rs` |
| 3. Context classification | Assigns each residual one of 128 contexts (OR-128) from its causal neighbourhood (W, N and NW residuals, each quantised to a half-octave level), using a frozen table. Nothing about the mapping is stored in the stream. | `src/lisa_or128*.rs` |
| 4. Entropy coding | Binarized, count-adaptive range coding with logistic mixing and per-context adaptive probability models (PKON coding). | `src/lisa.rs`, `src/pkon2x.rs` |

The Stage 1 mode, Stage 3 rule and Stage 4 coding version are recorded in the stream, so the decoder needs no out-of-band information and rejects versions it doesn't implement.

### PKON2 and the PKON container

PKON2 (codec "X4" and its production variants) codes 8-bit gray, gray+alpha, RGB and RGBA images with an adaptive, context-mixing coder. The PKON v2 container
([docs/pkon-format-v2.md](docs/pkon-format-v2.md)) has a 32-byte header, codec metadata and a payload:

| Codec id | Payload |
|---|---|
| 1, 2, 3 | PKON2 for standard images (payloads identical to `.pkon2` v1; codec 3 is the default writer) |
| 4 | LISA RAW (1–16 bit, single channel, with CFA metadata) |

`pkon::decode` returns `Decoded::Standard` or `Decoded::Raw` and still reads legacy `.pkon2` v1 files ([docs/pkon2-format-v1.md](docs/pkon2-format-v1.md)).
A JPEG is encoded from its *decoded* pixels; the original JPEG bytes, EXIF and ICC profile are not recoverable.

### DNG ingestion and exact sample preservation

A DNG is read through `tools/lisa_dng_extract.py` (run as a subprocess by `src/dng.rs`) and encoded from its **stored raster, untouched**:
no crop, black-level subtraction, white balance, scaling or demosaic.

### Integrity checks and round-trip verification

Integrity hashes are computed on the canonical sample bytes (`src/integrity.rs`). The container header carries a CRC and length checks. Every benchmark above decodes every output and compares every sample with the source.

### Frontend and Windows decoder

```
cargo run --release --bin serve        # then open http://127.0.0.1:8080/
```

A localhost-only converter UI ([frontend/](frontend/)): PNG/JPEG to `.pkon`, DNG to `.pkon` (LISA RAW), and `.pkon`/`.pkon2` back to PNG or lossless u16/u8
sample export, with an integrity report on every conversion. Nothing is stored on disk. [wic/](wic/) is a separate crate that registers a
Windows Imaging Component decoder for Photos and Explorer thumbnails (`wic/install.ps1`, per-user, no admin; `wic/test.ps1` checks every pixel). Build it from inside `wic/`.

## Benchmark reports and documentation

| Resource | What it is |
|---|---|
| [benchmarks/lisa-vs-jxl-png/report.md](benchmarks/lisa-vs-jxl-png/report.md) | **LISA RAW**: 25 DNGs, payload bytes vs JPEG XL and PNG, 16-bit samples |
| [benchmarks/pkon2/20261028-015857Z-bench25-diversity/report.md](benchmarks/pkon2/20261028-015857Z-bench25-diversity/report.md) | **PKON2**: 25 diverse images, production codec 3, size and speed vs JPEG XL, WebP, PNG |
| [benchmarks/pkon2/20261027-155433Z-bench50/report.md](benchmarks/pkon2/20261027-155433Z-bench50/report.md) | **PKON2**: 50 images, per-image wins and losses |
| [benchmarks/pkon2/20261027-143537Z-ext-benchmark/report.md](benchmarks/pkon2/20261027-143537Z-ext-benchmark/report.md) | **PKON2**: earlier 23-image external comparison (JPEG XL was smaller here) |
| [benchmarks/pkon2/20261027-pkon2p-production/report_benchmark.md](benchmarks/pkon2/20261027-pkon2p-production/report_benchmark.md) | **PKON2P** (codec 2) vs previous production codec |
| [docs/pkon-format-v2.md](docs/pkon-format-v2.md), [docs/pkon2-format-v1.md](docs/pkon2-format-v1.md) | Container format specifications |
| [historical/](historical/) | The development history, including failed and superseded experiments |

Reproduce from the repo root:

```
# PKON2 converter benchmarks (need data/dataset25.json or dataset50.json; see benchmarks/pkon2/build_manifest*.py)
cargo run --release --bin bench50          # also: bench25, bench_external, bench_p
python benchmarks/pkon2/report50.py benchmarks/pkon2/<stamp>-bench50

# LISA vs JPEG XL vs PNG (inputs: data/praw1/canon/*.u16)
cargo run --release --bin exp_lisa_vs_jxl_png -- benchmarks/lisa-vs-jxl-png/source_manifest.json <scratch>/lisa_raw.json 3
python -I benchmarks/lisa-vs-jxl-png/exp_lisa_vs_jxl_png.py <scratch>
```

Use a scratch directory for the LISA reruns so the recorded results aren't overwritten (`<scratch>` needs `source_manifest.json` and `split.json`).


## Report a bug

Open a GitHub Issue at **https://github.com/varman96/pkon/issues**. Where applicable, please include:

- PKON version or Git commit.
- Operating system.
- Input image format and relevant properties (dimensions, bit depth, channels, camera and CFA layout for DNG).
- Steps to reproduce.
- Expected versus actual behaviour.
- Error messages or logs.
- Whether the problem affects encoding, decoding, or both.
- A minimal sample file, only if it's safe and legally permissible to share.

For compression or lossless-integrity bugs, say explicitly whether the decoded output differs from the source. Please don't upload private or sensitive images to a public issue; describe the file's properties instead.

## Contributing

1. Fork the repository.
2. Clone your fork.
3. Create a focused branch.
4. Make your change.
5. Build and run the relevant tests (below).
6. Open a pull request describing the change and how you verified it.

```
cargo fmt --check                          # formatting check
cargo check --all-targets --release
cargo test --release --test <target>       # focused: e.g. pkon_container, pkonfile_codec3, lisa_stage4, roundtrip
cargo test --release                       # full gate before opening the PR
```

`cargo fmt --check` is the recommended formatting check. The repository currently has pre-existing formatting issues, so it may report differences in files you didn't touch. Please don't reformat unrelated files just to make the check pass. Format the Rust files you actually changed, and keep formatting changes focused on your own change.

What I expect from a pull request:

- Keep it focused.
- Preserve lossless correctness. Decoding must reproduce the source exactly.
- Include reproducible benchmark evidence for any performance or compression claim.
- Don't silently change frozen benchmark baselines or historical results.
- Don't change established format behaviour without documenting the compatibility implications in [docs/](docs/).

## Build and development

```
cargo build --release
cargo check --all-targets --release
cargo test --release                       # full correctness gate (ffmpeg-dependent tests skip loudly if ffmpeg is missing)
cargo test --release --test lisa_or128     # OR-128 (reads historical/20261029-lisa-pcq1/train/map_K128.bin)
cargo test --release --test pkon_container
```

Other test targets: `lisa_bayer_plane`, `lisa_or128_tree`, `lisa_pcq`, `lisa_stage4`, `pkon2`, `pkonfile`, `pkonfile_codec3`, `roundtrip`.
Verification binaries: `verify_pkon2_file`, `verify_x4_promotion`.

Requirements:
- **FFmpeg** with libjxl (and libwebp) for the JPEG XL / WebP / PNG baselines; override the binary with `NANOSCOPE_FFMPEG`. Install libjxl (e.g. `winget install libjxl`) before quoting JPEG XL numbers as "the" baseline.
- **Python 3** with `numpy` and `tifffile` for DNG ingestion. `tools/lisa_dng_extract.py` must stay in `tools/`, because `src/dng.rs` finds it relative to the crate root.
- `data/` (gitignored) holds local datasets, e.g. `python tools/fetch_dataset.py`.

The Rust crate is still named `nanoscope` (`nanoscope::pkon`, `nanoscope::pkonfile`), and the FFmpeg override variable is `NANOSCOPE_FFMPEG`. These are code identifiers I haven't renamed; the project is PKON.

## Repository structure

```
README.md  Cargo.toml  Cargo.lock  build.rs  .gitignore
src/                 production library (lisa*, pkon*, dng, canon, integrity, ...)
  bin/               production binaries: serve, verify_pkon2_file, verify_x4_promotion
frontend/            serve.html, viewer.html (embedded by src/bin/serve.rs)
benchmarks/
  pkon2/             JPEG/PNG -> PKON2 benchmarks: sources, manifest/report scripts, reports and results
  lisa-vs-jxl-png/   LISA vs JPEG XL vs PNG: runner, script, report and raw results
experiments/bin/     experiment, probe and diagnostic binaries
historical/          development history, including failed and superseded experiments
tests/  tools/  docs/  wic/  data/
```

Reports in `historical/` are byte-for-byte as written and may still say `results/<name>`; read that as `historical/<name>`, or `benchmarks/pkon2/<name>` for the four PKON2 benchmark directories.
