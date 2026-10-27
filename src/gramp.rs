//! GRAMP: planar prediction for smooth illumination ramps in the G plane (plane 0) only; MED everywhere else.
//! Frozen spec: results/20261027-204338Z-gramp-ramp-only/criteria.md. Stateless: the decision and the prediction are pure functions of the
//! already reconstructed pixels, so encoder and decoder agree with no side information.
//!
//! Pixels: W(x-1,y) WW(x-2,y) N(x,y-1) NW(x-1,y-1) NE(x+1,y-1) NWW(x-2,y-1) NN(x,y-2) NNW(x-1,y-2) NNE(x+1,y-2).
//! Boundary: evaluated only if x>=2, y>=2 and x+1<w (all nine pixels inside the image); otherwise MED.
//! Horizontal measurements h1=W-WW h2=N-NW h3=NE-N h4=NW-NWW; vertical v1=W-NW v2=N-NN v3=NE-NNE v4=NW-NNW.
//! Ramp iff (a) max-min of h <= 2 and of v <= 2, (b) every |h|,|v| <= 8, (c) |Sh|+|Sv| >= 4 (Sh = sum h, Sv = sum v).
//! Prediction (i32, floor shift): clamp((4*(W+N+NW+NE) + Sh + 3*Sv + 8) >> 4, 0, 255)
//!   = mean of the plane extrapolations W+H, N+V, NW+H+V, NE-H+V with H = Sh/4, V = Sv/4.

const RANGE_MAX: i32 = 2;
const ABS_MAX: i32 = 8;
const SLOPE_MIN: i32 = 4;

/// Planar prediction if (x, y) is accepted as a ramp, else None (caller uses MED). Reads only earlier raster positions.
#[inline]
pub fn ramp_predict(p: &[u8], w: usize, x: usize, y: usize) -> Option<i32> {
    if x < 2 || y < 2 || x + 1 >= w {
        return None;
    }
    let i = y * w + x;
    let g = |j: usize| p[j] as i32;
    let (pw, pww) = (g(i - 1), g(i - 2));
    let (pn, pnw, pne, pnww) = (g(i - w), g(i - w - 1), g(i - w + 1), g(i - w - 2));
    let (pnn, pnnw, pnne) = (g(i - 2 * w), g(i - 2 * w - 1), g(i - 2 * w + 1));
    let h = [pw - pww, pn - pnw, pne - pn, pnw - pnww];
    let v = [pw - pnw, pn - pnn, pne - pnne, pnw - pnnw];
    let (hmax, hmin) = (h.iter().max().unwrap(), h.iter().min().unwrap());
    let (vmax, vmin) = (v.iter().max().unwrap(), v.iter().min().unwrap());
    if hmax - hmin > RANGE_MAX || vmax - vmin > RANGE_MAX {
        return None;
    }
    if h.iter().chain(v.iter()).any(|d| d.abs() > ABS_MAX) {
        return None;
    }
    let (sh, sv) = (h.iter().sum::<i32>(), v.iter().sum::<i32>());
    if sh.abs() + sv.abs() < SLOPE_MIN {
        return None;
    }
    Some(((4 * (pw + pn + pnw + pne) + sh + 3 * sv + 8) >> 4).clamp(0, 255))
}

