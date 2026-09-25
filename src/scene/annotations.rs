//! Scene construction for annotations and note effects.
use super::*;

// Note-effect geometry is expressed in scene units relative to a notehead.
const ORNAMENT_OFFSET_Y: f32 = 24.0;
const SLIDE_LENGTH: f32 = 15.0;
const SLIDE_ENDPOINT_OFFSET_X: f32 = 10.0;
const SLIDE_SLOPE_Y: f32 = 7.0;
const EFFECT_STROKE_WIDTH: f32 = 1.2;
const EFFECT_TEXT_SIZE: f32 = 10.0;
const BEND_HORIZONTAL_INSET: f32 = 0.35;
const BEND_VERTICAL_SCALE: f32 = 9.0;
const BEND_LABEL_OFFSET_Y: f32 = 9.0;
const BEND_ARROW_HALF_WIDTH: f32 = 3.0;
const BEND_ARROW_TAIL_LENGTH: f32 = 4.0;

// Chord-diagram geometry is deliberately fixed so diagrams remain legible at
// every beat width.
const CHORD_STRING_SPACING: f32 = 10.0;
const CHORD_FRET_SPACING: f32 = 8.0;
const CHORD_GRID_STROKE_WIDTH: f32 = 0.7;
const CHORD_BARRE_STROKE_WIDTH: f32 = 4.0;

