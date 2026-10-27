//! REL1 research copy: the production X4 coder (`pkon2x`, `x4_cfg()`) with optional EXTRA mixer inputs. Production code and the
//! file format are untouched. With no extras this is byte-identical to `pkonfile::encode_payload` (asserted by tests and by the
//! `rel1` runner on every image).
//!
//! Decoding dependencies (everything below is available to the decoder at the moment pixel (x,y) of plane k is decoded):
//!   * P (predictor disagreement): GAP and the gradient predictor are computed from plane k's W,N,NW,NE,WW,NN,NNE neighbours, exactly the
//!     pixels MED already reads (plus WW,NN,NNE which `gap()` reads and which are earlier in raster order). No pixel at or after (x,y) is
//!     read; `neighbors()`/`gap()` substitute earlier pixels at the borders. Integer arithmetic only; d = alt - MED is in [-255,255].
//!   * X (cross-plane residual): the zigzag residuals `zs[j]` of planes j<k, at the same index i. Planes are coded one after another, so
//!     all of plane j<k (including positions after i) is already decoded. Plane 0 has no earlier plane: X is absent there.
//!   * Dummy: a hash of (plane, x, y, salt) only: carries no image information, identical on both sides.
//!   * Extra-input weights and probabilities update in the same order as the base inputs, after the bit is coded.
//! Cross-entropy is accumulated per (plane, decision category) from the exact probability handed to the range coder.
use crate::canon::Canonical;
use crate::pkon2::*;
use crate::pkon2x::{BinDec, BinEnc, BitIo};
use std::sync::OnceLock;

const NI: usize = 6;
const NODES: usize = 27;
const SIGN: usize = 26;
const BASE_CAP: usize = 96;
const LIMIT: u32 = 255;
const MIXLR: i32 = 6;
pub const NCAT: usize = 5;
pub const CAT_NAMES: [&str; NCAT] = ["zero", "ge2", "class", "mantissa", "sign"];
pub const MAXP: usize = 4;
pub const FCARD: usize = 2401;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Extra {
    P,
    X,
    /// P and X as ONE joint context (49 x 49)
    Joint,
    /// meaningless input: hash of (plane, x, y, salt) modulo card
    Dummy { salt: u32, card: u32 },
    /// REL2 diagnostic: ONE class, always 0 -- a pure per-(plane, decision node) adaptive probability fed to the mixer (bias input)
    Const,
}
impl Extra {
    pub fn card(self) -> usize {
        match self {
            Extra::P | Extra::X => 49,
            Extra::Joint => FCARD,
            Extra::Dummy { card, .. } => card as usize,
            Extra::Const => 1,
        }
    }
}
#[derive(Clone, Copy, Debug)]
pub struct Arm {
    pub name: &'static str,
    pub extras: &'static [Extra],
}
pub const ARMS: [Arm; 8] = [
    Arm { name: "control", extras: &[] },
    Arm { name: "dummy", extras: &[Extra::Dummy { salt: 1, card: 49 }] },
    Arm { name: "P", extras: &[Extra::P] },
    Arm { name: "X", extras: &[Extra::X] },
    Arm { name: "P+X", extras: &[Extra::P, Extra::X] },
    Arm { name: "PxX", extras: &[Extra::Joint] },
    Arm { name: "dummy2", extras: &[Extra::Dummy { salt: 1, card: 49 }, Extra::Dummy { salt: 2, card: 49 }] },
    Arm { name: "dummyJ", extras: &[Extra::Dummy { salt: 3, card: FCARD as u32 }] },
];

/// REL2 final diagnostic arms (criteria: results/20261027-rel2-final-diag/criteria.md). Separate from `ARMS` so REL1 stays reproducible.
pub const DIAG_ARMS: [Arm; 5] = [
    Arm { name: "control", extras: &[] },
    Arm { name: "const", extras: &[Extra::Const] },
    Arm { name: "dummy", extras: &[Extra::Dummy { salt: 1, card: 49 }] },
    Arm { name: "P", extras: &[Extra::P] },
    Arm { name: "P+const", extras: &[Extra::P, Extra::Const] },
];

