//! PRAW Stage 1: structural decorrelation for RAW sample planes.
//!
//! Samples are visited in Morton (Z-order) order. Each sample is predicted from already reconstructed samples
//! (a neighbor is usable iff it is inside the image and already reconstructed), and the exact signed residual is
//! zigzag-mapped to an unsigned integer. Encoder and decoder run the identical `State`, whose decisions depend only on
//! reconstructed samples, so no side information is needed. No entropy coding happens here.
//!
//! Prediction per sample:
//!   1. Every pattern whose required neighbor offsets are all reconstructed is eligible (adaptive neighborhood).
//!   2. Each eligible pattern is scored by replaying it on nearby reconstructed samples (the training set) and summing
//!      a fixed-point log2 proxy of the zigzag residual (residual-entropy objective). Lowest cost wins; ties go to the
//!      lowest pattern index. If no pattern is scorable the first eligible one is used; if none is eligible the
//!      predictor is the mid-range value.
//!   3. A bounded per-(pattern, activity-context) bias estimate is added (locally estimated parameter) and the result
//!      is clamped to the sample range.

use crate::pkon2x::{BinDec, BinEnc, Bit, BitIo};
use std::fmt;

pub const MAX_DIM: usize = 1 << 20;
const MAX_SAMPLES: u64 = 1 << 32;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PrawError {
    ZeroDimension,
    DimensionTooLarge,
    UnsupportedBitDepth(u8),
    LengthMismatch { expected: usize, actual: usize },
    SampleOutOfRange { index: usize, value: u32 },
    CorruptResidual { index: usize },
    InvalidCfaDimensions { width: usize, height: usize },
    CfaPatternLengthMismatch { expected: usize, actual: usize },
    InvalidColorFilter(u8),
    CfaOriginOutOfRange { origin_x: usize, origin_y: usize },
    MetadataGeometryMismatch { expected: (usize, usize), actual: (usize, usize) },
    InvalidMetadata(&'static str),
    CorruptStream(&'static str),
}

impl fmt::Display for PrawError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ZeroDimension => write!(f, "width and height must be non-zero"),
            Self::DimensionTooLarge => write!(f, "dimensions exceed supported limits"),
            Self::UnsupportedBitDepth(b) => write!(f, "unsupported bit depth {b} (supported: 1..=16)"),
            Self::LengthMismatch { expected, actual } => write!(f, "expected {expected} values, got {actual}"),
            Self::SampleOutOfRange { index, value } => write!(f, "sample {index} = {value} exceeds bit depth"),
            Self::CorruptResidual { index } => write!(f, "residual {index} reconstructs outside the sample range"),
            Self::InvalidCfaDimensions { width, height } => write!(f, "invalid CFA repeat dimensions {width}x{height}"),
            Self::CfaPatternLengthMismatch { expected, actual } => {
                write!(f, "CFA pattern needs {expected} entries, got {actual}")
            }
            Self::InvalidColorFilter(c) => write!(f, "invalid color-filter identifier {c}"),
            Self::CfaOriginOutOfRange { origin_x, origin_y } => {
                write!(f, "CFA origin ({origin_x}, {origin_y}) lies outside the repeat pattern")
            }
            Self::MetadataGeometryMismatch { expected, actual } => write!(
                f,
                "metadata geometry {}x{} does not match image dimensions {}x{}",
                actual.0, actual.1, expected.0, expected.1
            ),
            Self::InvalidMetadata(why) => write!(f, "invalid image metadata: {why}"),
            Self::CorruptStream(why) => write!(f, "corrupt Stage 4 stream: {why}"),
        }
    }
}

impl std::error::Error for PrawError {}

/// Validates dimensions and bit depth; returns the sample count.
pub fn validate(width: usize, height: usize, bit_depth: u8) -> Result<usize, PrawError> {
    if width == 0 || height == 0 {
        return Err(PrawError::ZeroDimension);
    }
    if width > MAX_DIM || height > MAX_DIM {
        return Err(PrawError::DimensionTooLarge);
    }
    if !(1..=16).contains(&bit_depth) {
        return Err(PrawError::UnsupportedBitDepth(bit_depth));
    }
    let n = (width as u64).checked_mul(height as u64).ok_or(PrawError::DimensionTooLarge)?;
    if n > MAX_SAMPLES {
        return Err(PrawError::DimensionTooLarge);
    }
    usize::try_from(n).map_err(|_| PrawError::DimensionTooLarge)
}

// ---------------------------------------------------------------- 1.1 Morton traversal

/// Calls `f(x, y)` for every pixel of a `width` x `height` image exactly once, in Morton order
/// (x bits at even positions, y bits at odd positions). Equivalent to enumerating the power-of-two square by Morton
/// index and skipping out-of-image coordinates, but quadrants entirely outside the image are pruned.
pub fn morton_visit<E>(width: usize, height: usize, mut f: impl FnMut(usize, usize) -> Result<(), E>) -> Result<(), E> {
    let side = width.max(height).next_power_of_two();
    visit(0, 0, side, width, height, &mut f)
}

fn visit<E>(
    x0: usize,
    y0: usize,
    size: usize,
    w: usize,
    h: usize,
    f: &mut impl FnMut(usize, usize) -> Result<(), E>,
) -> Result<(), E> {
    if x0 >= w || y0 >= h {
        return Ok(());
    }
    if size == 1 {
        return f(x0, y0);
    }
    let half = size / 2;
    visit(x0, y0, half, w, h, f)?;
    visit(x0 + half, y0, half, w, h, f)?;
    visit(x0, y0 + half, half, w, h, f)?;
    visit(x0 + half, y0 + half, half, w, h, f)
}

// ---------------------------------------------------------------- 1.5 Residual mapping

/// Signed -> unsigned zigzag: 0,-1,1,-2,2 -> 0,1,2,3,4. Bijective over all of i32.
#[inline]
pub fn zigzag(r: i32) -> u32 {
    let r = r as i64;
    (if r >= 0 { 2 * r } else { -2 * r - 1 }) as u32
}

/// Exact inverse of [`zigzag`].
#[inline]
pub fn unzigzag(u: u32) -> i32 {
    let u = u as i64;
    (if u & 1 == 0 { u / 2 } else { -(u + 1) / 2 }) as i32
}

// ---------------------------------------------------------------- 1.2 / 1.3 Patterns

type Off = (i32, i32);
const W: Off = (-1, 0);
const WW: Off = (-2, 0);
const N: Off = (0, -1);
const NN: Off = (0, -2);
const NW: Off = (-1, -1);
const NE: Off = (1, -1);
const NNE: Off = (1, -2);
const E: Off = (1, 0);
const S: Off = (0, 1);

