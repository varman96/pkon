"""Merge the retained corpus images with the new ones into data/dataset50.json (the 50-image manifest the benchmark reads).
Excludes the two screenshots (screen_0/1) from the original 23. Also reports the closest near-duplicate pair by 16x16 average hash."""
import csv, json, os
from PIL import Image
D = os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "data")
old = [dict(r, set="retained") for r in csv.DictReader(open(f"{D}/manifest.csv", encoding="utf-8")) if r["category"] != "screen"]
new = [dict(r, set="new") for r in csv.DictReader(open(f"{D}/manifest_new50.csv", encoding="utf-8"))]
rows = old + new
for r in rows:
    r["url"] = r["url"].split("?")[0]
json.dump(rows, open(f"{D}/dataset50.json", "w", encoding="utf-8"), indent=1, ensure_ascii=False)

def ah(p):
    im = Image.open(f"{D}/{p}").convert("L").resize((16, 16))
    px = list(im.getdata()); m = sum(px) / 256
    return [int(v > m) for v in px]

hs = [ah(r["file"]) for r in rows]
pairs = sorted((sum(a != b for a, b in zip(hs[i], hs[j])), rows[i]["name"], rows[j]["name"]) for i in range(len(rows)) for j in range(i))
print(len(rows), "images;", len(old), "retained,", len(new), "new; closest aHash pairs (hamming/256):", pairs[:3])
