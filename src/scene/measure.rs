//! Scene construction for measure lines, headers, repeats, and navigation marks.

use crate::{
    Clef, DisplayMode, FermataKind, KeySignature, Measure, Navigation, RenderError, Simile, Track,
};
use smufl::Glyph as G;

use super::parameters::{
    EMPHASIZED_STROKE_WIDTH, STAFF_HEIGHT, STAFF_MIDDLE_LINE_OFFSET, THIN_STROKE_WIDTH,
    TIMELINE_EPSILON,
};
use super::planning::MeasurePlan;
use super::staff::draw_staff;
use super::{Scene, SceneOptions};

const PAGE_LEFT_EDGE: f32 = 44.0;
const TAB_STRING_LABEL_X: f32 = 22.0;
const TAB_STRING_LABEL_TEXT_SIZE: f32 = 13.0;
const DOUBLE_BAR_GAP: f32 = 4.0;
const FINAL_BAR_STROKE_WIDTH: f32 = 3.0;
const BAR_NUMBER_OFFSET_X: f32 = 10.0;
const BAR_NUMBER_OFFSET_Y: f32 = 36.0;
const BAR_NUMBER_TEXT_SIZE: f32 = 11.0;
const FREE_TIME_OFFSET_X: f32 = 50.0;
const FREE_TIME_OFFSET_Y: f32 = 95.0;
const FREE_TIME_TEXT_SIZE: f32 = 11.0;
const TRIPLET_FEEL_OFFSET_Y: f32 = 110.0;
const TRIPLET_FEEL_TEXT_SIZE: f32 = 11.0;
const SIMILE_OFFSET_X: f32 = 10.0;
const SIMILE_GLYPH_SIZE: f32 = 12.0;
const NAVIGATION_INSTRUCTION_OFFSET_Y: f32 = 78.0;
const NAVIGATION_INSTRUCTION_TEXT_SIZE: f32 = 12.0;
const NAVIGATION_SYMBOL_OFFSET_Y: f32 = 68.0;
const NAVIGATION_SYMBOL_GLYPH_SIZE: f32 = 10.0;
const DOUBLE_NAVIGATION_SYMBOL_GAP: f32 = 20.0;
const FERMATA_OFFSET_X: f32 = 6.0;
const FERMATA_OFFSET_Y: f32 = 12.0;
const FERMATA_GLYPH_SIZE: f32 = 9.0;
const REPEAT_BAR_STROKE_WIDTH: f32 = 2.5;
const REPEAT_DOT_OFFSET_Y: f32 = 5.0;
const REPEAT_END_DOT_OFFSET_X: f32 = 12.0;
const REPEAT_DOT_GLYPH_SIZE: f32 = 7.0;
const REPEAT_COUNT_RIGHT_INSET: f32 = 22.0;
const REPEAT_COUNT_OFFSET_Y: f32 = 35.0;
const REPEAT_COUNT_TEXT_SIZE: f32 = 11.0;
const TAB_TIME_SIGNATURE_OFFSET_X: f32 = 38.0;
const TAB_TIME_SIGNATURE_OFFSET_Y: f32 = 20.0;
const TAB_TIME_SIGNATURE_TEXT_SIZE: f32 = 12.0;
const STAFF_TIME_SIGNATURE_OFFSET_X: f32 = 17.0;
const STAFF_TIME_SIGNATURE_NUMERATOR_OFFSET_Y: f32 = 12.0;
const STAFF_TIME_SIGNATURE_DENOMINATOR_OFFSET_Y: f32 = 30.0;
const STAFF_TIME_SIGNATURE_TEXT_SIZE: f32 = 15.0;
const NUMBERED_KEY_OFFSET_X: f32 = 42.0;
const NUMBERED_KEY_OFFSET_Y: f32 = 42.0;
const NUMBERED_KEY_TEXT_SIZE: f32 = 11.0;
const TEMPO_NOTE_OFFSET_X: f32 = 60.0;
const TEMPO_NOTE_OFFSET_Y: f32 = 36.0;
const TEMPO_NOTE_GLYPH_SIZE: f32 = 6.0;
const TEMPO_TEXT_OFFSET_X: f32 = 92.0;
const TEMPO_TEXT_OFFSET_Y: f32 = 43.0;
const TEMPO_TEXT_SIZE: f32 = 11.0;
const MARKER_OFFSET_Y: f32 = 65.0;
const MARKER_TEXT_SIZE: f32 = 13.0;
const ALTERNATE_ENDING_LINE_OFFSET_Y: f32 = 12.0;
const ALTERNATE_ENDING_HOOK_HEIGHT: f32 = 8.0;
const ALTERNATE_ENDING_LABEL_OFFSET_X: f32 = 25.0;
const ALTERNATE_ENDING_LABEL_OFFSET_Y: f32 = 22.0;
const ALTERNATE_ENDING_LABEL_TEXT_SIZE: f32 = 11.0;
const MULTI_REST_STROKE_WIDTH: f32 = 5.0;
const MULTI_REST_END_HEIGHT: f32 = 9.0;
const MULTI_REST_LEFT_INSET: f32 = 8.0;
const MULTI_REST_RIGHT_INSET: f32 = 22.0;
const MULTI_REST_COUNT_OFFSET_Y: f32 = 24.0;
const MULTI_REST_COUNT_TEXT_SIZE: f32 = 13.0;

