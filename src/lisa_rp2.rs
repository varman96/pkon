//! EXPERIMENT (isolated, cost-only): second-difference-driven adaptation rate for the reward-penalty estimator.
//! Lockstep replay of (production Bit, fixed-rate RP alpha=1/32, second-difference RP) over the Stage 3 output, with the
//! same binarization/model selection as `lisa_rp.rs`. Nothing here is used by production LISA; no coding happens.
use crate::lisa::*;
use crate::pkon2x::Bit;

const E_MAX: usize = 33; // = S4_E_MAX in lisa.rs
const EPS: f64 = 1.0 / 1024.0;
pub const ALPHA_FIXED: f64 = 1.0 / 32.0;
pub const ALPHA_MIN: f64 = 1.0 / 255.0;
pub const ALPHA_MAX: f64 = 1.0 / 4.0;

/// Required mapping from the signed second difference to the adaptation rate.
pub fn alpha_from_second_diff(a: f64) -> f64 {
    let m = a.abs();
    ALPHA_MIN + (ALPHA_MAX - ALPHA_MIN) * (m / (m + 1.0))
}

/// Fixed-rate reward-penalty state (the existing candidate: alpha = 1/32).
#[derive(Clone, Copy)]
struct RpFixed {
    r: f64,
    p: f64,
}
impl RpFixed {
    fn new() -> Self {
        RpFixed { r: 0.5, p: 0.5 }
    }
    fn p1(&self) -> f64 {
        (self.r + EPS) / (self.r + self.p + 2.0 * EPS)
    }
    fn update(&mut self, b: u32) {
        let b = b as f64;
        self.r = (1.0 - ALPHA_FIXED) * self.r + ALPHA_FIXED * b;
        self.p = (1.0 - ALPHA_FIXED) * self.p + ALPHA_FIXED * (1.0 - b);
    }
}

/// Reward-penalty state with second-difference-driven rate. History is per state (= per independent decision stream).
#[derive(Clone, Copy)]
struct RpSecondDiff {
    r: f64,
    p: f64,
    n: u64,
    e_prev: f64,
    d_prev: f64,
}
impl RpSecondDiff {
    fn new() -> Self {
        RpSecondDiff { r: 0.5, p: 0.5, n: 0, e_prev: 0.0, d_prev: 0.0 }
    }
    fn p1(&self) -> f64 {
        (self.r + EPS) / (self.r + self.p + 2.0 * EPS)
    }
    /// Observe `b`: surprise from the pre-update p, then d, a, alpha, then a single R/P update.
    /// Returns Some((alpha, |a|)) when the dynamic rate was used, None for the two fixed-rate warm-up observations.
    fn step(&mut self, b: u32) -> Option<(f64, f64)> {
        let p = self.p1();
        let e = -(if b == 1 { p } else { 1.0 - p }).log2();
        let mut used = None;
        let mut alpha = ALPHA_FIXED;
        if self.n >= 1 {
            let d = e - self.e_prev;
            if self.n >= 2 {
                let a = d - self.d_prev;
                alpha = alpha_from_second_diff(a);
                used = Some((alpha, a.abs()));
            }
            self.d_prev = d;
        }
        self.e_prev = e;
        self.n += 1;
        let bf = b as f64;
        self.r = (1.0 - alpha) * self.r + alpha * bf;
        self.p = (1.0 - alpha) * self.p + alpha * (1.0 - bf);
        used
    }
}

#[derive(Clone, Copy)]
struct Models {
    pb: [[Bit; E_MAX + 1]; 2],
    pf: [[RpFixed; E_MAX + 1]; 2],
    pd: [[RpSecondDiff; E_MAX + 1]; 2],
}
impl Models {
    fn new() -> Self {
        Models { pb: [[Bit::NEW; E_MAX + 1]; 2], pf: [[RpFixed::new(); E_MAX + 1]; 2], pd: [[RpSecondDiff::new(); E_MAX + 1]; 2] }
    }
}

// ---------------------------------------------------------------- diagnostics

pub const SEGS: usize = 16;
/// Probability bins on the baseline (production) probability of the observed outcome.
pub const PBINS: [f64; 5] = [0.1, 0.3, 0.7, 0.9, 0.99];
/// Reporting-only bins on u = (alpha - alpha_min)/(alpha_max - alpha_min) = |a|/(|a|+1).
pub const U_NEAR_MIN: f64 = 0.1; // u < 0.1  (|a| < 1/9)
pub const U_NEAR_MAX: f64 = 0.9; // u > 0.9  (|a| > 9)
/// Reporting-only bins on |a|: upper edges; last bin is open.
pub const A_EDGES: [f64; 9] = [0.0, 0.01, 0.1, 0.5, 1.0, 2.0, 5.0, 10.0, f64::INFINITY];
pub const A_BINS: usize = 9;
pub const HIST: usize = 100;

