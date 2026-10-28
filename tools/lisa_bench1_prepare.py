"""LISA benchmark 1, step 1: extract canonical samples from the 25 selected DNGs and freeze the corpus.

Canonical representation: row-major, little-endian uint16, one sample per mosaic site, the full stored raster
(no ActiveArea/DefaultCrop crop, no black-level/white-balance/linearization/demosaic).
Run: python -I tools/lisa_bench1_prepare.py <out_manifest.json>"""
import hashlib, json, os, sys
import numpy as np
sys.path.insert(0, os.path.join(os.path.dirname(os.path.abspath(__file__))))
import lisa_dng

ROOT = os.path.join(os.path.dirname(os.path.abspath(__file__)), "..")
DNG = os.path.join(ROOT, "data", "praw1", "dng")
CANON = os.path.join(ROOT, "data", "praw1", "canon")


def sha(path):
    h = hashlib.sha256()
    with open(path, "rb") as f:
        for b in iter(lambda: f.read(1 << 22), b""):
            h.update(b)
    return h.hexdigest()


def bits_needed(v):
    return max(1, int(v).bit_length())


def main():
    sel = json.load(open(os.path.join(ROOT, "tools", "lisa_bench1_selection.json")))
    out = []
    for s in sel:
        p = os.path.join(DNG, s["id"] + ".dng")
        info, a = lisa_dng.read_dng(p)
        assert a.ndim == 2 and a.dtype in (np.uint8, np.uint16), (s["id"], a.shape, a.dtype)
        a16 = np.ascontiguousarray(a.astype("<u2"))
        h, w = a16.shape
        assert (w, h) == (info["width"], info["height"])
        mx, mn = int(a16.max()), int(a16.min())
        declared = int(info["bits_per_sample"])
        wl = max(info["white_level"] or [0])
        bd = min(declared, bits_needed(max(mx, wl)))
        assert mx < (1 << bd), (s["id"], mx, bd)
        cpath = os.path.join(CANON, s["id"] + ".u16")
        a16.tofile(cpath)
        # CFA metadata: DNG CFAPattern is relative to the ActiveArea origin; LISA wants the pattern position of
        # image sample (0,0).
        cfa = info["cfa_pattern"]
        dim = info["cfa_dim"]
        origin = None
        if cfa:
            aa = info["active_area"] or [0, 0, h, w]
            top, left = aa[0], aa[1]
            origin = [(-left) % dim[1], (-top) % dim[0]]
        row = dict(s)
        row.update(
            camera=f'{info["make"]} {info["model"]}'.strip(), width=w, height=h, declared_bits=declared, bit_depth=bd,
            sample_min=mn, sample_max=mx, white_level=info["white_level"], black_level=info["black_level"],
            active_area=info["active_area"], cfa_dim=dim, cfa=cfa, cfa_origin=origin,
            dng_compression=info["compression_name"], dng_compression_code=info["compression"],
            predictor=info["predictor"], tiled=info["tiled"], sample_format=info["sample_format"],
            stored_dtype=str(a.dtype), photometric=info["photometric"], dng_bytes=os.path.getsize(p),
            dng_sha256=sha(p), canon_bytes=a16.nbytes, canon_sha256=sha(cpath), canon_path=os.path.relpath(cpath, ROOT),
        )
        out.append(row)
        print(s["id"], w, h, declared, bd, info["compression_name"], flush=True)
    json.dump(out, open(sys.argv[1], "w"), indent=1)


main()
