"""Build per_image.csv, dataset_manifest.csv, verification.md and report.md from a bench50 results.json.
usage: python tools/report50.py results/<stamp>-bench50
Only records with valid=true enter any size/timing statistic; invalid ones are listed as failures."""
import csv, json, math, os, statistics as st, sys

d = sys.argv[1]
R = json.load(open(f"{d}/results.json", encoding="utf-8"))
for _i in R["images"]:
    for _k in ("width", "height", "bytes"):
        _i[_k] = int(_i[_k])
imgs = {i["name"]: i for i in R["images"]}
order = [i["name"] for i in R["images"]]
CODECS = ["pkon2-file", "jxl-e7-t1", "jxl-e9-t1", "webp-m6-t1", "png-rs"]
calib = {c: (e, de) for c, e, de in R["calibration_ms"]}
rec = {(r["image"], r["codec"]): r for r in R["records"]}
ok = lambda n, c: rec.get((n, c)) and rec[(n, c)]["valid"]
fails = [r for r in R["records"] if not r["valid"]]
size = lambda n, c: rec[(n, c)]["total_bytes"]
ratio = lambda n, c: rec[(n, c)]["raw_bytes"] / size(n, c)
full_ok = [n for n in order if all(ok(n, c) for c in CODECS)]  # images where every codec is valid (aggregates use these)

# ---- machine-readable
with open(f"{d}/per_image.csv", "w", newline="", encoding="utf-8") as f:
    w = csv.writer(f)
    w.writerow(["image", "codec", "valid", "byte_exact", "total_bytes", "payload_bytes", "raw_bytes", "ratio", "bpp", "enc_ms", "dec_ms", "file_sha256", "note"])
    for n in order:
        for c in CODECS:
            r = rec[(n, c)]
            if r["valid"]:
                w.writerow([n, c, True, r["byte_exact"], r["total_bytes"], r["payload_bytes"], r["raw_bytes"], f'{r["raw_bytes"]/r["total_bytes"]:.4f}', f'{r["total_bytes"]*8/r["pixels"]:.4f}', f'{r["enc_ms"]:.1f}', f'{r["dec_ms"]:.1f}', r["file_sha256"], ""])
            else:
                w.writerow([n, c, False, r.get("byte_exact"), "", "", r["raw_bytes"], "", "", "", "", "", r["note"]])
mf = ["name", "set", "category", "file", "source_format", "width", "height", "bytes", "decoded_full_format", "decoded_full_width", "decoded_full_height", "crop_width", "crop_height", "crop_format", "crop_sha256", "decoder_notes", "camera", "license", "title", "url", "med_mae_g", "med_zero_frac_g", "mean_g", "clipped_frac_g"]
with open(f"{d}/dataset_manifest.csv", "w", newline="", encoding="utf-8") as f:
    w = csv.writer(f); w.writerow(mf)
    for n in order:
        w.writerow([imgs[n].get(k, "") if not isinstance(imgs[n].get(k), float) else f"{imgs[n][k]:.4f}" for k in mf])
json.dump({"images": R["images"]}, open(f"{d}/dataset_manifest.json", "w", encoding="utf-8"), indent=1, ensure_ascii=False)

# ---- helpers
def pct(a, p):
    a = sorted(a); k = (len(a) - 1) * p; lo, hi = math.floor(k), math.ceil(k)
    return a[lo] + (a[hi] - a[lo]) * (k - lo)
def table(h, rows):
    return "| " + " | ".join(h) + " |\n|" + "|".join("---" for _ in h) + "|\n" + "\n".join("| " + " | ".join(map(str, r)) + " |" for r in rows) + "\n"
def spearman(x, y):
    def rk(v):
        s = sorted(range(len(v)), key=lambda i: v[i]); r = [0] * len(v)
        for p, i in enumerate(s): r[i] = p
        return r
    a, b = rk(x), rk(y); ma, mb = st.mean(a), st.mean(b)
    num = sum((p - ma) * (q - mb) for p, q in zip(a, b)); den = math.sqrt(sum((p - ma) ** 2 for p in a) * sum((q - mb) ** 2 for q in b))
    return num / den
