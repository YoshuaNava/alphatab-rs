//! Engraving submodule split from the main layout coordinator.
use super::*;

pub(crate) fn voice_offset(measure: &Measure, voice: usize, onset: f64) -> f32 {
    if voice == 0 {
        return 0.0;
    }
    let at = |v: &[Beat]| {
        let mut time = 0.0;
        v.iter()
            .find(|b| {
                time = b.start.unwrap_or(time);
                let found = (time - onset).abs() < 1e-8;
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
                    let distance = (i16::from(p.octave) * 7 + i16::from(p.step)
                        - i16::from(q.octave) * 7
                        - i16::from(q.step))
                    .abs();
                    distance == 1
                        || (distance == 0 && (d != duration || p.accidental != q.accidental))
                })
            })
        });
    if collision {
        13.0 * voice as f32
    } else {
        0.0
    }
}

pub(super) fn numbered_beat(
    page: &mut Layout,
    beat: &Beat,
    key: i8,
    x: f32,
    y: f32,
    width: f32,
    explicit_beam: bool,
) -> Result<(), RenderError> {
    let tonic = (i16::from(key) * 4).rem_euclid(7);
    let mut pitches: Vec<_> = beat.notes.iter().filter_map(|n| n.pitch).collect();
    pitches.sort_by_key(|p| i16::from(p.octave) * 7 + i16::from(p.step));
    if pitches.is_empty() {
        page.text(x, y + 20.0, "0", 17.0, false);
    }
    for (i, p) in pitches.iter().enumerate() {
        let yy = y + 20.0 - i as f32 * 24.0;
        let relative = i16::from(p.octave) * 7 + i16::from(p.step) - 28 - tonic;
        page.text(x, yy, relative.rem_euclid(7) + 1, 17.0, false);
        let delta = p.accidental - key_accidental(key, p.step);
        if delta != 0 {
            page.glyph_at_center(
                x - 17.0,
                yy + 4.0,
                if delta > 0 {
                    G::AccidentalSharp
                } else {
                    G::AccidentalFlat
                },
                7.0,
            )?;
        }
        let octave = relative.div_euclid(7);
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
                x - 8.0,
                y + 34.0 + level as f32 * 4.0,
                x + 8.0,
                y + 34.0 + level as f32 * 4.0,
                1.2,
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
                y + 20.0,
                "–",
                17.0,
                false,
            );
        }
    } else {
        for dot in 0..beat.duration.dots {
            page.glyph_at_center(
                x + 10.0 + f32::from(dot) * 5.0,
                y + 20.0,
                G::AugmentationDot,
                7.0,
            )?;
        }
    }
    Ok(())
}
