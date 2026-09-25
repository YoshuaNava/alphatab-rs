//! Validation and horizontal scene measurement before geometry emission.
use crate::{Beat, ChordDiagram, Clef, DisplayMode, Measure, Note, RenderError, Track};

use super::build::format_fret_label;
use super::parameters::{
    ANNOTATION_TEXT_SIZE, DOT_SPACING, EMPHASIZED_TEXT_SIZE, LARGE_TEXT_SIZE, SMALL_GLYPH_SIZE,
    STAFF_HEIGHT, STAFF_LINE_SPACING, STAFF_MIDDLE_LINE_OFFSET, TIMELINE_EPSILON,
};
use super::SceneOptions;

// Minimum horizontal allocations keep dense notation legible before row layout.
const MAXIMUM_STRING_COUNT: usize = 16;
const MINIMUM_SCENE_WIDTH: f32 = 160.0;
const MINIMUM_STRING_SPACING: f32 = 18.0;
const MINIMUM_BEAT_SPACING: f32 = 32.0;
const MINIMUM_SLUR_HEIGHT: f32 = 2.0;
const MAXIMUM_SLUR_HEIGHT: f32 = 80.0;
const MAXIMUM_SYSTEM_GAP: f32 = 200.0;
const MAXIMUM_METER_DENOMINATOR: u16 = 128;
const MAXIMUM_BEAM_UNIT: u16 = 256;
const NUMBERED_MINIMUM_BEAT_DURATION: f64 = 1.0;
const MAXIMUM_PITCH_STEP: u8 = 6;
const MINIMUM_ACCIDENTAL: i8 = -2;
const MAXIMUM_ACCIDENTAL: i8 = 2;
const MINIMUM_OCTAVE: i8 = -1;
const MAXIMUM_OCTAVE: i8 = 9;
const MAXIMUM_GRACE_DURATION: u16 = 128;
const MAXIMUM_TREMOLO_SLASHES: u8 = 5;
const CURVE_INITIAL_TIME: f32 = -1.0;
const CURVE_MINIMUM_TIME: f32 = 0.0;
const CURVE_MAXIMUM_TIME: f32 = 1.0;
const MAXIMUM_CURVE_SEMITONES: f32 = 12.0;

#[derive(Clone)]
/// Horizontal measurements needed to place one measure.
pub(crate) struct MeasurePlan {
    /// Rhythmic onset and allocated width for every shared beat column.
    pub(crate) columns: Vec<(f64, f32)>,
    /// Total measure width, including its header and trailing padding.
    pub(crate) width: f32,
    /// Space reserved for clefs, signatures, and other leading symbols.
    pub(crate) header: f32,
}

/// Validates a track and converts every measure into horizontal layout data.
pub(super) fn create_measure_plans(
    track: &Track,
    options: SceneOptions,
) -> Result<Vec<MeasurePlan>, RenderError> {
    validate_scene_inputs(track, options)?;
    track
        .measures
        .iter()
        .enumerate()
        .map(|(index, measure)| plan_measure(track, measure, index, options))
        .collect()
}

/// Checks constraints that apply to the complete layout request.
fn validate_scene_inputs(track: &Track, options: SceneOptions) -> Result<(), RenderError> {
    track.validate()?;
    options.validate()?;
    if track.clef != Clef::Percussion
        && options.display.renders_tab()
        && (track.strings.is_empty() || track.strings.len() > MAXIMUM_STRING_COUNT)
    {
        return Err(RenderError::invalid_input("expected 1–16 strings".into()));
    }
    if !options.width.is_finite()
        || options.width < MINIMUM_SCENE_WIDTH
        || !options.string_spacing.is_finite()
        || options.string_spacing < MINIMUM_STRING_SPACING
        || !options.beat_spacing.is_finite()
        || options.beat_spacing < MINIMUM_BEAT_SPACING
        || !options.engraving.slur_height.is_finite()
        || !(MINIMUM_SLUR_HEIGHT..=MAXIMUM_SLUR_HEIGHT).contains(&options.engraving.slur_height)
        || !options.engraving.system_gap.is_finite()
        || !(0.0..=MAXIMUM_SYSTEM_GAP).contains(&options.engraving.system_gap)
    {
        return Err(RenderError::invalid_input(
            "invalid layout dimensions".into(),
        ));
    }
    if options.bars_per_system == Some(0) {
        return Err(RenderError::invalid_input(
            "bars per system must be positive".into(),
        ));
    }
    Ok(())
}