struct Pattern {
    needs: &'static [Off],
    eval: fn(&dyn Fn(Off) -> i32) -> i32,
}

fn med3(a: i32, b: i32, c: i32) -> i32 {
    a.max(b).min(a.min(b).max(c))
}

/// Candidate patterns, in fixed priority order (index = tie-break rank and fallback order).
const PATTERNS: &[Pattern] = &[
    // 0: median edge detector (nonlinear)
    Pattern {
        needs: &[W, N, NW],
        eval: |g| {
            let (w, n, nw) = (g(W), g(N), g(NW));
            if nw >= w.max(n) {
                w.min(n)
            } else if nw <= w.min(n) {
                w.max(n)
            } else {
                w + n - nw
            }
        },
    },
    Pattern { needs: &[W], eval: |g| g(W) },
    Pattern { needs: &[N], eval: |g| g(N) },
    // 3: planar gradient
    Pattern { needs: &[W, N, NW], eval: |g| g(W) + g(N) - g(NW) },
    Pattern { needs: &[W, N], eval: |g| (g(W) + g(N) + 1) >> 1 },
    Pattern { needs: &[W, NE], eval: |g| (g(W) + g(NE) + 1) >> 1 },
    // 6, 7: linear extrapolation
    Pattern { needs: &[W, WW], eval: |g| 2 * g(W) - g(WW) },
    Pattern { needs: &[N, NN], eval: |g| 2 * g(N) - g(NN) },
    // 8: gradient-weighted blend of W and N, weights from local gradients, fixed-point rounded division
    Pattern {
        needs: &[W, WW, N, NN, NW, NE, NNE],
        eval: |g| {
            let (w, ww, n, nn, nw, ne, nne) = (g(W), g(WW), g(N), g(NN), g(NW), g(NE), g(NNE));
            let dh = ((w - ww).abs() + (n - nw).abs() + (n - ne).abs()) as i64;
            let dv = ((w - nw).abs() + (n - nn).abs() + (ne - nne).abs()) as i64;
            let s = dh + dv;
            if s == 0 {
                (w + n + 1) >> 1
            } else {
                ((w as i64 * dv + n as i64 * dh + s / 2) / s) as i32
            }
        },
    },
    // 9..11: interpolation from reconstructed samples on the far side (available in Morton order)
    Pattern { needs: &[W, E], eval: |g| (g(W) + g(E) + 1) >> 1 },
    Pattern { needs: &[N, S], eval: |g| (g(N) + g(S) + 1) >> 1 },
    Pattern { needs: &[W, N, E, S], eval: |g| (g(W) + g(N) + g(E) + g(S) + 2) >> 2 },
    // 12: median of three (nonlinear)
    Pattern { needs: &[W, N, NE], eval: |g| med3(g(W), g(N), g(NE)) },
    Pattern { needs: &[NE], eval: |g| g(NE) },
];

/// Training positions (relative to the current sample) on which candidate patterns are replayed.
const TRAIN: &[Off] = &[W, N, NW, NE, WW, NN, E, S, (-1, 1)];

const CTX: usize = 8;
const BIAS_LIMIT: i32 = 32;

/// Fixed-point log2 proxy of v >= 1, 4 fractional bits (linear mantissa interpolation).
#[inline]
fn log2_q4(v: u32) -> u64 {
    let e = 31 - v.leading_zeros();
    let frac = if e >= 4 { (v >> (e - 4)) & 15 } else { (v << (4 - e)) & 15 };
    (e * 16 + frac) as u64
}

struct Prediction {
    value: i32,
    raw: i32,
    sel: Option<(usize, usize)>, // (pattern, context)
}

struct State {
    w: usize,
    h: usize,
    maxv: i32,
    rec: Vec<u16>,
    known: Vec<bool>,
    bias_sum: [[i32; CTX]; PATTERNS.len()],
    bias_cnt: [[i32; CTX]; PATTERNS.len()],
    shift: u32,
}

impl State {
    fn new(w: usize, h: usize, bit_depth: u8, n: usize) -> Self {
        State {
            w,
            h,
            maxv: (1i32 << bit_depth) - 1,
            rec: vec![0; n],
            known: vec![false; n],
            bias_sum: [[0; CTX]; PATTERNS.len()],
            bias_cnt: [[0; CTX]; PATTERNS.len()],
            shift: (bit_depth as u32).saturating_sub(8),
        }
    }

    /// Reconstructed value at (x+dx, y+dy), or None if outside the image or not yet reconstructed.
    #[inline]
    fn fetch(&self, x: usize, y: usize, (dx, dy): Off) -> Option<i32> {
        let nx = x as i64 + dx as i64;
        let ny = y as i64 + dy as i64;
        if nx < 0 || ny < 0 || nx >= self.w as i64 || ny >= self.h as i64 {
            return None;
        }
        let i = ny as usize * self.w + nx as usize;
        self.known[i].then(|| self.rec[i] as i32)
    }

    /// Clamped prediction of pattern `p` at (x, y); None if any required neighbor is unavailable.
    fn predict_at(&self, p: usize, x: usize, y: usize) -> Option<i32> {
        let pat = &PATTERNS[p];
        if pat.needs.iter().any(|&o| self.fetch(x, y, o).is_none()) {
            return None;
        }
        let g = |o: Off| self.fetch(x, y, o).unwrap_or(0);
        Some((pat.eval)(&g).clamp(0, self.maxv))
    }

    /// Residual-entropy proxy of pattern `p` replayed on the training set; None if no training sample is scorable.
    fn score(&self, p: usize, x: usize, y: usize) -> Option<u64> {
        let mut cost = 0u64;
        let mut terms = 0u32;
        for &(dx, dy) in TRAIN {
            let Some(actual) = self.fetch(x, y, (dx, dy)) else { continue };
            let (qx, qy) = ((x as i64 + dx as i64) as usize, (y as i64 + dy as i64) as usize);
            let Some(pred) = self.predict_at(p, qx, qy) else { continue };
            cost += log2_q4(zigzag(actual - pred) + 1);
            terms += 1;
        }
        (terms > 0).then_some(cost)
    }

    /// Local activity context from W, N, NW, NE (0 if any is unavailable).
    fn context(&self, x: usize, y: usize) -> usize {
        match (self.fetch(x, y, W), self.fetch(x, y, N), self.fetch(x, y, NW), self.fetch(x, y, NE)) {
            (Some(w), Some(n), Some(nw), Some(ne)) => {
                let a = ((w - nw).abs() + (n - nw).abs() + (n - ne).abs()) as u32 >> self.shift;
                ((32 - a.leading_zeros()) as usize).min(CTX - 1)
            }
            _ => 0,
        }
    }

