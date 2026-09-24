//! Standalone model validation and resource limits.

use crate::{Beat, LayoutOptions, RenderError, ScoreDocument, Track};

/// Maximum measures accepted in one track layout.
pub const MAX_MEASURES: usize = 100_000;
/// Maximum voices accepted in one measure.
pub const MAX_VOICES_PER_MEASURE: usize = 32;
/// Maximum beats accepted in one voice.
pub const MAX_BEATS_PER_VOICE: usize = 100_000;
/// Maximum simultaneous notes accepted in one beat.
pub const MAX_NOTES_PER_BEAT: usize = 64;
/// Maximum control points accepted in a bend or whammy curve.
pub const MAX_CURVE_POINTS: usize = 4_096;
/// Maximum staves accepted in a score document.
pub const MAX_SCORE_STAVES: usize = 1_024;

fn validate_beat(beat: &Beat) -> Result<(), RenderError> {
    beat.compute_quarter_beats()?;
    if beat.notes.len() > MAX_NOTES_PER_BEAT {
        return Err(RenderError::resource_limit(format!(
            "a beat exceeds the {MAX_NOTES_PER_BEAT}-note resource limit"
        )));
    }
    if beat.annotations.whammy.len() > MAX_CURVE_POINTS {
        return Err(RenderError::resource_limit(format!(
            "a whammy curve exceeds the {MAX_CURVE_POINTS}-point resource limit"
        )));
    }
    for note in &beat.notes {
        if note.effects.bend.len() > MAX_CURVE_POINTS {
            return Err(RenderError::resource_limit(format!(
                "a bend curve exceeds the {MAX_CURVE_POINTS}-point resource limit"
            )));
        }
    }
    Ok(())
}

impl Track {
    /// Validates model invariants and resource limits without performing layout.
    pub fn validate(&self) -> Result<(), RenderError> {
        if self.measures.len() > MAX_MEASURES {
            return Err(RenderError::resource_limit(format!(
                "a track exceeds the {MAX_MEASURES}-measure resource limit"
            )));
        }
        for (measure_index, measure) in self.measures.iter().enumerate() {
            let (numerator, denominator) = measure.time_signature;
            if numerator == 0 || !denominator.is_power_of_two() || denominator > 128 {
                return Err(RenderError::resource_limit(format!(
                    "invalid meter in measure {}",
                    measure_index + 1
                )));
            }
            if measure.key_signature.unsigned_abs() > 7 {
                return Err(RenderError::invalid_input(format!(
                    "invalid key signature in measure {}",
                    measure_index + 1
                )));
            }
            if measure.voices.len() > MAX_VOICES_PER_MEASURE {
                return Err(RenderError::invalid_input(format!(
                    "measure {} exceeds the {MAX_VOICES_PER_MEASURE}-voice resource limit",
                    measure_index + 1
                )));
            }
            for voice in &measure.voices {
                if voice.len() > MAX_BEATS_PER_VOICE {
                    return Err(RenderError::resource_limit(format!(
                        "measure {} exceeds the {MAX_BEATS_PER_VOICE}-beat resource limit",
                        measure_index + 1
                    )));
                }
                for beat in voice {
                    validate_beat(beat)?;
                }
            }
        }
        Ok(())
    }

    /// Validates this track for the supplied rendering options without
    /// producing a layout.
    ///
    /// Unlike [`Track::validate`], this also checks notation-mode-dependent
    /// requirements such as string and pitch availability, spans, chord
    /// diagrams, onsets, and engraving dimensions.
    pub fn validate_for(&self, options: LayoutOptions) -> Result<(), RenderError> {
        crate::engrave::validate_for_layout(self, options)
    }
}

impl LayoutOptions {
    /// Validates geometry limits independently of a score model.
    pub fn validate(&self) -> Result<(), RenderError> {
        if !self.width.is_finite()
            || self.width < 160.0
            || !self.string_spacing.is_finite()
            || self.string_spacing < 18.0
            || !self.beat_spacing.is_finite()
            || self.beat_spacing < 32.0
            || self.bars_per_system == Some(0)
        {
            return Err(RenderError::invalid_input("invalid layout options".into()));
        }
        Ok(())
    }
}

impl ScoreDocument {
    /// Validates score-level structure and every contained staff track.
    pub fn validate(&self) -> Result<(), RenderError> {
        let staff_count = self
            .instruments
            .iter()
            .map(|instrument| instrument.staves.len())
            .sum::<usize>();
        if staff_count > MAX_SCORE_STAVES {
            return Err(RenderError::resource_limit(format!(
                "a score exceeds the {MAX_SCORE_STAVES}-staff resource limit"
            )));
        }
        for instrument in &self.instruments {
            for staff in &instrument.staves {
                staff.track.validate()?;
                staff.options.validate()?;
            }
        }
        Ok(())
    }
}