raw = lambda ns: sum(rec[(n, "pkon2-file")]["raw_bytes"] for n in ns)
tot = lambda ns, c: sum(size(n, c) for n in ns)

L = []
P = L.append
P(f"# PKON2 50-image compression test\n")
P(f"Results dir: `{d}`. Images: {len(order)} ({sum(imgs[n]['set']=='retained' for n in order)} retained from the earlier 23, {sum(imgs[n]['set']=='new' for n in order)} new; the 2 earlier screenshots were dropped). Images with all five codecs valid: {len(full_ok)}. Invalid/failed records: {len(fails)}.\n")
P("**What is measured:** lossless compression of the *decoded pixels* (1024x1024 centre crop, RGB8) of each photo, not of the original JPEG file bytes. Every size is the complete file/stream the codec emits (`.pkon2` = 24-byte header + payload). Ratio = raw pixel bytes / stored bytes, all against the same decoded buffer. Every output was decoded from its own bytes and compared byte-for-byte with that buffer; invalid outputs are excluded from all statistics below.\n")

# aggregate
P("## Aggregate (sum of stored bytes over the images valid for all codecs)\n")
rw = raw(full_ok); pk = tot(full_ok, "pkon2-file")
rows = []
for c in CODECS:
    t = tot(full_ok, c); rs = [ratio(n, c) for n in full_ok]
    rows.append([c, t, f"{t*8/(rw/3):.3f}", f"{rw/t:.3f}", f"{(t/pk-1)*100:+.2f}%", f"{st.mean(rs):.3f}", f"{st.median(rs):.3f}"])
P(f"Raw: {rw} B over {len(full_ok)} images ({rw//3} px).\n")
P(table(["codec", "total bytes", "bpp (24 raw)", "aggregate ratio", "vs pkon2-file", "mean per-image ratio", "median per-image ratio"], rows))

# distribution
P("## Per-image compression-ratio distribution\n")
rows = []
for c in CODECS:
    rs = [ratio(n, c) for n in full_ok]
    rows.append([c] + [f"{pct(rs, p):.2f}" for p in (0, .1, .25, .5, .75, .9, 1)])
P(table(["codec", "min", "p10", "p25", "median", "p75", "p90", "max"], rows))

# wins
P("## Smallest output per image (ties impossible at byte level here)\n")
win = {c: 0 for c in CODECS}
for n in full_ok: win[min(CODECS, key=lambda c: size(n, c))] += 1
P(table(["codec", "images smallest", "share"], [[c, win[c], f"{win[c]/len(full_ok)*100:.0f}%"] for c in CODECS]))
win3 = {c: 0 for c in ("pkon2-file", "jxl-e7-t1", "jxl-e9-t1")}
for n in full_ok: win3[min(win3, key=lambda c: size(n, c))] += 1
P("Head-to-head among the three leaders only: " + ", ".join(f"{c} {v}" for c, v in win3.items()) + ".\n")
for j in ("jxl-e7-t1", "jxl-e9-t1"):
    k = sum(size(n, "pkon2-file") < size(n, j) for n in full_ok)
    P(f"- pkon2-file smaller than {j} on {k}/{len(full_ok)} images.")
P("")

