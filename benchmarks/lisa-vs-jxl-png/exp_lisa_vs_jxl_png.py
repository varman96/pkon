"""External-codec side of the LISA vs JPEG XL vs PNG benchmark + report generation. Run with `python -I`.

Usage (from the repo root): python -I benchmarks/lisa-vs-jxl-png/exp_lisa_vs_jxl_png.py <exp_dir> [reps=3]   (exp_dir = benchmarks/lisa-vs-jxl-png)
Reads <exp_dir>/source_manifest.json, split.json, lisa_raw.json (from exp_lisa_vs_jxl_png.rs); writes ext_raw.json,
results.json, per_image.csv, report_tables.md. Source = frozen canonical u16-LE rasters (sha256-checked against the manifest).

Timing boundary (JXL/PNG): wall time of one `ffmpeg` process, raw sample bytes already in memory -> piped to stdin ->
compressed bytes read back from stdout (and the reverse for decode). Process start-up cost is measured separately and reported.
"""
import csv, hashlib, json, statistics, subprocess, sys, time
import numpy as np

EXP = sys.argv[1]
REPS = int(sys.argv[2]) if len(sys.argv) > 2 else 3
FF = "ffmpeg"
JXL_EFFORT = 7
CODECS = {
    "jxl": dict(
        enc=lambda w, h: [FF, "-v", "error", "-f", "rawvideo", "-pix_fmt", "gray16le", "-s", f"{w}x{h}", "-i", "pipe:0",
                          "-frames:v", "1", "-c:v", "libjxl", "-distance", "0", "-effort", str(JXL_EFFORT), "-modular", "1",
                          "-f", "image2pipe", "pipe:1"],
        dec=lambda w, h: [FF, "-v", "error", "-i", "pipe:0", "-frames:v", "1", "-f", "rawvideo", "-pix_fmt", "gray16le", "pipe:1"],
        order="<u2"),
    "png": dict(
        enc=lambda w, h: [FF, "-v", "error", "-f", "rawvideo", "-pix_fmt", "gray16be", "-s", f"{w}x{h}", "-i", "pipe:0",
                          "-frames:v", "1", "-c:v", "png", "-compression_level", "9", "-pred", "mixed",
                          "-f", "image2pipe", "pipe:1"],
        dec=lambda w, h: [FF, "-v", "error", "-i", "pipe:0", "-frames:v", "1", "-f", "rawvideo", "-pix_fmt", "gray16be", "pipe:1"],
        order=">u2"),
}


def run(cmd, data):
    t = time.perf_counter()
    p = subprocess.run(cmd, input=data, capture_output=True)
    dt = time.perf_counter() - t
    if p.returncode != 0:
        raise RuntimeError(p.stderr.decode()[-500:])
    return p.stdout, dt


def probe(blob):
    p = subprocess.run(["ffprobe", "-v", "error", "-select_streams", "v:0", "-show_entries",
                        "stream=codec_name,width,height,pix_fmt", "-of", "json", "-i", "pipe:0"], input=blob, capture_output=True)
    return json.loads(p.stdout)["streams"][0]


man = json.load(open(f"{EXP}/source_manifest.json"))
split = json.load(open(f"{EXP}/split.json"))
lisa = {r["id"]: r for r in json.load(open(f"{EXP}/lisa_raw.json"))}
ver = subprocess.run([FF, "-version"], capture_output=True, text=True).stdout.splitlines()[0]
ext = {}

# process start-up overhead calibration (16x16 image)
overhead = {}
for name, c in CODECS.items():
    tiny = np.zeros((16, 16), c["order"]).tobytes()
    blob, _ = run(c["enc"](16, 16), tiny)
    e = [run(c["enc"](16, 16), tiny)[1] for _ in range(5)]
    d = [run(c["dec"](16, 16), blob)[1] for _ in range(5)]
    overhead[name] = {"enc_median_s": statistics.median(e), "dec_median_s": statistics.median(d)}

