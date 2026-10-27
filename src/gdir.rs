//! GDIR: gradient-adaptive directional predictor for the G plane (plane 0) only. Frozen spec: results/20261027-203041Z-gdir-directional/criteria.md.
//!
//! Integer-only, derived from already reconstructed pixels and a small synchronized state, so encoder and decoder agree with no side information.
//! Neighbours (x right, y down): W(x-1,y) N(x,y-1) NW(x-1,y-1) NE(x+1,y-1) WW(x-2,y) NN(x,y-2) NNE(x+1,y-2) NNW(x-1,y-2) NWW(x-2,y-1).
//! A pixel is available iff it lies inside the image (raster order makes all of these causal); nothing is substituted.
//!
//! Candidates c in {W, N, NW, NE}: P_c = that neighbour (0..255, so clamping is the identity); available iff its neighbour is.
//! Fewer than two available candidates -> MED.
//!
//! Directional gradient (three absolute differences, a term is skipped, i.e. contributes 0, if either of its pixels is unavailable):
//!   g_W  = |W-WW|  + |N-NW|  + |NE-N|        g_N  = |W-NW|  + |N-NN|  + |NE-NNE|
//!   g_NW = |W-NWW| + |N-NNW| + |NE-NN|       g_NE = |W-N|   + |N-NNE| + |NW-NN|
//! History E_c (Q4 fixed point, 16 x mean abs error, init 64): after the pixel v is reconstructed, for every candidate that was available,
//!   E_c += ((|v - P_c| << 4) - E_c) >> 3      (arithmetic shift, rate 1/8)
//! Score   S_c = 16*g_c + E_c + 1   (>= 1, max ~16.3k).  Pick the two smallest (ties: lower index W<N<NW<NE) a (best), b.
//! Weight  w_c = floor(2^24 / S_c)  (lower score, larger weight).  den = w_a + w_b (MED if 0, which cannot happen).
//! Pred    = floor((2*(w_a*P_a + w_b*P_b) + den) / (2*den))   (round half up, u64; convex combination, so within 0..255)

const EINIT: i32 = 64;
const WSCALE: u32 = 1 << 24;

pub struct GDir {
    e: [i32; 4],
    // set by predict(), consumed by update()
    pc: [i32; 4],
    avail: [bool; 4],
}

impl Default for GDir {
    fn default() -> Self {
        Self::new()
    }
}

#[inline]
fn at(p: &[u8], w: usize, h: usize, x: isize, y: isize) -> Option<i32> {
    if x < 0 || y < 0 || x as usize >= w || y as usize >= h {
        None
    } else {
        Some(p[y as usize * w + x as usize] as i32)
    }
}
#[inline]
fn ad(a: Option<i32>, b: Option<i32>) -> i32 {
    match (a, b) {
        (Some(a), Some(b)) => (a - b).abs(),
        _ => 0,
    }
}

impl GDir {
    pub fn new() -> Self {
        GDir { e: [EINIT; 4], pc: [0; 4], avail: [false; 4] }
    }

    /// Prediction for (x, y) from `p` (only pixels earlier in raster order are read) given the production MED prediction.
    pub fn predict(&mut self, p: &[u8], w: usize, h: usize, x: usize, y: usize, med: i32) -> i32 {
        let (xi, yi) = (x as isize, y as isize);
        let g = |dx: isize, dy: isize| at(p, w, h, xi + dx, yi + dy);
        let (pw, pn, pnw, pne) = (g(-1, 0), g(0, -1), g(-1, -1), g(1, -1));
        let (pww, pnn, pnne, pnnw, pnww) = (g(-2, 0), g(0, -2), g(1, -2), g(-1, -2), g(-2, -1));
        let cand = [pw, pn, pnw, pne];
        let grad = [
            ad(pw, pww) + ad(pn, pnw) + ad(pne, pn),
            ad(pw, pnw) + ad(pn, pnn) + ad(pne, pnne),
            ad(pw, pnww) + ad(pn, pnnw) + ad(pne, pnn),
            ad(pw, pn) + ad(pn, pnne) + ad(pnw, pnn),
        ];
        self.avail = [false; 4];
        let mut best: [Option<(i32, usize)>; 2] = [None, None]; // (score, index), ascending score; strict < keeps lower index on ties
        for c in 0..4 {
            let Some(v) = cand[c] else { continue };
            self.avail[c] = true;
            self.pc[c] = v;
            let s = 16 * grad[c] + self.e[c] + 1;
            match best {
                [None, _] => best[0] = Some((s, c)),
                [Some(a), _] if s < a.0 => best = [Some((s, c)), Some(a)],
                [_, None] => best[1] = Some((s, c)),
                [_, Some(b)] if s < b.0 => best[1] = Some((s, c)),
                _ => {}
            }
        }
        let (Some((sa, a)), Some((sb, b))) = (best[0], best[1]) else { return med };
        let (wa, wb) = ((WSCALE / sa as u32) as u64, (WSCALE / sb as u32) as u64);
        let den = wa + wb;
        if den == 0 {
            return med;
        }
        ((2 * (wa * self.pc[a] as u64 + wb * self.pc[b] as u64) + den) / (2 * den)) as i32
    }