    fn predict(&self, x: usize, y: usize) -> Prediction {
        let mut best: Option<(u64, usize)> = None; // scorable winner
        let mut first_eligible: Option<usize> = None;
        for p in 0..PATTERNS.len() {
            if PATTERNS[p].needs.iter().any(|&o| self.fetch(x, y, o).is_none()) {
                continue;
            }
            first_eligible.get_or_insert(p);
            if let Some(c) = self.score(p, x, y) {
                if best.map_or(true, |(bc, _)| c < bc) {
                    best = Some((c, p));
                }
            }
        }
        let Some(p) = best.map(|b| b.1).or(first_eligible) else {
            let mid = 1 << (self.maxv.count_ones() - 1);
            return Prediction { value: mid, raw: mid, sel: None };
        };
        let raw = self.predict_at(p, x, y).unwrap_or(0);
        let ctx = self.context(x, y);
        let (s, c) = (self.bias_sum[p][ctx], self.bias_cnt[p][ctx]);
        let corr = if c > 0 { (s + (c >> 1)).div_euclid(c) } else { 0 };
        Prediction { value: (raw + corr).clamp(0, self.maxv), raw, sel: Some((p, ctx)) }
    }

    fn update(&mut self, x: usize, y: usize, value: i32, pred: &Prediction) {
        let i = y * self.w + x;
        self.rec[i] = value as u16;
        self.known[i] = true;
        if let Some((p, ctx)) = pred.sel {
            self.bias_sum[p][ctx] += value - pred.raw;
            self.bias_cnt[p][ctx] += 1;
            if self.bias_cnt[p][ctx] >= BIAS_LIMIT {
                self.bias_sum[p][ctx] >>= 1;
                self.bias_cnt[p][ctx] >>= 1;
            }
        }
    }
}

// ---------------------------------------------------------------- 1.7 Encode / decode

/// Encodes a row-major `width` x `height` sample plane. Returns the zigzag-mapped residuals in Morton order.
pub fn encode(samples: &[u16], width: usize, height: usize, bit_depth: u8) -> Result<Vec<u32>, PrawError> {
    let n = validate(width, height, bit_depth)?;
    if samples.len() != n {
        return Err(PrawError::LengthMismatch { expected: n, actual: samples.len() });
    }
    let maxv = (1u32 << bit_depth) - 1;
    if let Some((index, &v)) = samples.iter().enumerate().find(|(_, &v)| v as u32 > maxv) {
        return Err(PrawError::SampleOutOfRange { index, value: v as u32 });
    }
    let mut st = State::new(width, height, bit_depth, n);
    let mut out = Vec::with_capacity(n);
    morton_visit(width, height, |x, y| {
        let pred = st.predict(x, y);
        let v = samples[y * width + x] as i32;
        out.push(zigzag(v - pred.value));
        st.update(x, y, v, &pred);
        Ok(())
    })?;
    Ok(out)
}

/// Inverse of [`encode`]: returns the row-major sample plane.
pub fn decode(residuals: &[u32], width: usize, height: usize, bit_depth: u8) -> Result<Vec<u16>, PrawError> {
    let n = validate(width, height, bit_depth)?;
    if residuals.len() != n {
        return Err(PrawError::LengthMismatch { expected: n, actual: residuals.len() });
    }
    let mut st = State::new(width, height, bit_depth, n);
    let mut out = vec![0u16; n];
    let mut k = 0usize;
    morton_visit(width, height, |x, y| {
        let pred = st.predict(x, y);
        let v = pred.value as i64 + unzigzag(residuals[k]) as i64;
        if v < 0 || v > st.maxv as i64 {
            return Err(PrawError::CorruptResidual { index: k });
        }
        st.update(x, y, v as i32, &pred);
        out[y * width + x] = v as u16;
        k += 1;
        Ok(())
    })?;
    Ok(out)
}

// ---------------------------------------------------------------- 1.8 Image metadata handoff (descriptive only)
//
// Metadata describes the ORIGINAL samples. It is never an input to prediction, traversal, or residual mapping; the
// metadata-aware API below only validates it and carries it alongside the unchanged Stage 1 residual stream.

/// How the original samples are organized. Non-exhaustive so further layouts can be added without breaking callers.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SampleLayout {
    Unspecified,
    Monochrome,
    /// Linear / multichannel data (no CFA).
    Linear,
    /// Bayer mosaic: a 2x2 pattern with one red, one blue and two diagonal greens (any arrangement).
    BayerCfa,
    /// Any other CFA mosaic with an arbitrary repeat size.
    OtherCfa,
}

/// Color-filter identity of one CFA site. Discriminants follow the DNG `CFAPattern` codes.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum ColorFilter {
    Red = 0,
    Green = 1,
    Blue = 2,
    Cyan = 3,
    Magenta = 4,
    Yellow = 5,
    White = 6,
}

impl TryFrom<u8> for ColorFilter {
    type Error = PrawError;

    fn try_from(code: u8) -> Result<Self, PrawError> {
        Ok(match code {
            0 => Self::Red,
            1 => Self::Green,
            2 => Self::Blue,
            3 => Self::Cyan,
            4 => Self::Magenta,
            5 => Self::Yellow,
            6 => Self::White,
            c => return Err(PrawError::InvalidColorFilter(c)),
        })
    }
}

/// A repeating CFA pattern. `entries` is row-major (`width` x `height`). The image sample at (0, 0) corresponds to
/// pattern position (`origin_x`, `origin_y`); the pattern is stored exactly as supplied, never normalized.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CfaPattern {
    pub width: usize,
    pub height: usize,
    pub entries: Vec<ColorFilter>,
    pub origin_x: usize,
    pub origin_y: usize,
}

impl CfaPattern {
    /// Color filter at image coordinate (x, y). The pattern must have passed validation.
    pub fn color_at(&self, x: usize, y: usize) -> ColorFilter {
        self.entries[((y + self.origin_y) % self.height) * self.width + (x + self.origin_x) % self.width]
    }

    fn validate(&self) -> Result<(), PrawError> {
        let bad_dims = PrawError::InvalidCfaDimensions { width: self.width, height: self.height };
        if self.width == 0 || self.height == 0 || self.width > MAX_DIM || self.height > MAX_DIM {
            return Err(bad_dims);
        }
        let expected = self.width.checked_mul(self.height).ok_or(bad_dims)?;
        if self.entries.len() != expected {
            return Err(PrawError::CfaPatternLengthMismatch { expected, actual: self.entries.len() });
        }
        if self.origin_x >= self.width || self.origin_y >= self.height {
            return Err(PrawError::CfaOriginOutOfRange { origin_x: self.origin_x, origin_y: self.origin_y });
        }
        Ok(())
    }

