//! Validation and horizontal measurement performed before geometry emission.
use super::*;

#[derive(Clone)]
/// Horizontal measurements needed to place one measure.
pub(super) struct MeasurePlan {
    /// Rhythmic onset and allocated width for every shared beat column.
    pub(super) columns: Vec<(f64, f32)>,
    /// Total measure width, including its header and trailing padding.
    pub(super) width: f32,
    /// Space reserved for clefs, signatures, and other leading symbols.
    pub(super) header: f32,
}

/// Validates a track and converts every measure into horizontal layout data.
pub(super) fn create_measure_plans(
    track: &Track,
    options: LayoutOptions,
) -> Result<Vec<MeasurePlan>, RenderError> {
    validate_layout_inputs(track, options)?;
    track
        .measures
        .iter()
        .enumerate()
        .map(|(index, measure)| plan_measure(track, measure, index, options))
        .collect()
}

/// Checks constraints that apply to the complete layout request.
fn validate_layout_inputs(track: &Track, options: LayoutOptions) -> Result<(), RenderError> {
    track.validate()?;
    options.validate()?;
    if track.clef != Clef::Percussion
        && options.display.tab()
        && (track.strings.is_empty() || track.strings.len() > 16)
    {
        return Err(RenderError::invalid_input("expected 1–16 strings".into()));
    }
    if !options.width.is_finite()
        || options.width < 160.0
        || !options.string_spacing.is_finite()
        || options.string_spacing < 18.0
        || !options.beat_spacing.is_finite()
        || options.beat_spacing < 32.0
        || !options.engraving.slur_height.is_finite()
        || !(2.0..=80.0).contains(&options.engraving.slur_height)
        || !options.engraving.system_gap.is_finite()
        || !(0.0..=200.0).contains(&options.engraving.system_gap)
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
    options: LayoutOptions,
) -> Result<MeasurePlan, RenderError> {
    validate_measure(measure, index)?;
    let mut columns = Vec::new();
    for voice in &measure.voices {
        let mut time = 0.0;
        for beat in voice {
            if let Some(onset) = beat.start {
                if !onset.is_finite() || onset < time - 1e-8 {
                    return Err(RenderError::invalid_input(
                        "beat onsets must be finite, nonnegative and non-overlapping within a voice"
                            .into(),
                    ));
                }
                time = onset;
            }
            columns.push((time, measure_beat(track, beat, index, options)?));
            time += beat.quarter_beats()?;
        }
    }
    let columns = merge_columns(columns);
    let cancellation = if index > 0
        && track.measures[index - 1].key_signature != measure.key_signature
        && options.display != DisplayMode::Tablature
    {
        track.measures[index - 1].key_signature.unsigned_abs() as f32 * 7.0
    } else {
        0.0
    };
    let header = 106.0 + cancellation;
    let width = (columns.iter().map(|column| column.1).sum::<f32>() + header + 24.0)
        .max(180.0)
        .max(crate::text::width(&measure.marker, 13.0) + 40.0);
    Ok(MeasurePlan {
        columns,
        width,
        header,
    })
}

/// Checks meter, repeat, fermata, and beam-group invariants for one measure.
fn validate_measure(measure: &Measure, index: usize) -> Result<(), RenderError> {
    let (numerator, denominator) = measure.time_signature;
    if numerator == 0
        || !denominator.is_power_of_two()
        || denominator > 128
        || measure.key_signature.unsigned_abs() > 7
    {
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
    if !beam_unit.is_power_of_two() || beam_unit > 256 {
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
fn measure_beat(
    track: &Track,
    beat: &Beat,
    measure: usize,
    options: LayoutOptions,
) -> Result<f32, RenderError> {
    validate_curve(&beat.annotations.whammy, "invalid tremolo-bar curve")?;
    let mut width = options.beat_spacing;
    if options.display == DisplayMode::Numbered {
        width *= beat.duration.undotted_quarters().max(1.0) as f32;
    }
    if !beat.annotations.whammy.is_empty() {
        width = width.max(90.0);
    }
    if beat.annotations.strum_up.is_some() {
        width = width.max(65.0);
    }
    let mut strings = std::collections::HashSet::new();
    for note in &beat.notes {
        width = width.max(validate_note(track, note, measure, options, &mut strings)?);
    }
    width = width
        .max(crate::text::width(&beat.annotations.text, 11.0) + 16.0)
        .max(
            beat.annotations
                .lyrics
                .lines()
                .map(|line| crate::text::width(line, 11.0))
                .fold(0.0, f32::max)
                + 16.0,
        );
    let accidental_count = beat
        .notes
        .iter()
        .filter(|note| note.pitch.is_some_and(|pitch| pitch.accidental != 0))
        .count();
    width += accidental_count.saturating_sub(1) as f32 * 16.0;
    if let Some(chord) = &beat.annotations.chord {
        validate_chord(track, chord)?;
        width = width
            .max(chord.frets.len() as f32 * 10.0 + 30.0)
            .max(crate::text::width(&chord.name, 12.0) + 12.0);
    }
    Ok(width)
}

/// Validates one note and returns the width needed by its displayed label.
fn validate_note(
    track: &Track,
    note: &Note,
    measure: usize,
    options: LayoutOptions,
    strings: &mut std::collections::HashSet<usize>,
) -> Result<f32, RenderError> {
    if track.clef != Clef::Percussion
        && options.display.tab()
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
        pitch.step > 6 || !(-2..=2).contains(&pitch.accidental) || !(-1..=9).contains(&pitch.octave)
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
        && (!note.effects.grace_duration.is_power_of_two() || note.effects.grace_duration > 128)
    {
        return Err(RenderError::invalid_input("invalid grace duration".into()));
    }
    if note.effects.tremolo_slashes > 5 {
        return Err(RenderError::invalid_input(
            "too many tremolo slashes".into(),
        ));
    }
    let mut width = label(note).len() as f32 * 10.0 + 24.0;
    if note.effects.slide_in.is_some() || note.effects.slide_out.is_some() {
        width = width.max(75.0);
    }
    if note.effects.grace_fret.is_some() || !note.effects.bend.is_empty() {
        width = width.max(80.0);
    }
    Ok(width)
}

/// Checks that a normalized effect curve is finite and moves forward in time.
fn validate_curve(points: &[[f32; 2]], message: &str) -> Result<(), RenderError> {
    let mut previous = -1.0;
    for point in points {
        if !point[0].is_finite()
            || !point[1].is_finite()
            || !(0.0..=1.0).contains(&point[0])
            || point[0] < previous
            || point[1].abs() > 12.0
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
        || chord.frets.len() > 16
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
            .filter(|column| (column.0 - time).abs() < 1e-8)
        {
            last.1 = last.1.max(width);
        } else {
            merged.push((time, width));
        }
    }
    merged
}
