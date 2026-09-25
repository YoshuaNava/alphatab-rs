//! Non-destructive preparation of rendering-only state.
use super::*;
use std::borrow::Cow;

/// Rendering facts derived from a source [`Track`].  This type never owns or
/// borrows the score: transformations are queried by the engraver.
#[derive(Clone, Debug, Default)]
pub(crate) struct RenderState {
    generated_spans: Vec<Span>,
}

impl RenderState {
    /// Returns the beat as it should appear for this request.  The borrowed
    /// source is returned unchanged when no display transformation applies.
    pub(crate) fn beat<'a>(
        &self,
        source: &'a Beat,
        options: SceneOptions,
    ) -> Result<Cow<'a, Beat>, RenderError> {
        let effects = options.show_effects && options.elements.effects;
        let needs_copy = options.engraving.display_transposition != 0
            || options.engraving.fingering_mode == FingeringMode::Piano
            || options.display == DisplayMode::Slash
            || !options.show_chords
            || !options.elements.chord_diagrams
            || !options.show_dynamics
            || !options.elements.dynamics
            || !options.show_lyrics
            || !options.elements.lyrics
            || !effects;
        if !needs_copy {
            return Ok(Cow::Borrowed(source));
        }
        let mut beat = source.clone();
        let transpose = |pitch: &mut Option<Pitch>| -> Result<(), RenderError> {
            if let Some(value) = pitch {
                let midi = i16::from(value.to_midi()?)
                    + i16::from(options.engraving.display_transposition);
                *value = Pitch::from_midi(midi as u8, value.accidental < 0);
            }
            Ok(())
        };
        for note in &mut beat.notes {
            transpose(&mut note.pitch)?;
            transpose(&mut note.effects.grace_pitch)?;
            transpose(&mut note.effects.harmonic_pitch)?;
            if options.engraving.fingering_mode == FingeringMode::Piano {
                for label in [
                    &mut note.effects.fingering,
                    &mut note.effects.right_fingering,
                ]
                .into_iter()
                .flatten()
                {
                    *label = match label.as_str() {
                        "p" | "T" => "1",
                        "i" | "1" => "2",
                        "m" | "2" => "3",
                        "a" | "3" => "4",
                        "c" | "4" => "5",
                        other => other,
                    }
                    .into();
                }
            }
            // Ranges are drawn from generated spans, rather than locally.
            note.effects.hammer_on = false;
            note.effects.slide_legato = false;
            note.effects.palm_mute = false;
            note.effects.let_ring = false;
        }
        if options.display == DisplayMode::Slash {
            beat.notes.truncate(1);
            if let Some(note) = beat.notes.first_mut() {
                note.pitch = Some(Pitch {
                    step: 6,
                    octave: 4,
                    accidental: 0,
                });
                note.effects.head = NoteHead::Slash;
            }
        }
        if !options.show_chords || !options.elements.chord_diagrams {
            beat.annotations.chord = None;
        }
        if !options.show_dynamics || !options.elements.dynamics {
            beat.annotations.dynamic = None;
        }
        if !options.show_lyrics || !options.elements.lyrics {
            beat.annotations.lyrics.clear();
        }
        if !effects {
            beat.annotations.whammy.clear();
            beat.annotations.technique = None;
            beat.annotations.fade = None;
            beat.annotations.pedal = None;
            beat.annotations.wah_open = None;
            beat.annotations.golpe = false;
            beat.annotations.left_hand_tap = false;
            for note in &mut beat.notes {
                let head = note.effects.head;
                note.effects = NoteEffects {
                    head,
                    ..Default::default()
                };
            }
        }
        Ok(Cow::Owned(beat))
    }

    pub(crate) fn spans(&self, track: &Track, options: SceneOptions) -> Vec<Span> {
        let mut result = track.spans.clone();
        result.extend(self.generated_spans.clone());
        if options.display == DisplayMode::Slash {
            for span in &mut result {
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
        }
        if !options.show_effects || !options.elements.effects {
            result.retain(|span| {
                matches!(
                    span.kind,
                    SpanKind::Beam | SpanKind::Slur { .. } | SpanKind::Legato { .. }
                )
            });
        }
        result
    }
}