// ---------------------------------------------------------------- features

#[inline]
pub fn qd(d: i32) -> usize {
    match d {
        i32::MIN..=-9 => 0,
        -8..=-3 => 1,
        -2..=-1 => 2,
        0 => 3,
        1..=2 => 4,
        3..=8 => 5,
        _ => 6,
    }
}
/// The M7 signed-error quantiser.
#[inline]
pub fn q7(e: i8) -> usize {
    match e {
        i8::MIN..=-5 => 0,
        -4..=-2 => 1,
        -1 => 2,
        0 => 3,
        1 => 4,
        2..=4 => 5,
        _ => 6,
    }
}
/// P: signed disagreement of GAP and the gradient predictor from MED (49 classes). Reads only plane-k pixels before (x,y).
#[inline]
pub fn p_feature(p: &[u8], w: usize, x: usize, y: usize, nb: Nb, med: i32) -> usize {
    qd(gap(p, w, x, y, nb) - med) * 7 + qd(predict(Pred::Grad, nb) - med)
}
/// X: signed same-pixel residuals of the two nearest earlier planes (k>=1). Plane 1 sees only plane 0 (second term = zero class).
#[inline]
pub fn x_feature(k: usize, zs: &[Vec<u8>], i: usize) -> usize {
    let e = |j: usize| unzigzag(zs[j][i]) as i8;
    match k {
        0 => 24,
        1 => q7(e(0)) * 7 + 3,
        _ => q7(e(k - 1)) * 7 + q7(e(k - 2)),
    }
}
#[inline]
pub fn dummy_ctx(salt: u32, card: u32, k: usize, x: usize, y: usize) -> usize {
    let mut h = (x as u32).wrapping_mul(0x9E37_79B1) ^ (y as u32).wrapping_mul(0x85EB_CA77) ^ (k as u32 + 1).wrapping_mul(0xC2B2_AE3D) ^ salt.wrapping_mul(0x27D4_EB2F);
    h ^= h >> 15;
    h = h.wrapping_mul(0x2C1B_3C6D);
    h ^= h >> 12;
    h = h.wrapping_mul(0x297A_2D39);
    h ^= h >> 15;
    (h % card) as usize
}

// ---------------------------------------------------------------- probability machinery (copy of pkon2x, constants fixed)

struct Tables {
    stretch: [i16; 4096],
    squash: [u16; 4096],
    rate: [u32; 256],
    cost: Vec<f32>,
}
fn tables() -> &'static Tables {
    static T: OnceLock<Tables> = OnceLock::new();
    T.get_or_init(|| {
        let mut t = Tables { stretch: [0; 4096], squash: [0; 4096], rate: [0; 256], cost: vec![0.0; 65537] };
        for i in 0..4096 {
            let d = (i as f64 - 2048.0) / 256.0;
            t.squash[i] = (65536.0 / (1.0 + (-d).exp())).round().clamp(16.0, 65520.0) as u16;
            let p = (i as f64 + 0.5) / 4096.0;
            t.stretch[i] = ((p / (1.0 - p)).ln() * 256.0).round().clamp(-2047.0, 2047.0) as i16;
        }
        for n in 0..256 {
            t.rate[n] = (65536.0 / (n as f64 + 1.5)) as u32;
        }
        for p in 1..=65536usize {
            t.cost[p] = -((p as f64 / 65536.0).log2()) as f32;
        }
        t.cost[0] = 16.0;
        t
    })
}
#[derive(Clone, Copy)]
struct Bit {
    p: u16,
    n: u8,
}
impl Bit {
    const NEW: Bit = Bit { p: 32768, n: 0 };
    #[inline]
    fn update(&mut self, bit: u32, t: &Tables) {
        let target = (bit as i32) << 16;
        let dp = ((target - self.p as i32) as i64 * t.rate[self.n as usize] as i64) >> 16;
        self.p = (self.p as i64 + dp).clamp(32, 65535 - 32) as u16;
        if (self.n as u32) < LIMIT {
            self.n += 1;
        }
    }
}

