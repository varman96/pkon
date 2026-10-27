//! GBLEND: backward-adaptive blend of MED and a strictly causal linear predictor, G plane (plane 0) only.
//!
//! Everything is derived from already-decoded pixels, so encoder and decoder evolve identically and there is no side information.
//! * Linear predictor: 10 causal taps + bias. At the start of every 128-pixel tile row (y >= 128) the coefficients of each tile column are
//!   fitted by least squares on the 128x128 tile directly ABOVE it (fully decoded). Normal equations are accumulated in exact integer
//!   arithmetic; the 11x11 solve uses IEEE-754 f64 add/sub/mul/div only (no FMA, no libm), and the result is quantised to Q16 integers, so the
//!   per-pixel prediction is pure integer arithmetic. Tiles of the first tile row have no linear predictor (MED only).
//! * Blend: weights (1<<28)/(s^2+1) from s = sum of |error| of that predictor at W, N, NW, NE (same inverse-squared-error weighting as
//!   `Ens::Sq3`); if MED's local error sum is 0 the prediction is MED exactly (MED fallback). Pixels without a linear predictor count as
//!   equal error for both predictors.
pub const TILE: usize = 128;
const NT: usize = 11; // 10 taps + bias
const FIX: i64 = 1 << 16;

pub struct GBlend {
    w: usize,
    h: usize,
    coef: Vec<Option<[i32; NT]>>, // per tile column, for the current tile row
    e_med: Vec<u16>,
    e_ls: Vec<u16>,
    // values of the last predict() call, consumed by update()
    last_med: i32,
    last_ls: Option<i32>,
}

#[inline]
fn px(p: &[u8], w: usize, h: usize, x: isize, y: isize) -> i64 {
    if x < 0 || y < 0 || x as usize >= w || y as usize >= h {
        0
    } else {
        p[y as usize * w + x as usize] as i64
    }
}
#[inline]
fn taps(p: &[u8], w: usize, h: usize, x: usize, y: usize) -> [i64; NT] {
    let (x, y) = (x as isize, y as isize);
    [
        px(p, w, h, x - 1, y),
        px(p, w, h, x, y - 1),
        px(p, w, h, x - 1, y - 1),
        px(p, w, h, x + 1, y - 1),
        px(p, w, h, x - 2, y),
        px(p, w, h, x, y - 2),
        px(p, w, h, x + 1, y - 2),
        px(p, w, h, x - 2, y - 1),
        px(p, w, h, x + 2, y - 1),
        px(p, w, h, x - 1, y - 2),
        1,
    ]
}

/// Solve (A + ridge) c = b with partial pivoting; None if not finite.
fn solve(mut a: [[f64; NT]; NT], mut b: [f64; NT]) -> Option<[f64; NT]> {
    for i in 0..NT {
        a[i][i] += 0.5;
    }
    for c in 0..NT {
        let mut piv = c;
        for r in c + 1..NT {
            if a[r][c].abs() > a[piv][c].abs() {
                piv = r;
            }
        }
        a.swap(c, piv);
        b.swap(c, piv);
        let d = a[c][c];
        if !(d.abs() > 1e-9) {
            return None;
        }
        for r in c + 1..NT {
            let f = a[r][c] / d;
            for k in c..NT {
                a[r][k] -= f * a[c][k];
            }
            b[r] -= f * b[c];
        }
    }
    let mut x = [0f64; NT];
    for r in (0..NT).rev() {
        let mut s = b[r];
        for k in r + 1..NT {
            s -= a[r][k] * x[k];
        }
        x[r] = s / a[r][r];
        if !x[r].is_finite() {
            return None;
        }
    }
    Some(x)
}

impl GBlend {
    pub fn new(w: usize, h: usize) -> Self {
        GBlend { w, h, coef: vec![None; w.div_ceil(TILE)], e_med: vec![0; w * h], e_ls: vec![0; w * h], last_med: 0, last_ls: None }
    }

    /// Call before coding pixel (0, y) of plane 0 (all rows < y are decoded).
    pub fn start_row(&mut self, p: &[u8], y: usize) {
        if y % TILE != 0 || y < TILE {
            return;
        }
        let (w, h) = (self.w, self.h);
        for (tc, slot) in self.coef.iter_mut().enumerate() {
            let (x0, x1) = (tc * TILE, ((tc + 1) * TILE).min(w));
            let mut ata = [[0i64; NT]; NT];
            let mut atb = [0i64; NT];
            for yy in y - TILE..y {
                for xx in x0..x1 {
                    let t = taps(p, w, h, xx, yy);
                    let v = p[yy * w + xx] as i64;
                    for i in 0..NT {
                        atb[i] += t[i] * v;
                        for j in i..NT {
                            ata[i][j] += t[i] * t[j];
                        }
                    }
                }
            }
            let mut a = [[0f64; NT]; NT];
            let mut b = [0f64; NT];
            for i in 0..NT {
                b[i] = atb[i] as f64;
                for j in i..NT {
                    a[i][j] = ata[i][j] as f64;
                    a[j][i] = ata[i][j] as f64;
                }
            }
            *slot = solve(a, b).map(|c| std::array::from_fn(|i| (c[i] * FIX as f64).round().clamp(-(1i64 << 26) as f64, (1i64 << 26) as f64) as i32));
        }
    }

    fn esum(&self, e: &[u16], x: usize, y: usize) -> u64 {
        let i = y * self.w + x;
        let mut s = 0u64;
        if x > 0 {
            s += e[i - 1] as u64;
        }
        if y > 0 {
            s += e[i - self.w] as u64;
            if x > 0 {
                s += e[i - self.w - 1] as u64;
            }
            if x + 1 < self.w {
                s += e[i - self.w + 1] as u64;
            }
        }
        s
    }

    /// Blended prediction for (x, y) given the MED prediction.
    pub fn predict(&mut self, p: &[u8], x: usize, y: usize, med: i32) -> i32 {
        self.last_med = med;
        self.last_ls = None;
        let Some(c) = self.coef[x / TILE] else { return med };
        if y < TILE {
            return med;
        }
        let t = taps(p, self.w, self.h, x, y);
        let mut dot = 0i64;
        for i in 0..NT {
            dot += c[i] as i64 * t[i];
        }
        let ls = ((dot + FIX / 2) >> 16).clamp(0, 255) as i32;
        self.last_ls = Some(ls);
        let sm = self.esum(&self.e_med, x, y);
        if sm == 0 {
            return med;
        }
        let sl = self.esum(&self.e_ls, x, y);
        let wm = (1u64 << 28) / (sm * sm + 1);
        let wl = (1u64 << 28) / (sl * sl + 1);
        let den = wm + wl;
        ((2 * (wm * med as u64 + wl * ls as u64) + den) / (2 * den)) as i32
    }

    /// Call after the pixel value is known.
    pub fn update(&mut self, x: usize, y: usize, v: u8) {
        let i = y * self.w + x;
        let em = (v as i32 - self.last_med).unsigned_abs() as u16;
        self.e_med[i] = em;
        self.e_ls[i] = self.last_ls.map_or(em, |l| (v as i32 - l).unsigned_abs() as u16);
    }
}
