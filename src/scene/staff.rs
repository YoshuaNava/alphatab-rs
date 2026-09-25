//! Staff-notation scene construction helpers.
use crate::{Beat, Clef, Fret, KeySignature, Measure, NoteHead, Pitch, RenderError};
use smufl::Glyph as G;

use super::parameters::{DIATONIC_STEPS_PER_OCTAVE, STAFF_HEIGHT, STAFF_LINE_SPACING};
use super::rhythm::{draw_rest, stem_points_down};
use super::{select_flag_glyph, Scene};
use std::collections::HashMap;

const STAFF_STEP_HEIGHT: f32 = STAFF_LINE_SPACING / 2.0;
const TREBLE_BOTTOM_DIATONIC_POSITION: i16 = 30;
const BASS_BOTTOM_DIATONIC_POSITION: i16 = 18;
const ALTO_BOTTOM_DIATONIC_POSITION: i16 = 24;
const TENOR_BOTTOM_DIATONIC_POSITION: i16 = 22;
const STAFF_LINE_COUNT: usize = 5;
const STAFF_REFERENCE_SIZE: f32 = 10.0;
const STAFF_LINE_FALLBACK_WIDTH: f32 = 1.0;
const CLEF_ORIGIN_OFFSET_X: f32 = 8.0;
const CLEF_GLYPH_SIZE: f32 = 10.0;
const TREBLE_CLEF_OFFSET_Y: f32 = 30.0;
const BASS_CLEF_OFFSET_Y: f32 = 10.0;
const C_CLEF_OFFSET_Y: f32 = 20.0;
const OCTAVE_CLEF_OFFSET_X: f32 = 15.0;
const FIFTEENTH_CLEF_OFFSET_X: f32 = 12.0;
const OCTAVE_CLEF_ABOVE_OFFSET_Y: f32 = 25.0;
const FIFTEENTH_CLEF_BELOW_OFFSET_Y: f32 = 30.0;
const OCTAVE_CLEF_GLYPH_SIZE: f32 = 8.0;
const KEY_SIGNATURE_START_OFFSET_X: f32 = 34.0;
const KEY_SIGNATURE_GLYPH_SPACING: f32 = 7.0;
const KEY_SIGNATURE_GLYPH_SIZE: f32 = 7.0;
const KEY_SIGNATURE_REFERENCE_OCTAVE: i8 = 4;
const SHARP_KEY_UPPER_BOUND: f32 = -5.0;
const FLAT_KEY_UPPER_BOUND: f32 = 0.0;
const SHARP_KEY_LOWER_BOUND: f32 = 30.0;
const FLAT_KEY_LOWER_BOUND: f32 = 35.0;
const REST_BASELINE_OFFSET: f32 = 20.0;
const MULTI_VOICE_REST_OFFSET: f32 = 35.0;
const REST_DOT_OFFSET_X: f32 = 12.0;
const REST_DOT_OFFSET_Y: f32 = 3.0;
const REST_DOT_GLYPH_SIZE: f32 = 7.0;
const NOTE_COLLISION_DISTANCE: f32 = 6.0;
const NOTE_COLLISION_OFFSET_X: f32 = 10.0;
const ACCIDENTAL_COLLISION_DISTANCE: f32 = 28.0;
const ACCIDENTAL_OFFSET_X: f32 = 9.0;
const ACCIDENTAL_COLUMN_SPACING: f32 = 16.0;
const ACCIDENTAL_GLYPH_SIZE: f32 = 9.0;
const NOTE_GLYPH_SIZE: f32 = 8.0;
const GHOST_PARENTHESIS_PADDING: f32 = 1.0;
const GHOST_PARENTHESIS_GLYPH_SIZE: f32 = 8.0;
const HARMONIC_GLYPH_SIZE: f32 = 7.0;
const HARMONIC_ACCIDENTAL_OFFSET_X: f32 = 8.0;
const BEND_MATCH_EPSILON: f32 = 0.01;
const MIDI_MAXIMUM_VALUE: u8 = 127;
const BEND_TARGET_X_FRACTION: f32 = 0.32;
const STEM_ANCHOR_FALLBACK_X: f32 = 1.25;
const GRACE_GLYPH_SIZE: f32 = 5.0;
const STEM_LENGTH: f32 = 20.0;
const STEM_FALLBACK_WIDTH: f32 = 1.1;
const GRACE_STEM_STROKE_WIDTH: f32 = 0.9;
const GRACE_SLUR_START_OFFSET_X: f32 = 24.0;
const GRACE_SLUR_END_OFFSET_X: f32 = 14.0;
const GRACE_SLUR_START_OFFSET_Y: f32 = 10.0;
const GRACE_SLUR_END_OFFSET_Y: f32 = 17.0;
const GRACE_SLIDE_START_OFFSET_X: f32 = 18.0;
const GRACE_SLIDE_END_OFFSET_X: f32 = 7.0;
const GRACE_SLIDE_OFFSET_Y: f32 = 3.0;
const GRACE_BEND_START_OFFSET_X: f32 = 22.0;
const GRACE_BEND_END_OFFSET_X: f32 = 7.0;
const GRACE_BEND_OFFSET_Y: f32 = 5.0;
const GRACE_BEND_ARCH_HEIGHT: f32 = -6.0;
const GRACE_SLUR_ARCH_HEIGHT: f32 = 4.0;
const BEND_SLUR_NOTE_OFFSET_X: f32 = 6.0;
const BEND_SLUR_TARGET_OFFSET_X: f32 = 5.0;
const BEND_SLUR_OFFSET_Y: f32 = 6.0;
const BEND_SLUR_ARCH_HEIGHT: f32 = -5.0;
const LEDGER_LINE_LEFT_EXTENSION: f32 = 9.0;
const LEDGER_LINE_RIGHT_EXTENSION: f32 = 10.0;
const LEDGER_LINE_BOTTOM_OFFSET: f32 = 50.0;

