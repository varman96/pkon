//! Research-only: the production X4 coder (`pkon2x`, config `x4_cfg()`) with its tunable constants lifted into a
//! `Theta`. At `Theta::DEFAULT` it is byte-identical to `pkon2x::run_x(x4_cfg())` (asserted by a test and by the
//! `learn1` binary on every image), so `pkon2x` itself is untouched and the file format is unchanged.
//!
//! Every `Theta` value is a plain integer that a decoder can be handed (or derive); nothing here is image-specific.
//! `CeEnc` additionally accumulates the exact cross-entropy (sum of -log2 P(actual bit)) of the coded stream.
use crate::canon::Canonical;
use crate::pkon2::*;
use crate::pkon2x::{BinDec, BinEnc, BitIo};
use std::sync::OnceLock;

const NI: usize = 4;
const NODES: usize = 27;
const SIGN: usize = 26;
const MAG_CTX: usize = 96;

/// The X4 constants that were fixed by hand. Index order = `PARAMS` order.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Theta {
    /// adaptive-probability count ceiling (`LIMIT`); the update rate floor is 1/(limit+1.5)
    pub limit: u32,
    /// mixer learning rate (`MIXLR`)
    pub mixlr: i32,
    /// initial mixer weight sum, permille (1250 = the hand-set 1.25)
    pub mixinit: u32,
    /// second cut point of `err_level` (the first stays 0)
    pub errcut: i32,
    /// activity scale in quarters: `act_bucket(activity * actscale / 4)`
    pub actscale: i32,
}
impl Theta {
    pub const DEFAULT: Theta = Theta { limit: 255, mixlr: 6, mixinit: 1250, errcut: 8, actscale: 4 };
    pub fn from_levels(l: &[usize; NP]) -> Theta {
        Theta { limit: PARAMS[0].levels[l[0]] as u32, mixlr: PARAMS[1].levels[l[1]] as i32, mixinit: PARAMS[2].levels[l[2]] as u32, errcut: PARAMS[3].levels[l[3]] as i32, actscale: PARAMS[4].levels[l[4]] as i32 }
    }
}

pub const NP: usize = 5;
pub struct Param {
    pub name: &'static str,
    pub levels: &'static [i64],
    /// index of the hand-set production value
    pub default: usize,
}
/// The search grid. Levels bracket the hand-set value; the grid itself is fixed before any data is looked at.
pub const PARAMS: [Param; NP] = [
    Param { name: "limit", levels: &[32, 64, 128, 255], default: 3 },
    Param { name: "mixlr", levels: &[2, 4, 6, 10], default: 2 },
    Param { name: "mixinit", levels: &[800, 1250, 1700], default: 1 },
    Param { name: "errcut", levels: &[5, 8, 14], default: 1 },
    Param { name: "actscale", levels: &[3, 4, 6], default: 1 },
];
pub const GRID: usize = 4 * 4 * 3 * 3 * 3;
pub fn grid_levels(idx: usize) -> [usize; NP] {
    let mut r = [0usize; NP];
    let mut v = idx;
    for p in (0..NP).rev() {
        let n = PARAMS[p].levels.len();
        r[p] = v % n;
        v /= n;
    }
    r
}
pub fn grid_index(l: &[usize; NP]) -> usize {
    let mut v = 0;
    for p in 0..NP {
        v = v * PARAMS[p].levels.len() + l[p];
    }
    v
}
pub fn default_index() -> usize {
    let mut l = [0usize; NP];
    for p in 0..NP {
        l[p] = PARAMS[p].default;
    }
    grid_index(&l)
}

struct Tables {
    stretch: [i16; 4096],
    squash: [u16; 4096],
    rate: [u32; 256],
    cost: Vec<f32>, // cost[p] = -log2(p / 65536), bits
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
    fn update(&mut self, bit: u32, t: &Tables, limit: u32) {
        let target = (bit as i32) << 16;
        let dp = ((target - self.p as i32) as i64 * t.rate[self.n as usize] as i64) >> 16;
        self.p = (self.p as i64 + dp).clamp(32, 65535 - 32) as u16;
        if (self.n as u32) < limit {
            self.n += 1;
        }
    }
}

/// Encoder that also tallies the cross-entropy of every coded bit.
pub struct CeEnc {
    pub inner: BinEnc,
    pub bits: f64,
    acc: f32,
}
impl CeEnc {
    pub fn new() -> Self {
        CeEnc { inner: BinEnc::new(), bits: 0.0, acc: 0.0 }
    }
    pub fn finish(mut self) -> (Vec<u8>, f64) {
        self.bits += self.acc as f64;
        (self.inner.finish(), self.bits)
    }
}
impl BitIo for CeEnc {
    #[inline]
    fn code(&mut self, p1: u32, bit: u32) -> u32 {
        let t = tables();
        self.acc += if bit == 1 { t.cost[p1 as usize] } else { t.cost[65536 - p1 as usize] };
        if self.acc > 1024.0 {
            self.bits += self.acc as f64;
            self.acc = 0.0;
        }
        self.inner.code(p1, bit)
    }
}

