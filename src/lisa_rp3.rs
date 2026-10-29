//! EXPERIMENT (isolated, cost-only): Kalman-gain adaptation rate for the reward-penalty estimator.
//! Lockstep replay of (production Bit, fixed-rate RP alpha=1/32, Kalman-gain RP) over the Stage 3 output, with the
//! same binarization/model selection as `lisa_rp.rs`. Nothing here is used by production LISA; no coding happens.
//!
//! Per decision stream (class, kind, slot), with p_t = RP probability before seeing b_t:
//!   a_t = p_t - 2 p_{t-1} + p_{t-2}   (a_t = 0 until two prior estimates exist, i.e. for t < 2)
//!   Q_t = a_t^2,  R_t = p_t (1 - p_t),  V^-_t = V_{t-1} + Q_t,  alpha_t = V^-_t / (V^-_t + R_t)
//!   r' = (1-alpha) r + alpha b,  p' = (1-alpha) p + alpha (1-b)   (RP components; p_{t+1} is read back through eps)
//!   V_t = (1 - alpha_t) V^-_t,  V_{-1} = 0.5
//! Only floating-point safeguard: if V^- + R == 0 exactly (0/0), alpha = 0 (no update). Never reached in practice,
//! because R_t > 0 whenever eps > 0 keeps p_t strictly inside (0,1).
use crate::lisa::*;
use crate::pkon2x::Bit;

const E_MAX: usize = 33; // = S4_E_MAX in lisa.rs
const EPS: f64 = 1.0 / 1024.0;
pub const ALPHA_FIXED: f64 = 1.0 / 32.0;
pub const V_INIT: f64 = 0.5;

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

/// What one Kalman step computed (all from state available before the bit).
#[derive(Clone, Copy)]
pub struct KStep {
    pub alpha: f64,
    pub alpha_no_q: f64, // counterfactual V/(V+R) with Q = 0, for the response diagnostic
    pub abs_a: f64,
    pub age: u64,
    pub valid: bool,
}

#[derive(Clone, Copy)]
pub struct RpKalman {
    r: f64,
    p: f64,
    v: f64,
    p_m1: f64, // p_{t-1}
    p_m2: f64, // p_{t-2}
    n: u64,
}
fn gain(vm: f64, rr: f64) -> f64 {
    let den = vm + rr;
    if den > 0.0 { vm / den } else { 0.0 }
}
impl RpKalman {
    pub fn new() -> Self {
        RpKalman { r: 0.5, p: 0.5, v: V_INIT, p_m1: 0.0, p_m2: 0.0, n: 0 }
    }
    pub fn p1(&self) -> f64 {
        (self.r + EPS) / (self.r + self.p + 2.0 * EPS)
    }
    /// (alpha, V^-, alpha with Q=0, a) for the current bit; depends only on state before the bit.
    fn plan(&self) -> (f64, f64, f64, f64) {
        let pt = self.p1();
        let a = if self.n >= 2 { pt - 2.0 * self.p_m1 + self.p_m2 } else { 0.0 };
        let rr = pt * (1.0 - pt);
        let vm = self.v + a * a;
        (gain(vm, rr), vm, gain(self.v, rr), a)
    }
    /// Observe `b`: gain from prior state, then a single R/P update, then V and probability history.
    pub fn step(&mut self, b: u32) -> KStep {
        let pt = self.p1();
        let (alpha, vm, alpha_no_q, a) = self.plan();
        let bf = b as f64;
        self.r = (1.0 - alpha) * self.r + alpha * bf;
        self.p = (1.0 - alpha) * self.p + alpha * (1.0 - bf);
        self.v = (1.0 - alpha) * vm;
        self.p_m2 = self.p_m1;
        self.p_m1 = pt;
        let age = self.n;
        self.n += 1;
        let pn = self.p1();
        let valid = [alpha, vm, self.v, self.r, self.p, pn].iter().all(|x| x.is_finite())
            && (0.0..=1.0).contains(&alpha)
            && self.v >= 0.0
            && pn > 0.0
            && pn < 1.0;
        KStep { alpha, alpha_no_q, abs_a: a.abs(), age, valid }
    }
}