    /// Call after the pixel value is reconstructed.
    pub fn update(&mut self, v: u8) {
        for c in 0..4 {
            if self.avail[c] {
                let err = (v as i32 - self.pc[c]).abs() << 4;
                self.e[c] += (err - self.e[c]) >> 3;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pkon2::{neighbors, predict, Pred};

    fn lcg(s: &mut u64) -> u8 {
        *s = s.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        (*s >> 33) as u8
    }
    /// smooth-ish test content with edges and noise
    fn image(w: usize, h: usize, seed: u64) -> Vec<u8> {
        let mut s = seed;
        (0..w * h).map(|i| ((i % w) * 3 / 2 + (i / w) * 2) as u8 ^ (lcg(&mut s) & 7) ^ if (i % w) / 5 % 2 == 0 { 0 } else { 40 }).collect()
    }

    /// Encoder view (full image visible) and decoder view (future pixels poisoned, filled in as reconstructed) must give identical predictions at
    /// every sample, including every boundary shape; this also proves no future pixel is read.
    #[test]
    fn enc_dec_predictions_identical_and_causal() {
        for (w, h) in [(1, 1), (1, 7), (7, 1), (2, 2), (2, 9), (3, 3), (5, 4), (17, 13), (64, 40)] {
            for seed in 1..4u64 {
                let img = image(w, h, seed);
                let mut enc = GDir::new();
                let mut dec = GDir::new();
                let mut dec2 = GDir::new();
                let mut dbuf = vec![0xA5u8; w * h];
                let mut dbuf2 = vec![0x5Au8; w * h]; // different poison: result must not depend on it
                for y in 0..h {
                    for x in 0..w {
                        let i = y * w + x;
                        let med = predict(Pred::Med, neighbors(&img, w, x, y));
                        let pe = enc.predict(&img, w, h, x, y, med);
                        let pd = dec.predict(&dbuf, w, h, x, y, predict(Pred::Med, neighbors(&dbuf, w, x, y)));
                        let pd2 = dec2.predict(&dbuf2, w, h, x, y, predict(Pred::Med, neighbors(&dbuf2, w, x, y)));
                        assert_eq!(pe, pd, "enc/dec mismatch {w}x{h} ({x},{y})");
                        assert_eq!(pe, pd2, "poison-dependent {w}x{h} ({x},{y})");
                        assert!((0..=255).contains(&pe));
                        // decoder reconstructs from its own prediction and the transmitted residual
                        let v = img[i];
                        dbuf[i] = (pd as u8).wrapping_add(v.wrapping_sub(pe as u8));
                        dbuf2[i] = (pd2 as u8).wrapping_add(v.wrapping_sub(pe as u8));
                        enc.update(v);
                        dec.update(dbuf[i]);
                        dec2.update(dbuf2[i]);
                        assert_eq!(enc.e, dec.e, "state diverged {w}x{h} ({x},{y})");
                    }
                }
                assert_eq!(dbuf, img);
            }
        }
    }

    #[test]
    fn availability_and_med_fallback() {
        let img = image(8, 8, 9);
        let mut g = GDir::new();
        // (0,0): nothing available -> MED
        assert_eq!(g.predict(&img, 8, 8, 0, 0, 77), 77);
        assert_eq!(g.avail, [false; 4]);
        g.update(img[0]);
        // first row, x>0: only W available -> fewer than two -> MED
        assert_eq!(g.predict(&img, 8, 8, 1, 0, 55), 55);
        assert_eq!(g.avail, [true, false, false, false]);
        g.update(img[1]);
        // x=0, y=1: N and NE available, W/NW not
        let _ = g.predict(&img, 8, 8, 0, 1, 55);
        assert_eq!(g.avail, [false, true, false, true]);
        // last column, y>0: W, N, NW available, NE not
        let _ = g.predict(&img, 8, 8, 7, 3, 55);
        assert_eq!(g.avail, [true, true, true, false]);
        // interior: all four
        let _ = g.predict(&img, 8, 8, 3, 3, 55);
        assert_eq!(g.avail, [true; 4]);
        // width 1, y>0: only N available -> MED
        let col = image(1, 5, 2);
        assert_eq!(g.predict(&col, 1, 5, 0, 3, 31), 31);
        assert_eq!(g.avail, [false, true, false, false]);
    }

    #[test]
    fn flat_and_weighting() {
        // constant image: every candidate equals the constant, so the prediction is that constant
        let img = vec![123u8; 36];
        let mut g = GDir::new();
        for y in 0..6 {
            for x in 0..6 {
                assert_eq!(g.predict(&img, 6, 6, x, y, 123), 123);
                g.update(123);
            }
        }
        // vertical stripes: W differs strongly, N equals the pixel -> N (vertical) must dominate
        let (w, h) = (12, 12);
        let stripes: Vec<u8> = (0..w * h).map(|i| if (i % w) % 2 == 0 { 20 } else { 200 }).collect();
        let mut g = GDir::new();
        let mut last = 0;
        for y in 0..h {
            for x in 0..w {
                last = g.predict(&stripes, w, h, x, y, 0);
                g.update(stripes[y * w + x]);
            }
        }
        assert_eq!(last, stripes[h * w - 1] as i32, "vertical structure should predict exactly from N");
    }
}
