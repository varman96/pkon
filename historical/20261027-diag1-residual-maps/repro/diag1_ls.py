#!/usr/bin/env python3
"""DIAG1 part 4: how much better could a better SPATIAL predictor be?  Oracle least-squares predictors (fitted per 128x128 tile on the tile itself,
so this is an optimistic bound, not an implementable codec) compared with MED under an identical held-out static cost model:
cost = -log2 P(class | ctx) [class = bit length of |e|, ctx = neighbourhood activity x causal residual energy, fitted on a 32px checkerboard, scored on the rest]
       + (class-1) mantissa bits for class>=2 + 1 sign bit for e != 0.
 A = same-plane causal neighbours only (10 taps);  B = A + cross-plane gradient taps from the already-decoded planes (planes 1,2 only).
python tools/diag1_ls.py --raw <rawdir> --out <diag dir>"""
import argparse, csv, json, os, sys
import numpy as np
sys.path.insert(0, os.path.dirname(__file__))
from diag1_analyze import load, shift, bucket, H, W

TILE = 128
yy, xx = np.mgrid[0:H, 0:W]
TRAIN = (((yy >> 5) + (xx >> 5)) & 1 == 0)

def bitlen(a):
    return np.minimum(np.ceil(np.log2(a + 1)).astype(np.int32), 8)  # |e|=0 ->0, 1->1, 2-3->2, 4-7->3 ...

def cost(e, act):
    ae = np.minimum(np.abs(e), 255)
    cls = np.minimum(bitlen(ae), 8)
    en = np.minimum(shift(ae, 0, 1), 64) + np.minimum(shift(ae, 1, 0), 64) + np.minimum(shift(ae, 1, 1), 64) + np.minimum(shift(ae, 1, -1), 64)
    C = (act * 8 + bucket(en)).ravel()
    cl = cls.ravel(); tr = TRAIN.ravel()
    cnt = np.bincount((C * 9 + cl)[tr], minlength=64 * 9).reshape(64, 9) + 0.4
    p = cnt / cnt.sum(1, keepdims=True)
    te = ~tr
    h = -np.log2(p.ravel()[(C * 9 + cl)[te]]).mean()
    mant = np.where(cls >= 2, cls - 1, 0).ravel()[te].mean()
    sign = (ae > 0).ravel()[te].mean()
    return float(h + mant + sign)

def taps(p):
    S = lambda dy, dx: shift(p, dy, dx, 0).astype(np.float64)
    # edge-replicated neighbours are not needed: first rows/cols are a negligible fraction and excluded by the fit mask
    return [S(0, 1), S(1, 0), S(1, 1), S(1, -1), S(0, 2), S(2, 0), S(2, -1), S(1, 2), S(1, -2), S(2, 1)]

def ls_residual(p, extra, tile=TILE):
    """per-tile oracle LS; returns integer residual p - round(pred)."""
    T = taps(p) + extra
    X = np.stack(T, -1)
    out = np.zeros((H, W), np.int32)
    pf = p.astype(np.float64)
    for ty in range(0, H, tile):
        for tx in range(0, W, tile):
            sl = (slice(ty, ty + tile), slice(tx, tx + tile))
            A = X[sl].reshape(-1, X.shape[-1]); b = pf[sl].ravel()
            A1 = np.concatenate([A, np.ones((len(A), 1))], 1)
            coef, *_ = np.linalg.lstsq(A1, b, rcond=None)
            pred = np.clip(np.rint(A1 @ coef), 0, 255)
            out[sl] = (b - pred).reshape(tile, tile).astype(np.int32)
    return out

def main():
    ap = argparse.ArgumentParser(); ap.add_argument("--raw", required=True); ap.add_argument("--out", required=True)
    a = ap.parse_args()
    G = json.load(open(os.path.join(a.out, "groups.json")))
    grp = {n: "LOSE" for n in G["lose"]}; grp.update({n: "CONTROL" for n in G["control"]}); grp.update({n: "OTHER_WIN" for n in G["other"]})
    rows = []
    for n in sorted(grp):
        D = load(a.raw, n)
        img = D["img"].astype(np.int32); g = img[..., 1]
        planes = [g, (img[..., 0] - g + 128) & 255, (img[..., 2] - g + 128) & 255]
        for k, pn in enumerate(["G", "R-G", "B-G"]):
            p = planes[k]
            wd = lambda x, y: np.abs(((x - y) & 255).astype(np.uint8).view(np.int8).astype(np.int32))
            act = bucket(wd(shift(p, 0, 1, 0), shift(p, 1, 1, 0)) + wd(shift(p, 1, 0, 0), shift(p, 1, 1, 0)) + wd(shift(p, 1, -1, 0), shift(p, 1, 0, 0)))
            med = D["med"][k].astype(np.int32)
            cm = cost(med, act)
            ca = cost(ls_residual(p, []), act)
            row = dict(image=n, group=grp[n], plane=pn, med_cost=cm, ls_spatial_cost=ca, bits_actual_coder=float(D["c4p"][k].mean()))
            if k >= 1:
                ex = []
                for j in range(k):
                    q = planes[j].astype(np.float64)
                    S = lambda dy, dx: shift(planes[j], dy, dx, 0).astype(np.float64)
                    ex += [q, q - S(0, 1), q - S(1, 0), q - S(1, 1), q - S(1, -1)]
                row["ls_cross_cost"] = cost(ls_residual(p, ex), act)
            else:
                row["ls_cross_cost"] = ""
            rows.append(row); print(n, pn, round(cm, 3), round(ca, 3), row["ls_cross_cost"], flush=True)
    with open(os.path.join(a.out, "oracle_ls_predictor.csv"), "w", newline="") as f:
        w = csv.DictWriter(f, fieldnames=list(rows[0].keys())); w.writeheader(); w.writerows(rows)

if __name__ == "__main__":
    main()