/// Shared model and geometry used while drawing one measure frame.
pub(super) struct MeasureFrame<'a> {
    pub(super) track: &'a Track,
    pub(super) measure: &'a Measure,
    pub(super) plan: &'a MeasurePlan,
    pub(super) index: usize,
    pub(super) options: SceneOptions,
    pub(super) x: f32,
    pub(super) staff_y: f32,
    pub(super) tab_y: f32,
    pub(super) bottom: f32,
    pub(super) tab: bool,
    pub(super) staff: bool,
    pub(super) tab_height: f32,
    pub(super) clef: Clef,
}

/// Draws the frame, labels, signatures, repeats, and rests for one measure.
pub(super) fn render_measure_frame(
    page: &mut Scene,
    frame: &MeasureFrame<'_>,
) -> Result<(), RenderError> {
    draw_notation_lines(page, frame)?;
    draw_measure_labels(page, frame)?;
    draw_signatures(page, frame)?;
    draw_repeats_and_rest(page, frame)?;
    Ok(())
}

/// Draws notation lines and the barlines enclosing a measure.
fn draw_notation_lines(page: &mut Scene, frame: &MeasureFrame<'_>) -> Result<(), RenderError> {
    let MeasureFrame {
        track,
        measure: m,
        plan,
        index: mi,
        options,
        x,
        staff_y: y,
        tab_y: ty,
        bottom,
        tab,
        staff,
        clef: current_clef,
        ..
    } = *frame;
    // Emit the fixed notation grid before measure-local symbols and notes.
    if tab {
        for (s, string) in track.strings.iter().enumerate() {
            let sy = ty + s as f32 * options.string_spacing;
            page.line(x, sy, x + plan.width, sy, THIN_STROKE_WIDTH);
            if x == PAGE_LEFT_EDGE {
                page.text(
                    TAB_STRING_LABEL_X,
                    sy,
                    string,
                    TAB_STRING_LABEL_TEXT_SIZE,
                    false,
                );
            }
        }
    }
    if options.display == DisplayMode::Slash {
        page.line(
            x,
            y + STAFF_MIDDLE_LINE_OFFSET,
            x + plan.width,
            y + STAFF_MIDDLE_LINE_OFFSET,
            THIN_STROKE_WIDTH,
        );
    } else if staff {
        draw_staff(
            page,
            current_clef,
            m,
            x,
            y,
            plan.width,
            (
                x == PAGE_LEFT_EDGE
                    || mi == 0
                    || m.clef.is_some()
                    || track.measures[mi - 1].key_signature != m.key_signature,
                if mi > 0 && track.measures[mi - 1].key_signature != m.key_signature {
                    track.measures[mi - 1].key_signature
                } else {
                    KeySignature::Natural
                },
            ),
        )?;
    }
    let bar_bottom = if tab { bottom } else { y + STAFF_HEIGHT };
    page.line(x, y, x, bar_bottom, THIN_STROKE_WIDTH);
    page.line(
        x + plan.width,
        y,
        x + plan.width,
        bar_bottom,
        EMPHASIZED_STROKE_WIDTH,
    );
    if m.double_bar {
        page.line(
            x + plan.width - DOUBLE_BAR_GAP,
            y,
            x + plan.width - DOUBLE_BAR_GAP,
            bar_bottom,
            EMPHASIZED_STROKE_WIDTH,
        );
    }
    if mi + 1 == track.measures.len() {
        page.line(
            x + plan.width - DOUBLE_BAR_GAP,
            y,
            x + plan.width - DOUBLE_BAR_GAP,
            bar_bottom,
            FINAL_BAR_STROKE_WIDTH,
        );
    }
    Ok(())
}

