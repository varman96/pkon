//! PKON2 milestone 6: a binarized, count-adaptive, logistic-mixing residual coder.
//!
//! Same pixel order, SubG transform, MED predictor and zigzag residual `z` as `pkon2::run`; ONLY the
//! entropy-coding stage changes. The baseline learns a 256-ary frequency table per context, so every
//! extra context pays the full cost of learning a whole distribution (M5 dilution control: +0.34%).
//! Here `z` is binarized (FLIF-style zero/exponent/mantissa/sign decisions; JPEG XL's hybrid-uint
//! token + raw bits is the same idea) and every binary decision has its own small adaptive
//! probability per context, so a context only learns the few decisions that actually occur in it.
//! Predictions from several differently-conditioned contexts are combined by a logistic mixer
//! (PAQ-style), which is how shared statistics are exploited without one table per joint context.
//!
//! Binarization of z (zz: 0, 1=-1, 2=+1, 3=-2, ...): m = (z+1)>>1 = |e|, class c = bit length of m
//! (0 for m=0), coded as up to 8 unary decisions; then c-1 mantissa bits (first two modelled in
//! context, rest by a plain per-(plane,class,bit) adaptive probability); then the sign if m>0.
//! Encoder and decoder share `run_x` and `Coder::code_sym`; the only difference is `BitIo`.
use crate::canon::Canonical;
use crate::pkon2::*;
use std::sync::OnceLock;

pub const NI: usize = 4;
const NODES: usize = 27;
const SIGN: usize = 26;
const LIMIT: usize = 255;
const MIXLR: i32 = 6;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum In {
    /// activity(8) x prev-plane residual magnitude(4): the frozen baseline context
    A,
    /// quantised zz(eW)(4) x zz(eN)(4) x prev(4): the shape of the neighbouring errors
    B,
    /// activity(8) x error-energy level(3) x prev(4): the M5 context
    C,
    /// wide error energy over 8 neighbours(8) x fine prev-plane magnitude(8)
    D,
}
const MAG_CTX: usize = 96;

#[derive(Clone, Debug)]
pub struct XCfg {
    pub name: &'static str,
    /// context kinds feeding the magnitude decisions; 1 input = no mixer, direct probability
    pub inputs: Vec<In>,
    /// sign decision conditioned on neighbouring / previous-plane residual signs (mixed); else a single flat model
    pub sign_ctx: bool,
}

struct Tables {
    stretch: [i16; 4096],
    squash: [u16; 4096],
    rate: [u32; LIMIT + 1],
}
fn tables() -> &'static Tables {
    static T: OnceLock<Tables> = OnceLock::new();
    T.get_or_init(|| {
        let mut t = Tables { stretch: [0; 4096], squash: [0; 4096], rate: [0; LIMIT + 1] };
        for i in 0..4096 {
            let d = (i as f64 - 2048.0) / 256.0;
            t.squash[i] = (65536.0 / (1.0 + (-d).exp())).round().clamp(16.0, 65520.0) as u16;
            let p = (i as f64 + 0.5) / 4096.0;
            t.stretch[i] = ((p / (1.0 - p)).ln() * 256.0).round().clamp(-2047.0, 2047.0) as i16;
        }
        for n in 0..=LIMIT {
            t.rate[n] = (65536.0 / (n as f64 + 1.5)) as u32;
        }
        t
    })
}

#[derive(Clone, Copy)]
struct Bit {
    p: u16, // P(bit = 1), 16-bit
    n: u8,
}
impl Bit {
    const NEW: Bit = Bit { p: 32768, n: 0 };
    #[inline]
    fn update(&mut self, bit: u32, t: &Tables) {
        let target = (bit as i32) << 16;
        let dp = ((target - self.p as i32) as i64 * t.rate[self.n as usize] as i64) >> 16;
        self.p = (self.p as i64 + dp).clamp(32, 65535 - 32) as u16;
        if (self.n as usize) < LIMIT {
            self.n += 1;
        }
    }
}

// ---------- binary range coder (LZMA-style carry handling, 16-bit probabilities) ----------

pub trait BitIo {
    /// P(bit=1) as 16 bits. Encoders take `bit` and return it; decoders ignore it and return the decoded bit.
    fn code(&mut self, p1: u32, bit: u32) -> u32;
}

