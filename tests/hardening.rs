use alphatab_rs::*;
use proptest::prelude::*;

proptest! {
    #[test]
    fn valid_durations_are_finite_and_positive(
        value in prop::sample::select(vec![1_i16, 2, 4, 8, 16, 32, 64, 128, 256]),
        dots in 0_u8..=3,
    ) {
        let duration = Duration { value, dots, tuplet: None };
        let quarters = duration.quarter_beats().unwrap();
        prop_assert!(quarters.is_finite() && quarters > 0.0);
    }

    #[test]
    fn layout_geometry_remains_finite_and_preserves_addresses(
        frets in prop::collection::vec(0_u16..=36, 1..64),
    ) {
        let beats = frets
            .iter()
            .map(|fret| Beat::with_notes(
                Duration { value: 8, ..Duration::default() },
                [Note { string: 1, fret: Fret::Number(*fret), ..Note::default() }],
            ))
            .collect();
        let track = Track {
            strings: vec!["E".into()],
            measures: vec![Measure { voices: vec![beats], ..Measure::default() }],
            ..Track::default()
        };
        let page = layout(&track, LayoutOptions::default()).unwrap();
        prop_assert_eq!(page.beats.len(), frets.len());
        prop_assert!(page.width.is_finite() && page.height.is_finite());
        for primitive in page.primitives {
            match primitive {
                Primitive::Glyph { at, space, .. } => {
                    prop_assert!(at.into_iter().all(f32::is_finite) && space.is_finite());
                }
                Primitive::Line { from, to, width, .. } => {
                    prop_assert!(from.into_iter().chain(to).all(f32::is_finite));
                    prop_assert!(width.is_finite());
                }
                Primitive::Curve { points, width, .. } => {
                    prop_assert!(points.into_iter().flatten().all(f32::is_finite));
                    prop_assert!(width.is_finite());
                }
                Primitive::Text { at, size, .. } => {
                    prop_assert!(at.into_iter().all(f32::is_finite) && size.is_finite());
                }
            }
        }
    }
}

#[test]
fn errors_have_stable_categories() {
    let error = QuarterTime::new(f64::NAN).unwrap_err();
    assert_eq!(error.kind(), ErrorKind::InvalidInput);

    let error = LayoutWorker::new()
        .unwrap()
        .receive_timeout(std::time::Duration::from_millis(1))
        .unwrap_err();
    assert_eq!(error.kind(), ErrorKind::Worker);
}

#[test]
fn worker_coalesces_and_shuts_down_without_panicking() {
    let worker = LayoutWorker::new().unwrap();
    for revision in 1..=32 {
        worker
            .request(revision, Track::default(), LayoutOptions::default())
            .unwrap();
    }
    let result = worker
        .receive_timeout(std::time::Duration::from_secs(2))
        .unwrap();
    assert!((1..=32).contains(&result.revision));
    worker.shutdown().unwrap();
}

#[test]
fn full_validation_rejects_layout_specific_input() {
    let track = Track::default();
    assert!(track.validate().is_ok());
    assert!(track.validate_for(LayoutOptions::default()).is_err());
}

#[test]
fn validation_enforces_curve_resource_limits() {
    let mut track = Track::default();
    track.measures.push(Measure {
        voices: vec![vec![Beat {
            annotations: BeatAnnotations {
                whammy: vec![[0.0, 0.0]; MAX_CURVE_POINTS + 1],
                ..BeatAnnotations::default()
            },
            ..Beat::default()
        }]],
        ..Measure::default()
    });
    let error = track.validate().unwrap_err();
    assert_eq!(error.kind(), ErrorKind::ResourceLimit);
}
