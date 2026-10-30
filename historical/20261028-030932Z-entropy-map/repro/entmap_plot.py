"""ENTMAP figures + derived tables. Reads only existing files: the bench25 results (per_image.csv, results.json) and the
entmap_analyze outputs. Usage: python -I tools/entmap_plot.py <bench25 results dir> <entropy-map results dir>
Writes <dir>/data/map_data.csv, aggregate.csv, category_summary.csv, plane_summary.csv and <dir>/figures/*.{png,svg,pdf}."""
import csv, json, math, sys
from collections import defaultdict
import numpy as np
import matplotlib

matplotlib.use("Agg")
import matplotlib.pyplot as plt
from matplotlib.lines import Line2D
from matplotlib.patches import Patch, Rectangle

BENCH, D = sys.argv[1:3]
DATA, FIG = f"{D}/data", f"{D}/figures"

# ---------------------------------------------------------------- style (light surface; three categorical hues validated all-pairs)
SURF, INK, INK2, MUTED, GRID, AXIS = "#fcfcfb", "#0b0b0b", "#52514e", "#898781", "#e1e0d9", "#c3c2b7"
BAND, BAND2, HATCH = "#bdbbb2", "#8f8d86", "#a9a79e"
plt.rcParams.update({"font.family": ["Segoe UI", "DejaVu Sans"], "font.size": 9, "axes.edgecolor": AXIS, "axes.linewidth": 0.8,
                     "axes.labelcolor": INK2, "xtick.color": INK2, "ytick.color": INK2, "axes.facecolor": SURF, "figure.facecolor": SURF,
                     "savefig.facecolor": SURF, "axes.grid": False, "hatch.color": HATCH, "hatch.linewidth": 0.6, "svg.fonttype": "none"})
CODECS = ["pkon2-file", "jxl-e9-t1", "jxl-e7-t1", "webp-m6-t1", "png-rs"]
LAB = {"pkon2-file": "PKON2 REL6 (codec 3)", "jxl-e9-t1": "JPEG XL lossless e9", "jxl-e7-t1": "JPEG XL lossless e7",
       "webp-m6-t1": "WebP lossless m6", "png-rs": "PNG (png-rs best)"}
SHORT = {"pkon2-file": "PKON2", "jxl-e9-t1": "JXL e9", "jxl-e7-t1": "JXL e7", "webp-m6-t1": "WebP m6", "png-rs": "PNG"}
STY = {"pkon2-file": dict(color="#2a78d6", marker="o", mfc="#2a78d6", ms=7.5),
       "jxl-e9-t1": dict(color="#eb6834", marker="D", mfc="#eb6834", ms=5.5),
       "jxl-e7-t1": dict(color="#eb6834", marker="D", mfc=SURF, ms=5.5),
       "webp-m6-t1": dict(color="#1baf7a", marker="^", mfc="#1baf7a", ms=6.5),
       "png-rs": dict(color=MUTED, marker="s", mfc=MUTED, ms=5)}
GROUPS = [("Sky & gradients", ["clouds_n0", "skyline_n0", "sunset_n0"]),
          ("People", ["portrait_0", "child_n0", "woman_n0"]),
          ("Animals", ["pet_0", "pet2_1", "bird_n0"]),
          ("Architecture & interiors", ["indoor_0", "building_n0", "church_n0"]),
          ("Water", ["waterfall_n0", "lake_n0"]),
          ("Foliage & ground texture", ["outdoor_0", "foliage_0", "path_n0"]),
          ("Other texture", ["food_0", "snow_n0"]),
          ("Low light / noise", ["night_0", "night_1"]),
          ("Graphic, text, document, blur", ["blurry_1", "screen_0", "text_1", "document_1"])]
GROUP_OF = {n: g for g, ns in GROUPS for n in ns}


def save(fig, name):
    for ext in ("png", "svg", "pdf"):
        fig.savefig(f"{FIG}/{name}.{ext}", dpi=200 if ext == "png" else None, bbox_inches="tight", pad_inches=0.15)
    plt.close(fig)


def wcsv(path, rows):
    keys = list(dict.fromkeys(k for r in rows for k in r))
    with open(path, "w", newline="", encoding="utf-8") as f:
        w = csv.DictWriter(f, fieldnames=keys)
        w.writeheader()
        w.writerows(rows)


def num(v):
    try:
        return float(v)
    except (TypeError, ValueError):
        return float("nan")


# ---------------------------------------------------------------- load
bench = {(r["image"], r["codec"]): r for r in csv.DictReader(open(f"{BENCH}/per_image.csv", encoding="utf-8"))}
R = json.load(open(f"{BENCH}/results.json", encoding="utf-8"))
floor = {c: (e, d) for c, e, d in R["calibration_ms"]}
meta = {i["name"]: i for i in R["images"]}
ent = {r["image"]: {k: (num(v) if k != "image" else v) for k, v in r.items()} for r in csv.DictReader(open(f"{DATA}/entropy_per_image.csv", encoding="utf-8"))}
jpg = {r["image"]: r for r in csv.DictReader(open(f"{DATA}/jpeg_provenance.csv", encoding="utf-8"))}
plane = defaultdict(dict)  # (image, domain, plane) -> model -> (plug-in, Miller-Madow, adaptive) bits per sample
for r in csv.DictReader(open(f"{DATA}/entropy_per_plane.csv", encoding="utf-8")):
    plane[(r["image"], r["domain"], int(r["plane"]))][r["model"]] = (float(r["plugin_bits_per_sample"]), float(r["mm_bits_per_sample"]),
                                                                     float(r["adaptive_bits_per_sample"]))
PROBES = {"P": ("L3_P", "L2_null49"), "parity": ("L2_parity", "L2_null4"), "phase8": ("L2_phase8", "L2_null64")}
names = [n for _, ns in GROUPS for n in ns]
assert sorted(names) == sorted(ent), "group map must cover the 25 images exactly"

