"""Aggregates per_image_phase*.csv / timings_phase*.csv into aggregate.csv, per_image.csv, timings.csv and decision_inputs.txt. Frozen rules from the task spec."""
import csv, statistics as st, sys, os, glob
D = os.path.dirname(os.path.abspath(__file__))
def rd(p): return list(csv.DictReader(open(p)))
per = []; tim = []
for ph in (1, 2):
    p = f"{D}/per_image_phase{ph}.csv"
    if os.path.exists(p): per += rd(p); tim += rd(f"{D}/timings_phase{ph}.csv")
for r in per:
    r['A'] = int(r['A_bytes']); r['B'] = int(r['B_bytes']); r['pct'] = float(r['pct_change'])
    r['pct_exact'] = 100.0 * (r['A'] - r['B']) / r['A']
groups = {
 'DEV': lambda r: r['split'] == 'dev',
 'VAL': lambda r: r['split'] == 'val',
 'DEV+VAL': lambda r: r['split'] in ('dev', 'val'),
 'TEST_light8': lambda r: r['split'] == 'test_light',
 'TEST_used7': lambda r: r['split'] == 'test_used',
 'TEST_all15': lambda r: r['split'].startswith('test'),
}
# timing
T = {}
for r in tim: T.setdefault((r['image'], r['arm']), []).append((float(r['encode_ms']), float(r['decode_ms'])))
img_split = {r['image']: r['split'] for r in per}
def timing_stats(names):
    out = {}
    for k, idx in (('enc', 0), ('dec', 1)):
        ratios, sa, sb, ma, mb, var = [], 0, 0, [], [], []
        for n in names:
            a = [x[idx] for x in T[(n, 'A')]]; b = [x[idx] for x in T[(n, 'B')]]
            A, B = st.median(a), st.median(b)
            ratios.append(B / A); sa += A; sb += B; ma.append(A); mb.append(B)
            var += [(max(a) - min(a)) / A, (max(b) - min(b)) / B]
        out[k] = dict(median_of_per_image_ratio=st.median(ratios), aggregate_ratio=sb / sa, ratio_of_median_times=st.median(mb) / st.median(ma),
                      min_ratio=min(ratios), max_ratio=max(ratios), median_rep_spread=st.median(var), max_rep_spread=max(var), medA=st.median(ma), medB=st.median(mb))
    return out
rows = []; dec = []
agg = {}
for g, f in groups.items():
    rs = [r for r in per if f(r)]
    if not rs: continue
    A = sum(r['A'] for r in rs); B = sum(r['B'] for r in rs)
    pcts = [r['pct_exact'] for r in rs]
    a = dict(n=len(rs), A=A, B=B, diff=B - A, agg_pct=100.0 * (A - B) / A, smaller=sum(r['B'] < r['A'] for r in rs), equal=sum(r['B'] == r['A'] for r in rs),
             larger=sum(r['B'] > r['A'] for r in rs), min_pct=min(pcts), median_pct=st.median(pcts), max_growth_pct=max(0.0, -min(pcts)),
             exact=all(r['exact_A'] == 'true' and r['exact_B'] == 'true' for r in rs))
    ts = timing_stats([r['image'] for r in rs]) if all((r['image'], 'A') in T for r in rs) else None
    a['timing'] = ts; agg[g] = a
    rows.append([g, a['n'], A, B, a['diff'], f"{a['agg_pct']:.4f}", a['smaller'], a['equal'], a['larger'], f"{a['min_pct']:.4f}", f"{a['median_pct']:.4f}", f"{a['max_growth_pct']:.4f}", a['exact']] +
                ([f"{ts['enc']['median_of_per_image_ratio']:.4f}", f"{ts['enc']['aggregate_ratio']:.4f}", f"{ts['dec']['median_of_per_image_ratio']:.4f}", f"{ts['dec']['aggregate_ratio']:.4f}"] if ts else ['', '', '', '']))
with open(f"{D}/aggregate.csv", 'w', newline='') as fh:
    w = csv.writer(fh)
    w.writerow(['group', 'images', 'A_bytes', 'B_bytes', 'B_minus_A', 'aggregate_reduction_pct', 'images_smaller', 'images_equal', 'images_larger', 'min_image_pct', 'median_image_pct', 'max_image_growth_pct', 'all_exact',
                'enc_ratio_median_of_per_image', 'enc_ratio_aggregate', 'dec_ratio_median_of_per_image', 'dec_ratio_aggregate'])
    w.writerows(rows)
# combined per_image.csv / timings.csv with the Time medians
with open(f"{D}/per_image.csv", 'w', newline='') as fh:
    w = csv.writer(fh)
    w.writerow(['image', 'split', 'A_bytes', 'B_bytes', 'B_minus_A', 'pct_reduction', 'exact_A', 'exact_B', 'A_enc_ms_median', 'B_enc_ms_median', 'A_dec_ms_median', 'B_dec_ms_median'])
    for r in per:
        m = lambda arm, i: f"{st.median([x[i] for x in T[(r['image'], arm)]]):.2f}" if (r['image'], arm) in T else ''
        w.writerow([r['image'], r['split'], r['A'], r['B'], r['B'] - r['A'], f"{r['pct_exact']:.4f}", r['exact_A'], r['exact_B'], m('A', 0), m('B', 0), m('A', 1), m('B', 1)])
