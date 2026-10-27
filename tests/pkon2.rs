//! Focused gate for the PKON2 probe coder: every config must reproduce odd-sized, gray, RGB and
//! RGBA synthetic images bit-exactly (including noise, which forces the full residual range).
use nanoscope::corpus::synthetic;
use nanoscope::pkon2::*;

#[test]
fn every_config_round_trips_exactly() {
    let mut tested = 0;
    for e in synthetic().iter().filter(|e| supported(&e.image.spec)) {
        for &pred in &PREDS {
            for &color in &[Color::Raw, Color::SubG, Color::Coupled] {
                for &ctx in &[Ctx::None, Ctx::Act, Ctx::ActX] {
                    let o = measure(&e.image, Config { pred, color, ctx });
                    assert!(o.exact, "{} {pred:?}/{color:?}/{ctx:?} did not round-trip", e.name);
                    tested += 1;
                }
            }
        }
    }
    for e in synthetic().iter().filter(|e| supported(&e.image.spec)) {
        for win in [2u8, 4, 6] {
            let o = measure(&e.image, Config { pred: Pred::Med, color: Color::Adaptive(win), ctx: Ctx::ActX });
            assert!(o.exact, "{} Adaptive({win}) did not round-trip", e.name);
            tested += 1;
        }
    }
    assert!(tested > 100, "only {tested} combos ran");
}

#[test]
fn crop_keeps_exact_pixels() {
    let e = synthetic().into_iter().find(|e| e.name == "odd_rgb").unwrap();
    let c = center_crop(&e.image, 8);
    let (x0, y0) = ((e.image.spec.width - 8) / 2, (e.image.spec.height - 8) / 2);
    for y in 0..8 {
        for x in 0..8 {
            for ch in 0..3 {
                assert_eq!(c.sample(x, y, ch), e.image.sample(x0 + x, y0 + y, ch));
            }
        }
    }
}