    fn validate_bayer(&self) -> Result<(), PrawError> {
        if (self.width, self.height) != (2, 2) {
            return Err(PrawError::InvalidMetadata("Bayer CFA requires a 2x2 repeat"));
        }
        let count = |c| self.entries.iter().filter(|&&e| e == c).count();
        if count(ColorFilter::Red) != 1 || count(ColorFilter::Blue) != 1 || count(ColorFilter::Green) != 2 {
            return Err(PrawError::InvalidMetadata("Bayer CFA requires one red, one blue and two green entries"));
        }
        let g = ColorFilter::Green;
        let diagonal = (self.entries[0] == g && self.entries[3] == g) || (self.entries[1] == g && self.entries[2] == g);
        if !diagonal {
            return Err(PrawError::InvalidMetadata("Bayer CFA greens must lie on a diagonal"));
        }
        Ok(())
    }
}

/// Descriptive information about the original image samples, supplied by the upstream input layer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImageMetadata {
    pub width: usize,
    pub height: usize,
    pub layout: SampleLayout,
    /// Required for `BayerCfa` / `OtherCfa`; must be `None` for all other layouts.
    pub cfa: Option<CfaPattern>,
}

impl ImageMetadata {
    /// Metadata for a non-CFA layout (`Unspecified`, `Monochrome`, `Linear`).
    pub fn without_cfa(width: usize, height: usize, layout: SampleLayout) -> Self {
        ImageMetadata { width, height, layout, cfa: None }
    }

    /// Metadata for a CFA layout; the pattern and phase are taken exactly as given.
    pub fn with_cfa(width: usize, height: usize, layout: SampleLayout, cfa: CfaPattern) -> Self {
        ImageMetadata { width, height, layout, cfa: Some(cfa) }
    }

    /// Deterministic structural validation. Does not check bit depth or sample data.
    pub fn validate(&self) -> Result<(), PrawError> {
        if self.width == 0 || self.height == 0 {
            return Err(PrawError::ZeroDimension);
        }
        if self.width > MAX_DIM || self.height > MAX_DIM {
            return Err(PrawError::DimensionTooLarge);
        }
        match (self.layout, &self.cfa) {
            (SampleLayout::BayerCfa, Some(p)) => {
                p.validate()?;
                p.validate_bayer()
            }
            (SampleLayout::OtherCfa, Some(p)) => p.validate(),
            (SampleLayout::BayerCfa | SampleLayout::OtherCfa, None) => {
                Err(PrawError::InvalidMetadata("CFA layout requires a CFA pattern"))
            }
            (SampleLayout::Unspecified | SampleLayout::Monochrome | SampleLayout::Linear, None) => Ok(()),
            (SampleLayout::Unspecified | SampleLayout::Monochrome | SampleLayout::Linear, Some(_)) => {
                Err(PrawError::InvalidMetadata("CFA pattern supplied for a non-CFA layout"))
            }
        }
    }
}

/// Stage 1 output: the unchanged Morton-order residual stream plus the separate descriptive metadata.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Stage1Output {
    /// Zigzag-mapped residuals, identical to [`encode`] output for the same samples and dimensions.
    pub residuals: Vec<u32>,
    pub metadata: ImageMetadata,
    pub bit_depth: u8,
}

impl Stage1Output {
    /// Residual `k` belongs to the pixel visited `k`-th by [`morton_visit`] over `metadata.width` x `metadata.height`
    /// (x bits at even Morton positions, y bits at odd positions, out-of-image coordinates skipped).
    pub fn coordinates(&self) -> Vec<(usize, usize)> {
        let mut v = Vec::with_capacity(self.residuals.len());
        let _ = morton_visit::<()>(self.metadata.width, self.metadata.height, |x, y| {
            v.push((x, y));
            Ok(())
        });
        v
    }
}

/// Metadata-aware [`encode`]. `metadata` must be valid and match `width` x `height`; it does not affect the residuals.
pub fn encode_with_metadata(
    samples: &[u16],
    width: usize,
    height: usize,
    bit_depth: u8,
    metadata: ImageMetadata,
) -> Result<Stage1Output, PrawError> {
    metadata.validate()?;
    if (metadata.width, metadata.height) != (width, height) {
        return Err(PrawError::MetadataGeometryMismatch {
            expected: (width, height),
            actual: (metadata.width, metadata.height),
        });
    }
    let residuals = encode(samples, width, height, bit_depth)?;
    Ok(Stage1Output { residuals, metadata, bit_depth })
}

/// Metadata-aware [`decode`]: returns the row-major sample plane for `out.metadata` dimensions.
pub fn decode_with_metadata(out: &Stage1Output) -> Result<Vec<u16>, PrawError> {
    out.metadata.validate()?;
    decode(&out.residuals, out.metadata.width, out.metadata.height, out.bit_depth)
}

// ---------------------------------------------------------------- Stage 2: Bayer-aware organization
//
// Stage 2 is a reversible permutation of the Stage 1 residuals. Values are never altered. The output carries the data
// together with the cumulative reconstruction history (oldest record first); reversal pops records newest-first.

/// One applied transformation, with everything needed to reverse it. Later stages append their own variants.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReconstructionRecord {
    /// Stage 1 structural decorrelation (Morton-order zigzag residuals) at the given bit depth.
    Stage1Decorrelation { bit_depth: u8 },
    /// Stage 2 organization of the Stage 1 residuals.
    Stage2Organization(Stage2Record),
    /// Stage 3 context classification. Residual values are unchanged; the record only identifies the fixed rule set.
    Stage3Context(Stage3Record),
    /// Stage 4 entropy coding. Residuals are recoverable only by decoding the payload under this record's rules.
    Stage4Entropy(Stage4Record),
}

/// Identity of a Bayer plane. `Green1` is the green sharing a CFA row with red; `Green2` shares a row with blue.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlaneRole {
    Red,
    Green1,
    Green2,
    Blue,
}

/// Geometry of one Bayer plane. Plane `i` holds the pixels (`offset_x + 2*u`, `offset_y + 2*v`) in row-major (u, v)
/// order, with `i = offset_y * 2 + offset_x` in image coordinates (not pattern coordinates).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlaneInfo {
    pub role: PlaneRole,
    pub color: ColorFilter,
    pub offset_x: usize,
    pub offset_y: usize,
    pub width: usize,
    pub height: usize,
}

/// How Stage 2 organized the residuals.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Stage2Mode {
    /// Residuals kept in Stage 1 Morton order.
    Identity,
    /// Four CFA-position planes, in image-parity order (0,0), (1,0), (0,1), (1,1).
    BayerPlanes { cfa: CfaPattern, planes: [PlaneInfo; 4] },
}