#[derive(Clone, Copy)]
struct Models {
    pb: [[Bit; E_MAX + 1]; 2],
    pf: [[RpFixed; E_MAX + 1]; 2],
    pk: [[RpKalman; E_MAX + 1]; 2],
}
impl Models {
    fn new() -> Self {
        Models { pb: [[Bit::NEW; E_MAX + 1]; 2], pf: [[RpFixed::new(); E_MAX + 1]; 2], pk: [[RpKalman::new(); E_MAX + 1]; 2] }
    }
}

pub const SEGS: usize = 16;
pub const OCT: usize = 42; // octave histogram: bin k = alpha in [2^-(k+1), 2^-k), last bin catches the rest; zeros separate
pub const A_EDGES: [f64; 9] = [0.0, 1e-4, 1e-3, 1e-2, 0.03, 0.1, 0.3, 1.0, f64::INFINITY];
pub const AGE_EDGES: [u64; 7] = [10, 100, 1_000, 10_000, 100_000, 1_000_000, u64::MAX];

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

pub struct Diag {
    pub n_dec: u64,
    pub total: Acc,
    pub seg: [Acc; SEGS],
    pub n_rate: u64,
    pub sum: f64,
    pub sum_no_q: f64,
    pub min: f64,
    pub max: f64,
    pub zero: u64,
    pub oct: [u64; OCT],
    pub a_bin: [(u64, f64, f64); 9],   // by |a|: count, sum alpha, sum alpha_no_q
    pub age_bin: [(u64, f64, f64); 7], // by stream age (age >= 2 only)
    pub invalid: u64,
    pub fixed_bits: f64, // raw p=1/2 mantissa bits, identical for all three estimators
    pub trace: [(u64, u64, u64); 3], // n, ones, hash per estimator
}

pub fn diagnose(s3: &Stage3Output, total_residuals: usize) -> Diag {
    let mut d = Diag {
        n_dec: 0,
        total: Acc::default(),
        seg: [Acc::default(); SEGS],
        n_rate: 0,
        sum: 0.0,
        sum_no_q: 0.0,
        min: f64::INFINITY,
        max: f64::NEG_INFINITY,
        zero: 0,
        oct: [0; OCT],
        a_bin: [(0, 0.0, 0.0); 9],
        age_bin: [(0, 0.0, 0.0); 7],
        invalid: 0,
        fixed_bits: 0.0,
        trace: [(0, 0, 0); 3],
    };
    let mut ms = [Models::new(); 4]; // reset exactly once per image, indexed by Stage 3 class
    let mut done = 0usize;
    let mut planes: Vec<(&[u32], &[ActivityClass])> = Vec::new();
    match (s3.data(), s3.contexts()) {
        (OrganizedResiduals::BayerPlanes(r), OrganizedContexts::BayerPlanes(c)) => {
            for (r, c) in r.iter().zip(c) {
                planes.push((r, c));
            }
        }
        (OrganizedResiduals::Morton(r), OrganizedContexts::Morton(c)) => planes.push((r, c)),
        _ => panic!("contexts do not match the organized data"),
    }
    for (res, ctx) in planes {
        for (&v, c) in res.iter().zip(ctx) {
            let ci = c.index();
            let seg = (done * SEGS / total_residuals).min(SEGS - 1);
            done += 1;
            let x = u64::from(v) + 1;
            let e_in = (64 - x.leading_zeros()) as usize;
            let mut e = E_MAX;
            let mut dec: Vec<(usize, usize, u32)> = Vec::with_capacity(36);
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
                let (pb, pf, pk) = (m.pb[kind][s].p1() as f64 / 65536.0, m.pf[kind][s].p1(), m.pk[kind][s].p1());
                let cost = |p: f64| -(if b == 1 { p } else { 1.0 - p }).log2();
                let costs = [cost(pb), cost(pf), cost(pk)];
                m.pb[kind][s].adapt(b);
                m.pf[kind][s].update(b);
                let k = m.pk[kind][s].step(b);
                let sid = ((ci * 2 + kind) * (E_MAX + 1) + s) as u64;
                for t in &mut d.trace {
                    t.0 += 1;
                    t.1 += b as u64;
                    t.2 = (t.2 ^ (sid << 1 | b as u64)).wrapping_mul(0x100000001b3).rotate_left(5);
                }
                d.n_dec += 1;
                d.total.add(costs);
                d.seg[seg].add(costs);
                if !k.valid {
                    d.invalid += 1;
                }
                d.n_rate += 1;
                d.sum += k.alpha;
                d.sum_no_q += k.alpha_no_q;
                d.min = d.min.min(k.alpha);
                d.max = d.max.max(k.alpha);
                if k.alpha == 0.0 {
                    d.zero += 1;
                } else {
                    d.oct[((-k.alpha.log2()).floor().max(0.0) as usize).min(OCT - 1)] += 1;
                }
                let bi = A_EDGES[1..].iter().position(|&t| k.abs_a < t).unwrap_or(8);
                let ab = &mut d.a_bin[bi];
                ab.0 += 1;
                ab.1 += k.alpha;
                ab.2 += k.alpha_no_q;
                if k.age >= 2 {
                    let gi = AGE_EDGES.iter().position(|&t| k.age < t).unwrap_or(6);
                    let g = &mut d.age_bin[gi];
                    g.0 += 1;
                    g.1 += k.alpha;
                    g.2 += k.alpha_no_q;
                }
            }
        }
    }
    d
}

