//! Conversion between the external `guitarpro` model and this crate's compact score model.
//!
//! ## Legacy Guitar Pro beat time
//!
//! A legacy Guitar Pro beat can carry a timestamp in *source ticks*. There are
//! 960 ticks in one quarter note, and the timestamp origin within every measure
//! is itself tick 960—not zero. Consequently, tick 960 is the start of the
//! measure (`0.0` quarter notes), tick 1920 is one quarter note into it, and so
//! on. [`crate::guitar_pro::convert_track`] translates those values with:
//!
//! ```text
//! onset_in_quarter_notes = (source_ticks - 960) / 960
//! ```
//!
//! A beat without a source timestamp follows sequentially after the prior beat
//! in its own voice.

use crate::{Bar, Beat, Duration, Fret, Note, RenderError, Track, Voice};

/// Legacy Guitar Pro measures beat positions in 960 source ticks per quarter note.
///
/// These are file-format ticks, not MIDI ticks and not renderer units.
const QUARTER_TICKS: f64 = 960.0;
/// Legacy Guitar Pro's per-measure timestamp origin.
///
/// The first beat is stored at tick `960`, so tick `960` becomes `0.0` quarter
/// notes and tick `1920` becomes `1.0` quarter notes.
const BEAT_ORIGIN_TICKS: i64 = 960;

/// Native track plus non-fatal import diagnostics.
#[derive(Debug)]
pub struct ImportReport {
    /// The compact score model used by this crate.
    pub track: Track,
    /// Source features that could not be represented.
    pub warnings: Vec<String>,
}

/// Converts a legacy Guitar Pro per-measure timestamp to quarter-note time.
///
/// Guitar Pro stores the measure origin as tick 960 and uses 960 ticks per
/// quarter note, so tick 960 becomes `0.0` and tick 1920 becomes `1.0`.
fn convert_start_ticks_to_quarter_notes(ticks: i64) -> f64 {
    (ticks - BEAT_ORIGIN_TICKS) as f64 / QUARTER_TICKS
}

/// Converts Guitar Pro's duration flags and tuplet pair to a native duration.
///
/// It copies the note value (for example, `4` for a quarter note), converts the
/// dot flags to a count (`0`, `1`, or `2`), and rejects a value that does not
/// fit in the native representation.
///
/// Guitar Pro writes a normal duration's tuplet as `(1, 1)`. We store that as
/// `None`; an actual triplet remains `Some((3, 2))`.
fn convert_duration(
    source: &guitarpro::model::legacy::key_signature::Duration,
) -> Result<Duration, RenderError> {
    let tuplet = (source.tuplet_enters, source.tuplet_times);
    Ok(Duration {
        value: i16::try_from(source.value).map_err(|_| RenderError("invalid duration".into()))?,
        dots: if source.double_dotted {
            2
        } else {
            u8::from(source.dotted)
        },
        tuplet: (tuplet != (1, 1)).then_some(tuplet),
    })
}

/// Converts all Guitar Pro notes at one rhythmic position to native tab notes.
///
/// Rest notes are omitted because an empty native note list represents a rest.
/// For pitched tracks, this also derives the MIDI value required by staff mode.
fn convert_notes(
    source_track: &guitarpro::Track,
    source_beat: &guitarpro::Beat,
    strings: &[u8],
    warnings: &mut Vec<String>,
) -> Result<Vec<Note>, RenderError> {
    let mut notes = Vec::new();
    for source_note in &source_beat.notes {
        if source_note.kind == guitarpro::NoteType::Rest {
            continue;
        }
        let string = usize::try_from(source_note.string)
            .map_err(|_| RenderError("invalid string index".into()))?;
        let fret =
            u16::try_from(source_note.value).map_err(|_| RenderError("negative fret".into()))?;
        let fret_value = match source_note.kind {
            guitarpro::NoteType::Dead => Fret::Dead,
            guitarpro::NoteType::Tie => Fret::Tied(fret),
            guitarpro::NoteType::Normal => Fret::Number(fret),
            _ => {
                warnings.push("Unknown note kind omitted".into());
                continue;
            }
        };
        let midi = if source_track.percussion_track {
            None
        } else {
            let open = strings
                .get(
                    string
                        .checked_sub(1)
                        .ok_or_else(|| RenderError("zero string index".into()))?,
                )
                .ok_or_else(|| RenderError("string index out of range".into()))?;
            let midi = i32::from(*open) + i32::from(fret) + i32::from(source_track.offset)
                - i32::from(source_track.transpose_chromatic)
                - i32::from(source_track.transpose_octave) * 12;
            Some(
                u8::try_from(midi)
                    .map_err(|_| RenderError("written pitch outside MIDI range".into()))?,
            )
        };
        notes.push(Note {
            string,
            fret: fret_value,
            midi,
        });
    }
    Ok(notes)
}

