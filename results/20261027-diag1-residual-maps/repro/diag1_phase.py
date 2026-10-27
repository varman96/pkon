#!/usr/bin/env python3
"""DIAG1 part 3: position-phase structure (JPEG chroma subsampling / 8x8 DCT grid leave period-2/8/16 traces in the decoded pixels).
Measures (a) relative bit cost per (x%2,y%2) phase, (b) held-out static information of phase features given the production-like base context
(same base as diag1_extra.py), with dummy floors of the same cardinality.   python tools/diag1_phase.py --raw <rawdir> --out <diag dir>"""
import argparse, csv, json, os, sys
import numpy as np
sys.path.insert(0, os.path.dirname(__file__))
from diag1_analyze import load, shift, bucket, heldout_len, H, W
from diag1_extra import prev_q, zz

def base_ctx(D, k):
    img = D["img"].astype(np.int32); g = img[..., 1]
    p = [g, (img[..., 0] - g + 128) & 255, (img[..., 2] - g + 128) & 255][k]
    r = D["med"][k].astype(np.int32); ar = np.abs(r)
    pw, pn, pnw, pne = shift(p, 0, 1, 0), shift(p, 1, 0, 0), shift(p, 1, 1, 0), shift(p, 1, -1, 0)
    wd = lambda a, b: np.abs(((a - b) & 255).astype(np.uint8).view(np.int8).astype(np.int32))
    act = bucket(wd(pw, pnw) + wd(pn, pnw) + wd(pne, pn))
    mx, mn = np.maximum(pw, pn), np.minimum(pw, pn)
    br = np.where(pnw >= mx, 0, np.where(pnw <= mn, 1, 2))
    en = np.minimum(shift(ar, 0, 1), 64) + np.minimum(shift(ar, 1, 0), 64) + np.minimum(shift(ar, 1, 1), 64) + np.minimum(shift(ar, 1, -1), 64)
    pq = prev_q(zz(D["med"][k - 1])) if k >= 1 else np.zeros((H, W), np.int32)
    return ((act * 3 + br) * 4 + pq) * 8 + bucket(en), 8 * 3 * 4 * 8, (np.clip(r, -8, 8) + 8).ravel()

def main():
    ap = argparse.ArgumentParser(); ap.add_argument("--raw", required=True); ap.add_argument("--out", required=True)
    a = ap.parse_args()
    G = json.load(open(os.path.join(a.out, "groups.json")))
    grp = {n: "LOSE" for n in G["lose"]}; grp.update({n: "CONTROL" for n in G["control"]}); grp.update({n: "OTHER_WIN" for n in G["other"]})
    yy, xx = np.mgrid[0:H, 0:W]
    train = (((yy >> 5) + (xx >> 5)) & 1 == 0).ravel()
    feats = {"phase x%2,y%2 (4)": ((yy % 2) * 2 + xx % 2, 4), "phase x%4,y%4 (16)": ((yy % 4) * 4 + xx % 4, 16),
             "phase x%16,y%16 (256)": ((yy % 16) * 16 + xx % 16, 256), "phase x%8,y%8 (64)": ((yy % 8) * 8 + xx % 8, 64)}
    for dc in (4, 16, 64, 256):
        feats[f"dummy hash card {dc}"] = (((yy * 73856093) ^ (xx * 19349663) ^ (0x9E3779B1 + dc)) % dc, dc)
    rows, ph = [], []
    for n in sorted(grp):
        D = load(a.raw, n)
        for k, pn in enumerate(["G", "R-G", "B-G"]):
            C, nC, T = base_ctx(D, k)
            base = heldout_len(T, C.ravel(), nC, train)
            for name, (f, nf) in feats.items():
                v = base - heldout_len(T, C.ravel() * nf + f.ravel(), nC * nf, train)
                rows.append(dict(image=n, group=grp[n], plane=pn, quantity=name, bits_per_pixel=v, pct_of_plane_bits=100 * v / float(D["c4p"][k].mean())))
            m = D["c4p"][k].reshape(H // 2, 2, W // 2, 2).mean(axis=(0, 2)); m = m / m.mean()
            ph.append(dict(image=n, group=grp[n], plane=pn, rel_cost_00=m[0, 0], rel_cost_01=m[0, 1], rel_cost_10=m[1, 0], rel_cost_11=m[1, 1]))
        print(n, flush=True)
    for fn, rr in (("phase_static_information.csv", rows), ("phase2_relative_cost.csv", ph)):
        with open(os.path.join(a.out, fn), "w", newline="") as f:
            w = csv.DictWriter(f, fieldnames=list(rr[0].keys())); w.writeheader(); w.writerows(rr)

if __name__ == "__main__":
    main()
