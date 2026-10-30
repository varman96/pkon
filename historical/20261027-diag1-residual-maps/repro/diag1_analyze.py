#!/usr/bin/env python3
"""DIAG1 analysis: residual maps, contact sheets and quantitative diagnostics from the arrays written by src/bin/diag1.rs.

  python tools/diag1_analyze.py --raw <rawdir> --out results/<diag dir>

Conventions (also written to the report):
  r        = signed residual (pixel - MED) mod 256 read as int8, exactly the value the codec zigzags. Coded planes are the SubG planes
             G, R-G+128, B-G+128 (plane order 0,1,2).  |r| ranges 0..128.
  abs map  = min(|r|*8, 255): 8 grey levels per unit of error, saturates at |r| >= 32. SAME mapping for every image and group.
  signed   = clip(r*4 + 128, 0, 255): zero = grey 128, positive error brighter, negative darker, saturates at |r| >= 32.
  cost map = min(bits*32, 255): bits spent on the pixel by production codec 2 (fixed scale, 0..8 bits).
  P gain   = bits(codec 1) - bits(codec 2) per pixel, display only, diverging scale +-2 bits.
Downscaled sheets use block means (display only); every statistic is computed on the full-resolution arrays.
"""
import argparse, csv, json, math, os, shutil, subprocess
import numpy as np
from PIL import Image
import matplotlib
matplotlib.use("Agg")
import matplotlib.pyplot as plt

W = H = 1024
N = W * H
PN = ["G", "R-G", "B-G"]
ACT_CUTS = [0, 2, 5, 10, 20, 40, 80]  # act_bucket: 0,1-2,3-5,6-10,11-20,21-40,41-80,>80


def bucket(a):
    return np.searchsorted(np.array(ACT_CUTS), a, side="left").astype(np.int32)


def q7(e):
    e = e.astype(np.int32)
    return np.select([e <= -5, e <= -2, e == -1, e == 0, e == 1, e <= 4], [0, 1, 2, 3, 4, 5], 6).astype(np.int32)


def shift(a, dy, dx, fill=0):
    """value at (y-dy, x-dx): dy=1,dx=0 -> N neighbour, dy=0,dx=1 -> W."""
    out = np.full_like(a, fill)
    ys, yd = (slice(dy, None), slice(0, H - dy)) if dy > 0 else (slice(None), slice(None))
    if dx > 0:
        xs, xd = slice(dx, None), slice(0, W - dx)
    elif dx < 0:
        xs, xd = slice(0, W + dx), slice(-dx, None)
    else:
        xs, xd = slice(None), slice(None)
    out[ys, xs] = a[yd, xd]
    return out


def load(raw, name):
    d = os.path.join(raw, name)
    r = lambda f, dt: np.fromfile(os.path.join(d, f), dtype=dt).reshape(3, H, W)
    return dict(
        med=r("med_sub.i8", np.int8), rgb=r("med_rgb.i8", np.int8), pcls=r("pcls.u8", np.uint8),
        c4p=r("cost_x4p.f32", np.float32), c4=r("cost_x4.f32", np.float32),
        img=np.array(Image.open(os.path.join(d, "rgb.png")).convert("RGB")),
        meta=json.load(open(os.path.join(d, "meta.json"))))


def entropy_bits(r):
    c = np.bincount(r.astype(np.int16).ravel() + 128, minlength=256).astype(np.float64)
    p = c[c > 0] / c.sum()
    return float(-(p * np.log2(p)).sum())