/// Stage 2 reconstruction record: the image geometry and the organization applied.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Stage2Record {
    pub width: usize,
    pub height: usize,
    pub mode: Stage2Mode,
}

/// Organized residual data. The variant must agree with the newest [`Stage2Record`] in the history.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OrganizedResiduals {
    /// Stage 1 residual sequence, Morton order, unchanged.
    Morton(Vec<u32>),
    /// One residual vector per plane, ordered as in [`Stage2Mode::BayerPlanes`].
    BayerPlanes([Vec<u32>; 4]),
}

/// Stage 2 output: organized residuals bound to the metadata and reconstruction history that describe them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Stage2Output {
    data: OrganizedResiduals,
    metadata: ImageMetadata,
    bit_depth: u8,
    history: Vec<ReconstructionRecord>,
}

impl Stage2Output {
    pub fn data(&self) -> &OrganizedResiduals {
        &self.data
    }
    pub fn metadata(&self) -> &ImageMetadata {
        &self.metadata
    }
    pub fn bit_depth(&self) -> u8 {
        self.bit_depth
    }
    /// Cumulative history, oldest first; reverse in the opposite order.
    pub fn history(&self) -> &[ReconstructionRecord] {
        &self.history
    }
    /// Organization mode of the Stage 2 record, if it is the newest one.
    pub fn mode(&self) -> Option<&Stage2Mode> {
        match self.history.last() {
            Some(ReconstructionRecord::Stage2Organization(r)) => Some(&r.mode),
            _ => None,
        }
    }
    /// Appends a record for a later stage's transformation together with the data it produced, in a single step so
    /// data and history cannot be mismatched.
    pub fn extend(&mut self, record: ReconstructionRecord, data: OrganizedResiduals) {
        self.history.push(record);
        self.data = data;
    }
}

fn plane_dims(width: usize, height: usize, ox: usize, oy: usize) -> Result<(usize, usize), PrawError> {
    let dim = |n: usize, o: usize| n.checked_add(1).and_then(|v| v.checked_sub(o)).map(|v| v / 2);
    match (dim(width, ox), dim(height, oy)) {
        (Some(w), Some(h)) => Ok((w, h)),
        _ => Err(PrawError::DimensionTooLarge),
    }
}

fn bayer_planes(width: usize, height: usize, cfa: &CfaPattern) -> Result<[PlaneInfo; 4], PrawError> {
    let color = |i: usize| cfa.color_at(i & 1, i >> 1);
    let red_i = (0..4)
        .find(|&i| color(i) == ColorFilter::Red)
        .ok_or(PrawError::InvalidMetadata("Bayer CFA has no red"))?;
    let mut planes = [PlaneInfo {
        role: PlaneRole::Red,
        color: ColorFilter::Red,
        offset_x: 0,
        offset_y: 0,
        width: 0,
        height: 0,
    }; 4];
    for (i, p) in planes.iter_mut().enumerate() {
        let (ox, oy) = (i & 1, i >> 1);
        let c = color(i);
        let role = match c {
            ColorFilter::Red => PlaneRole::Red,
            ColorFilter::Blue => PlaneRole::Blue,
            ColorFilter::Green if oy == red_i >> 1 => PlaneRole::Green1,
            ColorFilter::Green => PlaneRole::Green2,
            _ => return Err(PrawError::InvalidMetadata("Bayer CFA contains a non-RGB entry")),
        };
        let (w, h) = plane_dims(width, height, ox, oy)?;
        *p = PlaneInfo { role, color: c, offset_x: ox, offset_y: oy, width: w, height: h };
    }
    Ok(planes)
}

/// Index of the pixel (x, y) within its plane's vector, with checked arithmetic.
fn plane_index(planes: &[PlaneInfo; 4], x: usize, y: usize) -> Result<(usize, usize), PrawError> {
    let pi = (y & 1) * 2 + (x & 1);
    let i = (y >> 1)
        .checked_mul(planes[pi].width)
        .and_then(|v| v.checked_add(x >> 1))
        .ok_or(PrawError::InvalidMetadata("impossible Bayer plane mapping"))?;
    Ok((pi, i))
}

/// Stage 2 entry point: organizes Stage 1's residuals. Bayer mosaics are de-interleaved into four CFA-position planes;
/// every other supported layout passes through unchanged. Consumes the Stage 1 result so the data cannot be detached
/// from its metadata.
pub fn organize_stage2(stage1: Stage1Output) -> Result<Stage2Output, PrawError> {
    let Stage1Output { residuals, metadata, bit_depth } = stage1;
    metadata.validate()?;
    let (width, height) = (metadata.width, metadata.height);
    let n = validate(width, height, bit_depth)?;
    if residuals.len() != n {
        return Err(PrawError::LengthMismatch { expected: n, actual: residuals.len() });
    }

    let (mode, data) = match (metadata.layout, &metadata.cfa) {
        (SampleLayout::BayerCfa, Some(cfa)) => {
            let planes = bayer_planes(width, height, cfa)?;
            let mut total = 0usize;
            let mut out: [Vec<u32>; 4] = Default::default();
            for (v, p) in out.iter_mut().zip(&planes) {
                let len = p.width.checked_mul(p.height).ok_or(PrawError::DimensionTooLarge)?;
                total = total.checked_add(len).ok_or(PrawError::DimensionTooLarge)?;
                *v = vec![0; len];
            }
            if total != n {
                return Err(PrawError::InvalidMetadata("Bayer plane sizes do not cover the image"));
            }
            let mut k = 0usize;
            morton_visit::<PrawError>(width, height, |x, y| {
                let (pi, i) = plane_index(&planes, x, y)?;
                *out[pi].get_mut(i).ok_or(PrawError::InvalidMetadata("impossible Bayer plane mapping"))? = residuals[k];
                k += 1;
                Ok(())
            })?;
            (Stage2Mode::BayerPlanes { cfa: cfa.clone(), planes }, OrganizedResiduals::BayerPlanes(out))
        }
        (SampleLayout::BayerCfa, None) => {
            return Err(PrawError::InvalidMetadata("CFA layout requires a CFA pattern"));
        }
        (SampleLayout::Unspecified | SampleLayout::Monochrome | SampleLayout::Linear | SampleLayout::OtherCfa, _) => {
            (Stage2Mode::Identity, OrganizedResiduals::Morton(residuals))
        }
    };

    let history = vec![
        ReconstructionRecord::Stage1Decorrelation { bit_depth },
        ReconstructionRecord::Stage2Organization(Stage2Record { width, height, mode }),
    ];
    Ok(Stage2Output { data, metadata, bit_depth, history })
}