pub(super) fn draw_note_effects(
    page: &mut Layout,
    n: &Note,
    position: [f32; 4],
    bend_curve: bool,
) -> Result<(), RenderError> {
    let [x, y, width, lane] = position;
    let e = &n.effects;
    // Place compact note-attached effects around the notehead before adding
    // lane-based labels and curves above the beat.
    if let Some(ornament) = e.ornament {
        page.glyph_at_center(x, y - ORNAMENT_OFFSET_Y, ornament.resolve_glyph(), 8.0)?;
    }
    if let Some(direction) = e.slide_in {
        let delta = if direction == SlideDirection::Up {
            SLIDE_SLOPE_Y
        } else {
            -SLIDE_SLOPE_Y
        };
        page.line(
            x - SLIDE_ENDPOINT_OFFSET_X - SLIDE_LENGTH,
            y + delta,
            x - SLIDE_ENDPOINT_OFFSET_X,
            y,
            EFFECT_STROKE_WIDTH,
        );
    }
    if let Some(direction) = e.slide_out {
        let delta = if direction == SlideDirection::Up {
            -SLIDE_SLOPE_Y
        } else {
            SLIDE_SLOPE_Y
        };
        page.line(
            x + SLIDE_ENDPOINT_OFFSET_X,
            y,
            x + SLIDE_ENDPOINT_OFFSET_X + SLIDE_LENGTH,
            y + delta,
            EFFECT_STROKE_WIDTH,
        );
    }
    if let Some(fret) = e.grace_fret {
        page.text(
            x - 20.0,
            y - 6.0,
            if e.grace_dead {
                "x".into()
            } else {
                fret.to_string()
            },
            10.0,
            true,
        );
        if e.grace_slide {
            page.line(x - 17.0, y + 3.0, x - 7.0, y, 1.0);
        } else if e.grace_bend {
            page.curve([x - 19.0, y - 5.0], [x - 7.0, y - 5.0], -6.0);
        } else if e.grace_slur {
            page.curve([x - 19.0, y + 3.0], [x - 7.0, y + 3.0], 4.0);
        }
    }
    if let Some(fret) = e.trill_fret {
        page.text(x, lane, format!("tr {fret}"), EFFECT_TEXT_SIZE, false);
    }
    for (enabled, code, offset) in [
        (e.staccato, G::ArticStaccatoAbove, -9.0),
        (e.accent, G::ArticAccentAbove, -17.0),
        (e.heavy_accent, G::ArticMarcatoAbove, -27.0),
    ] {
        if enabled {
            page.glyph_at_center(x, y + offset, code, 7.0)?;
        }
    }
    for i in 0..e.tremolo_slashes {
        page.line(
            x - 4.0,
            y + 9.0 + i as f32 * 4.0,
            x + 5.0,
            y + 5.0 + i as f32 * 4.0,
            2.0,
        );
    }
    let mut labels = vec![];
    if let Some(h) = &e.harmonic {
        labels.push(h.clone());
    }
    if let Some(f) = &e.fingering {
        labels.push(f.clone());
    }
    if let Some(f) = &e.right_fingering {
        labels.push(f.clone());
    }
    if !labels.is_empty() {
        page.text(x, lane, labels.join(" "), EFFECT_TEXT_SIZE, false);
    }
    if e.vibrato {
        let mut last = [x - width / 3.0, lane - 6.0];
        for i in 1..=16 {
            let p = [
                x - width / 3.0 + width * 2.0 / 3.0 * i as f32 / 16.0,
                lane - 6.0 + (i as f32 * std::f32::consts::FRAC_PI_2).sin() * 2.0,
            ];
            page.line(last[0], last[1], p[0], p[1], 1.0);
            last = p;
        }
    }
    // Bend points use beat-relative horizontal positions and semitone-derived
    // vertical values; convert them into a drawable curve in the annotation lane.
    if bend_curve && !e.bend.is_empty() {
        let points: Vec<_> = e
            .bend
            .iter()
            .map(|p| {
                [
                    x - width * BEND_HORIZONTAL_INSET
                        + p[0] * width * (1.0 - BEND_HORIZONTAL_INSET * 2.0),
                    lane - p[1] * BEND_VERTICAL_SCALE,
                ]
            })
            .collect();
        if e.bend[0][1] != 0.0 {
            let first = points[0];
            page.line(first[0], lane, first[0], first[1], 1.0);
            draw_bend_arrow(page, first, e.bend[0][1] > 0.0);
            page.text(
                first[0],
                first[1] - BEND_LABEL_OFFSET_Y,
                format_bend_label(e.bend[0][1]),
                9.0,
                false,
            );
        }
        for (i, pair) in points.windows(2).enumerate() {
            page.curve(pair[0], pair[1], 0.0);
            let change = e.bend[i + 1][1] - e.bend[i][1];
            if change.abs() > f32::EPSILON {
                draw_bend_arrow(page, pair[1], change > 0.0);
                page.text(
                    pair[1][0],
                    pair[1][1] - BEND_LABEL_OFFSET_Y,
                    format_bend_label(e.bend[i + 1][1]),
                    BEND_LABEL_OFFSET_Y,
                    false,
                );
            }
        }
        if e.bend_vibrato {
            if let Some(last) = points.last() {
                for i in 0..6 {
                    let xx = last[0] - 18.0 + i as f32 * 3.0;
                    page.line(xx, last[1], xx + 1.5, last[1] - 3.0, 1.0);
                    page.line(xx + 1.5, last[1] - 3.0, xx + 3.0, last[1], 1.0);
                }
            }
        }
    }
    Ok(())
}
fn format_bend_label(semitones: f32) -> String {
    match semitones {
        0.5 => "¼".into(),
        1.0 => "½".into(),
        2.0 => "full".into(),
        3.0 => "1½".into(),
        value => format!("{}", value / 2.0),
    }
}
fn draw_bend_arrow(page: &mut Layout, end: [f32; 2], up: bool) {
    let tail = end[1]
        + if up {
            BEND_ARROW_TAIL_LENGTH
        } else {
            -BEND_ARROW_TAIL_LENGTH
        };
    page.line(end[0] - BEND_ARROW_HALF_WIDTH, tail, end[0], end[1], 1.0);
    page.line(end[0] + BEND_ARROW_HALF_WIDTH, tail, end[0], end[1], 1.0);
}
pub(super) fn draw_beat_annotations(
    page: &mut Layout,
    a: &BeatAnnotations,
    x: f32,
    top: f32,
    rhythm: f32,
    width: f32,
    options: LayoutOptions,
) -> Result<(), RenderError> {
    use crate::elements::{dynamic_glyph, EngravingMetrics, LaneStack, MeasuredElement};

    let metrics = EngravingMetrics::default();
    let mut above = LaneStack::above(top, metrics.above_gap, metrics.annotation_gap);
    let mut below = LaneStack::below(rhythm, metrics.below_rhythm_gap, metrics.annotation_gap);

    if !a.text.is_empty() {
        above.place(
            page,
            x,
            &MeasuredElement::text(&a.text, metrics.annotation_text_size),
        )?;
    }
    if let Some(up) = a.pick_up {
        above.place(
            page,
            x,
            &MeasuredElement::text(if up { "↑" } else { "↓" }, 14.0),
        )?;
    }
    if a.golpe {
        above.place(
            page,
            x,
            &MeasuredElement::glyph(G::GuitarGolpe, metrics.music_size)?,
        )?;
    }
    if a.left_hand_tap {
        above.place(
            page,
            x,
            &MeasuredElement::glyph(G::GuitarLeftHandTapping, metrics.music_size)?,
        )?;
    }
    if let Some(open) = a.wah_open {
        above.place(
            page,
            x,
            &MeasuredElement::glyph(
                if open {
                    G::GuitarOpenPedal
                } else {
                    G::GuitarClosePedal
                },
                metrics.music_size,
            )?,
        )?;
    }
    if let Some(tempo) = a.tempo {
        above.place(
            page,
            x,
            &MeasuredElement::group(vec![
                ([-18.0, 0.0], MeasuredElement::glyph(G::NoteQuarterUp, 6.0)?),
                (
                    [12.0, 0.0],
                    MeasuredElement::text(format!("= {tempo}"), 11.0),
                ),
            ]),
        )?;
    }
    if let Some(seconds) = a.timer_seconds {
        above.place(
            page,
            x,
            &MeasuredElement::text(format!("{}:{:02}", seconds / 60, seconds % 60), 10.0),
        )?;
    }
    if let Some(fade) = a.fade {
        let left = x - width * 0.3;
        let right = x + width * 0.3;
        let yy = above.reserve(10.0);
        match fade {
            Fade::In => {
                page.line(left, yy, right, yy - 5.0, 1.0);
                page.line(left, yy, right, yy + 5.0, 1.0);
            }
            Fade::Out => {
                page.line(left, yy - 5.0, right, yy, 1.0);
                page.line(left, yy + 5.0, right, yy, 1.0);
            }
            Fade::Swell => {
                for sign in [-1.0, 1.0] {
                    page.line(left, yy, x, yy + sign * 5.0, 1.0);
                    page.line(x, yy + sign * 5.0, right, yy, 1.0);
                }
            }
        }
    }
    if let Some(technique) = a.technique {
        above.place(
            page,
            x,
            &MeasuredElement::text(
                match technique {
                    PluckingTechnique::Tap => "T",
                    PluckingTechnique::Slap => "S",
                    PluckingTechnique::Pop => "P",
                },
                12.0,
            ),
        )?;
    }
    if !a.whammy.is_empty() {
        let min_offset = a
            .whammy
            .iter()
            .map(|point| -point[1] * 4.0)
            .fold(f32::INFINITY, f32::min);
        let max_offset = a
            .whammy
            .iter()
            .map(|point| -point[1] * 4.0)
            .fold(f32::NEG_INFINITY, f32::max);
        let center = above.reserve(max_offset - min_offset + 30.0);
        let base = center - (min_offset + max_offset) / 2.0 + 6.0;
        let points: Vec<_> = a
            .whammy
            .iter()
            .map(|p| [x - width * 0.35 + p[0] * width * 0.7, base - p[1] * 4.0])
            .collect();
        for pair in points.windows(2) {
            page.line(pair[0][0], pair[0][1], pair[1][0], pair[1][1], 1.2);
        }
        page.text(
            x,
            center - (max_offset - min_offset) / 2.0 - 10.0,
            "w.bar",
            10.0,
            false,
        );
        for (i, p) in a.whammy.iter().enumerate() {
            if i == 0 || p[1] != a.whammy[i - 1][1] {
                page.text(
                    points[i][0],
                    points[i][1] - 9.0,
                    format!("{:+}", p[1] / 2.0),
                    9.0,
                    false,
                );
            }
        }
    }
    if options.show_dynamics && options.elements.dynamics {
        if let Some(dynamic) = &a.dynamic {
            if let Some(code) = dynamic_glyph(dynamic) {
                below.place(page, x, &MeasuredElement::glyph(code, metrics.music_size)?)?;
            } else {
                below.place(page, x, &MeasuredElement::text(dynamic, 12.0))?;
            }
        }
    }
    if options.show_lyrics && options.elements.lyrics && !a.lyrics.is_empty() {
        for line in a.lyrics.lines() {
            below.place(
                page,
                x,
                &MeasuredElement::text(line, metrics.lyric_text_size),
            )?;
        }
    }
    if let Some(pedal) = a.pedal {
        below.place(
            page,
            x,
            &MeasuredElement::glyph(
                if pedal == Pedal::Down {
                    G::KeyboardPedalPed
                } else {
                    G::KeyboardPedalUp
                },
                metrics.music_size,
            )?,
        )?;
    }
    if let Some(crescendo) = a.crescendo {
        let left = x - width * 0.35;
        let right = x + width * 0.35;
        let yy = below.reserve(8.0);
        let (a, b) = if crescendo { (0.0, 4.0) } else { (4.0, 0.0) };
        page.line(left, yy - a, right, yy - b, 1.0);
        page.line(left, yy + a, right, yy + b, 1.0);
    }
    // Draw a self-contained chord diagram above the beat after reserving its
    // entire height, so it cannot overlap other stacked annotations.
    if options.show_chords && options.elements.chord_diagrams {
        if let Some(chord) = &a.chord {
            let half_string_spacing = CHORD_STRING_SPACING / 2.0;
            let left = x - (chord.frets.len() - 1) as f32 * half_string_spacing;
            let right = x + (chord.frets.len() - 1) as f32 * half_string_spacing;
            let diagram_height = 36.0 + f32::from(chord.compute_rows()) * CHORD_FRET_SPACING;
            let center = above.reserve(diagram_height);
            let y = center - diagram_height / 2.0 + 22.0;
            page.text(x, y - 24.0, &chord.name, 12.0, false);
            for i in 0..=chord.compute_rows() {
                page.line(
                    left,
                    y + i as f32 * CHORD_FRET_SPACING,
                    right,
                    y + i as f32 * CHORD_FRET_SPACING,
                    if i == 0 && chord.first_fret == 1 {
                        2.0
                    } else {
                        CHORD_GRID_STROKE_WIDTH
                    },
                );
            }
            for (s, fret) in chord.frets.iter().rev().enumerate() {
                let sx = left + s as f32 * CHORD_STRING_SPACING;
                page.line(
                    sx,
                    y,
                    sx,
                    y + f32::from(chord.compute_rows()) * CHORD_FRET_SPACING,
                    CHORD_GRID_STROKE_WIDTH,
                );
                if let Some(finger) = chord.fingers.iter().rev().nth(s) {
                    page.text(
                        sx,
                        y + f32::from(chord.compute_rows()) * CHORD_FRET_SPACING + 10.0,
                        finger,
                        9.0,
                        false,
                    );
                }
                match fret {
                    None => page.text(sx, y - 9.0, "x", 10.0, false),
                    Some(0) => page.text(sx, y - 9.0, "o", 10.0, false),
                    Some(f) => {
                        page.glyph_at_center(
                            sx - 2.0,
                            y + (*f - chord.first_fret) as f32 * CHORD_FRET_SPACING + 4.0,
                            G::AugmentationDot,
                            16.0,
                        )?;
                    }
                }
            }
            for barre in &chord.barres {
                let bx1 = right - (barre.first_string - 1) as f32 * CHORD_STRING_SPACING;
                let bx2 = right - (barre.last_string - 1) as f32 * CHORD_STRING_SPACING;
                let by = y + (barre.fret - chord.first_fret) as f32 * CHORD_FRET_SPACING + 4.0;
                page.line(bx1, by, bx2, by, CHORD_BARRE_STROKE_WIDTH);
            }
            if chord.first_fret > 1 {
                page.text(left - 10.0, y + 4.0, chord.first_fret, 10.0, false);
            }
        }
    }
    Ok(())
}