/// `BitIo` plus a hook that tells the sink which (plane, decision category) the next bit belongs to.
pub trait RelIo: BitIo {
    fn tag(&mut self, _cat: usize, _plane: usize) {}
}
impl RelIo for BinDec<'_> {}

pub struct CeEnc {
    inner: BinEnc,
    cur: (usize, usize),
    /// exact cross-entropy (bits) by [plane][category]
    pub bits: [[f64; NCAT]; MAXP],
    /// number of coded binary decisions by [plane][category]
    pub count: [[u64; NCAT]; MAXP],
}
impl CeEnc {
    pub fn new() -> Self {
        CeEnc { inner: BinEnc::new(), cur: (0, 0), bits: [[0.0; NCAT]; MAXP], count: [[0; NCAT]; MAXP] }
    }
    pub fn finish(self) -> (Vec<u8>, [[f64; NCAT]; MAXP], [[u64; NCAT]; MAXP]) {
        (self.inner.finish(), self.bits, self.count)
    }
}
impl BitIo for CeEnc {
    #[inline]
    fn code(&mut self, p1: u32, bit: u32) -> u32 {
        let t = tables();
        let c = if bit == 1 { t.cost[p1 as usize] } else { t.cost[65536 - p1 as usize] };
        self.bits[self.cur.0][self.cur.1] += c as f64;
        self.count[self.cur.0][self.cur.1] += 1;
        self.inner.code(p1, bit)
    }
}
impl RelIo for CeEnc {
    #[inline]
    fn tag(&mut self, cat: usize, plane: usize) {
        self.cur = (plane, cat);
    }
}

#[inline]
fn cat_of(node: usize) -> usize {
    match node {
        0 => 0,
        1 => 1,
        2..=7 => 2,
        SIGN => 4,
        _ => 3,
    }
}

#[derive(Clone, Copy, Default)]
struct ExtCtx {
    n: usize,
    slot: [usize; 2],
    ctx: [usize; 2],
}
struct SignCtx {
    p: usize,
    w: usize,
    n: usize,
    ne: usize,
}

pub struct Coder {
    t: &'static Tables,
    caps: [usize; NI],
    m: Vec<Vec<Bit>>,
    mixw: Vec<[i32; NI]>,
    tail: Vec<Bit>,
}
impl Coder {
    pub fn new(planes: usize, arm: &Arm) -> Self {
        assert!(planes <= MAXP && arm.extras.len() <= 2);
        let mut caps = [BASE_CAP; NI];
        caps[3] = BASE_CAP;
        caps[4] = 1;
        caps[5] = 1;
        for (j, e) in arm.extras.iter().enumerate() {
            caps[4 + j] = e.card();
        }
        let mut mixw = vec![[0i32; NI]; planes * NODES];
        for (i, w) in mixw.iter_mut().enumerate() {
            let nin = if i % NODES == SIGN { 3 } else { 4 };
            for j in 0..nin {
                w[j] = (65536.0 * 1.25 / nin as f64) as i32;
            }
        }
        Coder { t: tables(), caps, m: (0..NI).map(|s| vec![Bit::NEW; planes * caps[s] * NODES]).collect(), mixw, tail: vec![Bit::NEW; planes * 9 * 8] }
    }

