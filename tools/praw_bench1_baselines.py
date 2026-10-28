"""PRAW benchmark 1, step 3: PNG / JPEG XL / Zstandard on the identical canonical sample bytes, each decoded and
compared sample-for-sample. Single-threaded, one fixed configuration per codec.
Run: python -I tools/praw_bench1_baselines.py <dataset_manifest.json> <out.json> [--only id]"""
import json, sys, time, os
import numpy as np
import imagecodecs as ic

ROOT = os.path.join(os.path.dirname(os.path.abspath(__file__)), "..")
CFG = {
    "png": dict(level=9),                                   # zlib 9, libpng adaptive filtering, 16-bit gray
    "jxl": dict(lossless=True, effort=7, numthreads=1),     # modular lossless, libjxl default effort
    "zstd": dict(level=19),                                 # on raw LE16 canonical bytes, no shuffle
}


def run(name, a, raw):
    t = time.perf_counter()
    if name == "png":
        enc = ic.png_encode(a, **CFG[name])
    elif name == "jxl":
        enc = ic.jpegxl_encode(a, **CFG[name])
    else:
        enc = ic.zstd_encode(raw, **CFG[name])
    te = time.perf_counter() - t
    t = time.perf_counter()
    try:
        if name == "png":
            dec = ic.png_decode(enc)
            exact = dec.dtype == np.uint16 and dec.shape == a.shape and np.array_equal(dec, a)
        elif name == "jxl":
            dec = ic.jpegxl_decode(enc)
            exact = dec.dtype == np.uint16 and dec.shape == a.shape and np.array_equal(dec, a)
        else:
            exact = bytes(ic.zstd_decode(enc)) == raw
    except Exception as e:  # a decode failure is a failed gate, never a result
        exact, err = False, repr(e)
    td = time.perf_counter() - t
    return dict(bytes=len(enc), exact=bool(exact), enc_s=te, dec_s=td)


def main():
    man = json.load(open(sys.argv[1]))
    only = sys.argv[sys.argv.index("--only") + 1] if "--only" in sys.argv else None
    res = {"versions": dict(imagecodecs=ic.__version__, libjxl=ic.jpegxl_version(), zstd=ic.zstd_version(),
                            libpng=ic.png_version(), config=CFG), "images": {}}
    for m in man:
        if only and m["id"] != only:
            continue
        raw = open(os.path.join(ROOT, m["canon_path"]), "rb").read()
        a = np.frombuffer(raw, "<u2").reshape(m["height"], m["width"]).astype(np.uint16)
        assert len(raw) == m["canon_bytes"]
        res["images"][m["id"]] = {c: run(c, a, raw) for c in ("png", "jxl", "zstd")}
        print(m["id"], {c: (v["bytes"], v["exact"]) for c, v in res["images"][m["id"]].items()}, flush=True)
    json.dump(res, open(sys.argv[2], "w"), indent=1)


main()