# ---------------------------------------------------------------- derived per-image table
rows = []
for n in names:
    e, px = ent[n], int(bench[(n, "pkon2-file")]["pixels"])
    r = {"image": n, "group": GROUP_OF[n], "category": meta[n]["category"], "source_format": meta[n]["source_format"].split("/")[-1],
         "layout": meta[n]["crop_format"], "raw_bpp": e["raw_bpp"], "med_mae_g": meta[n]["med_mae_g"], "clipped_frac_g": meta[n]["clipped_frac_g"]}
    for c in CODECS:
        b = bench[(n, c)]
        r[f"bpp:{c}"] = int(b["total_bytes"]) * 8 / px
        r[f"enc_ms:{c}"], r[f"dec_ms:{c}"] = float(b["enc_ms_median"]), float(b["dec_ms_median"])
    pk = r["bpp:pkon2-file"]
    others = {c: r[f"bpp:{c}"] for c in CODECS[1:]}
    best = min(others, key=others.get)
    r["best_other_codec"], r["best_other_bpp"] = best, others[best]
    r["best_measured_bpp"] = min(pk, others[best])
    for k in ("H0_channels", "H0_subg_planes", "H0_med_rgb", "H0_med_subg", "H_joint_rgb_plugin", "TC_rgb_plugin", "I_RG", "I_BG", "I_RB",
              "I_resid_RG", "I_resid_BG", "unique_colors", "ctx_seen_share", "repeat_hit_share", "repeat_hit_share_of_med_miss", "med_miss_share",
              "G_H0", "G_H_given_W_plugin", "G_H_given_W_mm", "G_H_given_W_kt", "G_H_given_WN_plugin", "G_H_given_WN_mm", "G_H_given_WN_kt",
              "H0_ls_global", "H0_ls_tile"):
        r[k] = e[k]
    for dom in ("subg_med", "rgb_med", "subg_ls_global", "subg_ls_tile"):
        for mo in ("L0", "L1", "L2", "L3_P", "L2_parity", "L2_phase8", "L2_null4", "L2_null49", "L2_null64"):
            for est in ("plugin", "mm", "adaptive"):
                k = f"{dom}:{mo}:{est}"
                if k in e:
                    r[k] = e[k]
    # null-calibrated probes: net signal = estimator(L2 x feature) - estimator(L2 x random context of the same cardinality)
    for f, (mo, null) in PROBES.items():
        r[f"net:{f}:mm"] = e[f"subg_med:{mo}:mm"] - e[f"subg_med:{null}:mm"]
        r[f"net:{f}:adaptive"] = e[f"subg_med:{mo}:adaptive"] - e[f"subg_med:{null}:adaptive"]
        r[f"static_cal:{f}"] = e["subg_med:L2:mm"] + r[f"net:{f}:mm"]
    # per-plane predictor choice: min(MED, LS-tile) under L2 per plane + 1 flag bit per plane (LS coefficient cost inside the adaptive value)
    nplanes = int(e["channels"])
    tile_coef_per_plane = e["ls_tile_coef_bits"] / 3 / px
    sel_mm = sel_ad = 0.0
    for k in range(nplanes):
        med_, ls_ = plane[(n, "subg_med", k)]["L2"], plane[(n, "subg_ls_tile", k)]["L2"]
        sel_mm += min(med_[1], ls_[1])
        sel_ad += min(med_[2], ls_[2] + (tile_coef_per_plane if k < 3 else 0.0))
    r["pred_select_L2:mm"], r["pred_select_L2:adaptive"] = sel_mm + nplanes / px, sel_ad + nplanes / px
    adaptive_models = {"L2": e["subg_med:L2:adaptive"], "L3_P": e["subg_med:L3_P:adaptive"], "L2_parity": e["subg_med:L2_parity:adaptive"],
                       "L2_phase8": e["subg_med:L2_phase8:adaptive"], "pred_select_L2": r["pred_select_L2:adaptive"],
                       "ls_global": e["subg_ls_global:L2:adaptive"], "ls_tile": e["subg_ls_tile:L2:adaptive"]}
    static_models = {"L2": e["subg_med:L2:mm"], "L3_P": r["static_cal:P"], "L2_parity": r["static_cal:parity"], "L2_phase8": r["static_cal:phase8"],
                     "pred_select_L2": r["pred_select_L2:mm"], "ls_global": e["subg_ls_global:L2:mm"], "ls_tile": e["subg_ls_tile:L2:mm"]}
    bm = min(adaptive_models, key=adaptive_models.get)
    r["best_adaptive_model"], r["best_adaptive_bpp"] = bm, adaptive_models[bm]
    bs = min(static_models, key=static_models.get)
    r["lowest_static_model"], r["lowest_static_bpp"] = bs, static_models[bs]
    if n in jpg:
        j = jpg[n]
        for k in ("subsampling", "ijg_quality_est", "luma_q_mean", "progressive"):
            r[f"jpeg_{k}"] = j[k]
        r["src_jpeg_bpp_full"] = float(j["src_payload_bpp_fullimage"])
        r["src_jpeg_bpp_crop_adj"] = float(j["src_payload_bpp_crop_adjusted"])
        r["crop_share_of_full"] = float(j["crop_share_of_full"])
        r["lattice_aligned"], r["lattice_offset4"] = num(j["lattice_aligned"]), num(j["lattice_offset4"])
    # gap indicators, % of PKON2 bits (negative = the reference/codec needs fewer bits than PKON2)
    pct = lambda v: (v / pk - 1) * 100
    r["gap%:best_other_codec"] = pct(others[best])
    r["gap%:L3_static"], r["gap%:L3_adaptive"] = pct(r["static_cal:P"]), pct(e["subg_med:L3_P:adaptive"])
    r["gap%:L2_static"], r["gap%:L2_adaptive"] = pct(e["subg_med:L2:mm"]), pct(e["subg_med:L2:adaptive"])
    r["gap%:best_adaptive"], r["gap%:lowest_static"] = pct(r["best_adaptive_bpp"]), pct(r["lowest_static_bpp"])
    base = e["subg_med:L2:adaptive"]
    d = lambda v: (v - base) / pk * 100
    r["probe%:ls_global"] = d(e["subg_ls_global:L2:adaptive"])
    r["probe%:ls_tile"] = d(e["subg_ls_tile:L2:adaptive"])
    r["probe%:pred_select"] = d(r["pred_select_L2:adaptive"])
    for f in PROBES:
        r[f"probe%:{f}_net_mm"] = r[f"net:{f}:mm"] / pk * 100
        r[f"probe%:{f}_net_adaptive"] = r[f"net:{f}:adaptive"] / pk * 100
        r[f"probe%:{f}_vs_L2_adaptive"] = d(e[f"subg_med:{PROBES[f][0]}:adaptive"])
    r["learning_cost%:L3"] = (e["subg_med:L3_P:adaptive"] - r["static_cal:P"]) / pk * 100
    # optimistic envelope: PKON2 minus the summed static net signals of the three probes PKON2 does not use (assumes additivity and full recovery)
    r["signal:pred_select_mm"] = r["pred_select_L2:mm"] - e["subg_med:L2:mm"]
    r["envelope_unexploited_bpp"] = pk + sum(min(0.0, s) for s in (r["net:phase8:mm"], r["net:parity:mm"], r["signal:pred_select_mm"]))
    r["gap%:envelope"] = pct(r["envelope_unexploited_bpp"])
    r["xchan%:transform_zero_order"] = (e["H0_med_rgb"] - e["H0_med_subg"]) / pk * 100
    r["xchan%:transform_under_L2"] = (e["rgb_med:L2:adaptive"] - e["subg_med:L2:adaptive"]) / pk * 100
    r["xchan%:prevplane_ctx"] = (e["subg_med:L1:adaptive"] - e["subg_med:L2:adaptive"]) / pk * 100
    if n in jpg:
        r["gap%:src_jpeg_full"], r["gap%:src_jpeg_crop_adj"] = pct(r["src_jpeg_bpp_full"]), pct(r["src_jpeg_bpp_crop_adj"])
    rows.append(r)
R_ = {r["image"]: r for r in rows}
wcsv(f"{DATA}/map_data.csv", rows)