    /// Hash of every probability, count and mixer weight: encoder and decoder must end in the same state.
    pub fn state_hash(&self) -> u64 {
        let mut h = 0xcbf29ce484222325u64;
        let mut mix = |v: u64| {
            h ^= v;
            h = h.wrapping_mul(0x100000001b3);
        };
        for tbl in &self.m {
            for b in tbl {
                mix(b.p as u64 | (b.n as u64) << 16);
            }
        }
        for b in &self.tail {
            mix(b.p as u64 | (b.n as u64) << 16);
        }
        for w in &self.mixw {
            for v in w {
                mix(*v as u32 as u64);
            }
        }
        h
    }
    /// Mean final mixer weight of the extra inputs for the sign node and node 0, per plane (adaptation diagnostic).
    pub fn extra_weights(&self, planes: usize, n_extra: usize) -> Vec<(usize, usize, f64, f64)> {
        let mut v = vec![];
        for p in 0..planes {
            for j in 0..n_extra {
                v.push((p, j, self.mixw[p * NODES][4 + j] as f64 / 65536.0, self.mixw[p * NODES + SIGN][3 + j] as f64 / 65536.0));
            }
        }
        v
    }

    #[inline]
    fn bit<I: RelIo>(&mut self, io: &mut I, plane: usize, node: usize, base: &[usize; 4], nb: usize, ex: &ExtCtx, want: u32) -> u32 {
        let t = self.t;
        let nin = nb + ex.n;
        let mut sl = [0usize; NI];
        let mut cx = [0usize; NI];
        for i in 0..nb {
            sl[i] = i;
            cx[i] = base[i];
        }
        for j in 0..ex.n {
            sl[nb + j] = ex.slot[j];
            cx[nb + j] = ex.ctx[j];
        }
        let mut idx = [0usize; NI];
        for i in 0..nin {
            idx[i] = (plane * self.caps[sl[i]] + cx[i]) * NODES + node;
        }
        let mut st = [0i32; NI];
        let ws = &mut self.mixw[plane * NODES + node];
        let mut dot = 0i64;
        for i in 0..nin {
            st[i] = t.stretch[(self.m[sl[i]][idx[i]].p >> 4) as usize] as i32;
            dot += ws[i] as i64 * st[i] as i64;
        }
        let d = ((dot >> 16) as i32).clamp(-2047, 2047);
        let p = t.squash[(d + 2048) as usize] as u32;
        io.tag(cat_of(node), plane);
        let bit = io.code(p, want);
        let err = ((bit as i32) << 12) - (p as i32 >> 4);
        for i in 0..nin {
            ws[i] += (st[i] * err * MIXLR + 0x8000) >> 16;
            self.m[sl[i]][idx[i]].update(bit, t);
        }
        bit
    }

    #[inline]
    fn tail_bit<I: RelIo>(&mut self, io: &mut I, plane: usize, class: usize, pos: usize, want: u32) -> u32 {
        let b = &mut self.tail[(plane * 9 + class) * 8 + pos];
        io.tag(3, plane);
        let bit = io.code((b.p as u32).clamp(32, 65535 - 32), want);
        b.update(bit, self.t);
        bit
    }

    fn code_sym<I: RelIo>(&mut self, io: &mut I, k: usize, mc: &[usize; 4], sg: &SignCtx, ex: &ExtCtx, z_in: u8) -> u8 {
        let m_in = (z_in as u32 + 1) >> 1;
        let c_in = if m_in == 0 { 0 } else { 32 - m_in.leading_zeros() as usize };
        let mut c = 0usize;
        while c < 8 {
            if self.bit(io, k, c, mc, 4, ex, (c_in > c) as u32) == 0 {
                break;
            }
            c += 1;
        }
        let m: u32 = if c <= 1 {
            c as u32
        } else if c == 8 {
            128
        } else {
            let nb = c - 1;
            let mut v = 1u32;
            for i in (0..nb).rev() {
                let want = (m_in >> i) & 1;
                let b = if i == nb - 1 {
                    self.bit(io, k, 8 + c, mc, 4, ex, want)
                } else if i == nb - 2 {
                    self.bit(io, k, 16 + (c - 3) * 2 + (v & 1) as usize, mc, 4, ex, want)
                } else {
                    self.tail_bit(io, k, c, i, want)
                };
                v = (v << 1) | b;
            }
            v
        };
        if m == 0 {
            return 0;
        }
        let want = (z_in & 1) as u32;
        let cls = (c - 1).min(3);
        let sc = [sg.p * 4 + cls, sg.w * 9 + sg.n * 3 + sg.ne, sg.p * 9 + sg.w * 3 + sg.n, 0];
        let neg = self.bit(io, k, SIGN, &sc, 3, ex, want);
        (2 * m - 1 + (1 - neg)) as u8
    }
}

