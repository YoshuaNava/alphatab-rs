use alphatab_rs::{guitar_pro::convert_track, Fret};
use guitarpro::{Beat, BeatStatus, Measure, MeasureHeader, Note, NoteType, Song, Track, Voice};

#[test]
fn uses_explicit_beat_offsets_and_imports_written_pitch() {
    const FRET: i16 = 3;
    const MIDI: u8 = 67;
    const ONSET_TICKS: i64 = 1920;
    const ONSET_QUARTERS: f64 = 1.0;
    let track = Track {
        strings: vec![(1, 64)],
        measures: vec![Measure {
            voices: vec![Voice {
                beats: vec![
                    Beat {
                        status: BeatStatus::Empty,
                        ..Default::default()
                    },
                    Beat {
                        start: Some(ONSET_TICKS),
                        notes: vec![Note {
                            string: 1,
                            value: FRET,
                            kind: NoteType::Normal,
                            ..Default::default()
                        }],
                        ..Default::default()
                    },
                ],
                ..Default::default()
            }],
            ..Default::default()
        }],
        ..Default::default()
    };
    let song = Song {
        measure_headers: vec![MeasureHeader::default()],
        ..Default::default()
    };
    let imported = convert_track(&song, &track).unwrap();
    let beats = &imported.track.measures[0].voices[0];
    assert_eq!(beats.len(), 1);
    assert_eq!(beats[0].start, Some(ONSET_QUARTERS));
    assert_eq!(beats[0].notes[0].fret, Fret::Number(FRET as u16));
    assert_eq!(
        beats[0].notes[0].pitch,
        Some(alphatab_rs::Pitch::from_midi(MIDI, false))
    );
}

#[test]
fn imports_percussion_clef() {
    let track = Track {
        strings: vec![(1, 35)],
        percussion_track: true,
        ..Default::default()
    };
    let imported = convert_track(&Song::default(), &track).unwrap();
    assert_eq!(imported.track.clef, alphatab_rs::Clef::Percussion);
}

#[test]
fn imports_slides_whammy_strokes_and_written_transposition() {
    use guitarpro::model::legacy::effects::{BendEffect, BendPoint};
    const WRITTEN_MIDI: u8 = 79;
    let mut note = Note {
        string: 1,
        value: 3,
        kind: NoteType::Normal,
        ..Default::default()
    };
    note.effect.slides = vec![
        guitarpro::SlideType::IntoFromBelow,
        guitarpro::SlideType::OutDownwards,
    ];
    let mut beat = Beat {
        notes: vec![note],
        ..Default::default()
    };
    beat.effect.tremolo_bar = Some(BendEffect {
        points: vec![
            BendPoint {
                position: 0,
                value: 0,
                vibrato: false,
            },
            BendPoint {
                position: 12,
                value: -2,
                vibrato: false,
            },
        ],
        ..Default::default()
    });
    beat.effect.stroke.direction = guitarpro::BeatStrokeDirection::Up;
    beat.effect.slap_effect = guitarpro::SlapEffect::Tapping;
    let track = Track {
        strings: vec![(1, 64)],
        transpose_octave: -1,
        measures: vec![Measure {
            voices: vec![Voice {
                beats: vec![beat],
                ..Default::default()
            }],
            ..Default::default()
        }],
        ..Default::default()
    };
    let song = Song {
        measure_headers: vec![MeasureHeader {
            repeat_close: 2,
            ..Default::default()
        }],
        ..Default::default()
    };
    let report = convert_track(&song, &track).unwrap();
    let m = &report.track.measures[0];
    let b = &m.voices[0][0];
    let n = &b.notes[0];
    assert_eq!(
        n.pitch,
        Some(alphatab_rs::Pitch::from_midi(WRITTEN_MIDI, false))
    );
    assert_eq!(n.effects.slide_in, Some(alphatab_rs::SlideDirection::Up));
    assert_eq!(n.effects.slide_out, Some(alphatab_rs::SlideDirection::Down));
    assert_eq!(b.annotations.whammy, vec![[0.0, 0.0], [1.0, -2.0]]);
    assert_eq!(b.annotations.strum_up, Some(true));
    assert_eq!(
        b.annotations.technique,
        Some(alphatab_rs::PluckingTechnique::Tap)
    );
    assert_eq!(m.repeat_count, Some(3));
    assert!(!report.warnings.iter().any(|w| w.contains("Slide-in/out")));
}

