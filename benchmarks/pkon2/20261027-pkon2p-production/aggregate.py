import csv, json, statistics, os, sys
D = "results/20261027-pkon2p-production"

def load(name):
    rows, extra = {}, {}
    for r in csv.reader(open(f"{D}/{name}.csv")):
        if r[0] == "image":
            continue
        if r[0].startswith("#total"):
            extra["enc_total"], extra["dec_total"] = float(r[7]), float(r[8])
        elif r[0].startswith("#peak"):
            extra["peak"] = int(r[1]) if r[1] else None
        else:
            rows[r[0]] = dict(file=int(r[1]), payload=int(r[2]), raw=int(r[3]), exact=r[4] == "true", hdr=r[5] == "true", det=r[6] == "true", enc=float(r[7]), dec=float(r[8]), codec=int(r[9]))
    return rows, extra

runs = {k: load(k) for k in ["run_old_a", "run_old_b", "run_new1_a", "run_new1_b", "run_new2_a", "run_new2_b"]}
frozen = {}
for r in csv.DictReader(open("results/20261027-155433Z-bench50/per_image.csv")):
    if r["codec"] == "pkon2-file":
        frozen[r["image"]] = int(r["total_bytes"])
sp = json.load(open("results/learn1-splits.json"))
split = {}
for k, v in (("train_pool", "DEV"), ("val", "VAL"), ("test", "TEST")):
    for n in sp[k]:
        split[n] = v
names = list(frozen.keys())
assert len(names) == 50 and all(n in split for n in names)

def best(prefix, field):
    a, b = runs[prefix + "_a"][0], runs[prefix + "_b"][0]
    return {n: min(a[n][field], b[n][field]) for n in names}

# sizes must be identical between rounds
for p in ["old", "new1", "new2"]:
    a, b = runs[f"run_{p}_a"][0], runs[f"run_{p}_b"][0]
    assert all(a[n]["file"] == b[n]["file"] for n in names), f"{p}: sizes differ between rounds"
old = runs["run_old_a"][0]; n1 = runs["run_new1_a"][0]; n2 = runs["run_new2_a"][0]
old_matches_frozen = all(old[n]["file"] == frozen[n] for n in names)
n1_matches_frozen = all(n1[n]["file"] == frozen[n] for n in names)
all_exact = all(r[n]["exact"] and r[n]["hdr"] and r[n]["det"] for k in runs for r in [runs[k][0]] for n in names)
codec_ok = all(runs[k][0][n]["codec"] == c for k, c in [("run_old_a", 1), ("run_new1_a", 1), ("run_new2_a", 2)] for n in names)

enc_old, dec_old = best("old" if False else "run_old", "enc"), None
def tbest(p, f):
    return {n: min(runs[f"run_{p}_a"][0][n][f], runs[f"run_{p}_b"][0][n][f]) for n in names}
T = {p: (tbest(p, "enc"), tbest(p, "dec")) for p in ["old", "new1", "new2"]}

def agg(sel):
    ns = [n for n in names if sel(n)]
    b = sum(old[n]["file"] for n in ns); a = sum(n2[n]["file"] for n in ns); raw = sum(old[n]["raw"] for n in ns)
    return len(ns), b, a, 100 * (b - a) / b, raw / b, raw / a, sum(1 for n in ns if n2[n]["file"] < old[n]["file"])

md = ["# PKON2P (codec 2) production benchmark: 50 images, complete `.pkon2` files\n"]
md.append("Same decoded pixels, 1024-px centre crops (crop sha256 checked against the frozen bench50 manifest), `pkonfile::encode`/`decode` entry points, every size = 24-byte header + payload. Baseline = the **pre-change production code** (git HEAD 7833411, built separately, codec 1); new = working tree, codec 2. Held-out TEST images (15) are evaluated here for the first time in this investigation; DEV/VAL were used by REL1/REL2.\n")
md.append("## Integrity of the comparison\n")
md.append(f"- pre-change build's file sizes equal the frozen bench50 `pkon2-file` sizes on all 50 images: **{old_matches_frozen}**")
md.append(f"- working-tree codec 1 sizes equal the frozen baseline on all 50 images (existing format output unchanged): **{n1_matches_frozen}**")
md.append(f"- every image, every run (6 runs): exact pixel round trip from file bytes, header spec/CRC/length verified, re-encode deterministic: **{all_exact}**; header codec byte as expected: **{codec_ok}**\n")
md.append("## Aggregate\n")
md.append("| set | images | baseline bytes | PKON2P bytes | change | ratio before | ratio after | images smaller |\n|---|---|---|---|---|---|---|---|")
for label, sel in [("ALL", lambda n: True), ("DEV", lambda n: split[n] == "DEV"), ("VAL", lambda n: split[n] == "VAL"), ("TEST (held out)", lambda n: split[n] == "TEST")]:
    c, b, a, pc, r0, r1, imp = agg(sel)
    md.append(f"| {label} | {c} | {b} | {a} | -{pc:.3f}% | {r0:.4f} | {r1:.4f} | {imp}/{c} |")
