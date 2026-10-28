use nanoscope::praw::*;

fn image(w: usize, h: usize, bits: u8) -> Vec<u16> {
    let maxv = (1u32 << bits) - 1;
    let mut s = 12345u32;
    let mut rnd = move || {
        s = s.wrapping_mul(1664525).wrapping_add(1013904223);
        s >> 16
    };
    (0..w * h)
        .map(|i| {
            let (x, y) = (i % w, i / w);
            let amp = match (x * 4 / w, y * 2 / h) {
                (0, _) => 0,
                (1, _) => 3,
                (2, _) => 24,
                _ => maxv,
            };
            ((maxv / 2) as i64 + (rnd() as i64 % (amp as i64 + 1)) - amp as i64 / 2).clamp(0, maxv as i64) as u16
        })
        .collect()
}

fn roundtrip(samples: &[u16], meta: ImageMetadata, bits: u8) {
    let (w, h) = (meta.width, meta.height);
    let s1 = encode_with_metadata(samples, w, h, bits, meta).unwrap();
    let s2 = organize_stage2(s1).unwrap();
    let s3 = classify_stage3(s2).unwrap();
    let classes: Vec<ActivityClass> = match s3.contexts() {
        OrganizedContexts::Morton(c) => c.clone(),
        OrganizedContexts::BayerPlanes(p) => p.iter().flatten().copied().collect(),
    };
    for k in 0..ActivityClass::COUNT {
        assert!(classes.iter().any(|c| c.index() == k), "class {k} never occurs");
    }

    let s4 = encode_stage4(s3.clone()).unwrap();
    assert!(matches!(s4.history().last(), Some(ReconstructionRecord::Stage4Entropy(_))));
    assert_eq!(&s4.history()[..s4.history().len() - 1], s3.history());

    let back3 = decode_stage4(&s4).unwrap();
    assert_eq!(back3, s3, "decoded Stage 3 differs");
    let s2b = back3.into_stage2();
    let (s1b, _) = restore_stage1(&s2b).unwrap();
    assert_eq!(decode_with_metadata(&s1b).unwrap(), samples, "samples differ");
}

#[test]
fn stage4_end_to_end() {
    let (w, h) = (23, 17); // odd dimensions -> uneven Bayer planes
    let cfa = CfaPattern {
        width: 2,
        height: 2,
        entries: vec![ColorFilter::Red, ColorFilter::Green, ColorFilter::Green, ColorFilter::Blue],
        origin_x: 0,
        origin_y: 0,
    };
    let bayer = ImageMetadata::with_cfa(w, h, SampleLayout::BayerCfa, cfa);
    roundtrip(&image(w, h, 12), bayer, 12);
    roundtrip(&image(19, 14, 10), ImageMetadata::without_cfa(19, 14, SampleLayout::Monochrome), 10);
}