#[test]
fn imports_metadata_structure_grace_harmonics_and_chord_fingerings() {
    use guitarpro::model::legacy::chord::Chord;
    use guitarpro::model::legacy::effects::{GraceEffect, HarmonicEffect};
    use guitarpro::{Fingering, GraceEffectTransition, HarmonicType};
    const HARMONIC_MIDI: u8 = 88;
    const CAPO: i32 = 0;
    let mut note = Note {
        string: 1,
        value: 5,
        kind: NoteType::Normal,
        ..Default::default()
    };
    note.effect.harmonic = Some(HarmonicEffect {
        kind: HarmonicType::Natural,
        ..Default::default()
    });
    note.effect.right_hand_finger = Fingering::Index;
    note.effect.left_hand_finger = Fingering::Middle;
    note.effect.grace = Some(GraceEffect {
        fret: 3,
        is_dead: true,
        transition: GraceEffectTransition::Bend,
        ..Default::default()
    });
    let mut beat = Beat {
        notes: vec![note],
        ..Default::default()
    };
    beat.effect.has_rasgueado = true;
    beat.effect.chord = Some(Chord {
        name: "Wide".into(),
        strings: vec![1, 9],
        fingerings: vec![Fingering::Index, Fingering::Little],
        show: Some(true),
        ..Default::default()
    });
    let source = Track {
        number: 1,
        offset: CAPO,
        strings: vec![(1, 64), (2, 59)],
        measures: vec![
            Measure {
                voices: vec![Voice {
                    beats: vec![beat],
                    ..Default::default()
                }],
                line_break: guitarpro::LineBreak::Break,
                ..Default::default()
            },
            Measure {
                simile_mark: Some("Simple".into()),
                ..Default::default()
            },
        ],
        ..Default::default()
    };
    let song = Song {
        name: "Composition".into(),
        author: "Composer".into(),
        measure_headers: vec![
            MeasureHeader {
                free_time: true,
                double_bar: true,
                triplet_feel: guitarpro::TripletFeel::Eighth,
                ..Default::default()
            },
            MeasureHeader::default(),
        ],
        ..Default::default()
    };
    let result = convert_track(&song, &source).unwrap();
    let m = &result.track.measures[0];
    let b = &m.voices[0][0];
    assert_eq!(b.notes[0].pitch.unwrap().to_midi().unwrap(), HARMONIC_MIDI);
    assert_eq!(b.notes[0].effects.right_fingering.as_deref(), Some("i"));
    assert_eq!(b.notes[0].effects.fingering.as_deref(), Some("2"));
    assert!(b.notes[0].effects.grace_dead && b.notes[0].effects.grace_bend);
    assert!(b.annotations.rasgueado);
    assert_eq!(b.annotations.chord.as_ref().unwrap().fingers, ["1", "4"]);
    assert!(m.free_time && m.double_bar);
    assert!(result.track.measures[1].break_before);
    assert_eq!(
        result.track.measures[1].simile,
        Some(alphatab_rs::Simile::Single)
    );
    assert_eq!(result.track.metadata.title, song.name);
    assert!(result.warnings.is_empty(), "{:?}", result.warnings);
}

#[test]
fn imports_private_beam_hints_through_guitarpro_public_conversion() {
    use guitarpro::convert::{
        legacy::loaded_score_to_legacy_song, optimized::legacy::legacy_song_to_loaded_score,
    };
    use guitarpro::model::optimized::global::MeasureIndex;
    const FLAGS: i16 = 0x0001 | 0x0002 | 0x0800;
    const SECONDARY: u8 = 2;
    let source = Track {
        strings: vec![(1, 64)],
        measures: vec![Measure {
            voices: vec![Voice {
                beats: vec![Beat {
                    notes: vec![Note {
                        string: 1,
                        kind: NoteType::Normal,
                        ..Default::default()
                    }],
                    ..Default::default()
                }],
                ..Default::default()
            }],
            ..Default::default()
        }],
        ..Default::default()
    };
    let mut song = Song {
        tracks: vec![source],
        measure_headers: vec![MeasureHeader::default()],
        ..Default::default()
    };
    let mut optimized = legacy_song_to_loaded_score(&song);
    let b = &mut optimized.score.tracks[0]
        .measures
        .get_mut(&MeasureIndex(0))
        .unwrap()
        .voices
        .get_mut(&0)
        .unwrap()
        .beats[0];
    b.gp_beat_flags2 = Some(FLAGS);
    b.gp_break_secondary = Some(SECONDARY);
    song = loaded_score_to_legacy_song(&optimized);
    let result = convert_track(&song, &song.tracks[0]).unwrap();
    let a = &result.track.measures[0].voices[0][0].annotations;
    assert_eq!(a.beaming, alphatab_rs::Beaming::Break);
    assert_eq!(a.stem, alphatab_rs::StemDirection::Down);
    assert_eq!(a.break_secondary, SECONDARY);
}