/// Draws bar numbers, directions, navigation marks, and fermatas.
fn draw_measure_labels(page: &mut Scene, frame: &MeasureFrame<'_>) -> Result<(), RenderError> {
    let MeasureFrame {
        measure: m,
        plan,
        index: mi,
        options,
        x,
        staff_y: y,
        tab_y: ty,
        tab,
        tab_height,
        ..
    } = *frame;
    // Place metadata above the measure; annotations inside beats are handled later.
    if options.show_bar_numbers && options.elements.bar_numbers {
        page.text(
            x + BAR_NUMBER_OFFSET_X,
            y - BAR_NUMBER_OFFSET_Y,
            m.display_number.unwrap_or(mi + 1),
            BAR_NUMBER_TEXT_SIZE,
            false,
        );
    }
    if m.free_time {
        page.text(
            x + FREE_TIME_OFFSET_X,
            y - FREE_TIME_OFFSET_Y,
            "Free time",
            FREE_TIME_TEXT_SIZE,
            false,
        );
    }
    if let Some(feel) = &m.triplet_feel {
        page.text(
            x + plan.width / 2.0,
            y - TRIPLET_FEEL_OFFSET_Y,
            feel,
            TRIPLET_FEEL_TEXT_SIZE,
            false,
        );
    }
    if let Some(simile) = m.simile {
        let code = match simile {
            Simile::Single => G::Repeat1Bar,
            _ => G::Repeat2Bars,
        };
        if simile != Simile::DoubleFirst {
            page.glyph_at_center(
                x + plan.width / 2.0 - SIMILE_OFFSET_X,
                if tab {
                    ty + tab_height / 2.0
                } else {
                    y + STAFF_MIDDLE_LINE_OFFSET
                },
                code,
                SIMILE_GLYPH_SIZE,
            )?;
        }
    }
    if let Some(nav) = &m.navigation {
        match nav {
            Navigation::Instruction(text) => page.text(
                x + plan.width / 2.0,
                y - NAVIGATION_INSTRUCTION_OFFSET_Y,
                text,
                NAVIGATION_INSTRUCTION_TEXT_SIZE,
                false,
            ),
            _ => {
                let code = if matches!(nav, Navigation::Coda | Navigation::DoubleCoda) {
                    G::Coda
                } else {
                    G::Segno
                };
                page.glyph_at_center(
                    x + plan.width / 2.0,
                    y - NAVIGATION_SYMBOL_OFFSET_Y,
                    code,
                    NAVIGATION_SYMBOL_GLYPH_SIZE,
                )?;
                if matches!(nav, Navigation::DoubleCoda | Navigation::DoubleSegno) {
                    page.glyph_at_center(
                        x + plan.width / 2.0 + DOUBLE_NAVIGATION_SYMBOL_GAP,
                        y - NAVIGATION_SYMBOL_OFFSET_Y,
                        code,
                        NAVIGATION_SYMBOL_GLYPH_SIZE,
                    )?;
                }
            }
        }
    }
    for f in &m.fermatas {
        let ci = plan
            .columns
            .partition_point(|c| c.0 < f.start - TIMELINE_EPSILON)
            .min(plan.columns.len().saturating_sub(1));
        let fx = x
            + plan.header
            + plan.columns[..ci].iter().map(|c| c.1).sum::<f32>()
            + plan.columns.get(ci).map_or(0.0, |c| c.1 / 2.0);
        page.glyph_at_center(
            fx - FERMATA_OFFSET_X,
            y - FERMATA_OFFSET_Y,
            match f.kind {
                FermataKind::Normal => G::FermataAbove,
                FermataKind::Short => G::FermataShortAbove,
                FermataKind::Long => G::FermataLongAbove,
            },
            FERMATA_GLYPH_SIZE,
        )?;
    }
    if options.elements.repeat_counts {
        if let Some(count) = m.repeat_count {
            page.text(
                x + plan.width - REPEAT_COUNT_RIGHT_INSET,
                y - REPEAT_COUNT_OFFSET_Y,
                format!("{count}×"),
                REPEAT_COUNT_TEXT_SIZE,
                false,
            );
        }
    }
    Ok(())
}

