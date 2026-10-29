//! EXPERIMENT (isolated, cost-only): continuous reward-penalty bit estimator vs the production Stage 4 counted-rate
//! estimator. Replays the exact Stage 4 binarization/model selection (see `S4Model::code` in lisa.rs) over the Stage 3
//! output and sums prequential ideal costs. Nothing here is used by production LISA; no coding or decoding happens.
use crate::lisa::*;
use crate::pkon2x::Bit;

const E_MAX: usize = 33; // = S4_E_MAX in lisa.rs
const RHO: f64 = 31.0 / 32.0;
const EPS: f64 = 1.0 / 1024.0;

/// One probability state; `p1` is evaluated before `update`.
trait Est {
    fn fresh() -> Self;
    fn p1(&self) -> f64;
    fn update(&mut self, bit: u32);
}
impl Est for Bit {
    fn fresh() -> Self {
        Bit::NEW
    }
    fn p1(&self) -> f64 {
        Bit::p1(self) as f64 / 65536.0
    }
    fn update(&mut self, bit: u32) {
        self.adapt(bit)
    }
}
#[derive(Clone, Copy)]
struct Rp {
    r: f64,
    p: f64,
}
impl Est for Rp {
    fn fresh() -> Self {
        Rp { r: 0.5, p: 0.5 }
    }
    fn p1(&self) -> f64 {
        (self.r + EPS) / (self.r + self.p + 2.0 * EPS)
    }
    fn update(&mut self, bit: u32) {
        let b = bit as f64;
        self.r = RHO * self.r + (1.0 - RHO) * b;
        self.p = RHO * self.p + (1.0 - RHO) * (1.0 - b);
    }
}

#[derive(Clone, Copy)]
struct Model<E: Est + Copy> {
    unary: [E; E_MAX + 1],
    mant: [E; E_MAX + 1],
}

#[inline]
fn adaptive<E: Est>(cost: &mut f64, s: &mut E, bit: u32) {
    let p = s.p1();
    *cost -= if bit == 1 { p } else { 1.0 - p }.log2();
    s.update(bit);
}

impl<E: Est + Copy> Model<E> {
    fn new() -> Self {
        Model { unary: [E::fresh(); E_MAX + 1], mant: [E::fresh(); E_MAX + 1] }
    }
    /// Same decision sequence as the production binarization; fixed p = 1/2 bits cost exactly 1 bit.
    fn code(&mut self, cost: &mut f64, v: u32) {
        let x = u64::from(v) + 1;
        let e_in = (64 - x.leading_zeros()) as usize;
        let mut e = E_MAX;
        for i in 1..E_MAX {
            let b = (e_in > i) as u32;
            adaptive(cost, &mut self.unary[i], b);
            if b == 0 {
                e = i;
                break;
            }
        }
        for j in (0..e - 1).rev() {
            if j == e - 2 {
                adaptive(cost, &mut self.mant[e], ((x >> j) & 1) as u32);
            } else {
                *cost += 1.0;
            }
        }
    }
}

fn replay<E: Est + Copy>(s3: &Stage3Output) -> f64 {
    let mut models = [Model::<E>::new(); ActivityClass::COUNT];
    let mut cost = 0.0;
    let mut run = |res: &[u32], ctx: &[ActivityClass]| {
        for (&v, c) in res.iter().zip(ctx) {
            models[c.index()].code(&mut cost, v);
        }
    };
    match (s3.data(), s3.contexts()) {
        (OrganizedResiduals::Morton(r), OrganizedContexts::Morton(c)) => run(r, c),
        (OrganizedResiduals::BayerPlanes(r), OrganizedContexts::BayerPlanes(c)) => {
            for (r, c) in r.iter().zip(c) {
                run(r, c);
            }
        }
        _ => panic!("contexts do not match the organized data"),
    }
    cost
}

/// Returns (baseline_bits, candidate_bits, sample_count).
pub fn compare(s3: &Stage3Output, n: usize) -> (f64, f64, usize) {
    (replay::<Bit>(s3), replay::<Rp>(s3), n)
}

// ---------------------------------------------------------------- diagnostics (read-only lockstep replay)

pub const SEGS: usize = 16;
pub const PBINS: [f64; 5] = [0.1, 0.3, 0.7, 0.9, 0.99];

#[derive(Clone, Copy, Default)]
pub struct SlotAcc {
    pub n: u64,
    pub ones: u64,
    pub base: f64,
    pub cand: f64,
}
#[derive(Default)]
pub struct Diag {
    pub n_dec: u64,
    pub seg: [[f64; 3]; SEGS], // base, cand, decisions
    pub plane: [[f64; 3]; 4],
    pub class: [[f64; 3]; 4],
    /// [class][kind 0=unary 1=mant][slot]
    pub slot: Vec<SlotAcc>,
    /// by baseline probability assigned to the observed outcome: [base, cand, n]
    pub pbin: [[f64; 3]; 6],
    pub fixed_bits: f64,
}

pub fn slot_index(c: usize, kind: usize, s: usize) -> usize {
    (c * 2 + kind) * (E_MAX + 1) + s
}

pub fn diagnose(s3: &Stage3Output, total_residuals: usize) -> Diag {
    let mut d = Diag { slot: vec![SlotAcc::default(); 4 * 2 * (E_MAX + 1)], ..Default::default() };
    let mut mb = [Model::<Bit>::new(); 4];
    let mut mc = [Model::<Rp>::new(); 4];
    let mut done = 0usize;
    let mut planes: Vec<(usize, &[u32], &[ActivityClass])> = Vec::new();
    match (s3.data(), s3.contexts()) {
        (OrganizedResiduals::BayerPlanes(r), OrganizedContexts::BayerPlanes(c)) => {
            for (i, (r, c)) in r.iter().zip(c).enumerate() {
                planes.push((i, r, c));
            }
        }
        (OrganizedResiduals::Morton(r), OrganizedContexts::Morton(c)) => planes.push((0, r, c)),
        _ => panic!("mismatch"),
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
                let (sb, sc) =
                    if kind == 0 { (&mut mb[ci].unary[s], &mut mc[ci].unary[s]) } else { (&mut mb[ci].mant[s], &mut mc[ci].mant[s]) };
                let (pb, pc) = (Est::p1(sb), Est::p1(sc));
                let (qb, qc) = if b == 1 { (pb, pc) } else { (1.0 - pb, 1.0 - pc) };
                let (cb, cc) = (-qb.log2(), -qc.log2());
                Est::update(sb, b);
                Est::update(sc, b);
                d.n_dec += 1;
                for a in [&mut d.seg[seg], &mut d.plane[pl], &mut d.class[ci]] {
                    a[0] += cb;
                    a[1] += cc;
                    a[2] += 1.0;
                }
                let sl = &mut d.slot[slot_index(ci, kind, s)];
                sl.n += 1;
                sl.ones += b as u64;
                sl.base += cb;
                sl.cand += cc;
                let bi = PBINS.iter().position(|&t| qb < t).unwrap_or(5);
                d.pbin[bi][0] += cb;
                d.pbin[bi][1] += cc;
                d.pbin[bi][2] += 1.0;
            }
        }
    }
    d
}

/// Static (whole-image, per slot) empirical entropy in bits: the cost of an oracle stationary distribution.
pub fn static_cost(a: &SlotAcc) -> f64 {
    let (n, k) = (a.n as f64, a.ones as f64);
    let h = |p: f64| if p <= 0.0 || p >= 1.0 { 0.0 } else { -(p * p.log2() + (1.0 - p) * (1.0 - p).log2()) };
    n * h(k / n)
}
