#!/usr/bin/env python3
"""DIAG1 part 5: overlap check on a small base context (activity x MED branch, 24 ctx) so the 49x27 joint is not drowned by dilution.
Held-out static gain (bits/px) of: P, texture-direction T, phase2 PH, and the pairs P+T, P+PH, P+T+PH, relative to the base."""
import argparse, csv, json, os, sys
import numpy as np
sys.path.insert(0, os.path.dirname(__file__))
from diag1_analyze import load, shift, bucket, heldout_len, H, W
yy, xx = np.mgrid[0:H, 0:W]
TRAIN = (((yy >> 5) + (xx >> 5)) & 1 == 0).ravel()
def main():
    ap = argparse.ArgumentParser(); ap.add_argument("--raw", required=True); ap.add_argument("--out", required=True)
    a = ap.parse_args()
    G = json.load(open(os.path.join(a.out, "groups.json")))
    grp = {n: "LOSE" for n in G["lose"]}; grp.update({n: "CONTROL" for n in G["control"]}); grp.update({n: "OTHER_WIN" for n in G["other"]})
    rows = []
    sg = lambda x: np.sign(x).astype(np.int32) + 1
    for n in sorted(grp):
        D = load(a.raw, n)
        img = D["img"].astype(np.int32); g = img[..., 1]
        planes = [g, (img[..., 0] - g + 128) & 255, (img[..., 2] - g + 128) & 255]
        for k, pn in enumerate(["G", "R-G", "B-G"]):
            p = planes[k]
            pw, pn_, pnw, pne = shift(p, 0, 1, 0), shift(p, 1, 0, 0), shift(p, 1, 1, 0), shift(p, 1, -1, 0)
            wd = lambda x, y: np.abs(((x - y) & 255).astype(np.uint8).view(np.int8).astype(np.int32))
            act = bucket(wd(pw, pnw) + wd(pn_, pnw) + wd(pne, pn_))
            mx, mn = np.maximum(pw, pn_), np.minimum(pw, pn_)
            br = np.where(pnw >= mx, 0, np.where(pnw <= mn, 1, 2))
            C = (act * 3 + br).ravel()
            T = (np.clip(D["med"][k].astype(np.int32), -8, 8) + 8).ravel()
            P = D["pcls"][k].astype(np.int32).ravel()
            X = (sg(pw - pnw) * 9 + sg(pn_ - pnw) * 3 + sg(pne - pn_)).ravel()
            PH = ((yy % 2) * 2 + xx % 2).ravel()
            base = heldout_len(T, C, 24, TRAIN)
            f = {"P": (P, 49), "T": (X, 27), "PH": (PH, 4), "P+T": (P * 27 + X, 49 * 27), "P+PH": (P * 4 + PH, 196), "T+PH": (X * 4 + PH, 108), "P+T+PH": ((P * 27 + X) * 4 + PH, 49 * 27 * 4)}
            r = dict(image=n, group=grp[n], plane=pn, base=base)
            for name, (v, nf) in f.items():
                r[name] = base - heldout_len(T, C * nf + v, 24 * nf, TRAIN)
            rows.append(r)
        print(n, flush=True)
    with open(os.path.join(a.out, "overlap_small_base.csv"), "w", newline="") as f:
        w = csv.DictWriter(f, fieldnames=list(rows[0].keys())); w.writeheader(); w.writerows(rows)
if __name__ == "__main__":
    main()
