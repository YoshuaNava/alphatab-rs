//! Adapter for already-parsed `guitarpro` values. No file-format parser lives here.
use crate::*;
use guitarpro::{BeatStatus, NoteType};

#[derive(Debug)]
/// Converted track plus recoverable source-fidelity warnings.
pub struct ImportReport {
    /// Native rendering model produced by the adapter.
    pub track: Track,
    /// Deduplicated warnings for unsupported or inconsistent source values.
    pub warnings: Vec<String>,
}

/// Converts one parsed Guitar Pro track into the native rendering model.
///
/// The `song` supplies shared headers and metadata. File decoding remains the
/// responsibility of the `guitarpro` crate.
pub fn convert_track(
    song: &guitarpro::Song,
    source: &guitarpro::Track,
) -> Result<ImportReport, RenderError> {
    convert_track_inner(song, source)
        .map_err(|error| RenderError::new(crate::ErrorKind::Import, error.message().to_owned()))
}

fn convert_track_inner(
    song: &guitarpro::Song,
    source: &guitarpro::Track,
) -> Result<ImportReport, RenderError> {
    let mut warnings = std::collections::BTreeSet::new();
    let mut measures = vec![];
    let mut prior_dynamic = None;
    let mut prior_tempo = 0;
    let mut prior_clef = None;
    // guitarpro's public optimized conversion exposes its otherwise private beat display hints.
    let hints_song = guitarpro::Song {
        tracks: vec![source.clone()],
        measure_headers: song.measure_headers.clone(),
        ..Default::default()
    };
    let hints = guitarpro::convert::optimized::legacy::legacy_song_to_loaded_score(&hints_song);
    let hint_track = hints.score.tracks.first().ok_or_else(|| {
        RenderError::invalid_input("guitarpro display conversion returned no track".into())
    })?;
    for (mi, measure) in source.measures.iter().enumerate() {
        let h = song.measure_headers.get(mi).ok_or_else(|| {
            RenderError::invalid_input(format!("missing header for measure {}", mi + 1))
        })?;
        let mut voices = vec![];
        for (vi, voice) in measure.voices.iter().enumerate() {
            let mut beats = vec![];
            let mut start = 0.0;
            for (bi, beat) in voice.beats.iter().enumerate() {
                let duration = convert_duration(&beat.duration)?;
                if beat.status == BeatStatus::Empty {
                    continue;
                }
                if let Some(onset) = beat.start {
                    // The legacy model uses 960 as the origin in every measure.
                    let onset = (onset - 960) as f64 / 960.0;
                    if onset >= start - 1e-8 {
                        start = onset;
                    } else {
                        warnings.insert(format!("Measure {}, voice {}: overlapping source beat timestamps; sequential duration used", mi + 1, vi + 1));
                    }
                }
                let mut notes = vec![];
                for n in &beat.notes {
                    if n.kind == NoteType::Rest {
                        continue;
                    }
                    let string = usize::try_from(n.string)
                        .map_err(|_| RenderError::invalid_input("negative string index".into()))?;
                    let open = if source.percussion_track {
                        0
                    } else {
                        source
                            .strings
                            .get(string.checked_sub(1).ok_or_else(|| {
                                RenderError::invalid_input("zero string index".into())
                            })?)
                            .ok_or_else(|| {
                                RenderError::invalid_input("string index out of range".into())
                            })?
                            .1
                    };
                    let fret = match n.kind {
                        NoteType::Dead => Fret::Dead,
                        NoteType::Normal => Fret::Number(convert_fret(n.value)?),
                        NoteType::Tie => Fret::Tied(convert_fret(n.value)?),
                        _ => return Err(RenderError::invalid_input("unknown note type".into())),
                    };
                    let harmonic_fret = n
                        .effect
                        .harmonic
                        .as_ref()
                        .filter(|h| h.kind == guitarpro::HarmonicType::Natural)
                        .map_or(i32::from(n.value), |h| {
                            natural_harmonic(h.fret.map(i32::from).unwrap_or(i32::from(n.value)))
                        });
                    let ottava_shift = match beat.octave {
                        guitarpro::Octave::Ottava => -12,
                        guitarpro::Octave::Quindicesima => -24,
                        guitarpro::Octave::OttavaBassa => 12,
                        guitarpro::Octave::QuindicesimaBassa => 24,
                        guitarpro::Octave::None => 0,
                    };
                    let initial_bend = if let Some(bend) = &n.effect.bend {
                        if bend.semitone_length == 0 {
                            return Err(RenderError::invalid_input("invalid bend scale".into()));
                        }
                        bend.points.first().map_or(0.0, |p| {
                            f32::from(p.value) / f32::from(bend.semitone_length)
                        })
                    } else {
                        0.0
                    };
                    let midi = i32::from(open)
                        + harmonic_fret
                        + source.offset
                        + ottava_shift
                        + initial_bend.floor() as i32
                        - source.transpose_chromatic
                        - source.transpose_octave * 12;
                    let percussion = if source.percussion_track {
                        Some(convert_fret(n.value)?)
                    } else {
                        None
                    };
                    let pitch = if let Some(id) = percussion {
                        Some(crate::percussion::resolve(id)?.0)
                    } else if (0..=127).contains(&midi) {
                        Some(if n.swap_accidentals {
                            Pitch::from_midi(midi as u8, h.key_signature.key >= 0)
                        } else {
                            Pitch::in_key(midi as u8, h.key_signature.key)
                        })
                    } else {
                        return Err(RenderError::invalid_input(format!(
                            "written pitch outside MIDI range in measure {}",
                            mi + 1
                        )));
                    };
                    let e = &n.effect;
                    let mut bend = vec![];
                    if let Some(b) = &e.bend {
                        if b.max_position == 0 || b.semitone_length == 0 {
                            return Err(RenderError::invalid_input(
                                "invalid Guitar Pro bend scale".into(),
                            ));
                        }
                        for p in &b.points {
                            bend.push([
                                f32::from(p.position) / f32::from(b.max_position),
                                f32::from(p.value) / f32::from(b.semitone_length),
                            ]);
                        }
                    }
                    if e.tremolo_picking.as_ref().is_some_and(|t| {
                        !t.duration.value.is_power_of_two()
                            || t.duration.value < 8
                            || t.duration.value > 256
                    }) {
                        return Err(RenderError::invalid_input(
                            "invalid tremolo picking duration".into(),
                        ));
                    }
                    let effects = NoteEffects {
                        quarter_tone: if initial_bend.fract().abs() > 0.01 {
                            1
                        } else {
                            0
                        },
                        grace_dead: e.grace.as_ref().is_some_and(|g| g.is_dead),
                        grace_bend: e.grace.as_ref().is_some_and(|g| {
                            g.transition == guitarpro::GraceEffectTransition::Bend
                        }),
                        grace_slur: e.grace.as_ref().is_some_and(|g| {
                            g.transition == guitarpro::GraceEffectTransition::Hammer
                        }),
                        grace_duration: e.grace.as_ref().map_or(8, |g| u16::from(g.duration)),
                        head: if e
                            .harmonic
                            .as_ref()
                            .is_some_and(|h| h.kind == guitarpro::HarmonicType::Natural)
                        {
                            NoteHead::Diamond
                        } else {
                            NoteHead::Normal
                        },
                        harmonic_pitch: e
                            .harmonic
                            .as_ref()
                            .filter(|h| h.kind != guitarpro::HarmonicType::Natural)
                            .map(|harmonic| {
                                let sounding = if let Some(p) = &harmonic.pitch {
                                    let shift = match harmonic.octave {
                                        Some(guitarpro::Octave::Quindicesima) => 24,
                                        Some(guitarpro::Octave::OttavaBassa) => -12,
                                        Some(guitarpro::Octave::QuindicesimaBassa) => -24,
                                        _ => 12,
                                    };
                                    midi + (i32::from(p.value)
                                        - (i32::from(open) + i32::from(n.value) + source.offset)
                                            .rem_euclid(12))
                                    .rem_euclid(12)
                                        + shift
                                } else if let Some(fret) = harmonic.fret {
                                    let relative =
                                        if harmonic.kind == guitarpro::HarmonicType::Tapped {
                                            i32::from(fret) - i32::from(n.value)
                                        } else {
                                            i32::from(fret)
                                        };
                                    midi + natural_harmonic(relative)
                                } else {
                                    midi + 12
                                };
                                let sounding = u8::try_from(sounding)
                                    .ok()
                                    .filter(|p| *p <= 127)
                                    .ok_or_else(|| {
                                        RenderError::invalid_input(
                                            "harmonic pitch outside MIDI range".into(),
                                        )
                                    })?;
                                Ok(Pitch::in_key(sounding, h.key_signature.key))
                            })
                            .transpose()?,
                        right_fingering: fingering(&e.right_hand_finger, true, &mut warnings),
                        bend_vibrato: e
                            .bend
                            .as_ref()
                            .is_some_and(|b| b.points.iter().any(|p| p.vibrato)),
                        slide_legato: e.slides.contains(&guitarpro::SlideType::LegatoSlideTo),
                        slide_in: e.slides.iter().find_map(|s| match s {
                            guitarpro::SlideType::IntoFromAbove => Some(SlideDirection::Down),
                            guitarpro::SlideType::IntoFromBelow => Some(SlideDirection::Up),
                            _ => None,
                        }),
                        slide_out: e.slides.iter().find_map(|s| match s {
                            guitarpro::SlideType::OutDownwards => Some(SlideDirection::Down),
                            guitarpro::SlideType::OutUpWards => Some(SlideDirection::Up),
                            _ => None,
                        }),
                        grace_pitch: e
                            .grace
                            .as_ref()
                            .map(|g| {
                                let fret = if g.is_dead {
                                    0
                                } else {
                                    convert_fret(i16::from(g.fret))?
                                };
                                let midi = i32::from(open)
                                    + i32::from(fret)
                                    + source.offset
                                    + ottava_shift
                                    - source.transpose_chromatic
                                    - source.transpose_octave * 12;
                                u8::try_from(midi)
                                    .ok()
                                    .filter(|v| *v <= 127)
                                    .map(|v| Pitch::in_key(v, h.key_signature.key))
                                    .ok_or(())
                                    .map_err(|_| {
                                        RenderError::invalid_input(
                                            "grace pitch outside MIDI range".into(),
                                        )
                                    })
                            })
                            .transpose()?,
                        grace_on_beat: e.grace.as_ref().is_some_and(|g| g.is_on_beat),
                        grace_slide: e.grace.as_ref().is_some_and(|g| {
                            g.transition == guitarpro::GraceEffectTransition::Slide
                        }),
                        ornament: e.ornament.as_deref().and_then(|o| match o {
                            "Turn" => Some(Ornament::Turn),
                            "InvertedTurn" => Some(Ornament::InvertedTurn),
                            "UpperMordent" => Some(Ornament::UpperMordent),
                            "LowerMordent" => Some(Ornament::LowerMordent),
                            "Trill" => Some(Ornament::Trill),
                            _ => None,
                        }),
                        ghost: e.ghost_note,
                        hammer_on: e.hammer,
                        slide: e.slides.iter().any(|s| {
                            matches!(
                                s,
                                guitarpro::SlideType::ShiftSlideTo
                                    | guitarpro::SlideType::LegatoSlideTo
                            )
                        }),
                        vibrato: e.vibrato || beat.effect.vibrato,
                        palm_mute: e.palm_mute,
                        let_ring: e.let_ring,
                        staccato: e.staccato,
                        accent: e.accentuated_note,
                        heavy_accent: e.heavy_accentuated_note,
                        harmonic: e.harmonic.as_ref().map(|h| {
                            match h.kind {
                                guitarpro::HarmonicType::Natural => "N.H.",
                                guitarpro::HarmonicType::Artificial => "A.H.",
                                guitarpro::HarmonicType::Tapped => "T.H.",
                                guitarpro::HarmonicType::Pinch => "P.H.",
                                guitarpro::HarmonicType::Semi => "S.H.",
                            }
                            .into()
                        }),
                        bend,
                        grace_fret: e
                            .grace
                            .as_ref()
                            .map(|g| {
                                if g.is_dead {
                                    Ok(0)
                                } else {
                                    convert_fret(i16::from(g.fret))
                                }
                            })
                            .transpose()?,
                        trill_fret: e
                            .trill
                            .as_ref()
                            .map(|t| convert_fret(i16::from(t.fret)))
                            .transpose()?,
                        tremolo_slashes: e
                            .tremolo_picking
                            .as_ref()
                            .map_or(0, |t| t.duration.value.ilog2().saturating_sub(2) as u8),
                        fingering: fingering(&e.left_hand_finger, false, &mut warnings),
                    };
                    if let (Some(ornament), None) = (e.ornament.as_deref(), effects.ornament) {
                        warnings.insert(format!("Unrecognized Guitar Pro ornament: {}", ornament));
                    }
                    notes.push(Note {
                        string,
                        fret,
                        pitch,
                        effects,
                        percussion,
                    });
                }
                let dynamic = beat.notes.first().map(|n| n.velocity);
                let hint = hint_track
                    .measures
                    .get(&guitarpro::model::optimized::global::MeasureIndex(
                        mi as u16,
                    ))
                    .and_then(|m| m.voices.get(&(vi as u8)))
                    .and_then(|v| v.beats.get(bi))
                    .ok_or_else(|| {
                        RenderError::invalid_input(
                            "guitarpro display conversion lost a beat".into(),
                        )
                    })?;
                let flags = hint.gp_beat_flags2.unwrap_or(0);
                let mut annotations = BeatAnnotations {
                    beaming: if flags & 1 != 0 {
                        Beaming::Break
                    } else if flags & 4 != 0 {
                        Beaming::Join
                    } else {
                        Beaming::Auto
                    },
                    stem: if flags & 2 != 0 || voice.directions == guitarpro::VoiceDirection::Down {
                        StemDirection::Down
                    } else if flags & 8 != 0 || voice.directions == guitarpro::VoiceDirection::Up {
                        StemDirection::Up
                    } else {
                        StemDirection::Auto
                    },
                    break_secondary: hint.gp_break_secondary.unwrap_or(0),
                    tuplet_start: flags & 0x200 != 0,
                    tuplet_end: flags & 0x400 != 0,
                    force_tuplet_bracket: flags & 0x2000 != 0,
                    rasgueado: beat.effect.has_rasgueado,
                    fade: beat.effect.fade_in.then_some(Fade::In),
                    ottava: match beat.octave {
                        guitarpro::Octave::Ottava => Some(Ottava::Above8),
                        guitarpro::Octave::Quindicesima => Some(Ottava::Above15),
                        guitarpro::Octave::OttavaBassa => Some(Ottava::Below8),
                        guitarpro::Octave::QuindicesimaBassa => Some(Ottava::Below15),
                        guitarpro::Octave::None => None,
                    },
                    text: beat.text.clone(),
                    ..Default::default()
                };
                if dynamic != prior_dynamic {
                    annotations.dynamic = dynamic.map(|v| {
                        ["ppp", "pp", "p", "mp", "mf", "f", "ff", "fff"]
                            [((v - 15) / 16).clamp(0, 7) as usize]
                            .into()
                    });
                    prior_dynamic = dynamic;
                }
                if let Some(chord) = &beat.effect.chord {
                    let frets: Vec<_> = chord
                        .strings
                        .iter()
                        .take(source.strings.len())
                        .map(|f| u16::try_from(*f).ok())
                        .collect();
                    if frets.len() == source.strings.len() {
                        let first = frets
                            .iter()
                            .flatten()
                            .copied()
                            .filter(|f| *f > 0)
                            .min()
                            .unwrap_or(1);
                        if chord.show != Some(false) {
                            annotations.chord = Some(ChordDiagram {
                                name: chord.name.clone(),
                                first_fret: chord
                                    .first_fret
                                    .map(u16::from)
                                    .filter(|f| *f > 0 && *f <= first)
                                    .unwrap_or(first),
                                fret_count: 5,
                                fingers: if chord.fingerings.len() >= frets.len() {
                                    chord
                                        .fingerings
                                        .iter()
                                        .take(source.strings.len())
                                        .map(|f| {
                                            fingering(f, false, &mut warnings).unwrap_or_default()
                                        })
                                        .collect()
                                } else {
                                    vec![]
                                },
                                frets,
                                barres: chord
                                    .barres
                                    .iter()
                                    .map(|b| {
                                        Ok(Barre {
                                            fret: convert_fret(i16::from(b.fret))?,
                                            first_string: usize::try_from(b.start.min(b.end))
                                                .map_err(|_| {
                                                    RenderError::invalid_input(
                                                        "negative barre string".into(),
                                                    )
                                                })?,
                                            last_string: usize::try_from(b.start.max(b.end))
                                                .map_err(|_| {
                                                    RenderError::invalid_input(
                                                        "negative barre string".into(),
                                                    )
                                                })?,
                                        })
                                    })
                                    .collect::<Result<Vec<_>, RenderError>>()?,
                            });
                        } else {
                            annotations.text = format!("{} {}", chord.name, annotations.text);
                        }
                    } else {
                        warnings
                            .insert("Incomplete chord diagrams are shown as chord names".into());
                        annotations.text = format!("{} {}", chord.name, annotations.text);
                    }
                }
                annotations.pick_up = match beat.effect.pick_stroke {
                    guitarpro::BeatStrokeDirection::Up => Some(true),
                    guitarpro::BeatStrokeDirection::Down => Some(false),
                    _ => None,
                };
                if let Some(bar) = &beat.effect.tremolo_bar {
                    if bar.max_position == 0 || bar.semitone_length == 0 {
                        return Err(RenderError::invalid_input(
                            "invalid tremolo-bar scale".into(),
                        ));
                    }
                    annotations.whammy = bar
                        .points
                        .iter()
                        .map(|p| {
                            [
                                f32::from(p.position) / f32::from(bar.max_position),
                                f32::from(p.value) / f32::from(bar.semitone_length),
                            ]
                        })
                        .collect();
                }
                annotations.strum_up = match beat.effect.stroke.direction {
                    guitarpro::BeatStrokeDirection::Up => Some(true),
                    guitarpro::BeatStrokeDirection::Down => Some(false),
                    _ => None,
                };
                annotations.technique = match beat.effect.slap_effect {
                    guitarpro::SlapEffect::Tapping => Some(PluckingTechnique::Tap),
                    guitarpro::SlapEffect::Slapping => Some(PluckingTechnique::Slap),
                    guitarpro::SlapEffect::Popping => Some(PluckingTechnique::Pop),
                    _ => None,
                };
                if let Some(mix) = &beat.effect.mix_table_change {
                    if !mix.hide_tempo {
                        annotations.tempo = mix.tempo.as_ref().map(|t| u16::from(t.value));
                    }
                    if let Some(wah) = &mix.wah {
                        if wah.display {
                            annotations.wah_open = match wah.value {
                                -2 => Some(false),
                                0..=100 => Some(true),
                                _ => None,
                            };
                        }
                    }
                    if !mix.tempo_name.is_empty() && !mix.hide_tempo {
                        annotations.text = format!("{} {}", mix.tempo_name, annotations.text);
                    }
                }
                let d = &beat.duration;
                beats.push(Beat {
                    start: Some(start),
                    notes,
                    annotations,
                    duration: Duration {
                        value: i16::try_from(d.value).map_err(|_| {
                            RenderError::invalid_input("duration denominator out of range".into())
                        })?,
                        dots: if d.double_dotted {
                            2
                        } else {
                            u8::from(d.dotted)
                        },
                        tuplet: if d.tuplet_enters == 1 && d.tuplet_times == 1 {
                            None
                        } else {
                            Some((d.tuplet_enters, d.tuplet_times))
                        },
                    },
                });
                start += duration.quarter_beats()?;
            }
            voices.push(beats);
        }
        let tempo = if h.tempo > 0 {
            h.tempo
        } else {
            i32::from(song.tempo)
        };
        let tempo_mark =
            if !song.hide_tempo && tempo > 0 && tempo != prior_tempo {
                Some(u16::try_from(tempo).map_err(|_| {
                    RenderError::invalid_input("tempo exceeds renderer range".into())
                })?)
            } else {
                None
            };
        prior_tempo = tempo;
        let navigation = h.direction.as_ref().map(|d| {
            use guitarpro::DirectionSign::*;
            match d {
                Coda => Navigation::Coda,
                DoubleCoda => Navigation::DoubleCoda,
                Segno => Navigation::Segno,
                SegnoSegno => Navigation::DoubleSegno,
                _ => Navigation::Instruction(
                    match d {
                        Fine => "Fine",
                        DaCapo => "D.C.",
                        DaCapoAlCoda => "D.C. al Coda",
                        DaCapoAlDoubleCoda => "D.C. al Double Coda",
                        DaCapoAlFine => "D.C. al Fine",
                        DaSegno => "D.S.",
                        DaSegnoAlCoda => "D.S. al Coda",
                        DaSegnoAlDoubleCoda => "D.S. al Double Coda",
                        DaSegnoAlFine => "D.S. al Fine",
                        DaSegnoSegno => "D.S.S.",
                        DaSegnoSegnoAlCoda => "D.S.S. al Coda",
                        DaSegnoSegnoAlDoubleCoda => "D.S.S. al Double Coda",
                        DaSegnoSegnoAlFine => "D.S.S. al Fine",
                        DaCoda => "To Coda",
                        DaDoubleCoda => "To Double Coda",
                        _ => unreachable!(),
                    }
                    .into(),
                ),
            }
        });
        let fermatas = h
            .fermatas
            .iter()
            .map(|(kind, offset)| {
                let values = offset
                    .split('/')
                    .map(str::parse::<f64>)
                    .collect::<Result<Vec<_>, _>>()
                    .map_err(|_| RenderError::invalid_input("invalid fermata offset".into()))?;
                let start = match values.as_slice() {
                    [n, d] if *d > 0.0 => n / d * 4.0,
                    [n] => *n,
                    _ => return Err(RenderError::invalid_input("invalid fermata offset".into())),
                };
                let kind = match kind.as_str() {
                    "Short" => FermataKind::Short,
                    "Long" => FermataKind::Long,
                    "Normal" | "Medium" | "" => FermataKind::Normal,
                    _ => {
                        return Err(RenderError::invalid_input(format!(
                            "unknown fermata kind {kind}"
                        )))
                    }
                };
                Ok(Fermata { start, kind })
            })
            .collect::<Result<Vec<_>, RenderError>>()?;
        let clef = if source.percussion_track {
            Clef::Percussion
        } else {
            match measure.clef {
                guitarpro::MeasureClef::Bass => Clef::Bass,
                guitarpro::MeasureClef::Tenor => Clef::Tenor,
                guitarpro::MeasureClef::Alto => Clef::Alto,
                _ => Clef::Treble,
            }
        };
        let clef_change = if prior_clef != Some(clef) {
            Some(clef)
        } else {
            None
        };
        prior_clef = Some(clef);
        measures.push(Measure {
            time_signature: (
                u8::try_from(h.time_signature.numerator)
                    .map_err(|_| RenderError::invalid_input("negative meter".into()))?,
                h.time_signature.denominator.value,
            ),
            voices,
            repeat_start: h.repeat_open,
            repeat_end: h.repeat_close >= 0,
            repeat_count: repeat_count(h.repeat_close, song.version.number.0),
            key_signature: h.key_signature.key,
            tempo: tempo_mark,
            marker: h
                .marker
                .as_ref()
                .map_or_else(String::new, |m| m.title.clone()),
            alternate_endings: (0..8)
                .filter(|i| h.repeat_alternative & (1 << i) != 0)
                .map(|i| i + 1)
                .collect(),
            break_before: mi > 0
                && matches!(
                    source.measures[mi - 1].line_break,
                    guitarpro::LineBreak::Break
                ),
            double_bar: h.double_bar || measure.has_double_bar,
            free_time: h.free_time,
            triplet_feel: if mi == 0 || song.measure_headers[mi - 1].triplet_feel != h.triplet_feel
            {
                match h.triplet_feel {
                    guitarpro::TripletFeel::None => None,
                    guitarpro::TripletFeel::Eighth => Some("Swing eighths".into()),
                    guitarpro::TripletFeel::Sixteenth => Some("Swing sixteenths".into()),
                }
            } else {
                None
            },
            simile: match measure.simile_mark.as_deref() {
                None | Some("None") => None,
                Some("Simple") => Some(Simile::Single),
                Some("FirstOfDouble") => Some(Simile::DoubleFirst),
                Some("SecondOfDouble") => Some(Simile::DoubleSecond),
                Some(other) => {
                    warnings.insert(format!("Unknown Guitar Pro simile: {other}"));
                    None
                }
            },
            beam_groups: beam_groups(&h.time_signature, song.version.number.0)?,
            beam_unit: Some(8),
            rest_count: 0,
            display_number: None,
            navigation,
            fermatas,
            clef: clef_change,
        });
    }
    if i32::from(song.lyrics.track_choice) == source.number && song.lyrics.track_choice > 0 {
        for (_, start, text) in &song.lyrics.lines {
            let mut visible = String::new();
            let mut hidden = false;
            for c in text.chars() {
                match c {
                    '[' => hidden = true,
                    ']' => hidden = false,
                    _ if !hidden => visible.push(c),
                    _ => {}
                }
            }
            let mut syllables = visible.split_whitespace().map(|s| s.replace('+', " "));
            for measure in measures
                .iter_mut()
                .skip(usize::from(start.saturating_sub(1)))
            {
                if let Some(voice) = measure.voices.first_mut() {
                    for beat in voice {
                        if beat.notes.is_empty()
                            || beat.notes.iter().all(|n| matches!(n.fret, Fret::Tied(_)))
                        {
                            continue;
                        }
                        if let Some(syllable) = syllables.next() {
                            if !beat.annotations.lyrics.is_empty() {
                                beat.annotations.lyrics.push('\n');
                            }
                            beat.annotations.lyrics.push_str(&syllable);
                        }
                    }
                }
            }
            if syllables.next().is_some() {
                warnings.insert("Lyrics extend beyond the available beats".into());
            }
        }
    }
    let track = Track {
        metadata: ScoreMetadata {
            title: song.name.clone(),
            subtitle: song.subtitle.clone(),
            artist: song.artist.clone(),
            album: song.album.clone(),
            words: song.words.clone(),
            music: song.author.clone(),
            copyright: song.copyright.clone(),
            instructions: song.instructions.clone(),
        },
        spans: vec![],
        capo: u16::try_from(source.offset)
            .map_err(|_| RenderError::invalid_input("invalid capo fret".into()))?,
        name: source.name.clone(),
        strings: source
            .strings
            .iter()
            .map(|(_, midi)| {
                [
                    "C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B",
                ][midi.rem_euclid(12) as usize]
                    .into()
            })
            .collect(),
        measures,
        clef: if source.percussion_track {
            Clef::Percussion
        } else if source.strings.iter().map(|s| s.1).max().unwrap_or(64) < 50 {
            Clef::Bass
        } else {
            Clef::Treble
        },
    };
    layout(&track, LayoutOptions::default())?;
    Ok(ImportReport {
        track,
        warnings: warnings.into_iter().collect(),
    })
}