for m in man:
    id_, w, h = m["id"], m["width"], m["height"]
    raw = open(m["canon_path"].replace("\\", "/"), "rb").read()
    sha_ok = hashlib.sha256(raw).hexdigest() == m["canon_sha256"] and len(raw) == w * h * 2
    src = np.frombuffer(raw, "<u2").reshape(h, w)
    for name, c in CODECS.items():
        inp = raw if c["order"] == "<u2" else src.astype(">u2").tobytes()
        run(c["enc"](w, h), inp)  # warm-up
        encs, digests, blob = [], set(), None
        for _ in range(REPS):
            blob, dt = run(c["enc"](w, h), inp)
            encs.append(dt)
            digests.add(hashlib.sha256(blob).hexdigest())
        run(c["dec"](w, h), blob)  # warm-up
        decs = []
        for _ in range(REPS):
            out, dt = run(c["dec"](w, h), blob)
            decs.append(dt)
        dec_arr = np.frombuffer(out, c["order"])
        pr = probe(blob)
        mism = int(np.count_nonzero(dec_arr != src.reshape(-1))) if dec_arr.size == w * h else w * h
        ext.setdefault(id_, {})[name] = {
            "bytes": len(blob), "deterministic": len(digests) == 1, "mismatches": mism, "probe": pr,
            "dims_ok": (pr["width"], pr["height"]) == (w, h), "decoded_samples": int(dec_arr.size), "canon_sha_ok": sha_ok,
            "enc_s": encs, "dec_s": decs, "enc_median_s": statistics.median(encs), "dec_median_s": statistics.median(decs)}
        print(id_, name, len(blob), f"enc {statistics.median(encs):.2f}s dec {statistics.median(decs):.2f}s mism={mism} {pr['pix_fmt']}", flush=True)
json.dump({"ffmpeg": ver, "overhead": overhead, "per_image": ext}, open(f"{EXP}/ext_raw.json", "w"), indent=1)

# ---- merge + aggregate
rows = []
for m in man:
    i = m["id"]
    px = m["width"] * m["height"]
    L = lisa[i]
    sp = next(s for s in ("train", "val", "test") if i in split[s])
    r = {"id": i, "split": sp, "width": m["width"], "height": m["height"], "bit_depth": m["bit_depth"],
         "sample_min": m["sample_min"], "sample_max": m["sample_max"], "pixels": px, "raw_bytes": px * 2, "layout": L["layout"]}
    for name, d in (("lisa", {"bytes": L["payload_bytes"], "mismatches": L["mismatches"], "enc_median_s": L["enc_median_s"],
                              "dec_median_s": L["dec_median_s"], "deterministic": L["deterministic"]}),
                    ("jxl", ext[i]["jxl"]), ("png", ext[i]["png"])):
        ok = d["mismatches"] == 0 and d["deterministic"] and d.get("dims_ok", True)
        r[name] = {"bytes": d["bytes"], "ratio": px * 2 / d["bytes"], "enc_s": d["enc_median_s"], "dec_s": d["dec_median_s"],
                   "enc_px_s": px / d["enc_median_s"], "enc_mib_s": px * 2 / 2**20 / d["enc_median_s"],
                   "dec_px_s": px / d["dec_median_s"], "dec_mib_s": px * 2 / 2**20 / d["dec_median_s"],
                   "mismatches": d["mismatches"], "deterministic": d["deterministic"], "exact": ok}
    rows.append(r)


def agg(sel):
    px = sum(r["pixels"] for r in sel)
    raw = px * 2
    o = {"n": len(sel), "pixels": px, "raw_bytes": raw}
    for c in ("lisa", "jxl", "png"):
        b = sum(r[c]["bytes"] for r in sel)
        te = sum(r[c]["enc_s"] for r in sel)
        td = sum(r[c]["dec_s"] for r in sel)
        o[c] = {"bytes": b, "ratio": raw / b, "exact_images": sum(r[c]["exact"] for r in sel), "enc_s": te, "dec_s": td,
                "enc_px_s": px / te, "enc_mib_s": raw / 2**20 / te, "dec_px_s": px / td, "dec_mib_s": raw / 2**20 / td}
    for c in ("jxl", "png"):
        o[f"{c}_vs_lisa_pct"] = (o[c]["bytes"] / o["lisa"]["bytes"] - 1) * 100
    o["png_vs_jxl_pct"] = (o["png"]["bytes"] / o["jxl"]["bytes"] - 1) * 100
    return o


