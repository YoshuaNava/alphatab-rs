use alphatab_rs::*;

const PAGE_WIDTH: f32 = 650.0;
fn note(midi: u8) -> Note {
    Note {
        string: 1,
        fret: Fret::Number(u16::from(midi.saturating_sub(60))),
        pitch: Some(Pitch::from_midi(midi, false)),
        ..Default::default()
    }
}
fn eighth(midi: u8) -> Beat {
    Beat {
        duration: Duration {
            value: 8,
            ..Default::default()
        },
        notes: vec![note(midi)],
        ..Default::default()
    }
}
fn track(measures: usize) -> Track {
    Track {
        strings: vec!["E".into()],
        measures: (0..measures)
            .map(|_| Measure {
                voices: vec![vec![eighth(64), eighth(65), eighth(67), eighth(69)]],
                ..Default::default()
            })
            .collect(),
        ..Default::default()
    }
}
fn address(measure: usize, beat: usize) -> BeatAddress {
    BeatAddress {
        measure,
        voice: 0,
        beat,
    }
}
fn texts(page: &Scene, expected: &str) -> usize {
    page.primitives
        .iter()
        .filter(|p| matches!(p, Primitive::Text { text, .. } if text == expected))
        .count()
}
fn glyphs(page: &Scene, expected: u32) -> usize {
    page.primitives
        .iter()
        .filter(|p| matches!(p, Primitive::Glyph { code, .. } if u32::from(*code) == expected))
        .count()
}