/// Validates source spans and derives automatic legato/effect spans.  No field
/// of the supplied track is rewritten.
pub(crate) fn prepare(track: &Track, options: SceneOptions) -> Result<RenderState, RenderError> {
    for span in &track.spans {
        let first = beat(track, span.start)?;
        let last = beat(track, span.end)?;
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
            for (measure, item) in track.measures.iter().enumerate() {
                if let Some(voice) = item.voices.get(span.start.voice) {
                    for (beat, value) in voice.iter().enumerate() {
                        if contains(
                            span,
                            BeatAddress {
                                measure,
                                voice: span.start.voice,
                                beat,
                            },
                        ) && (value.notes.is_empty()
                            || value.duration.value < 8
                            || value.annotations.stem != first.annotations.stem)
                        {
                            return Err(RenderError::invalid_input("explicit beams require short notes with a consistent stem direction".into()));
                        }
                    }
                }
            }
        }
    }
    for (index, span) in track
        .spans
        .iter()
        .enumerate()
        .filter(|(_, span)| matches!(span.kind, SpanKind::Beam))
    {
        if track.spans[..index].iter().any(|other| {
            matches!(other.kind, SpanKind::Beam)
                && (contains(span, other.start) || contains(other, span.start))
        }) {
            return Err(RenderError::invalid_input(
                "explicit beam groups cannot overlap".into(),
            ));
        }
    }
    if options.engraving.display_transposition != 0 {
        for note in track
            .measures
            .iter()
            .flat_map(|m| &m.voices)
            .flatten()
            .flat_map(|b| &b.notes)
        {
            for pitch in [
                note.pitch,
                note.effects.grace_pitch,
                note.effects.harmonic_pitch,
            ]
            .into_iter()
            .flatten()
            {
                let value = i16::from(pitch.to_midi()?)
                    + i16::from(options.engraving.display_transposition);
                if !(0..=127).contains(&value) {
                    return Err(RenderError::invalid_input(
                        "display transposition moves a pitch outside MIDI range".into(),
                    ));
                }
            }
        }
    }
    let mut generated_spans = Vec::new();
    let mut previous = std::collections::HashMap::new();
    for (mi, measure) in track.measures.iter().enumerate() {
        for (vi, voice) in measure.voices.iter().enumerate() {
            for (bi, item) in voice.iter().enumerate() {
                let address = BeatAddress {
                    measure: mi,
                    voice: vi,
                    beat: bi,
                };
                for (ni, note) in item.notes.iter().enumerate() {
                    let fret = match note.fret {
                        Fret::Number(v) | Fret::Tied(v) => v,
                        Fret::Dead => 0,
                    };
                    if let Some(&(start, start_note, prior, hammer, slide)) =
                        previous.get(&(vi, note.string))
                    {
                        if hammer || slide {
                            generated_spans.push(Span {
                                start,
                                end: address,
                                kind: SpanKind::Legato {
                                    start_note: Some(start_note),
                                    end_note: Some(ni),
                                    text: if hammer {
                                        if fret > prior {
                                            "H"
                                        } else {
                                            "P"
                                        }
                                    } else {
                                        "sl."
                                    }
                                    .into(),
                                },
                                placement: Placement::Above,
                            });
                        }
                    }
                    previous.insert(
                        (vi, note.string),
                        (
                            address,
                            ni,
                            fret,
                            note.effects.hammer_on,
                            note.effects.slide_legato,
                        ),
                    );
                }
            }
        }
    }
    let voices = track
        .measures
        .iter()
        .map(|measure| measure.voices.len())
        .max()
        .unwrap_or(0);
    for voice in 0..voices {
        for effect in AutoRange::ALL {
            let mut run: Option<Span> = None;
            for (measure, item) in track.measures.iter().enumerate() {
                let Some(beats) = item.voices.get(voice) else {
                    if let Some(span) = run.take() {
                        generated_spans.push(span);
                    }
                    continue;
                };
                for (beat, value) in beats.iter().enumerate() {
                    let address = BeatAddress {
                        measure,
                        voice,
                        beat,
                    };
                    if let Some(kind) = effect.kind(value) {
                        if run
                            .as_ref()
                            .is_some_and(|span| same_range_kind(&span.kind, &kind))
                        {
                            run.as_mut().unwrap().end = address;
                        } else {
                            if let Some(span) = run.take() {
                                generated_spans.push(span);
                            }
                            run = Some(Span {
                                start: address,
                                end: address,
                                placement: range_placement(&kind),
                                kind,
                            });
                        }
                    } else if let Some(span) = run.take() {
                        generated_spans.push(span);
                    }
                }
            }
            if let Some(span) = run {
                generated_spans.push(span);
            }
        }
    }
    Ok(RenderState { generated_spans })
}