# ---------------------------------------------------------------- aggregates (bits summed over images / pixels summed; all crops are 1024^2)
mean = lambda k, sub=names: float(np.mean([R_[n][k] for n in sub]))
JP = [n for n in names if n in jpg]
agg = []
def A(kind, key, label, value, lo=None, hi=None, subset="all 25", note=""):
    agg.append({"kind": kind, "key": key, "label": label, "bpp": value, "band_low": lo, "band_high": hi, "subset": subset, "note": note})
A("measured", "raw", "Uncompressed crop", mean("raw_bpp"), note="23 RGB at 24 bpp + 2 RGBA at 32 bpp")
for c in CODECS:
    A("measured", c, LAB[c], mean(f"bpp:{c}"))
A("empirical", "H0_channels", "Zero-order pixel histogram, per channel (sum)", mean("H0_channels"))
A("empirical", "H0_subg_planes", "Zero-order, after SubG colour transform", mean("H0_subg_planes"))
A("empirical", "H0_med_rgb", "Zero-order MED residual, RGB planes", mean("H0_med_rgb"))
A("empirical", "H0_med_subg", "Zero-order MED residual, SubG planes (PKON2 symbols)", mean("H0_med_subg"))
NOTE_S = "band_low = static (Miller-Madow); band_high = adaptive (KT)"
NOTE_C = "band_low = L2 static + net signal vs same-cardinality random context; band_high = adaptive (KT)"
for mo, lab in (("L1", "Conditional: spatial contexts"), ("L2", "Conditional: + previous-plane contexts")):
    A("estimate", f"subg_med:{mo}", lab, mean(f"subg_med:{mo}:adaptive"), mean(f"subg_med:{mo}:mm"), mean(f"subg_med:{mo}:adaptive"), note=NOTE_S)
for f, lab in (("P", "Conditional: + P feature (all PKON2 features)"), ("parity", "Probe: L2 + pixel parity"), ("phase8", "Probe: L2 + 8x8 JPEG-grid phase")):
    mo = PROBES[f][0]
    A("estimate", f"subg_med:{mo}", lab, mean(f"subg_med:{mo}:adaptive"), mean(f"static_cal:{f}"), mean(f"subg_med:{mo}:adaptive"), note=NOTE_C)
    A("estimate", f"net:{f}", f"Net signal of {f} vs same-cardinality random context (static, Miller-Madow)", mean(f"net:{f}:mm"),
      note=f"bpp, negative = information beyond the L2 contexts; adaptive-estimator net {mean(f'net:{f}:adaptive'):.3f} "
           "(inflated: a uniform random split pays more learning cost than a real, uneven one)")
for dom, lab in (("subg_ls_global", "Probe: global LS predictor + L2 contexts"), ("subg_ls_tile", "Probe: per-tile LS predictor + L2 contexts")):
    A("estimate", dom, lab, mean(f"{dom}:L2:adaptive"), mean(f"{dom}:L2:mm"), mean(f"{dom}:L2:adaptive"), note=NOTE_S + "; adaptive includes 16 bits per LS coefficient")
A("estimate", "pred_select", "Probe: per-plane choice MED vs tile-LS (L2)", mean("pred_select_L2:adaptive"), mean("pred_select_L2:mm"), mean("pred_select_L2:adaptive"), note=NOTE_S)
A("estimate", "best_adaptive", "Lowest adaptive estimate per image (single probe)", mean("best_adaptive_bpp"))
A("estimate", "lowest_static", "Lowest calibrated static estimate per image (single probe)", mean("lowest_static_bpp"))
A("hypothesis", "envelope", "Optimistic envelope: PKON2 minus summed net signals of phase, parity, predictor choice", mean("envelope_unexploited_bpp"),
  note="assumes the three signals are additive, not already captured by PKON2's mixer, and fully recoverable - none of which is established")
A("measured", "pkon2-file@jpeg23", "PKON2 on the 23 JPEG-origin images", mean("bpp:pkon2-file", JP), subset="23 JPEG-origin")
A("measured", "best@jpeg23", "Best measured codec per image, 23 JPEG-origin", mean("best_measured_bpp", JP), subset="23 JPEG-origin")
A("provenance", "src_jpeg_full", "Source JPEG payload, full-image average", mean("src_jpeg_bpp_full", JP), subset="23 JPEG-origin",
  note="bits of the original JPEG file per decoded pixel over the WHOLE photograph, not the crop")
A("provenance", "src_jpeg_crop_adj", "Source JPEG payload, crop-adjusted (rough)", mean("src_jpeg_bpp_crop_adj", JP), subset="23 JPEG-origin",
  note="full-image average x crop/full MED-residual entropy ratio (model-dependent)")
G = {"G_H0": "H(G) zero-order", "G_H_given_W": "H(G | W)", "G_H_given_WN": "H(G | W, N)"}
for k, lab in G.items():
    if k == "G_H0":
        A("empirical", k, f"G plane: {lab}", mean(k), note="bits per G sample")
    else:
        A("estimate", k, f"G plane: {lab}", mean(f"{k}_kt"), mean(f"{k}_mm"), mean(f"{k}_kt"), note=f"bits per G sample; raw plug-in {mean(k + '_plugin'):.3f}")
wcsv(f"{DATA}/aggregate.csv", agg)
AG = {a["key"]: a for a in agg}

# per-plane summary (aggregate bits per sample) for the PKON2 representation
prow = []
for dom in ("subg_med", "rgb_med", "subg_ls_tile"):
    for k, pn in enumerate(["G", "R-G" if dom != "rgb_med" else "R", "B-G" if dom != "rgb_med" else "B"]):
        for mo in ("L0", "L1", "L2", "L3_P", "L2_null49"):
            vals = [plane[(n, dom, k)].get(mo) for n in names]
            if all(v is not None for v in vals):
                prow.append({"domain": dom, "plane": pn, "model": mo, "plugin_bits_per_sample": float(np.mean([v[0] for v in vals])),
                             "mm_bits_per_sample": float(np.mean([v[1] for v in vals])), "adaptive_bits_per_sample": float(np.mean([v[2] for v in vals]))})
wcsv(f"{DATA}/plane_summary.csv", prow)

# per-group summary
crow = []
for g, ns in GROUPS:
    jn = [n for n in ns if n in jpg]
    crow.append({"group": g, "images": " ".join(ns), "n": len(ns), "pkon2_bpp": mean("bpp:pkon2-file", ns), "jxl_e9_bpp": mean("bpp:jxl-e9-t1", ns),
                 "jxl_e7_bpp": mean("bpp:jxl-e7-t1", ns), "webp_bpp": mean("bpp:webp-m6-t1", ns), "png_bpp": mean("bpp:png-rs", ns),
                 "best_measured_bpp": mean("best_measured_bpp", ns), "H0_channels": mean("H0_channels", ns), "H0_med_subg": mean("H0_med_subg", ns),
                 "L3_static_cal": mean("static_cal:P", ns), "L3_adaptive": mean("subg_med:L3_P:adaptive", ns),
                 "best_adaptive": mean("best_adaptive_bpp", ns), "lowest_static_cal": mean("lowest_static_bpp", ns),
                 "net_phase8_mm": mean("net:phase8:mm", ns), "net_parity_mm": mean("net:parity:mm", ns), "net_P_mm": mean("net:P:mm", ns),
                 "learning_cost%_L3": mean("learning_cost%:L3", ns), "repeat_hit_of_med_miss": mean("repeat_hit_share_of_med_miss", ns),
                 "src_jpeg_bpp_full": mean("src_jpeg_bpp_full", jn) if jn else None, "n_jpeg": len(jn)})