res = {"all25": agg(rows), "heldout8_test": agg([r for r in rows if r["split"] == "test"]), "per_image": rows,
       "ffmpeg": ver, "overhead": overhead, "jxl_effort": JXL_EFFORT, "reps": REPS}
json.dump(res, open(f"{EXP}/results.json", "w"), indent=1)
KEYS = ("bytes", "ratio", "enc_s", "dec_s", "enc_mib_s", "dec_mib_s", "mismatches")
with open(f"{EXP}/per_image.csv", "w", newline="") as f:
    wr = csv.writer(f)
    wr.writerow(["id", "split", "w", "h", "bits", "raw_bytes"] + [f"{c}_{k}" for c in ("lisa", "jxl", "png") for k in KEYS])
    for r in rows:
        wr.writerow([r["id"], r["split"], r["width"], r["height"], r["bit_depth"], r["raw_bytes"]] +
                    [r[c][k] for c in ("lisa", "jxl", "png") for k in KEYS])


def table(a, title):
    s = (f"### {title} ({a['n']} images, {a['pixels']:,} px, {a['raw_bytes']:,} raw bytes)\n\n"
         "| codec | bytes | ratio | vs LISA | enc px/s | enc MiB/s | dec px/s | dec MiB/s | exact |\n|---|---|---|---|---|---|---|---|---|\n")
    for c in ("lisa", "jxl", "png"):
        x = a[c]
        d = "-" if c == "lisa" else f"{a[c + '_vs_lisa_pct']:+.2f}%"
        s += (f"| {c.upper()} | {x['bytes']:,} | {x['ratio']:.4f} | {d} | {x['enc_px_s']:,.0f} | {x['enc_mib_s']:.2f} | "
              f"{x['dec_px_s']:,.0f} | {x['dec_mib_s']:.2f} | {x['exact_images']}/{a['n']} |\n")
    return s + f"\nPNG vs JXL: {a['png_vs_jxl_pct']:+.2f}%\n\n"


md = (f"ffmpeg: `{ver}`; JXL `-c:v libjxl -distance 0 -modular 1 -effort {JXL_EFFORT}`; PNG `-c:v png -compression_level 9 -pred mixed` "
      f"(gray16be). 1 warm-up + {REPS} measured reps, median.\n\n")
md += table(res["all25"], "All 25") + table(res["heldout8_test"], "Held-out 8 (test)")
md += ("## Per image\n\n| id | split | WxH | bits | raw B | LISA B | JXL B | PNG B | LISA enc/dec s | JXL enc/dec s | PNG enc/dec s | exact L/J/P |\n"
       "|---|---|---|---|---|---|---|---|---|---|---|---|\n")
for r in rows:
    md += (f"| {r['id']} | {r['split']} | {r['width']}x{r['height']} | {r['bit_depth']} | {r['raw_bytes']:,} | {r['lisa']['bytes']:,} | "
           f"{r['jxl']['bytes']:,} | {r['png']['bytes']:,} | {r['lisa']['enc_s']:.2f}/{r['lisa']['dec_s']:.2f} | "
           f"{r['jxl']['enc_s']:.2f}/{r['jxl']['dec_s']:.2f} | {r['png']['enc_s']:.2f}/{r['png']['dec_s']:.2f} | "
           f"{int(r['lisa']['exact'])}/{int(r['jxl']['exact'])}/{int(r['png']['exact'])} |\n")
md += f"\nffmpeg process start-up (16x16 image, median of 5): {json.dumps(overhead)}\n"
open(f"{EXP}/report_tables.md", "w").write(md)
print(md)
