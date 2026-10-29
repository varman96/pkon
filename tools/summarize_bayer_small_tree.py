"""Summarize results/20261029-lisa-bayer-plane-small-tree/results.json into summary.md + per_image.csv.
Usage: python -I tools/summarize_bayer_small_tree.py <results_dir>"""
import csv, json, statistics, sys

d = sys.argv[1]
rows = json.load(open(f"{d}/results.json"))
V = ["table128_prod", "planar_table128", "tree128", "tree64", "tree32"]
b = lambda r, v: r["variants"][v]["bytes"]
pct = lambda a, base: (a / base - 1) * 100

with open(f"{d}/per_image.csv", "w", newline="") as f:
    w = csv.writer(f)
    w.writerow(["id", "split", "layout", "bit_depth", "samples"] + [f"{v}_bytes" for v in V] +
               ["planar_vs_prod_pct", "tree128_vs_table_pct", "tree64_vs_table_pct", "tree32_vs_table_pct",
                "tree64_vs_tree128_pct", "tree32_vs_tree128_pct"])
    for r in rows:
        t = b(r, "table128_prod")
        w.writerow([r["id"], r["split"], r["layout"], r["bit_depth"], r["samples"]] + [b(r, v) for v in V] +
                   [f"{pct(b(r, 'planar_table128'), t):+.3f}", f"{pct(b(r, 'tree128'), t):+.3f}", f"{pct(b(r, 'tree64'), t):+.3f}",
                    f"{pct(b(r, 'tree32'), t):+.3f}", f"{pct(b(r, 'tree64'), b(r, 'tree128')):+.3f}",
                    f"{pct(b(r, 'tree32'), b(r, 'tree128')):+.3f}"])

out = []
out.append(f"all_ok = {all(r['ok'] for r in rows)}  ({len(rows)} images)\n")
groups = [("ALL 25", rows)] + [(s, [r for r in rows if r["split"] == s]) for s in ("train", "val", "test")]
out.append("## Aggregate payload bytes (Stage 4 payload)\n")
out.append("| group | n | table128 (prod) | planar+table128 | tree128 | tree64 | tree32 |\n|---|---|---|---|---|---|---|")
for name, g in groups:
    tot = {v: sum(b(r, v) for r in g) for v in V}
    cells = [f"{tot['table128_prod']:,}"] + [f"{tot[v]:,} ({pct(tot[v], tot['table128_prod']):+.3f}%)" for v in V[1:]]
    out.append(f"| {name} | {len(g)} | " + " | ".join(cells) + " |")
out.append("\n## Tree variants vs tree128 (aggregate)\n")
out.append("| group | tree64 vs tree128 | tree32 vs tree128 |\n|---|---|---|")
for name, g in groups:
    t = sum(b(r, "tree128") for r in g)
    out.append(f"| {name} | {pct(sum(b(r,'tree64') for r in g), t):+.3f}% | {pct(sum(b(r,'tree32') for r in g), t):+.3f}% |")
out.append("\n## Per image (bytes; % vs production table128)\n")
out.append("| image | split | bits | prod table128 | planar | tree128 | tree64 | tree32 |\n|---|---|---|---|---|---|---|---|")
for r in rows:
    t = b(r, "table128_prod")
    out.append(f"| {r['id']} | {r['split']} | {r['bit_depth']} | {t:,} | " +
               " | ".join(f"{b(r, v):,} ({pct(b(r, v), t):+.2f}%)" for v in V[1:]) + " |")
bay = [r for r in rows if r["variants"]["planar_table128"]["applies"]]
out.append(f"\nPlanar Stage 1 applies to {len(bay)}/{len(rows)} images (Bayer metadata); the others are byte-identical to production by construction.")
out.append(f"Planar smaller on {sum(b(r,'planar_table128') < b(r,'table128_prod') for r in bay)}/{len(bay)} Bayer images; "
           f"tree64 smaller than table on {sum(b(r,'tree64') < b(r,'table128_prod') for r in rows)}/25, tree32 on {sum(b(r,'tree32') < b(r,'table128_prod') for r in rows)}/25.")

out.append("\n## Context occupancy (per image, then pooled)\n")
out.append("Per-image visit counts. bottom50 = share of visits in the least-visited half of the leaves. low = leaves with < 2,048 visits in that image.\n")
for v in ["table128_prod", "tree128", "tree64", "tree32", "planar_table128"]:
    ocs = [r["variants"][v]["occupancy"] for r in rows]
    b50 = [o["bottom50_share"] for o in ocs]
    lowsh = [o["visits_share_in_leaves_lt_2048"] for o in ocs]
    lown = [o["leaves_lt_2048"] for o in ocs]
    out.append(f"- **{v}** ({ocs[0]['leaves']} contexts): bottom-50% share median {statistics.median(b50)*100:.2f}%, "
               f"<5% on {sum(x < 0.05 for x in b50)}/25 images; leaves with <2,048 visits per image: median {statistics.median(lown):.0f}, max {max(lown)}; "
               f"visit share in those leaves: median {statistics.median(lowsh)*100:.3f}%, max {max(lowsh)*100:.3f}%; unused contexts (0 visits) median {statistics.median(o['leaves']-o['used'] for o in ocs):.0f}")
out.append("\nPooled per split (sum of per-image visit vectors; bottom-50% share of pooled visits):\n")
out.append("| variant | split | bottom-50% share | min leaf visits | max leaf visits |\n|---|---|---|---|---|")
for v in ["table128_prod", "tree128", "tree64", "tree32"]:
    for s in ("train", "val", "test"):
        g = [r for r in rows if r["split"] == s]
        k = len(g[0]["variants"][v]["occupancy"]["visits"])
        pooled = [sum(r["variants"][v]["occupancy"]["visits"][i] for r in g) for i in range(k)]
        srt = sorted(pooled)
        out.append(f"| {v} | {s} | {sum(srt[:k//2])/sum(srt)*100:.2f}% | {srt[0]:,} | {srt[-1]:,} |")
open(f"{d}/summary.md", "w").write("\n".join(out) + "\n")
print("\n".join(out))
