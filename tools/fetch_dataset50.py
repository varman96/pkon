"""Extend the PKON2 corpus towards 50 distinct consumer photos (Wikimedia Commons, CC0, camera-EXIF JPEG only).

Same acceptance rule as fetch_dataset.py (1.5-16 MP, <= 12 MB, originals, camera Model EXIF present), applied in a
fixed query order; the first PER_QUERY accepted hits per query are taken until NEW_TARGET images are collected.
Skips any url/title already in data/manifest.csv. Writes data/new50/<name>.jpg and data/manifest_new50.csv.
Screenshots/synthetic content are not queried.
"""
import csv, json, os, sys, time, urllib.parse, urllib.request

sys.stdout.reconfigure(encoding="utf-8")
UA = {"User-Agent": "nanoscope-research/0.1 (lossless compression research; varmanvishnu96@gmail.com)"}
API = "https://commons.wikimedia.org/w/api.php"
HERE = os.path.dirname(os.path.abspath(__file__))
DATA = os.path.join(HERE, "..", "data")
OUT = os.path.join(DATA, "new50")
PER_QUERY, NEW_TARGET = 1, 29  # one per query: no near-duplicate pairs from the same search
QUERIES = [("beach", "beach sea"), ("street", "city street"), ("building", "building facade"), ("forest", "forest trees"),
    ("flower", "flower garden"), ("sunset", "sunset sky"), ("snow", "snow winter"), ("bird", "bird"),
    ("bridge", "bridge"), ("kitchen", "kitchen interior"), ("group", "group of people"),
    ("child", "child playing"), ("market", "market stall"), ("lake", "lake reflection"), ("macro", "insect macro"),
    ("church", "church interior"), ("citynight", "city lights evening"), ("fruit", "fruit vegetables"),
    ("boat", "boat harbour"), ("desert", "desert dunes"), ("stairs", "staircase"),
    ("woman", "portrait woman"), ("oldman", "elderly man"), ("waterfall", "waterfall"), ("peak", "mountain peak"),
    ("restaurant", "restaurant interior"), ("graffiti", "graffiti wall"), ("station", "train station"), ("bark", "tree bark texture"),
    ("clouds", "clouds"), ("coast", "rocky coast"), ("skyline", "skyline"), ("path", "garden path"), ("crowd", "festival crowd"),
    ("fog", "morning fog"), ("cafe", "cafe table"), ("river", "river valley"), ("alley", "old town alley"), ("bicycle", "bicycle"),
    ("lighthouse", "lighthouse"), ("temple", "temple"), ("farm", "farm field")]

def get(url):
    for i in range(4):
        try:
            with urllib.request.urlopen(urllib.request.Request(url, headers=UA), timeout=90) as r:
                return r.read()
        except Exception as e:
            print("retry", url[:80], e, file=sys.stderr); time.sleep(4 * (i + 1))
    raise SystemExit("fetch failed: " + url)

def search(term):
    q = {"action": "query", "format": "json", "generator": "search", "gsrnamespace": 6,
         "gsrsearch": f"incategory:CC-Zero {term} -painting -MET -museum -sculpture -statue -Gogh -drawing", "gsrlimit": 40,
         "prop": "imageinfo", "iiprop": "url|size|mime|extmetadata|metadata"}
    d = json.loads(get(API + "?" + urllib.parse.urlencode(q)))
    pages = sorted(d.get("query", {}).get("pages", {}).values(), key=lambda p: p.get("index", 0))
    return [p for p in pages if p.get("imageinfo")]

os.makedirs(OUT, exist_ok=True)
old = list(csv.DictReader(open(os.path.join(DATA, "manifest.csv"), encoding="utf-8")))
seen = {r["url"].split("?")[0] for r in old}; titles = {r["title"] for r in old}
rows = []
for cat, term in QUERIES:
    if len(rows) >= NEW_TARGET: break
    n = 0
    for p in search(term):
        ii = p["imageinfo"][0]; mp = ii["width"] * ii["height"] / 1e6
        exif = {m["name"]: m["value"] for m in (ii.get("metadata") or []) if isinstance(m.get("value"), str)}
        camera = (exif.get("Make", "") + " " + exif.get("Model", "")).strip()
        u = ii["url"].split("?")[0]
        if ii["mime"] != "image/jpeg" or not exif.get("Model") or not (1.5 <= mp <= 16) or ii["size"] > 12_000_000: continue
        if u in seen or p["title"] in titles: continue
        seen.add(u)
        name = f"{cat}_n{n}"
        path = os.path.join(OUT, name + ".jpg")
        if not os.path.exists(path):
            open(path, "wb").write(get(ii["url"])); time.sleep(1)
        md = ii.get("extmetadata", {})
        rows.append(dict(name=name, category=cat, file="new50/" + name + ".jpg", source_format=ii["mime"], width=ii["width"],
                         height=ii["height"], bytes=ii["size"], url=ii["url"], title=p["title"], camera=camera,
                         license=md.get("LicenseShortName", {}).get("value", ""),
                         artist=md.get("Artist", {}).get("value", "")[:80].replace("\n", " ")))
        print(len(rows), name, p["title"], ii["width"], ii["height"], camera)
        n += 1
        if n >= PER_QUERY or len(rows) >= NEW_TARGET: break
with open(os.path.join(DATA, "manifest_new50.csv"), "w", newline="", encoding="utf-8") as f:
    w = csv.DictWriter(f, fieldnames=list(rows[0])); w.writeheader(); w.writerows(rows)
print("new images:", len(rows))
