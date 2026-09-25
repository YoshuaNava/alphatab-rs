//! Scene construction for annotations and note effects.
use crate::{BeatAnnotations, Fade, Note, Pedal, PluckingTechnique, RenderError, SlideDirection};
use smufl::Glyph as G;

use super::parameters::{SMALL_GLYPH_SIZE, THIN_STROKE_WIDTH};
use super::{Scene, SceneOptions};

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
const GRACE_NOTE_OFFSET_X: f32 = 20.0;
const GRACE_NOTE_OFFSET_Y: f32 = 6.0;
const GRACE_CONNECTION_START_X: f32 = 17.0;
const GRACE_CONNECTION_END_X: f32 = 7.0;
const ARTICULATION_STACK_OFFSETS: [f32; 3] = [-9.0, -17.0, -27.0];
const TREMOLO_SLASH_SPACING: f32 = 4.0;
const VIBRATO_SAMPLE_COUNT: usize = 16;
const VIBRATO_AMPLITUDE: f32 = 2.0;
const ORNAMENT_GLYPH_SIZE: f32 = 8.0;
const GRACE_TEXT_SIZE: f32 = 10.0;
const GRACE_CONNECTION_START_Y: f32 = 3.0;
const GRACE_CURVE_START_OFFSET_X: f32 = 19.0;
const GRACE_CURVE_END_OFFSET_X: f32 = 7.0;
const GRACE_BEND_CURVE_OFFSET_Y: f32 = 5.0;
const GRACE_BEND_CURVE_HEIGHT: f32 = -6.0;
const GRACE_SLUR_CURVE_OFFSET_Y: f32 = 3.0;
const GRACE_SLUR_CURVE_HEIGHT: f32 = 4.0;
const TREMOLO_START_OFFSET_X: f32 = 4.0;
const TREMOLO_END_OFFSET_X: f32 = 5.0;
const TREMOLO_START_OFFSET_Y: f32 = 9.0;
const TREMOLO_END_OFFSET_Y: f32 = 5.0;
const TREMOLO_STROKE_WIDTH: f32 = 2.0;
const VIBRATO_START_WIDTH_FRACTION: f32 = 1.0 / 3.0;
const VIBRATO_WIDTH_FRACTION: f32 = 2.0 / 3.0;
const VIBRATO_BASELINE_OFFSET_Y: f32 = 6.0;
const BEND_STROKE_WIDTH: f32 = 1.0;
const BEND_LABEL_TEXT_SIZE: f32 = 9.0;
const BEND_VIBRATO_SAMPLE_COUNT: usize = 6;
const BEND_VIBRATO_START_OFFSET_X: f32 = 18.0;
const BEND_VIBRATO_SAMPLE_SPACING: f32 = 3.0;
const BEND_VIBRATO_PEAK_OFFSET_X: f32 = 1.5;
const BEND_VIBRATO_PEAK_OFFSET_Y: f32 = 3.0;

