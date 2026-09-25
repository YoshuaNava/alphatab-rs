//! Model and layout validation used before engraving.

use crate::{Beat, RenderError, SceneOptions, Track};

const MAX_MEASURES: usize = 100_000;
const MAX_VOICES_PER_MEASURE: usize = 32;
const MAX_BEATS_PER_VOICE: usize = 100_000;
const MAX_NOTES_PER_BEAT: usize = 64;
const MAX_CURVE_POINTS: usize = 4_096;
const MAX_METER_DENOMINATOR: u16 = 128;
const MIN_SCENE_WIDTH: f32 = 160.0;
const MIN_STRING_SPACING: f32 = 18.0;
const MIN_BEAT_SPACING: f32 = 32.0;

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
    pub(crate) fn validate(&self) -> Result<(), RenderError> {
        if self.measures.len() > MAX_MEASURES {
            return Err(RenderError::resource_limit(format!(
                "a track exceeds the {MAX_MEASURES}-measure resource limit"
            )));
        }
        for (measure_index, measure) in self.measures.iter().enumerate() {
            let (numerator, denominator) = measure.time_signature;
            if numerator == 0
                || !denominator.is_power_of_two()
                || denominator > MAX_METER_DENOMINATOR
            {
                return Err(RenderError::resource_limit(format!(
                    "invalid meter in measure {}",
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
}

impl SceneOptions {
    pub(crate) fn validate(&self) -> Result<(), RenderError> {
        if !self.width.is_finite()
            || self.width < MIN_SCENE_WIDTH
            || !self.string_spacing.is_finite()
            || self.string_spacing < MIN_STRING_SPACING
            || !self.beat_spacing.is_finite()
            || self.beat_spacing < MIN_BEAT_SPACING
            || self.bars_per_system == Some(0)
        {
            return Err(RenderError::invalid_input("invalid scene options".into()));
        }
        Ok(())
    }
}
