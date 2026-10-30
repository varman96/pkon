#!/usr/bin/env python3
"""DIAG1 part 2: held-out static information of candidate features given a base context that mimics the production coder's own
context (activity x MED branch x previous-plane magnitude x local residual energy). Calibrated by the production P feature (known real gain)
and a dummy hash feature (overfit floor).   python tools/diag1_extra.py --raw <rawdir> --out <diag dir>
"""
import argparse, csv, json, os, sys
import numpy as np
sys.path.insert(0, os.path.dirname(__file__))
from diag1_analyze import load, shift, bucket, q7, heldout_len, H, W

def prev_q(z):  # production prev_q on zigzag magnitude: 0, 1..2, 3..8, >8
    return np.select([z == 0, z <= 2, z <= 8], [0, 1, 2], 3).astype(np.int32)

def zz(r):
    r = r.astype(np.int32)
    return np.where(r >= 0, 2 * r, -2 * r - 1)

def box(a, rad):
    p = np.pad(a.astype(np.float32), rad, mode="constant")
    s = np.zeros_like(a, dtype=np.float32)
    for dy in range(2 * rad + 1):
        for dx in range(2 * rad + 1):
            s += p[dy:dy + H, dx:dx + W]
    return s

def run(D, k):
    img = D["img"].astype(np.int32)
    g = img[..., 1]
    planes = [g, (img[..., 0] - g + 128) & 255, (img[..., 2] - g + 128) & 255]
    p = planes[k]
    r = D["med"][k].astype(np.int32); ar = np.abs(r)
    pw, pn, pnw, pne = shift(p, 0, 1, 0), shift(p, 1, 0, 0), shift(p, 1, 1, 0), shift(p, 1, -1, 0)
    wd = lambda a, b: np.abs(((a - b) & 255).astype(np.uint8).view(np.int8).astype(np.int32))
    act = bucket(wd(pw, pnw) + wd(pn, pnw) + wd(pne, pn))
    mx, mn = np.maximum(pw, pn), np.minimum(pw, pn)
    br = np.where(pnw >= mx, 0, np.where(pnw <= mn, 1, 2))
    en = np.minimum(shift(ar, 0, 1), 64) + np.minimum(shift(ar, 1, 0), 64) + np.minimum(shift(ar, 1, 1), 64) + np.minimum(shift(ar, 1, -1), 64)
    pq = prev_q(zz(D["med"][k - 1])) if k >= 1 else np.zeros((H, W), np.int32)
    C = ((act * 3 + br) * 4 + pq) * 8 + bucket(en)
    nC = 8 * 3 * 4 * 8
    T = (np.clip(r, -8, 8) + 8).ravel()
    yy, xx = np.mgrid[0:H, 0:W]
    train = (((yy >> 5) + (xx >> 5)) & 1 == 0).ravel()
    sg = lambda a: np.sign(a).astype(np.int32) + 1
    F = {}
    F["P (production feature; real gain known)"] = (D["pcls"][k].astype(np.int32), 49)
    F["dummy hash card 49 (overfit floor)"] = (((yy * 73856093) ^ (xx * 19349663) ^ 0x9E3779B1) % 49, 49)
    for dc in (7, 8, 27):
        F[f"dummy hash card {dc} (overfit floor)"] = (((yy * 73856093) ^ (xx * 19349663) ^ (0x9E3779B1 + dc)) % dc, dc)
    F["texture dir sgn(W-NW,N-NW,NE-N)"] = (sg(pw - pnw) * 9 + sg(pn - pnw) * 3 + sg(pne - pn), 27)
    if k >= 1:
        for j in range(k):  # fully decoded earlier planes: window includes pixels EAST/SOUTH of the current one
            a = np.abs(D["med"][j].astype(np.int32))
            tag = f"plane{j}"
            F[f"{tag} |r| 3x3 window energy (incl. E/S)"] = (bucket(box(np.minimum(a, 64), 1) / 9 * 3), 8)
            F[f"{tag} |r| 5x5 window energy (incl. E/S)"] = (bucket(box(np.minimum(a, 64), 2) / 25 * 3), 8)
            F[f"{tag} |r| at pixel, 8 buckets"] = (bucket(a), 8)
            F[f"{tag} signed r at pixel (q7)"] = (q7(D["med"][j]), 7)
            gp = planes[j]
            pad = np.pad(gp, 1, mode="edge")
            st = np.stack([pad[dy:dy + H, dx:dx + W] for dy in range(3) for dx in range(3)])
            F[f"{tag} pixel 3x3 range (edge strength incl. E/S)"] = (bucket(st.max(0) - st.min(0)), 8)
    if k == 2:
        F["plane0 signed r given plane1 signed r"] = (q7(D["med"][1]) * 7 + q7(D["med"][0]), 49)
    out = {}
    base = heldout_len(T, C.ravel(), nC, train)
    out["H(T|base ctx)"] = base
    for name, (f, nf) in F.items():
        f = np.asarray(f).ravel().astype(np.int64)
        if name.startswith("plane0 signed"):
            c0 = (C * 7 + q7(D["med"][1])).ravel()
            b2 = heldout_len(T, c0, nC * 7, train)
            out[name] = b2 - heldout_len(T, c0 * 7 + q7(D["med"][0]).ravel(), nC * 49, train)
        else:
            out[name] = base - heldout_len(T, C.ravel() * nf + f, nC * nf, train)
    return out

def main():
    ap = argparse.ArgumentParser(); ap.add_argument("--raw", required=True); ap.add_argument("--out", required=True)
    a = ap.parse_args()
    groups = json.load(open(os.path.join(a.out, "groups.json")))
    grp = {n: "LOSE" for n in groups["lose"]}; grp.update({n: "CONTROL" for n in groups["control"]}); grp.update({n: "OTHER_WIN" for n in groups["other"]})
    rows = []
    for n in sorted(grp):
        D = load(a.raw, n)
        for k, pn in enumerate(["G", "R-G", "B-G"]):
            for q, v in run(D, k).items():
                rows.append(dict(image=n, group=grp[n], plane=pn, quantity=q, bits_per_pixel=v))
        print(n, flush=True)
    with open(os.path.join(a.out, "static_information_base_ctx.csv"), "w", newline="") as f:
        w = csv.DictWriter(f, fieldnames=list(rows[0].keys())); w.writeheader(); w.writerows(rows)

if __name__ == "__main__":
    main()
