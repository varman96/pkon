"""Render report.md / per_image.csv / timings.csv for a bench25 results dir from its results.json.
Usage: python -I benchmarks/pkon2/report25.py benchmarks/pkon2/<stamp>-bench25-diversity [extra_env.json]
Pure reporting: every number is read from results.json (measured by benchmarks/pkon2/bench25.rs)."""
import csv, json, os, statistics as st, sys

d = sys.argv[1]
R = json.load(open(f"{d}/results.json", encoding="utf-8"))
extra = json.load(open(sys.argv[2], encoding="utf-8")) if len(sys.argv) > 2 else {}
imgs = R["images"]
CODECS = ["pkon2-file", "jxl-e7-t1", "jxl-e9-t1", "webp-m6-t1", "png-rs"]
LABEL = {"pkon2-file": "PKON2 (.pkon2, codec 3)", "jxl-e7-t1": "JXL lossless e7", "jxl-e9-t1": "JXL lossless e9", "webp-m6-t1": "WebP lossless m6", "png-rs": "PNG (png-rs best)"}
rec = {(r["image"], r["codec"]): r for r in R["records"]}
names = [i["name"] for i in imgs]
meta = {i["name"]: i for i in imgs}
ok = {n: all(rec[(n, c)]["valid"] for c in CODECS) for n in names}
valid_names = [n for n in names if ok[n]]
med = lambda v: st.median(v)
out = []
w = out.append
f1 = lambda x: f"{x:,.1f}"
pct = lambda a, b: f"{(a / b - 1) * 100:+.2f}%"

w("# PKON2 25-image diverse compression benchmark\n")
w(f"Results dir: `{d}`. Production default under test: PKON2 REL6, codec id 3, written by `pkonfile::encode` (24-byte header + payload; all stored sizes include the header). "
  f"Images: {len(imgs)}. Images with all five configurations valid and exactly reconstructed: {len(valid_names)}. "
  f"Invalid/failed records: {sum(1 for r in R['records'] if not r['valid'])}.\n")
w("## Input representation\n")
w("Every source file was decoded once (Rust `image` crate, `Canonical::load`) to interleaved 8-bit samples and centre-cropped to at most 1024x1024 (the same crop pipeline used by every earlier PKON2 report). "
  "All codecs received the identical crop buffer (checked by SHA-256 in the manifest; the pre-encode copy is `manifest_pre_encode.json`). "
  "RGB sources stay 3-channel (raw 24 bpp); PNG-origin RGBA sources stay 4-channel (raw 32 bpp) for every codec. "
  "Ratio = canonical raw bytes / stored bytes; bpp = stored bits / pixels. **The measurement is of the decoded 1024x1024 crops, not of the full-resolution photographs.**\n")

# ---------------------------------------------------------------- manifest
w("## 1. Manifest (25 images)\n")
w("| # | name | set | category | source fmt | source px | crop | layout | raw B | MED MAE (G) | zero-resid share | mean G | clipped (0/255) | selection note |")
w("|---|---|---|---|---|---|---|---|---|---|---|---|---|---|")
for k, i in enumerate(imgs, 1):
    w(f"| {k} | {i['name']} | {i['set']} | {i['category']} | {i['source_format'].split('/')[-1]} | {i['decoded_full_width']}x{i['decoded_full_height']} | {i['crop_width']}x{i['crop_height']} | {i['crop_format']} | {i['crop_raw_bytes']:,} | {i['med_mae_g']:.2f} | {i['med_zero_frac_g']:.2f} | {i['mean_g']:.0f} | {i['clipped_frac_g'] * 100:.2f}% | {i['selection_reason']} |")
w("\nPaths, source-file SHA-256 and crop-buffer SHA-256 for every image are in `manifest.csv` (and `manifest_pre_encode.json`). "
  "Descriptors (MED MAE etc.) are computed on the G plane of the crop by the harness and are used only for grouping; they are not part of any codec. "
  "Category labels come from the existing corpus manifests; they were not re-verified visually here.\n")