wcsv(f"{DATA}/category_summary.csv", crow)


# ---------------------------------------------------------------- figure helpers
def hgrid(ax, axis="x"):
    ax.grid(True, axis=axis, color=GRID, lw=0.6, zorder=0)
    ax.set_axisbelow(True)
    for s in ("top", "right"):
        ax.spines[s].set_visible(False)


def logx(ax, lo, hi, ticks):
    ax.set_xscale("log", base=2)
    ax.set_xlim(lo, hi)
    ax.set_xticks(ticks)
    ax.set_xticklabels([f"{t:g}" for t in ticks])
    ax.minorticks_off()


def row_layout():
    """y positions top-down: aggregate row, then groups with a gap; returns (ypos by image, group header positions, ylim)."""
    y, pos, heads = 0.0, {}, []
    pos["__all__"] = y
    y += 1.5
    for g, ns in GROUPS:
        heads.append((g, y - 0.15))
        y += 0.55
        for n in sorted(ns, key=lambda n: R_[n]["bpp:pkon2-file"]):
            pos[n] = y
            y += 1.0
        y += 0.35
    return pos, heads, y


def draw_rows(ax, pos, heads, ymax, show_names=True):
    """Layers A and B per image on a log2 bpp axis."""
    rows_ = [("__all__", None)] + [(n, R_[n]) for n in names]
    for n, r in rows_:
        y = pos[n]
        if n == "__all__":
            vals = {"H0_channels": AG["H0_channels"]["bpp"], "H0_med_subg": AG["H0_med_subg"]["bpp"],
                    "l3lo": AG["subg_med:L3_P"]["band_low"], "l3hi": AG["subg_med:L3_P"]["band_high"],
                    "best_adaptive": AG["best_adaptive"]["bpp"], "lowest_static": AG["lowest_static"]["bpp"], "src": None}
            cod = {c: AG[c]["bpp"] for c in CODECS}
            ax.axhspan(y - 0.6, y + 0.6, color="#f0efec", zorder=0, lw=0)
        else:
            vals = {"H0_channels": r["H0_channels"], "H0_med_subg": r["H0_med_subg"], "l3lo": r["static_cal:P"],
                    "l3hi": r["subg_med:L3_P:adaptive"], "best_adaptive": r["best_adaptive_bpp"], "lowest_static": r["lowest_static_bpp"],
                    "src": r.get("src_jpeg_bpp_full"), "src_adj": r.get("src_jpeg_bpp_crop_adj")}
            cod = {c: r[f"bpp:{c}"] for c in CODECS}
        lo = min([vals["lowest_static"]] + list(cod.values()) + ([vals["src"]] if vals.get("src") else []))
        yc, yb = y - 0.1, y + 0.3  # codec markers / model-estimate track just beneath them
        ax.plot([lo, vals["H0_channels"]], [yc, yc], color=GRID, lw=1.0, zorder=1, solid_capstyle="butt")
        # layer B references (achromatic)
        ax.plot([vals["l3lo"], vals["l3hi"]], [yb, yb], color=BAND2, lw=3.2, solid_capstyle="butt", zorder=2)
        ax.plot([vals["lowest_static"]], [yb], marker="|", color=INK, ms=7, mew=1.3, zorder=2)
        ax.plot([vals["H0_med_subg"]], [yc], marker="o", mfc=SURF, mec=INK2, ms=5.5, mew=1.1, zorder=3)
        ax.plot([vals["H0_channels"]], [yc], marker="s", mfc=SURF, mec=MUTED, ms=5, mew=1.1, zorder=3)
        if vals.get("src"):
            ax.plot([vals["src_adj"], vals["src"]], [yc, yc], color=INK2, lw=0.8, zorder=2)
            ax.plot([vals["src_adj"]], [yc], marker="*", mfc=SURF, mec=INK, ms=6, mew=0.8, zorder=3)
            ax.plot([vals["src"]], [yc], marker="*", mfc=INK, mec=INK, ms=8, mew=0.6, zorder=3)
        # layer A measured codecs (PKON2 underneath so the smaller markers stay visible on top of it)
        for zo, c in enumerate(CODECS):
            s = STY[c]
            ax.plot([cod[c]], [yc], ls="none", marker=s["marker"], mfc=s["mfc"], mec=s["color"], ms=s["ms"], mew=1.3, zorder=5 + zo)
        if show_names:
            lab = "All 25 (aggregate)" if n == "__all__" else n
            ax.text(-0.01, y, lab, transform=ax.get_yaxis_transform(), ha="right", va="center", fontsize=8.5,
                    color=INK if n == "__all__" else INK2, fontweight="bold" if n == "__all__" else "normal")
    for g, y in heads:
        ax.text(-0.01, y, g.upper(), transform=ax.get_yaxis_transform(), ha="right", va="center", fontsize=7, color=MUTED, fontweight="bold")
        ax.axhline(y + 0.12, color=GRID, lw=0.5, zorder=0)
    ax.set_ylim(ymax, -1)
    ax.set_yticks([])
    ax.spines["left"].set_visible(False)
    ax.axvline(24, color=AXIS, lw=0.8, zorder=0)
    ax.text(24, -0.85, "raw 24 bpp", fontsize=7, color=MUTED, ha="center", va="bottom")


def legend_handles():
    meas = [Line2D([], [], ls="none", marker=STY[c]["marker"], mfc=STY[c]["mfc"], mec=STY[c]["color"], ms=STY[c]["ms"], mew=1.3, label=LAB[c]) for c in CODECS]
    refs = [Line2D([], [], ls="none", marker="s", mfc=SURF, mec=MUTED, ms=5, mew=1.1, label="Zero-order pixel histogram, ΣH(channel)  [computed]"),
            Line2D([], [], ls="none", marker="o", mfc=SURF, mec=INK2, ms=5.5, mew=1.1, label="Zero-order entropy of PKON2's MED/SubG residual  [computed]"),
            Line2D([], [], color=BAND2, lw=3.2, label="Conditional entropy given all PKON2 causal features (track under\nthe codecs): static, null-calibrated (left) … adaptive (right)  [estimate]"),
            Line2D([], [], ls="none", marker="|", color=INK, ms=7, mew=1.3, label="Lowest calibrated static estimate, any single probe  [estimate]"),
            Line2D([], [], ls="none", marker="*", mfc=INK, mec=INK, ms=8, label="Source JPEG file, bits per decoded pixel (whole photo)  [measured file size]"),
            Line2D([], [], ls="none", marker="*", mfc=SURF, mec=INK, ms=6, mew=0.8, label="… rescaled to the crop's relative complexity  [rough estimate]")]
    return meas, refs