/// Draws time and key information, tempo, markers, and alternate endings.
fn draw_signatures(page: &mut Scene, frame: &MeasureFrame<'_>) -> Result<(), RenderError> {
    let MeasureFrame {
        track,
        measure: m,
        plan,
        index: mi,
        options,
        x,
        staff_y: y,
        tab_y: ty,
        tab,
        staff,
        ..
    } = *frame;
    if mi == 0 || x == PAGE_LEFT_EDGE || track.measures[mi - 1].time_signature != m.time_signature {
        if tab || options.display == DisplayMode::Numbered {
            page.text(
                x + TAB_TIME_SIGNATURE_OFFSET_X,
                ty - TAB_TIME_SIGNATURE_OFFSET_Y,
                format!("{}/{}", m.time_signature.0, m.time_signature.1),
                TAB_TIME_SIGNATURE_TEXT_SIZE,
                false,
            );
        }
        if staff {
            page.text(
                x + plan.header - STAFF_TIME_SIGNATURE_OFFSET_X,
                y + STAFF_TIME_SIGNATURE_NUMERATOR_OFFSET_Y,
                m.time_signature.0,
                STAFF_TIME_SIGNATURE_TEXT_SIZE,
                true,
            );
            page.text(
                x + plan.header - STAFF_TIME_SIGNATURE_OFFSET_X,
                y + STAFF_TIME_SIGNATURE_DENOMINATOR_OFFSET_Y,
                m.time_signature.1,
                STAFF_TIME_SIGNATURE_TEXT_SIZE,
                true,
            );
        }
    }
    if options.display == DisplayMode::Numbered
        && (mi == 0 || track.measures[mi - 1].key_signature != m.key_signature)
    {
        let tonic = [
            "Cb", "Gb", "Db", "Ab", "Eb", "Bb", "F", "C", "G", "D", "A", "E", "B", "F#", "C#",
        ][(m.key_signature.signed_value() + 7) as usize];
        page.text(
            x + NUMBERED_KEY_OFFSET_X,
            y - NUMBERED_KEY_OFFSET_Y,
            format!("1 = {tonic}"),
            NUMBERED_KEY_TEXT_SIZE,
            false,
        );
    }
    if let Some(tempo) = m.tempo {
        page.glyph_at_center(
            x + TEMPO_NOTE_OFFSET_X,
            y - TEMPO_NOTE_OFFSET_Y,
            G::NoteQuarterUp,
            TEMPO_NOTE_GLYPH_SIZE,
        )?;
        page.text(
            x + TEMPO_TEXT_OFFSET_X,
            y - TEMPO_TEXT_OFFSET_Y,
            format!("= {tempo}"),
            TEMPO_TEXT_SIZE,
            false,
        );
    }
    if !m.marker.is_empty() {
        page.text(
            x + plan.width / 2.0,
            y - MARKER_OFFSET_Y,
            &m.marker,
            MARKER_TEXT_SIZE,
            false,
        );
    }
    if !m.alternate_endings.is_empty() {
        page.line(
            x,
            y - ALTERNATE_ENDING_LINE_OFFSET_Y,
            x + plan.width,
            y - ALTERNATE_ENDING_LINE_OFFSET_Y,
            THIN_STROKE_WIDTH,
        );
        page.line(
            x,
            y - ALTERNATE_ENDING_LINE_OFFSET_Y,
            x,
            y - ALTERNATE_ENDING_LINE_OFFSET_Y + ALTERNATE_ENDING_HOOK_HEIGHT,
            THIN_STROKE_WIDTH,
        );
        page.text(
            x + ALTERNATE_ENDING_LABEL_OFFSET_X,
            y - ALTERNATE_ENDING_LABEL_OFFSET_Y,
            m.alternate_endings
                .iter()
                .map(u8::to_string)
                .collect::<Vec<_>>()
                .join(","),
            ALTERNATE_ENDING_LABEL_TEXT_SIZE,
            false,
        );
    }
    Ok(())
}