struct SignCtx {
    p: usize,
    w: usize,
    n: usize,
    ne: usize,
}

pub struct Coder {
    t: &'static Tables,
    th: Theta,
    m: Vec<Vec<Bit>>,
    mixw: Vec<[i32; NI]>,
    tail: Vec<Bit>,
}
impl Coder {
    pub fn new(planes: usize, th: Theta) -> Self {
        let mut mixw = vec![[0i32; NI]; planes * NODES];
        for (i, w) in mixw.iter_mut().enumerate() {
            let nin = if i % NODES == SIGN { 3 } else { 4 };
            for j in 0..nin {
                w[j] = (65536.0 * th.mixinit as f64 / 1000.0 / nin as f64) as i32;
            }
        }
        Coder { t: tables(), th, m: (0..NI).map(|_| vec![Bit::NEW; planes * MAG_CTX * NODES]).collect(), mixw, tail: vec![Bit::NEW; planes * 9 * 8] }
    }

    #[inline]
    fn bit<I: BitIo>(&mut self, io: &mut I, plane: usize, node: usize, ctxs: &[usize; NI], nin: usize, want: u32) -> u32 {
        let t = self.t;
        let (limit, mixlr) = (self.th.limit, self.th.mixlr);
        let mut idx = [0usize; NI];
        for i in 0..nin {
            idx[i] = (plane * MAG_CTX + ctxs[i]) * NODES + node;
        }
        let mut st = [0i32; NI];
        let ws = &mut self.mixw[plane * NODES + node];
        let mut dot = 0i64;
        for i in 0..nin {
            st[i] = t.stretch[(self.m[i][idx[i]].p >> 4) as usize] as i32;
            dot += ws[i] as i64 * st[i] as i64;
        }
        let d = ((dot >> 16) as i32).clamp(-2047, 2047);
        let p = t.squash[(d + 2048) as usize] as u32;
        let bit = io.code(p, want);
        let err = ((bit as i32) << 12) - (p as i32 >> 4);
        for i in 0..nin {
            ws[i] += (st[i] * err * mixlr + 0x8000) >> 16;
            self.m[i][idx[i]].update(bit, t, limit);
        }
        bit
    }

    #[inline]
    fn tail_bit<I: BitIo>(&mut self, io: &mut I, plane: usize, class: usize, pos: usize, want: u32) -> u32 {
        let b = &mut self.tail[(plane * 9 + class) * 8 + pos];
        let bit = io.code((b.p as u32).clamp(32, 65535 - 32), want);
        b.update(bit, self.t, self.th.limit);
        bit
    }