pk_total = AG["pkon2-file"]["bpp"]
rel = lambda v: (v / pk_total - 1) * 100
l3lo, l3hi = rel(AG["subg_med:L3_P"]["band_low"]), rel(AG["subg_med:L3_P"]["band_high"])
ba, bp, env = rel(AG["best_adaptive"]["bpp"]), rel(AG["lowest_static"]["bpp"]), rel(AG["envelope"]["bpp"])
src_rel = (AG["src_jpeg_full"]["bpp"] / AG["pkon2-file@jpeg23"]["bpp"] - 1) * 100
YLO = env - 3.5


def frontier(ax, kind, xlim, label_ms=False, legend=True):
    """Layer C: measured (time, size-vs-PKON2) points, the model-estimate band/lines (no compute position) and the hypothesis region."""
    for c in CODECS:
        s = STY[c]
        xs = [R_[n][f"{kind}_ms:{c}"] for n in names]
        ys = [(R_[n][f"bpp:{c}"] / R_[n]["bpp:pkon2-file"] - 1) * 100 for n in names]
        ax.plot(xs, ys, ls="none", marker=s["marker"], mfc=s["mfc"], mec=s["color"], ms=s["ms"] * 0.62, mew=0.8, alpha=0.4, zorder=3)
        ex, ey = float(np.mean(xs)), rel(AG[c]["bpp"])
        ax.plot([ex], [ey], ls="none", marker=s["marker"], mfc=s["mfc"], mec=s["color"], ms=s["ms"] * 1.45, mew=1.8, zorder=6)
        fl = floor[c][0 if kind == "enc" else 1]
        if fl > 5:
            ax.plot([ex - fl, ex], [ey, ey], color=s["color"], lw=0.8, zorder=5)
            ax.plot([ex - fl], [ey], ls="none", marker="|", color=s["color"], ms=9, mew=1.2, zorder=5)
        txt = f"{SHORT[c]}  {ex:,.0f} ms" if label_ms else SHORT[c]
        dy = -12 if c == "jxl-e9-t1" and kind == "dec" else 5
        ax.annotate(txt, (ex, ey), xytext=(9, dy), textcoords="offset points", fontsize=8.5, color=INK, fontweight="bold" if c == "pkon2-file" else "normal")
    xpk = float(np.mean([R_[n][f"{kind}_ms:pkon2-file"] for n in names]))
    ax.axhspan(l3lo, l3hi, color=BAND, alpha=0.55, lw=0, zorder=1)
    ax.axhline(ba, color=BAND2, lw=1.0, zorder=2)
    ax.axhline(bp, color=INK2, lw=0.9, zorder=2)
    ax.add_patch(Rectangle((xpk * 1.5, env), xlim[1] - xpk * 1.5, -env, facecolor="none", edgecolor=HATCH, hatch="////", lw=0, zorder=1))
    ax.text(xlim[1] / 1.12, env + 0.25, "HYPOTHESIS · more modelling effort at unknown compute;\nlower edge = optimistic envelope of unexploited signals",
            fontsize=7.5, color=INK2, ha="right", va="bottom", zorder=7, bbox=dict(facecolor=SURF, edgecolor="none", pad=1.5))
    ax.axhline(0, color=AXIS, lw=0.8, zorder=1)
    ax.set_xscale("log")
    ax.set_xlim(*xlim)
    ax.set_ylim(YLO, 24)
    hgrid(ax, "both")
    if kind == "enc":
        ax.annotate(f"source JPEG files {src_rel:+.0f}% (23 JPEG crops; off scale)", xy=(xlim[0] * 2.2, YLO), xytext=(xlim[0] * 2.2, YLO + 1.2),
                    fontsize=7.5, color=INK2, arrowprops=dict(arrowstyle="-|>", color=INK2, lw=0.8), ha="left", va="bottom")
    if legend:
        ax.legend(handles=[Patch(facecolor=BAND, alpha=0.55, label=f"conditional entropy, PKON2 features\nstatic … adaptive ({l3lo:+.1f}% … {l3hi:+.1f}%)"),
                           Line2D([], [], color=BAND2, lw=1.0, label=f"lowest adaptive estimate ({ba:+.1f}%)"),
                           Line2D([], [], color=INK2, lw=0.9, label=f"lowest calibrated static ({bp:+.1f}%)"),
                           Patch(facecolor="none", edgecolor=HATCH, hatch="////", label=f"hypothesis; envelope {env:+.1f}%")],
                  loc="upper left", fontsize=7.5, frameon=True, facecolor=SURF, edgecolor=GRID, title="estimates: no compute position", title_fontsize=7)


# ================================================================ FIGURE 1: the reference map
pos, heads, ymax = row_layout()
fig = plt.figure(figsize=(17.5, 15))
gs = fig.add_gridspec(3, 2, width_ratios=[1.3, 1], height_ratios=[1.0, 1.0, 0.27], wspace=0.47, hspace=0.3, left=0.12, right=0.985, top=0.915, bottom=0.03)
axA = fig.add_subplot(gs[0:2, 0])
draw_rows(axA, pos, heads, ymax)
logx(axA, 0.4, 28, [0.5, 1, 2, 4, 8, 16, 24])
hgrid(axA)
axA.set_xlabel("bits per pixel (all channels; log scale)")
axA.set_title("A + B · Per image: measured codecs (colour) against entropy references (grey/black)", loc="left", fontsize=10.5, color=INK, pad=14)

# ---- right top: aggregate ladder (Layer B)
axL = fig.add_subplot(gs[0, 1])
ladder = [("raw", "Uncompressed crop", "measured"), ("H0_channels", "Pixel histogram, per channel", "empirical"),
          ("H0_subg_planes", "… after SubG colour transform", "empirical"), ("H0_med_rgb", "MED residual, RGB planes", "empirical"),
          ("H0_med_subg", "MED residual, SubG (PKON2 symbols)", "empirical"), ("png-rs", "PNG", "measured"),
          ("subg_med:L1", "H(r | spatial ctx)", "estimate"), ("webp-m6-t1", "WebP m6", "measured"),
          ("subg_med:L2", "H(r | + previous plane)", "estimate"), ("jxl-e7-t1", "JPEG XL e7", "measured"),
          ("jxl-e9-t1", "JPEG XL e9", "measured"), ("subg_med:L3_P", "H(r | + P) = all PKON2 features", "estimate"),
          ("subg_med:L2_phase8", "Probe: L2 + JPEG 8×8 phase", "estimate"),
          ("pkon2-file", "PKON2 REL6", "measured"), ("best_adaptive", "Lowest adaptive, per image", "estimate"),
          ("lowest_static", "Lowest calibrated static, per image", "estimate"), ("envelope", "Optimistic envelope (hypothesis)", "hypothesis")]