/// Draws repeat barlines and the compact multi-measure rest symbol.
fn draw_repeats_and_rest(page: &mut Scene, frame: &MeasureFrame<'_>) -> Result<(), RenderError> {
    let MeasureFrame {
        measure: m,
        plan,
        x,
        staff_y: y,
        tab_y: ty,
        bottom,
        tab,
        staff,
        tab_height,
        ..
    } = *frame;
    let bar_bottom = if tab { bottom } else { y + STAFF_HEIGHT };
    for (repeat, bx, dot) in [
        (m.repeat_start, x + DOUBLE_BAR_GAP, x + BAR_NUMBER_OFFSET_X),
        (
            m.repeat_end,
            x + plan.width - DOUBLE_BAR_GAP,
            x + plan.width - REPEAT_END_DOT_OFFSET_X,
        ),
    ] {
        if repeat {
            page.line(bx, y, bx, bar_bottom, REPEAT_BAR_STROKE_WIDTH);
            for dy in [-REPEAT_DOT_OFFSET_Y, REPEAT_DOT_OFFSET_Y] {
                page.glyph_at_center(
                    dot,
                    (if tab {
                        ty + tab_height / 2.0
                    } else {
                        y + STAFF_MIDDLE_LINE_OFFSET
                    }) + dy,
                    G::AugmentationDot,
                    REPEAT_DOT_GLYPH_SIZE,
                )?;
            }
        }
    }
    if m.rest_count > 1 {
        let left = x + plan.header + MULTI_REST_LEFT_INSET;
        let right = x + plan.width - MULTI_REST_RIGHT_INSET;
        for yy in [
            staff.then_some(y + STAFF_MIDDLE_LINE_OFFSET),
            tab.then_some(ty + tab_height / 2.0),
        ]
        .into_iter()
        .flatten()
        {
            page.line(left, yy, right, yy, MULTI_REST_STROKE_WIDTH);
            page.line(
                left,
                yy - MULTI_REST_END_HEIGHT,
                left,
                yy + MULTI_REST_END_HEIGHT,
                EMPHASIZED_STROKE_WIDTH,
            );
            page.line(
                right,
                yy - MULTI_REST_END_HEIGHT,
                right,
                yy + MULTI_REST_END_HEIGHT,
                EMPHASIZED_STROKE_WIDTH,
            );
            page.text(
                (left + right) / 2.0,
                yy - MULTI_REST_COUNT_OFFSET_Y,
                m.rest_count,
                MULTI_REST_COUNT_TEXT_SIZE,
                true,
            );
        }
    }
    Ok(())
}
