//! Scene construction for numbered notation.
use crate::{Beat, KeySignature, Measure, RenderError};
use smufl::Glyph as G;

use super::parameters::{
    DIATONIC_STEPS_PER_OCTAVE, DOT_SPACING, EMPHASIZED_STROKE_WIDTH, SMALL_GLYPH_SIZE,
    TIMELINE_EPSILON,
};
use super::staff::compute_key_accidental;
use super::{Primitive, Scene};

const VOICE_COLLISION_OFFSET: f32 = 13.0;
const NUMBER_BASELINE_OFFSET: f32 = 20.0;
const NUMBER_STACK_SPACING: f32 = 24.0;
const NUMBER_TEXT_SIZE: f32 = 17.0;
const ACCIDENTAL_OFFSET_X: f32 = 17.0;
const ACCIDENTAL_OFFSET_Y: f32 = 4.0;
const BEAM_HALF_WIDTH: f32 = 8.0;
const BEAM_BASELINE_OFFSET: f32 = 34.0;
const BEAM_LEVEL_SPACING: f32 = 4.0;

pub(crate) fn compute_voice_offset(measure: &Measure, voice: usize, onset: f64) -> f32 {
    if voice == 0 {
        return 0.0;
    }
    let at = |v: &[Beat]| {
        let mut time = 0.0;
        v.iter()
            .find(|b| {
                time = b.start.unwrap_or(time);
                let found = (time - onset).abs() < TIMELINE_EPSILON;
                time += b.compute_quarter_beats().expect("validated duration");
                found
            })
            .map(|b| {
                (
                    b.duration.value,
                    b.notes.iter().filter_map(|n| n.pitch).collect::<Vec<_>>(),
                )
            })
    };
    let Some((duration, pitches)) = at(&measure.voices[voice]) else {
        return 0.0;
    };
    let collision = measure.voices[..voice]
        .iter()
        .filter_map(|v| at(v))
        .any(|(d, others)| {
            pitches.iter().any(|p| {
                others.iter().any(|q| {
                    let distance = (i16::from(p.octave) * DIATONIC_STEPS_PER_OCTAVE
                        + i16::from(p.step)
                        - i16::from(q.octave) * DIATONIC_STEPS_PER_OCTAVE
                        - i16::from(q.step))
                    .abs();
                    distance == 1
                        || (distance == 0 && (d != duration || p.accidental != q.accidental))
                })
            })
        });
    if collision {
        VOICE_COLLISION_OFFSET * voice as f32
    } else {
        0.0
    }
}

pub(super) fn draw_numbered_beat(
    page: &mut Scene,
    beat: &Beat,
    key: KeySignature,
    x: f32,
    y: f32,
    width: f32,
    explicit_beam: bool,
) -> Result<(), RenderError> {
    let tonic = (i16::from(key.signed_value()) * 4).rem_euclid(DIATONIC_STEPS_PER_OCTAVE);
    let mut pitches: Vec<_> = beat.notes.iter().filter_map(|n| n.pitch).collect();
    pitches.sort_by_key(|p| i16::from(p.octave) * DIATONIC_STEPS_PER_OCTAVE + i16::from(p.step));
    if pitches.is_empty() {
        page.text(x, y + NUMBER_BASELINE_OFFSET, "0", NUMBER_TEXT_SIZE, false);
    }
    for (i, p) in pitches.iter().enumerate() {
        let yy = y + NUMBER_BASELINE_OFFSET - i as f32 * NUMBER_STACK_SPACING;
        let relative =
            i16::from(p.octave) * DIATONIC_STEPS_PER_OCTAVE + i16::from(p.step) - 28 - tonic;
        page.text(
            x,
            yy,
            relative.rem_euclid(DIATONIC_STEPS_PER_OCTAVE) + 1,
            NUMBER_TEXT_SIZE,
            false,
        );
        let delta = p.accidental - compute_key_accidental(key, p.step);
        if delta != 0 {
            page.glyph_at_center(
                x - ACCIDENTAL_OFFSET_X,
                yy + ACCIDENTAL_OFFSET_Y,
                if delta > 0 {
                    G::AccidentalSharp
                } else {
                    G::AccidentalFlat
                },
                SMALL_GLYPH_SIZE,
            )?;
        }
        let octave = relative.div_euclid(DIATONIC_STEPS_PER_OCTAVE);
        for dot in 0..octave.unsigned_abs() {
            page.glyph_at_center(
                x - 1.0,
                yy + if octave > 0 {
                    -12.0 - f32::from(dot) * 4.0
                } else {
                    14.0 + f32::from(dot) * 4.0
                },
                G::AugmentationDot,
                5.0,
            )?;
        }
    }
    if !explicit_beam {
        for level in 0..beat.duration.beam_level_count() {
            page.line(
                x - BEAM_HALF_WIDTH,
                y + BEAM_BASELINE_OFFSET + level as f32 * BEAM_LEVEL_SPACING,
                x + BEAM_HALF_WIDTH,
                y + BEAM_BASELINE_OFFSET + level as f32 * BEAM_LEVEL_SPACING,
                EMPHASIZED_STROKE_WIDTH,
            );
        }
    }
    if beat.duration.undotted_quarter_beats() >= 2.0 {
        let quarters = (beat.duration.undotted_quarter_beats()
            * beat.duration.augmentation_dot_factor())
        .floor() as usize;
        let start = x - width * 0.35;
        // Move the number to the first quarter, then fill the remaining quarters with dashes.
        for primitive in page.primitives.iter_mut().rev() {
            match primitive {
                Primitive::Text { at, .. } | Primitive::Glyph { at, .. }
                    if (at[0] - x).abs() < 20.0 && at[1] >= y - 150.0 && at[1] <= y + 34.0 =>
                {
                    at[0] += start - x
                }
                _ => break,
            }
        }
        for i in 1..quarters {
            page.text(
                start + width * 0.7 * i as f32 / quarters as f32,
                y + NUMBER_BASELINE_OFFSET,
                "–",
                NUMBER_TEXT_SIZE,
                false,
            );
        }
    } else {
        for dot in 0..beat.duration.dots {
            page.glyph_at_center(
                x + 10.0 + f32::from(dot) * DOT_SPACING,
                y + NUMBER_BASELINE_OFFSET,
                G::AugmentationDot,
                SMALL_GLYPH_SIZE,
            )?;
        }
    }
    Ok(())
}