/// Validates and measures one measure without emitting drawing primitives.
fn plan_measure(
    track: &Track,
    measure: &Measure,
    index: usize,
    options: SceneOptions,
) -> Result<MeasurePlan, RenderError> {
    validate_measure(measure, index)?;
    let mut columns = Vec::new();
    for voice in &measure.voices {
        let mut time = 0.0;
        for beat in voice {
            if let Some(onset) = beat.start {
                if !onset.is_finite() || onset < time - TIMELINE_EPSILON {
                    return Err(RenderError::invalid_input(
                        "beat onsets must be finite, nonnegative and non-overlapping within a voice"
                            .into(),
                    ));
                }
                time = onset;
            }
            columns.push((time, compute_beat_width(track, beat, index, options)?));
            time += beat.compute_quarter_beats()?;
        }
    }
    let columns = merge_columns(columns);
    let cancellation = if index > 0
        && track.measures[index - 1].key_signature != measure.key_signature
        && options.display != DisplayMode::Tablature
    {
        track.measures[index - 1].key_signature.accidental_count() as f32 * 7.0
    } else {
        0.0
    };
    let header = STAFF_HEIGHT * 2.0 + LARGE_TEXT_SIZE + SMALL_GLYPH_SIZE + cancellation;
    let width = (columns.iter().map(|column| column.1).sum::<f32>()
        + header
        + STAFF_MIDDLE_LINE_OFFSET
        + DOT_SPACING)
        .max(STAFF_HEIGHT * 4.0 + STAFF_LINE_SPACING * 2.0)
        .max(crate::text::width(&measure.marker, EMPHASIZED_TEXT_SIZE) + STAFF_HEIGHT);
    Ok(MeasurePlan {
        columns,
        width,
        header,
    })
}

/// Checks meter, repeat, fermata, and beam-group invariants for one measure.
fn validate_measure(measure: &Measure, index: usize) -> Result<(), RenderError> {
    let (numerator, denominator) = measure.time_signature;
    if numerator == 0 || !denominator.is_power_of_two() || denominator > MAXIMUM_METER_DENOMINATOR {
        return Err(RenderError::invalid_input(format!(
            "invalid meter or key in measure {}",
            index + 1
        )));
    }
    if measure
        .repeat_count
        .is_some_and(|count| count == 0 || !measure.repeat_end)
    {
        return Err(RenderError::invalid_input(
            "repeat count requires an end-repeat and at least one pass".into(),
        ));
    }
    if measure
        .fermatas
        .iter()
        .any(|fermata| !fermata.start.is_finite() || fermata.start < 0.0)
    {
        return Err(RenderError::invalid_input("invalid fermata onset".into()));
    }
    let beam_unit = measure.beam_unit.unwrap_or(denominator);
    if !beam_unit.is_power_of_two() || beam_unit > MAXIMUM_BEAM_UNIT {
        return Err(RenderError::invalid_input("invalid beam unit".into()));
    }
    if !measure.beam_groups.is_empty()
        && (measure.beam_groups.contains(&0)
            || measure
                .beam_groups
                .iter()
                .map(|group| u32::from(*group))
                .sum::<u32>()
                * u32::from(denominator)
                != u32::from(numerator) * u32::from(beam_unit))
    {
        return Err(RenderError::invalid_input(
            "beam groups must sum to the meter numerator".into(),
        ));
    }
    Ok(())
}

/// Returns the minimum horizontal space required by a beat and its contents.
fn compute_beat_width(
    track: &Track,
    beat: &Beat,
    measure: usize,
    options: SceneOptions,
) -> Result<f32, RenderError> {
    validate_curve(&beat.annotations.whammy, "invalid tremolo-bar curve")?;
    let mut width = options.beat_spacing;
    if options.display == DisplayMode::Numbered {
        width *= beat
            .duration
            .undotted_quarter_beats()
            .max(NUMBERED_MINIMUM_BEAT_DURATION) as f32;
    }
    if !beat.annotations.whammy.is_empty() {
        width = width.max(STAFF_HEIGHT * 2.0 + STAFF_LINE_SPACING);
    }
    if beat.annotations.strum_up.is_some() {
        width = width.max(STAFF_HEIGHT + STAFF_MIDDLE_LINE_OFFSET + SMALL_GLYPH_SIZE);
    }
    let mut strings = std::collections::HashSet::new();
    for note in &beat.notes {
        width = width.max(validate_note(track, note, measure, options, &mut strings)?);
    }
    width = width
        .max(
            crate::text::width(&beat.annotations.text, ANNOTATION_TEXT_SIZE)
                + STAFF_MIDDLE_LINE_OFFSET
                - DOT_SPACING,
        )
        .max(
            beat.annotations
                .lyrics
                .lines()
                .map(|line| crate::text::width(line, ANNOTATION_TEXT_SIZE))
                .fold(0.0, f32::max)
                + STAFF_MIDDLE_LINE_OFFSET
                - DOT_SPACING,
        );
    let accidental_count = beat
        .notes
        .iter()
        .filter(|note| note.pitch.is_some_and(|pitch| pitch.accidental != 0))
        .count();
    width += accidental_count.saturating_sub(1) as f32 * (STAFF_MIDDLE_LINE_OFFSET - DOT_SPACING);
    if let Some(chord) = &beat.annotations.chord {
        validate_chord(track, chord)?;
        width = width
            .max(chord.frets.len() as f32 * STAFF_LINE_SPACING + STAFF_HEIGHT - STAFF_LINE_SPACING)
            .max(crate::text::width(&chord.name, EMPHASIZED_TEXT_SIZE) + EMPHASIZED_TEXT_SIZE);
    }
    Ok(width)
}

