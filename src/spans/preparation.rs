//! Track preparation and automatic span derivation.
use super::*;

pub(crate) fn prepare(track: &Track, options: LayoutOptions) -> Result<Track, RenderError> {
    let mut track = track.clone();
    if options.engraving.display_transposition != 0 {
        let transpose = |pitch: &mut Option<Pitch>| -> Result<(), RenderError> {
            if let Some(value) = pitch {
                let midi = i16::from(value.to_midi()?)
                    + i16::from(options.engraving.display_transposition);
                if !(0..=127).contains(&midi) {
                    return Err(RenderError::invalid_input(
                        "display transposition moves a pitch outside MIDI range".into(),
                    ));
                }
                *value = Pitch::from_midi(midi as u8, value.accidental < 0);
            }
            Ok(())
        };
        for note in track
            .measures
            .iter_mut()
            .flat_map(|m| &mut m.voices)
            .flatten()
            .flat_map(|beat| &mut beat.notes)
        {
            transpose(&mut note.pitch)?;
            transpose(&mut note.effects.grace_pitch)?;
            transpose(&mut note.effects.harmonic_pitch)?;
        }
    }
    if options.engraving.fingering_mode == FingeringMode::Piano {
        let piano = |value: &mut Option<String>| {
            if let Some(label) = value {
                *label = match label.as_str() {
                    "p" | "T" => "1",
                    "i" | "1" => "2",
                    "m" | "2" => "3",
                    "a" | "3" => "4",
                    "c" | "4" => "5",
                    other => other,
                }
                .to_string();
            }
        };
        for note in track
            .measures
            .iter_mut()
            .flat_map(|m| &mut m.voices)
            .flatten()
            .flat_map(|beat| &mut beat.notes)
        {
            piano(&mut note.effects.fingering);
            piano(&mut note.effects.right_fingering);
        }
    }
    for span in &track.spans {
        let first = beat(&track, span.start)?;
        let last = beat(&track, span.end)?;
        if span.start.voice != span.end.voice
            || (span.start.measure, span.start.beat) > (span.end.measure, span.end.beat)
        {
            return Err(RenderError::invalid_input(
                "span endpoints must be ordered in the same voice".into(),
            ));
        }
        if let SpanKind::Slur {
            start_note,
            end_note,
        }
        | SpanKind::Legato {
            start_note,
            end_note,
            ..
        } = span.kind
        {
            if first.notes.is_empty()
                || last.notes.is_empty()
                || start_note.is_some_and(|i| i >= first.notes.len())
                || end_note.is_some_and(|i| i >= last.notes.len())
            {
                return Err(RenderError::invalid_input(
                    "slur endpoints must reference notes".into(),
                ));
            }
        }
        if matches!(span.kind, SpanKind::Beam) {
            if span.start == span.end {
                return Err(RenderError::invalid_input(
                    "a beam group needs at least two beats".into(),
                ));
            }
            for (mi, m) in track.measures.iter().enumerate() {
                if let Some(voice) = m.voices.get(span.start.voice) {
                    for (bi, b) in voice.iter().enumerate() {
                        if contains(
                            span,
                            BeatAddress {
                                measure: mi,
                                voice: span.start.voice,
                                beat: bi,
                            },
                        ) && (b.notes.is_empty()
                            || b.duration.value < 8
                            || b.annotations.stem != first.annotations.stem)
                        {
                            return Err(RenderError::invalid_input("explicit beams require short notes with a consistent stem direction".into()));
                        }
                    }
                }
            }
        }
    }
    for (i, a) in track
        .spans
        .iter()
        .enumerate()
        .filter(|(_, s)| matches!(s.kind, SpanKind::Beam))
    {
        if track.spans[..i].iter().any(|b| {
            matches!(b.kind, SpanKind::Beam) && (contains(a, b.start) || contains(b, a.start))
        }) {
            return Err(RenderError::invalid_input(
                "explicit beam groups cannot overlap".into(),
            ));
        }
    }
    let mut previous: std::collections::HashMap<(usize, usize), OutgoingLegato> =
        std::collections::HashMap::new();
    for (mi, m) in track.measures.iter_mut().enumerate() {
        previous.retain(|(vi, _), _| m.voices.get(*vi).is_some_and(|v| !v.is_empty()));
        for (vi, voice) in m.voices.iter_mut().enumerate() {
            for (bi, b) in voice.iter_mut().enumerate() {
                if b.notes.is_empty() {
                    previous.retain(|(v, _), _| *v != vi);
                }
                let address = BeatAddress {
                    measure: mi,
                    voice: vi,
                    beat: bi,
                };
                for (ni, n) in b.notes.iter_mut().enumerate() {
                    let fret = match n.fret {
                        Fret::Number(f) | Fret::Tied(f) => f,
                        Fret::Dead => 0,
                    };
                    if let Some(&OutgoingLegato {
                        address: start,
                        note: start_note,
                        fret: prior_fret,
                        hammer,
                        slide,
                    }) = previous.get(&(vi, n.string))
                    {
                        if hammer || slide {
                            let text = if hammer {
                                if fret > prior_fret {
                                    "H"
                                } else {
                                    "P"
                                }
                            } else {
                                "sl."
                            };
                            track.spans.push(Span {
                                start,
                                end: address,
                                kind: SpanKind::Legato {
                                    start_note: Some(start_note),
                                    end_note: Some(ni),
                                    text: text.into(),
                                },
                                placement: Placement::Above,
                            });
                        }
                    }
                    previous.insert(
                        (vi, n.string),
                        OutgoingLegato {
                            address,
                            note: ni,
                            fret,
                            hammer: n.effects.hammer_on,
                            slide: n.effects.slide_legato,
                        },
                    );
                    n.effects.hammer_on = false;
                    n.effects.slide_legato = false;
                }
            }
        }
    }
    if options.display == DisplayMode::Slash {
        for span in &mut track.spans {
            if let SpanKind::Slur {
                start_note,
                end_note,
            }
            | SpanKind::Legato {
                start_note,
                end_note,
                ..
            } = &mut span.kind
            {
                *start_note = None;
                *end_note = None;
            }
        }
        track.clef = Clef::Treble;
        for m in &mut track.measures {
            m.clef = None;
            m.key_signature = KeySignature::Natural;
            for b in m.voices.iter_mut().flatten() {
                b.notes.truncate(1);
                if let Some(note) = b.notes.first_mut() {
                    note.pitch = Some(Pitch {
                        step: 6,
                        octave: 4,
                        accidental: 0,
                    });
                    note.effects.head = NoteHead::Slash;
                }
            }
        }
    }
    let voices = track
        .measures
        .iter()
        .map(|m| m.voices.len())
        .max()
        .unwrap_or(0);
    // Group repeated per-beat effects into a single range; silence terminates a run.
    for vi in 0..voices {
        for effect in AutoRange::ALL {
            let mut run: Option<Span> = None;
            for (mi, m) in track.measures.iter_mut().enumerate() {
                let Some(voice) = m.voices.get_mut(vi) else {
                    if let Some(span) = run.take() {
                        track.spans.push(span);
                    }
                    continue;
                };
                if voice.is_empty() {
                    if let Some(span) = run.take() {
                        track.spans.push(span);
                    }
                }
                for (bi, b) in voice.iter_mut().enumerate() {
                    let kind = effect.take(b);
                    let address = BeatAddress {
                        measure: mi,
                        voice: vi,
                        beat: bi,
                    };
                    if let Some(kind) = kind {
                        let same = run
                            .as_ref()
                            .is_some_and(|span| same_range_kind(&span.kind, &kind));
                        if same {
                            run.as_mut().unwrap().end = address;
                        } else {
                            if let Some(span) = run.take() {
                                track.spans.push(span);
                            }
                            run = Some(Span {
                                start: address,
                                end: address,
                                placement: range_placement(&kind),
                                kind,
                            });
                        }
                    } else if let Some(span) = run.take() {
                        track.spans.push(span);
                    }
                }
            }
            if let Some(span) = run {
                track.spans.push(span);
            }
        }
    }
    if !options.show_effects || !options.elements.effects {
        track.spans.retain(|s| {
            matches!(
                s.kind,
                SpanKind::Beam | SpanKind::Slur { .. } | SpanKind::Legato { .. }
            )
        });
    }
    for m in &mut track.measures {
        for b in m.voices.iter_mut().flatten() {
            if !options.show_chords || !options.elements.chord_diagrams {
                b.annotations.chord = None;
            }
            if !options.show_dynamics || !options.elements.dynamics {
                b.annotations.dynamic = None;
            }
            if !options.show_lyrics || !options.elements.lyrics {
                b.annotations.lyrics.clear();
            }
            if !options.show_effects || !options.elements.effects {
                b.annotations.whammy.clear();
                b.annotations.technique = None;
                b.annotations.fade = None;
                b.annotations.pedal = None;
                b.annotations.wah_open = None;
                b.annotations.golpe = false;
                b.annotations.left_hand_tap = false;
                for n in &mut b.notes {
                    let head = n.effects.head;
                    n.effects = NoteEffects {
                        head,
                        ..Default::default()
                    };
                }
            }
        }
    }
    Ok(track)
}