ladder.sort(key=lambda t: -(AG[t[0]]["band_low"] if AG[t[0]]["band_low"] is not None else AG[t[0]]["bpp"]))
for i, (k, lab, kind) in enumerate(ladder):
    a = AG[k]
    if kind == "measured":
        col = STY[k]["color"] if k in STY else INK2
        axL.barh(i, a["bpp"], height=0.62, color=col, alpha=0.9 if k in STY else 0.35, lw=0, zorder=2)
        txt = f"{a['bpp']:.2f}"
    elif kind == "empirical":
        axL.barh(i, a["bpp"], height=0.62, color=SURF, edgecolor=INK2, lw=0.9, zorder=2)
        txt = f"{a['bpp']:.2f}"
    elif kind == "hypothesis":
        axL.barh(i, a["bpp"], height=0.62, color=SURF, edgecolor=HATCH, hatch="////", lw=0.6, zorder=2)
        txt = f"{a['bpp']:.2f}"
    else:
        lo, hi = (a["band_low"], a["band_high"]) if a["band_low"] is not None else (a["bpp"], a["bpp"])
        axL.barh(i, lo, height=0.62, color=BAND, lw=0, zorder=2)
        if hi > lo:
            axL.barh(i, hi - lo, left=lo, height=0.62, color=SURF, edgecolor=BAND2, hatch="////", lw=0.6, zorder=2)
        txt = f"{lo:.2f}–{hi:.2f}" if hi > lo + 0.005 else f"{lo:.2f}"
    end = a["band_high"] if a["band_high"] is not None else a["bpp"]
    axL.text(end + 0.25, i, txt, va="center", fontsize=8, color=INK)
    axL.text(-0.3, i, lab, va="center", ha="right", fontsize=8.2, color=INK if kind == "measured" else INK2,
             fontweight="bold" if k == "pkon2-file" else "normal")
axL.set_ylim(len(ladder) - 0.4, -0.6)
axL.set_yticks([])
axL.set_xlim(0, 27.5)
axL.spines["left"].set_visible(False)
hgrid(axL)
axL.set_xlabel("bits per pixel, mean over the 25 crops (= total bits / total pixels)")
axL.set_title("B · Aggregate information ladder: what each description needs", loc="left", fontsize=10.5, color=INK, pad=8)
axL.legend(handles=[Patch(facecolor="#2a78d6", label="measured file size"), Patch(facecolor=SURF, edgecolor=INK2, label="computed directly from pixels"),
                    Patch(facecolor=BAND, label="estimate · static (statistics known; bias-/null-corrected)"),
                    Patch(facecolor=SURF, edgecolor=BAND2, hatch="////", label="… + cost of learning them (adaptive)")],
           loc="lower right", fontsize=7.5, frameon=False)

# ---- right middle: frontier vs compute (Layer C)
axC = fig.add_subplot(gs[1, 1])
frontier(axC, "enc", (20, 60000))
axC.set_xlabel("encode time per 1-MP crop, ms (log; median of 3 reps; single thread)")
axC.set_ylabel("size vs PKON2 (%) · small = image, large = aggregate")
axC.set_title("C · Practical frontier: measured points, estimate band, hypothesis region", loc="left", fontsize=10.5, color=INK, pad=8)

# ---- legend strip
axG = fig.add_subplot(gs[2, :])
axG.axis("off")
meas, refs = legend_handles()
l1 = axG.legend(handles=meas, title="MEASURED (bench25, 1024×1024 centre crops)", loc="upper left", bbox_to_anchor=(0.0, 1.0), fontsize=8.5,
                title_fontsize=8.5, frameon=False, alignment="left")
axG.add_artist(l1)
l2 = axG.legend(handles=refs, title="COMPUTED / ESTIMATED FROM THE SAME CROPS", loc="upper left", bbox_to_anchor=(0.2, 1.0), fontsize=8.5,
                title_fontsize=8.5, frameon=False, alignment="left")
axG.add_artist(l2)
concept = [Patch(facecolor="none", edgecolor=HATCH, hatch="////", label="Hypothesis region / optimistic envelope — no measured points inside"),
           Patch(facecolor=BAND, alpha=0.55, label="Estimate band drawn without a compute position")]
axG.legend(handles=concept, title="CONCEPTUAL (NOT MEASURED)", loc="upper left", bbox_to_anchor=(0.62, 1.0), fontsize=8.5, title_fontsize=8.5, frameon=False, alignment="left")
axG.text(0.62, 0.5, "No value on this map is a lower bound on achievable size. Entropy estimates are\nrelative to their model class; a richer model can go below them.\n"
         "'Static' = statistics known for free (bias- and null-corrected);\n'adaptive' = learned from the image itself (Dirichlet/KT code length).",
         transform=axG.transAxes, fontsize=8, color=INK2, va="top")
fig.suptitle("PKON2 compression landscape — reference map (bench25 diversity set, 25 × 1024² crops)", x=0.12, y=0.985, ha="left", fontsize=14, color=INK, fontweight="bold")
fig.text(0.12, 0.957, "Measured: results/20261028-015857Z-bench25-diversity. Estimates: this directory's data/, computed from crops verified "
         "byte-identical (SHA-256) to the benchmark inputs.", fontsize=8.5, color=INK2)
save(fig, "fig1_reference_map")


# ================================================================ FIGURE 2: information landscape details
fig, axs = plt.subplots(1, 3, figsize=(17, 5.6), gridspec_kw={"width_ratios": [1.05, 1, 1.1], "wspace": 0.42})
# (a) G plane: direct spatial conditioning vs prediction + residual contexts
ax = axs[0]
gp = lambda dom, mo, i: float(np.mean([plane[(n, dom, 0)][mo][i] for n in names]))
g_l3_cal = gp("subg_med", "L2", 1) + gp("subg_med", "L3_P", 1) - gp("subg_med", "L2_null49", 1)
items = [("H(G)  zero-order", AG["G_H0"]["bpp"], AG["G_H0"]["bpp"], "empirical", None),
         ("H(G | W)", AG["G_H_given_W"]["band_low"], AG["G_H_given_W"]["band_high"], "estimate", mean("G_H_given_W_plugin")),
         ("H(G | W, N)", AG["G_H_given_WN"]["band_low"], AG["G_H_given_WN"]["band_high"], "estimate", mean("G_H_given_WN_plugin")),
         ("H(r)  MED residual, zero-order", gp("subg_med", "L0", 0), gp("subg_med", "L0", 0), "empirical", None),
         ("H(r | spatial ctx)", gp("subg_med", "L1", 1), gp("subg_med", "L1", 2), "estimate", gp("subg_med", "L1", 0)),
         ("H(r | spatial ctx, P)", g_l3_cal, gp("subg_med", "L3_P", 2), "estimate", gp("subg_med", "L3_P", 0))]
for i, (lab, lo, hi, kind, raw) in enumerate(items):
    if kind == "empirical":
        ax.barh(i, lo, height=0.6, color=SURF, edgecolor=INK2, lw=0.9)
        t = f"{lo:.2f}"
    else:
        ax.barh(i, lo, height=0.6, color=BAND, lw=0)
        ax.barh(i, hi - lo, left=lo, height=0.6, color=SURF, edgecolor=BAND2, hatch="////", lw=0.6)
        ax.plot([raw], [i], marker="|", color=INK, ms=13, mew=1.2)
        t = f"{lo:.2f} static … {hi:.2f} adaptive"
    ax.text(max(lo, hi) + 0.1, i, t, va="center", fontsize=8, color=INK)