pub(crate) fn compute_pitch_y(p: Pitch, clef: Clef, y: f32) -> f32 {
    let bottom = match clef {
        Clef::Treble
        | Clef::Treble8Above
        | Clef::Treble8Below
        | Clef::Treble15Above
        | Clef::Treble15Below
        | Clef::Percussion => TREBLE_BOTTOM_DIATONIC_POSITION,
        Clef::Bass
        | Clef::Bass8Above
        | Clef::Bass8Below
        | Clef::Bass15Above
        | Clef::Bass15Below => BASS_BOTTOM_DIATONIC_POSITION,
        Clef::Alto
        | Clef::Alto8Above
        | Clef::Alto8Below
        | Clef::Alto15Above
        | Clef::Alto15Below => ALTO_BOTTOM_DIATONIC_POSITION,
        Clef::Tenor
        | Clef::Tenor8Above
        | Clef::Tenor8Below
        | Clef::Tenor15Above
        | Clef::Tenor15Below => TENOR_BOTTOM_DIATONIC_POSITION,
    };
    y + STAFF_HEIGHT
        - (i16::from(p.octave) * DIATONIC_STEPS_PER_OCTAVE + i16::from(p.step) - bottom) as f32
            * STAFF_STEP_HEIGHT
}
pub(super) fn compute_key_accidental(key: KeySignature, step: u8) -> i8 {
    let fifths = key.signed_value();
    let order = if fifths >= 0 {
        [3, 0, 4, 1, 5, 2, 6]
    } else {
        [6, 2, 5, 1, 4, 0, 3]
    };
    if order[..key.accidental_count() as usize].contains(&step) {
        fifths.signum()
    } else {
        0
    }
}
pub(super) fn draw_staff(
    page: &mut Scene,
    clef: Clef,
    m: &Measure,
    x: f32,
    y: f32,
    width: f32,
    signature: (bool, KeySignature),
) -> Result<(), RenderError> {
    let staff_width = crate::music_font::thickness(
        crate::music_font::metadata()
            .engraving_defaults
            .staff_line_thickness,
        STAFF_REFERENCE_SIZE,
        STAFF_LINE_FALLBACK_WIDTH,
    );
    for i in 0..STAFF_LINE_COUNT {
        page.line(
            x,
            y + i as f32 * STAFF_LINE_SPACING,
            x + width,
            y + i as f32 * STAFF_LINE_SPACING,
            staff_width,
        );
    }
    if signature.0 {
        let (code, cy) = match clef {
            Clef::Treble => (G::GClef, y + TREBLE_CLEF_OFFSET_Y),
            Clef::Treble8Above => (G::GClef8Va, y + TREBLE_CLEF_OFFSET_Y),
            Clef::Treble8Below => (G::GClef8Vb, y + TREBLE_CLEF_OFFSET_Y),
            Clef::Treble15Above => (G::GClef15Ma, y + TREBLE_CLEF_OFFSET_Y),
            Clef::Treble15Below => (G::GClef15Mb, y + TREBLE_CLEF_OFFSET_Y),
            Clef::Bass8Above => (G::FClef8Va, y + BASS_CLEF_OFFSET_Y),
            Clef::Bass8Below => (G::FClef8Vb, y + BASS_CLEF_OFFSET_Y),
            Clef::Bass15Above => (G::FClef15Ma, y + BASS_CLEF_OFFSET_Y),
            Clef::Bass15Below => (G::FClef15Mb, y + BASS_CLEF_OFFSET_Y),
            Clef::Bass => (G::FClef, y + BASS_CLEF_OFFSET_Y),
            Clef::Alto | Clef::Alto8Above | Clef::Alto15Above | Clef::Alto15Below => {
                (G::CClef, y + C_CLEF_OFFSET_Y)
            }
            Clef::Alto8Below => (G::CClef8Vb, y + C_CLEF_OFFSET_Y),
            Clef::Tenor | Clef::Tenor8Above | Clef::Tenor15Above | Clef::Tenor15Below => {
                (G::CClef, y + BASS_CLEF_OFFSET_Y)
            }
            Clef::Tenor8Below => (G::CClef8Vb, y + BASS_CLEF_OFFSET_Y),
            Clef::Percussion => (G::UnpitchedPercussionClef1, y + C_CLEF_OFFSET_Y),
        };
        page.glyph_at_origin(x + CLEF_ORIGIN_OFFSET_X, cy, code, CLEF_GLYPH_SIZE)?;
        match clef {
            Clef::Alto8Above | Clef::Tenor8Above => page.glyph_at_origin(
                x + OCTAVE_CLEF_OFFSET_X,
                cy - OCTAVE_CLEF_ABOVE_OFFSET_Y,
                G::Clef8,
                OCTAVE_CLEF_GLYPH_SIZE,
            )?,
            Clef::Alto15Above | Clef::Tenor15Above => page.glyph_at_origin(
                x + FIFTEENTH_CLEF_OFFSET_X,
                cy - OCTAVE_CLEF_ABOVE_OFFSET_Y,
                G::Clef15,
                OCTAVE_CLEF_GLYPH_SIZE,
            )?,
            Clef::Alto15Below | Clef::Tenor15Below => page.glyph_at_origin(
                x + FIFTEENTH_CLEF_OFFSET_X,
                cy + FIFTEENTH_CLEF_BELOW_OFFSET_Y,
                G::Clef15,
                OCTAVE_CLEF_GLYPH_SIZE,
            )?,
            _ => {}
        }
        let mut index = 0;
        for (key, cancel) in [(signature.1, true), (m.key_signature, false)] {
            if clef == Clef::Percussion {
                break;
            }
            let fifths = key.signed_value();
            let steps = if fifths >= 0 {
                [3, 0, 4, 1, 5, 2, 6]
            } else {
                [6, 2, 5, 1, 4, 0, 3]
            };
            for step in steps.iter().take(key.accidental_count() as usize) {
                let mut pitch = Pitch {
                    step: *step,
                    octave: KEY_SIGNATURE_REFERENCE_OCTAVE,
                    accidental: 0,
                };
                let upper = if fifths >= 0 {
                    SHARP_KEY_UPPER_BOUND
                } else {
                    FLAT_KEY_UPPER_BOUND
                };
                let lower = if fifths >= 0 {
                    SHARP_KEY_LOWER_BOUND
                } else {
                    FLAT_KEY_LOWER_BOUND
                };
                while compute_pitch_y(pitch, clef, 0.0) > lower {
                    pitch.octave += 1;
                }
                while compute_pitch_y(pitch, clef, 0.0) < upper {
                    pitch.octave -= 1;
                }
                page.glyph_at_origin(
                    x + KEY_SIGNATURE_START_OFFSET_X + index as f32 * KEY_SIGNATURE_GLYPH_SPACING,
                    compute_pitch_y(pitch, clef, y),
                    if cancel {
                        G::AccidentalNatural
                    } else if fifths > 0 {
                        G::AccidentalSharp
                    } else {
                        G::AccidentalFlat
                    },
                    KEY_SIGNATURE_GLYPH_SIZE,
                )?;
                index += 1;
            }
        }
    }
    Ok(())
}
pub(super) struct StaffStyle {
    pub(super) clef: Clef,
    pub(super) voice: usize,
    pub(super) stem_end: Option<f32>,
    pub(super) suppress_stem: bool,
    pub(super) multiple_voices: bool,
    pub(super) column_width: f32,
    pub(super) slash: bool,
}
pub(super) fn draw_staff_beat(
    page: &mut Scene,
    beat: &Beat,
    style: StaffStyle,
    x: f32,
    y: f32,
    accidentals: &std::collections::HashSet<(usize, usize, usize)>,
    address: (usize, usize),
) -> Result<(), RenderError> {
    let StaffStyle {
        clef,
        voice,
        stem_end,
        suppress_stem,
        multiple_voices,
        column_width,
        slash,
    } = style;
    if beat.notes.is_empty() {
        let rest_y = y
            + REST_BASELINE_OFFSET
            + if multiple_voices {
                if voice % 2 == 1 {
                    MULTI_VOICE_REST_OFFSET
                } else {
                    -MULTI_VOICE_REST_OFFSET
                }
            } else {
                0.0
            };
        draw_rest(page, beat.duration.value, x, rest_y)?;
        for dot in 0..beat.duration.dots {
            page.glyph_at_origin(
                x + REST_DOT_OFFSET_X + f32::from(dot) * super::parameters::DOT_SPACING,
                rest_y - REST_DOT_OFFSET_Y,
                G::AugmentationDot,
                REST_DOT_GLYPH_SIZE,
            )?;
        }
        return Ok(());
    }
    let mut pitches: Vec<_> = beat
        .notes
        .iter()
        .enumerate()
        .map(|(i, n)| (i, n, n.pitch.unwrap()))
        .collect();
    pitches.sort_by_key(|(_, _, p)| i16::from(p.octave) * 7 + i16::from(p.step));
    let mut low = f32::MIN;
    let mut high = f32::MAX;
    let mut low_stem = None;
    let mut high_stem = None;
    let mut last_y = f32::INFINITY;
    let mut displaced = false;
    let mut accidental_columns: Vec<f32> = vec![];
    for (ni, note, p) in pitches {
        let ny = compute_pitch_y(p, clef, y);
        if let Some(grace) = note.effects.grace_pitch {
            let gy = compute_pitch_y(grace, clef, y);
            let grace_code = if note.effects.grace_dead {
                G::NoteheadXBlack
            } else {
                G::NoteheadBlack
            };
            let grace_origin =
                page.glyph_at_center(x - STEM_LENGTH, gy, grace_code, GRACE_GLYPH_SIZE)?;
            let grace_anchor = crate::glyph::load(grace_code)?
                .metrics
                .stem_up
                .unwrap_or([STEM_ANCHOR_FALLBACK_X, 0.0]);
            let grace_stem_x = grace_origin[0] + grace_anchor[0] * GRACE_GLYPH_SIZE;
            page.line(
                grace_stem_x,
                gy,
                grace_stem_x,
                gy - STEM_LENGTH,
                GRACE_STEM_STROKE_WIDTH,
            );
            let grace_value = note.effects.grace_duration.max(8);
            page.glyph_at_origin(
                grace_stem_x,
                gy - STEM_LENGTH,
                select_flag_glyph(grace_value.ilog2().saturating_sub(2), false),
                GRACE_GLYPH_SIZE,
            )?;
            if !note.effects.grace_on_beat {
                page.line(
                    x - GRACE_SLUR_START_OFFSET_X,
                    gy - GRACE_SLUR_START_OFFSET_Y,
                    x - GRACE_SLUR_END_OFFSET_X,
                    gy - GRACE_SLUR_END_OFFSET_Y,
                    GRACE_STEM_STROKE_WIDTH,
                );
            }
            if note.effects.grace_slide {
                page.line(
                    x - GRACE_SLIDE_START_OFFSET_X,
                    gy + GRACE_SLIDE_OFFSET_Y,
                    x - GRACE_SLIDE_END_OFFSET_X,
                    ny + GRACE_SLIDE_OFFSET_Y,
                    STAFF_LINE_FALLBACK_WIDTH,
                );
            } else if note.effects.grace_bend {
                page.curve(
                    [x - GRACE_BEND_START_OFFSET_X, gy - GRACE_BEND_OFFSET_Y],
                    [x - GRACE_BEND_END_OFFSET_X, ny - GRACE_BEND_OFFSET_Y],
                    GRACE_BEND_ARCH_HEIGHT,
                );
            } else if note.effects.grace_slur {
                page.curve(
                    [x - GRACE_BEND_START_OFFSET_X, gy + GRACE_BEND_OFFSET_Y],
                    [x - GRACE_BEND_END_OFFSET_X, ny + GRACE_BEND_OFFSET_Y],
                    GRACE_SLUR_ARCH_HEIGHT,
                );
            }
        }
        low = low.max(ny);
        high = high.min(ny);
        displaced = (last_y - ny).abs() < NOTE_COLLISION_DISTANCE && !displaced;
        let nx = x + if displaced {
            NOTE_COLLISION_OFFSET_X
        } else {
            0.0
        };
        last_y = ny;
        if accidentals.contains(&(address.0, address.1, ni)) || note.effects.quarter_tone != 0 {
            let code = if note.effects.quarter_tone != 0 {
                match p.accidental * 2 + note.effects.quarter_tone {
                    -3 => G::AccidentalThreeQuarterTonesFlatZimmermann,
                    -1 => G::AccidentalQuarterToneFlatStein,
                    1 => G::AccidentalQuarterToneSharpStein,
                    3 => G::AccidentalThreeQuarterTonesSharpStein,
                    _ => {
                        return Err(RenderError::invalid_input(
                            "quarter-tone spelling exceeds supported accidental range".into(),
                        ))
                    }
                }
            } else {
                match p.accidental {
                    -2 => G::AccidentalDoubleFlat,
                    -1 => G::AccidentalFlat,
                    0 => G::AccidentalNatural,
                    1 => G::AccidentalSharp,
                    _ => G::AccidentalDoubleSharp,
                }
            };
            let col = accidental_columns
                .iter()
                .position(|last| (ny - *last).abs() >= ACCIDENTAL_COLLISION_DISTANCE)
                .unwrap_or(accidental_columns.len());
            if col == accidental_columns.len() {
                accidental_columns.push(ny);
            } else {
                accidental_columns[col] = ny;
            }
            page.glyph_at_right_center(
                x - ACCIDENTAL_OFFSET_X - col as f32 * ACCIDENTAL_COLUMN_SPACING,
                ny,
                code,
                ACCIDENTAL_GLYPH_SIZE,
            )?;
        }
        let code = if let Some(id) = note.percussion {
            let (_, heads) = crate::percussion::resolve(id)?;
            heads[match beat.duration.value {
                1 => 2,
                2 => 1,
                _ => 0,
            }]
        } else if matches!(note.fret, Fret::Dead) {
            G::NoteheadXBlack
        } else if slash {
            NoteHead::Slash.resolve_glyph(beat.duration.value)
        } else {
            note.effects.head.resolve_glyph(beat.duration.value)
        };
        if note.effects.ghost {
            let outline = crate::glyph::load(code)?;
            let half_width = (outline.bounds[2] - outline.bounds[0]) * 4.0;
            page.glyph_at_right_center(
                nx - half_width - GHOST_PARENTHESIS_PADDING,
                ny,
                G::NoteheadParenthesisLeft,
                GHOST_PARENTHESIS_GLYPH_SIZE,
            )?;
            page.glyph_at_left_center(
                nx + half_width + GHOST_PARENTHESIS_PADDING,
                ny,
                G::NoteheadParenthesisRight,
                GHOST_PARENTHESIS_GLYPH_SIZE,
            )?;
        }
        let note_origin = page.glyph_at_center(nx, ny, code, NOTE_GLYPH_SIZE)?;
        let stem_glyph = (note_origin[0], note_origin[1], code);
        if low_stem.is_none_or(|(_, y, _)| ny > y) {
            low_stem = Some(stem_glyph);
        }
        if high_stem.is_none_or(|(_, y, _)| ny < y) {
            high_stem = Some(stem_glyph);
        }
        if let Some(touch) = note.effects.harmonic_pitch {
            let hy = compute_pitch_y(touch, clef, y);
            page.glyph_at_center(
                nx,
                hy,
                NoteHead::Diamond.resolve_glyph(beat.duration.value),
                HARMONIC_GLYPH_SIZE,
            )?;
            low = low.max(hy);
            high = high.min(hy);
            draw_ledger_lines(page, nx, hy, y);
            if touch.accidental != 0 {
                page.glyph_at_right_center(
                    nx - HARMONIC_ACCIDENTAL_OFFSET_X,
                    hy,
                    if touch.accidental > 0 {
                        G::AccidentalSharp
                    } else {
                        G::AccidentalFlat
                    },
                    HARMONIC_GLYPH_SIZE,
                )?;
            }
        }
        draw_ledger_lines(page, nx, ny, y);
        if let (Some(initial), Some(target)) = (note.effects.bend.first(), note.effects.bend.last())
        {
            let target = if (target[1] - initial[1]).abs() < BEND_MATCH_EPSILON {
                note.effects
                    .bend
                    .iter()
                    .max_by(|a, b| a[1].total_cmp(&b[1]))
                    .unwrap()
            } else {
                target
            };
            let shift = target[1] - initial[1];
            if shift.abs() > BEND_MATCH_EPSILON {
                let midi = i16::from(p.to_midi()?) + shift.round() as i16;
                let midi = u8::try_from(midi)
                    .ok()
                    .filter(|n| *n <= MIDI_MAXIMUM_VALUE)
                    .ok_or_else(|| {
                        RenderError::invalid_input("bend target outside MIDI range".into())
                    })?;
                let pitch = Pitch::from_midi(midi, p.accidental < 0);
                let by = compute_pitch_y(pitch, clef, y);
                let bx = nx + column_width * BEND_TARGET_X_FRACTION;
                let bend_origin =
                    page.glyph_at_center(bx, by, G::NoteheadBlack, GRACE_GLYPH_SIZE)?;
                let bend_anchor = crate::glyph::load(G::NoteheadBlack)?
                    .metrics
                    .stem_up
                    .unwrap_or([STEM_ANCHOR_FALLBACK_X, 0.0]);
                let bend_stem_x = bend_origin[0] + bend_anchor[0] * GRACE_GLYPH_SIZE;
                page.line(
                    bend_stem_x,
                    by,
                    bend_stem_x,
                    by - STEM_LENGTH,
                    GRACE_STEM_STROKE_WIDTH,
                );
                draw_ledger_lines(page, bx, by, y);
                if pitch.accidental != 0 {
                    page.glyph_at_right_center(
                        bx - NOTE_COLLISION_DISTANCE,
                        by,
                        if pitch.accidental > 0 {
                            G::AccidentalSharp
                        } else {
                            G::AccidentalFlat
                        },
                        GRACE_GLYPH_SIZE,
                    )?;
                }
                page.curve(
                    [nx + BEND_SLUR_NOTE_OFFSET_X, ny - BEND_SLUR_OFFSET_Y],
                    [bx - BEND_SLUR_TARGET_OFFSET_X, by - BEND_SLUR_OFFSET_Y],
                    BEND_SLUR_ARCH_HEIGHT,
                );
            }
        }
        for dot in 0..beat.duration.dots {
            page.glyph_at_origin(
                nx + REST_DOT_OFFSET_X + dot as f32 * super::parameters::DOT_SPACING,
                ny - REST_DOT_OFFSET_Y,
                G::AugmentationDot,
                REST_DOT_GLYPH_SIZE,
            )?;
        }
    }
    if (beat.duration.value > 1 || beat.duration.value == -4) && !suppress_stem {
        let down = stem_points_down(beat, voice);
        let (origin_x, origin_y, code) = if down { high_stem } else { low_stem }
            .expect("a non-empty beat has a stem-bearing notehead");
        let anchor = if down {
            crate::glyph::load(code)?.metrics.stem_down
        } else {
            crate::glyph::load(code)?.metrics.stem_up
        }
        .unwrap_or([if down { 0.0 } else { 1.25 }, 0.0]);
        let sx = origin_x + anchor[0] * NOTE_GLYPH_SIZE;
        let from = origin_y + anchor[1] * NOTE_GLYPH_SIZE;
        let end = stem_end.unwrap_or(if down {
            low + TREBLE_CLEF_OFFSET_Y
        } else {
            high - TREBLE_CLEF_OFFSET_Y
        });
        let stem_width = crate::music_font::thickness(
            crate::music_font::metadata()
                .engraving_defaults
                .stem_thickness,
            STAFF_REFERENCE_SIZE,
            STEM_FALLBACK_WIDTH,
        );
        page.line(sx, from, sx, end, stem_width);
        let levels = beat.duration.beam_level_count();
        if levels > 0 && stem_end.is_none() {
            page.glyph_at_origin(sx, end, select_flag_glyph(levels, down), NOTE_GLYPH_SIZE)?;
        }
    }
    Ok(())
}