/// cost accumulators in order [production, fixed-rate RP, second-difference RP] plus decision count.
#[derive(Clone, Copy, Default)]
pub struct Acc {
    pub c: [f64; 3],
    pub n: f64,
}
impl Acc {
    fn add(&mut self, c: [f64; 3]) {
        for i in 0..3 {
            self.c[i] += c[i];
        }
        self.n += 1.0;
    }
}

#[derive(Clone, Copy)]
pub struct RateStats {
    pub n: u64,
    pub sum: f64,
    pub min: f64,
    pub max: f64,
    pub hist: [u64; HIST], // over alpha in [alpha_min, alpha_max]
    pub near_min: u64,
    pub mid: u64,
    pub near_max: u64,
}
impl Default for RateStats {
    fn default() -> Self {
        RateStats { n: 0, sum: 0.0, min: 0.0, max: 0.0, hist: [0; HIST], near_min: 0, mid: 0, near_max: 0 }
    }
}
impl RateStats {
    fn add(&mut self, alpha: f64) {
        if self.n == 0 {
            self.min = alpha;
            self.max = alpha;
        }
        self.n += 1;
        self.sum += alpha;
        self.min = self.min.min(alpha);
        self.max = self.max.max(alpha);
        let u = (alpha - ALPHA_MIN) / (ALPHA_MAX - ALPHA_MIN);
        self.hist[((u * HIST as f64) as usize).min(HIST - 1)] += 1;
        if u < U_NEAR_MIN {
            self.near_min += 1;
        } else if u > U_NEAR_MAX {
            self.near_max += 1;
        } else {
            self.mid += 1;
        }
    }
    pub fn quantile(&self, q: f64) -> f64 {
        let target = (q * self.n as f64).ceil() as u64;
        let mut cum = 0;
        for (i, &h) in self.hist.iter().enumerate() {
            cum += h;
            if cum >= target {
                let u = (i as f64 + 0.5) / HIST as f64;
                return ALPHA_MIN + (ALPHA_MAX - ALPHA_MIN) * u;
            }
        }
        self.max
    }
}

/// Integrity trackers, one per estimator: they see the symbol stream through each estimator's own update call.
#[derive(Clone, Copy, Default)]
pub struct Trace {
    pub n: u64,
    pub ones: u64,
    pub hash: u64,
}
impl Trace {
    fn see(&mut self, slot_id: usize, b: u32) {
        self.n += 1;
        self.ones += b as u64;
        self.hash = (self.hash ^ ((slot_id as u64) << 1 | b as u64)).wrapping_mul(0x100000001b3).rotate_left(5);
    }
}

pub struct Diag {
    pub n_dec: u64,
    pub fixed_bits: f64,
    pub seg: [Acc; SEGS],
    pub plane: [Acc; 4],
    pub class: [Acc; 4],
    pub kind: [Acc; 2], // 0 unary, 1 mantissa
    pub pbin: [Acc; 6],
    pub rate: RateStats,
    pub rate_class: [RateStats; 4],
    pub warmup: Acc, // decisions where second-diff used the fixed warm-up rate
    pub dynamic: Acc, // decisions where the dynamic rate was used
    pub abin: [(Acc, f64); A_BINS], // by |a|: costs and sum of alpha
    /// [class][kind][slot]
    pub slot: Vec<Acc>,
    pub trace: [Trace; 3],
    pub resets: [u64; 3],
}

pub fn slot_index(c: usize, kind: usize, s: usize) -> usize {
    (c * 2 + kind) * (E_MAX + 1) + s
}