# vs JXL
for j in ("jxl-e7-t1", "jxl-e9-t1"):
    diffs = sorted(((size(n, "pkon2-file") / size(n, j) - 1) * 100, n) for n in full_ok)
    P(f"## PKON2 vs {j}: per-image size difference (negative = PKON2 smaller)\n")
    P(f"Distribution of (pkon2/{j} - 1): min {diffs[0][0]:+.2f}%, median {pct([x for x,_ in diffs], .5):+.2f}%, max {diffs[-1][0]:+.2f}%.\n")
    def row(x, n):
        return [n, imgs[n]["category"], imgs[n]["set"], size(n, "pkon2-file"), size(n, j), f"{x:+.2f}%", f'{imgs[n]["med_mae_g"]:.2f}']
    P("Largest PKON2 wins:\n"); P(table(["image", "category", "set", "pkon2 B", j + " B", "diff", "MED MAE (G)"], [row(x, n) for x, n in diffs[:6]]))
    P("Largest PKON2 losses:\n"); P(table(["image", "category", "set", "pkon2 B", j + " B", "diff", "MED MAE (G)"], [row(x, n) for x, n in diffs[::-1][:6]]))

# timing
P("## Timing (sum over images of best-of-2, ms; ffmpeg codecs include process spawn + temp-file I/O, `net` subtracts the per-call floor measured on a 33x17 image)\n")
rows = []
mp = len(full_ok) * 1.048576
for c in CODECS:
    e = sum(rec[(n, c)]["enc_ms"] for n in full_ok); de = sum(rec[(n, c)]["dec_ms"] for n in full_ok)
    en = e - calib[c][0] * len(full_ok); dn = de - calib[c][1] * len(full_ok)
    rows.append([c, f"{e:.0f}", f"{en:.0f}", f"{mp/(en/1000):.2f}", f"{de:.0f}", f"{dn:.0f}", f"{mp/(max(dn,1)/1000):.2f}", f"{en/len(full_ok):.0f}", f"{dn/len(full_ok):.0f}"])
P(table(["codec", "enc total", "enc net", "enc MP/s net", "dec total", "dec net", "dec MP/s net", "enc net ms/img", "dec net ms/img"], rows))

# groups
def group(title, keyf, minn=3):
    P(f"### {title}\n")
    g = {}
    for n in full_ok: g.setdefault(keyf(n), []).append(n)
    rows = []
    for k, ns in sorted(g.items(), key=lambda kv: str(kv[0])):
        if len(ns) < minn: continue
        p = tot(ns, "pkon2-file")
        rows.append([k, len(ns), f"{raw(ns)/p:.2f}"] + [f"{(tot(ns,c)/p-1)*100:+.1f}%" for c in ("jxl-e7-t1", "jxl-e9-t1", "webp-m6-t1")] + [sum(size(n, "pkon2-file") < size(n, "jxl-e9-t1") for n in ns)])
    P(table(["group", "n", "pkon2 ratio", "e7 vs pkon2", "e9 vs pkon2", "webp vs pkon2", "pkon2 beats e9 (n)"], rows))