#[test]
fn spans_continue_across_systems_and_group_effects() {
    const SYSTEMS: usize = 3;
    let mut t = track(SYSTEMS);
    for m in &mut t.measures {
        for b in &mut m.voices[0] {
            b.notes[0].effects.palm_mute = true;
        }
    }
    t.spans.push(Span {
        start: address(0, 0),
        end: address(2, 3),
        kind: SpanKind::Crescendo,
        placement: Placement::Above,
    });
    t.spans.push(Span {
        start: address(0, 0),
        end: address(2, 3),
        kind: SpanKind::Slur {
            start_note: Some(0),
            end_note: Some(0),
        },
        placement: Placement::Above,
    });
    let p = engrave(
        &t,
        SceneOptions {
            width: PAGE_WIDTH,
            bars_per_system: Some(1),
            display: DisplayMode::Both,
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(p.systems.len(), SYSTEMS);
    assert_eq!(texts(&p, "P.M."), 1);
    assert_eq!(texts(&p, "(P.M.)"), SYSTEMS - 1);
    for primitive in &p.primitives {
        match primitive {
            Primitive::Line { from, to, .. } => {
                assert!(from.iter().chain(to).all(|v| v.is_finite()))
            }
            Primitive::Curve { points, .. } => {
                assert!(points.iter().flatten().all(|v| v.is_finite()))
            }
            Primitive::Text { at, .. } | Primitive::Glyph { at, .. } => {
                assert!(at.iter().all(|v| v.is_finite()))
            }
        }
    }
}

#[test]
fn validates_explicit_connections() {
    let mut t = track(1);
    t.spans.push(Span {
        start: address(0, 0),
        end: address(9, 0),
        kind: SpanKind::LetRing,
        placement: Placement::Above,
    });
    assert!(engrave(&t, Default::default())
        .unwrap_err()
        .message()
        .contains("endpoint"));
    t.spans[0].end = address(0, 3);
    t.spans[0].kind = SpanKind::Slur {
        start_note: Some(5),
        end_note: None,
    };
    assert!(engrave(&t, Default::default())
        .unwrap_err()
        .message()
        .contains("notes"));
    t.spans[0].kind = SpanKind::Beam;
    t.measures[0].voices[0][1].notes.clear();
    assert!(engrave(&t, Default::default())
        .unwrap_err()
        .message()
        .contains("beam"));
}

#[test]
fn beams_can_cross_barlines_and_resume_after_breaks() {
    // Bravura recommends a beam thickness of 0.5 staff spaces; this renderer's
    // standard staff space is 10 layout units.
    const BEAM_WIDTH: f32 = 5.0;
    let mut t = track(2);
    t.spans.push(Span {
        start: address(0, 0),
        end: address(1, 3),
        kind: SpanKind::Beam,
        placement: Placement::Above,
    });
    let p = engrave(
        &t,
        SceneOptions {
            flow: LayoutMode::Horizontal,
            display: DisplayMode::Standard,
            ..Default::default()
        },
    )
    .unwrap();
    let left = p
        .beats
        .iter()
        .find(|b| b.measure == 0 && b.beat == 3)
        .unwrap();
    let right = p
        .beats
        .iter()
        .find(|b| b.measure == 1 && b.beat == 0)
        .unwrap();
    let x1 = (left.cursor_rect[0] + left.cursor_rect[2]) / 2.0 + 5.0;
    let x2 = (right.cursor_rect[0] + right.cursor_rect[2]) / 2.0 + 5.0;
    assert!(p.primitives.iter().any(|p| matches!(p, Primitive::Line { from,to,width, .. } if *width == BEAM_WIDTH && from[0] == x1 && to[0] == x2)));
    let wrapped = engrave(
        &t,
        SceneOptions {
            width: PAGE_WIDTH,
            bars_per_system: Some(1),
            display: DisplayMode::Standard,
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(wrapped.systems.len(), 2);
    assert!(
        wrapped
            .primitives
            .iter()
            .filter(|p| matches!(p, Primitive::Line { width,.. } if *width == BEAM_WIDTH))
            .count()
            >= 6
    );
}

#[test]
fn nested_tuplets_affect_onsets_and_render_in_standard_notation() {
    const RATIO: (u8, u8) = (3, 2);
    const DURATION: f64 = 2.0 / 9.0;
    let mut t = track(1);
    t.measures[0].voices[0] = (0..9)
        .map(|_| {
            let mut b = eighth(64);
            b.duration.tuplet = Some(RATIO);
            b.annotations.tuplets.push(RATIO);
            b.annotations.force_tuplet_bracket = true;
            b
        })
        .collect();
    let p = engrave(
        &t,
        SceneOptions {
            display: DisplayMode::Standard,
            ..Default::default()
        },
    )
    .unwrap();
    assert!((p.beats[1].start - DURATION).abs() < 1e-8);
    assert_eq!(texts(&p, "3"), 4);
    let first = &p.beats[0];
    let last = &p.beats[8];
    let left = (first.cursor_rect[0] + first.cursor_rect[2]) / 2.0 - 8.0;
    let right = (last.cursor_rect[0] + last.cursor_rect[2]) / 2.0 + 8.0;
    assert!(p.primitives.iter().any(|p| matches!(p,Primitive::Line { from,to,width, .. } if *width == 1.0 && from[0] == left && to[0] == right && from[1] == to[1])));
}

#[test]
fn custom_meter_groups_and_explicit_beam_breaks_change_geometry() {
    // Bravura recommends a beam thickness of 0.5 staff spaces; this renderer's
    // standard staff space is 10 layout units.
    const BEAM_WIDTH: f32 = 5.0;
    let mut t = track(1);
    t.measures[0].time_signature = (4, 8);
    t.measures[0].beam_groups = vec![3, 1];
    let options = SceneOptions {
        display: DisplayMode::Standard,
        ..Default::default()
    };
    let before = engrave(&t, options).unwrap();
    t.measures[0].voices[0][1].annotations.beaming = Beaming::Break;
    let after = engrave(&t, options).unwrap();
    let count = |p: &Scene| {
        p.primitives.iter().filter(|p| matches!(p,Primitive::Line { width,from,to, .. } if *width == BEAM_WIDTH && (to[0]-from[0]).abs()>10.0)).count()
    };
    assert!(count(&before) > count(&after));
    t.measures[0].beam_groups = vec![2];
    assert!(engrave(&t, options)
        .unwrap_err()
        .message()
        .contains("beam groups"));
}

#[test]
fn wide_chord_diagrams_keep_fingerings_and_frets() {
    const HIGH_FRET: u16 = 9;
    let mut t = track(1);
    t.strings = vec!["E".into(), "B".into(), "G".into(), "D".into()];
    t.measures[0].voices[0][0].annotations.chord = Some(ChordDiagram {
        name: "wide voicing".into(),
        first_fret: 1,
        frets: vec![Some(1), Some(HIGH_FRET), None, Some(0)],
        fingers: vec!["1".into(), "4".into(), String::new(), String::new()],
        ..Default::default()
    });
    let page = engrave(&t, Default::default()).unwrap();
    assert_eq!(texts(&page, "wide voicing"), 1);
    assert!(texts(&page, "4") >= 1);
    t.measures[0].voices[0][0]
        .annotations
        .chord
        .as_mut()
        .unwrap()
        .fingers
        .pop();
    assert!(engrave(&t, Default::default())
        .unwrap_err()
        .message()
        .contains("fingerings"));
}

#[test]
fn multi_measure_rests_condense_without_losing_addresses() {
    const COUNT: usize = 5;
    let mut t = track(COUNT);
    for m in &mut t.measures {
        m.voices = vec![vec![Beat {
            duration: Duration {
                value: 1,
                ..Default::default()
            },
            ..Default::default()
        }]];
    }
    let options = SceneOptions {
        multi_measure_rests: true,
        ..Default::default()
    };
    let page = engrave(&t, options).unwrap();
    assert_eq!(page.beats.len(), COUNT);
    assert_eq!(texts(&page, &COUNT.to_string()), 1);
    for (i, b) in page.beats.iter().enumerate() {
        assert_eq!(b.measure, i);
        assert_eq!(
            page.hit_test((b.rect[0] + b.rect[2]) / 2.0, (b.rect[1] + b.rect[3]) / 2.0)
                .unwrap()
                .measure,
            i
        );
    }
    t.measures[2].marker = "Rehearsal".into();
    let split = engrave(&t, options).unwrap();
    assert_eq!(texts(&split, "Rehearsal"), 1);
    assert!(split.beats[2].rect[0] > split.beats[1].rect[2]);
}

#[test]
fn score_tracks_support_independent_modes_meters_and_instrument_groups() {
    let upper = track(1);
    let mut lower = track(1);
    lower.name = "Lower".into();
    lower.clef = Clef::Bass;
    lower.measures[0].time_signature = (3, 4);
    let standard = SceneOptions {
        display: DisplayMode::Standard,
        ..Default::default()
    };
    let tabs = SceneOptions {
        display: DisplayMode::Tablature,
        ..Default::default()
    };
    let parts = vec![
        ScoreTrack {
            track: upper,
            options: standard,
        },
        ScoreTrack {
            track: lower,
            options: tabs,
        },
    ];
    let page = engrave_instruments(
        &parts,
        &[InstrumentGroup {
            name: "Piano".into(),
            tracks: 0..2,
            bracket: InstrumentBracket::Brace,
        }],
    )
    .unwrap();
    assert_eq!(page.beats.iter().filter(|b| b.track == 0).count(), 4);
    assert_eq!(page.beats.iter().filter(|b| b.track == 1).count(), 4);
    assert!(texts(&page.geometry, "Piano") >= 1);
}

#[test]
fn staff_aware_document_draws_cross_staff_beams_and_validates_master_bars() {
    let upper = Staff {
        track: track(1),
        options: SceneOptions {
            display: DisplayMode::Standard,
            ..Default::default()
        },
        master_bar_map: vec![],
    };
    let mut lower_track = track(1);
    lower_track.clef = Clef::Bass;
    let lower = Staff {
        track: lower_track,
        options: SceneOptions {
            display: DisplayMode::Standard,
            ..Default::default()
        },
        master_bar_map: vec![],
    };
    let mut document = ScoreDocument {
        master_bars: vec![MasterBar {
            start_quarters: 0.0,
            duration_quarters: 4.0,
        }],
        instruments: vec![Instrument {
            name: "Piano".into(),
            staves: vec![upper, lower],
            bracket: InstrumentBracket::Brace,
        }],
        cross_staff_spans: vec![CrossStaffSpan {
            start: ScoreBeatAddress {
                track: 0,
                beat: address(0, 0),
            },
            end: ScoreBeatAddress {
                track: 1,
                beat: address(0, 3),
            },
            kind: CrossStaffSpanKind::Beam,
        }],
    };
    let page = engrave_document(&document).unwrap();
    assert!(page
        .geometry
        .primitives
        .iter()
        .any(|primitive| matches!(primitive,
        Primitive::Line { width, .. } if *width == 5.0)));
    document.master_bars[0].duration_quarters = 0.0;
    assert!(engrave_document(&document)
        .unwrap_err()
        .message()
        .contains("master bars"));
}

#[test]
fn staff_measure_maps_align_differently_partitioned_parts() {
    let upper = Staff {
        track: track(2),
        options: SceneOptions::default(),
        master_bar_map: vec![],
    };
    let lower = Staff {
        track: track(1),
        options: SceneOptions::default(),
        master_bar_map: vec![1],
    };
    let document = ScoreDocument {
        master_bars: vec![
            MasterBar {
                start_quarters: 0.0,
                duration_quarters: 4.0,
            },
            MasterBar {
                start_quarters: 4.0,
                duration_quarters: 4.0,
            },
        ],
        instruments: vec![Instrument {
            name: "Mapped".into(),
            staves: vec![upper, lower],
            bracket: InstrumentBracket::Bracket,
        }],
        cross_staff_spans: vec![],
    };
    let page = engrave_document(&document).unwrap();
    assert_eq!(page.beats.iter().filter(|beat| beat.track == 0).count(), 8);
    assert_eq!(page.beats.iter().filter(|beat| beat.track == 1).count(), 4);
    assert!(page
        .beats
        .iter()
        .filter(|beat| beat.track == 1)
        .all(|beat| beat.beat.measure == 1));
}

#[test]
fn render_style_and_revision_cache_are_shared_by_svg_output() {
    let options = SceneOptions {
        style: RenderStyle {
            foreground: Color::opaque(10, 20, 30),
            background: Color::opaque(240, 241, 242),
            text: Some(Color::opaque(50, 60, 70)),
            ..Default::default()
        },
        ..Default::default()
    };
    let source = track(1);
    let mut cache = SceneCache::default();
    let first_scene = cache
        .scene_or_try_build(7, || engrave(&source, options))
        .unwrap();
    let first = SvgRenderer::new(first_scene).render();
    let second_scene = cache
        .scene_or_try_build(7, || {
            Err(RenderError::new(
                ErrorKind::Internal,
                "cache should be used",
            ))
        })
        .unwrap();
    let second = SvgRenderer::new(second_scene).render();
    assert_eq!(first, second);
    assert!(first.contains("rgb(10 20 30)"));
    assert!(first.contains("rgb(50 60 70)"));
    assert!(first.contains("rgb(240 241 242)"));
    let mut individually_styled = engrave(&source, options).unwrap();
    let text_index = individually_styled
        .primitives
        .iter()
        .position(|primitive| matches!(primitive, Primitive::Text { .. }))
        .unwrap();
    individually_styled
        .set_primitive_color(text_index, Some(Color::opaque(200, 10, 20)))
        .unwrap();
    assert!(SvgRenderer::new(&individually_styled).render().contains("rgb(200 10 20)"));
}

#[test]
fn element_visibility_can_hide_individual_metadata_and_score_labels() {
    let mut source = track(1);
    source.name = "Lead guitar".into();
    source.metadata.title = "Visible title".into();
    source.metadata.artist = "Hidden artist".into();
    source.measures[0].repeat_end = true;
    source.measures[0].repeat_count = Some(2);
    let page = engrave(
        &source,
        SceneOptions {
            elements: ElementVisibility {
                artist: false,
                track_names: false,
                repeat_counts: false,
                ..Default::default()
            },
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(texts(&page, "Visible title"), 1);
    assert_eq!(texts(&page, "Hidden artist"), 0);
    assert_eq!(texts(&page, "Lead guitar"), 0);
    assert_eq!(texts(&page, "2×"), 0);
}

#[test]
fn background_layout_returns_the_requested_revision() {
    const REVISION: u64 = 42;
    let worker = SceneWorker::new().unwrap();
    worker
        .request(REVISION, track(1), SceneOptions::default())
        .unwrap();
    let result = worker
        .receive_timeout(std::time::Duration::from_secs(2))
        .unwrap();
    assert_eq!(result.revision, REVISION);
    assert_eq!(result.scene.unwrap().beats.len(), 4);
}

#[test]
fn engraving_settings_apply_transposition_fingering_and_system_gap() {
    let mut source = track(2);
    source.measures[0].voices[0][0].notes[0].effects.fingering = Some("p".into());
    let page = engrave(
        &source,
        SceneOptions {
            bars_per_system: Some(1),
            display: DisplayMode::Standard,
            engraving: EngravingSettings {
                display_transposition: 12,
                fingering_mode: FingeringMode::Piano,
                system_gap: 80.0,
                ..Default::default()
            },
            elements: ElementVisibility {
                bar_numbers: false,
                ..Default::default()
            },
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(texts(&page, "1"), 1);
    assert_eq!(page.systems.len(), 2);
    assert!(page.systems[1][0] - page.systems[0][1] >= 0.0);
    let mut invalid = SceneOptions::default();
    invalid.engraving.slur_height = f32::NAN;
    assert!(engrave(&source, invalid).is_err());
}

#[test]
fn score_condenses_only_shared_rests_and_preserves_track_identity() {
    const COUNT: usize = 3;
    let mut t = track(COUNT);
    for m in &mut t.measures {
        for b in &mut m.voices[0] {
            b.notes.clear();
        }
    }
    let options = SceneOptions {
        multi_measure_rests: true,
        ..Default::default()
    };
    let page = engrave_score(&[t.clone(), t.clone()], options).unwrap();
    assert_eq!(page.beats.len(), COUNT * 4 * 2);
    assert!(page
        .beats
        .iter()
        .any(|b| b.track == 1 && b.beat.measure == COUNT - 1));
    let mut played = t.clone();
    played.measures[1].voices[0][0].notes = vec![note(64)];
    let uncondensed = engrave_score(&[t, played], options).unwrap();
    assert_eq!(uncondensed.beats.len(), page.beats.len());
}

#[test]
fn all_new_clefs_and_noteheads_have_outlines() {
    const CLEFS: [Clef; 16] = [
        Clef::Treble8Above,
        Clef::Treble8Below,
        Clef::Treble15Above,
        Clef::Treble15Below,
        Clef::Bass8Above,
        Clef::Bass8Below,
        Clef::Bass15Above,
        Clef::Bass15Below,
        Clef::Alto8Above,
        Clef::Alto8Below,
        Clef::Alto15Above,
        Clef::Alto15Below,
        Clef::Tenor8Above,
        Clef::Tenor8Below,
        Clef::Tenor15Above,
        Clef::Tenor15Below,
    ];
    const HEADS: [NoteHead; 6] = [
        NoteHead::Normal,
        NoteHead::Diamond,
        NoteHead::Cross,
        NoteHead::Slash,
        NoteHead::Triangle,
        NoteHead::CircleCross,
    ];
    let mut t = track(1);
    for clef in CLEFS {
        for head in HEADS {
            for value in [1, 2, 4] {
                t.clef = clef;
                t.measures[0].voices[0][0].notes[0].effects.head = head;
                t.measures[0].voices[0][0].duration.value = value;
                let page = engrave(
                    &t,
                    SceneOptions {
                        display: DisplayMode::Standard,
                        ..Default::default()
                    },
                )
                .unwrap();
                assert!(SvgRenderer::new(&page).render().contains("<path"));
            }
        }
    }
}

#[test]
fn simultaneous_conflicting_accidentals_are_both_explicit() {
    const SHARP: u32 = 0xE262;
    const NATURAL: u32 = 0xE261;
    let mut t = track(1);
    let a = eighth(66);
    let b = eighth(65);
    t.measures[0].voices = vec![vec![a], vec![b]];
    let page = engrave(
        &t,
        SceneOptions {
            display: DisplayMode::Standard,
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(glyphs(&page, SHARP), 1);
    assert_eq!(glyphs(&page, NATURAL), 1);
}

#[test]
fn key_spelling_handles_remote_keys() {
    const B_SHARP: Pitch = Pitch {
        step: 6,
        octave: 3,
        accidental: 1,
    };
    const C_FLAT: Pitch = Pitch {
        step: 0,
        octave: 4,
        accidental: -1,
    };
    assert_eq!(Pitch::spell_in_key(60, 7), B_SHARP);
    assert_eq!(Pitch::spell_in_key(59, -7), C_FLAT);
}

#[test]
fn metadata_is_measured_and_visibility_controls_apply() {
    let mut t = track(1);
    t.metadata = ScoreMetadata {
        title: "Title & author".into(),
        music: "Composer".into(),
        ..Default::default()
    };
    t.measures[0].voices[0][0].annotations.lyrics = "A verse".into();
    let shown = engrave(&t, Default::default()).unwrap();
    let hidden = engrave(
        &t,
        SceneOptions {
            show_metadata: false,
            show_lyrics: false,
            show_bar_numbers: false,
            tab_rhythm: TabRhythm::Hidden,
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(texts(&shown, "Title & author"), 1);
    assert_eq!(texts(&hidden, "Title & author"), 0);
    assert_eq!(texts(&hidden, "A verse"), 0);
    assert!(shown.height > hidden.height);
}

#[test]
fn renders_similes_grace_details_and_effect_symbols() {
    const SIMILE: u32 = 0xE500;
    let mut t = track(2);
    t.measures[1].simile = Some(Simile::Single);
    let b = &mut t.measures[0].voices[0][0];
    b.notes[0].effects.grace_dead = true;
    b.notes[0].effects.grace_pitch = Some(Pitch::from_midi(62, false));
    b.notes[0].effects.grace_fret = Some(0);
    b.notes[0].effects.grace_bend = true;
    b.annotations.golpe = true;
    b.annotations.wah_open = Some(true);
    b.annotations.pedal = Some(Pedal::Down);
    b.annotations.fade = Some(Fade::Swell);
    let p = engrave(
        &t,
        SceneOptions {
            display: DisplayMode::Both,
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(glyphs(&p, SIMILE), 1);
    assert!(glyphs(&p, 0xE0A9) > 0);
    assert_eq!(p.beats.len(), 8);
}

#[test]
fn numbered_and_slash_notation_render_without_guitar_strings() {
    let mut t = track(1);
    t.strings.clear();
    t.measures[0].voices[0][0].duration = Duration {
        value: 1,
        ..Default::default()
    };
    let numbered = engrave(
        &t,
        SceneOptions {
            display: DisplayMode::Numbered,
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(texts(&numbered, "1 = C"), 1);
    assert_eq!(texts(&numbered, "–"), 3);
    let slash = engrave(
        &t,
        SceneOptions {
            display: DisplayMode::Slash,
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(glyphs(&slash, 0xE102), 1);
    assert_eq!(glyphs(&slash, 0xE100), 3);
    assert_eq!(glyphs(&slash, 0xE050), 0);
}

#[test]
fn long_and_256th_durations_and_triple_dots_are_supported() {
    const BREVE_QUARTERS: f64 = 8.0;
    const LONGA_QUARTERS: f64 = 16.0;
    let mut t = track(1);
    t.measures[0].voices[0] = [-4, -2, 1, 256]
        .into_iter()
        .map(|value| Beat {
            duration: Duration {
                value,
                dots: 3,
                tuplet: None,
            },
            notes: vec![note(64)],
            ..Default::default()
        })
        .collect();
    assert_eq!(
        Duration {
            value: -2,
            ..Default::default()
        }
        .compute_quarter_beats()
        .unwrap(),
        BREVE_QUARTERS
    );
    assert_eq!(
        Duration {
            value: -4,
            ..Default::default()
        }
        .compute_quarter_beats()
        .unwrap(),
        LONGA_QUARTERS
    );
    for mode in [
        DisplayMode::Tablature,
        DisplayMode::Standard,
        DisplayMode::Numbered,
    ] {
        engrave(
            &t,
            SceneOptions {
                display: mode,
                ..Default::default()
            },
        )
        .unwrap();
    }
}

#[test]
fn score_zoom_pages_selection_and_running_headers_keep_geometry() {
    const ZOOM: f32 = 1.5;
    let t = track(3);
    let score = engrave_score(
        &[t.clone(), t.clone()],
        SceneOptions {
            width: PAGE_WIDTH,
            bars_per_system: Some(1),
            ..Default::default()
        },
    )
    .unwrap();
    let scaled = score.scaled(ZOOM).unwrap();
    assert_eq!(
        scaled.beats[0].beat.rect[0],
        score.beats[0].beat.rect[0] * ZOOM
    );
    let height = score
        .geometry
        .systems
        .iter()
        .map(|s| s[1] - s[0])
        .fold(0.0, f32::max)
        + 1.0;
    let pages = score.paginate(height).unwrap();
    assert_eq!(
        pages.iter().map(|p| p.beats.len()).sum::<usize>(),
        score.beats.len()
    );
    let selection = ScoreSelection {
        anchor: ScoreBeatAddress {
            track: 0,
            beat: address(0, 0),
        },
        end: ScoreBeatAddress {
            track: 1,
            beat: address(1, 2),
        },
    };
    assert!(selection.contains(
        &score,
        ScoreBeatAddress {
            track: 1,
            beat: address(0, 2)
        }
    ));
    assert!(!selection.contains(
        &score,
        ScoreBeatAddress {
            track: 0,
            beat: address(2, 0)
        }
    ));
    let solo = engrave(
        &t,
        SceneOptions {
            width: PAGE_WIDTH,
            bars_per_system: Some(1),
            ..Default::default()
        },
    )
    .unwrap();
    let height = solo.systems.iter().map(|s| s[1] - s[0]).fold(0.0, f32::max) + 70.0;
    let pages = solo
        .paginate_with_headers(height, "Running title", "Copyright")
        .unwrap();
    assert_eq!(pages.len(), 3);
    assert!(pages
        .iter()
        .all(|p| texts(p, "Running title") == 1 && texts(p, "Copyright") == 1));
    assert_eq!(
        pages.iter().map(|p| p.beats.len()).sum::<usize>(),
        solo.beats.len()
    );
}

#[test]
fn hammer_pull_and_legato_slides_engrave_on_both_staves() {
    let mut t = track(1);
    t.measures[0].voices[0][0].notes[0].effects.hammer_on = true;
    t.measures[0].voices[0][1].notes[0].effects.slide = true;
    t.measures[0].voices[0][1].notes[0].effects.slide_legato = true;
    let p = engrave(
        &t,
        SceneOptions {
            display: DisplayMode::Both,
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(texts(&p, "H"), 2);
    assert_eq!(texts(&p, "sl."), 2);
}

#[test]
fn quarter_tone_accidentals_cancel_and_empty_diagrams_are_errors() {
    const QUARTER_SHARP: u32 = 0xE282;
    const NATURAL: u32 = 0xE261;
    let mut t = track(1);
    t.measures[0].voices[0] = vec![eighth(64), eighth(64)];
    t.measures[0].voices[0][0].notes[0].effects.quarter_tone = 1;
    let options = SceneOptions {
        display: DisplayMode::Standard,
        ..Default::default()
    };
    let p = engrave(&t, options).unwrap();
    assert_eq!(glyphs(&p, QUARTER_SHARP), 1);
    assert_eq!(glyphs(&p, NATURAL), 1);
    t.strings.clear();
    t.measures[0].voices[0][0].annotations.chord = Some(ChordDiagram {
        first_fret: 1,
        ..Default::default()
    });
    assert!(engrave(&t, options).is_err());
}