#[inline]
fn zat(z: &[u8], w: usize, x: isize, y: isize) -> u8 {
    if x < 0 || y < 0 || x as usize >= w {
        0
    } else {
        z[y as usize * w + x as usize]
    }
}
#[inline]
fn sgn(z: u8) -> usize {
    if z == 0 {
        0
    } else if z & 1 == 1 {
        1
    } else {
        2
    }
}

/// Counts for the static information estimates (encoder pass only). Indexed [plane][cond ctx][feature (P*49+X)][outcome].
pub struct Stats {
    pub sign: Vec<u32>, // cond = existing sign context #2 (27)
    pub zero: Vec<u32>, // cond = existing magnitude context A/b (96); outcome = (z == 0)
    pub npix: [u64; MAXP],
}
impl Stats {
    pub fn new() -> Self {
        Stats { sign: vec![0; MAXP * 27 * FCARD * 2], zero: vec![0; MAXP * 96 * FCARD * 2], npix: [0; MAXP] }
    }
}

pub struct Trace {
    pub hash: u64,
}

/// X4 (A/b,B,C,D + sign contexts, plain MED) plus `arm.extras`. Returns a hash of every pixel's contexts and symbols.
pub fn run_rel<I: RelIo>(pl: &mut Planes, arm: &Arm, coder: &mut Coder, io: &mut I, mut stats: Option<&mut Stats>) -> u64 {
    let (w, h, np) = (pl.w, pl.h, pl.p.len());
    let mut zs: Vec<Vec<u8>> = vec![vec![0u8; w * h]; np];
    let zero = vec![0u8; w * h];
    let need_p = arm.extras.iter().any(|e| matches!(e, Extra::P | Extra::Joint)) || stats.is_some();
    let mut trace = 0xcbf29ce484222325u64;
    let mixh = |v: u64, t: &mut u64| {
        *t ^= v;
        *t = t.wrapping_mul(0x100000001b3);
    };
    for k in 0..np {
        for y in 0..h {
            for x in 0..w {
                let i = y * w + x;
                let (xi, yi) = (x as isize, y as isize);
                let nb = neighbors(&pl.p[k], w, x, y);
                let med = predict(Pred::Med, nb);
                let pred = med;
                let act = act_bucket(activity(nb));
                let (cur_z, prev_z): (&[u8], &[u8]) = if k > 0 { (&zs[k], &zs[k - 1]) } else { (&zs[0], &zero) };
                let pq = prev_q(prev_z[i]);
                let e = err_energy(cur_z, w, x, y);
                let (zw, zn, zne) = (zat(cur_z, w, xi - 1, yi), zat(cur_z, w, xi, yi - 1), zat(cur_z, w, xi + 1, yi - 1));
                let (mx, mn) = (nb.w.max(nb.n), nb.w.min(nb.n));
                let br = if nb.nw >= mx { 0 } else if nb.nw <= mn { 1 } else { 2 };
                let ew = e + zat(cur_z, w, xi - 2, yi) as i32 + zat(cur_z, w, xi, yi - 2) as i32 + zat(cur_z, w, xi - 1, yi - 2) as i32 + zat(cur_z, w, xi + 1, yi - 2) as i32;
                let mc = [
                    (act * 4 + pq) * 3 + br,
                    (prev_q(zw) * 4 + prev_q(zn)) * 4 + pq,
                    (act * 3 + err_level(e)) * 4 + pq,
                    act_bucket(ew / 2) * 8 + act_bucket(prev_z[i] as i32),
                ];
                let sg = SignCtx { p: sgn(prev_z[i]), w: sgn(zw), n: sgn(zn), ne: sgn(zne) };
                // ---- extra features
                let pf = if need_p { p_feature(&pl.p[k], w, x, y, nb, med) } else { 0 };
                let xf = x_feature(k, &zs, i);
                let mut ex = ExtCtx::default();
                for (j, e) in arm.extras.iter().enumerate() {
                    let c = match *e {
                        Extra::P => Some(pf),
                        Extra::X => (k > 0).then_some(xf),
                        Extra::Joint => Some(pf * 49 + xf),
                        Extra::Dummy { salt, card } => Some(dummy_ctx(salt, card, k, x, y)),
                        Extra::Const => Some(0),
                    };
                    if let Some(c) = c {
                        assert!(c < e.card(), "context out of range");
                        ex.slot[ex.n] = 4 + j;
                        ex.ctx[ex.n] = c;
                        ex.n += 1;
                    }
                }
                let z_in = zigzag((pl.p[k][i] as i32 - pred) as u8);
                let z = coder.code_sym(io, k, &mc, &sg, &ex, z_in);
                pl.p[k][i] = (pred as u8).wrapping_add(unzigzag(z));
                zs[k][i] = z;
                if let Some(s) = stats.as_deref_mut() {
                    let f = pf * 49 + xf;
                    s.npix[k] += 1;
                    s.zero[((k * 96 + mc[0]) * FCARD + f) * 2 + (z == 0) as usize] += 1;
                    if z != 0 {
                        s.sign[((k * 27 + sg.w * 9 + sg.n * 3 + sg.ne) * FCARD + f) * 2 + (z & 1) as usize] += 1;
                    }
                }
                mixh(k as u64 | (mc[0] as u64) << 4 | (mc[1] as u64) << 14 | (mc[2] as u64) << 24 | (mc[3] as u64) << 34, &mut trace);
                mixh((ex.ctx[0] as u64) | (ex.ctx[1] as u64) << 12 | (ex.n as u64) << 24 | (z as u64) << 32, &mut trace);
            }
        }
    }
    trace
}