// Chord-diagram geometry is deliberately fixed so diagrams remain legible at
// every beat width.
const CHORD_STRING_SPACING: f32 = 10.0;
const CHORD_FRET_SPACING: f32 = 8.0;
const CHORD_GRID_STROKE_WIDTH: f32 = 0.7;
const CHORD_BARRE_STROKE_WIDTH: f32 = 4.0;
const PICKUP_TEXT_SIZE: f32 = 14.0;
const TEMPO_NOTE_OFFSET_X: f32 = 18.0;
const TEMPO_NOTE_GLYPH_SIZE: f32 = 6.0;
const TEMPO_TEXT_OFFSET_X: f32 = 12.0;
const TEMPO_TEXT_SIZE: f32 = 11.0;
const TIMER_TEXT_SIZE: f32 = 10.0;
const SECONDS_PER_MINUTE: u32 = 60;
const FADE_WIDTH_FRACTION: f32 = 0.3;
const FADE_LANE_HEIGHT: f32 = 10.0;
const FADE_OPENING: f32 = 5.0;
const TECHNIQUE_TEXT_SIZE: f32 = 12.0;
const WHAMMY_VERTICAL_SCALE: f32 = 4.0;
const WHAMMY_VERTICAL_PADDING: f32 = 30.0;
const WHAMMY_BASELINE_OFFSET_Y: f32 = 6.0;
const WHAMMY_STROKE_WIDTH: f32 = 1.2;
const WHAMMY_LABEL_OFFSET_Y: f32 = 10.0;
const WHAMMY_LABEL_TEXT_SIZE: f32 = 10.0;
const WHAMMY_POINT_LABEL_OFFSET_Y: f32 = 9.0;
const WHAMMY_POINT_LABEL_TEXT_SIZE: f32 = 9.0;
const DYNAMIC_TEXT_SIZE: f32 = 12.0;
const CRESCENDO_WIDTH_FRACTION: f32 = 0.35;
const CRESCENDO_LANE_HEIGHT: f32 = 8.0;
const CRESCENDO_OPENING: f32 = 4.0;
const CHORD_DIAGRAM_BASE_HEIGHT: f32 = 36.0;
const CHORD_DIAGRAM_GRID_OFFSET_Y: f32 = 22.0;
const CHORD_NAME_OFFSET_Y: f32 = 24.0;
const CHORD_NAME_TEXT_SIZE: f32 = 12.0;
const CHORD_NUT_STROKE_WIDTH: f32 = 2.0;
const CHORD_FINGER_OFFSET_Y: f32 = 10.0;
const CHORD_FINGER_TEXT_SIZE: f32 = 9.0;
const CHORD_MARKER_OFFSET_Y: f32 = 9.0;
const CHORD_MARKER_TEXT_SIZE: f32 = 10.0;
const CHORD_DOT_OFFSET_X: f32 = 2.0;
const CHORD_DOT_OFFSET_Y: f32 = 4.0;
const CHORD_DOT_GLYPH_SIZE: f32 = 16.0;
const CHORD_FIRST_FRET_LABEL_OFFSET_X: f32 = 10.0;

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
            y - ORNAMENT_OFFSET_Y,
            ornament.resolve_glyph(),
            ORNAMENT_GLYPH_SIZE,
        )?;
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
            x - GRACE_NOTE_OFFSET_X,
            y - GRACE_NOTE_OFFSET_Y,
            if e.grace_dead {
                "x".into()
            } else {
                fret.to_string()
            },
            GRACE_TEXT_SIZE,
            true,
        );
        if e.grace_slide {
            page.line(
                x - GRACE_CONNECTION_START_X,
                y + GRACE_CONNECTION_START_Y,
                x - GRACE_CONNECTION_END_X,
                y,
                THIN_STROKE_WIDTH,
            );
        } else if e.grace_bend {
            page.curve(
                [
                    x - GRACE_CURVE_START_OFFSET_X,
                    y - GRACE_BEND_CURVE_OFFSET_Y,
                ],
                [x - GRACE_CURVE_END_OFFSET_X, y - GRACE_BEND_CURVE_OFFSET_Y],
                GRACE_BEND_CURVE_HEIGHT,
            );
        } else if e.grace_slur {
            page.curve(
                [
                    x - GRACE_CURVE_START_OFFSET_X,
                    y + GRACE_SLUR_CURVE_OFFSET_Y,
                ],
                [x - GRACE_CURVE_END_OFFSET_X, y + GRACE_SLUR_CURVE_OFFSET_Y],
                GRACE_SLUR_CURVE_HEIGHT,
            );
        }
    }
    if let Some(fret) = e.trill_fret {
        page.text(x, lane, format!("tr {fret}"), EFFECT_TEXT_SIZE, false);
    }
    for (enabled, code, offset) in [
        (
            e.staccato,
            G::ArticStaccatoAbove,
            ARTICULATION_STACK_OFFSETS[0],
        ),
        (e.accent, G::ArticAccentAbove, ARTICULATION_STACK_OFFSETS[1]),
        (
            e.heavy_accent,
            G::ArticMarcatoAbove,
            ARTICULATION_STACK_OFFSETS[2],
        ),
    ] {
        if enabled {
            page.glyph_at_center(x, y + offset, code, SMALL_GLYPH_SIZE)?;
        }
    }
    for i in 0..e.tremolo_slashes {
        page.line(
            x - TREMOLO_START_OFFSET_X,
            y + TREMOLO_START_OFFSET_Y + i as f32 * TREMOLO_SLASH_SPACING,
            x + TREMOLO_END_OFFSET_X,
            y + TREMOLO_END_OFFSET_Y + i as f32 * TREMOLO_SLASH_SPACING,
            TREMOLO_STROKE_WIDTH,
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
        let mut last = [
            x - width * VIBRATO_START_WIDTH_FRACTION,
            lane - VIBRATO_BASELINE_OFFSET_Y,
        ];
        for i in 1..=VIBRATO_SAMPLE_COUNT {
            let p = [
                x - width * VIBRATO_START_WIDTH_FRACTION
                    + width * VIBRATO_WIDTH_FRACTION * i as f32 / VIBRATO_SAMPLE_COUNT as f32,
                lane - VIBRATO_BASELINE_OFFSET_Y
                    + (i as f32 * std::f32::consts::FRAC_PI_2).sin() * VIBRATO_AMPLITUDE,
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
                    x - width * BEND_HORIZONTAL_INSET
                        + p[0] * width * (1.0 - BEND_HORIZONTAL_INSET * 2.0),
                    lane - p[1] * BEND_VERTICAL_SCALE,
                ]
            })
            .collect();
        if e.bend[0][1] != 0.0 {
            let first = points[0];
            page.line(first[0], lane, first[0], first[1], BEND_STROKE_WIDTH);
            draw_bend_arrow(page, first, e.bend[0][1] > 0.0);
            page.text(
                first[0],
                first[1] - BEND_LABEL_OFFSET_Y,
                format_bend_label(e.bend[0][1]),
                BEND_LABEL_TEXT_SIZE,
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
                for i in 0..BEND_VIBRATO_SAMPLE_COUNT {
                    let xx = last[0] - BEND_VIBRATO_START_OFFSET_X
                        + i as f32 * BEND_VIBRATO_SAMPLE_SPACING;
                    page.line(
                        xx,
                        last[1],
                        xx + BEND_VIBRATO_PEAK_OFFSET_X,
                        last[1] - BEND_VIBRATO_PEAK_OFFSET_Y,
                        BEND_STROKE_WIDTH,
                    );
                    page.line(
                        xx + BEND_VIBRATO_PEAK_OFFSET_X,
                        last[1] - BEND_VIBRATO_PEAK_OFFSET_Y,
                        xx + BEND_VIBRATO_SAMPLE_SPACING,
                        last[1],
                        BEND_STROKE_WIDTH,
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
    let tail = end[1]
        + if up {
            BEND_ARROW_TAIL_LENGTH
        } else {
            -BEND_ARROW_TAIL_LENGTH
        };
    page.line(
        end[0] - BEND_ARROW_HALF_WIDTH,
        tail,
        end[0],
        end[1],
        BEND_STROKE_WIDTH,
    );
    page.line(
        end[0] + BEND_ARROW_HALF_WIDTH,
        tail,
        end[0],
        end[1],
        BEND_STROKE_WIDTH,
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
            &MeasuredElement::text(if up { "↑" } else { "↓" }, PICKUP_TEXT_SIZE),
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
                    [-TEMPO_NOTE_OFFSET_X, 0.0],
                    MeasuredElement::glyph(G::NoteQuarterUp, TEMPO_NOTE_GLYPH_SIZE)?,
                ),
                (
                    [TEMPO_TEXT_OFFSET_X, 0.0],
                    MeasuredElement::text(format!("= {tempo}"), TEMPO_TEXT_SIZE),
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
                TIMER_TEXT_SIZE,
            ),
        )?;
    }
    if let Some(fade) = a.fade {
        let left = x - width * FADE_WIDTH_FRACTION;
        let right = x + width * FADE_WIDTH_FRACTION;
        let yy = above.reserve(FADE_LANE_HEIGHT);
        match fade {
            Fade::In => {
                page.line(left, yy, right, yy - FADE_OPENING, BEND_STROKE_WIDTH);
                page.line(left, yy, right, yy + FADE_OPENING, BEND_STROKE_WIDTH);
            }
            Fade::Out => {
                page.line(left, yy - FADE_OPENING, right, yy, BEND_STROKE_WIDTH);
                page.line(left, yy + FADE_OPENING, right, yy, BEND_STROKE_WIDTH);
            }
            Fade::Swell => {
                for sign in [-1.0, 1.0] {
                    page.line(left, yy, x, yy + sign * FADE_OPENING, BEND_STROKE_WIDTH);
                    page.line(x, yy + sign * FADE_OPENING, right, yy, BEND_STROKE_WIDTH);
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
                TECHNIQUE_TEXT_SIZE,
            ),
        )?;
    }
    if !a.whammy.is_empty() {
        let min_offset = a
            .whammy
            .iter()
            .map(|point| -point[1] * WHAMMY_VERTICAL_SCALE)
            .fold(f32::INFINITY, f32::min);
        let max_offset = a
            .whammy
            .iter()
            .map(|point| -point[1] * WHAMMY_VERTICAL_SCALE)
            .fold(f32::NEG_INFINITY, f32::max);
        let center = above.reserve(max_offset - min_offset + WHAMMY_VERTICAL_PADDING);
        let base = center - (min_offset + max_offset) / 2.0 + WHAMMY_BASELINE_OFFSET_Y;
        let points: Vec<_> = a
            .whammy
            .iter()
            .map(|p| {
                [
                    x - width * CRESCENDO_WIDTH_FRACTION
                        + p[0] * width * (CRESCENDO_WIDTH_FRACTION * 2.0),
                    base - p[1] * WHAMMY_VERTICAL_SCALE,
                ]
            })
            .collect();
        for pair in points.windows(2) {
            page.line(
                pair[0][0],
                pair[0][1],
                pair[1][0],
                pair[1][1],
                WHAMMY_STROKE_WIDTH,
            );
        }
        page.text(
            x,
            center - (max_offset - min_offset) / 2.0 - WHAMMY_LABEL_OFFSET_Y,
            "w.bar",
            WHAMMY_LABEL_TEXT_SIZE,
            false,
        );
        for (i, p) in a.whammy.iter().enumerate() {
            if i == 0 || p[1] != a.whammy[i - 1][1] {
                page.text(
                    points[i][0],
                    points[i][1] - WHAMMY_POINT_LABEL_OFFSET_Y,
                    format!("{:+}", p[1] / 2.0),
                    WHAMMY_POINT_LABEL_TEXT_SIZE,
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
                below.place(page, x, &MeasuredElement::text(dynamic, DYNAMIC_TEXT_SIZE))?;
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
        let left = x - width * CRESCENDO_WIDTH_FRACTION;
        let right = x + width * CRESCENDO_WIDTH_FRACTION;
        let yy = below.reserve(CRESCENDO_LANE_HEIGHT);
        let (a, b) = if crescendo {
            (0.0, CRESCENDO_OPENING)
        } else {
            (CRESCENDO_OPENING, 0.0)
        };
        page.line(left, yy - a, right, yy - b, BEND_STROKE_WIDTH);
        page.line(left, yy + a, right, yy + b, BEND_STROKE_WIDTH);
    }
    // Draw a self-contained chord diagram above the beat after reserving its
    // entire height, so it cannot overlap other stacked annotations.
    if options.show_chords && options.elements.chord_diagrams {
        if let Some(chord) = &a.chord {
            let half_string_spacing = CHORD_STRING_SPACING / 2.0;
            let left = x - (chord.frets.len() - 1) as f32 * half_string_spacing;
            let right = x + (chord.frets.len() - 1) as f32 * half_string_spacing;
            let diagram_height =
                CHORD_DIAGRAM_BASE_HEIGHT + f32::from(chord.compute_rows()) * CHORD_FRET_SPACING;
            let center = above.reserve(diagram_height);
            let y = center - diagram_height / 2.0 + CHORD_DIAGRAM_GRID_OFFSET_Y;
            page.text(
                x,
                y - CHORD_NAME_OFFSET_Y,
                &chord.name,
                CHORD_NAME_TEXT_SIZE,
                false,
            );
            for i in 0..=chord.compute_rows() {
                page.line(
                    left,
                    y + i as f32 * CHORD_FRET_SPACING,
                    right,
                    y + i as f32 * CHORD_FRET_SPACING,
                    if i == 0 && chord.first_fret == 1 {
                        CHORD_NUT_STROKE_WIDTH
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
                        y + f32::from(chord.compute_rows()) * CHORD_FRET_SPACING
                            + CHORD_FINGER_OFFSET_Y,
                        finger,
                        CHORD_FINGER_TEXT_SIZE,
                        false,
                    );
                }
                match fret {
                    None => page.text(
                        sx,
                        y - CHORD_MARKER_OFFSET_Y,
                        "x",
                        CHORD_MARKER_TEXT_SIZE,
                        false,
                    ),
                    Some(0) => page.text(
                        sx,
                        y - CHORD_MARKER_OFFSET_Y,
                        "o",
                        CHORD_MARKER_TEXT_SIZE,
                        false,
                    ),
                    Some(f) => {
                        page.glyph_at_center(
                            sx - CHORD_DOT_OFFSET_X,
                            y + (*f - chord.first_fret) as f32 * CHORD_FRET_SPACING
                                + CHORD_DOT_OFFSET_Y,
                            G::AugmentationDot,
                            CHORD_DOT_GLYPH_SIZE,
                        )?;
                    }
                }
            }
            for barre in &chord.barres {
                let bx1 = right - (barre.first_string - 1) as f32 * CHORD_STRING_SPACING;
                let bx2 = right - (barre.last_string - 1) as f32 * CHORD_STRING_SPACING;
                let by = y
                    + (barre.fret - chord.first_fret) as f32 * CHORD_FRET_SPACING
                    + CHORD_DOT_OFFSET_Y;
                page.line(bx1, by, bx2, by, CHORD_BARRE_STROKE_WIDTH);
            }
            if chord.first_fret > 1 {
                page.text(
                    left - CHORD_FIRST_FRET_LABEL_OFFSET_X,
                    y + CHORD_DOT_OFFSET_Y,
                    chord.first_fret,
                    CHORD_MARKER_TEXT_SIZE,
                    false,
                );
            }
        }
    }
    Ok(())
}