#[cfg(test)]
mod tests {
    use super::*;

    const SEQ: [u32; 12] = [1, 1, 0, 1, 0, 0, 1, 1, 1, 0, 0, 1];

    #[test]
    fn update_order_matches_hand_derivation() {
        let (mut r, mut p, mut v) = (0.5f64, 0.5f64, V_INIT);
        let mut hist: Vec<f64> = vec![];
        let mut m = RpKalman::new();
        for (t, &b) in SEQ.iter().enumerate() {
            let pt = (r + EPS) / (r + p + 2.0 * EPS);
            assert_eq!(m.p1(), pt, "probability for bit {t} must be the pre-observation one");
            let a = if t >= 2 { pt - 2.0 * hist[t - 1] + hist[t - 2] } else { 0.0 };
            let vm = v + a * a;
            let alpha = vm / (vm + pt * (1.0 - pt));
            r = (1.0 - alpha) * r + alpha * b as f64;
            p = (1.0 - alpha) * p + alpha * (1.0 - b as f64);
            v = (1.0 - alpha) * vm;
            hist.push(pt);
            let k = m.step(b);
            assert_eq!(k.alpha, alpha);
            assert_eq!((m.r, m.p, m.v), (r, p, v));
        }
        // first gain is fixed by the specified initialization: 0.5 / (0.5 + 0.25) = 2/3
        let k = RpKalman::new().step(0);
        assert!((k.alpha - 0.5 / (0.5 + 0.25)).abs() < 1e-12);
    }

    #[test]
    fn alpha_is_causal_independent_of_current_bit() {
        let mut m = RpKalman::new();
        for &b in &SEQ {
            let a0 = m.plan().0;
            assert_eq!(m.step_clone(0), a0);
            assert_eq!(m.step_clone(1), a0);
            m.step(b);
        }
    }
    impl RpKalman {
        fn step_clone(&self, b: u32) -> f64 {
            let mut c = *self;
            c.step(b).alpha
        }
    }

    #[test]
    fn state_finite_and_valid_on_adversarial_streams() {
        let streams: Vec<Vec<u32>> = vec![
            vec![0; 200_000],
            vec![1; 200_000],
            (0..200_000).map(|i| (i & 1) as u32).collect(),
            (0..200_000).map(|i| ((i / 5000) & 1) as u32).collect(),
            (0..200_000u64).map(|i| ((i.wrapping_mul(0x9E3779B97F4A7C15) >> 40) & 1) as u32).collect(),
        ];
        for s in streams {
            let mut m = RpKalman::new();
            for &b in &s {
                let k = m.step(b);
                assert!(k.valid && k.alpha >= 0.0 && k.alpha <= 1.0);
            }
        }
    }

    #[test]
    fn zero_denominator_is_safe() {
        assert_eq!(gain(0.0, 0.0), 0.0);
        assert_eq!(gain(1e-300, 0.0), 1.0);
    }
}