/// Research diagnostics over a fully known G plane (decoder-identical state): not coded-bit savings.
#[derive(Default, Clone, Debug)]
pub struct RampDiag {
    pub samples: u64,
    pub ramp: u64,
    pub differs: u64, // ramp samples where planar != MED
    pub zero_med: u64,
    pub zero_planar: u64,
    pub abs_med: u64,
    pub abs_planar: u64,
}
pub fn diagnose(p: &[u8], w: usize, h: usize) -> RampDiag {
    let mut d = RampDiag::default();
    for y in 0..h {
        for x in 0..w {
            d.samples += 1;
            if let Some(pl) = ramp_predict(p, w, x, y) {
                let med = crate::pkon2::predict(crate::pkon2::Pred::Med, crate::pkon2::neighbors(p, w, x, y));
                let v = p[y * w + x] as i32;
                d.ramp += 1;
                d.differs += (pl != med) as u64;
                d.zero_med += (v == med) as u64;
                d.zero_planar += (v == pl) as u64;
                d.abs_med += (v - med).unsigned_abs() as u64;
                d.abs_planar += (v - pl).unsigned_abs() as u64;
            }
        }
    }
    d
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pkon2::{neighbors, predict, Pred};

    fn lcg(s: &mut u64) -> u8 {
        *s = s.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        (*s >> 33) as u8
    }

    /// Encoder view (everything visible) vs decoder view (future pixels poisoned, filled as reconstructed): identical decisions and predictions.
    #[test]
    fn enc_dec_identical_and_causal() {
        for (w, h) in [(1, 1), (2, 9), (3, 3), (4, 4), (5, 5), (16, 12), (70, 50)] {
            for seed in 1..4u64 {
                let mut s = seed;
                // ramps with mild noise plus an edge region so both outcomes occur
                let img: Vec<u8> = (0..w * h)
                    .map(|i| {
                        let (x, y) = (i % w, i / w);
                        let base = 20 + x * 2 + y * 3 + if x > w / 2 && y > h / 2 { 90 } else { 0 };
                        (base as u8).wrapping_add(if lcg(&mut s) & 15 == 0 { 1 } else { 0 })
                    })
                    .collect();
                let mut dec = vec![0xA5u8; w * h];
                let mut dec2 = vec![0x5Au8; w * h];
                for y in 0..h {
                    for x in 0..w {
                        let i = y * w + x;
                        let (a, b, c) = (ramp_predict(&img, w, x, y), ramp_predict(&dec, w, x, y), ramp_predict(&dec2, w, x, y));
                        assert_eq!(a, b, "decision/prediction mismatch {w}x{h} ({x},{y})");
                        assert_eq!(a, c, "poison-dependent {w}x{h} ({x},{y})");
                        let med = predict(Pred::Med, neighbors(&img, w, x, y));
                        let pe = a.unwrap_or(med);
                        let pd = b.unwrap_or_else(|| predict(Pred::Med, neighbors(&dec, w, x, y)));
                        let pd2 = c.unwrap_or_else(|| predict(Pred::Med, neighbors(&dec2, w, x, y)));
                        assert_eq!(pe, pd);
                        assert_eq!(pe, pd2);
                        dec[i] = (pd as u8).wrapping_add(img[i].wrapping_sub(pe as u8));
                        dec2[i] = (pd2 as u8).wrapping_add(img[i].wrapping_sub(pe as u8));
                    }
                }
                assert_eq!(dec, img);
            }
        }
    }

    #[test]
    fn detector_cases() {
        let w = 8;
        let mk = |f: &dyn Fn(usize, usize) -> i32| -> Vec<u8> { (0..w * w).map(|i| f(i % w, i / w).clamp(0, 255) as u8).collect() };
        // exact linear ramp (both gradients same sign: MED over-shoots by min(gx,gy), planar is exact)
        let ramp = mk(&|x, y| 50 + 2 * x as i32 + 3 * y as i32);
        let r = ramp_predict(&ramp, w, 4, 4).expect("ramp accepted");
        assert_eq!(r, ramp[4 * w + 4] as i32);
        assert_ne!(predict(Pred::Med, neighbors(&ramp, w, 4, 4)), r);
        // boundaries -> None
        assert_eq!(ramp_predict(&ramp, w, 1, 4), None);
        assert_eq!(ramp_predict(&ramp, w, 4, 1), None);
        assert_eq!(ramp_predict(&ramp, w, w - 1, 4), None);
        // flat -> None (no slope evidence)
        assert_eq!(ramp_predict(&mk(&|_, _| 100), w, 4, 4), None);
        // abrupt edge in the neighbourhood -> None
        assert_eq!(ramp_predict(&mk(&|x, y| 50 + 2 * y as i32 + if x >= 4 { 60 } else { 0 }), w, 4, 4), None);
        // gradient steeper than the edge guard -> None
        assert_eq!(ramp_predict(&mk(&|x, y| 20 + 12 * x as i32 + y as i32), w, 4, 4), None);
        // inconsistent (texture / checkerboard) -> None
        assert_eq!(ramp_predict(&mk(&|x, y| 100 + if (x + y) % 2 == 0 { 0 } else { 4 }), w, 4, 4), None);
        // slope one pixel/step on x only: accepted, exact
        let hx = mk(&|x, _| 30 + x as i32);
        assert_eq!(ramp_predict(&hx, w, 4, 4), Some(hx[4 * w + 4] as i32));
        // result always in range
        let hi = mk(&|x, y| 240 + (x + y) as i32);
        if let Some(v) = ramp_predict(&hi, w, 5, 5) {
            assert!((0..=255).contains(&v));
        }
    }
}