pub struct RelEnc {
    pub stream: Vec<u8>,
    pub coded_bytes: usize,
    pub bits: [[f64; NCAT]; MAXP],
    pub count: [[u64; NCAT]; MAXP],
    pub trace: u64,
    pub state: u64,
    /// final mixer weight of each extra input: (plane, extra index, weight at node 0, weight at the sign node)
    pub extra_w: Vec<(usize, usize, f64, f64)>,
    pub ms: f64,
}
pub fn encode_rel(sub: &Planes, arm: &Arm, stats: Option<&mut Stats>) -> RelEnc {
    let mut work = Planes { w: sub.w, h: sub.h, p: sub.p.clone() };
    let mut coder = Coder::new(work.p.len(), arm);
    let mut io = CeEnc::new();
    let t = std::time::Instant::now();
    let trace = run_rel(&mut work, arm, &mut coder, &mut io, stats);
    let (stream, bits, count) = io.finish();
    let ms = t.elapsed().as_secs_f64() * 1e3;
    RelEnc { coded_bytes: stream.len() + HEADER_BYTES, stream, bits, count, trace, state: coder.state_hash(), extra_w: coder.extra_weights(work.p.len(), arm.extras.len()), ms }
}
pub struct RelDec {
    pub planes: Vec<Vec<u8>>,
    pub trace: u64,
    pub state: u64,
    pub consumed: usize,
    pub ms: f64,
}
/// Decode `stream` into SubG-domain planes (caller applies `color_inv`). Never panics on arbitrary input.
pub fn decode_rel(stream: &[u8], arm: &Arm, w: usize, h: usize, np: usize) -> RelDec {
    let mut pl = Planes { w, h, p: vec![vec![0; w * h]; np] };
    let mut coder = Coder::new(np, arm);
    let mut dec = BinDec::new(stream);
    let t = std::time::Instant::now();
    let trace = run_rel(&mut pl, arm, &mut coder, &mut dec, None);
    let ms = t.elapsed().as_secs_f64() * 1e3;
    RelDec { planes: pl.p, trace, state: coder.state_hash(), consumed: dec.consumed(), ms }
}
pub fn invert(dec_planes: Vec<Vec<u8>>, w: usize, h: usize) -> Vec<Vec<u8>> {
    let mut pl = Planes { w, h, p: dec_planes };
    color_inv(&mut pl, Color::SubG);
    pl.p
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::canon::{Layout, Spec};
    use crate::pkonfile::encode_payload;

    fn img(w: u32, h: u32, layout: Layout, kind: u32) -> Canonical {
        let ch = layout.channels();
        let mut s = 12345u32 + kind;
        let data: Vec<u8> = (0..w as usize * h as usize * ch)
            .map(|i| {
                s = s.wrapping_mul(1664525).wrapping_add(1013904223);
                let (px, c) = (i / ch, i % ch);
                let (x, y) = (px % w as usize, px / w as usize);
                match kind {
                    0 => 0,
                    1 => 255,
                    2 => if (x + y) % 2 == 0 { 0 } else { 255 }, // maximal residuals
                    3 => (s >> 24) as u8,                          // noise
                    4 => ((x * 3 + y * 2 + c * 40) % 256) as u8,   // ramp with wraparound
                    5 => if c == 1 { (s >> 24) as u8 } else { 255 - (s >> 28) as u8 }, // wrapped chroma differences
                    _ => ((x / 3 + y / 2) as u8).wrapping_add((s >> 29) as u8),
                }
            })
            .collect();
        Canonical::new(Spec { width: w, height: h, layout, bit_depth: 8 }, data).unwrap()
    }

    fn roundtrip(c: &Canonical, arm: &Arm) -> (usize, Vec<u8>) {
        let (sub, orig) = crate::pkonlearn::prepared(c);
        let e = encode_rel(&sub, arm, None);
        let d = decode_rel(&e.stream, arm, sub.w, sub.h, sub.p.len());
        assert_eq!(d.trace, e.trace, "{}: context/symbol trace differs", arm.name);
        assert_eq!(d.state, e.state, "{}: model state differs", arm.name);
        assert!(d.consumed <= e.stream.len() + 4, "{}: decoder over-read", arm.name);
        assert_eq!(invert(d.planes, sub.w, sub.h), orig, "{}: pixels differ", arm.name);
        let e2 = encode_rel(&sub, arm, None);
        assert_eq!(e.stream, e2.stream, "{}: not repeatable", arm.name);
        (e.coded_bytes, e.stream)
    }

    #[test]
    fn control_equals_production_and_every_arm_round_trips_on_edge_images() {
        let sizes = [(1u32, 1u32), (1, 7), (9, 1), (2, 2), (3, 3), (5, 4), (17, 13), (64, 33)];
        for layout in [Layout::Rgb, Layout::Gray] {
            for &(w, h) in &sizes {
                for kind in 0..7 {
                    let c = img(w, h, layout, kind);
                    let prod = encode_payload(&c);
                    for arm in ARMS.iter() {
                        let (_, stream) = roundtrip(&c, arm);
                        if arm.extras.is_empty() {
                            assert_eq!(stream, prod, "control != production ({w}x{h} {layout:?} kind {kind})");
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn diag_arms_round_trip_on_edge_images() {
        let sizes = [(1u32, 1u32), (1, 7), (9, 1), (3, 3), (17, 13), (64, 33)];
        for layout in [Layout::Rgb, Layout::Gray] {
            for &(w, h) in &sizes {
                for kind in 0..7 {
                    let c = img(w, h, layout, kind);
                    let prod = encode_payload(&c);
                    for arm in DIAG_ARMS.iter() {
                        let (_, stream) = roundtrip(&c, arm);
                        if arm.extras.is_empty() {
                            assert_eq!(stream, prod);
                        }
                    }
                }
            }
        }
    }

    /// Production codec 2 (`pkonfile`, `pkon2x::x4p_cfg`) must emit exactly the stream the independent REL1 P arm emits, and the production
    /// feature must equal REL1's `p_feature` at every pixel.
    #[test]
    fn production_codec2_equals_rel1_p_arm() {
        use crate::pkonfile::{encode_payload_codec, CODEC_X4P};
        let parm = ARMS.iter().find(|a| a.name == "P").unwrap();
        for layout in [Layout::Rgb, Layout::Gray] {
            for &(w, h) in &[(1u32, 1u32), (1, 7), (9, 1), (3, 3), (17, 13), (64, 33)] {
                for kind in 0..7 {
                    let c = img(w, h, layout, kind);
                    let (sub, _) = crate::pkonlearn::prepared(&c);
                    assert_eq!(encode_payload_codec(&c, CODEC_X4P), encode_rel(&sub, parm, None).stream, "{w}x{h} {layout:?} kind {kind}");
                }
            }
        }
        let (w, h) = (11usize, 9usize);
        let mut s = 7u32;
        let p: Vec<u8> = (0..w * h).map(|_| { s = s.wrapping_mul(1664525).wrapping_add(1013904223); (s >> 24) as u8 }).collect();
        for y in 0..h {
            for x in 0..w {
                let nb = neighbors(&p, w, x, y);
                let med = predict(Pred::Med, nb);
                assert_eq!(crate::pkon2x::p_class(&p, w, x, y, nb, med), p_feature(&p, w, x, y, nb, med));
            }
        }
    }

    #[test]
    fn features_are_causal() {
        // P at (x,y) must not change when pixels at or after (x,y) in raster order change.
        let (w, h) = (9usize, 7usize);
        let mut s = 99u32;
        let base: Vec<u8> = (0..w * h).map(|_| { s = s.wrapping_mul(1664525).wrapping_add(1013904223); (s >> 24) as u8 }).collect();
        for y in 0..h {
            for x in 0..w {
                let i = y * w + x;
                let mut alt = base.clone();
                for j in i..w * h {
                    alt[j] = alt[j].wrapping_add(77);
                }
                let f = |p: &[u8]| {
                    let nb = neighbors(p, w, x, y);
                    p_feature(p, w, x, y, nb, predict(Pred::Med, nb))
                };
                assert_eq!(f(&base), f(&alt), "P reads a non-causal pixel at ({x},{y})");
            }
        }
    }

    #[test]
    fn feature_ranges_and_dummy_independence() {
        for d in -255..=255 {
            assert!(qd(d) < 7);
        }
        for e in i8::MIN..=i8::MAX {
            assert!(q7(e) < 7);
        }
        for z in 0..=255u8 {
            assert!(q7(unzigzag(z) as i8) < 7);
        }
        for &(salt, card) in &[(1u32, 49u32), (2, 49), (3, FCARD as u32)] {
            for k in 0..3 {
                for y in 0..40 {
                    for x in 0..40 {
                        assert!(dummy_ctx(salt, card, k, x, y) < card as usize);
                    }
                }
            }
        }
    }

    #[test]
    fn truncated_and_corrupted_streams_never_panic_and_over_read_is_visible() {
        let c = img(40, 30, Layout::Rgb, 6);
        let (sub, orig) = crate::pkonlearn::prepared(&c);
        for arm in ARMS.iter() {
            let e = encode_rel(&sub, arm, None);
            for cut in [0usize, 1, e.stream.len() / 3, e.stream.len() - 1] {
                let d = decode_rel(&e.stream[..cut], arm, sub.w, sub.h, 3);
                if cut < e.stream.len() - 8 {
                    assert!(d.consumed > cut, "{}: over-read not visible at cut {cut}", arm.name);
                }
                assert!(invert(d.planes, sub.w, sub.h) != orig || cut + 4 >= e.stream.len());
            }
            let mut bad = e.stream.clone();
            let mid = bad.len() / 2;
            bad[mid] ^= 0x5a;
            let d = decode_rel(&bad, arm, sub.w, sub.h, 3); // must not panic
            assert!(invert(d.planes, sub.w, sub.h) != orig);
        }
    }
}
