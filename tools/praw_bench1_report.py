"""PRAW benchmark 1, step 4: merge manifest + PRAW + baseline results into per_image.csv, summary.json and tables.md.
Run: python -I tools/praw_bench1_report.py <result_dir>"""
import csv, json, os, sys

d = sys.argv[1]
man = json.load(open(f"{d}/dataset_manifest.json"))
praw = {r["id"]: r for r in json.load(open(f"{d}/praw_results.json"))}
base = json.load(open(f"{d}/baseline_results.json"))
ids = [m["id"] for m in man]
codecs = ["praw", "png", "jxl", "zstd"]
label = {"praw": "PRAW", "png": "PNG", "jxl": "JPEG XL", "zstd": "Zstd-19"}


def size(i, c):
    return praw[i]["payload_bytes"] if c == "praw" else base["images"][i][c]["bytes"]


def ok(i, c):
    return praw[i]["exact"] if c == "praw" else base["images"][i][c]["exact"]


rows = []
for m in man:
    i = m["id"]
    r = {"id": i, "camera": m["camera"], "w": m["width"], "h": m["height"], "bits": m["bit_depth"],
         "dng_comp": m["dng_compression"], "canon": m["canon_bytes"], "cfa": praw[i]["layout"]}
    for c in codecs:
        r[c + "_bytes"] = size(i, c)
        r[c + "_ratio"] = m["canon_bytes"] / size(i, c)
        r[c + "_saved_pct"] = 100 * (1 - size(i, c) / m["canon_bytes"])
        r[c + "_exact"] = ok(i, c)
    rows.append(r)
with open(f"{d}/per_image.csv", "w", newline="") as f:
    w = csv.DictWriter(f, fieldnames=list(rows[0]))
    w.writeheader()
    w.writerows(rows)

tot = sum(r["canon"] for r in rows)
out = []
P = out.append
P("## Aggregate (total canonical bytes / total encoded bytes)\n")
P("| codec | total encoded B | ratio | saved % | mean per-image ratio | all exact |")
P("|---|---:|---:|---:|---:|:--:|")
for c in codecs:
    t = sum(r[c + "_bytes"] for r in rows)
    mr = sum(r[c + "_ratio"] for r in rows) / len(rows)
    P(f"| {label[c]} | {t:,} | {tot / t:.4f} | {100 * (1 - t / tot):.2f} | {mr:.4f} | {all(r[c + '_exact'] for r in rows)} |")
P(f"\nTotal canonical bytes: {tot:,} across {len(rows)} images.\n")

P("## Per-image (ratio = canonical bytes / encoded bytes; best per row in bold)\n")
P("| id | camera | WxH | bits | DNG | PRAW | PNG | JXL | Zstd | PRAW vs JXL |")
P("|---|---|---|--:|---|--:|--:|--:|--:|--:|")
for r in rows:
    rr = {c: r[c + "_ratio"] for c in codecs}
    best = max(rr, key=rr.get)
    cell = lambda c: f"**{rr[c]:.3f}**" if c == best else f"{rr[c]:.3f}"
    d_jxl = 100 * (r["jxl_bytes"] / r["praw_bytes"] - 1)
    P(f"| {r['id']} | {r['camera']} | {r['w']}x{r['h']} | {r['bits']} | {r['dng_comp']} | {cell('praw')} | {cell('png')} | {cell('jxl')} | {cell('zstd')} | {d_jxl:+.1f}% |")

P("\n('PRAW vs JXL' = how much smaller PRAW is than JXL: +x% means PRAW output is x% smaller.)\n")
P("## Win/loss of each codec against PRAW (encoded bytes, per image)\n")
P("| codec | smaller than PRAW | larger than PRAW |")
P("|---|--:|--:|")
for c in codecs[1:]:
    wins = sum(r[c + "_bytes"] < r["praw_bytes"] for r in rows)
    P(f"| {label[c]} | {wins} | {len(rows) - wins} |")
P("")
P("## PRAW best / worst by ratio\n")
rs = sorted(rows, key=lambda r: r["praw_ratio"])
P("Worst: " + ", ".join(f"{r['id']} ({r['praw_ratio']:.3f})" for r in rs[:4]))
P("\nBest: " + ", ".join(f"{r['id']} ({r['praw_ratio']:.3f})" for r in rs[::-1][:4]) + "\n")

P("## PRAW diagnostics\n")
P("| id | Stage 2 | bits/sample (payload) | S1 zero% | S1 mean zz | S1 H0 b/s | Low% | Mod% | High% | VHigh% | class-H0 b/s | enc s | dec s |")
P("|---|---|--:|--:|--:|--:|--:|--:|--:|--:|--:|--:|--:|")
for m in man:
    p = praw[m["id"]]
    n = p["width"] * p["height"]
    cl = p["classes"]
    t = p["time_s"]
    P(f"| {p['id']} | {p['stage2_mode']} | {p['bits_per_sample']:.3f} | {100 * p['stage1']['zero_frac']:.1f} | {p['stage1']['mean_zz']:.1f} | "
      f"{p['stage1']['h0_bits']:.3f} | " + " | ".join(f"{100 * c['frac']:.1f}" for c in cl) +
      f" | {8 * p['class_static_h0_bytes'] / n:.3f} | {t['stage1'] + t['stage2'] + t['stage3'] + t['stage4']:.0f} | {t['dec_stage4'] + t['dec_restore']:.0f} |")

P("\n## Correctness gate\n")
P("| codec | exact / total |")
P("|---|--:|")
for c in codecs:
    P(f"| {label[c]} | {sum(r[c + '_exact'] for r in rows)} / {len(rows)} |")
open(f"{d}/tables.md", "w").write("\n".join(out) + "\n")
print("\n".join(out))