/// Exact inverse of [`organize_stage2`]: restores the Stage 1 residual sequence (Morton order, values unchanged) and
/// returns it with the inherited history that precedes the Stage 2 record. Requires Stage 2 to be the newest record.
pub fn restore_stage1(out: &Stage2Output) -> Result<(Stage1Output, Vec<ReconstructionRecord>), PrawError> {
    let Some((ReconstructionRecord::Stage2Organization(rec), inherited)) = out.history.split_last() else {
        return Err(PrawError::InvalidMetadata("newest reconstruction record is not Stage 2"));
    };
    out.metadata.validate()?;
    let (width, height) = (out.metadata.width, out.metadata.height);
    if (rec.width, rec.height) != (width, height) {
        return Err(PrawError::MetadataGeometryMismatch { expected: (width, height), actual: (rec.width, rec.height) });
    }
    let n = validate(width, height, out.bit_depth)?;

    let residuals = match (&rec.mode, &out.data) {
        (Stage2Mode::Identity, OrganizedResiduals::Morton(r)) => {
            if r.len() != n {
                return Err(PrawError::LengthMismatch { expected: n, actual: r.len() });
            }
            r.clone()
        }
        (Stage2Mode::BayerPlanes { cfa, planes }, OrganizedResiduals::BayerPlanes(data)) => {
            if out.metadata.cfa.as_ref() != Some(cfa) || bayer_planes(width, height, cfa)? != *planes {
                return Err(PrawError::InvalidMetadata("Stage 2 record disagrees with metadata"));
            }
            for (v, p) in data.iter().zip(planes) {
                let len = p.width.checked_mul(p.height).ok_or(PrawError::DimensionTooLarge)?;
                if v.len() != len {
                    return Err(PrawError::LengthMismatch { expected: len, actual: v.len() });
                }
            }
            let mut r = Vec::with_capacity(n);
            morton_visit::<PrawError>(width, height, |x, y| {
                let (pi, i) = plane_index(planes, x, y)?;
                r.push(*data[pi].get(i).ok_or(PrawError::InvalidMetadata("impossible Bayer plane mapping"))?);
                Ok(())
            })?;
            r
        }
        _ => return Err(PrawError::InvalidMetadata("organized data does not match the Stage 2 record")),
    };
    Ok((Stage1Output { residuals, metadata: out.metadata.clone(), bit_depth: out.bit_depth }, inherited.to_vec()))
}

// ---------------------------------------------------------------- Stage 3: deterministic context classification
//
// Stage 3 leaves every residual unchanged and labels each one with a local-activity class derived only from residuals
// that precede it in the coding order (left, above, upper-left). A decoder reproduces each label from already decoded
// residuals before decoding the residual itself, so no label stream is stored. No probabilities or coding live here.
//
// Activity score = mean of the eligible neighbors' zigzag residuals, rounded half up in integer arithmetic:
//   score = (sum + n/2) / n, with n = number of eligible neighbors (1..=3). No eligible neighbor => Low.
// Classes (fixed thresholds, upper bounds exclusive): Low < 3 <= Moderate < 12 <= High < 48 <= VeryHigh.

/// Version of the fixed Stage 3 rule set (neighborhood, score and thresholds). Bump if any of them ever changes.
pub const STAGE3_RULE_VERSION: u8 = 1;
/// First score of Moderate, High and VeryHigh respectively.
pub const STAGE3_THRESHOLDS: [u32; 3] = [3, 12, 48];

/// Local-activity class. Discriminants are stable and usable directly as a context index (0..=3).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u8)]
pub enum ActivityClass {
    Low = 0,
    Moderate = 1,
    High = 2,
    VeryHigh = 3,
}

impl ActivityClass {
    pub const COUNT: usize = 4;

    #[inline]
    pub fn index(self) -> usize {
        self as usize
    }
}

/// Stage 3 reconstruction record. Describes the classification rule; it does not transform residual values.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Stage3Record {
    pub rule_version: u8,
    pub class_count: u8,
    pub thresholds: [u32; 3],
}

impl Stage3Record {
    fn current() -> Self {
        Stage3Record {
            rule_version: STAGE3_RULE_VERSION,
            class_count: ActivityClass::COUNT as u8,
            thresholds: STAGE3_THRESHOLDS,
        }
    }
}

/// Classifies one residual from its causal neighbors (`None` = outside the plane or not yet available). Encoder and
/// decoder both call this with the already known zigzag residuals of left, above and upper-left.
#[inline]
pub fn classify_activity(left: Option<u32>, above: Option<u32>, above_left: Option<u32>) -> ActivityClass {
    let (mut sum, mut n) = (0u64, 0u64);
    for v in [left, above, above_left].into_iter().flatten() {
        sum += u64::from(v);
        n += 1;
    }
    if n == 0 {
        return ActivityClass::Low;
    }
    let score = (sum + n / 2) / n;
    let [t1, t2, t3] = STAGE3_THRESHOLDS.map(u64::from);
    if score < t1 {
        ActivityClass::Low
    } else if score < t2 {
        ActivityClass::Moderate
    } else if score < t3 {
        ActivityClass::High
    } else {
        ActivityClass::VeryHigh
    }
}

/// One class per residual, laid out exactly like the residuals in [`OrganizedResiduals`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OrganizedContexts {
    /// Parallel to [`OrganizedResiduals::Morton`]. Neighbors are the spatial W, N, NW pixels, which always precede the
    /// current pixel in Morton order.
    Morton(Vec<ActivityClass>),
    /// Parallel to [`OrganizedResiduals::BayerPlanes`]. Neighbors are W, N, NW inside the same row-major plane.
    BayerPlanes([Vec<ActivityClass>; 4]),
}

/// Stage 3 output: the Stage 2 result (residuals, metadata and history, with the Stage 3 record appended) plus the
/// per-residual classes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Stage3Output {
    base: Stage2Output,
    contexts: OrganizedContexts,
}

impl Stage3Output {
    /// Unchanged Stage 2 residuals.
    pub fn data(&self) -> &OrganizedResiduals {
        &self.base.data
    }
    pub fn contexts(&self) -> &OrganizedContexts {
        &self.contexts
    }
    pub fn metadata(&self) -> &ImageMetadata {
        &self.base.metadata
    }
    pub fn bit_depth(&self) -> u8 {
        self.base.bit_depth
    }
    /// Cumulative history, oldest first; the newest record is Stage 3.
    pub fn history(&self) -> &[ReconstructionRecord] {
        &self.base.history
    }
    /// Stage 2 organization mode (geometry for Stage 4), found beneath the Stage 3 record.
    pub fn mode(&self) -> Option<&Stage2Mode> {
        self.base.history.iter().rev().find_map(|r| match r {
            ReconstructionRecord::Stage2Organization(r) => Some(&r.mode),
            _ => None,
        })
    }
    /// Drops the classes and the Stage 3 record, returning the exact Stage 2 output.
    pub fn into_stage2(mut self) -> Stage2Output {
        self.base.history.pop();
        self.base
    }
}