ax.text(0.99, 0.02, "| = raw plug-in (overfits when contexts are many)", transform=ax.transAxes, fontsize=7.5, color=MUTED, ha="right")
ax.set_yticks(range(len(items)))
ax.set_yticklabels([t[0] for t in items], fontsize=8.5)
ax.set_ylim(len(items) - 0.4, -0.6)
ax.set_xlim(0, 9.5)
hgrid(ax)
ax.set_xlabel("bits per G sample (mean of 25 crops)")
ax.set_title("a · Spatial conditioning, G plane\nraw-neighbour contexts collapse under learning cost", loc="left", fontsize=10, color=INK)
# (b) cross-channel: what SubG and previous-plane contexts remove, per image
ax = axs[1]
ys = np.arange(len(names))
order = sorted(names, key=lambda n: R_[n]["xchan%:transform_zero_order"])
for i, n in enumerate(order):
    r = R_[n]
    a, b_ = r["xchan%:transform_zero_order"], r["xchan%:transform_under_L2"]
    ax.plot([b_, a], [i, i], color=GRID, lw=1.2, zorder=1)
    ax.plot([a], [i], marker="o", mfc=SURF, mec=INK2, ms=5.5, mew=1.1, ls="none", zorder=3)
    ax.plot([b_], [i], marker="o", mfc=BAND2, mec=BAND2, ms=5.5, ls="none", zorder=3)
    ax.plot([r["xchan%:prevplane_ctx"]], [i], marker="|", color=INK, ms=8, mew=1.3, ls="none", zorder=3)
ax.set_yticks(ys)
ax.set_yticklabels(order, fontsize=7.5)
ax.set_ylim(len(order) - 0.5, -0.5)
ax.axvline(0, color=AXIS, lw=0.8)
hgrid(ax)
ax.set_xlabel("bits removed, % of PKON2's file size")
ax.set_title("b · Cross-channel redundancy\nSubG transform and previous-plane contexts", loc="left", fontsize=10, color=INK)
ax.legend(handles=[Line2D([], [], ls="none", marker="o", mfc=SURF, mec=INK2, label="SubG vs RGB, zero-order residual entropy"),
                   Line2D([], [], ls="none", marker="o", mfc=BAND2, mec=BAND2, label="SubG vs RGB, both with L2 contexts (adaptive)"),
                   Line2D([], [], ls="none", marker="|", color=INK, ms=8, mew=1.3, label="previous-plane contexts on top of SubG (L1→L2)")],
          loc="upper center", bbox_to_anchor=(0.45, -0.1), fontsize=7.5, frameon=False)
# (c) per-plane: where the bits sit in the PKON2 representation
ax = axs[2]
P = {(p["domain"], p["plane"], p["model"]): p for p in prow}
planes_ = ["G", "R-G", "B-G"]
mods = [("L0", "zero-order"), ("L1", "spatial ctx"), ("L2", "+ prev-plane"), ("L3_P", "+ P")]
w = 0.19
for j, (mo, ml) in enumerate(mods):
    for i, pn in enumerate(planes_):
        p = P[("subg_med", pn, mo)]
        x = i + (j - 1.5) * (w + 0.02)
        lo, hi = p["mm_bits_per_sample"], p["adaptive_bits_per_sample"]
        if mo == "L3_P":  # null-calibrated static edge
            lo = P[("subg_med", pn, "L2")]["mm_bits_per_sample"] + lo - P[("subg_med", pn, "L2_null49")]["mm_bits_per_sample"]
        shade = ["#e9e8e3", "#d4d2ca", BAND, BAND2][j]
        ax.bar(x, lo, width=w, color=shade, lw=0, zorder=2)
        if hi - lo > 0.004:
            ax.bar(x, hi - lo, bottom=lo, width=w, color=SURF, edgecolor=BAND2, hatch="////", lw=0.5, zorder=2)
        if i == 0:
            ax.text(x, hi + 0.05, ml, rotation=90, fontsize=7, color=INK2, ha="center", va="bottom")
        ax.text(x, -0.08, f"{lo:.2f}", rotation=90, fontsize=6.5, color=INK2, ha="center", va="top")
ax.set_xticks(range(3))
ax.set_xticklabels([f"plane {k}: {pn}" for k, pn in enumerate(planes_)])
ax.tick_params(axis="x", pad=26)
ax.set_ylim(-0.0, 4.6)
hgrid(ax, "y")
ax.set_ylabel("bits per sample (mean of 25 crops)")
ax.set_title("c · Where the bits sit: PKON2's three coded planes\nstatic (solid) + learning cost (hatched)", loc="left", fontsize=10, color=INK)
fig.suptitle("Information landscape — conditioning, cross-channel dependency and per-plane entropy (computed from the 25 crops)",
             x=0.06, ha="left", y=1.04, fontsize=12.5, color=INK, fontweight="bold")
save(fig, "fig2_information_landscape")


# ================================================================ FIGURE 3: frontier vs compute (encode and decode)
fig, axs = plt.subplots(1, 2, figsize=(16, 6.4), sharey=True, gridspec_kw={"wspace": 0.06})
for ax, kind, xlab in ((axs[0], "enc", "encode"), (axs[1], "dec", "decode")):
    frontier(ax, kind, (5, 100000), label_ms=True, legend=kind == "enc")
    ax.set_xlabel(f"{xlab} time per 1-MP crop, ms (log; median of 3 reps; single thread)")
    ax.set_title(f"{'a' if kind == 'enc' else 'b'} · Size vs {xlab} time", loc="left", fontsize=10.5, color=INK)
axs[0].set_ylabel("size vs PKON2 (%); small marks = images, large = aggregate")
axs[1].text(0.01, 0.02, "| = measured mean minus the ffmpeg no-op process floor (33×17 image; JXL/WebP only)\n"
            "off-scale points omitted: PNG > +24% on most images; WebP −49% and JXL −11…−18% on screen_0",
            transform=axs[1].transAxes, fontsize=7.5, color=MUTED, ha="left", va="bottom")
fig.suptitle("Practical frontier — compression efficiency vs computational effort (measured points only; regions are conceptual)",
             x=0.07, ha="left", y=1.0, fontsize=12.5, color=INK, fontweight="bold")
save(fig, "fig3_frontier_compute")


# ================================================================ FIGURE 4: headroom indicators per image
cols = [("Measured gap\nother codecs vs PKON2", "meas"), ("Model reference vs PKON2\nL2 and L3 bands", "model"),
        ("Predictor probe\nLS vs MED, both + L2 ctx", "pred"), ("Context probes, net of\nsame-size random context", "ctx"),
        ("Learning cost\nL3 adaptive − static", "learn"), ("Non-local repetition\nexact-context repeat hits", "rep"),
        ("Provenance\nsource JPEG vs PKON2", "src")]