pub fn diagnose(s3: &Stage3Output, total_residuals: usize) -> Diag {
    let mut d = Diag {
        n_dec: 0,
        fixed_bits: 0.0,
        seg: [Acc::default(); SEGS],
        plane: [Acc::default(); 4],
        class: [Acc::default(); 4],
        kind: [Acc::default(); 2],
        pbin: [Acc::default(); 6],
        rate: RateStats::default(),
        rate_class: [RateStats::default(); 4],
        warmup: Acc::default(),
        dynamic: Acc::default(),
        abin: [(Acc::default(), 0.0); A_BINS],
        slot: vec![Acc::default(); 4 * 2 * (E_MAX + 1)],
        trace: [Trace::default(); 3],
        resets: [1; 3], // all three estimators are created together exactly once per image
    };
    let mut ms = [Models::new(); 4]; // indexed by Stage 3 class, same as lisa_rp.rs
    let mut done = 0usize;
    let mut planes: Vec<(usize, &[u32], &[ActivityClass])> = Vec::new();
    match (s3.data(), s3.contexts()) {
        (OrganizedResiduals::BayerPlanes(r), OrganizedContexts::BayerPlanes(c)) => {
            for (i, (r, c)) in r.iter().zip(c).enumerate() {
                planes.push((i, r, c));
            }
        }
        (OrganizedResiduals::Morton(r), OrganizedContexts::Morton(c)) => planes.push((0, r, c)),
        _ => panic!("contexts do not match the organized data"),
    }
    for (pl, res, ctx) in planes {
        for (&v, c) in res.iter().zip(ctx) {
            let ci = c.index();
            let seg = (done * SEGS / total_residuals).min(SEGS - 1);
            done += 1;
            let x = u64::from(v) + 1;
            let e_in = (64 - x.leading_zeros()) as usize;
            let mut e = E_MAX;
            let mut dec: Vec<(usize, usize, u32)> = Vec::with_capacity(36); // kind, slot, bit
            for i in 1..E_MAX {
                let b = (e_in > i) as u32;
                dec.push((0, i, b));
                if b == 0 {
                    e = i;
                    break;
                }
            }
            for j in (0..e - 1).rev() {
                if j == e - 2 {
                    dec.push((1, e, ((x >> j) & 1) as u32));
                } else {
                    d.fixed_bits += 1.0;
                }
            }
            for (kind, s, b) in dec {
                let m = &mut ms[ci];
                let (pb, pf, pd) = (m.pb[kind][s].p1() as f64 / 65536.0, m.pf[kind][s].p1(), m.pd[kind][s].p1());
                let cost = |p: f64| -(if b == 1 { p } else { 1.0 - p }).log2();
                let costs = [cost(pb), cost(pf), cost(pd)];
                let qb = if b == 1 { pb } else { 1.0 - pb };
                m.pb[kind][s].adapt(b);
                m.pf[kind][s].update(b);
                let used = m.pd[kind][s].step(b);
                let sid = slot_index(ci, kind, s);
                for t in &mut d.trace {
                    t.see(sid, b);
                }
                d.n_dec += 1;
                for a in [&mut d.seg[seg], &mut d.plane[pl], &mut d.class[ci], &mut d.kind[kind], &mut d.slot[sid]] {
                    a.add(costs);
                }
                d.pbin[PBINS.iter().position(|&t| qb < t).unwrap_or(5)].add(costs);
                match used {
                    Some((alpha, am)) => {
                        d.rate.add(alpha);
                        d.rate_class[ci].add(alpha);
                        d.dynamic.add(costs);
                        let bi = A_EDGES[1..].iter().position(|&t| am < t).unwrap_or(A_BINS - 1);
                        d.abin[bi].0.add(costs);
                        d.abin[bi].1 += alpha;
                    }
                    None => d.warmup.add(costs),
                }
            }
        }
    }
    d
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mapping_zero_is_min() {
        assert_eq!(alpha_from_second_diff(0.0), ALPHA_MIN);
    }
    #[test]
    fn mapping_positive() {
        // |a| = 1 -> halfway between min and max
        let a = alpha_from_second_diff(1.0);
        assert!((a - (ALPHA_MIN + ALPHA_MAX) / 2.0).abs() < 1e-15);
    }
    #[test]
    fn mapping_negative_matches_magnitude() {
        assert_eq!(alpha_from_second_diff(-1.0), alpha_from_second_diff(1.0));
        assert!(alpha_from_second_diff(-3.0) > ALPHA_MIN);
    }
    #[test]
    fn mapping_large_stays_below_max() {
        let a = alpha_from_second_diff(1e6);
        assert!(a < ALPHA_MAX && a > 0.2499);
        assert!(alpha_from_second_diff(-1e6) < ALPHA_MAX);
    }
    #[test]
    fn mapping_strictly_in_open_unit_interval() {
        for &x in &[0.0, 1e-9, 0.01, 0.5, 1.0, -1.0, 7.0, -7.0, 1e3, -1e3, 1e9, -1e9, 33.0] {
            let a = alpha_from_second_diff(x);
            assert!(a > 0.0 && a < 1.0, "alpha {a} for a={x}");
        }
    }
    #[test]
    fn update_order_uses_pre_update_probability_and_warmup() {
        let mut s = RpSecondDiff::new();
        // obs 1 and 2: fixed rate, no dynamic report
        assert!(s.step(1).is_none());
        assert!(s.step(1).is_none());
        // reproduce by hand
        let (mut r, mut p) = (0.5f64, 0.5f64);
        let mut es = vec![];
        let mut ds: Vec<f64> = vec![];
        let seq = [1u32, 1, 0, 1, 0, 0, 1];
        let mut m = RpSecondDiff::new();
        for (t, &b) in seq.iter().enumerate() {
            let pr = (r + EPS) / (r + p + 2.0 * EPS);
            let e = -(if b == 1 { pr } else { 1.0 - pr }).log2();
            let mut alpha = ALPHA_FIXED;
            if t >= 1 {
                let d = e - es[t - 1];
                if t >= 2 {
                    alpha = alpha_from_second_diff(d - ds[t - 2]);
                }
                ds.push(d);
            }
            es.push(e);
            r = (1.0 - alpha) * r + alpha * b as f64;
            p = (1.0 - alpha) * p + alpha * (1.0 - b as f64);
            m.step(b);
            assert!((m.r - r).abs() < 1e-15 && (m.p - p).abs() < 1e-15);
        }
    }
}