fn classify_plane(v: &[u32], w: usize) -> Vec<ActivityClass> {
    let mut out = Vec::with_capacity(v.len());
    if w == 0 {
        return out;
    }
    for i in 0..v.len() {
        let (x, y) = (i % w, i / w);
        let left = (x > 0).then(|| v[i - 1]);
        let above = (y > 0).then(|| v[i - w]);
        let above_left = (x > 0 && y > 0).then(|| v[i - w - 1]);
        out.push(classify_activity(left, above, above_left));
    }
    out
}

/// Stage 3 entry point. Consumes the Stage 2 result, classifies every residual and appends a Stage 3 record.
pub fn classify_stage3(stage2: Stage2Output) -> Result<Stage3Output, PrawError> {
    stage2.metadata.validate()?;
    let (width, height) = (stage2.metadata.width, stage2.metadata.height);
    let n = validate(width, height, stage2.bit_depth)?;
    let Some(ReconstructionRecord::Stage2Organization(rec)) = stage2.history.last() else {
        return Err(PrawError::InvalidMetadata("newest reconstruction record is not Stage 2"));
    };
    if (rec.width, rec.height) != (width, height) {
        return Err(PrawError::MetadataGeometryMismatch { expected: (width, height), actual: (rec.width, rec.height) });
    }

    let contexts = match (&rec.mode, &stage2.data) {
        (Stage2Mode::Identity, OrganizedResiduals::Morton(r)) => {
            if r.len() != n {
                return Err(PrawError::LengthMismatch { expected: n, actual: r.len() });
            }
            // Row-major scratch of residuals already visited; W, N and NW precede the current pixel in Morton order.
            let mut seen = vec![0u32; n];
            let mut out = Vec::with_capacity(n);
            let mut k = 0usize;
            morton_visit::<PrawError>(width, height, |x, y| {
                let left = (x > 0).then(|| seen[y * width + x - 1]);
                let above = (y > 0).then(|| seen[(y - 1) * width + x]);
                let above_left = (x > 0 && y > 0).then(|| seen[(y - 1) * width + x - 1]);
                out.push(classify_activity(left, above, above_left));
                seen[y * width + x] = r[k];
                k += 1;
                Ok(())
            })?;
            OrganizedContexts::Morton(out)
        }
        (Stage2Mode::BayerPlanes { planes, .. }, OrganizedResiduals::BayerPlanes(data)) => {
            let mut out: [Vec<ActivityClass>; 4] = Default::default();
            for ((o, v), p) in out.iter_mut().zip(data).zip(planes) {
                let len = p.width.checked_mul(p.height).ok_or(PrawError::DimensionTooLarge)?;
                if v.len() != len {
                    return Err(PrawError::LengthMismatch { expected: len, actual: v.len() });
                }
                *o = classify_plane(v, p.width);
            }
            OrganizedContexts::BayerPlanes(out)
        }
        _ => return Err(PrawError::InvalidMetadata("organized data does not match the Stage 2 record")),
    };

    let mut base = stage2;
    base.history.push(ReconstructionRecord::Stage3Context(Stage3Record::current()));
    Ok(Stage3Output { base, contexts })
}

// ---------------------------------------------------------------- Stage 4: PKON context-adaptive entropy coding
//
// Each residual is coded with the PKON binary range coder and one of four independent adaptive models chosen by its
// Stage 3 class. A residual's zigzag value v is binarized as x = v + 1 (1..=2^32): the bit length e of x in unary
// (adaptive, one probability per position), then the e-1 bits below the leading one, MSB first; the first of those is
// adaptive per e and the rest are coded with fixed p = 1/2. Models start fresh (p = 1/2, count 0), are never shared or
// reset between planes, and only the selected model adapts. The decoder recomputes each class from already decoded
// residuals with the Stage 3 rule, so no class labels are stored. Traversal is the Stage 3 order: Bayer planes 0..3 in
// row-major order, or the Morton visitation order for the identity path.

/// Coding-mode/version of the Stage 4 stream defined here: four class-selected adaptive models, binarization above.
pub const STAGE4_CODING_VERSION: u8 = 1;
const S4_E_MAX: usize = 33;

/// Stage 4 reconstruction record.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Stage4Record {
    pub coding_version: u8,
    pub model_count: u8,
    /// Number of residuals coded (redundant with the geometry; validated on decode).
    pub residual_count: u64,
}

#[derive(Clone, Copy)]
struct S4Model {
    unary: [Bit; S4_E_MAX + 1],
    mant: [Bit; S4_E_MAX + 1],
}

impl S4Model {
    const NEW: S4Model = S4Model { unary: [Bit::NEW; S4_E_MAX + 1], mant: [Bit::NEW; S4_E_MAX + 1] };

    /// Codes one adaptive bit and updates it. Encoders pass the real `bit`; decoders pass anything and get the decoded one.
    #[inline]
    fn bit<I: BitIo>(io: &mut I, b: &mut Bit, bit: u32) -> u32 {
        let bit = io.code(b.p1(), bit);
        b.adapt(bit);
        bit
    }

    /// Shared encode/decode of one zigzag residual; returns the (de)coded value.
    fn code<I: BitIo>(&mut self, io: &mut I, v: u32) -> Result<u32, PrawError> {
        let x = u64::from(v) + 1;
        let e_in = (64 - x.leading_zeros()) as usize;
        let mut e = S4_E_MAX;
        for i in 1..S4_E_MAX {
            if Self::bit(io, &mut self.unary[i], (e_in > i) as u32) == 0 {
                e = i;
                break;
            }
        }
        let mut out = 1u64;
        for j in (0..e - 1).rev() {
            let want = ((x >> j) & 1) as u32;
            let b = if j == e - 2 { Self::bit(io, &mut self.mant[e], want) } else { io.code(32768, want) };
            out = (out << 1) | u64::from(b);
        }
        u32::try_from(out - 1).map_err(|_| PrawError::CorruptStream("residual exceeds 32 bits"))
    }
}

/// Stage 4 output: the entropy-coded Stage 3 residuals plus the inherited metadata and cumulative history (newest
/// record is Stage 4). Contexts are not stored; they are rebuilt while decoding.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Stage4Output {
    payload: Vec<u8>,
    metadata: ImageMetadata,
    bit_depth: u8,
    history: Vec<ReconstructionRecord>,
}