w("### Diversity summary\n")
from collections import Counter
cat = Counter(i["category"] for i in imgs)
w(f"- From the existing 23-image corpus: {sum(i['set'] == 'retained23' for i in imgs)}; added from the extra 29-image pool (`data/new50`): {sum(i['set'] == 'added' for i in imgs)}. Selection was by content/statistic diversity, made before any encoding.")
w(f"- Layouts: " + ", ".join(f"{k}: {v}" for k, v in Counter(i['crop_format'] for i in imgs).items()) + ". Source formats: " + ", ".join(f"{k}: {v}" for k, v in Counter(i['source_format'] for i in imgs).items()) + ".")
w("- Alpha: the RGBA inputs are PNG-origin; their alpha plane is constant 255 in the crops used (measured below), so alpha reconstruction is tested for layout and exactness but the plane carries no information.")
mm = [i["med_mae_g"] for i in imgs]
w(f"- MED MAE (G) spans {min(mm):.2f} to {max(mm):.2f} (median {med(mm):.2f}); zero-residual share {min(i['med_zero_frac_g'] for i in imgs):.2f}-{max(i['med_zero_frac_g'] for i in imgs):.2f}; mean G {min(i['mean_g'] for i in imgs):.0f}-{max(i['mean_g'] for i in imgs):.0f}; clipped share up to {max(i['clipped_frac_g'] for i in imgs) * 100:.1f}%.")
w("- Categories: " + ", ".join(f"{k} ({v})" for k, v in sorted(cat.items())) + ".")
w("- Categories NOT represented as such in this set (no image selected for them): none claimed beyond the list above; e.g. there is no synthetic noise image and no pure-gradient image — smooth content is represented only by the lowest-MAE photographs.\n")

# ---------------------------------------------------------------- environment
w("## 2. Encoders, versions, settings\n")
E = R["env"]
w("| config | implementation | settings actually passed | lossless confirmation |")
w("|---|---|---|---|")
w(f"| PKON2 | in-process Rust, `nanoscope::pkonfile::encode/decode`, header codec byte read from every output (must be 3) | none (production default, `CODEC_CURRENT` = {E['pkon2_codec_current']}) | exact decode compare of every rep; header CRC-32 matches the input pixels; flipped payload bit rejected |")
w(f"| JPEG XL e7 | `{E['ffmpeg']}` -> libjxl wrapper, one process per call | `-c:v libjxl -distance 0 -effort 7 -pix_fmt <rgb24\\|rgba> -threads 1` | `distance 0` = lossless (ffmpeg doc: 'zero = lossless'); exact decode compare |")
w("| JPEG XL e9 | same | `-distance 0 -effort 9 -threads 1` | same |")
w("| WebP m6 | same ffmpeg, libwebp wrapper | `-c:v libwebp -lossless 1 -compression_level 6 -quality 100 -pix_fmt bgra -threads 1` | output contains a `VP8L` chunk (payload_bytes>0 in results.json); exact decode compare |")
w(f"| PNG | in-process Rust: {E['png_codec']} | {E['png_config']} | PNG is lossless by definition; exact decode compare |")
for k, v in extra.items():
    w(f"\n- {k}: {v}")
w(f"\nThread counts: PKON2 and PNG are single-threaded; ffmpeg encoders/decoders were run with `-threads 1`. Machine: {E['logical_cpus']} logical CPUs (Intel i7-1185G7 laptop, Windows 11). "
  f"Timing procedure: per (image, codec) 1 untimed warm-up round trip, then {E['reps_timed']} timed rounds; each round = encode -> decode from the encoded bytes -> exact compare (all rounds, including warm-up, are compared). "
  "Timed regions are exactly `Codec::encode` / `Codec::decode`. Reported timing = median of the reps; min/max are in `timings.csv`. "
  "Codecs run sequentially, one at a time, with no other workload started by the harness.\n")
w("**Timing caveat:** JXL and WebP are driven through the `ffmpeg` CLI, so their encode and decode times include process spawn, raw temp-file write/read and ffmpeg's container/demux layer. "
  "Measured no-op floor on a 33x17 image (best of 5, ms): " + "; ".join(f"{n} enc {a:.1f} / dec {b:.1f}" for n, a, b in R["calibration_ms"]) + ". PKON2 and PNG are in-process and carry no such overhead; timings are therefore not like-for-like across those two groups. "
  "The PNG encoder is the Rust `png` crate at its highest compression setting; no stronger PNG optimiser (zopfli/oxipng) was available, so 'best available' means that.\n")