/// Converts one Guitar Pro track using its song headers and tuning information.
pub fn convert_track(
    song: &guitarpro::Song,
    source: &guitarpro::Track,
) -> Result<ImportReport, RenderError> {
    // Keep recoverable omissions separate from malformed source data, which is
    // returned as an error below.
    let mut warnings = vec![];

    // Normalize the Guitar Pro `(string_number, MIDI_pitch)` tuples to the
    // top-to-bottom open-string tuning used by `Track`.
    // Example:
    //      [(1, 64), (2, 59), (3, 55)] becomes [64, 59, 55]
    let strings = source
        .strings
        .iter()
        .map(|(_, midi)| {
            u8::try_from(*midi).map_err(|_| RenderError("invalid string tuning".into()))
        })
        .collect::<Result<Vec<_>, _>>()?;

    // Convert measures in source order.
    // Headers own the meter; track measures own the voices and notes.
    let mut bars = Vec::with_capacity(source.measures.len());
    for (mi, source_measure) in source.measures.iter().enumerate() {
        let header = song
            .measure_headers
            .get(mi)
            .ok_or_else(|| RenderError(format!("missing header for measure {}", mi + 1)))?;
        let numerator = u8::try_from(header.time_signature.numerator)
            .map_err(|_| RenderError("invalid meter".into()))?;
        let denominator = u16::try_from(header.time_signature.denominator.value)
            .map_err(|_| RenderError("invalid meter".into()))?;
        let mut voices = Vec::new();

        // Voices share the measure origin, but each voice advances independently.
        for source_voice in &source_measure.voices {
            let mut beats = Vec::new();
            let mut onset = 0.0;
            for source_beat in &source_voice.beats {
                // Empty beats are source timeline placeholders, not written rests.
                if source_beat.status == guitarpro::BeatStatus::Empty {
                    continue;
                }

                // Preserve written rhythm: denominator, dots, and tuplets.
                let duration = convert_duration(&source_beat.duration)?;

                // Legacy timestamps are per-measure source ticks: tick 960 is
                // the measure origin, and each further 960 ticks is one quarter
                // note. Do not move backward over the prior sequential beat.
                if let Some(ticks) = source_beat.start {
                    onset = convert_start_ticks_to_quarter_notes(ticks).max(onset);
                }

                let notes = convert_notes(source, source_beat, &strings, &mut warnings)?;
                beats.push(Beat {
                    start: Some(onset),
                    duration,
                    notes,
                });

                onset += duration.compute_quarter_beats()?;
            }
            voices.push(Voice { beats });
        }
        bars.push(Bar {
            time_signature: (numerator, denominator),
            voices,
        });
    }

    // Report each recoverable omission once.
    warnings.sort();
    warnings.dedup();

    // Return a source-independent score model to the renderer.
    Ok(ImportReport {
        track: Track {
            name: source.name.clone(),
            strings,
            bars,
        },
        warnings,
    })
}

#[cfg(test)]
mod tests {
    use super::{convert_duration, convert_notes, convert_start_ticks_to_quarter_notes};
    use crate::Fret;

    #[test]
    fn converts_legacy_measure_origin_and_quarter_note() {
        const MEASURE_ORIGIN_TICKS: i64 = 960;
        const NEXT_QUARTER_TICKS: i64 = 1920;
        assert_eq!(
            convert_start_ticks_to_quarter_notes(MEASURE_ORIGIN_TICKS),
            0.0
        );
        assert_eq!(
            convert_start_ticks_to_quarter_notes(NEXT_QUARTER_TICKS),
            1.0
        );
    }

    #[test]
    fn converts_guitar_pro_duration_flags_and_tuplets() {
        use guitarpro::model::legacy::key_signature::Duration as GuitarProDuration;

        let source = GuitarProDuration {
            value: 8,
            dotted: false,
            double_dotted: true,
            min_time: 0,
            tuplet_enters: 3,
            tuplet_times: 2,
        };
        let duration = convert_duration(&source).unwrap();

        assert_eq!(duration.value, 8);
        assert_eq!(duration.dots, 2);
        assert_eq!(duration.tuplet, Some((3, 2)));
    }

    #[test]
    fn omits_guitar_pro_identity_tuplet() {
        let duration = convert_duration(&Default::default()).unwrap();

        assert_eq!(duration.tuplet, None);
    }

    #[test]
    fn converts_normal_dead_and_tied_notes_and_omits_rests() {
        use guitarpro::{Beat, Note, NoteType, Track};

        let source_track = Track::default();
        let source_beat = Beat {
            notes: vec![
                Note {
                    string: 1,
                    value: 3,
                    kind: NoteType::Normal,
                    ..Default::default()
                },
                Note {
                    string: 2,
                    value: 0,
                    kind: NoteType::Dead,
                    ..Default::default()
                },
                Note {
                    string: 3,
                    value: 5,
                    kind: NoteType::Tie,
                    ..Default::default()
                },
                Note {
                    kind: NoteType::Rest,
                    ..Default::default()
                },
            ],
            ..Default::default()
        };
        let mut warnings = vec![];

        let notes =
            convert_notes(&source_track, &source_beat, &[64, 59, 55], &mut warnings).unwrap();

        assert_eq!(notes.len(), 3);
        assert_eq!(notes[0].fret, Fret::Number(3));
        assert_eq!(notes[0].midi, Some(67));
        assert_eq!(notes[1].fret, Fret::Dead);
        assert_eq!(notes[2].fret, Fret::Tied(5));
        assert!(warnings.is_empty());
    }
}