impl Stage4Output {
    pub fn payload(&self) -> &[u8] {
        &self.payload
    }
    pub fn metadata(&self) -> &ImageMetadata {
        &self.metadata
    }
    pub fn bit_depth(&self) -> u8 {
        self.bit_depth
    }
    /// Cumulative history, oldest first; the newest record is Stage 4.
    pub fn history(&self) -> &[ReconstructionRecord] {
        &self.history
    }
}

/// Stage 4 entry point: entropy-codes the Stage 3 residuals, selecting the model by each residual's Stage 3 class.
pub fn encode_stage4(stage3: Stage3Output) -> Result<Stage4Output, PrawError> {
    let (width, height) = (stage3.base.metadata.width, stage3.base.metadata.height);
    validate(width, height, stage3.base.bit_depth)?;
    let mut count = 0u64;
    let mut models = [S4Model::NEW; ActivityClass::COUNT];
    let mut enc = BinEnc::new();
    let mut code_all = |res: &[u32], ctx: &[ActivityClass]| -> Result<(), PrawError> {
        if res.len() != ctx.len() {
            return Err(PrawError::LengthMismatch { expected: res.len(), actual: ctx.len() });
        }
        count += res.len() as u64;
        for (&v, c) in res.iter().zip(ctx) {
            models[c.index()].code(&mut enc, v)?;
        }
        Ok(())
    };
    match (&stage3.base.data, &stage3.contexts) {
        (OrganizedResiduals::Morton(r), OrganizedContexts::Morton(c)) => code_all(r, c)?,
        (OrganizedResiduals::BayerPlanes(r), OrganizedContexts::BayerPlanes(c)) => {
            for (r, c) in r.iter().zip(c) {
                code_all(r, c)?;
            }
        }
        _ => return Err(PrawError::InvalidMetadata("contexts do not match the organized data")),
    }
    let payload = enc.finish();
    let Stage2Output { metadata, bit_depth, mut history, .. } = stage3.base;
    history.push(ReconstructionRecord::Stage4Entropy(Stage4Record {
        coding_version: STAGE4_CODING_VERSION,
        model_count: ActivityClass::COUNT as u8,
        residual_count: count,
    }));
    Ok(Stage4Output { payload, metadata, bit_depth, history })
}

/// Exact inverse of [`encode_stage4`]: returns the Stage 3 representation (residuals, contexts, history without the
/// Stage 4 record). Feed it to [`Stage3Output::into_stage2`] and [`restore_stage1`] to continue reversing.
pub fn decode_stage4(out: &Stage4Output) -> Result<Stage3Output, PrawError> {
    let Some((ReconstructionRecord::Stage4Entropy(rec), inherited)) = out.history.split_last() else {
        return Err(PrawError::InvalidMetadata("newest reconstruction record is not Stage 4"));
    };
    if rec.coding_version != STAGE4_CODING_VERSION || usize::from(rec.model_count) != ActivityClass::COUNT {
        return Err(PrawError::CorruptStream("unsupported Stage 4 coding mode or version"));
    }
    if inherited.last() != Some(&ReconstructionRecord::Stage3Context(Stage3Record::current())) {
        return Err(PrawError::InvalidMetadata("Stage 4 does not follow a supported Stage 3 record"));
    }
    out.metadata.validate()?;
    let (width, height) = (out.metadata.width, out.metadata.height);
    let n = validate(width, height, out.bit_depth)?;
    let Some(ReconstructionRecord::Stage2Organization(s2)) =
        inherited.iter().rev().find(|r| matches!(r, ReconstructionRecord::Stage2Organization(_)))
    else {
        return Err(PrawError::InvalidMetadata("missing Stage 2 record"));
    };
    if (s2.width, s2.height) != (width, height) {
        return Err(PrawError::MetadataGeometryMismatch { expected: (width, height), actual: (s2.width, s2.height) });
    }
    if rec.residual_count != n as u64 {
        return Err(PrawError::CorruptStream("residual count disagrees with geometry"));
    }

    let mut models = [S4Model::NEW; ActivityClass::COUNT];
    let mut dec = BinDec::new(&out.payload);
    let mut next = |c: ActivityClass| models[c.index()].code(&mut dec, 0);

    let (data, contexts) = match &s2.mode {
        Stage2Mode::Identity => {
            let mut seen = vec![0u32; n];
            let (mut res, mut ctx) = (Vec::with_capacity(n), Vec::with_capacity(n));
            morton_visit::<PrawError>(width, height, |x, y| {
                let left = (x > 0).then(|| seen[y * width + x - 1]);
                let above = (y > 0).then(|| seen[(y - 1) * width + x]);
                let above_left = (x > 0 && y > 0).then(|| seen[(y - 1) * width + x - 1]);
                let c = classify_activity(left, above, above_left);
                let v = next(c)?;
                seen[y * width + x] = v;
                res.push(v);
                ctx.push(c);
                Ok(())
            })?;
            (OrganizedResiduals::Morton(res), OrganizedContexts::Morton(ctx))
        }
        Stage2Mode::BayerPlanes { cfa, planes } => {
            if out.metadata.cfa.as_ref() != Some(cfa) || bayer_planes(width, height, cfa)? != *planes {
                return Err(PrawError::InvalidMetadata("Stage 2 record disagrees with metadata"));
            }
            let mut res: [Vec<u32>; 4] = Default::default();
            let mut ctx: [Vec<ActivityClass>; 4] = Default::default();
            for ((r, c), p) in res.iter_mut().zip(ctx.iter_mut()).zip(planes) {
                let (w, len) = (p.width, p.width.checked_mul(p.height).ok_or(PrawError::DimensionTooLarge)?);
                r.reserve_exact(len);
                c.reserve_exact(len);
                for i in 0..len {
                    let (x, y) = (i % w, i / w);
                    let left = (x > 0).then(|| r[i - 1]);
                    let above = (y > 0).then(|| r[i - w]);
                    let above_left = (x > 0 && y > 0).then(|| r[i - w - 1]);
                    let cl = classify_activity(left, above, above_left);
                    r.push(next(cl)?);
                    c.push(cl);
                }
            }
            (OrganizedResiduals::BayerPlanes(res), OrganizedContexts::BayerPlanes(ctx))
        }
    };
    if dec.consumed() != out.payload.len() {
        return Err(PrawError::CorruptStream("payload truncated or has trailing bytes"));
    }
    let base = Stage2Output { data, metadata: out.metadata.clone(), bit_depth: out.bit_depth, history: inherited.to_vec() };
    Ok(Stage3Output { base, contexts })
}
