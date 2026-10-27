//! PKON2 milestone-1 probe: a deliberately small context-modelled lossless coder for 8-bit
//! planes, built so each statistical hypothesis is a config switch and every config is
//! measured by REAL coded bytes (adaptive range coder), exact round trip, and entropy estimates.
//!
//! Pipeline per image: optional reversible colour transform -> per-plane causal predictor ->
//! residual mod 256 (zigzag) -> adaptive frequency model chosen by a causal context -> range coder.
//! The decoder runs the same `run` loop, so a config only counts if decode reproduces the pixels.
use crate::canon::{Canonical, Layout, Spec};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Pred {
    W,
    N,
    Avg,
    Grad,
    Med,
    /// CALIC's Gradient-Adjusted Predictor (milestone 3). Not in `PREDS`; uses `gap()`, not `predict()`.
    Gap,
}
pub const PREDS: [Pred; 5] = [Pred::W, Pred::N, Pred::Avg, Pred::Grad, Pred::Med];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Color {
    /// planes coded as G,R,B (G first so it can be context); no transform
    Raw,
    /// G, R-G+128, B-G+128 (mod 256)
    SubG,
    /// planes G,R,B; R and B predictions are shifted by G's actual-minus-predicted error
    Coupled,
    /// Milestone 2: per pixel, choose between the SubG and Coupled predictions for R and B by which
    /// one had the smaller summed residual over already-decoded neighbours (window id: 2, 4 or 6
    /// neighbours). No signalling bits; planes stay in the original domain.
    Adaptive(u8),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Ctx {
    /// one model per plane
    None,
    /// 8 buckets of local gradient activity of the same plane
    Act,
    /// activity x 4 buckets of the previous plane's residual magnitude at this pixel
    ActX,
    /// Milestone 4: bucket of E = zz(eW)+zz(eN)+zz(eNW)+zz(eNE), the zigzag residuals already coded
    /// for those neighbours in the same plane (zz ~ 2|e|). Same bucket edges as `Act`.
    Err,
    /// `Err` x the same 4 previous-plane residual buckets as `ActX`.
    ErrX,
    /// Milestone 5: 8 activity buckets x 3 error-energy levels (`ERR_CUTS`) x 4 previous-plane buckets.
    ActErrX,
    /// Milestone 5 control: like `ActErrX` but the 3-way split is (x+y)%3, which carries no information.
    /// Measures pure model-dilution cost of tripling the contexts.
    DummyX,
}
impl Ctx {
    fn count(self) -> usize {
        match self {
            Ctx::None => 1,
            Ctx::Act => 8,
            Ctx::ActX => 32,
            Ctx::Err => 8,
            Ctx::ErrX => 32,
            Ctx::ActErrX | Ctx::DummyX => 96,
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Config {
    pub pred: Pred,
    pub color: Color,
    pub ctx: Ctx,
}
impl Config {
    pub fn label(&self) -> String {
        format!("{:?}/{:?}/{:?}", self.pred, self.color, self.ctx)
    }
}

// ---------- 8-bit planes ----------

/// Planes in coding order. RGB(A) -> [G,R,B,(A)]; gray(+alpha) -> [Y,(A)].
pub struct Planes {
    pub w: usize,
    pub h: usize,
    pub p: Vec<Vec<u8>>,
}

pub fn supported(spec: &Spec) -> bool {
    spec.bit_depth == 8
}

fn order(layout: Layout) -> Vec<usize> {
    match layout {
        Layout::Rgb => vec![1, 0, 2],
        Layout::Rgba => vec![1, 0, 2, 3],
        l => (0..l.channels()).collect(),
    }
}

pub fn to_planes(img: &Canonical) -> Planes {
    let (w, h, ch) = (img.spec.width as usize, img.spec.height as usize, img.spec.layout.channels());
    let p = order(img.spec.layout).iter().map(|&c| (0..w * h).map(|i| img.data[i * ch + c]).collect()).collect();
    Planes { w, h, p }
}

/// Center crop (exact pixel values, no resampling). `max == 0` disables.
pub fn center_crop(img: &Canonical, max: u32) -> Canonical {
    let (w, h) = (img.spec.width, img.spec.height);
    if max == 0 || (w <= max && h <= max) {
        return img.clone();
    }
    let (cw, ch) = (w.min(max), h.min(max));
    let (x0, y0) = ((w - cw) / 2, (h - ch) / 2);
    let row = img.spec.layout.channels() * img.spec.bytes_per_sample();
    let mut data = Vec::with_capacity(cw as usize * ch as usize * row);
    for y in y0..y0 + ch {
        let s = (y as usize * w as usize + x0 as usize) * row;
        data.extend_from_slice(&img.data[s..s + cw as usize * row]);
    }
    Canonical::new(Spec { width: cw, height: ch, ..img.spec }, data).unwrap()
}

pub fn color_fwd(pl: &mut Planes, c: Color) {
    if c == Color::SubG && pl.p.len() >= 3 {
        for i in 0..pl.w * pl.h {
            let g = pl.p[0][i];
            pl.p[1][i] = pl.p[1][i].wrapping_sub(g).wrapping_add(128);
            pl.p[2][i] = pl.p[2][i].wrapping_sub(g).wrapping_add(128);
        }
    }
}
pub fn color_inv(pl: &mut Planes, c: Color) {
    if c == Color::SubG && pl.p.len() >= 3 {
        for i in 0..pl.w * pl.h {
            let g = pl.p[0][i];
            pl.p[1][i] = pl.p[1][i].wrapping_sub(128).wrapping_add(g);
            pl.p[2][i] = pl.p[2][i].wrapping_sub(128).wrapping_add(g);
        }
    }
}

// ---------- prediction ----------

#[derive(Clone, Copy)]
pub struct Nb {
    pub w: i32,
    pub n: i32,
    pub nw: i32,
    pub ne: i32,
}

#[inline]
pub fn neighbors(p: &[u8], width: usize, x: usize, y: usize) -> Nb {
    let i = y * width + x;
    let w = if x > 0 { p[i - 1] as i32 } else if y > 0 { p[i - width] as i32 } else { 0 };
    let n = if y > 0 { p[i - width] as i32 } else { w };
    let nw = if x > 0 && y > 0 { p[i - width - 1] as i32 } else { n };
    let ne = if y > 0 && x + 1 < width { p[i - width + 1] as i32 } else { n };
    Nb { w, n, nw, ne }
}

#[inline]
pub fn predict(pred: Pred, b: Nb) -> i32 {
    match pred {
        Pred::W => b.w,
        Pred::N => b.n,
        Pred::Avg => (b.w + b.n) >> 1,
        Pred::Grad => (b.w + b.n - b.nw).clamp(0, 255),
        Pred::Gap => unreachable!("Gap needs extended neighbours: use gap()"),
        Pred::Med => {
            let (mx, mn) = (b.w.max(b.n), b.w.min(b.n));
            if b.nw >= mx {
                mn
            } else if b.nw <= mn {
                mx
            } else {
                b.w + b.n - b.nw
            }
        }
    }
}

/// GAP (Wu & Memon, CALIC) in integer arithmetic. Uses W,N,NW,NE plus WW,NN,NNE.
///   dh = |W-WW|+|N-NW|+|N-NE|   dv = |W-NW|+|N-NN|+|NE-NNE|
///   dv-dh > 80 -> W (sharp horizontal edge); < -80 -> N (sharp vertical edge)
///   else base = (W+N)/2 + (NE-NW)/4, pulled toward W (or N) by 1/2, 3/4 when |dv-dh| > 32, > 8.
/// Computed at 4x scale (p4) so no precision is lost before the final rounding.
/// Versus MED: MED picks min/max/plane from 3 pixels; GAP weighs 7 pixels by local gradient energy.
/// Extra cost vs MED: 3 more neighbour loads + ~12 integer ops per sample; no state.
#[inline]
pub fn gap(p: &[u8], width: usize, x: usize, y: usize, b: Nb) -> i32 {
    let i = y * width + x;
    let ww = if x > 1 { p[i - 2] as i32 } else { b.w };
    let nn = if y > 1 { p[i - 2 * width] as i32 } else { b.n };
    let nne = if y > 1 && x + 1 < width { p[i - 2 * width + 1] as i32 } else { b.ne };
    let dh = (b.w - ww).abs() + (b.n - b.nw).abs() + (b.n - b.ne).abs();
    let dv = (b.w - b.nw).abs() + (b.n - nn).abs() + (b.ne - nne).abs();
    let d = dv - dh;
    if d > 80 {
        return b.w;
    }
    if d < -80 {
        return b.n;
    }
    let mut p4 = 2 * (b.w + b.n) + (b.ne - b.nw);
    if d > 32 {
        p4 = (p4 + 4 * b.w) >> 1;
    } else if d > 8 {
        p4 = (3 * p4 + 4 * b.w) >> 2;
    } else if d < -32 {
        p4 = (p4 + 4 * b.n) >> 1;
    } else if d < -8 {
        p4 = (3 * p4 + 4 * b.n) >> 2;
    }
    ((p4 + 2) >> 2).clamp(0, 255)
}

/// |a-b| read as a wrapped signed byte, so chroma planes that wrap around 0/255 stay sane.
#[inline]
fn wdiff(a: i32, b: i32) -> i32 {
    ((a - b) as u8 as i8 as i32).abs()
}

#[inline]
pub fn activity(b: Nb) -> i32 {
    wdiff(b.w, b.nw) + wdiff(b.n, b.nw) + wdiff(b.ne, b.n)
}

#[inline]
pub fn act_bucket(a: i32) -> usize {
    match a {
        0 => 0,
        1..=2 => 1,
        3..=5 => 2,
        6..=10 => 3,
        11..=20 => 4,
        21..=40 => 5,
        41..=80 => 6,
        _ => 7,
    }
}

#[inline]
pub fn zigzag(r: u8) -> u8 {
    let s = r as i8;
    ((s << 1) ^ (s >> 7)) as u8
}
#[inline]
pub fn unzigzag(z: u8) -> u8 {
    ((z >> 1) as i8 ^ -((z & 1) as i8)) as u8
}

// ---------- symbol coding backends ----------

pub trait SymIo {
    /// Encoders take `sym` and return it; decoders ignore it and return the decoded symbol.
    fn code(&mut self, model: usize, sym: u8) -> u8;
}

const INC: u32 = 24;
const LIMIT: u32 = 1 << 15;

#[derive(Clone)]
struct Model {
    f: [u16; 256],
    total: u32,
}
impl Model {
    fn new() -> Self {
        Model { f: [1; 256], total: 256 }
    }
    fn update(&mut self, s: u8) {
        self.f[s as usize] += INC as u16;
        self.total += INC;
        if self.total > LIMIT {
            self.total = 0;
            for v in self.f.iter_mut() {
                *v = (*v + 1) >> 1;
                self.total += *v as u32;
            }
        }
    }
}

pub struct Encoder {
    low: u64,
    range: u32,
    cache: u8,
    cache_size: u64,
    out: Vec<u8>,
    models: Vec<Model>,
    /// per-model symbol histograms, for entropy estimates (not part of the bitstream)
    hist: Vec<[u32; 256]>,
}
impl Encoder {
    pub fn new(nmodels: usize) -> Self {
        Encoder {
            low: 0,
            range: u32::MAX,
            cache: 0,
            cache_size: 1,
            out: vec![],
            models: vec![Model::new(); nmodels],
            hist: vec![[0; 256]; nmodels],
        }
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
    /// The first byte of an LZMA-style range coder stream is always 0; it is dropped.
    pub fn finish(mut self) -> (Vec<u8>, Vec<[u32; 256]>) {
        for _ in 0..5 {
            self.shift_low();
        }
        self.out.remove(0);
        (self.out, self.hist)
    }
}
impl SymIo for Encoder {
    fn code(&mut self, mi: usize, sym: u8) -> u8 {
        self.hist[mi][sym as usize] += 1;
        let m = &mut self.models[mi];
        let cum: u32 = m.f[..sym as usize].iter().map(|&v| v as u32).sum();
        let r = self.range / m.total;
        self.low += (cum * r) as u64;
        self.range = r * m.f[sym as usize] as u32;
        m.update(sym);
        while self.range < (1 << 24) {
            self.range <<= 8;
            self.shift_low();
        }
        sym
    }
}

pub struct Decoder<'a> {
    code: u32,
    range: u32,
    buf: &'a [u8],
    pos: usize,
    models: Vec<Model>,
}
impl<'a> Decoder<'a> {
    pub fn new(buf: &'a [u8], nmodels: usize) -> Self {
        let mut d = Decoder { code: 0, range: u32::MAX, buf, pos: 0, models: vec![Model::new(); nmodels] };
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
impl SymIo for Decoder<'_> {
    fn code(&mut self, mi: usize, _sym: u8) -> u8 {
        let m = &mut self.models[mi];
        let r = self.range / m.total;
        let v = (self.code / r).min(m.total - 1);
        let (mut cum, mut s) = (0u32, 0usize);
        while cum + m.f[s] as u32 <= v {
            cum += m.f[s] as u32;
            s += 1;
        }
        self.code -= cum * r;
        self.range = r * m.f[s] as u32;
        m.update(s as u8);
        while self.range < (1 << 24) {
            self.code = (self.code << 8) | self.next() as u32;
            self.range <<= 8;
        }
        s as u8
    }
}

// ---------- the shared encode/decode loop ----------

pub fn nmodels(planes: usize, ctx: Ctx) -> usize {
    planes * ctx.count()
}

/// Encode: `pl` holds the (colour-transformed) pixels. Decode: `pl` is zeroed and gets filled.
/// Same code path for both, so a config that decodes exactly is exactly what was measured.
/// Returns, for `Color::Adaptive`, how many R/B pixels used the Coupled prediction (else 0).
pub fn run(pl: &mut Planes, cfg: Config, io: &mut dyn SymIo) -> u64 {
    if let Color::Adaptive(win) = cfg.color {
        return run_adaptive(pl, cfg, win, io);
    }
    let (w, h, np) = (pl.w, pl.h, pl.p.len());
    let mut gerr = vec![0i16; w * h]; // plane 0: actual - predicted, for Coupled
    let mut prev_z = vec![0u8; w * h]; // previous plane's zigzag residuals, for ActX
    let mut cur_z = vec![0u8; w * h];
    for k in 0..np {
        for y in 0..h {
            for x in 0..w {
                let i = y * w + x;
                let nb = neighbors(&pl.p[k], w, x, y);
                let mut pred = if cfg.pred == Pred::Gap { gap(&pl.p[k], w, x, y, nb) } else { predict(cfg.pred, nb) };
                if cfg.color == Color::Coupled && (k == 1 || k == 2) {
                    pred = (pred + gerr[i] as i32).clamp(0, 255);
                }
                let c = match cfg.ctx {
                    Ctx::None => 0,
                    Ctx::Act => act_bucket(activity(nb)),
                    Ctx::Err => act_bucket(err_energy(&cur_z, w, x, y)),
                    Ctx::ActErrX => {
                        (act_bucket(activity(nb)) * 3 + err_level(err_energy(&cur_z, w, x, y))) * 4 + prev_q(prev_z[i])
                    }
                    Ctx::DummyX => (act_bucket(activity(nb)) * 3 + (x + y) % 3) * 4 + prev_q(prev_z[i]),
                    Ctx::ErrX => act_bucket(err_energy(&cur_z, w, x, y)) * 4 + prev_q(prev_z[i]),
                    Ctx::ActX => {
                        let q = match prev_z[i] {
                            0 => 0,
                            1..=2 => 1,
                            3..=8 => 2,
                            _ => 3,
                        };
                        act_bucket(activity(nb)) * 4 + q
                    }
                };
                let z = zigzag((pl.p[k][i] as i32 - pred) as u8);
                let z = io.code(k * cfg.ctx.count() + c, z);
                pl.p[k][i] = (pred as u8).wrapping_add(unzigzag(z));
                cur_z[i] = z;
                if k == 0 {
                    gerr[i] = pl.p[0][i] as i16 - pred as i16;
                }
            }
        }
        std::mem::swap(&mut prev_z, &mut cur_z);
    }
    0
}

/// Cut points for the 3 error-energy levels (E in zigzag units): level 0 if E <= CUTS[0], 1 if <= CUTS[1],
/// else 2. Chosen as the terciles of E pooled over all planes of the 20 Milestone-1 camera JPEGs
/// (MED + SubG), measured by `probe_m5_cuts` BEFORE the benchmark. Same in encoder and decoder.
pub const ERR_CUTS: [i32; 2] = [0, 8]; // measured by probe_m5_cuts: pooled terciles of E, 20 JPEGs

#[inline]
pub fn err_level(e: i32) -> usize {
    (e > ERR_CUTS[0]) as usize + (e > ERR_CUTS[1]) as usize
}

/// Histogram of E (index = E, 0..=1020) for MED residuals in the SubG domain, all planes pooled.
pub fn energy_histogram(img: &Canonical) -> Vec<u64> {
    let mut pl = to_planes(img);
    color_fwd(&mut pl, Color::SubG);
    let (w, h) = (pl.w, pl.h);
    let mut hist = vec![0u64; 1021];
    for k in 0..pl.p.len() {
        let mut z = vec![0u8; w * h];
        for y in 0..h {
            for x in 0..w {
                let i = y * w + x;
                let pred = predict(Pred::Med, neighbors(&pl.p[k], w, x, y));
                z[i] = zigzag((pl.p[k][i] as i32 - pred) as u8);
                hist[err_energy(&z, w, x, y) as usize] += 1;
            }
        }
    }
    hist
}

/// Sum of already-coded zigzag residuals at W, N, NW, NE (same plane); outside the image counts 0.
#[inline]
pub fn err_energy(z: &[u8], w: usize, x: usize, y: usize) -> i32 {
    let i = y * w + x;
    let mut e = 0i32;
    if x > 0 {
        e += z[i - 1] as i32;
    }
    if y > 0 {
        e += z[i - w] as i32;
        if x > 0 {
            e += z[i - w - 1] as i32;
        }
        if x + 1 < w {
            e += z[i - w + 1] as i32;
        }
    }
    e
}

pub fn prev_q(z: u8) -> usize {
    match z {
        0 => 0,
        1..=2 => 1,
        3..=8 => 2,
        _ => 3,
    }
}

/// Offsets (dx, dy) of the already-decoded neighbours whose predictor errors drive the choice.
fn window(win: u8) -> &'static [(isize, isize)] {
    match win {
        2 => &[(-1, 0), (0, -1)],
        4 => &[(-1, 0), (0, -1), (-1, -1), (1, -1)],
        _ => &[(-1, 0), (0, -1), (-1, -1), (1, -1), (-2, 0), (0, -2)],
    }
}

/// Same contexts and entropy coder as the baseline `SubG/ActX` path. Plane 0 and any plane >= 3 are
/// coded exactly as in `run`. For planes 1 and 2 both candidate predictions are formed:
///   A (SubG):    G + MED(R-G+128 neighbours) - 128   -- identical residual to the SubG transform
///   B (Coupled): MED(R neighbours) + (G - MED(G))
/// and the one with the smaller summed past error over `window(win)` is used (ties -> A).
fn run_adaptive(pl: &mut Planes, cfg: Config, win: u8, io: &mut dyn SymIo) -> u64 {
    let (w, h, np) = (pl.w, pl.h, pl.p.len());
    let offs = window(win);
    let mut gerr = vec![0i16; w * h];
    let mut prev_z = vec![0u8; w * h];
    let mut cur_z = vec![0u8; w * h];
    let (mut d, mut ea, mut eb) = (vec![0u8; w * h], vec![0u8; w * h], vec![0u8; w * h]);
    let mut used_b = 0u64;
    let nc = cfg.ctx.count();
    for k in 0..np {
        let chroma = k == 1 || k == 2;
        for y in 0..h {
            for x in 0..w {
                let i = y * w + x;
                let (pred, ctx_nb, alt) = if chroma {
                    let g = pl.p[0][i];
                    let nbd = neighbors(&d, w, x, y);
                    let pred_a = (predict(Pred::Med, nbd) as u8).wrapping_sub(128).wrapping_add(g) as i32;
                    let pred_b = (predict(Pred::Med, neighbors(&pl.p[k], w, x, y)) + gerr[i] as i32).clamp(0, 255);
                    let (mut sa, mut sb) = (0i32, 0i32);
                    for &(dx, dy) in offs {
                        let (nx, ny) = (x as isize + dx, y as isize + dy);
                        if nx >= 0 && ny >= 0 && (nx as usize) < w {
                            let j = ny as usize * w + nx as usize;
                            sa += ea[j] as i32;
                            sb += eb[j] as i32;
                        }
                    }
                    if sb < sa {
                        (pred_b, nbd, Some((pred_a, pred_b)))
                    } else {
                        (pred_a, nbd, Some((pred_a, pred_b)))
                    }
                } else {
                    let nb = neighbors(&pl.p[k], w, x, y);
                    (predict(Pred::Med, nb), nb, None)
                };
                let c = act_bucket(activity(ctx_nb)) * 4 + prev_q(prev_z[i]);
                let z = zigzag((pl.p[k][i] as i32 - pred) as u8);
                let z = io.code(k * nc + c, z);
                let v = (pred as u8).wrapping_add(unzigzag(z));
                pl.p[k][i] = v;
                cur_z[i] = z;
                if k == 0 {
                    gerr[i] = v as i16 - pred as i16;
                }
                if let Some((pa, pb)) = alt {
                    used_b += (pred == pb && pb != pa) as u64;
                    let g = pl.p[0][i];
                    d[i] = v.wrapping_sub(g).wrapping_add(128);
                    ea[i] = zigzag(v.wrapping_sub(pa as u8));
                    eb[i] = zigzag(v.wrapping_sub(pb as u8));
                }
            }
        }
        std::mem::swap(&mut prev_z, &mut cur_z);
    }
    used_b
}

pub const HEADER_BYTES: usize = 11; // w,h u32 + layout + depth + config byte

pub struct Outcome {
    pub coded_bytes: usize, // stream + header
    pub h0_bytes: f64,      // order-0 entropy per plane (static, no table cost)
    pub hctx_bytes: f64,    // context-conditional entropy (static, no table cost): optimistic
    pub zero_frac: f64,
    pub small_frac: f64, // |residual| <= 2
    pub enc_ms: f64,
    pub dec_ms: f64,
    pub exact: bool,
    /// Adaptive only: share of R/B pixels that used the Coupled prediction.
    pub coupled_frac: f64,
}

fn entropy_bits(h: &[u32; 256]) -> f64 {
    let n: u64 = h.iter().map(|&v| v as u64).sum();
    h.iter().filter(|&&v| v > 0).map(|&v| -(v as f64) * (v as f64 / n as f64).log2()).sum()
}

pub fn measure(img: &Canonical, cfg: Config) -> Outcome {
    let orig = to_planes(img);
    let np = orig.p.len();
    let mut work = Planes { w: orig.w, h: orig.h, p: orig.p.clone() };
    color_fwd(&mut work, cfg.color);
    let mut enc = Encoder::new(nmodels(np, cfg.ctx));
    let t = std::time::Instant::now();
    let used_b = run(&mut work, cfg, &mut enc);
    let (stream, hist) = enc.finish();
    let enc_ms = t.elapsed().as_secs_f64() * 1e3;

    let mut dec_pl = Planes { w: orig.w, h: orig.h, p: vec![vec![0; orig.w * orig.h]; np] };
    let t = std::time::Instant::now();
    run(&mut dec_pl, cfg, &mut Decoder::new(&stream, nmodels(np, cfg.ctx)));
    let dec_ms = t.elapsed().as_secs_f64() * 1e3;
    color_inv(&mut dec_pl, cfg.color);
    let exact = dec_pl.p == orig.p;

    let nc = cfg.ctx.count();
    let (mut h0, mut hc, mut zero, mut small, mut total) = (0.0, 0.0, 0u64, 0u64, 0u64);
    for k in 0..np {
        let mut merged = [0u32; 256];
        for c in 0..nc {
            let hh = &hist[k * nc + c];
            hc += entropy_bits(hh);
            for s in 0..256 {
                merged[s] += hh[s];
            }
        }
        h0 += entropy_bits(&merged);
        zero += merged[0] as u64;
        small += merged[..5].iter().map(|&v| v as u64).sum::<u64>();
        total += merged.iter().map(|&v| v as u64).sum::<u64>();
    }
    Outcome {
        coded_bytes: stream.len() + HEADER_BYTES,
        h0_bytes: h0 / 8.0,
        hctx_bytes: hc / 8.0,
        zero_frac: zero as f64 / total as f64,
        small_frac: small as f64 / total as f64,
        enc_ms,
        dec_ms,
        exact,
        coupled_frac: used_b as f64 / (2 * orig.w * orig.h) as f64,
    }
}