#[test]
fn keeps_empty_voice_identity_and_reports_unknown_effect_values() {
    let mut n = Note {
        string: 1,
        kind: NoteType::Normal,
        ..Default::default()
    };
    n.effect.ornament = Some("UnknownOrnament".into());
    n.effect.right_hand_finger = guitarpro::Fingering::Unknown(12);
    let source = Track {
        strings: vec![(1, 64)],
        measures: vec![Measure {
            voices: vec![
                Voice::default(),
                Voice {
                    beats: vec![Beat {
                        notes: vec![n],
                        ..Default::default()
                    }],
                    ..Default::default()
                },
            ],
            ..Default::default()
        }],
        ..Default::default()
    };
    let song = Song {
        measure_headers: vec![MeasureHeader::default()],
        ..Default::default()
    };
    let report = convert_track(&song, &source).unwrap();
    assert!(report.track.measures[0].voices[0].is_empty());
    assert_eq!(report.track.measures[0].voices[1].len(), 1);
    assert!(report
        .warnings
        .iter()
        .any(|w| w.contains("UnknownOrnament")));
    assert!(report.warnings.iter().any(|w| w.contains("12")));
}

#[test]
fn artificial_harmonics_and_prebends_have_written_pitches() {
    use guitarpro::model::legacy::effects::{BendEffect, BendPoint, HarmonicEffect};
    const ARTIFICIAL_MIDI: u8 = 79;
    const PREBEND_MIDI: u8 = 69;
    let mut harmonic = Note {
        string: 1,
        value: 3,
        kind: NoteType::Normal,
        ..Default::default()
    };
    harmonic.effect.harmonic = Some(HarmonicEffect {
        kind: guitarpro::HarmonicType::Artificial,
        ..Default::default()
    });
    let mut bend = harmonic.clone();
    bend.effect.harmonic = None;
    bend.effect.bend = Some(BendEffect {
        points: vec![
            BendPoint {
                position: 0,
                value: 2,
                vibrato: false,
            },
            BendPoint {
                position: 12,
                value: 0,
                vibrato: true,
            },
        ],
        ..Default::default()
    });
    let source = Track {
        strings: vec![(1, 64)],
        measures: vec![Measure {
            voices: vec![Voice {
                beats: vec![
                    Beat {
                        notes: vec![harmonic],
                        ..Default::default()
                    },
                    Beat {
                        notes: vec![bend],
                        ..Default::default()
                    },
                ],
                ..Default::default()
            }],
            ..Default::default()
        }],
        ..Default::default()
    };
    let report = convert_track(
        &Song {
            measure_headers: vec![MeasureHeader::default()],
            ..Default::default()
        },
        &source,
    )
    .unwrap();
    let b = &report.track.measures[0].voices[0];
    assert_eq!(
        b[0].notes[0]
            .effects
            .harmonic_pitch
            .unwrap()
            .to_midi()
            .unwrap(),
        ARTIFICIAL_MIDI
    );
    assert_eq!(
        b[1].notes[0].pitch.unwrap().to_midi().unwrap(),
        PREBEND_MIDI
    );
    assert!(b[1].notes[0].effects.bend_vibrato);
    alphatab_rs::layout(
        &report.track,
        alphatab_rs::LayoutOptions {
            display: alphatab_rs::DisplayMode::Both,
            ..Default::default()
        },
    )
    .unwrap();
}