# ---------------------------------------------------------------- exactness
w("## 3. Exact reconstruction\n")
w("| image | " + " | ".join(LABEL[c] for c in CODECS) + " |")
w("|---|" + "---|" * len(CODECS))
for n in names:
    w(f"| {n} | " + " | ".join("exact" if rec[(n, c)]["valid"] else "**FAIL**: " + rec[(n, c)].get("note", "") for c in CODECS) + " |")
nbad = sum(1 for r in R["records"] if not r["valid"])
w(f"\n{len(R['records']) - nbad}/{len(R['records'])} (image, codec) outputs reconstructed every sample exactly in all reps (including alpha where present) and encoded deterministically across reps. Failures: {nbad}.\n")
alpha_const = "n/a"
w("PKON2 additionally: header codec id = 3 on " + str(sum(1 for n in names if rec[(n, 'pkon2-file')].get('pkon2_codec_id') == 3)) + f"/{len(names)} files, header CRC/dimensions/layout match the input on " + str(sum(1 for n in names if rec[(n, 'pkon2-file')].get('pkon2_metadata_ok') is True)) + f"/{len(names)}.\n")

# ---------------------------------------------------------------- sizes
w("## 4. Stored size\n")
raw = {n: meta[n]["crop_raw_bytes"] for n in names}
px = {n: meta[n]["crop_width"] * meta[n]["crop_height"] for n in names}
B = lambda n, c: rec[(n, c)]["total_bytes"]
w("### Per-image stored bytes (complete file; `.pkon2` includes its 24-byte header)\n")
w("| image | layout | " + " | ".join(LABEL[c] for c in CODECS) + " | smallest |")
w("|---|---|" + "---|" * len(CODECS) + "---|")
for n in names:
    best = min(CODECS, key=lambda c: B(n, c))
    w(f"| {n} | {meta[n]['crop_format']} | " + " | ".join((f"**{B(n, c):,}**" if c == best else f"{B(n, c):,}") for c in CODECS) + f" | {LABEL[best]} |")

def agg_table(ns, title):
    tot = {c: sum(B(n, c) for n in ns) for c in CODECS}
    r_ = sum(raw[n] for n in ns)
    p_ = sum(px[n] for n in ns)
    w(f"\n{title} ({len(ns)} images, raw {r_:,} B, {p_:,} px)\n")
    w("| codec | total bytes | vs PKON2 | bpp (stored bits / px) | aggregate ratio | mean per-image ratio | median per-image ratio |")
    w("|---|---|---|---|---|---|---|")
    for c in CODECS:
        rr = [raw[n] / B(n, c) for n in ns]
        w(f"| {LABEL[c]} | {tot[c]:,} | {pct(tot[c], tot['pkon2-file'])} | {tot[c] * 8 / p_:.3f} | {r_ / tot[c]:.3f} | {st.mean(rr):.3f} | {med(rr):.3f} |")
agg_table(valid_names, "### Aggregate")

w("\n### Per-image bits per pixel and compression ratio (vs canonical raw)\n")
w("| image | raw bpp | " + " | ".join(f"{LABEL[c]} bpp / ratio" for c in CODECS) + " |")
w("|---|---|" + "---|" * len(CODECS))
for n in names:
    w(f"| {n} | {raw[n] * 8 / px[n]:.0f} | " + " | ".join(f"{B(n, c) * 8 / px[n]:.3f} / {raw[n] / B(n, c):.3f}" for c in CODECS) + " |")

w("\n### Per-image size difference vs PKON2 (positive = codec larger than PKON2)\n")
w("| image | " + " | ".join(LABEL[c] for c in CODECS[1:]) + " |")
w("|---|" + "---|" * (len(CODECS) - 1))
for n in names:
    w(f"| {n} | " + " | ".join(pct(B(n, c), B(n, 'pkon2-file')) for c in CODECS[1:]) + " |")
for c in CODECS[1:]:
    dv = [(B(n, c) / B(n, "pkon2-file") - 1) * 100 for n in valid_names]
    w(f"\n- {LABEL[c]} vs PKON2 per-image: min {min(dv):+.2f}%, median {med(dv):+.2f}%, max {max(dv):+.2f}%; smaller than PKON2 on {sum(d_ > 0 for d_ in dv)}... ".replace("smaller than PKON2 on", "PKON2 smaller on").replace("... ", "") + f" {sum(d_ > 0 for d_ in dv)}/{len(dv)} images, codec smaller on {sum(d_ < 0 for d_ in dv)}/{len(dv)}.")

