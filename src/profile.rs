//! Research-only bit-cost profiler for the PKON2 relational coder (`pkonrel`). Removable: delete this file, its `lib.rs` line, `experiments/bin/bitprof.rs`
//! and the `OBSERVE`/`observe`/`observe_tail`/`bit_cost` hooks in `pkonrel.rs`; nothing else depends on it.
//!
//! `ProfEnc` is a range-coder sink (same `BinEnc` as `CeEnc`) that, through `RelIo::observe`, sees every probability the coder is actually given
//! (final mixed probability, and each mixer input's own probability and count, all BEFORE the state update) and accumulates, per decision,
//! the ideal cost `-log2 P(actual bit)` using the same 65537-entry table as `CeEnc`. It never alters a probability, a bit, or an update.
//!
//! Estimators (all research estimates, none is a physical bit count of the range-coded stream):
//! * model cost: sum of ideal costs of the probabilities the coder was given (exact w.r.t. the table; not the physical stream bits).
//! * solo cost of an input: ideal cost of that input's OWN stored probability for the same decision (what that family would cost alone).
//! * static empirical entropy H_s of a (plane, node, slot) partition: for every context value with counts (n0, n1), n * H2(n1/n), summed.
//!   Two-pass, fits the image itself, ignores the cost of describing the parameters: optimistic.
//! * KT cost: sequential Krichevsky-Trofimov code length of the same partition (cost of a plain adaptive count per context, includes learning
//!   cost, so it is a fair "simple adaptive estimator on one family" reference). Computed per image (the codec adapts per image), then summed.

use crate::pkonrel::{bit_cost, RelIo, NODES};
use crate::pkon2x::{BinEnc, BitIo};

pub const NP: usize = 3;
pub const NSLOT: usize = 6;
pub const CAP: usize = 96;
pub const NB: usize = 10;
pub const NCATP: usize = 6;
pub const CATP_NAMES: [&str; NCATP] = ["zero (node 0)", "ge2 (node 1)", "class (nodes 2-7)", "mantissa, modeled (nodes 8-25)", "sign (node 26)", "tail (context-free)"];
pub const BUCKET_NAMES: [&str; NB] = ["0", "1", "2-3", "4-7", "8-15", "16-31", "32-63", "64-127", "128-254", "255 (limit)"];

pub fn node_cat(node: usize) -> usize {
    match node {
        0 => 0,
        1 => 1,
        2..=7 => 2,
        26 => 4,
        _ => 3,
    }
}
fn bucket(n: u8) -> usize {
    match n {
        0 => 0,
        1 => 1,
        2..=3 => 2,
        4..=7 => 3,
        8..=15 => 4,
        16..=31 => 5,
        32..=63 => 6,
        64..=127 => 7,
        128..=254 => 8,
        _ => 9,
    }
}

#[derive(Clone, Copy, Default)]
pub struct Cell {
    pub n: [u32; 2],
    pub mix: f64,
    pub solo: f64,
}
#[derive(Clone, Copy, Default)]
pub struct TailCell {
    pub bits: f64,
    pub n: u64,
    pub n1: u64,
}
#[derive(Clone, Copy, Default)]
pub struct StCell {
    pub bits: f64,
    pub n: u64,
}