with open(f"{D}/timings.csv", 'w', newline='') as fh:
    w = csv.writer(fh); w.writerow(['image', 'split', 'arm', 'rep', 'order_in_rep', 'encode_ms', 'decode_ms'])
    for r in tim: w.writerow([r['image'], r['split'], r['arm'], r['rep'], r['order_in_rep'], r['encode_ms'], r['decode_ms']])
# decision (frozen rules)
L = []
def g(n): return agg.get(n)
dv, dev, val = g('DEV+VAL'), g('DEV'), g('VAL')
if dv:
    tsd = dv['timing']
    enc_r, dec_r = tsd['enc']['median_of_per_image_ratio'], tsd['dec']['median_of_per_image_ratio']
    c = {
     '1 exact+format gates (DEV/VAL)': dv['exact'],
     '2 DEV/VAL union >= 0.25%': dv['agg_pct'] >= 0.25, '3 DEV >= 0.25%': dev['agg_pct'] >= 0.25, '4 VAL >= 0.25%': val['agg_pct'] >= 0.25,
     '5 >=80% DEV/VAL images smaller': dv['smaller'] / dv['n'] >= 0.8, '6 no DEV/VAL image >0.10% larger': dv['max_growth_pct'] <= 0.10,
    }
    for k, v in c.items(): L.append(f"{k}: {v}")
    L.append(f"DEV/VAL timing (median of per-image median-of-3 ratios) enc {enc_r:.4f} dec {dec_r:.4f}; aggregate-sum enc {tsd['enc']['aggregate_ratio']:.4f} dec {tsd['dec']['aggregate_ratio']:.4f}; ratio-of-median-times enc {tsd['enc']['ratio_of_median_times']:.4f} dec {tsd['dec']['ratio_of_median_times']:.4f}")
    L.append(f"10 timing <=1.15: {enc_r <= 1.15 and dec_r <= 1.15}   reject timing >1.25: {enc_r > 1.25 or dec_r > 1.25}")
    rej = []
    if not dv['exact']: rej.append('correctness')
    if dv['agg_pct'] < 0.10: rej.append('DEV/VAL < 0.10%')
    if dev['agg_pct'] < 0 or val['agg_pct'] < 0: rej.append('DEV or VAL aggregate growth')
    if dv['larger'] / dv['n'] > 0.2: rej.append('>20% DEV/VAL images grow')
    if dv['max_growth_pct'] > 0.5: rej.append('image grows >0.5%')
    if enc_r > 1.25 or dec_r > 1.25: rej.append('timing >1.25x')
    L.append("DEV/VAL reject conditions triggered: " + (', '.join(rej) if rej else 'none'))
    L.append("DEV/VAL-only promotion criteria (1-6,10) all hold: " + str(all(c.values()) and enc_r <= 1.15 and dec_r <= 1.15))
for n in ('TEST_light8', 'TEST_used7', 'TEST_all15'):
    a = g(n)
    if a:
        L.append(f"{n}: n={a['n']} A={a['A']} B={a['B']} agg={a['agg_pct']:.4f}% smaller={a['smaller']}/{a['n']} larger={a['larger']} max_growth={a['max_growth_pct']:.4f}% exact={a['exact']}")
a = g('TEST_all15')
if a:
    L.append(f"TEST_all15 rule checks: 7 agg>=0.25: {a['agg_pct']>=0.25}; 8 >=80% smaller: {a['smaller']/a['n']>=0.8}; 9 no image >0.10% larger: {a['max_growth_pct']<=0.10}; reject: agg<0.10 {a['agg_pct']<0.10}, >20% grow {a['larger']/a['n']>0.2}, any >0.5% {a['max_growth_pct']>0.5}")
for n in agg:
    t = agg[n]['timing']
    if t:
        L.append(f"timing[{n}] enc: med A {t['enc']['medA']:.1f} ms B {t['enc']['medB']:.1f} ms, per-image ratio min/med/max {t['enc']['min_ratio']:.3f}/{t['enc']['median_of_per_image_ratio']:.3f}/{t['enc']['max_ratio']:.3f}, agg {t['enc']['aggregate_ratio']:.3f}, rep spread median/max {t['enc']['median_rep_spread']:.3f}/{t['enc']['max_rep_spread']:.3f}; "
                 f"dec: med A {t['dec']['medA']:.1f} B {t['dec']['medB']:.1f}, ratio min/med/max {t['dec']['min_ratio']:.3f}/{t['dec']['median_of_per_image_ratio']:.3f}/{t['dec']['max_ratio']:.3f}, agg {t['dec']['aggregate_ratio']:.3f}, spread {t['dec']['median_rep_spread']:.3f}/{t['dec']['max_rep_spread']:.3f}")
open(f"{D}/decision_inputs.txt", 'w').write('\n'.join(L) + '\n')
print('\n'.join(L))
