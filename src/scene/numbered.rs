//! Scene construction for numbered notation.
use crate::{Beat, KeySignature, Measure, RenderError};
use smufl::Glyph as G;

use super::parameters::{
    DIATONIC_STEPS_PER_OCTAVE, DOT_SPACING, EMPHASIZED_STROKE_WIDTH, EMPHASIZED_TEXT_SIZE,
    LARGE_TEXT_SIZE, NOTE_GLYPH_SIZE, SMALL_GLYPH_SIZE, STAFF_HEIGHT, STAFF_MIDDLE_LINE_OFFSET,
    STANDARD_TEXT_SIZE, THIN_STROKE_WIDTH, TIMELINE_EPSILON,
};
use super::staff::compute_key_accidental;
use super::{Primitive, Scene};

const TONIC_FIFTHS_TO_STEPS: i16 = 4;
const NUMBERED_REFERENCE_PITCH: i16 = 28;
const SCALE_DEGREE_START: i16 = 1;
const SUSTAIN_START_FRACTION: f32 = 0.35;
const SUSTAIN_DASH_FRACTION: f32 = 0.7;

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
        EMPHASIZED_TEXT_SIZE * voice as f32
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
    let tonic = (i16::from(key.signed_value()) * TONIC_FIFTHS_TO_STEPS)
        .rem_euclid(DIATONIC_STEPS_PER_OCTAVE);
    let mut pitches: Vec<_> = beat.notes.iter().filter_map(|n| n.pitch).collect();
    pitches.sort_by_key(|p| i16::from(p.octave) * DIATONIC_STEPS_PER_OCTAVE + i16::from(p.step));
    if pitches.is_empty() {
        page.text(x, y + STAFF_MIDDLE_LINE_OFFSET, "0", LARGE_TEXT_SIZE, false);
    }
    for (i, p) in pitches.iter().enumerate() {
        let yy = y + STAFF_MIDDLE_LINE_OFFSET - i as f32 * STAFF_MIDDLE_LINE_OFFSET;
        let relative = i16::from(p.octave) * DIATONIC_STEPS_PER_OCTAVE + i16::from(p.step)
            - NUMBERED_REFERENCE_PITCH
            - tonic;
        page.text(
            x,
            yy,
            relative.rem_euclid(DIATONIC_STEPS_PER_OCTAVE) + SCALE_DEGREE_START,
            LARGE_TEXT_SIZE,
            false,
        );
        let delta = p.accidental - compute_key_accidental(key, p.step);
        if delta != 0 {
            page.glyph_at_center(
                x - LARGE_TEXT_SIZE,
                yy + DOT_SPACING,
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
                x - THIN_STROKE_WIDTH,
                yy + if octave > 0 {
                    -EMPHASIZED_TEXT_SIZE - f32::from(dot) * DOT_SPACING
                } else {
                    LARGE_TEXT_SIZE + f32::from(dot) * DOT_SPACING
                },
                G::AugmentationDot,
                DOT_SPACING,
            )?;
        }
    }
    if !explicit_beam {
        for level in 0..beat.duration.beam_level_count() {
            page.line(
                x - NOTE_GLYPH_SIZE,
                y + STAFF_HEIGHT - DOT_SPACING + level as f32 * DOT_SPACING,
                x + NOTE_GLYPH_SIZE,
                y + STAFF_HEIGHT - DOT_SPACING + level as f32 * DOT_SPACING,
                EMPHASIZED_STROKE_WIDTH,
            );
        }
    }
    if beat.duration.undotted_quarter_beats() >= 2.0 {
        let quarters = (beat.duration.undotted_quarter_beats()
            * beat.duration.augmentation_dot_factor())
        .floor() as usize;
        let start = x - width * SUSTAIN_START_FRACTION;
        // Move the number to the first quarter, then fill the remaining quarters with dashes.
        for primitive in page.primitives.iter_mut().rev() {
            match primitive {
                Primitive::Text { at, .. } | Primitive::Glyph { at, .. }
                    if (at[0] - x).abs() < STAFF_MIDDLE_LINE_OFFSET
                        && at[1] >= y - STAFF_HEIGHT * 4.0
                        && at[1] <= y + STAFF_HEIGHT - DOT_SPACING =>
                {
                    at[0] += start - x
                }
                _ => break,
            }
        }
        for i in 1..quarters {
            page.text(
                start + width * SUSTAIN_DASH_FRACTION * i as f32 / quarters as f32,
                y + STAFF_MIDDLE_LINE_OFFSET,
                "–",
                LARGE_TEXT_SIZE,
                false,
            );
        }
    } else {
        for dot in 0..beat.duration.dots {
            page.glyph_at_center(
                x + STANDARD_TEXT_SIZE + f32::from(dot) * DOT_SPACING,
                y + STAFF_MIDDLE_LINE_OFFSET,
                G::AugmentationDot,
                SMALL_GLYPH_SIZE,
            )?;
        }
    }
    Ok(())
}
