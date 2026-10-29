use nanoscope::lisa::*;
use nanoscope::lisa_pcq::*;

fn image(w: usize, h: usize, bits: u8) -> Vec<u16> {
    let maxv = (1u32 << bits) - 1;
    let mut s = 987u32;
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

fn s3_of(samples: &[u16], meta: ImageMetadata, bits: u8) -> Stage3Output {
    let (w, h) = (meta.width, meta.height);
    classify_stage3(organize_stage2(encode_with_metadata(samples, w, h, bits, meta).unwrap()).unwrap()).unwrap()
}

fn check(samples: &[u16], meta: ImageMetadata, bits: u8) {
    let s3 = s3_of(samples, meta, bits);

    // Production unchanged and the experimental coder is a faithful twin: baseline classifier == encode_stage4 bytes.
    let prod = encode_stage4(s3.clone()).unwrap();
    let base = |l, a, al| classify_activity(l, a, al).index();
    let (ctrl, ctrl_ids) = pcq_encode(&s3, 4, base);
    assert_eq!(ctrl, prod.payload());
    let (res, dec_ids, end) = pcq_decode(&ctrl, &s3, 4, base);
    assert!(end && res == *s3.data() && dec_ids == ctrl_ids);

    // Frozen PCQ: lossless, decoder rebuilds identical context ids from decoded values only, deterministic bytes.
    let (bytes, ids) = pcq_encode(&s3, FROZEN_K, frozen_ctx);
    let (res, dec_ids, end) = pcq_decode(&bytes, &s3, FROZEN_K, frozen_ctx);
    assert!(end, "payload not fully consumed");
    assert_eq!(res, *s3.data(), "decoded residuals differ");
    assert_eq!(ids, dec_ids, "encoder/decoder context ids differ");
    assert!(ids.iter().all(|&c| (c as usize) < FROZEN_K));
    assert_eq!(pcq_encode(&s3, FROZEN_K, frozen_ctx).0, bytes, "non-deterministic output");

    // Residuals -> samples through the unchanged production inverse chain.
    let (s1, _) = restore_stage1(&s3.clone().into_stage2()).unwrap();
    assert_eq!(decode_with_metadata(&s1).unwrap(), samples);
}

#[test]
fn pcq_roundtrip_bayer_mono_and_depths() {
    let cfa = CfaPattern {
        width: 2,
        height: 2,
        entries: vec![ColorFilter::Red, ColorFilter::Green, ColorFilter::Green, ColorFilter::Blue],
        origin_x: 0,
        origin_y: 0,
    };
    let bayer = ImageMetadata::with_cfa(23, 17, SampleLayout::BayerCfa, cfa);
    check(&image(23, 17, 12), bayer.clone(), 12);
    check(&image(23, 17, 14), bayer, 14);
    check(&image(19, 14, 8), ImageMetadata::without_cfa(19, 14, SampleLayout::Monochrome), 8);
    check(&image(19, 14, 16), ImageMetadata::without_cfa(19, 14, SampleLayout::Monochrome), 16);
}

#[test]
fn key_and_frozen_table_are_total_and_in_range() {
    let vals = [0u32, 1, 2, 3, 4, 5, 6, 7, 11, 12, 48, 1023, 1535, 65535, 131070, u32::MAX];
    let opts: Vec<Option<u32>> = std::iter::once(None).chain(vals.iter().map(|&v| Some(v))).collect();
    for &l in &opts {
        for &a in &opts {
            for &al in &opts {
                assert!(key(l, a, al) < KEYS);
                assert!(frozen_ctx(l, a, al) < FROZEN_K);
            }
        }
    }
    // Every table entry is a valid context id and every context id is used.
    let t = &nanoscope::lisa_pcq_table::PCQ_TABLE;
    assert!(t.iter().all(|&c| (c as usize) < FROZEN_K));
    for c in 0..FROZEN_K {
        assert!(t.iter().any(|&x| x as usize == c), "context {c} unused");
    }
    // Levels are monotone in magnitude.
    assert!((0..200_000u32).all(|v| level(v) <= level(v + 1)));
}
