//! Scene construction for annotations and note effects.
use crate::{BeatAnnotations, Fade, Note, Pedal, PluckingTechnique, RenderError, SlideDirection};
use smufl::Glyph as G;

use super::parameters::{
    ANNOTATION_TEXT_SIZE, DIATONIC_STEPS_PER_OCTAVE, DOT_SPACING, EMPHASIZED_STROKE_WIDTH,
    EMPHASIZED_TEXT_SIZE, LARGE_TEXT_SIZE, NOTE_GLYPH_SIZE, SMALL_GLYPH_SIZE, SMALL_TEXT_SIZE,
    STAFF_HEIGHT, STAFF_LINE_SPACING, STAFF_MIDDLE_LINE_OFFSET, STANDARD_TEXT_SIZE,
    THIN_STROKE_WIDTH,
};
use super::{Scene, SceneOptions};

// Note-effect geometry is expressed in scene units relative to a notehead.

// Chord-diagram geometry is deliberately fixed so diagrams remain legible at
// every beat width.
const SECONDS_PER_MINUTE: u32 = 60;

pub(super) fn draw_note_effects(
    page: &mut Scene,
    n: &Note,
    position: [f32; 4],
    bend_curve: bool,
) -> Result<(), RenderError> {
    let [x, y, width, lane] = position;
    let e = &n.effects;
    // Place compact note-attached effects around the notehead before adding
    // lane-based labels and curves above the beat.
    if let Some(ornament) = e.ornament {
        page.glyph_at_center(
            x,
            y - STAFF_HEIGHT,
            ornament.resolve_glyph(),
            NOTE_GLYPH_SIZE,
        )?;
    }
    if let Some(direction) = e.slide_in {
        let delta = if direction == SlideDirection::Up {
            SMALL_GLYPH_SIZE
        } else {
            -SMALL_GLYPH_SIZE
        };
        page.line(
            x - STAFF_LINE_SPACING - STAFF_LINE_SPACING,
            y + delta,
            x - STAFF_LINE_SPACING,
            y,
            EMPHASIZED_STROKE_WIDTH,
        );
    }
    if let Some(direction) = e.slide_out {
        let delta = if direction == SlideDirection::Up {
            -SMALL_GLYPH_SIZE
        } else {
            SMALL_GLYPH_SIZE
        };
        page.line(
            x + STAFF_LINE_SPACING,
            y,
            x + STAFF_LINE_SPACING + STAFF_LINE_SPACING,
            y + delta,
            EMPHASIZED_STROKE_WIDTH,
        );
    }
    if let Some(fret) = e.grace_fret {
        page.text(
            x - STAFF_MIDDLE_LINE_OFFSET,
            y - SMALL_GLYPH_SIZE,
            if e.grace_dead {
                "x".into()
            } else {
                fret.to_string()
            },
            STANDARD_TEXT_SIZE,
            true,
        );
        if e.grace_slide {
            page.line(
                x - STAFF_MIDDLE_LINE_OFFSET,
                y + THIN_STROKE_WIDTH,
                x - SMALL_GLYPH_SIZE,
                y,
                THIN_STROKE_WIDTH,
            );
        } else if e.grace_bend {
            page.curve(
                [x - STAFF_MIDDLE_LINE_OFFSET, y - DOT_SPACING],
                [x - SMALL_GLYPH_SIZE, y - DOT_SPACING],
                -SMALL_GLYPH_SIZE,
            );
        } else if e.grace_slur {
            page.curve(
                [x - STAFF_MIDDLE_LINE_OFFSET, y + THIN_STROKE_WIDTH],
                [x - SMALL_GLYPH_SIZE, y + THIN_STROKE_WIDTH],
                DOT_SPACING,
            );
        }
    }
    if let Some(fret) = e.trill_fret {
        page.text(x, lane, format!("tr {fret}"), STANDARD_TEXT_SIZE, false);
    }
    for (enabled, code, offset) in [
        (
            e.staccato,
            G::ArticStaccatoAbove,
            [
                -SMALL_TEXT_SIZE,
                -SMALL_TEXT_SIZE - NOTE_GLYPH_SIZE,
                -STAFF_HEIGHT + SMALL_GLYPH_SIZE,
            ][0],
        ),
        (
            e.accent,
            G::ArticAccentAbove,
            [
                -SMALL_TEXT_SIZE,
                -SMALL_TEXT_SIZE - NOTE_GLYPH_SIZE,
                -STAFF_HEIGHT + SMALL_GLYPH_SIZE,
            ][1],
        ),
        (
            e.heavy_accent,
            G::ArticMarcatoAbove,
            [
                -SMALL_TEXT_SIZE,
                -SMALL_TEXT_SIZE - NOTE_GLYPH_SIZE,
                -STAFF_HEIGHT + SMALL_GLYPH_SIZE,
            ][2],
        ),
    ] {
        if enabled {
            page.glyph_at_center(x, y + offset, code, SMALL_GLYPH_SIZE)?;
        }
    }
    for i in 0..e.tremolo_slashes {
        page.line(
            x - THIN_STROKE_WIDTH,
            y + SMALL_TEXT_SIZE + i as f32 * DOT_SPACING,
            x + DOT_SPACING,
            y + DOT_SPACING + i as f32 * DOT_SPACING,
            EMPHASIZED_STROKE_WIDTH,
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
        page.text(x, lane, labels.join(" "), STANDARD_TEXT_SIZE, false);
    }
    if e.vibrato {
        let mut last = [x - width * (1.0 / 3.0), lane - SMALL_GLYPH_SIZE];
        for i in 1..=(DIATONIC_STEPS_PER_OCTAVE as usize * 2) {
            let p = [
                x - width * (1.0 / 3.0)
                    + width * (2.0 / 3.0) * i as f32
                        / (DIATONIC_STEPS_PER_OCTAVE as usize * 2) as f32,
                lane - SMALL_GLYPH_SIZE
                    + (i as f32 * std::f32::consts::FRAC_PI_2).sin() * THIN_STROKE_WIDTH,
            ];
            page.line(last[0], last[1], p[0], p[1], THIN_STROKE_WIDTH);
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
                    x - width * (1.0 / 3.0) + p[0] * width * (1.0 - (1.0 / 3.0) * 2.0),
                    lane - p[1] * SMALL_TEXT_SIZE,
                ]
            })
            .collect();
        if e.bend[0][1] != 0.0 {
            let first = points[0];
            page.line(first[0], lane, first[0], first[1], THIN_STROKE_WIDTH);
            draw_bend_arrow(page, first, e.bend[0][1] > 0.0);
            page.text(
                first[0],
                first[1] - SMALL_TEXT_SIZE,
                format_bend_label(e.bend[0][1]),
                SMALL_TEXT_SIZE,
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
                    pair[1][1] - SMALL_TEXT_SIZE,
                    format_bend_label(e.bend[i + 1][1]),
                    SMALL_TEXT_SIZE,
                    false,
                );
            }
        }
        if e.bend_vibrato {
            if let Some(last) = points.last() {
                for i in 0..(DIATONIC_STEPS_PER_OCTAVE as usize) {
                    let xx = last[0] - STAFF_MIDDLE_LINE_OFFSET
                        + i as f32 * (THIN_STROKE_WIDTH + THIN_STROKE_WIDTH);
                    page.line(
                        xx,
                        last[1],
                        xx + THIN_STROKE_WIDTH,
                        last[1] - (THIN_STROKE_WIDTH + THIN_STROKE_WIDTH),
                        THIN_STROKE_WIDTH,
                    );
                    page.line(
                        xx + THIN_STROKE_WIDTH,
                        last[1] - (THIN_STROKE_WIDTH + THIN_STROKE_WIDTH),
                        xx + (THIN_STROKE_WIDTH + THIN_STROKE_WIDTH),
                        last[1],
                        THIN_STROKE_WIDTH,
                    );
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
fn draw_bend_arrow(page: &mut Scene, end: [f32; 2], up: bool) {
    let tail = end[1] + if up { DOT_SPACING } else { -DOT_SPACING };
    page.line(
        end[0] - THIN_STROKE_WIDTH,
        tail,
        end[0],
        end[1],
        THIN_STROKE_WIDTH,
    );
    page.line(
        end[0] + THIN_STROKE_WIDTH,
        tail,
        end[0],
        end[1],
        THIN_STROKE_WIDTH,
    );
}
pub(super) fn draw_beat_annotations(
    page: &mut Scene,
    a: &BeatAnnotations,
    x: f32,
    top: f32,
    rhythm: f32,
    width: f32,
    options: SceneOptions,
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
            &MeasuredElement::text(if up { "↑" } else { "↓" }, LARGE_TEXT_SIZE),
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
                (
                    [-STAFF_LINE_SPACING - SMALL_GLYPH_SIZE, 0.0],
                    MeasuredElement::glyph(G::NoteQuarterUp, SMALL_GLYPH_SIZE)?,
                ),
                (
                    [EMPHASIZED_TEXT_SIZE, 0.0],
                    MeasuredElement::text(format!("= {tempo}"), ANNOTATION_TEXT_SIZE),
                ),
            ]),
        )?;
    }
    if let Some(seconds) = a.timer_seconds {
        above.place(
            page,
            x,
            &MeasuredElement::text(
                format!(
                    "{}:{:02}",
                    seconds / SECONDS_PER_MINUTE,
                    seconds % SECONDS_PER_MINUTE
                ),
                STANDARD_TEXT_SIZE,
            ),
        )?;
    }
    if let Some(fade) = a.fade {
        let left = x - width * (1.0 / 3.0);
        let right = x + width * (1.0 / 3.0);
        let yy = above.reserve(STAFF_LINE_SPACING);
        match fade {
            Fade::In => {
                page.line(left, yy, right, yy - DOT_SPACING, THIN_STROKE_WIDTH);
                page.line(left, yy, right, yy + DOT_SPACING, THIN_STROKE_WIDTH);
            }
            Fade::Out => {
                page.line(left, yy - DOT_SPACING, right, yy, THIN_STROKE_WIDTH);
                page.line(left, yy + DOT_SPACING, right, yy, THIN_STROKE_WIDTH);
            }
            Fade::Swell => {
                for sign in [-1.0, 1.0] {
                    page.line(left, yy, x, yy + sign * DOT_SPACING, THIN_STROKE_WIDTH);
                    page.line(x, yy + sign * DOT_SPACING, right, yy, THIN_STROKE_WIDTH);
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
                EMPHASIZED_TEXT_SIZE,
            ),
        )?;
    }
    if !a.whammy.is_empty() {
        let min_offset = a
            .whammy
            .iter()
            .map(|point| -point[1] * (SMALL_GLYPH_SIZE / 2.0))
            .fold(f32::INFINITY, f32::min);
        let max_offset = a
            .whammy
            .iter()
            .map(|point| -point[1] * (SMALL_GLYPH_SIZE / 2.0))
            .fold(f32::NEG_INFINITY, f32::max);
        let center = above.reserve(max_offset - min_offset + STAFF_HEIGHT);
        let base = center - (min_offset + max_offset) / 2.0 + SMALL_GLYPH_SIZE;
        let points: Vec<_> = a
            .whammy
            .iter()
            .map(|p| {
                [
                    x - width * (1.0 / 3.0) + p[0] * width * ((1.0 / 3.0) * 2.0),
                    base - p[1] * (SMALL_GLYPH_SIZE / 2.0),
                ]
            })
            .collect();
        for pair in points.windows(2) {
            page.line(
                pair[0][0],
                pair[0][1],
                pair[1][0],
                pair[1][1],
                EMPHASIZED_STROKE_WIDTH,
            );
        }
        page.text(
            x,
            center - (max_offset - min_offset) / 2.0 - STAFF_LINE_SPACING,
            "w.bar",
            STANDARD_TEXT_SIZE,
            false,
        );
        for (i, p) in a.whammy.iter().enumerate() {
            if i == 0 || p[1] != a.whammy[i - 1][1] {
                page.text(
                    points[i][0],
                    points[i][1] - SMALL_TEXT_SIZE,
                    format!("{:+}", p[1] / 2.0),
                    SMALL_TEXT_SIZE,
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
                below.place(
                    page,
                    x,
                    &MeasuredElement::text(dynamic, EMPHASIZED_TEXT_SIZE),
                )?;
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
        let left = x - width * (1.0 / 3.0);
        let right = x + width * (1.0 / 3.0);
        let yy = below.reserve(NOTE_GLYPH_SIZE);
        let (a, b) = if crescendo {
            (0.0, DOT_SPACING)
        } else {
            (DOT_SPACING, 0.0)
        };
        page.line(left, yy - a, right, yy - b, THIN_STROKE_WIDTH);
        page.line(left, yy + a, right, yy + b, THIN_STROKE_WIDTH);
    }
    // Draw a self-contained chord diagram above the beat after reserving its
    // entire height, so it cannot overlap other stacked annotations.
    if options.show_chords && options.elements.chord_diagrams {
        if let Some(chord) = &a.chord {
            let half_string_spacing = STAFF_LINE_SPACING / 2.0;
            let left = x - (chord.frets.len() - 1) as f32 * half_string_spacing;
            let right = x + (chord.frets.len() - 1) as f32 * half_string_spacing;
            let diagram_height = STAFF_HEIGHT + f32::from(chord.compute_rows()) * NOTE_GLYPH_SIZE;
            let center = above.reserve(diagram_height);
            let y = center - diagram_height / 2.0 + STAFF_MIDDLE_LINE_OFFSET;
            page.text(
                x,
                y - (STAFF_HEIGHT - SMALL_GLYPH_SIZE),
                &chord.name,
                EMPHASIZED_TEXT_SIZE,
                false,
            );
            for i in 0..=chord.compute_rows() {
                page.line(
                    left,
                    y + i as f32 * NOTE_GLYPH_SIZE,
                    right,
                    y + i as f32 * NOTE_GLYPH_SIZE,
                    if i == 0 && chord.first_fret == 1 {
                        EMPHASIZED_STROKE_WIDTH
                    } else {
                        THIN_STROKE_WIDTH
                    },
                );
            }
            for (s, fret) in chord.frets.iter().rev().enumerate() {
                let sx = left + s as f32 * STAFF_LINE_SPACING;
                page.line(
                    sx,
                    y,
                    sx,
                    y + f32::from(chord.compute_rows()) * NOTE_GLYPH_SIZE,
                    THIN_STROKE_WIDTH,
                );
                if let Some(finger) = chord.fingers.iter().rev().nth(s) {
                    page.text(
                        sx,
                        y + f32::from(chord.compute_rows()) * NOTE_GLYPH_SIZE + STAFF_LINE_SPACING,
                        finger,
                        SMALL_TEXT_SIZE,
                        false,
                    );
                }
                match fret {
                    None => page.text(sx, y - SMALL_TEXT_SIZE, "x", STANDARD_TEXT_SIZE, false),
                    Some(0) => page.text(sx, y - SMALL_TEXT_SIZE, "o", STANDARD_TEXT_SIZE, false),
                    Some(f) => {
                        page.glyph_at_center(
                            sx - (THIN_STROKE_WIDTH + THIN_STROKE_WIDTH),
                            y + (*f - chord.first_fret) as f32 * NOTE_GLYPH_SIZE + DOT_SPACING,
                            G::AugmentationDot,
                            LARGE_TEXT_SIZE,
                        )?;
                    }
                }
            }
            for barre in &chord.barres {
                let bx1 = right - (barre.first_string - 1) as f32 * STAFF_LINE_SPACING;
                let bx2 = right - (barre.last_string - 1) as f32 * STAFF_LINE_SPACING;
                let by = y + (barre.fret - chord.first_fret) as f32 * NOTE_GLYPH_SIZE + DOT_SPACING;
                page.line(bx1, by, bx2, by, EMPHASIZED_STROKE_WIDTH);
            }
            if chord.first_fret > 1 {
                page.text(
                    left - STAFF_LINE_SPACING,
                    y + DOT_SPACING,
                    chord.first_fret,
                    STANDARD_TEXT_SIZE,
                    false,
                );
            }
        }
    }
    Ok(())
}