pub struct BinEnc {
    low: u64,
    range: u32,
    cache: u8,
    cache_size: u64,
    out: Vec<u8>,
}
impl BinEnc {
    pub fn new() -> Self {
        BinEnc { low: 0, range: u32::MAX, cache: 0, cache_size: 1, out: vec![] }
    }
    fn shift_low(&mut self) {
        if (self.low as u32) < 0xFF00_0000 || (self.low >> 32) != 0 {
            let carry = (self.low >> 32) as u8;
            let mut t = self.cache;
            loop {
                self.out.push(t.wrapping_add(carry));
                t = 0xFF;
                self.cache_size -= 1;
                if self.cache_size == 0 {
                    break;
                }
            }
            self.cache = ((self.low >> 24) & 0xFF) as u8;
        }
        self.cache_size += 1;
        self.low = (self.low & 0x00FF_FFFF) << 8;
    }
    pub fn finish(mut self) -> Vec<u8> {
        for _ in 0..5 {
            self.shift_low();
        }
        self.out.remove(0);
        self.out
    }
}
impl BitIo for BinEnc {
    #[inline]
    fn code(&mut self, p1: u32, bit: u32) -> u32 {
        let bound = ((self.range as u64 * p1 as u64) >> 16) as u32;
        if bit == 1 {
            self.range = bound;
        } else {
            self.low += bound as u64;
            self.range -= bound;
        }
        while self.range < (1 << 24) {
            self.range <<= 8;
            self.shift_low();
        }
        bit
    }
}

pub struct BinDec<'a> {
    code: u32,
    range: u32,
    buf: &'a [u8],
    pos: usize,
}
impl<'a> BinDec<'a> {
    pub fn new(buf: &'a [u8]) -> Self {
        let mut d = BinDec { code: 0, range: u32::MAX, buf, pos: 0 };
        for _ in 0..4 {
            d.code = (d.code << 8) | d.next() as u32;
        }
        d
    }
    fn next(&mut self) -> u8 {
        let b = self.buf.get(self.pos).copied().unwrap_or(0);
        self.pos += 1;
        b
    }
}
impl BitIo for BinDec<'_> {
    #[inline]
    fn code(&mut self, p1: u32, _bit: u32) -> u32 {
        let bound = ((self.range as u64 * p1 as u64) >> 16) as u32;
        let bit = if self.code < bound {
            self.range = bound;
            1
        } else {
            self.code -= bound;
            self.range -= bound;
            0
        };
        while self.range < (1 << 24) {
            self.code = (self.code << 8) | self.next() as u32;
            self.range <<= 8;
        }
        bit
    }
}

// ---------- model ----------

pub struct SignCtx {
    p: usize,
    w: usize,
    n: usize,
    ne: usize,
}

pub struct Coder {
    t: &'static Tables,
    m: Vec<Vec<Bit>>, // [input][(plane*MAG_CTX + ctx)*NODES + node]
    mixw: Vec<[i32; NI]>, // [plane*NODES + node]
    tail: Vec<Bit>,   // [(plane*9 + class)*8 + bitpos]
    nmag: usize,
    sign_ctx: bool,
}
impl Coder {
    pub fn new(planes: usize, cfg: &XCfg) -> Self {
        let nmag = cfg.inputs.len();
        let nsign = if cfg.sign_ctx { 3 } else { 1 };
        let mut mixw = vec![[0i32; NI]; planes * NODES];
        for (i, w) in mixw.iter_mut().enumerate() {
            let nin = if i % NODES == SIGN { nsign } else { nmag };
            for j in 0..nin {
                w[j] = (65536.0 * 1.25 / nin as f64) as i32;
            }
        }
        Coder {
            t: tables(),
            m: (0..NI).map(|_| vec![Bit::NEW; planes * MAG_CTX * NODES]).collect(),
            mixw,
            tail: vec![Bit::NEW; planes * 9 * 8],
            nmag,
            sign_ctx: cfg.sign_ctx,
        }
    }
    /// Bytes of adaptive state (probabilities + mixer weights + tail), for the memory report.
    pub fn state_bytes(&self) -> usize {
        self.m.iter().map(|v| v.len() * std::mem::size_of::<Bit>()).sum::<usize>()
            + self.mixw.len() * std::mem::size_of::<[i32; NI]>()
            + self.tail.len() * std::mem::size_of::<Bit>()
    }

    #[inline]
    fn bit<I: BitIo>(&mut self, io: &mut I, plane: usize, node: usize, ctxs: &[usize; NI], nin: usize, want: u32) -> u32 {
        let t = self.t;
        let mut idx = [0usize; NI];
        for i in 0..nin {
            idx[i] = (plane * MAG_CTX + ctxs[i]) * NODES + node;
        }
        if nin == 1 {
            let b = &mut self.m[0][idx[0]];
            let bit = io.code((b.p as u32).clamp(32, 65535 - 32), want);
            b.update(bit, t);
            return bit;
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
            ws[i] += (st[i] * err * MIXLR + 0x8000) >> 16;
            self.m[i][idx[i]].update(bit, t);
        }
        bit
    }

