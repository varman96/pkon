"""ENTMAP report tables: prints the summary facts and the per-image / per-group markdown tables used in report.md, from data/map_data.csv.
Usage: python -I tools/entmap_tables.py <entropy-map results dir>/data > tables.md"""
import csv, sys, statistics as st
sys.stdout.reconfigure(encoding="utf-8")
d = sys.argv[1]
rows = list(csv.DictReader(open(d + "/map_data.csv", encoding="utf-8")))
f = lambda r, k: float(r[k]) if r.get(k) not in (None, "") else float("nan")
mean = lambda k, rs=rows: st.mean(f(r, k) for r in rs)
print("cross-channel means:", {k: round(mean(k), 3) for k in ("I_RG", "I_BG", "I_RB", "TC_rgb_plugin", "H_joint_rgb_plugin", "I_resid_RG", "I_resid_BG",
                                                              "unique_colors", "rgb_med:L2:adaptive", "subg_med:L1:adaptive", "subg_med:L2:adaptive")})
print("unique colours range", min(f(r, "unique_colors") for r in rows), max(f(r, "unique_colors") for r in rows))
print("PKON2 below L3 static-cal on", sum(f(r, "gap%:L3_static") > 0 for r in rows), "images; below L3 adaptive on", sum(f(r, "gap%:L3_adaptive") > 0 for r in rows))
print("lowest adaptive below PKON2 on", [r["image"] for r in rows if f(r, "gap%:best_adaptive") < 0])
print("lowest static below PKON2 on", [r["image"] for r in rows if f(r, "gap%:lowest_static") < 0])
print("lattice aligned range", min(f(r, "lattice_aligned") for r in rows if r.get("lattice_aligned") not in ("", "nan")), max(f(r, "lattice_aligned") for r in rows if r.get("lattice_aligned") not in ("", "nan")))
print("lattice offset range", min(f(r, "lattice_offset4") for r in rows if r.get("lattice_offset4") not in ("", "nan")), max(f(r, "lattice_offset4") for r in rows if r.get("lattice_offset4") not in ("", "nan")))
print("envelope % range", min(f(r, "gap%:envelope") for r in rows), max(f(r, "gap%:envelope") for r in rows))
print()
print("| image | group | PKON2 bpp | best other (Δ%) | ΣH0 ch. | H0 resid. | L3 static…adaptive (Δ% vs PKON2) | lowest adaptive (Δ%) | lowest cal. static (Δ%) | pred. choice | phase | parity | P | learn. cost | repeat hits | src JPEG bpp (Δ%) | JPEG q / sub | lattice |")
print("|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|")
SH = {"jxl-e9-t1": "JXL e9", "jxl-e7-t1": "JXL e7", "webp-m6-t1": "WebP", "png-rs": "PNG"}
for r in rows:
    src = f"{f(r, 'src_jpeg_bpp_full'):.2f} ({f(r, 'gap%:src_jpeg_full'):+.0f}%)" if r.get("src_jpeg_bpp_full") else "PNG source"
    q = f"{r['jpeg_ijg_quality_est']} / {r['jpeg_subsampling']}" if r.get("jpeg_ijg_quality_est") else "–"
    la = r.get("lattice_aligned", "")
    la = f"{float(la):.2f} / {f(r, 'lattice_offset4'):.2f}" if la not in ("", "nan") else "n/a"
    print(f"| {r['image']} | {r['group']} | {f(r, 'bpp:pkon2-file'):.3f} | {SH[r['best_other_codec']]} {f(r, 'gap%:best_other_codec'):+.1f} | {f(r, 'H0_channels'):.2f} | "
          f"{f(r, 'H0_med_subg'):.2f} | {f(r, 'static_cal:P'):.3f}…{f(r, 'subg_med:L3_P:adaptive'):.3f} ({f(r, 'gap%:L3_static'):+.1f}…{f(r, 'gap%:L3_adaptive'):+.1f}) | "
          f"{f(r, 'best_adaptive_bpp'):.3f} ({f(r, 'gap%:best_adaptive'):+.1f}) | {f(r, 'lowest_static_bpp'):.3f} ({f(r, 'gap%:lowest_static'):+.1f}) | "
          f"{f(r, 'signal:pred_select_mm') / f(r, 'bpp:pkon2-file') * 100:+.1f} | {f(r, 'probe%:phase8_net_mm'):+.1f} | {f(r, 'probe%:parity_net_mm'):+.1f} | {f(r, 'probe%:P_net_mm'):+.1f} | "
          f"{f(r, 'learning_cost%:L3'):.1f} | {f(r, 'repeat_hit_share_of_med_miss') * 100:.1f}% | {src} | {q} | {la} |")
print()
print("aggregate pred-select static signal % of PKON2:", round(mean("signal:pred_select_mm") / mean("bpp:pkon2-file") * 100, 2))
for k in ("probe%:phase8_net_mm", "probe%:parity_net_mm", "probe%:P_net_mm", "learning_cost%:L3", "gap%:envelope"):
    print(k, round(mean(k), 2), "median", round(st.median(f(r, k) for r in rows), 2))
print()
groups = list(dict.fromkeys(r["group"] for r in rows))
print("| group | n | PKON2 bpp | JXL e9 Δ% | WebP Δ% | best measured Δ% | L3 static…adaptive Δ% | lowest cal. static Δ% | phase | parity | pred. choice | repeat hits | src JPEG bpp (n) |")
print("|---|---|---|---|---|---|---|---|---|---|---|---|---|")
for g in groups:
    rs = [r for r in rows if r["group"] == g]
    pk = sum(f(r, "bpp:pkon2-file") for r in rs)
    pct = lambda k: (sum(f(r, k) for r in rs) / pk - 1) * 100
    sig = lambda k: sum(f(r, k) for r in rs) / pk * 100
    jr = [r for r in rs if r.get("src_jpeg_bpp_full")]
    srcs = f"{st.mean(f(r, 'src_jpeg_bpp_full') for r in jr):.2f} vs {st.mean(f(r, 'bpp:pkon2-file') for r in jr):.2f} ({len(jr)})" if jr else "–"
    print(f"| {g} | {len(rs)} | {pk / len(rs):.3f} | {pct('bpp:jxl-e9-t1'):+.1f} | {pct('bpp:webp-m6-t1'):+.1f} | {pct('best_measured_bpp'):+.1f} | "
          f"{pct('static_cal:P'):+.1f}…{pct('subg_med:L3_P:adaptive'):+.1f} | {pct('lowest_static_bpp'):+.1f} | {sig('net:phase8:mm'):+.1f} | {sig('net:parity:mm'):+.1f} | "
          f"{sig('signal:pred_select_mm'):+.1f} | {st.mean(f(r, 'repeat_hit_share_of_med_miss') for r in rs) * 100:.1f}% | {srcs} |")