/// Normalize the parser's duration representation at the import boundary.
fn convert_duration(
    source: &guitarpro::model::legacy::key_signature::Duration,
) -> Result<Duration, RenderError> {
    Ok(Duration {
        value: i16::try_from(source.value)
            .map_err(|_| RenderError::invalid_input("duration denominator out of range".into()))?,
        dots: if source.double_dotted {
            2
        } else {
            u8::from(source.dotted)
        },
        tuplet: if source.tuplet_enters == 1 && source.tuplet_times == 1 {
            None
        } else {
            Some((source.tuplet_enters, source.tuplet_times))
        },
    })
}

/// GP versions disagree on whether the stored repeat value includes the first pass.
fn repeat_count(source: i8, major_version: u8) -> Option<u8> {
    (source >= 0).then(|| source as u8 + u8::from(major_version < 6))
}

/// GP5 stores custom beam groups in eighth-note units. Other versions do not
/// expose the same field with compatible semantics, so they use meter defaults.
fn beam_groups(
    meter: &guitarpro::TimeSignature,
    major_version: u8,
) -> Result<Vec<u8>, RenderError> {
    let numerator = u32::try_from(meter.numerator)
        .map_err(|_| RenderError::invalid_input("negative meter".into()))?;
    let covers_measure = meter.beams.iter().map(|n| u32::from(*n)).sum::<u32>()
        * u32::from(meter.denominator.value)
        == numerator * 8;
    Ok(if major_version == 5 && covers_measure {
        meter.beams.iter().copied().filter(|n| *n > 0).collect()
    } else {
        vec![]
    })
}

fn convert_fret(value: i16) -> Result<u16, RenderError> {
    u16::try_from(value).map_err(|_| RenderError::invalid_input(format!("negative fret {value}")))
}

fn fingering(
    f: &guitarpro::Fingering,
    right: bool,
    warnings: &mut std::collections::BTreeSet<String>,
) -> Option<String> {
    use guitarpro::Fingering::*;
    let index = match f {
        Open => return None,
        Thumb => 0,
        Index => 1,
        Middle => 2,
        Annular => 3,
        Little => 4,
        Unknown(value) => {
            warnings.insert(format!("Unknown fingering value: {value}"));
            return None;
        }
    };
    Some(
        if right {
            ["p", "i", "m", "a", "c"][index]
        } else {
            ["T", "1", "2", "3", "4"][index]
        }
        .into(),
    )
}

fn natural_harmonic(fret: i32) -> i32 {
    match fret {
        3 => 31,
        4 | 9 | 16 => 28,
        5 | 24 => 24,
        6 | 10 | 14 | 15 => 34,
        7 | 19 => 19,
        8 | 17 | 22 => 36,
        12 => 12,
        _ => 0,
    }
}
