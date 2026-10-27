"""Fetch the PKON2 milestone-1 probe set from Wikimedia Commons (CC0 only), no hand-picking.

Rule (fixed before looking at results): for each query, take the first N search hits (Commons
search order) that are CC0, camera-EXIF JPEG (or PNG for screen-like classes), JPEG/PNG, 1.5-16 MP, <= 12 MB. Originals only, never thumbnails
(a thumbnail is a resampled image and would have different statistics).
Writes data/<name>.<ext> and data/manifest.csv. Re-running skips files already present.
"""
import csv, json, os, sys, time, urllib.parse, urllib.request

sys.stdout.reconfigure(encoding="utf-8")
UA = {"User-Agent": "nanoscope-research/0.1 (lossless compression research; varmanvishnu96@gmail.com)"}
API = "https://commons.wikimedia.org/w/api.php"
OUT = os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "data")
PER_QUERY = 2
# category label -> search terms
QUERIES = {
    "outdoor":  "mountain hiking -painting",
    "indoor":   "living room -MET -museum -painting",
    "portrait": "portrait person",
    "pet":      "dog",
    "pet2":     "cat",
    "food":     "food dish",
    "foliage":  "foliage leaves",
    "night":    "night",
    "blurry":   "blurry",
    "noisy":    "noisy high iso",
    "text":     "sign text",
    "document": "receipt document",
    "screen":   "screenshot",
}

def get(url):
    for i in range(4):
        try:
            with urllib.request.urlopen(urllib.request.Request(url, headers=UA), timeout=60) as r:
                return r.read()
        except Exception as e:
            print("retry", url[:80], e, file=sys.stderr); time.sleep(3 * (i + 1))
    raise SystemExit("fetch failed: " + url)

def search(term):
    q = {"action": "query", "format": "json", "generator": "search", "gsrnamespace": 6,
         "gsrsearch": f"incategory:CC-Zero {term}", "gsrlimit": 40,
         "prop": "imageinfo", "iiprop": "url|size|mime|extmetadata|metadata"}
    d = json.loads(get(API + "?" + urllib.parse.urlencode(q)))
    pages = sorted(d.get("query", {}).get("pages", {}).values(), key=lambda p: p.get("index", 0))
    return [p for p in pages if p.get("imageinfo")]

os.makedirs(OUT, exist_ok=True)
rows, seen = [], set()
for cat, term in QUERIES.items():
    n = 0
    for p in search(term):
        ii = p["imageinfo"][0]
        mp = ii["width"] * ii["height"] / 1e6
        ext = {"image/jpeg": "jpg", "image/png": "png"}.get(ii["mime"])
        exif = {m["name"]: m["value"] for m in (ii.get("metadata") or []) if isinstance(m.get("value"), str)}
        camera = (exif.get("Make", "") + " " + exif.get("Model", "")).strip()
        # JPEGs must carry camera EXIF (excludes scans/paintings/renders); PNGs only for screen/blurry classes.
        if ext == "jpg" and not exif.get("Model"):
            continue
        if ext == "png" and cat not in ("screen", "blurry", "document", "text"):
            continue
        if not ext or not (1.5 <= mp <= 16) or ii["size"] > 12_000_000 or ii["url"] in seen:
            continue
        seen.add(ii["url"])
        name = f"{cat}_{n}"
        path = os.path.join(OUT, f"{name}.{ext}")
        if not os.path.exists(path):
            open(path, "wb").write(get(ii["url"])); time.sleep(1)
        md = ii.get("extmetadata", {})
        rows.append(dict(name=name, category=cat, file=os.path.basename(path), source_format=ii["mime"],
                         width=ii["width"], height=ii["height"], bytes=ii["size"], url=ii["url"],
                         title=p["title"], camera=camera, license=md.get("LicenseShortName", {}).get("value", ""),
                         artist=md.get("Artist", {}).get("value", "")[:80].replace("\n", " ")))
        print(name, p["title"], ii["width"], ii["height"], ii["mime"])
        n += 1
        if n >= PER_QUERY: break
with open(os.path.join(OUT, "manifest.csv"), "w", newline="", encoding="utf-8") as f:
    w = csv.DictWriter(f, fieldnames=list(rows[0])); w.writeheader(); w.writerows(rows)
json.dump(rows, open(os.path.join(OUT, "manifest.json"), "w", encoding="utf-8"), indent=1, ensure_ascii=False)
