//! Engraving submodule split from the main layout coordinator.
use super::*;
use std::collections::HashMap;

pub(crate) fn pitch_y(p: Pitch, clef: Clef, y: f32) -> f32 {
    let bottom = match clef {
        Clef::Treble
        | Clef::Treble8Above
        | Clef::Treble8Below
        | Clef::Treble15Above
        | Clef::Treble15Below
        | Clef::Percussion => 30,
        Clef::Bass
        | Clef::Bass8Above
        | Clef::Bass8Below
        | Clef::Bass15Above
        | Clef::Bass15Below => 18,
        Clef::Alto
        | Clef::Alto8Above
        | Clef::Alto8Below
        | Clef::Alto15Above
        | Clef::Alto15Below => 24,
        Clef::Tenor
        | Clef::Tenor8Above
        | Clef::Tenor8Below
        | Clef::Tenor15Above
        | Clef::Tenor15Below => 22,
    };
    y + 40.0 - (i16::from(p.octave) * 7 + i16::from(p.step) - bottom) as f32 * 5.0
}
pub(super) fn key_accidental(key: KeySignature, step: u8) -> i8 {
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
    page: &mut Layout,
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
        10.0,
        1.0,
    );
    for i in 0..5 {
        page.line(
            x,
            y + i as f32 * 10.0,
            x + width,
            y + i as f32 * 10.0,
            staff_width,
        );
    }
    if signature.0 {
        let (code, cy) = match clef {
            Clef::Treble => (G::GClef, y + 30.0),
            Clef::Treble8Above => (G::GClef8Va, y + 30.0),
            Clef::Treble8Below => (G::GClef8Vb, y + 30.0),
            Clef::Treble15Above => (G::GClef15Ma, y + 30.0),
            Clef::Treble15Below => (G::GClef15Mb, y + 30.0),
            Clef::Bass8Above => (G::FClef8Va, y + 10.0),
            Clef::Bass8Below => (G::FClef8Vb, y + 10.0),
            Clef::Bass15Above => (G::FClef15Ma, y + 10.0),
            Clef::Bass15Below => (G::FClef15Mb, y + 10.0),
            Clef::Bass => (G::FClef, y + 10.0),
            Clef::Alto | Clef::Alto8Above | Clef::Alto15Above | Clef::Alto15Below => {
                (G::CClef, y + 20.0)
            }
            Clef::Alto8Below => (G::CClef8Vb, y + 20.0),
            Clef::Tenor | Clef::Tenor8Above | Clef::Tenor15Above | Clef::Tenor15Below => {
                (G::CClef, y + 10.0)
            }
            Clef::Tenor8Below => (G::CClef8Vb, y + 10.0),
            Clef::Percussion => (G::UnpitchedPercussionClef1, y + 20.0),
        };
        page.glyph_at_origin(x + 8.0, cy, code, 10.0)?;
        match clef {
            Clef::Alto8Above | Clef::Tenor8Above => {
                page.glyph_at_origin(x + 15.0, cy - 25.0, G::Clef8, 8.0)?
            }
            Clef::Alto15Above | Clef::Tenor15Above => {
                page.glyph_at_origin(x + 12.0, cy - 25.0, G::Clef15, 8.0)?
            }
            Clef::Alto15Below | Clef::Tenor15Below => {
                page.glyph_at_origin(x + 12.0, cy + 30.0, G::Clef15, 8.0)?
            }
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
                    octave: 4,
                    accidental: 0,
                };
                let upper = if fifths >= 0 { -5.0 } else { 0.0 };
                let lower = if fifths >= 0 { 30.0 } else { 35.0 };
                while pitch_y(pitch, clef, 0.0) > lower {
                    pitch.octave += 1;
                }
                while pitch_y(pitch, clef, 0.0) < upper {
                    pitch.octave -= 1;
                }
                page.glyph_at_origin(
                    x + 34.0 + index as f32 * 7.0,
                    pitch_y(pitch, clef, y),
                    if cancel {
                        G::AccidentalNatural
                    } else if fifths > 0 {
                        G::AccidentalSharp
                    } else {
                        G::AccidentalFlat
                    },
                    7.0,
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
pub(super) fn staff_beat(
    page: &mut Layout,
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
            + 20.0
            + if multiple_voices {
                if voice % 2 == 1 {
                    35.0
                } else {
                    -35.0
                }
            } else {
                0.0
            };
        rest(page, beat.duration.value, x, rest_y)?;
        for dot in 0..beat.duration.dots {
            page.glyph_at_origin(
                x + 12.0 + f32::from(dot) * 5.0,
                rest_y - 3.0,
                G::AugmentationDot,
                7.0,
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
        let ny = pitch_y(p, clef, y);
        if let Some(grace) = note.effects.grace_pitch {
            let gy = pitch_y(grace, clef, y);
            let grace_code = if note.effects.grace_dead {
                G::NoteheadXBlack
            } else {
                G::NoteheadBlack
            };
            let grace_origin = page.glyph_at_center(x - 20.0, gy, grace_code, 5.0)?;
            let grace_anchor = crate::glyph::load(grace_code)?
                .metrics
                .stem_up
                .unwrap_or([1.25, 0.0]);
            let grace_stem_x = grace_origin[0] + grace_anchor[0] * 5.0;
            page.line(grace_stem_x, gy, grace_stem_x, gy - 20.0, 0.9);
            let grace_value = note.effects.grace_duration.max(8);
            page.glyph_at_origin(
                grace_stem_x,
                gy - 20.0,
                flag_glyph(grace_value.ilog2().saturating_sub(2), false),
                5.0,
            )?;
            if !note.effects.grace_on_beat {
                page.line(x - 24.0, gy - 10.0, x - 14.0, gy - 17.0, 0.9);
            }
            if note.effects.grace_slide {
                page.line(x - 18.0, gy + 3.0, x - 7.0, ny + 3.0, 1.0);
            } else if note.effects.grace_bend {
                page.curve([x - 22.0, gy - 5.0], [x - 7.0, ny - 5.0], -6.0);
            } else if note.effects.grace_slur {
                page.curve([x - 22.0, gy + 5.0], [x - 7.0, ny + 5.0], 4.0);
            }
        }
        low = low.max(ny);
        high = high.min(ny);
        displaced = (last_y - ny).abs() < 6.0 && !displaced;
        let nx = x + if displaced { 10.0 } else { 0.0 };
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
                .position(|last| (ny - *last).abs() >= 28.0)
                .unwrap_or(accidental_columns.len());
            if col == accidental_columns.len() {
                accidental_columns.push(ny);
            } else {
                accidental_columns[col] = ny;
            }
            page.glyph_at_right_center(x - 9.0 - col as f32 * 16.0, ny, code, 9.0)?;
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
            page.glyph_at_right_center(nx - half_width - 1.0, ny, G::NoteheadParenthesisLeft, 8.0)?;
            page.glyph_at_left_center(nx + half_width + 1.0, ny, G::NoteheadParenthesisRight, 8.0)?;
        }
        let note_origin = page.glyph_at_center(nx, ny, code, 8.0)?;
        let stem_glyph = (note_origin[0], note_origin[1], code);
        if low_stem.is_none_or(|(_, y, _)| ny > y) {
            low_stem = Some(stem_glyph);
        }
        if high_stem.is_none_or(|(_, y, _)| ny < y) {
            high_stem = Some(stem_glyph);
        }
        if let Some(touch) = note.effects.harmonic_pitch {
            let hy = pitch_y(touch, clef, y);
            page.glyph_at_center(
                nx,
                hy,
                NoteHead::Diamond.resolve_glyph(beat.duration.value),
                7.0,
            )?;
            low = low.max(hy);
            high = high.min(hy);
            ledger_lines(page, nx, hy, y);
            if touch.accidental != 0 {
                page.glyph_at_right_center(
                    nx - 8.0,
                    hy,
                    if touch.accidental > 0 {
                        G::AccidentalSharp
                    } else {
                        G::AccidentalFlat
                    },
                    7.0,
                )?;
            }
        }
        ledger_lines(page, nx, ny, y);
        if let (Some(initial), Some(target)) = (note.effects.bend.first(), note.effects.bend.last())
        {
            let target = if (target[1] - initial[1]).abs() < 0.01 {
                note.effects
                    .bend
                    .iter()
                    .max_by(|a, b| a[1].total_cmp(&b[1]))
                    .unwrap()
            } else {
                target
            };
            let shift = target[1] - initial[1];
            if shift.abs() > 0.01 {
                let midi = i16::from(p.to_midi()?) + shift.round() as i16;
                let midi = u8::try_from(midi)
                    .ok()
                    .filter(|n| *n <= 127)
                    .ok_or_else(|| {
                        RenderError::invalid_input("bend target outside MIDI range".into())
                    })?;
                let pitch = Pitch::from_midi(midi, p.accidental < 0);
                let by = pitch_y(pitch, clef, y);
                let bx = nx + column_width * 0.32;
                let bend_origin = page.glyph_at_center(bx, by, G::NoteheadBlack, 5.0)?;
                let bend_anchor = crate::glyph::load(G::NoteheadBlack)?
                    .metrics
                    .stem_up
                    .unwrap_or([1.25, 0.0]);
                let bend_stem_x = bend_origin[0] + bend_anchor[0] * 5.0;
                page.line(bend_stem_x, by, bend_stem_x, by - 20.0, 0.9);
                ledger_lines(page, bx, by, y);
                if pitch.accidental != 0 {
                    page.glyph_at_right_center(
                        bx - 6.0,
                        by,
                        if pitch.accidental > 0 {
                            G::AccidentalSharp
                        } else {
                            G::AccidentalFlat
                        },
                        5.0,
                    )?;
                }
                page.curve([nx + 6.0, ny - 6.0], [bx - 5.0, by - 6.0], -5.0);
            }
        }
        for dot in 0..beat.duration.dots {
            page.glyph_at_origin(
                nx + 12.0 + dot as f32 * 5.0,
                ny - 3.0,
                G::AugmentationDot,
                7.0,
            )?;
        }
    }
    if (beat.duration.value > 1 || beat.duration.value == -4) && !suppress_stem {
        let down = stem_down(beat, voice);
        let (origin_x, origin_y, code) = if down { high_stem } else { low_stem }
            .expect("a non-empty beat has a stem-bearing notehead");
        let anchor = if down {
            crate::glyph::load(code)?.metrics.stem_down
        } else {
            crate::glyph::load(code)?.metrics.stem_up
        }
        .unwrap_or([if down { 0.0 } else { 1.25 }, 0.0]);
        let sx = origin_x + anchor[0] * 8.0;
        let from = origin_y + anchor[1] * 8.0;
        let end = stem_end.unwrap_or(if down { low + 30.0 } else { high - 30.0 });
        let stem_width = crate::music_font::thickness(
            crate::music_font::metadata()
                .engraving_defaults
                .stem_thickness,
            10.0,
            1.1,
        );
        page.line(sx, from, sx, end, stem_width);
        let levels = beat.duration.beam_level_count();
        if levels > 0 && stem_end.is_none() {
            page.glyph_at_origin(sx, end, flag_glyph(levels, down), 8.0)?;
        }
    }
    Ok(())
}

pub(super) fn ledger_lines(page: &mut Layout, x: f32, note_y: f32, staff_y: f32) {
    let width = crate::music_font::thickness(
        crate::music_font::metadata()
            .engraving_defaults
            .leger_line_thickness,
        10.0,
        1.0,
    );
    let mut yy = staff_y - 10.0;
    while yy >= note_y {
        page.line(x - 9.0, yy, x + 10.0, yy, width);
        yy -= 10.0;
    }
    yy = staff_y + 50.0;
    while yy <= note_y {
        page.line(x - 9.0, yy, x + 10.0, yy, width);
        yy += 10.0;
    }
}
pub(super) fn accidental_marks(m: &Measure) -> std::collections::HashSet<(usize, usize, usize)> {
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
                .unwrap_or(Some(key_accidental(m.key_signature, p.step)));
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