/// Validates one note and returns the width needed by its displayed label.
fn validate_note(
    track: &Track,
    note: &Note,
    measure: usize,
    options: SceneOptions,
    strings: &mut std::collections::HashSet<usize>,
) -> Result<f32, RenderError> {
    if track.clef != Clef::Percussion
        && options.display.renders_tab()
        && (note.string == 0 || note.string > track.strings.len() || !strings.insert(note.string))
    {
        return Err(RenderError::invalid_input(format!(
            "invalid or duplicate string in measure {}",
            measure + 1
        )));
    }
    if let Some(id) = note.percussion {
        crate::percussion::resolve(id)?;
    }
    if (options.display != DisplayMode::Tablature || track.clef == Clef::Percussion)
        && note.pitch.is_none()
    {
        return Err(RenderError::invalid_input(format!(
            "standard notation needs a written pitch in measure {}",
            measure + 1
        )));
    }
    if [
        note.pitch,
        note.effects.grace_pitch,
        note.effects.harmonic_pitch,
    ]
    .into_iter()
    .flatten()
    .any(|pitch| {
        pitch.step > MAXIMUM_PITCH_STEP
            || !(MINIMUM_ACCIDENTAL..=MAXIMUM_ACCIDENTAL).contains(&pitch.accidental)
            || !(MINIMUM_OCTAVE..=MAXIMUM_OCTAVE).contains(&pitch.octave)
    }) {
        return Err(RenderError::invalid_input("invalid written pitch".into()));
    }
    validate_curve(&note.effects.bend, "invalid bend curve")?;
    if !(-1..=1).contains(&note.effects.quarter_tone) {
        return Err(RenderError::invalid_input(
            "invalid quarter-tone offset".into(),
        ));
    }
    if note.effects.grace_duration > 0
        && (!note.effects.grace_duration.is_power_of_two()
            || note.effects.grace_duration > MAXIMUM_GRACE_DURATION)
    {
        return Err(RenderError::invalid_input("invalid grace duration".into()));
    }
    if note.effects.tremolo_slashes > MAXIMUM_TREMOLO_SLASHES {
        return Err(RenderError::invalid_input(
            "too many tremolo slashes".into(),
        ));
    }
    let mut width = format_fret_label(note).len() as f32 * STAFF_LINE_SPACING
        + STAFF_MIDDLE_LINE_OFFSET
        + DOT_SPACING;
    if note.effects.slide_in.is_some() || note.effects.slide_out.is_some() {
        width = width.max(STAFF_HEIGHT + STAFF_MIDDLE_LINE_OFFSET + SMALL_GLYPH_SIZE);
    }
    if note.effects.grace_fret.is_some() || !note.effects.bend.is_empty() {
        width = width.max(STAFF_HEIGHT * 2.0);
    }
    Ok(width)
}

/// Checks that a normalized effect curve is finite and moves forward in time.
fn validate_curve(points: &[[f32; 2]], message: &str) -> Result<(), RenderError> {
    let mut previous = CURVE_INITIAL_TIME;
    for point in points {
        if !point[0].is_finite()
            || !point[1].is_finite()
            || !(CURVE_MINIMUM_TIME..=CURVE_MAXIMUM_TIME).contains(&point[0])
            || point[0] < previous
            || point[1].abs() > MAXIMUM_CURVE_SEMITONES
        {
            return Err(RenderError::invalid_input(message.into()));
        }
        previous = point[0];
    }
    Ok(())
}

/// Checks that a chord diagram is compatible with the current instrument.
fn validate_chord(track: &Track, chord: &ChordDiagram) -> Result<(), RenderError> {
    if chord.frets.is_empty()
        || chord.frets.len() > MAXIMUM_STRING_COUNT
        || (!track.strings.is_empty() && chord.frets.len() != track.strings.len())
        || chord.first_fret == 0
        || chord
            .frets
            .iter()
            .flatten()
            .any(|fret| *fret != 0 && *fret < chord.first_fret)
    {
        return Err(RenderError::invalid_input(
            "chord frets must be open or at/above the first fret".into(),
        ));
    }
    if !chord.fingers.is_empty() && chord.fingers.len() != chord.frets.len() {
        return Err(RenderError::invalid_input(
            "chord fingerings must match its strings".into(),
        ));
    }
    if chord.barres.iter().any(|barre| {
        barre.first_string == 0
            || barre.first_string > barre.last_string
            || barre.last_string > chord.frets.len()
            || barre.fret < chord.first_fret
    }) {
        return Err(RenderError::invalid_input("invalid chord barre".into()));
    }
    Ok(())
}

/// Coalesces simultaneous beat columns while retaining the widest requirement.
fn merge_columns(mut columns: Vec<(f64, f32)>) -> Vec<(f64, f32)> {
    columns.sort_by(|left, right| left.0.total_cmp(&right.0));
    let mut merged: Vec<(f64, f32)> = Vec::new();
    for (time, width) in columns {
        if let Some(last) = merged
            .last_mut()
            .filter(|column| (column.0 - time).abs() < TIMELINE_EPSILON)
        {
            last.1 = last.1.max(width);
        } else {
            merged.push((time, width));
        }
    }
    merged
}