pub struct ProfEnc {
    inner: BinEnc,
    /// sum of costs over every `code` call (independent of the observation hooks)
    pub code_bits: f64,
    pub code_n: u64,
    /// sum of costs over every observed decision (mixed + tail)
    pub obs_bits: f64,
    pub obs_n: u64,
    pub node_bits: [[f64; NODES]; NP],
    pub node_n: [[u64; NODES]; NP],
    pub node_ones: [[u64; NODES]; NP],
    pub tail: Vec<TailCell>, // [(plane*9 + class)*8 + pos]
    pub cells: Vec<Cell>,    // [((plane*NODES + node)*NSLOT + slot)*CAP + ctx]
    pub st: [[[StCell; NB]; NCATP]; NP],
    pub dropped: u64,
    pub ext: Ext,
}
impl ProfEnc {
    pub fn new() -> Self {
        ProfEnc {
            inner: BinEnc::new(),
            code_bits: 0.0,
            code_n: 0,
            obs_bits: 0.0,
            obs_n: 0,
            node_bits: [[0.0; NODES]; NP],
            node_n: [[0; NODES]; NP],
            node_ones: [[0; NODES]; NP],
            tail: vec![TailCell::default(); NP * 9 * 8],
            cells: vec![Cell::default(); NP * NODES * NSLOT * CAP],
            st: [[[StCell::default(); NB]; NCATP]; NP],
            dropped: 0,
            ext: Ext::new(),
        }
    }
    pub fn finish(self) -> (Vec<u8>, ProfData) {
        let stream = self.inner.finish();
        (
            stream,
            ProfData {
                code_bits: self.code_bits,
                code_n: self.code_n,
                obs_bits: self.obs_bits,
                obs_n: self.obs_n,
                node_bits: self.node_bits,
                node_n: self.node_n,
                node_ones: self.node_ones,
                tail: self.tail,
                cells: self.cells,
                st: self.st,
                dropped: self.dropped,
                ext: self.ext,
            },
        )
    }
    /// bytes of profiler state (per image; the aggregate is about 1.5x this)
    pub fn state_bytes() -> usize {
        NP * NODES * NSLOT * CAP * std::mem::size_of::<Cell>() + NP * 9 * 8 * std::mem::size_of::<TailCell>() + NP * NCATP * NB * std::mem::size_of::<StCell>() + NP * NODES * 24 + Ext::state_bytes()
    }
}
impl Default for ProfEnc {
    fn default() -> Self {
        Self::new()
    }
}
pub struct ProfData {
    pub code_bits: f64,
    pub code_n: u64,
    pub obs_bits: f64,
    pub obs_n: u64,
    pub node_bits: [[f64; NODES]; NP],
    pub node_n: [[u64; NODES]; NP],
    pub node_ones: [[u64; NODES]; NP],
    pub tail: Vec<TailCell>,
    pub cells: Vec<Cell>,
    pub st: [[[StCell; NB]; NCATP]; NP],
    pub dropped: u64,
    pub ext: Ext,
}
impl ProfData {
    /// (bits, decisions) per plane per category (6 categories, the last one is the context-free tail)
    pub fn cat(&self, plane: usize) -> ([f64; NCATP], [u64; NCATP]) {
        let (mut b, mut n) = ([0.0; NCATP], [0u64; NCATP]);
        for node in 0..NODES {
            let c = node_cat(node);
            b[c] += self.node_bits[plane][node];
            n[c] += self.node_n[plane][node];
        }
        for k in 0..72 {
            let t = &self.tail[plane * 72 + k];
            b[5] += t.bits;
            n[5] += t.n;
        }
        (b, n)
    }
}

impl BitIo for ProfEnc {
    #[inline]
    fn code(&mut self, p1: u32, bit: u32) -> u32 {
        self.code_bits += bit_cost(p1, bit) as f64;
        self.code_n += 1;
        self.inner.code(p1, bit)
    }
}
impl RelIo for ProfEnc {
    const OBSERVE: bool = true;
    fn observe(&mut self, plane: usize, node: usize, slots: &[usize], ctxs: &[usize], inputs: &[(u16, u8)], p: u32, bit: u32) {
        let c = bit_cost(p, bit) as f64;
        self.obs_bits += c;
        self.obs_n += 1;
        self.node_bits[plane][node] += c;
        self.node_n[plane][node] += 1;
        self.node_ones[plane][node] += bit as u64;
        let base = (plane * NODES + node) * NSLOT;
        let mut minn = 255u8;
        for i in 0..slots.len() {
            if slots[i] < 4 {
                minn = minn.min(inputs[i].1);
            }
            if slots[i] >= NSLOT || ctxs[i] >= CAP {
                self.dropped += 1;
                continue;
            }
            let cell = &mut self.cells[(base + slots[i]) * CAP + ctxs[i]];
            cell.n[bit as usize] += 1;
            cell.mix += c;
            cell.solo += bit_cost(inputs[i].0 as u32, bit) as f64;
        }
        let s = &mut self.st[plane][node_cat(node)][bucket(minn)];
        s.bits += c;
        s.n += 1;
        self.ext.observe(plane, node, slots, ctxs, p, bit, c);
    }
    fn observe_tail(&mut self, plane: usize, class: usize, pos: usize, p: u32, n: u8, bit: u32) {
        let c = bit_cost(p, bit) as f64;
        self.obs_bits += c;
        self.obs_n += 1;
        let t = &mut self.tail[(plane * 9 + class) * 8 + pos];
        t.bits += c;
        t.n += 1;
        t.n1 += bit as u64;
        let s = &mut self.st[plane][5][bucket(n)];
        s.bits += c;
        s.n += 1;
    }
}

// ---------------------------------------------------------------- extra references: calibration of the mixed probability, joint contexts