def block_mean(a, f):
    h, w = a.shape[:2]
    return a.reshape(h // f, f, w // f, f, *a.shape[2:]).mean(axis=(1, 3))


# ---------------------------------------------------------------- reconstruct SubG planes to verify the residual convention
def check_convention(D):
    """Rebuild the coded planes from the source crop and recompute MED with an independent numpy implementation of neighbors()/predict();
    compares against the Rust-exported residuals on every pixel."""
    img = D["img"].astype(np.int32)
    g, rr, bb = img[..., 1], img[..., 0], img[..., 2]
    planes = [g, (rr - g + 128) & 255, (bb - g + 128) & 255]
    ok = []
    for k, p in enumerate(planes):
        wv = np.zeros_like(p); nv = np.zeros_like(p); nw = np.zeros_like(p); ne = np.zeros_like(p)
        wv[:, 1:] = p[:, :-1]; wv[1:, 0] = p[:-1, 0]  # x==0,y>0: W := N ; (0,0): 0
        nv[1:] = p[:-1]; nv[0] = wv[0]  # y==0: N := W
        nw[1:, 1:] = p[:-1, :-1]; nw[0] = nv[0]; nw[1:, 0] = nv[1:, 0]
        ne[1:, :-1] = p[:-1, 1:]; ne[0] = nv[0]; ne[1:, -1] = nv[1:, -1]
        mx, mn = np.maximum(wv, nv), np.minimum(wv, nv)
        pred = np.where(nw >= mx, mn, np.where(nw <= mn, mx, wv + nv - nw))
        res = ((p - pred) & 255).astype(np.uint8).view(np.int8)
        ok.append(bool((res == D["med"][k]).all()))
    return ok


# ---------------------------------------------------------------- held-out static conditional entropy
def heldout_len(T, ctx, nctx, train):
    """bits/pixel on the held-out pixels of a static model P(T|ctx) fitted on the training pixels (add-0.4 smoothing)."""
    nT = 17
    idx = ctx.astype(np.int64) * nT + T
    tr = np.bincount(idx[train], minlength=nctx * nT).reshape(nctx, nT).astype(np.float64) + 0.4
    p = tr / tr.sum(axis=1, keepdims=True)
    te = ~train
    return float(-np.log2(p.ravel()[idx[te]]).mean())


def feature_set(D, k):
    r = D["med"][k].astype(np.int32)
    # plane pixel values are not exported; neighbour structure is derived from the source crop so analysis stays independent of Rust
    img = D["img"].astype(np.int32)
    g = img[..., 1]
    plane = [g, (img[..., 0] - g + 128) & 255, (img[..., 2] - g + 128) & 255][k]
    pw, pn, pnw, pne = shift(plane, 0, 1, 0), shift(plane, 1, 0, 0), shift(plane, 1, 1, 0), shift(plane, 1, -1, 0)
    wd = lambda a, b: np.abs(((a - b) & 255).astype(np.uint8).view(np.int8).astype(np.int32))
    act = bucket(wd(pw, pnw) + wd(pn, pnw) + wd(pne, pn))
    ar = np.abs(r)
    en = np.minimum(shift(ar, 0, 1), 64) + np.minimum(shift(ar, 1, 0), 64) + np.minimum(shift(ar, 1, 1), 64) + np.minimum(shift(ar, 1, -1), 64)
    C = act * 8 + bucket(en)
    T = np.clip(r, -8, 8) + 8
    far = bucket(np.minimum(shift(ar, 0, 2), 64) + np.minimum(shift(ar, 2, 0), 64) + np.minimum(shift(ar, 2, -1), 64) + np.minimum(shift(ar, 1, 2), 64))
    sg = lambda a: np.sign(a).astype(np.int32) + 1
    F = {}
    F["P(prod feature)"] = (D["pcls"][k].astype(np.int32), 49)
    F["sgn+mag W,N (q7xq7)"] = (q7(shift(r, 0, 1)) * 7 + q7(shift(r, 1, 0)), 49)
    F["far energy (WW,NN,NNE,NEE)"] = (far, 8)
    F["texture dir (sgn W-NW,N-NW,NE-N)"] = (sg(pw - pnw) * 9 + sg(pn - pnw) * 3 + sg(pne - pn), 27)
    yy, xx = np.mgrid[0:H, 0:W]
    F["block phase x%8,y%8"] = ((yy % 8) * 8 + xx % 8, 64)
    F["pair phase x%2,y%2"] = ((yy % 2) * 2 + xx % 2, 4)
    F["dummy hash(49)"] = (((yy * 73856093) ^ (xx * 19349663) ^ 0x9E3779B1) % 49, 49)
    if k >= 1:
        F["prev-plane r at pixel (q7)"] = (q7(D["med"][k - 1]), 7)
    if k == 2:
        F["plane0 r at pixel (q7), given plane1 r"] = (q7(D["med"][1]) * 7 + q7(D["med"][0]), 49)
    return C, T, F, r


def static_info(D):
    out = {}
    train = (((np.arange(H)[:, None] >> 5) + (np.arange(W)[None, :] >> 5)) & 1 == 0)
    for k in range(3):
        C, T, F, r = feature_set(D, k)
        base = heldout_len(T.ravel(), C.ravel(), 64, train.ravel())
        res = {"H(T|C)": base}
        for name, (f, nf) in F.items():
            if name.startswith("plane0"):
                b2 = heldout_len(T.ravel(), (C * 7 + q7(D["med"][1])).ravel(), 64 * 7, train.ravel())
                res[name] = b2 - heldout_len(T.ravel(), (C * 49 + f).ravel(), 64 * 49, train.ravel())
            else:
                res[name] = base - heldout_len(T.ravel(), (C * nf + f).ravel(), 64 * nf, train.ravel())
        out[k] = res
    return out


# ---------------------------------------------------------------- per-image scalar stats
def spearman_free_corr(a, b):
    a = a.astype(np.float64).ravel(); b = b.astype(np.float64).ravel()
    a -= a.mean(); b -= b.mean()
    d = math.sqrt((a * a).sum() * (b * b).sum())
    return float((a * b).sum() / d) if d > 0 else 0.0


def region_masks(D):
    g = D["img"][..., 1].astype(np.int32)
    pad = np.pad(g, 1, mode="edge")
    stack = np.stack([pad[dy:dy + H, dx:dx + W] for dy in range(3) for dx in range(3)])
    rng = stack.max(0) - stack.min(0)  # 3x3 range of the G channel (non-causal, analysis only)
    return {"smooth(range<=4)": rng <= 4, "texture(5..32)": (rng > 4) & (rng <= 32), "edge(range>32)": rng > 32}


def match_stats(D, k, cost):
    """exact-context match model: previous occurrence of the same (W,N,NW,NE) pixel values (coded plane); how much of the cost sits on
    pixels whose value that previous occurrence would have predicted exactly."""
    img = D["img"].astype(np.int32)
    g = img[..., 1]
    plane = [g, (img[..., 0] - g + 128) & 255, (img[..., 2] - g + 128) & 255][k]
    key = ((shift(plane, 0, 1, 0).astype(np.int64) * 256 + shift(plane, 1, 0, 0)) * 256 + shift(plane, 1, 1, 0)) * 256 + shift(plane, 1, -1, 0)
    key = key.ravel(); v = plane.ravel()
    order = np.argsort(key, kind="stable")
    ks = key[order]
    same = np.zeros(N, bool); same[1:] = ks[1:] == ks[:-1]
    prev = np.full(N, -1, np.int64); prev[order[1:][same[1:]]] = order[:-1][same[1:]]
    has = prev >= 0
    hit = has & (v[np.where(has, prev, 0)] == v)
    c = cost.ravel()
    return dict(has_prev=float(has.mean()), hit_given_prev=float(hit.sum() / max(has.sum(), 1)), hit_frac=float(hit.mean()),
                bits_share_on_hits=float(c[hit].sum() / c.sum()), mean_bits_on_hits=float(c[hit].mean()) if hit.any() else 0.0)


def image_stats(name, D, grp, split):
    rows, info = [], {}
    regs = region_masks(D)
    tot_bits = float(D["c4p"].sum())
    for k in range(3):
        r = D["med"][k]; ar = np.abs(r.astype(np.int32)); rg = np.abs(D["rgb"][k].astype(np.int32))
        c, c4 = D["c4p"][k], D["c4"][k]
        row = dict(image=name, group=grp, split=split, plane=PN[k],
                   mae_subg=float(ar.mean()), mae_rgb=float(rg.mean()), med_abs_subg=float(np.median(ar)), p99_abs=float(np.percentile(ar, 99)),
                   frac_zero=float((r == 0).mean()), frac_abs_ge16=float((ar >= 16).mean()), h0_bits=entropy_bits(r),
                   bpp_x4p=float(c.mean()), bpp_x4=float(c4.mean()), p_gain_bpp=float(c4.mean() - c.mean()), bits_share=float(c.sum() / tot_bits),
                   ctx_gain_vs_h0=entropy_bits(r) - float(c.mean()))
        rf = r.astype(np.float64)
        row["corr_W"] = spearman_free_corr(rf[:, 1:], rf[:, :-1]); row["corr_N"] = spearman_free_corr(rf[1:], rf[:-1])
        row["corr_abs_W"] = spearman_free_corr(ar[:, 1:], ar[:, :-1]); row["corr_abs_N"] = spearman_free_corr(ar[1:], ar[:-1])
        nz = (r[:, 1:] != 0) & (r[:, :-1] != 0)
        row["sign_agree_W"] = float((np.sign(r[:, 1:])[nz] == np.sign(r[:, :-1])[nz]).mean()) if nz.any() else 0.5
        nzn = (r[1:] != 0) & (r[:-1] != 0)
        row["sign_agree_N"] = float((np.sign(r[1:])[nzn] == np.sign(r[:-1])[nzn]).mean()) if nzn.any() else 0.5
        if k >= 1:
            row["corr_prev_plane"] = spearman_free_corr(rf, D["med"][k - 1].astype(np.float64))
            row["corr_abs_prev_plane"] = spearman_free_corr(ar, np.abs(D["med"][k - 1].astype(np.int32)))
            nzp = (r != 0) & (D["med"][k - 1] != 0)
            row["sign_agree_prev_plane"] = float((np.sign(r)[nzp] == np.sign(D["med"][k - 1])[nzp]).mean()) if nzp.any() else 0.5
        if k == 2:
            row["corr_plane0"] = spearman_free_corr(rf, D["med"][0].astype(np.float64))
            # variance explained by a per-image least-squares fit r2 ~ a*r1 + b*r0
            A = np.stack([D["med"][1].ravel().astype(np.float64), D["med"][0].ravel().astype(np.float64)], 1)
            coef, *_ = np.linalg.lstsq(A, rf.ravel(), rcond=None)
            res = rf.ravel() - A @ coef
            row["r2_from_r1_r0"] = float(1 - res.var() / rf.var()) if rf.var() > 0 else 0.0
        for rn, m in regs.items():
            row[f"frac[{rn}]"] = float(m.mean())
            row[f"bits_share[{rn}]"] = float(c[m].sum() / c.sum()) if m.any() else 0.0
            row[f"mean_bits[{rn}]"] = float(c[m].mean()) if m.any() else 0.0
            row[f"mae[{rn}]"] = float(ar[m].mean()) if m.any() else 0.0
        ph = c.reshape(H // 8, 8, W // 8, 8).mean(axis=(0, 2))
        row["phase8_rel_range"] = float((ph.max() - ph.min()) / ph.mean())
        ph_col = c.reshape(H, W // 8, 8).mean(axis=(0, 1)); ph_row = c.reshape(H // 8, 8, W).mean(axis=(0, 2))
        row["phase8_col_rel_range"] = float((ph_col.max() - ph_col.min()) / ph_col.mean())
        row["phase8_row_rel_range"] = float((ph_row.max() - ph_row.min()) / ph_row.mean())
        p2 = c.reshape(H // 2, 2, W // 2, 2).mean(axis=(0, 2))
        row["phase2_rel_range"] = float((p2.max() - p2.min()) / p2.mean())
        ms = match_stats(D, k, c)
        for a, b in ms.items():
            row["match_" + a] = b
        rows.append(row)
    return rows


# ---------------------------------------------------------------- images
def save_abs(path, r):
    Image.fromarray(np.minimum(np.abs(r.astype(np.int32)) * 8, 255).astype(np.uint8)).save(path)


def save_signed(path, r):
    Image.fromarray(np.clip(r.astype(np.int32) * 4 + 128, 0, 255).astype(np.uint8)).save(path)


def abs_disp(r, f):
    return np.minimum(block_mean(np.abs(r.astype(np.float32)), f) * 8, 255)


def cost_disp(c, f):
    return np.minimum(block_mean(c, f) * 32, 255)


def sheet_image(path, name, D, info):
    f = 2
    fig, ax = plt.subplots(2, 4, figsize=(20, 10.4))
    ax = ax.ravel()
    ax[0].imshow(block_mean(D["img"].astype(np.float32), f).astype(np.uint8)); ax[0].set_title(f"{name} source (2x down)")
    for k in range(3):
        ax[1 + k].imshow(abs_disp(D["med"][k], f), cmap="gray", vmin=0, vmax=255); ax[1 + k].set_title(f"|MED residual| {PN[k]} (x8, sat |r|>=32)")
    ax[4].imshow(cost_disp(D["c4p"].sum(0) / 3, f), cmap="gray", vmin=0, vmax=255); ax[4].set_title("PKON2P bits/px, mean of 3 planes (x32, sat 8 b)")
    d = block_mean((D["c4"] - D["c4p"]).sum(0), f)
    ax[5].imshow(d, cmap="RdBu", vmin=-4, vmax=4); ax[5].set_title("bits saved by P (X4 - X4P), +-4 b, blue = P saved")
    ax[6].imshow(np.clip(D["med"][1].astype(np.int32) * 4 + 128, 0, 255)[::f, ::f], cmap="gray", vmin=0, vmax=255); ax[6].set_title("signed r, R-G (zero=128, x4)")
    ax[7].imshow(np.clip(D["med"][2].astype(np.int32) * 4 + 128, 0, 255)[::f, ::f], cmap="gray", vmin=0, vmax=255); ax[7].set_title("signed r, B-G (zero=128, x4)")
    for a in ax:
        a.axis("off")
    fig.suptitle(info, fontsize=12)
    fig.tight_layout(); fig.savefig(path, dpi=60); plt.close(fig)


def contact_sheet(path, title, names, data, labels):
    f = 4
    rows = len(names)
    fig, ax = plt.subplots(rows, 6, figsize=(18, 3.05 * rows), squeeze=False)
    for i, n in enumerate(names):
        D = data[n]
        ax[i][0].imshow(block_mean(D["img"].astype(np.float32), f).astype(np.uint8)); ax[i][0].set_ylabel(labels[n], fontsize=8, rotation=0, ha="right", va="center")
        for k in range(3):
            ax[i][1 + k].imshow(abs_disp(D["med"][k], f), cmap="gray", vmin=0, vmax=255)
        ax[i][4].imshow(cost_disp(D["c4p"].sum(0) / 3, f), cmap="gray", vmin=0, vmax=255)
        ax[i][5].imshow(block_mean((D["c4"] - D["c4p"]).sum(0), f), cmap="RdBu", vmin=-4, vmax=4)
        if i == 0:
            for j, t in enumerate(["source", "|r| G", "|r| R-G", "|r| B-G", "PKON2P bits/px", "bits saved by P"]):
                ax[i][j].set_title(t)
    for a in ax.ravel():
        a.set_xticks([]); a.set_yticks([])
    fig.suptitle(title + "  (fixed scales: |r| x8 sat 32; bits x32 sat 8; P gain +-4 b)")
    fig.tight_layout(); fig.savefig(path, dpi=55); plt.close(fig)


# ---------------------------------------------------------------- main
def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--raw", required=True); ap.add_argument("--out", required=True)
    a = ap.parse_args()
    out = a.out
    for s in ["maps", "sheets", "data"]:
        os.makedirs(os.path.join(out, s), exist_ok=True)
    splits = json.load(open("results/learn1-splits.json"))
    split_of = {n: "DEV" for n in splits["train_pool"]}; split_of.update({n: "VAL" for n in splits["val"]})
    p_bytes = {r["image"]: int(r["pkon2p_file_bytes"]) for r in csv.DictReader(open("results/20261027-pkon2p-production/per_image.csv"))}
    jx = {}
    for r in csv.DictReader(open("results/20261027-155433Z-bench50/per_image.csv")):
        jx.setdefault(r["image"], {})[r["codec"]] = int(r["total_bytes"])
    names = sorted(split_of)
    # ---- frozen grouping rule (written into the report before any residual statistic was computed)
    gap = {n: 100.0 * (p_bytes[n] - jx[n]["jxl-e9-t1"]) / jx[n]["jxl-e9-t1"] for n in names}
    lose = sorted([n for n in names if gap[n] > 0], key=lambda n: -gap[n])
    non = [n for n in names if gap[n] <= 0]
    bpp = {n: p_bytes[n] * 8 / N for n in names}
    ctrl, pool = [], list(non)
    for l in lose:  # matched control: nearest overall P bits/pixel among not-yet-used non-losers
        c = min(pool, key=lambda n: abs(bpp[n] - bpp[l])); ctrl.append(c); pool.remove(c)
    grp = {n: "LOSE" for n in lose}
    grp.update({n: "CONTROL" if n in ctrl else "OTHER_WIN" for n in non})
    data, rows, info = {}, [], {}
    conv = {}
    for n in names:
        D = load(a.raw, n)
        assert D["meta"]["x4p_stream_bytes"] == D["meta"]["production_x4p_payload"]
        conv[n] = check_convention(D)
        assert all(conv[n]), f"independent numpy MED disagrees with exported residuals on {n}: {conv[n]}"
        rows += image_stats(n, D, grp[n], split_of[n])
        info[n] = static_info(D)
        data[n] = D if grp[n] != "OTHER_WIN" else D
        print(n, grp[n], "ok", flush=True)
        if grp[n] == "OTHER_WIN":
            data[n] = {k: D[k] for k in ["med", "c4p", "c4", "img"]}  # keep memory modest
        else:
            data[n] = {k: D[k] for k in ["med", "c4p", "c4", "img"]}
    # ---- csv outputs
    keys = list(rows[0].keys())
    for r in rows:
        for k in r:
            if k not in keys: keys.append(k)
    allk = []
    for r in rows:
        for k in r:
            if k not in allk: allk.append(k)
    with open(os.path.join(out, "per_image_plane_stats.csv"), "w", newline="") as f:
        w = csv.DictWriter(f, fieldnames=allk, restval=""); w.writeheader(); w.writerows(rows)
    srows = []
    for n in names:
        for k in range(3):
            for fn, v in info[n][k].items():
                srows.append(dict(image=n, group=grp[n], split=split_of[n], plane=PN[k], quantity=fn, bits_per_pixel=v,
                                  pct_of_actual_plane_bpp=100 * v / next(r["bpp_x4p"] for r in rows if r["image"] == n and r["plane"] == PN[k]) if fn != "H(T|C)" else ""))
    with open(os.path.join(out, "static_information.csv"), "w", newline="") as f:
        w = csv.DictWriter(f, fieldnames=list(srows[0].keys())); w.writeheader(); w.writerows(srows)
    # ---- images: full-res maps for LOSE + CONTROL, sheets for everyone
    manifest = []
    for n in names:
        D = data[n]
        tag = f"{grp[n]} gap_vs_e9={gap[n]:+.2f}% ({split_of[n]}) P={p_bytes[n]}B e9={jx[n]['jxl-e9-t1']}B"
        sheet_image(os.path.join(out, "sheets", f"{n}.png"), n, D, tag)
        files = [f"sheets/{n}.png"]
        if grp[n] in ("LOSE", "CONTROL"):
            md = os.path.join(out, "maps", n); os.makedirs(md, exist_ok=True)
            Image.fromarray(D["img"]).save(os.path.join(md, "source.png"))
            for k in range(3):
                save_abs(os.path.join(md, f"abs_{PN[k]}.png"), D["med"][k]); save_signed(os.path.join(md, f"signed_{PN[k]}.png"), D["med"][k])
            np.savez_compressed(os.path.join(out, "data", f"{n}_med_residuals_int8.npz"), subg_planes_G_RmG_BmG=D["med"])
            files += [f"maps/{n}/{x}" for x in sorted(os.listdir(md))] + [f"data/{n}_med_residuals_int8.npz"]
        manifest.append(dict(image=n, split=split_of[n], group=grp[n], pkon2p_file_bytes=p_bytes[n], jxl_e9_bytes=jx[n]["jxl-e9-t1"], jxl_e7_bytes=jx[n]["jxl-e7-t1"],
                             gap_vs_e9_pct=round(gap[n], 3), crop_sha256=data[n] and json.load(open(os.path.join(a.raw, n, "meta.json")))["crop_sha256"],
                             source_file=f"data/{n}.*", outputs=";".join(files)))
    json.dump(manifest, open(os.path.join(out, "manifest.json"), "w"), indent=1)
    with open(os.path.join(out, "manifest.csv"), "w", newline="") as f:
        w = csv.DictWriter(f, fieldnames=list(manifest[0].keys())); w.writeheader(); w.writerows(manifest)
    lab = lambda n: f"{n}\n{grp[n]} {gap[n]:+.1f}%"
    labels = {n: lab(n) for n in names}
    contact_sheet(os.path.join(out, "sheets", "contact_LOSE.png"), "LOSE: JPEG XL e9 smaller than PKON2P (DEV+VAL)", lose, data, labels)
    contact_sheet(os.path.join(out, "sheets", "contact_CONTROL_matched.png"), "CONTROL: PKON2P smaller, matched to losers by bits/pixel", ctrl, data, labels)
    others = [n for n in non if n not in ctrl]
    for i in range(0, len(others), 7):
        contact_sheet(os.path.join(out, "sheets", f"contact_OTHER_WIN_{i // 7 + 1}.png"), "OTHER wins (PKON2P smaller than JXL e9)", others[i:i + 7], data, labels)
    json.dump(dict(lose=lose, control=ctrl, other=others, gap=gap, convention_check=conv), open(os.path.join(out, "groups.json"), "w"), indent=1)
    print("done")


if __name__ == "__main__":
    main()