fig, axs = plt.subplots(1, len(cols), figsize=(19, 11.5), sharey=True, gridspec_kw={"wspace": 0.12})
yp = {n: pos[n] for n in names}
for ax, (title, key) in zip(axs, cols):
    for g, y in heads:
        ax.axhline(y + 0.12, color=GRID, lw=0.5, zorder=0)
    for n in names:
        r, y = R_[n], yp[n]
        if key == "meas":
            for c in CODECS[1:4]:
                v = (r[f"bpp:{c}"] / r["bpp:pkon2-file"] - 1) * 100
                s = STY[c]
                ax.plot([max(min(v, 48), -48)], [y], ls="none", marker=s["marker"], mfc=s["mfc"], mec=s["color"], ms=s["ms"] * 0.85, mew=1.1, zorder=3)
        elif key == "model":
            ax.plot([r["gap%:L2_static"], r["gap%:L2_adaptive"]], [y - 0.17] * 2, color="#d4d2ca", lw=4.5, solid_capstyle="round")
            ax.plot([r["gap%:L3_static"], r["gap%:L3_adaptive"]], [y + 0.17] * 2, color=BAND2, lw=4.5, solid_capstyle="round")
            ax.plot([r["gap%:envelope"]], [y], marker="<", mfc=SURF, mec=INK, ms=5.5, mew=1, ls="none", zorder=4)
        elif key == "pred":
            PCLIP = 22
            for k_, mfc in (("probe%:ls_global", SURF), ("probe%:ls_tile", INK2)):
                v = r[k_]
                ax.plot([min(v, PCLIP)], [y], marker="o" if v <= PCLIP else ">", mfc=mfc, mec=INK2, ms=5, mew=1, ls="none")
                if v > PCLIP:
                    ax.text(PCLIP - 1.2, y + (-0.32 if mfc == SURF else 0.42), f"{v:+.0f}", fontsize=6.5, color=INK2, ha="right", va="center")
            ax.plot([r["probe%:pred_select"]], [y], marker="|", color=INK, ms=9, mew=1.5, ls="none")
        elif key == "ctx":
            ax.plot([r["probe%:P_net_mm"]], [y], marker="o", mfc=SURF, mec=INK2, ms=5, mew=1, ls="none", zorder=3)
            ax.plot([r["probe%:parity_net_mm"]], [y], marker="s", mfc=BAND2, mec=BAND2, ms=4.5, ls="none", zorder=3)
            ax.plot([r["probe%:phase8_net_mm"]], [y], marker="*", mfc=INK, mec=INK, ms=7.5, ls="none", zorder=4)
        elif key == "learn":
            ax.barh(y, r["learning_cost%:L3"], height=0.6, color=BAND, lw=0)
        elif key == "rep":
            ax.barh(y, r["repeat_hit_share_of_med_miss"] * 100, height=0.6, color=BAND2, lw=0)
        elif key == "src" and "gap%:src_jpeg_full" in r:
            ax.plot([r["gap%:src_jpeg_crop_adj"], r["gap%:src_jpeg_full"]], [y, y], color=INK2, lw=0.8)
            ax.plot([r["gap%:src_jpeg_crop_adj"]], [y], marker="*", mfc=SURF, mec=INK, ms=6, mew=0.8, ls="none")
            ax.plot([r["gap%:src_jpeg_full"]], [y], marker="*", mfc=INK, mec=INK, ms=8, ls="none")
        elif key == "src":
            ax.text(-2, y, "PNG source", fontsize=7, color=MUTED, va="center", ha="right")
    ax.axvline(0, color=AXIS, lw=0.8, zorder=1)
    hgrid(ax)
    ax.set_title(title, fontsize=9, color=INK, loc="left")
    ax.set_yticks([])
    ax.spines["left"].set_visible(False)
axs[0].set_ylim(ymax, 0.6)
for n in names:
    axs[0].text(-0.04, yp[n], n, transform=axs[0].get_yaxis_transform(), ha="right", va="center", fontsize=8, color=INK2)
for g, y in heads:
    axs[0].text(-0.04, y, g.upper(), transform=axs[0].get_yaxis_transform(), ha="right", va="center", fontsize=6.5, color=MUTED, fontweight="bold")
xl = {"meas": "% vs PKON2 (clipped ±48)", "model": "% vs PKON2", "pred": "Δ bits, % of PKON2", "ctx": "Δ bits (static), % of PKON2",
      "learn": "% of PKON2", "rep": "% of MED-miss pixels", "src": "% vs PKON2"}
for ax, (_, key) in zip(axs, cols):
    ax.set_xlabel(xl[key], fontsize=8)
axs[0].set_xlim(-50, 50)
axs[2].set_xlim(-10, 23.5)
axs[0].legend(handles=[Line2D([], [], ls="none", marker=STY[c]["marker"], mfc=STY[c]["mfc"], mec=STY[c]["color"], ms=5.5, label=SHORT[c]) for c in CODECS[1:4]],
              loc="lower left", fontsize=7.5, frameon=True, facecolor=SURF, edgecolor=GRID, bbox_to_anchor=(0, -0.13))
axs[1].legend(handles=[Line2D([], [], color="#d4d2ca", lw=4.5, label="L2: spatial + prev-plane"), Line2D([], [], color=BAND2, lw=4.5, label="L3: + P (null-calibrated)"),
                       Line2D([], [], ls="none", marker="<", mfc=SURF, mec=INK, ms=5.5, label="optimistic envelope (hypothesis)")],
              loc="lower left", fontsize=7.5, frameon=True, facecolor=SURF, edgecolor=GRID, bbox_to_anchor=(0, -0.13), title="static … adaptive", title_fontsize=7)
axs[2].legend(handles=[Line2D([], [], ls="none", marker="o", mfc=SURF, mec=INK2, label="global LS"), Line2D([], [], ls="none", marker="o", mfc=INK2, mec=INK2, label="per-tile LS (+coef. cost)"),
                       Line2D([], [], ls="none", marker="|", color=INK, ms=9, mew=1.5, label="best of MED/LS per plane")],
              loc="lower left", fontsize=7.5, frameon=True, facecolor=SURF, edgecolor=GRID, bbox_to_anchor=(0, -0.15))
axs[3].legend(handles=[Line2D([], [], ls="none", marker="o", mfc=SURF, mec=INK2, ms=5, label="P (49 classes; PKON2 feature)"),
                       Line2D([], [], ls="none", marker="s", mfc=BAND2, mec=BAND2, ms=4.5, label="pixel parity (4)"),
                       Line2D([], [], ls="none", marker="*", mfc=INK, mec=INK, ms=7.5, label="8×8 source-grid phase (64)")],
              loc="lower left", fontsize=7.5, frameon=True, facecolor=SURF, edgecolor=GRID, bbox_to_anchor=(0, -0.15))
axs[6].legend(handles=[Line2D([], [], ls="none", marker="*", mfc=INK, mec=INK, ms=8, label="whole-photo average"), Line2D([], [], ls="none", marker="*", mfc=SURF, mec=INK, ms=6, label="crop-adjusted (rough)")],
              loc="lower left", fontsize=7.5, frameon=True, facecolor=SURF, edgecolor=GRID, bbox_to_anchor=(0, -0.12))
fig.suptitle("Headroom indicators per image — separate lenses on the gap, NOT additive components (negative = fewer bits than PKON2 / than the L2 baseline)",
             x=0.08, ha="left", y=0.94, fontsize=12, color=INK, fontweight="bold")
save(fig, "fig4_headroom_indicators")
print("figures written")