ratios0 = [old[n]["raw"] / old[n]["file"] for n in names]; ratios1 = [n2[n]["raw"] / n2[n]["file"] for n in names]
md.append(f"\nMean per-image ratio {statistics.mean(ratios0):.4f} -> {statistics.mean(ratios1):.4f}; median {statistics.median(ratios0):.4f} -> {statistics.median(ratios1):.4f}.\n")
deltas = {n: 100 * (old[n]["file"] - n2[n]["file"]) / old[n]["file"] for n in names}
worst = min(deltas, key=deltas.get); bestn = max(deltas, key=deltas.get)
md.append(f"Per-image saving: min {deltas[worst]:+.3f}% ({worst}), median {statistics.median(deltas.values()):+.3f}%, max {deltas[bestn]:+.3f}% ({bestn}). Images that got larger: {[n for n in names if deltas[n] <= 0] or 'none'}.\n")
md.append("## Per image\n")
md.append("| image | split | baseline B | PKON2P B | saving | ratio after | enc ms old -> new | dec ms old -> new |\n|---|---|---|---|---|---|---|---|")
for n in sorted(names, key=lambda n: deltas[n]):
    md.append(f"| {n} | {split[n]} | {old[n]['file']} | {n2[n]['file']} | {deltas[n]:+.3f}% | {n2[n]['raw']/n2[n]['file']:.3f} | {T['old'][0][n]:.0f} -> {T['new2'][0][n]:.0f} | {T['old'][1][n]:.0f} -> {T['new2'][1][n]:.0f} |")
md.append("\n## Previously difficult images (fur / foliage / text / texture)\n")
md.append("| image | baseline B | PKON2P B | saving |\n|---|---|---|---|")
for n in ["foliage_0", "foliage_1", "forest_n0", "text_0", "text_1", "document_0", "document_1", "pet_0", "pet_1", "pet2_0", "pet2_1", "river_n0", "bird_n0", "graffiti_n0", "night_0"]:
    if n in names:
        md.append(f"| {n} | {old[n]['file']} | {n2[n]['file']} | {deltas[n]:+.3f}% |")
md.append("\n## Time and memory (single thread, best of 2 repetitions x 2 passes per image, sequential on one machine)\n")
def tot(p, i): return sum(T[p][i].values())
md.append("| build | encode total ms | decode total ms | enc vs old | dec vs old | peak working set (best of 2 runs) |\n|---|---|---|---|---|---|")
for p, label in [("old", "pre-change production (codec 1)"), ("new1", "working tree, codec 1"), ("new2", "working tree, codec 2 (PKON2P)")]:
    pk = [runs[f"run_{p}_{x}"][1].get("peak") for x in "ab"]
    pk = [v for v in pk if v]
    md.append(f"| {label} | {tot(p,0):.0f} | {tot(p,1):.0f} | x{tot(p,0)/tot('old',0):.3f} | x{tot(p,1)/tot('old',1):.3f} | {min(pk)/1e6:.1f} MB |" if pk else f"| {label} | {tot(p,0):.0f} | {tot(p,1):.0f} | x{tot(p,0)/tot('old',0):.3f} | x{tot(p,1)/tot('old',1):.3f} | n/a |")
raw_tot = sum(old[n]["raw"] for n in names)
md.append(f"\nThroughput (codec 2): encode {raw_tot/1e6/(tot('new2',0)/1e3):.2f} MB/s, decode {raw_tot/1e6/(tot('new2',1)/1e3):.2f} MB/s of raw pixels (old: {raw_tot/1e6/(tot('old',0)/1e3):.2f} / {raw_tot/1e6/(tot('old',1)/1e3):.2f}). Per-image time ratio spread new2/old (enc): min {min(T['new2'][0][n]/T['old'][0][n] for n in names):.2f}, max {max(T['new2'][0][n]/T['old'][0][n] for n in names):.2f}.")
md.append("Model state: the mixer weight tables grow from [i32;4] to [i32;5] per (plane,node) and one more 96-context probability table per plane is allocated (3 planes x 96 x 27 x 4 B = 31 KB): negligible against the decoded image buffers.\n")
open(f"{D}/report_benchmark.md", "w").write("\n".join(md))
with open(f"{D}/per_image.csv", "w", newline="") as f:
    w = csv.writer(f)
    w.writerow(["image", "split", "raw_bytes", "baseline_file_bytes", "pkon2p_file_bytes", "saving_pct", "enc_ms_old", "enc_ms_new", "dec_ms_old", "dec_ms_new", "exact"])
    for n in names:
        w.writerow([n, split[n], old[n]["raw"], old[n]["file"], n2[n]["file"], f"{deltas[n]:.4f}", T["old"][0][n], T["new2"][0][n], T["old"][1][n], T["new2"][1][n], n2[n]["exact"]])
print("\n".join(md[:30]))
