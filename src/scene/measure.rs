//! Scene construction for measure lines, headers, repeats, and navigation marks.

use crate::{
    Clef, DisplayMode, FermataKind, KeySignature, Measure, Navigation, RenderError, Simile, Track,
};
use smufl::Glyph as G;

use super::parameters::{
    ANNOTATION_TEXT_SIZE, DOT_SPACING, EMPHASIZED_STROKE_WIDTH, EMPHASIZED_TEXT_SIZE,
    LARGE_TEXT_SIZE, NOTE_GLYPH_SIZE, PAGE_LAYOUT, SMALL_GLYPH_SIZE, SMALL_TEXT_SIZE, STAFF_HEIGHT,
    STAFF_LINE_SPACING, STAFF_MIDDLE_LINE_OFFSET, STANDARD_TEXT_SIZE, THIN_STROKE_WIDTH,
    TIMELINE_EPSILON,
};
use super::planning::MeasurePlan;
use super::staff::draw_staff;
use super::{Scene, SceneOptions};

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
            if x == PAGE_LAYOUT.left_edge {
                page.text(
                    STAFF_MIDDLE_LINE_OFFSET + DOT_SPACING,
                    sy,
                    string,
                    LARGE_TEXT_SIZE,
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
                x == PAGE_LAYOUT.left_edge
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
            x + plan.width - DOT_SPACING,
            y,
            x + plan.width - DOT_SPACING,
            bar_bottom,
            EMPHASIZED_STROKE_WIDTH,
        );
    }
    if mi + 1 == track.measures.len() {
        page.line(
            x + plan.width - DOT_SPACING,
            y,
            x + plan.width - DOT_SPACING,
            bar_bottom,
            EMPHASIZED_STROKE_WIDTH,
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
            x + STAFF_LINE_SPACING,
            y - STAFF_HEIGHT - DOT_SPACING,
            m.display_number.unwrap_or(mi + 1),
            ANNOTATION_TEXT_SIZE,
            false,
        );
    }
    if m.free_time {
        page.text(
            x + STAFF_HEIGHT + STAFF_LINE_SPACING,
            y - STAFF_HEIGHT * 2.0 + EMPHASIZED_TEXT_SIZE,
            "Free time",
            ANNOTATION_TEXT_SIZE,
            false,
        );
    }
    if let Some(feel) = &m.triplet_feel {
        page.text(
            x + plan.width / 2.0,
            y - STAFF_HEIGHT * 2.0 + STAFF_HEIGHT - STAFF_LINE_SPACING,
            feel,
            ANNOTATION_TEXT_SIZE,
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
                x + plan.width / 2.0 - STAFF_LINE_SPACING,
                if tab {
                    ty + tab_height / 2.0
                } else {
                    y + STAFF_MIDDLE_LINE_OFFSET
                },
                code,
                EMPHASIZED_TEXT_SIZE,
            )?;
        }
    }
    if let Some(nav) = &m.navigation {
        match nav {
            Navigation::Instruction(text) => page.text(
                x + plan.width / 2.0,
                y - STAFF_HEIGHT * 2.0,
                text,
                EMPHASIZED_TEXT_SIZE,
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
                    y - STAFF_HEIGHT + STAFF_MIDDLE_LINE_OFFSET + NOTE_GLYPH_SIZE,
                    code,
                    STANDARD_TEXT_SIZE,
                )?;
                if matches!(nav, Navigation::DoubleCoda | Navigation::DoubleSegno) {
                    page.glyph_at_center(
                        x + plan.width / 2.0 + STAFF_MIDDLE_LINE_OFFSET,
                        y - STAFF_HEIGHT + STAFF_MIDDLE_LINE_OFFSET + NOTE_GLYPH_SIZE,
                        code,
                        STANDARD_TEXT_SIZE,
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
            fx - SMALL_GLYPH_SIZE,
            y - EMPHASIZED_TEXT_SIZE,
            match f.kind {
                FermataKind::Normal => G::FermataAbove,
                FermataKind::Short => G::FermataShortAbove,
                FermataKind::Long => G::FermataLongAbove,
            },
            SMALL_TEXT_SIZE,
        )?;
    }
    if options.elements.repeat_counts {
        if let Some(count) = m.repeat_count {
            page.text(
                x + plan.width - STAFF_MIDDLE_LINE_OFFSET + DOT_SPACING,
                y - STAFF_HEIGHT - DOT_SPACING,
                format!("{count}×"),
                ANNOTATION_TEXT_SIZE,
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
    if mi == 0
        || x == PAGE_LAYOUT.left_edge
        || track.measures[mi - 1].time_signature != m.time_signature
    {
        if tab || options.display == DisplayMode::Numbered {
            page.text(
                x + STAFF_HEIGHT - THIN_STROKE_WIDTH - THIN_STROKE_WIDTH,
                ty - STAFF_MIDDLE_LINE_OFFSET,
                format!("{}/{}", m.time_signature.0, m.time_signature.1),
                EMPHASIZED_TEXT_SIZE,
                false,
            );
        }
        if staff {
            page.text(
                x + plan.header
                    - STAFF_MIDDLE_LINE_OFFSET
                    - THIN_STROKE_WIDTH
                    - THIN_STROKE_WIDTH
                    - THIN_STROKE_WIDTH,
                y + EMPHASIZED_TEXT_SIZE,
                m.time_signature.0,
                LARGE_TEXT_SIZE,
                true,
            );
            page.text(
                x + plan.header
                    - STAFF_MIDDLE_LINE_OFFSET
                    - THIN_STROKE_WIDTH
                    - THIN_STROKE_WIDTH
                    - THIN_STROKE_WIDTH,
                y + STAFF_HEIGHT - STAFF_LINE_SPACING,
                m.time_signature.1,
                LARGE_TEXT_SIZE,
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
            x + STAFF_HEIGHT + THIN_STROKE_WIDTH + THIN_STROKE_WIDTH,
            y - STAFF_HEIGHT + THIN_STROKE_WIDTH + THIN_STROKE_WIDTH,
            format!("1 = {tonic}"),
            ANNOTATION_TEXT_SIZE,
            false,
        );
    }
    if let Some(tempo) = m.tempo {
        page.glyph_at_center(
            x + STAFF_HEIGHT + STAFF_MIDDLE_LINE_OFFSET,
            y - STAFF_HEIGHT + DOT_SPACING,
            G::NoteQuarterUp,
            SMALL_GLYPH_SIZE,
        )?;
        page.text(
            x + STAFF_HEIGHT * 2.0 + EMPHASIZED_TEXT_SIZE,
            y - STAFF_HEIGHT - SMALL_GLYPH_SIZE,
            format!("= {tempo}"),
            ANNOTATION_TEXT_SIZE,
            false,
        );
    }
    if !m.marker.is_empty() {
        page.text(
            x + plan.width / 2.0,
            y - STAFF_HEIGHT + STAFF_MIDDLE_LINE_OFFSET + SMALL_TEXT_SIZE,
            &m.marker,
            LARGE_TEXT_SIZE,
            false,
        );
    }
    if !m.alternate_endings.is_empty() {
        page.line(
            x,
            y - EMPHASIZED_TEXT_SIZE,
            x + plan.width,
            y - EMPHASIZED_TEXT_SIZE,
            THIN_STROKE_WIDTH,
        );
        page.line(
            x,
            y - EMPHASIZED_TEXT_SIZE,
            x,
            y - EMPHASIZED_TEXT_SIZE + NOTE_GLYPH_SIZE,
            THIN_STROKE_WIDTH,
        );
        page.text(
            x + STAFF_MIDDLE_LINE_OFFSET + DOT_SPACING,
            y - STAFF_MIDDLE_LINE_OFFSET + DOT_SPACING,
            m.alternate_endings
                .iter()
                .map(u8::to_string)
                .collect::<Vec<_>>()
                .join(","),
            ANNOTATION_TEXT_SIZE,
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
        (m.repeat_start, x + DOT_SPACING, x + STAFF_LINE_SPACING),
        (
            m.repeat_end,
            x + plan.width - DOT_SPACING,
            x + plan.width - EMPHASIZED_TEXT_SIZE,
        ),
    ] {
        if repeat {
            page.line(bx, y, bx, bar_bottom, EMPHASIZED_STROKE_WIDTH);
            for dy in [-DOT_SPACING, DOT_SPACING] {
                page.glyph_at_center(
                    dot,
                    (if tab {
                        ty + tab_height / 2.0
                    } else {
                        y + STAFF_MIDDLE_LINE_OFFSET
                    }) + dy,
                    G::AugmentationDot,
                    SMALL_GLYPH_SIZE,
                )?;
            }
        }
    }
    if m.rest_count > 1 {
        let left = x + plan.header + NOTE_GLYPH_SIZE;
        let right = x + plan.width - STAFF_MIDDLE_LINE_OFFSET + DOT_SPACING;
        for yy in [
            staff.then_some(y + STAFF_MIDDLE_LINE_OFFSET),
            tab.then_some(ty + tab_height / 2.0),
        ]
        .into_iter()
        .flatten()
        {
            page.line(left, yy, right, yy, DOT_SPACING);
            page.line(
                left,
                yy - SMALL_TEXT_SIZE,
                left,
                yy + SMALL_TEXT_SIZE,
                EMPHASIZED_STROKE_WIDTH,
            );
            page.line(
                right,
                yy - SMALL_TEXT_SIZE,
                right,
                yy + SMALL_TEXT_SIZE,
                EMPHASIZED_STROKE_WIDTH,
            );
            page.text(
                (left + right) / 2.0,
                yy - STAFF_MIDDLE_LINE_OFFSET + DOT_SPACING,
                m.rest_count,
                LARGE_TEXT_SIZE,
                true,
            );
        }
    }
    Ok(())
}
