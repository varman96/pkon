"""Select the 25-image diversity benchmark set (data/dataset25.json) from the existing 23-image corpus (data/manifest.json)
plus the 29 extra images (data/manifest_new50.csv). Selection is by diversity of content/statistics, not by codec results.
Nothing in data/ is modified except the new dataset25.json."""
import csv, json, os
D = os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "..", "data")
old = {r["name"]: dict(r, set="retained23") for r in json.load(open(f"{D}/manifest.json", encoding="utf-8"))}
new = {r["name"]: dict(r, set="added") for r in csv.DictReader(open(f"{D}/manifest_new50.csv", encoding="utf-8"))}
PICK = [  # (name, why)
 ("outdoor_0", "mountain landscape, high-frequency detail"),
 ("indoor_0", "bright indoor scene, smooth surfaces"),
 ("portrait_0", "people: portrait"),
 ("pet_0", "animal: fur detail"),
 ("pet2_1", "animal: heavily over-exposed regions"),
 ("food_0", "mixed detail, close range"),
 ("foliage_0", "dense foliage, fine texture"),
 ("night_0", "low-light, sensor noise"),
 ("night_1", "low-light, comparatively clean"),
 ("blurry_1", "PNG origin, RGBA, defocus blur"),
 ("screen_0", "PNG origin, RGBA, UI screenshot (graphic content)"),
 ("text_1", "sharp edges: printed text"),
 ("document_1", "document: flat paper, clipped highlights"),
 ("clouds_n0", "sky / clouds"),
 ("skyline_n0", "sky + architecture, very smooth"),
 ("sunset_n0", "dusk sky, colour gradients"),
 ("snow_n0", "snow, bright low-contrast texture"),
 ("path_n0", "extreme fine texture (gravel/ground)"),
 ("building_n0", "architecture: facade"),
 ("church_n0", "architecture: detailed stonework"),
 ("child_n0", "people: smooth skin, shallow depth of field"),
 ("woman_n0", "people: high-key portrait"),
 ("bird_n0", "animal: bokeh / background blur"),
 ("waterfall_n0", "moving water, mixed detail"),
 ("lake_n0", "water + reflections"),
]
rows = []
for n, why in PICK:
    r = dict(old.get(n) or new[n]); r["url"] = r["url"].split("?")[0]; r["selection_reason"] = why; rows.append(r)
assert len(rows) == 25 and len({r["name"] for r in rows}) == 25
json.dump(rows, open(f"{D}/dataset25.json", "w", encoding="utf-8"), indent=1, ensure_ascii=False)
print(len(rows), "images;", sum(r["set"] == "retained23" for r in rows), "from the 23-image corpus,", sum(r["set"] == "added" for r in rows), "added")
