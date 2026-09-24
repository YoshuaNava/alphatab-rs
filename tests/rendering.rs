use alphatab_rs::*;

fn track() -> Track {
    Track {
        name: "Tabs <&>".into(),
        strings: ["E", "B", "G", "D", "A", "E"].map(String::from).to_vec(),
        measures: vec![Measure {
            voices: vec![vec![
                Beat {
                    duration: Duration::QUARTER,
                    notes: vec![
                        Note {
                            string: 1,
                            fret: Fret::Number(12),
                            ..Default::default()
                        },
                        Note {
                            string: 6,
                            fret: Fret::Dead,
                            ..Default::default()
                        }
                    ],
                    ..Default::default()
                };
                4
            ]],
            ..Default::default()
        }],
        ..Default::default()
    }
}

#[test]
fn wraps_measures_and_keeps_all_frets_inside_page() {
    const WIDTH: f32 = 380.0;
    const MEASURES: usize = 3;
    let mut track = track();
    track.measures = vec![track.measures[0].clone(); MEASURES];
    let page = layout(
        &track,
        LayoutOptions {
            width: WIDTH,
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(page.beats.len(), MEASURES * 4);
    assert!(page.beats[4].rect[1] > page.beats[0].rect[1]);
    for beat in &page.beats {
        assert!(beat.rect[0] >= 0.0 && beat.rect[2] <= page.width);
        assert!(beat.rect[3] <= page.height);
        let hit = page
            .hit_test(
                (beat.rect[0] + beat.rect[2]) / 2.0,
                (beat.rect[1] + beat.rect[3]) / 2.0,
            )
            .unwrap();
        assert_eq!(
            (hit.measure, hit.voice, hit.beat),
            (beat.measure, beat.voice, beat.beat)
        );
    }
    let svg = page.to_svg();
    assert!(svg.contains("Tabs &lt;&amp;&gt;"));
    assert!(!svg.contains("Tabs <&>"));
    assert!(svg.contains(">12</text>"));
    assert!(svg.contains(">x</text>"));
}

#[test]
fn exports_png_and_vector_pdf_from_shared_svg_geometry() {
    const PNG_MAGIC: &[u8] = b"\x89PNG\r\n\x1a\n";
    const PDF_MAGIC: &[u8] = b"%PDF-";
    let page = layout(&track(), LayoutOptions::default()).unwrap();
    let png = page.to_png(RasterOptions { scale: 1.0 }).unwrap();
    let pdf = page.to_pdf().unwrap();
    assert!(png.starts_with(PNG_MAGIC));
    assert!(pdf.starts_with(PDF_MAGIC));
    let outlined = page.to_svg_outlined_text().unwrap();
    assert!(!outlined.contains("<text"));
    assert!(outlined.contains("<path"));
    assert!(page.to_png(RasterOptions { scale: 0.0 }).is_err());
}

#[test]
fn cross_system_tie_halves_stay_with_their_own_system() {
    let note = |fret| Note {
        string: 1,
        fret,
        pitch: Some(Pitch {
            step: 2,
            octave: 4,
            accidental: 0,
        }),
        ..Default::default()
    };
    let mut second = Measure {
        voices: vec![vec![Beat {
            notes: vec![Note {
                fret: Fret::Number(5),
                ..note(Fret::Number(5))
            }],
            ..Default::default()
        }]],
        ..Default::default()
    };
    second.break_before = true;
    let track = Track {
        strings: vec!["E".into()],
        measures: vec![
            Measure {
                voices: vec![vec![Beat {
                    notes: vec![note(Fret::Number(5))],
                    ..Default::default()
                }]],
                ..Default::default()
            },
            second,
        ],
        ..Default::default()
    };

    let page = layout(
        &track,
        LayoutOptions {
            display: DisplayMode::Both,
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(page.systems.len(), 2);
    let mut tie_segments_per_system = [0, 0];
    for primitive in &page.primitives {
        let Primitive::Line {
            from, to, width, ..
        } = primitive
        else {
            continue;
        };
        if (*width - 1.2).abs() > f32::EPSILON {
            continue;
        }
        let y = (from[1] + to[1]) / 2.0;
        if let Some(system) = page
            .systems
            .iter()
            .position(|range| y >= range[0] && y < range[1])
        {
            tie_segments_per_system[system] += 1;
        }
    }
    assert!(tie_segments_per_system.into_iter().all(|count| count > 0));
    // Assigning the outgoing half to system 0 must not capture the staff and
    // tablature lines already emitted for system 1.
    for system in &page.systems {
        let structural_lines = page
            .primitives
            .iter()
            .filter(|primitive| {
                matches!(primitive, Primitive::Line { from, to, .. }
                    if (from[1] - to[1]).abs() < f32::EPSILON
                        && to[0] - from[0] > 100.0
                        && from[1] >= system[0]
                        && from[1] < system[1])
            })
            .count();
        assert!(structural_lines >= 6);
    }
}

#[test]
fn aligns_voices_and_chord_notes() {
    const EPSILON: f32 = 0.001;
    let mut track = track();
    let mut half = track.measures[0].voices[0][0].clone();
    half.duration.value = 2;
    track.measures[0].voices.push(vec![half; 2]);
    let page = layout(&track, LayoutOptions::default()).unwrap();
    assert!((page.beats[0].rect[0] - page.beats[4].rect[0]).abs() < EPSILON);
    assert!((page.beats[2].rect[0] - page.beats[5].rect[0]).abs() < EPSILON);
    let positions: Vec<_> = page
        .primitives
        .iter()
        .filter_map(|p| match p {
            Primitive::Text { at, text, .. } if text == "12" || text == "x" => Some(at),
            _ => None,
        })
        .collect();
    assert_eq!(positions[0][0], positions[1][0]);
    assert!(positions[0][1] < positions[1][1]);
}

#[test]
fn handles_dense_measures_rests_and_tuplets() {
    const NARROW: f32 = 160.0;
    let mut track = track();
    track.measures[0].voices = vec![vec![
        Beat {
            duration: Duration {
                value: 8,
                dots: 1,
                tuplet: Some((3, 2))
            },
            notes: vec![],
            ..Default::default()
        };
        24
    ]];
    let page = layout(
        &track,
        LayoutOptions {
            width: NARROW,
            ..Default::default()
        },
    )
    .unwrap();
    assert!(page.width > NARROW);
    assert!(page.to_svg().contains("data-smufl=\"E4E6\""));
    assert!(page.to_svg().contains(">3</text>"));
    assert_eq!(
        track.measures[0].voices[0][0]
            .duration
            .compute_quarter_beats()
            .unwrap(),
        0.5
    );
}

#[test]
fn rejects_malformed_models_and_options() {
    const INVALID_DURATIONS: [i16; 5] = [0, 3, 512, -1, -3];
    for value in INVALID_DURATIONS {
        assert!(Duration {
            value,
            ..Duration::QUARTER
        }
        .compute_quarter_beats()
        .is_err());
    }
    assert!(Duration {
        tuplet: Some((0, 2)),
        ..Duration::QUARTER
    }
    .compute_quarter_beats()
    .is_err());
    for width in [f32::NAN, f32::INFINITY, 0.0] {
        assert!(layout(
            &track(),
            LayoutOptions {
                width,
                ..Default::default()
            }
        )
        .is_err());
    }
    let mut track = track();
    track.measures[0].voices[0][0].notes[0].string = 0;
    assert!(layout(&track, LayoutOptions::default()).is_err());
    track.measures[0].voices[0][0].notes[0].string = 6;
    assert!(layout(&track, LayoutOptions::default()).is_err());
    track.strings.clear();
    assert!(layout(&track, LayoutOptions::default()).is_err());
}

#[test]
fn renders_empty_track_and_empty_measure() {
    let mut track = track();
    track.measures = vec![Measure::default()];
    let page = layout(&track, LayoutOptions::default()).unwrap();
    assert!(page.beats.is_empty());
    assert!(page.height > 80.0);
    track.measures.clear();
    assert!(layout(&track, LayoutOptions::default())
        .unwrap()
        .to_svg()
        .ends_with("</svg>"));
}

#[test]
fn engraves_music_glyphs_beams_and_written_pitches() {
    const EIGHTH_REST: char = '\u{E4E6}';
    const TREBLE_CLEF: char = '\u{E050}';
    const SHARP: char = '\u{E262}';
    let mut track = track();
    track.measures[0].voices = vec![(0..8)
        .map(|i| Beat {
            duration: Duration {
                value: 8,
                ..Default::default()
            },
            notes: if i == 7 {
                vec![]
            } else {
                vec![Note {
                    string: 1,
                    fret: Fret::Number(i),
                    pitch: Some(Pitch::from_midi(60 + i as u8, false)),
                    ..Default::default()
                }]
            },
            ..Default::default()
        })
        .collect()];
    let page = layout(
        &track,
        LayoutOptions {
            display: DisplayMode::Both,
            ..Default::default()
        },
    )
    .unwrap();
    for code in [EIGHTH_REST, TREBLE_CLEF, SHARP] {
        assert!(page.primitives.iter().any(|p|matches!(p,Primitive::Glyph{code:c,outline,..} if *c==code && !outline.vertices.is_empty() && !outline.indices.is_empty())));
    }
    // Bravura specifies beams at 0.5 of the 10-unit staff space.
    assert!(page.primitives.iter().any(
        |p| matches!(p,Primitive::Line{from,to,width, ..} if *width==5.0 && to[0]-from[0]>20.0)
    ));
    let svg = page.to_svg();
    assert!(!svg.contains("r/8"));
    assert!(svg.contains("<path data-smufl="));
    assert!(!svg.contains("NaN"));
}

#[test]
fn validates_staff_pitch_and_handles_minimum_key_value_without_panicking() {
    let mut t = track();
    assert!(layout(
        &t,
        LayoutOptions {
            display: DisplayMode::Standard,
            ..Default::default()
        }
    )
    .is_err());
    t.measures[0].key_signature = i8::MIN;
    assert!(layout(&t, Default::default()).is_err());
    t.measures[0].key_signature = 0;
    t.measures[0].voices[0][0].notes[0].effects.bend = vec![[0.5, 1.0], [0.0, 2.0]];
    assert!(layout(&t, Default::default()).is_err());
}

#[test]
fn pagination_and_zoom_preserve_every_beat_address() {
    const MEASURES: usize = 4;
    const ZOOM: f32 = 0.5;
    let mut t = track();
    t.measures = vec![t.measures[0].clone(); MEASURES];
    let page = layout(
        &t,
        LayoutOptions {
            width: 380.0,
            ..Default::default()
        },
    )
    .unwrap();
    let scaled = page.scaled(ZOOM).unwrap();
    assert_eq!(scaled.width, page.width * ZOOM);
    for (a, b) in page.beats.iter().zip(&scaled.beats) {
        assert_eq!(b.rect[0], a.rect[0] * ZOOM);
    }
    let height = scaled
        .systems
        .iter()
        .map(|s| s[1] - s[0])
        .fold(0.0, f32::max)
        + 1.0;
    let pages = scaled.paginate(height).unwrap();
    assert_eq!(
        pages.iter().map(|p| p.beats.len()).sum::<usize>(),
        page.beats.len()
    );
    let addresses: Vec<_> = pages
        .iter()
        .flat_map(|p| &p.beats)
        .map(|b| (b.measure, b.voice, b.beat))
        .collect();
    let expected: Vec<_> = page
        .beats
        .iter()
        .map(|b| (b.measure, b.voice, b.beat))
        .collect();
    assert_eq!(addresses, expected);
    assert!(page.scaled(f32::NAN).is_err());
    assert!(page.paginate(100.0).is_err());
}

#[test]
fn pagination_to_fit_scales_an_oversized_system_without_losing_addresses() {
    const PAGE_HEIGHT: f32 = 100.0;
    let mut source = track();
    source.measures = vec![source.measures[0].clone(); 3];
    let page = layout(
        &source,
        LayoutOptions {
            bars_per_system: Some(3),
            ..Default::default()
        },
    )
    .unwrap();
    assert!(page.paginate(PAGE_HEIGHT).is_err());
    let pages = page.paginate_to_fit(PAGE_HEIGHT).unwrap();
    assert_eq!(
        pages.iter().map(|p| p.beats.len()).sum::<usize>(),
        page.beats.len()
    );
    assert!(pages.iter().all(|p| p.height == PAGE_HEIGHT));
}

#[test]
fn horizontal_layout_keeps_measures_on_one_system() {
    let mut t = track();
    t.measures = vec![t.measures[0].clone(); 4];
    let page = layout(
        &t,
        LayoutOptions {
            width: 200.0,
            flow: LayoutMode::Horizontal,
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(page.systems.len(), 1);
    assert!(page.width > 200.0);
    assert_eq!(
        page.beats.first().unwrap().rect[1],
        page.beats.last().unwrap().rect[1]
    );
}

#[test]
fn svg_replaces_xml_forbidden_control_characters() {
    let mut t = track();
    t.name = "Title\u{5}<&".into();
    let svg = layout(&t, Default::default()).unwrap().to_svg();
    assert!(svg.contains("Title\u{FFFD}&lt;&amp;"));
    assert!(!svg.contains('\u{5}'));
}

#[test]
fn paints_native_egui_meshes_without_installing_fonts() {
    let mut t = track();
    t.measures[0].voices[0][0].notes.clear();
    let page = layout(&t, Default::default()).unwrap();
    let context = egui::Context::default();
    let mut output = context.run_ui(Default::default(), |ui| {
        page.show(ui);
    });
    assert!(!output.shapes.is_empty());
    output.textures_delta.clear();
}

#[test]
fn cancels_previous_key_and_rejects_overlapping_onsets() {
    const NATURAL: char = '\u{E261}';
    let mut t = track();
    for beat in &mut t.measures[0].voices[0] {
        for note in &mut beat.notes {
            note.pitch = Some(Pitch::from_midi(64, false));
        }
    }
    t.measures[0].key_signature = 1;
    let mut second = t.measures[0].clone();
    second.key_signature = 0;
    t.measures.push(second);
    let page = layout(
        &t,
        LayoutOptions {
            display: DisplayMode::Both,
            ..Default::default()
        },
    )
    .unwrap();
    assert!(page
        .primitives
        .iter()
        .any(|p| matches!(p,Primitive::Glyph{code,..} if *code==NATURAL)));
    t.measures[0].voices[0][1].start = Some(0.0);
    assert!(layout(&t, Default::default()).is_err());
}

#[test]
fn aligns_tracks_with_different_rhythms_and_retains_track_identity() {
    let first = track();
    let mut second = first.clone();
    second.name = "Bass".into();
    second.measures[0].voices[0].truncate(2);
    for beat in &mut second.measures[0].voices[0] {
        beat.duration.value = 2;
    }
    let score = layout_score(&[first, second], Default::default()).unwrap();
    let guitar = &score.beats[0];
    let bass = &score.beats[4];
    assert_eq!(guitar.beat.rect[0], bass.beat.rect[0]);
    assert!(bass.beat.rect[1] > guitar.beat.rect[1]);
    assert_eq!(score.beats[2].beat.rect[0], score.beats[5].beat.rect[0]);
    let r = bass.beat.rect;
    assert_eq!(
        score
            .hit_test((r[0] + r[2]) / 2.0, (r[1] + r[3]) / 2.0)
            .unwrap()
            .track,
        1
    );
    assert_eq!(score.beats.len(), 6);
}

#[test]
fn renders_extended_guitar_effects_and_validates_their_geometry() {
    let mut t = track();
    t.measures[0].repeat_end = true;
    t.measures[0].repeat_count = Some(3);
    let b = &mut t.measures[0].voices[0][0];
    b.notes[0].effects.slide_in = Some(SlideDirection::Up);
    b.notes[0].effects.slide_out = Some(SlideDirection::Down);
    b.annotations.whammy = vec![[0.0, 0.0], [0.5, -2.0], [1.0, 0.0]];
    b.annotations.technique = Some(PluckingTechnique::Slap);
    b.annotations.strum_up = Some(true);
    b.annotations.chord = Some(ChordDiagram {
        name: "F".into(),
        frets: vec![Some(1), Some(1), Some(2), Some(3), Some(3), Some(1)],
        first_fret: 1,
        barres: vec![Barre {
            fret: 1,
            first_string: 1,
            last_string: 6,
        }],
        ..Default::default()
    });
    let page = layout(&t, Default::default()).unwrap();
    let svg = page.to_svg();
    for text in [">w.bar</text>", ">3×</text>", ">S</text>"] {
        assert!(svg.contains(text));
    }
    assert!(page.primitives.iter().any(|p|matches!(p,Primitive::Line {from,to,width, ..} if *width==4.0 && (from[0]-to[0]).abs()==50.0)));
    t.measures[0].voices[0][0].annotations.whammy[1][0] = f32::NAN;
    assert!(layout(&t, Default::default()).is_err());
    t.measures[0].voices[0][0].annotations.whammy.clear();
    t.measures[0].voices[0][0]
        .annotations
        .chord
        .as_mut()
        .unwrap()
        .barres[0]
        .last_string = 7;
    assert!(layout(&t, Default::default()).is_err());
}

#[test]
fn combined_mode_draws_ties_on_both_staves() {
    let mut t = track();
    t.measures[0].voices = vec![vec![
        Beat {
            notes: vec![Note {
                string: 1,
                fret: Fret::Number(5),
                pitch: Some(Pitch::from_midi(69, false)),
                ..Default::default()
            }],
            ..Default::default()
        };
        2
    ]];
    t.measures[0].voices[0][1].notes[0].fret = Fret::Tied(5);
    let page = layout(
        &t,
        LayoutOptions {
            display: DisplayMode::Both,
            ..Default::default()
        },
    )
    .unwrap();
    let arcs: Vec<_> = page
        .primitives
        .iter()
        .filter_map(|p| match p {
            Primitive::Curve { points, width, .. }
                if *width == 1.2 && points[3][0] > points[0][0] =>
            {
                Some(points.iter().map(|point| point[1]).sum::<f32>() / 4.0)
            }
            _ => None,
        })
        .collect();
    assert_eq!(arcs.len(), 2);
    let min = arcs.iter().copied().fold(f32::INFINITY, f32::min);
    let max = arcs.iter().copied().fold(f32::NEG_INFINITY, f32::max);
    assert!(
        max - min > 79.0,
        "ties must occupy separate staff and tab bands: {arcs:?}"
    );
}

#[test]
fn percussion_chords_use_unpitched_clef_and_distinct_heads() {
    const SNARE: u16 = 38;
    const HIHAT: u16 = 42;
    let notes = [SNARE, HIHAT]
        .map(|id| Note {
            percussion: Some(id),
            pitch: Some(percussion::resolve(id).unwrap().0),
            ..Default::default()
        })
        .to_vec();
    let t = Track {
        clef: Clef::Percussion,
        measures: vec![Measure {
            voices: vec![vec![Beat {
                notes,
                ..Default::default()
            }]],
            ..Default::default()
        }],
        ..Default::default()
    };
    let page = layout(&t, Default::default()).unwrap();
    for code in ['\u{E069}', '\u{E0A4}', '\u{E0A9}'] {
        assert!(page
            .primitives
            .iter()
            .any(|p| matches!(p,Primitive::Glyph {code:c,..} if *c==code)));
    }
    assert!(percussion::resolve(u16::MAX).is_err());
}

#[test]
fn ornaments_fermatas_navigation_and_grace_notes_render_as_music_glyphs() {
    let mut t = track();
    t.measures.truncate(1);
    t.measures[0].navigation = Some(Navigation::Coda);
    t.measures[0].fermatas = vec![Fermata {
        start: 0.0,
        kind: FermataKind::Long,
    }];
    for b in &mut t.measures[0].voices[0] {
        for n in &mut b.notes {
            n.pitch = Some(Pitch::from_midi(64, false));
        }
    }
    let effects = &mut t.measures[0].voices[0][0].notes[0].effects;
    effects.ornament = Some(Ornament::Turn);
    effects.grace_fret = Some(2);
    effects.grace_pitch = Some(Pitch::from_midi(62, false));
    let page = layout(
        &t,
        LayoutOptions {
            display: DisplayMode::Both,
            ..Default::default()
        },
    )
    .unwrap();
    for code in ['\u{E048}', '\u{E4C6}', '\u{E567}'] {
        assert!(page
            .primitives
            .iter()
            .any(|p| matches!(p,Primitive::Glyph {code:c,..} if *c==code)));
    }
    assert!(page
        .primitives
        .iter()
        .any(|p| matches!(p,Primitive::Glyph {code,space,..} if *code=='\u{E0A4}' && *space==5.0)));
}

#[test]
fn composite_dynamics_are_visually_centered_on_the_beat() {
    let mut t = track();
    t.measures.truncate(1);
    t.measures[0].voices[0].truncate(1);
    t.measures[0].voices[0][0].annotations.dynamic = Some("fff".into());
    let page = layout(&t, LayoutOptions::default()).unwrap();
    let beat_center = (page.beats[0].cursor_rect[0] + page.beats[0].cursor_rect[2]) / 2.0;
    let visual_center = page
        .primitives
        .iter()
        .find_map(|primitive| match primitive {
            Primitive::Glyph {
                at,
                space,
                code: '\u{E530}',
                outline,
                ..
            } => Some(at[0] + (outline.bounds[0] + outline.bounds[2]) * space / 2.0),
            _ => None,
        })
        .unwrap();
    assert!((visual_center - beat_center).abs() < 0.001);
}

#[test]
fn justification_and_width_constraints_are_explicit() {
    const WIDTH: f32 = 800.0;
    let page = layout(
        &track(),
        LayoutOptions {
            width: WIDTH,
            justify: true,
            strict_width: true,
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(page.width, WIDTH);
    assert!(page
        .primitives
        .iter()
        .any(|p| matches!(p,Primitive::Line {to,..} if to[0]==WIDTH-20.0)));
    assert!(layout(
        &track(),
        LayoutOptions {
            width: 160.0,
            strict_width: true,
            ..Default::default()
        }
    )
    .is_err());
}

#[test]
fn selection_uses_onsets_across_unequal_voices() {
    let mut t = track();
    let mut half = t.measures[0].voices[0][0].clone();
    half.duration.value = 2;
    t.measures[0].voices.push(vec![half; 2]);
    let page = layout(&t, Default::default()).unwrap();
    let selection = Selection {
        anchor: BeatAddress {
            measure: 0,
            voice: 0,
            beat: 2,
        },
        end: BeatAddress {
            measure: 0,
            voice: 0,
            beat: 2,
        },
    };
    assert!(selection.contains(
        &page,
        BeatAddress {
            measure: 0,
            voice: 1,
            beat: 1
        }
    ));
    assert!(!selection.contains(
        &page,
        BeatAddress {
            measure: 0,
            voice: 1,
            beat: 0
        }
    ));
}