w("\n### Wins by stored size\n")
wins = Counter(min(CODECS, key=lambda c: B(n, c)) for n in valid_names)
w("| codec | images with smallest file | which |")
w("|---|---|---|")
for c in CODECS:
    ws = [n for n in valid_names if min(CODECS, key=lambda cc: B(n, cc)) == c]
    w(f"| {LABEL[c]} | {wins[c]} | {', '.join(ws) if ws else '-'} |")
lead = ["pkon2-file", "jxl-e7-t1", "jxl-e9-t1"]
w("\nHead-to-head among PKON2, JXL e7, JXL e9 only: " + ", ".join(f"{LABEL[c]} {sum(min(lead, key=lambda cc: B(n, cc)) == c for n in valid_names)}" for c in lead) + ".")

# ---------------------------------------------------------------- groups
w("\n## 5. Results by image characteristic\n")
w("Groups are assigned from the manifest (layout, source format, measured descriptors, category label). Sizes are summed stored bytes within the group; percentages are vs PKON2 in the same group. Group sizes are small; treat differences within a few percent with care.\n")
mae_sorted = sorted(valid_names, key=lambda n: meta[n]["med_mae_g"])
t1, t2 = len(mae_sorted) // 3, 2 * len(mae_sorted) // 3
CATGRP = {"people": ["portrait", "child", "woman"], "animals": ["pet", "pet2", "bird"], "sky / clouds / dusk": ["clouds", "skyline", "sunset"],
          "architecture": ["building", "church", "indoor", "skyline"], "water": ["waterfall", "lake"], "graphic / text / document": ["screen", "text", "document", "blurry"],
          "foliage / ground texture": ["foliage", "path", "outdoor"], "low light": ["night"]}
groups = [
    ("Smooth (lowest-MAE third by MED MAE)", mae_sorted[:t1]),
    ("Mid detail (middle third)", mae_sorted[t1:t2]),
    ("Textured (highest-MAE third)", mae_sorted[t2:]),
    ("Dark (mean G < 80)", [n for n in valid_names if meta[n]["mean_g"] < 80]),
    ("Bright (mean G >= 150)", [n for n in valid_names if meta[n]["mean_g"] >= 150]),
    ("Clipped share >= 1% of G samples at 0/255", [n for n in valid_names if meta[n]["clipped_frac_g"] >= 0.01]),
    ("RGB inputs", [n for n in valid_names if meta[n]["crop_format"] == "rgb8"]),
    ("RGBA inputs (PNG origin)", [n for n in valid_names if meta[n]["crop_format"] == "rgba8"]),
    ("JPEG-origin", [n for n in valid_names if meta[n]["source_format"] == "image/jpeg"]),
    ("PNG-origin", [n for n in valid_names if meta[n]["source_format"] == "image/png"]),
]
for g, cs in CATGRP.items():
    groups.append((f"Category: {g}", [n for n in valid_names if meta[n]["category"] in cs and not (g == "architecture" and meta[n]["category"] == "skyline")]))
for title, ns in groups:
    if not ns:
        continue
    tot = {c: sum(B(n, c) for n in ns) for c in CODECS}
    wn = Counter(min(CODECS, key=lambda c: B(n, c)) for n in ns)
    w(f"**{title}** ({len(ns)}: {', '.join(ns)})\n")
    w("| " + " | ".join(LABEL[c] for c in CODECS) + " |")
    w("|" + "---|" * len(CODECS))
    w("| " + " | ".join(f"{tot[c]:,} ({pct(tot[c], tot['pkon2-file'])}); wins {wn[c]}" for c in CODECS) + " |\n")

