#!/usr/bin/env python3
"""DIAG1 part 6: causal-feasibility check of the oracle-LS signal on the G plane. Coefficients for each 128x128 tile are fitted ONLY on the tile
directly above it (already decoded when the tile is reached); tiles in the first tile row keep MED. Same held-out cost model as diag1_ls.py.
Conservative: real backward-adaptive LS/RLS (per-pixel update) would see fresher data than a tile-above fit."""
import argparse, csv, json, os, sys
import numpy as np
sys.path.insert(0, os.path.dirname(__file__))
from diag1_analyze import load, shift, bucket, H, W
from diag1_ls import cost, taps, TILE

def causal(p, med):
    X = np.stack(taps(p), -1); pf = p.astype(np.float64)
    out = med.copy()
    for ty in range(TILE, H, TILE):
        for tx in range(0, W, TILE):
            src = (slice(ty - TILE, ty), slice(tx, tx + TILE)); dst = (slice(ty, ty + TILE), slice(tx, tx + TILE))
            A = np.concatenate([X[src].reshape(-1, X.shape[-1]), np.ones((TILE * TILE, 1))], 1)
            coef, *_ = np.linalg.lstsq(A, pf[src].ravel(), rcond=None)
            B = np.concatenate([X[dst].reshape(-1, X.shape[-1]), np.ones((TILE * TILE, 1))], 1)
            out[dst] = (pf[dst].ravel() - np.clip(np.rint(B @ coef), 0, 255)).reshape(TILE, TILE).astype(np.int32)
    return out

def main():
    ap = argparse.ArgumentParser(); ap.add_argument("--raw", required=True); ap.add_argument("--out", required=True)
    a = ap.parse_args()
    G = json.load(open(os.path.join(a.out, "groups.json")))
    grp = {n: "LOSE" for n in G["lose"]}; grp.update({n: "CONTROL" for n in G["control"]}); grp.update({n: "OTHER_WIN" for n in G["other"]})
    rows = []
    for n in sorted(grp):
        D = load(a.raw, n); p = D["img"][..., 1].astype(np.int32)
        wd = lambda x, y: np.abs(((x - y) & 255).astype(np.uint8).view(np.int8).astype(np.int32))
        act = bucket(wd(shift(p, 0, 1, 0), shift(p, 1, 1, 0)) + wd(shift(p, 1, 0, 0), shift(p, 1, 1, 0)) + wd(shift(p, 1, -1, 0), shift(p, 1, 0, 0)))
        med = D["med"][0].astype(np.int32)
        rows.append(dict(image=n, group=grp[n], med_cost=cost(med, act), causal_ls_cost=cost(causal(p, med), act)))
        print(n, rows[-1]["med_cost"], rows[-1]["causal_ls_cost"], flush=True)
    with open(os.path.join(a.out, "causal_ls_G.csv"), "w", newline="") as f:
        w = csv.DictWriter(f, fieldnames=list(rows[0].keys())); w.writeheader(); w.writerows(rows)

if __name__ == "__main__":
    main()
