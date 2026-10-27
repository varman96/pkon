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
    for e in synthetic().iter().filter(|e| supported(&e.image.spec)) {
        for color in [Color::Raw, Color::SubG] {
            let o = measure(&e.image, Config { pred: Pred::Gap, color, ctx: Ctx::ActX });
            assert!(o.exact, "{} Gap/{color:?} did not round-trip", e.name);
            tested += 1;
        }
    }
    for e in synthetic().iter().filter(|e| supported(&e.image.spec)) {
        for ctx in [Ctx::Err, Ctx::ErrX, Ctx::ActErrX, Ctx::DummyX] {
            let o = measure(&e.image, Config { pred: Pred::Med, color: Color::SubG, ctx });
            assert!(o.exact, "{} SubG/{ctx:?} did not round-trip", e.name);
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

#[test]
fn binarized_mixing_coder_round_trips_exactly() {
    use nanoscope::pkon2x::*;
    let sets: Vec<XCfg> = vec![
        XCfg { name: "a", inputs: vec![In::A], sign_ctx: false, bias: Bias::Off, ens: Ens::Off },
        XCfg { name: "a+s", inputs: vec![In::A], sign_ctx: true, bias: Bias::Off, ens: Ens::Off },
        XCfg { name: "all", inputs: vec![In::A, In::B, In::C, In::D], sign_ctx: true, bias: Bias::Off, ens: Ens::Off },
    ];
    for e in synthetic().iter().filter(|e| supported(&e.image.spec)) {
        for c in &sets {
            let o = measure_x(&e.image, c);
            assert!(o.exact, "{} {} did not round-trip", e.name, c.name);
        }
    }
}

#[test]
fn bias_correction_round_trips_exactly() {
    use nanoscope::pkon2x::*;
    let inputs = vec![In::A, In::B, In::C, In::D];
    let mk = |bias| XCfg { name: "b", inputs: inputs.clone(), sign_ctx: true, bias, ens: Ens::Off };
    for e in synthetic().iter().filter(|e| supported(&e.image.spec)) {
        let off = measure_x(&e.image, &mk(Bias::Off));
        assert!(off.exact);
        for b in [Bias::Nb, Bias::NbPrev, Bias::NbBoth, Bias::NbTrunc, Bias::NbBothTrunc] {
            let o = measure_x(&e.image, &mk(b));
            assert!(o.exact, "{} {b:?} did not round-trip", e.name);
        }
    }
}

#[test]
fn weighted_ensemble_round_trips_exactly() {
    use nanoscope::pkon2x::*;
    let mk = |ens| XCfg { name: "e", inputs: vec![In::A, In::B, In::C, In::D], sign_ctx: true, bias: Bias::Off, ens };
    for e in synthetic().iter().filter(|e| supported(&e.image.spec)) {
        for en in [Ens::Three, Ens::Two, Ens::Select3, Ens::Snap3, Ens::Sq3] {
            let o = measure_x(&e.image, &mk(en));
            assert!(o.exact, "{} {en:?} did not round-trip", e.name);
        }
    }
}