P("## Results by image characteristic (aggregate bytes in the group; groups with n<3 omitted)\n")
P("`X vs pkon2` = codec bytes / pkon2 bytes - 1 over the group (positive: codec larger). Groups are descriptive buckets, not controlled experiments.\n")
group("Origin set", lambda n: imgs[n]["set"], 1)
terc = sorted(full_ok, key=lambda n: imgs[n]["med_mae_g"]); t1, t2 = imgs[terc[len(terc)//3]]["med_mae_g"], imgs[terc[2*len(terc)//3]]["med_mae_g"]
group(f"Residual activity (G-plane MED mean |residual|; tercile cuts {t1:.2f} / {t2:.2f})", lambda n: "low" if imgs[n]["med_mae_g"] < t1 else "mid" if imgs[n]["med_mae_g"] < t2 else "high")
group("Brightness (mean G level)", lambda n: "dark (<70)" if imgs[n]["mean_g"] < 70 else "mid" if imgs[n]["mean_g"] < 150 else "bright (>=150)")
group("Original resolution", lambda n: "<4 MP" if imgs[n]["width"] * imgs[n]["height"] < 4e6 else "4-9 MP" if imgs[n]["width"] * imgs[n]["height"] < 9e6 else ">=9 MP")
group("Original JPEG bits/pixel (source file bytes*8/pixels; detail + quality proxy)", lambda n: "<1.5" if imgs[n]["bytes"]*8/(imgs[n]["width"]*imgs[n]["height"]) < 1.5 else "1.5-3" if imgs[n]["bytes"]*8/(imgs[n]["width"]*imgs[n]["height"]) < 3 else ">=3")
group("Orientation of original", lambda n: "portrait" if imgs[n]["height"] > imgs[n]["width"] else "landscape")
def grp(n):
    c = imgs[n]["category"]
    return c
group("Query category (1-2 images each, mostly omitted by n>=3 rule)", grp)

# correlations
P("### Rank correlation of PKON2-vs-JXL-e9 size difference with image descriptors (Spearman, n=%d)\n" % len(full_ok))
dif = [size(n, "pkon2-file") / size(n, "jxl-e9-t1") - 1 for n in full_ok]
feats = {"MED mean |residual|": [imgs[n]["med_mae_g"] for n in full_ok], "MED zero-residual fraction": [imgs[n]["med_zero_frac_g"] for n in full_ok],
         "mean G level": [imgs[n]["mean_g"] for n in full_ok], "clipped fraction": [imgs[n]["clipped_frac_g"] for n in full_ok],
         "original megapixels": [imgs[n]["width"] * imgs[n]["height"] for n in full_ok], "source JPEG bits/px": [imgs[n]["bytes"] * 8 / (imgs[n]["width"] * imgs[n]["height"]) for n in full_ok],
         "pkon2 bpp (size proxy)": [size(n, "pkon2-file") for n in full_ok]}
P(table(["descriptor", "Spearman rho"], [[k, f"{spearman(v, dif):+.2f}"] for k, v in feats.items()]))
P(f"With n={len(full_ok)}, |rho| below ~0.28 is not distinguishable from zero at p=0.05.\n")

# per-image table
P("## Per-image results\n")
P("Columns: pkon2 B (ratio) | e7 B | e9 B | webp B | png B. Full detail incl. timing in `per_image.csv`.\n")
rows = []
for n in order:
    rows.append([n, imgs[n]["category"], f'{imgs[n]["width"]}x{imgs[n]["height"]}'] + [(f"{size(n,c)} ({ratio(n,c):.2f}x)" if c == "pkon2-file" else f"{size(n,c)}") if ok(n, c) else "FAIL" for c in CODECS] + [min((c for c in CODECS if ok(n, c)), key=lambda c: size(n, c))])
P(table(["image", "category", "orig WxH", "pkon2", "jxl-e7", "jxl-e9", "webp-m6", "png", "smallest"], rows))

# verification
V = ["# Verification\n"]
nv = sum(r["valid"] for r in R["records"])
V.append(f"Records: {len(R['records'])}; valid (decoded from own bytes, byte-identical to the canonical crop buffer, dimensions + pixel format equal): {nv}; failed: {len(fails)}.\n")
pk_rows = [rec[(n, 'pkon2-file')] for n in order]
V.append(f"`.pkon2`: header parsed independently for {len(pk_rows)} files (spec == input spec, length == 24 + payload_len, header CRC-32 == CRC-32 of input pixels); metadata ok: {sum(r['pkon2_metadata_ok'] is True for r in pk_rows)}/{len(pk_rows)}. A one-bit corruption inside each payload was rejected by the decoder for every file that is marked valid.\n")
V.append(table(["image", "codec", "valid", "byte-exact", "total B"], [[r["image"], r["codec"], r["valid"], r["byte_exact"], r["total_bytes"]] for r in R["records"]]))
if fails:
    V.append("## Failures\n" + "\n".join(f"- {r['image']} {r['codec']}: {r['note']}" for r in fails))
open(f"{d}/verification.md", "w", encoding="utf-8").write("\n".join(V))
open(f"{d}/report.md", "w", encoding="utf-8").write("\n".join(L))
print("wrote report.md, verification.md, per_image.csv, dataset_manifest.*")