pub const PAIRS: [(usize, usize); 10] = [(0, 1), (0, 2), (0, 3), (0, 4), (1, 2), (1, 3), (1, 4), (2, 3), (2, 4), (3, 4)];
pub const PAIR_NODES: usize = 2; // nodes 0 and 1
pub const PAIR_CELLS: usize = CAP * CAP;
pub const SIGN3: usize = 12 * 27 * 27;
pub const SIGN4: usize = SIGN3 * 49;
pub const NCAL1: usize = 32;
pub const NCAL2: usize = 16 * 8;

/// bucket of a 16-bit probability on the logit scale (stretch domain, 256 per natural-log unit, clamped to +-2047), `nb` buckets
fn pbucket(p: u32, nb: usize) -> usize {
    let d = ((p as f64 / (65536.0 - p as f64)).ln() * 256.0).clamp(-2047.0, 2047.0);
    ((((d + 2048.0) * nb as f64) / 4096.0) as usize).min(nb - 1)
}
fn pb_table(nb: usize) -> Vec<u8> {
    (0..4096u32).map(|i| pbucket((i << 4) + 8, nb) as u8).collect()
}

#[derive(Clone, Copy, Default)]
pub struct CalCell {
    pub n: [u64; 2],
    pub bits: f64,
    pub psum: f64, // sum of the model's P(1), to compare mean predicted vs empirical frequency
}