    fn code_sym<I: BitIo>(&mut self, io: &mut I, k: usize, mc: &[usize; NI], sg: &SignCtx, z_in: u8) -> u8 {
        let m_in = (z_in as u32 + 1) >> 1;
        let c_in = if m_in == 0 { 0 } else { 32 - m_in.leading_zeros() as usize };
        let mut c = 0usize;
        while c < 8 {
            if self.bit(io, k, c, mc, 4, (c_in > c) as u32) == 0 {
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
                    self.bit(io, k, 8 + c, mc, 4, want)
                } else if i == nb - 2 {
                    self.bit(io, k, 16 + (c - 3) * 2 + (v & 1) as usize, mc, 4, want)
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
        let neg = self.bit(io, k, SIGN, &sc, 3, want);
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

/// X4 (`In::Ab, B, C, D`, sign contexts, plain MED) with `th` in place of the hand-set constants.
pub fn run_l<I: BitIo>(pl: &mut Planes, th: Theta, coder: &mut Coder, io: &mut I) {
    let (w, h, np) = (pl.w, pl.h, pl.p.len());
    let mut prev_z = vec![0u8; w * h];
    let mut cur_z = vec![0u8; w * h];
    for k in 0..np {
        for y in 0..h {
            for x in 0..w {
                let i = y * w + x;
                let (xi, yi) = (x as isize, y as isize);
                let nb = neighbors(&pl.p[k], w, x, y);
                let pred = predict(Pred::Med, nb);
                let act = act_bucket(activity(nb) * th.actscale / 4);
                let pq = prev_q(prev_z[i]);
                let e = err_energy(&cur_z, w, x, y);
                let elvl = (e > 0) as usize + (e > th.errcut) as usize;
                let (zw, zn, zne) = (zat(&cur_z, w, xi - 1, yi), zat(&cur_z, w, xi, yi - 1), zat(&cur_z, w, xi + 1, yi - 1));
                let (mx, mn) = (nb.w.max(nb.n), nb.w.min(nb.n));
                let br = if nb.nw >= mx { 0 } else if nb.nw <= mn { 1 } else { 2 };
                let ew = e + zat(&cur_z, w, xi - 2, yi) as i32 + zat(&cur_z, w, xi, yi - 2) as i32
                    + zat(&cur_z, w, xi - 1, yi - 2) as i32 + zat(&cur_z, w, xi + 1, yi - 2) as i32;
                let mc = [
                    (act * 4 + pq) * 3 + br,
                    (prev_q(zw) * 4 + prev_q(zn)) * 4 + pq,
                    (act * 3 + elvl) * 4 + pq,
                    act_bucket(ew / 2) * 8 + act_bucket(prev_z[i] as i32),
                ];
                let sg = SignCtx { p: sgn(prev_z[i]), w: sgn(zw), n: sgn(zn), ne: sgn(zne) };
                let z_in = zigzag((pl.p[k][i] as i32 - pred) as u8);
                let z = coder.code_sym(io, k, &mc, &sg, z_in);
                pl.p[k][i] = (pred as u8).wrapping_add(unzigzag(z));
                cur_z[i] = z;
            }
        }
        std::mem::swap(&mut prev_z, &mut cur_z);
    }
}

pub struct Enc {
    pub stream: Vec<u8>,
    /// stream + the same 11-byte baseline header (comparable to `pkon2x::XOutcome::coded_bytes`)
    pub coded_bytes: usize,
    pub ce_bits: f64,
    pub ms: f64,
}

/// SubG-transformed planes of an image, ready for `encode`.
pub fn prepared(img: &Canonical) -> (Planes, Vec<Vec<u8>>) {
    let orig = to_planes(img);
    let mut work = Planes { w: orig.w, h: orig.h, p: orig.p.clone() };
    color_fwd(&mut work, Color::SubG);
    (work, orig.p)
}

pub fn encode(orig_sub: &Planes, th: Theta) -> Enc {
    let mut work = Planes { w: orig_sub.w, h: orig_sub.h, p: orig_sub.p.clone() };
    let mut coder = Coder::new(work.p.len(), th);
    let mut io = CeEnc::new();
    let t = std::time::Instant::now();
    run_l(&mut work, th, &mut coder, &mut io);
    let (stream, ce_bits) = io.finish();
    let ms = t.elapsed().as_secs_f64() * 1e3;
    Enc { coded_bytes: stream.len() + HEADER_BYTES, stream, ce_bits, ms }
}

/// Decode `stream` with `th` and report (exact?, ms).
pub fn decode_check(stream: &[u8], th: Theta, w: usize, h: usize, orig_planes: &[Vec<u8>]) -> (bool, f64) {
    let np = orig_planes.len();
    let mut pl = Planes { w, h, p: vec![vec![0; w * h]; np] };
    let mut coder = Coder::new(np, th);
    let t = std::time::Instant::now();
    run_l(&mut pl, th, &mut coder, &mut BinDec::new(stream));
    let ms = t.elapsed().as_secs_f64() * 1e3;
    color_inv(&mut pl, Color::SubG);
    (pl.p == orig_planes, ms)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::canon::{Layout, Spec};
    use crate::pkon2x::{measure_x, x4_cfg};

    #[test]
    fn default_theta_is_byte_identical_to_production_x4() {
        let (w, h) = (97u32, 61u32);
        let mut s = 7u32;
        let data: Vec<u8> = (0..w * h * 3)
            .map(|i| {
                s = s.wrapping_mul(1664525).wrapping_add(1013904223);
                ((i / 3) % 251) as u8 / 2 + ((s >> 24) as u8 & 15)
            })
            .collect();
        let img = Canonical::new(Spec { width: w, height: h, layout: Layout::Rgb, bit_depth: 8 }, data).unwrap();
        let (work, orig) = prepared(&img);
        let e = encode(&work, Theta::DEFAULT);
        assert_eq!(e.coded_bytes, measure_x(&img, &x4_cfg()).coded_bytes);
        let (ok, _) = decode_check(&e.stream, Theta::DEFAULT, work.w, work.h, &orig);
        assert!(ok);
        // a non-default theta must still decode exactly
        let th = Theta { limit: 64, mixlr: 10, mixinit: 800, errcut: 14, actscale: 6 };
        let e2 = encode(&work, th);
        assert!(decode_check(&e2.stream, th, work.w, work.h, &orig).0);
        assert!((e2.ce_bits / 8.0 - e2.stream.len() as f64).abs() < 0.01 * e2.stream.len() as f64 + 16.0);
    }
}