    #[inline]
    fn tail_bit<I: BitIo>(&mut self, io: &mut I, plane: usize, class: usize, pos: usize, want: u32) -> u32 {
        let b = &mut self.tail[(plane * 9 + class) * 8 + pos];
        let bit = io.code((b.p as u32).clamp(32, 65535 - 32), want);
        b.update(bit, self.t);
        bit
    }

    /// Encode `z_in` (encoder) or decode and return a zigzag residual (decoder).
    pub fn code_sym<I: BitIo>(&mut self, io: &mut I, k: usize, mc: &[usize; NI], sg: &SignCtx, z_in: u8) -> u8 {
        let m_in = (z_in as u32 + 1) >> 1;
        let c_in = if m_in == 0 { 0 } else { 32 - m_in.leading_zeros() as usize };
        let nm = self.nmag;
        let mut c = 0usize;
        while c < 8 {
            if self.bit(io, k, c, mc, nm, (c_in > c) as u32) == 0 {
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
                    self.bit(io, k, 8 + c, mc, nm, want)
                } else if i == nb - 2 {
                    self.bit(io, k, 16 + (c - 3) * 2 + (v & 1) as usize, mc, nm, want)
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
        let neg = if self.sign_ctx {
            let cls = (c - 1).min(3);
            let sc = [sg.p * 4 + cls, sg.w * 9 + sg.n * 3 + sg.ne, sg.p * 9 + sg.w * 3 + sg.n, 0];
            self.bit(io, k, SIGN, &sc, 3, want)
        } else {
            self.bit(io, k, SIGN, &[0; NI], 1, want)
        };
        (2 * m - 1 + (1 - neg)) as u8
    }
}

// ---------- shared loop ----------

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

pub fn run_x<I: BitIo>(pl: &mut Planes, cfg: &XCfg, coder: &mut Coder, io: &mut I) {
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
                let act = act_bucket(activity(nb));
                let pq = prev_q(prev_z[i]);
                let e = err_energy(&cur_z, w, x, y);
                let (zw, zn, zne) = (zat(&cur_z, w, xi - 1, yi), zat(&cur_z, w, xi, yi - 1), zat(&cur_z, w, xi + 1, yi - 1));
                let mut mc = [0usize; NI];
                for (s, kind) in cfg.inputs.iter().enumerate() {
                    mc[s] = match kind {
                        In::A => act * 4 + pq,
                        In::B => (prev_q(zw) * 4 + prev_q(zn)) * 4 + pq,
                        In::C => (act * 3 + err_level(e)) * 4 + pq,
                        In::D => {
                            let ew = e + zat(&cur_z, w, xi - 2, yi) as i32 + zat(&cur_z, w, xi, yi - 2) as i32
                                + zat(&cur_z, w, xi - 1, yi - 2) as i32 + zat(&cur_z, w, xi + 1, yi - 2) as i32;
                            act_bucket(ew / 2) * 8 + act_bucket(prev_z[i] as i32)
                        }
                    };
                }
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

pub struct XOutcome {
    pub coded_bytes: usize,
    pub enc_ms: f64,
    pub dec_ms: f64,
    pub exact: bool,
    pub state_bytes: usize,
}

/// Real coded bytes (stream + the same 11-byte header as the baseline) with a verified exact decode.
pub fn measure_x(img: &Canonical, cfg: &XCfg) -> XOutcome {
    let orig = to_planes(img);
    let np = orig.p.len();
    let mut work = Planes { w: orig.w, h: orig.h, p: orig.p.clone() };
    color_fwd(&mut work, Color::SubG);
    let mut coder = Coder::new(np, cfg);
    let mut enc = BinEnc::new();
    let t = std::time::Instant::now();
    run_x(&mut work, cfg, &mut coder, &mut enc);
    let stream = enc.finish();
    let enc_ms = t.elapsed().as_secs_f64() * 1e3;
    let state_bytes = coder.state_bytes();

    let mut dec_pl = Planes { w: orig.w, h: orig.h, p: vec![vec![0; orig.w * orig.h]; np] };
    let mut dcoder = Coder::new(np, cfg);
    let t = std::time::Instant::now();
    run_x(&mut dec_pl, cfg, &mut dcoder, &mut BinDec::new(&stream));
    let dec_ms = t.elapsed().as_secs_f64() * 1e3;
    color_inv(&mut dec_pl, Color::SubG);
    XOutcome { coded_bytes: stream.len() + HEADER_BYTES, enc_ms, dec_ms, exact: dec_pl.p == orig.p, state_bytes }
}