/// Extra references, all observed from the same decisions:
/// * `cal1`: per (plane, node), the decisions partitioned by the model's own final probability (32 logit buckets). The reference cost of
///   an ideally recalibrated output (what an SSE/APM stage could at best recover from that partition) is its H / KT.
/// * `cal2`: same with 16 probability buckets x the activity bucket of family A (non-sign nodes only).
/// * `pair`: for nodes 0 and 1 (non-sign), the joint context of every pair of families among A,B,C,D,P.
/// * `sign3`: sign node, joint (S0, S1, S2); `sign4`: joint (S0, S1, S2, P).
pub struct Ext {
    pub cal1: Vec<CalCell>,
    pub cal2: Vec<CalCell>,
    pub pair: Vec<[u32; 2]>,
    pub sign3: Vec<[u32; 2]>,
    pub sign4: Vec<[u32; 2]>,
    pb32: Vec<u8>,
    pb16: Vec<u8>,
}
impl Ext {
    pub fn new() -> Self {
        Ext {
            cal1: vec![CalCell::default(); NP * NODES * NCAL1],
            cal2: vec![CalCell::default(); NP * NODES * NCAL2],
            pair: vec![[0; 2]; NP * PAIR_NODES * PAIRS.len() * PAIR_CELLS],
            sign3: vec![[0; 2]; NP * SIGN3],
            sign4: vec![[0; 2]; NP * SIGN4],
            pb32: pb_table(NCAL1),
            pb16: pb_table(16),
        }
    }
    #[inline]
    fn observe(&mut self, plane: usize, node: usize, slots: &[usize], ctxs: &[usize], p: u32, bit: u32, c: f64) {
        let mut cx = [usize::MAX; NSLOT];
        for i in 0..slots.len() {
            if slots[i] < NSLOT {
                cx[slots[i]] = ctxs[i];
            }
        }
        let k = (p >> 4) as usize;
        let cell = &mut self.cal1[(plane * NODES + node) * NCAL1 + self.pb32[k] as usize];
        cell.n[bit as usize] += 1;
        cell.bits += c;
        cell.psum += p as f64 / 65536.0;
        if node != 26 {
            let act = (cx[0] / 12).min(7);
            let cell = &mut self.cal2[(plane * NODES + node) * NCAL2 + self.pb16[k] as usize * 8 + act];
            cell.n[bit as usize] += 1;
            cell.bits += c;
            cell.psum += p as f64 / 65536.0;
            if node < PAIR_NODES {
                for (pi, &(a, b)) in PAIRS.iter().enumerate() {
                    if cx[a] < CAP && cx[b] < CAP {
                        self.pair[(((plane * PAIR_NODES + node) * PAIRS.len() + pi) * CAP + cx[a]) * CAP + cx[b]][bit as usize] += 1;
                    }
                }
            }
        } else if cx[0] < 12 && cx[1] < 27 && cx[2] < 27 {
            let key = (cx[0] * 27 + cx[1]) * 27 + cx[2];
            self.sign3[plane * SIGN3 + key][bit as usize] += 1;
            if cx[4] < 49 {
                self.sign4[plane * SIGN4 + key * 49 + cx[4]][bit as usize] += 1;
            }
        }
    }
}
impl Ext {
    pub fn state_bytes() -> usize {
        (NP * NODES * (NCAL1 + NCAL2)) * std::mem::size_of::<CalCell>() + 8 * (NP * PAIR_NODES * PAIRS.len() * PAIR_CELLS + NP * SIGN3 + NP * SIGN4)
    }
}
impl Default for Ext {
    fn default() -> Self {
        Self::new()
    }
}
#[derive(Clone, Copy, Default)]
pub struct AggRef {
    pub n: u64,
    pub h: f64,
    pub kt: f64,
}
#[derive(Clone, Copy, Default)]
pub struct AggCal {
    pub n: [u64; 2],
    pub bits: f64,
    pub psum: f64,
    pub h: f64,
    pub kt: f64,
}
/// Aggregated extra references.
pub struct AggExt {
    pub cal1: Vec<AggCal>,
    pub cal2: Vec<AggCal>,
    pub pair: Vec<AggRef>, // [(plane*PAIR_NODES + node)*10 + pair]
    pub sign3: Vec<AggRef>,
    pub sign4: Vec<AggRef>,
}
impl AggExt {
    pub fn new() -> Self {
        AggExt { cal1: vec![AggCal::default(); NP * NODES * NCAL1], cal2: vec![AggCal::default(); NP * NODES * NCAL2], pair: vec![AggRef::default(); NP * PAIR_NODES * 10], sign3: vec![AggRef::default(); NP], sign4: vec![AggRef::default(); NP] }
    }
    fn joint(cells: &[[u32; 2]]) -> AggRef {
        let mut r = AggRef::default();
        for c in cells {
            let (n0, n1) = (c[0] as u64, c[1] as u64);
            if n0 + n1 > 0 {
                r.n += n0 + n1;
                r.h += h_static(n0, n1);
                r.kt += kt_bits(n0, n1);
            }
        }
        r
    }
    pub fn add(&mut self, e: &Ext) {
        let cal = |agg: &mut Vec<AggCal>, src: &Vec<CalCell>| {
            for (a, c) in agg.iter_mut().zip(src) {
                if c.n[0] + c.n[1] == 0 {
                    continue;
                }
                a.n[0] += c.n[0];
                a.n[1] += c.n[1];
                a.bits += c.bits;
                a.psum += c.psum;
                a.h += h_static(c.n[0], c.n[1]);
                a.kt += kt_bits(c.n[0], c.n[1]);
            }
        };
        cal(&mut self.cal1, &e.cal1);
        cal(&mut self.cal2, &e.cal2);
        for i in 0..NP * PAIR_NODES * 10 {
            let r = Self::joint(&e.pair[i * PAIR_CELLS..(i + 1) * PAIR_CELLS]);
            self.pair[i].n += r.n;
            self.pair[i].h += r.h;
            self.pair[i].kt += r.kt;
        }
        for p in 0..NP {
            let r = Self::joint(&e.sign3[p * SIGN3..(p + 1) * SIGN3]);
            self.sign3[p].n += r.n;
            self.sign3[p].h += r.h;
            self.sign3[p].kt += r.kt;
            let r = Self::joint(&e.sign4[p * SIGN4..(p + 1) * SIGN4]);
            self.sign4[p].n += r.n;
            self.sign4[p].h += r.h;
            self.sign4[p].kt += r.kt;
        }
    }
}
impl Default for AggExt {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------- reference estimators

fn ln_gamma(x: f64) -> f64 {
    // Lanczos approximation (g = 7, n = 9), accurate to ~1e-13 for x > 0
    const C: [f64; 9] = [0.99999999999980993, 676.5203681218851, -1259.1392167224028, 771.32342877765313, -176.61502916214059, 12.507343278686905, -0.13857109526572012, 9.9843695780195716e-6, 1.5056327351493116e-7];
    if x < 0.5 {
        return (std::f64::consts::PI / (std::f64::consts::PI * x).sin()).ln() - ln_gamma(1.0 - x);
    }
    let x = x - 1.0;
    let mut a = C[0];
    let t = x + 7.5;
    for (i, c) in C.iter().enumerate().skip(1) {
        a += c / (x + i as f64);
    }
    0.5 * (2.0 * std::f64::consts::PI).ln() + (x + 0.5) * t.ln() - t + a.ln()
}
/// static empirical entropy in bits of n0 zeros and n1 ones
pub fn h_static(n0: u64, n1: u64) -> f64 {
    let n = (n0 + n1) as f64;
    let t = |k: u64| if k == 0 { 0.0 } else { -(k as f64) * (k as f64 / n).log2() };
    t(n0) + t(n1)
}
/// sequential Krichevsky-Trofimov code length in bits for n0 zeros and n1 ones: -log2( G(n0+1/2) G(n1+1/2) / (pi G(n+1)) )
pub fn kt_bits(n0: u64, n1: u64) -> f64 {
    let n = (n0 + n1) as f64;
    (ln_gamma(n + 1.0) + std::f64::consts::PI.ln() - ln_gamma(n0 as f64 + 0.5) - ln_gamma(n1 as f64 + 0.5)) / std::f64::consts::LN_2
}

#[derive(Clone, Copy, Default)]
pub struct AggCell {
    pub n0: u64,
    pub n1: u64,
    pub mix: f64,
    pub solo: f64,
    pub h: f64,
    pub kt: f64,
}
#[derive(Clone, Copy, Default)]
pub struct AggTail {
    pub bits: f64,
    pub n: u64,
    pub n1: u64,
    pub h: f64,
    pub kt: f64,
}
#[derive(Clone, Copy, Default)]
pub struct AggNode {
    pub bits: f64,
    pub n: u64,
    pub n1: u64,
    pub h: f64, // unconditional static entropy of the node's bit (per image, summed)
    pub kt: f64,
}
pub struct Agg {
    pub cells: Vec<AggCell>,
    pub nodes: Vec<AggNode>, // [plane*NODES + node]
    pub tail: Vec<AggTail>,  // [(plane*9+class)*8+pos]
    pub st: [[[StCell; NB]; NCATP]; NP],
    pub images: usize,
    pub ext: AggExt,
}
impl Agg {
    pub fn new() -> Self {
        Agg { cells: vec![AggCell::default(); NP * NODES * NSLOT * CAP], nodes: vec![AggNode::default(); NP * NODES], tail: vec![AggTail::default(); NP * 72], st: [[[StCell::default(); NB]; NCATP]; NP], images: 0, ext: AggExt::new() }
    }
    pub fn add(&mut self, d: &ProfData) {
        self.images += 1;
        self.ext.add(&d.ext);
        for (a, c) in self.cells.iter_mut().zip(&d.cells) {
            let (n0, n1) = (c.n[0] as u64, c.n[1] as u64);
            if n0 + n1 == 0 {
                continue;
            }
            a.n0 += n0;
            a.n1 += n1;
            a.mix += c.mix;
            a.solo += c.solo;
            a.h += h_static(n0, n1);
            a.kt += kt_bits(n0, n1);
        }
        for p in 0..NP {
            for node in 0..NODES {
                let (n, ones) = (d.node_n[p][node], d.node_ones[p][node]);
                let a = &mut self.nodes[p * NODES + node];
                a.bits += d.node_bits[p][node];
                a.n += n;
                a.n1 += ones;
                if n > 0 {
                    a.h += h_static(n - ones, ones);
                    a.kt += kt_bits(n - ones, ones);
                }
            }
        }
        for (a, t) in self.tail.iter_mut().zip(&d.tail) {
            a.bits += t.bits;
            a.n += t.n;
            a.n1 += t.n1;
            if t.n > 0 {
                a.h += h_static(t.n - t.n1, t.n1);
                a.kt += kt_bits(t.n - t.n1, t.n1);
            }
        }
        for p in 0..NP {
            for c in 0..NCATP {
                for b in 0..NB {
                    self.st[p][c][b].bits += d.st[p][c][b].bits;
                    self.st[p][c][b].n += d.st[p][c][b].n;
                }
            }
        }
    }
    pub fn state_bytes() -> usize {
        NP * NODES * NSLOT * CAP * std::mem::size_of::<AggCell>() + NP * NODES * std::mem::size_of::<AggNode>() + NP * 72 * std::mem::size_of::<AggTail>() + NP * NCATP * NB * std::mem::size_of::<StCell>() + (NP * NODES * (NCAL1 + NCAL2)) * std::mem::size_of::<AggCal>()
    }
}
impl Default for Agg {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn estimators_sane() {
        assert!((kt_bits(0, 0)).abs() < 1e-9);
        assert!((kt_bits(1, 0) - 1.0).abs() < 1e-9); // P = 1/2
        assert!((kt_bits(2, 0) - (1.0 / 0.5f64 * 0.0 + (8.0f64 / 3.0).log2())).abs() < 1e-9); // P(00) = 1/2 * 3/4 = 3/8
        assert!((h_static(5, 5) - 10.0).abs() < 1e-9);
        assert!(h_static(10, 0).abs() < 1e-9);
        assert!(kt_bits(500, 500) > h_static(500, 500)); // KT pays a learning cost over the static entropy
        assert!(kt_bits(500, 500) - h_static(500, 500) < 6.0);
    }
}