pub(super) fn draw_ledger_lines(page: &mut Scene, x: f32, note_y: f32, staff_y: f32) {
    let width = crate::music_font::thickness(
        crate::music_font::metadata()
            .engraving_defaults
            .leger_line_thickness,
        STAFF_REFERENCE_SIZE,
        STAFF_LINE_FALLBACK_WIDTH,
    );
    let mut yy = staff_y - STAFF_LINE_SPACING;
    while yy >= note_y {
        page.line(
            x - LEDGER_LINE_LEFT_EXTENSION,
            yy,
            x + LEDGER_LINE_RIGHT_EXTENSION,
            yy,
            width,
        );
        yy -= STAFF_LINE_SPACING;
    }
    yy = staff_y + LEDGER_LINE_BOTTOM_OFFSET;
    while yy <= note_y {
        page.line(
            x - LEDGER_LINE_LEFT_EXTENSION,
            yy,
            x + LEDGER_LINE_RIGHT_EXTENSION,
            yy,
            width,
        );
        yy += STAFF_LINE_SPACING;
    }
}
pub(super) fn collect_accidental_marks(
    m: &Measure,
) -> std::collections::HashSet<(usize, usize, usize)> {
    let mut events = vec![];
    for (vi, voice) in m.voices.iter().enumerate() {
        let mut time = 0.0;
        for (bi, beat) in voice.iter().enumerate() {
            time = beat.start.unwrap_or(time);
            for (ni, n) in beat.notes.iter().enumerate() {
                if let Some(p) = n.pitch.filter(|_| n.percussion.is_none()) {
                    events.push((
                        time,
                        vi,
                        bi,
                        ni,
                        p,
                        matches!(n.fret, Fret::Tied(_)),
                        n.effects.quarter_tone,
                    ));
                }
            }
            time += beat.compute_quarter_beats().expect("validated duration");
        }
    }
    events.sort_by(|a, b| a.0.total_cmp(&b.0));
    let mut state: HashMap<(u8, i8), Option<i8>> = HashMap::new();
    let mut marks = std::collections::HashSet::new();
    let mut first = 0;
    while first < events.len() {
        let mut end = first + 1;
        while end < events.len() && (events[end].0 - events[first].0).abs() < 1e-8 {
            end += 1;
        }
        let mut changes: HashMap<(u8, i8), Option<i8>> = HashMap::new();
        for &(_, vi, bi, ni, p, tied, quarter_tone) in &events[first..end] {
            if tied {
                continue;
            }
            let key = (p.step, p.octave);
            let previous = state
                .get(&key)
                .copied()
                .unwrap_or(Some(compute_key_accidental(m.key_signature, p.step)));
            let conflict = events[first..end].iter().any(|e| {
                !e.5 && (e.4.step, e.4.octave) == key
                    && (e.4.accidental != p.accidental || e.6 != quarter_tone)
            });
            if previous != Some(p.accidental) || conflict {
                marks.insert((vi, bi, ni));
            }
            changes.insert(
                key,
                if conflict || quarter_tone != 0 {
                    None
                } else {
                    Some(p.accidental)
                },
            );
        }
        state.extend(changes);
        first = end;
    }
    marks
}