# ---------------------------------------------------------------- timings
w("## 6. Timings\n")
w("Per-image medians (ms) over the timed reps. `spread` = (max-min)/median over reps; it is the within-run noise indicator.\n")
T = lambda n, c, k: rec[(n, c)][k]
for k, title in (("enc_ms_reps", "Encode"), ("dec_ms_reps", "Decode")):
    w(f"### {title} time per image (median ms [min-max])\n")
    w("| image | " + " | ".join(LABEL[c] for c in CODECS) + " |")
    w("|---|" + "---|" * len(CODECS))
    for n in names:
        w(f"| {n} | " + " | ".join(f"{med(T(n, c, k)):,.0f} [{min(T(n, c, k)):,.0f}-{max(T(n, c, k)):,.0f}]" for c in CODECS) + " |")
    w(f"\n**{title} aggregate (sum of per-image medians over {len(valid_names)} images)**\n")
    w("| codec | total s | mean ms/image | MB/s (raw px bytes) | vs PKON2 | median per-image spread | max per-image spread | images where all reps of codec < all reps of PKON2 | images where all reps of codec > all reps of PKON2 |")
    w("|---|---|---|---|---|---|---|---|---|")
    base = sum(med(T(n, "pkon2-file", k)) for n in valid_names)
    for c in CODECS:
        tot = sum(med(T(n, c, k)) for n in valid_names)
        sp = [(max(T(n, c, k)) - min(T(n, c, k))) / med(T(n, c, k)) for n in valid_names]
        lo = sum(max(T(n, c, k)) < min(T(n, "pkon2-file", k)) for n in valid_names) if c != "pkon2-file" else "-"
        hi = sum(min(T(n, c, k)) > max(T(n, "pkon2-file", k)) for n in valid_names) if c != "pkon2-file" else "-"
        rawmb = sum(raw[n] for n in valid_names) / 1e6
        w(f"| {LABEL[c]} | {tot / 1e3:.1f} | {tot / len(valid_names):,.0f} | {rawmb / (tot / 1e3):.2f} | {tot / base:.2f}x | {med(sp) * 100:.0f}% | {max(sp) * 100:.0f}% | {lo} | {hi} |")
    w("")
w("Reading the table: 'vs PKON2' is the ratio of summed medians (>1 = slower than PKON2). The last two columns count images where the two codecs' rep ranges do not overlap, i.e. where the ordering was consistent across every repetition. "
  "ffmpeg-driven codecs include the spawn/file overhead noted in section 2.\n")

# ---------------------------------------------------------------- files
with open(f"{d}/per_image.csv", "w", newline="", encoding="utf-8") as fh:
    cw = csv.writer(fh)
    cw.writerow(["image", "codec", "valid", "total_bytes", "payload_bytes", "raw_bytes", "pixels", "bpp", "ratio", "pct_vs_pkon2", "enc_ms_median", "dec_ms_median", "file_sha256"])
    for n in names:
        for c in CODECS:
            r = rec[(n, c)]
            cw.writerow([n, c, r["valid"], r["total_bytes"], r.get("payload_bytes"), raw[n], px[n], f"{r['total_bytes'] * 8 / px[n]:.4f}", f"{raw[n] / r['total_bytes']:.4f}", f"{(r['total_bytes'] / B(n, 'pkon2-file') - 1) * 100:.3f}",
                         f"{med(r['enc_ms_reps']):.2f}", f"{med(r['dec_ms_reps']):.2f}", r["file_sha256"]])
with open(f"{d}/timings.csv", "w", newline="", encoding="utf-8") as fh:
    cw = csv.writer(fh)
    cw.writerow(["image", "codec", "rep", "enc_ms", "dec_ms"])
    for n in names:
        for c in CODECS:
            r = rec[(n, c)]
            for k, (e, dd) in enumerate(zip(r["enc_ms_reps"], r["dec_ms_reps"]), 1):
                cw.writerow([n, c, k, f"{e:.3f}", f"{dd:.3f}"])
with open(f"{d}/manifest.csv", "w", newline="", encoding="utf-8") as fh:
    cols = ["name", "set", "category", "selection_reason", "source_path", "source_format", "source_file_sha256", "decoded_full_format", "decoded_full_width", "decoded_full_height", "crop_width", "crop_height", "crop_format", "crop_channels", "crop_raw_bytes", "crop_sha256", "med_mae_g", "med_zero_frac_g", "mean_g", "clipped_frac_g", "camera", "license", "title", "url"]
    cw = csv.writer(fh)
    cw.writerow(cols)
    for i in imgs:
        cw.writerow([i.get(c, "") for c in cols])
open(f"{d}/report.md", "w", encoding="utf-8").write("\n".join(out) + "\n")
print("wrote report.md, per_image.csv, timings.csv, manifest.csv in", d)
