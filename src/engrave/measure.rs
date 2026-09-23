//! Rendering of measure lines, headers, repeats, and navigation marks.

use super::*;

/// Shared model and geometry used while drawing one measure frame.
pub(super) struct MeasureFrame<'a> {
    pub(super) track: &'a Track,
    pub(super) measure: &'a Measure,
    pub(super) plan: &'a MeasurePlan,
    pub(super) index: usize,
    pub(super) options: LayoutOptions,
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
    page: &mut Layout,
    frame: &MeasureFrame<'_>,
) -> Result<(), RenderError> {
    draw_notation_lines(page, frame)?;
    draw_measure_labels(page, frame)?;
    draw_signatures(page, frame)?;
    draw_repeats_and_rest(page, frame)?;
    Ok(())
}

/// Draws notation lines and the barlines enclosing a measure.
fn draw_notation_lines(page: &mut Layout, frame: &MeasureFrame<'_>) -> Result<(), RenderError> {
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
    if tab {
        for (s, string) in track.strings.iter().enumerate() {
            let sy = ty + s as f32 * options.string_spacing;
            page.line(x, sy, x + plan.width, sy, 1.0);
            if x == 44.0 {
                page.text(22.0, sy, string, 13.0, false);
            }
        }
    }
    if options.display == DisplayMode::Slash {
        page.line(x, y + 20.0, x + plan.width, y + 20.0, 1.0);
    } else if staff {
        draw_staff(
            page,
            current_clef,
            m,
            x,
            y,
            plan.width,
            (
                x == 44.0
                    || mi == 0
                    || m.clef.is_some()
                    || track.measures[mi - 1].key_signature != m.key_signature,
                if mi > 0 && track.measures[mi - 1].key_signature != m.key_signature {
                    track.measures[mi - 1].key_signature
                } else {
                    0
                },
            ),
        )?;
    }
    let bar_bottom = if tab { bottom } else { y + 40.0 };
    page.line(x, y, x, bar_bottom, 1.0);
    page.line(x + plan.width, y, x + plan.width, bar_bottom, 1.2);
    if m.double_bar {
        page.line(
            x + plan.width - 4.0,
            y,
            x + plan.width - 4.0,
            bar_bottom,
            1.2,
        );
    }
    if mi + 1 == track.measures.len() {
        page.line(
            x + plan.width - 4.0,
            y,
            x + plan.width - 4.0,
            bar_bottom,
            3.0,
        );
    }
    Ok(())
}

/// Draws bar numbers, directions, navigation marks, and fermatas.
fn draw_measure_labels(page: &mut Layout, frame: &MeasureFrame<'_>) -> Result<(), RenderError> {
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
    if options.show_bar_numbers && options.elements.bar_numbers {
        page.text(
            x + 10.0,
            y - 36.0,
            m.display_number.unwrap_or(mi + 1),
            11.0,
            false,
        );
    }
    if m.free_time {
        page.text(x + 50.0, y - 95.0, "Free time", 11.0, false);
    }
    if let Some(feel) = &m.triplet_feel {
        page.text(x + plan.width / 2.0, y - 110.0, feel, 11.0, false);
    }
    if let Some(simile) = m.simile {
        let code = match simile {
            Simile::Single => G::Repeat1Bar,
            _ => G::Repeat2Bars,
        };
        if simile != Simile::DoubleFirst {
            page.glyph_at_center(
                x + plan.width / 2.0 - 10.0,
                if tab { ty + tab_height / 2.0 } else { y + 20.0 },
                code,
                12.0,
            )?;
        }
    }
    if let Some(nav) = &m.navigation {
        match nav {
            Navigation::Instruction(text) => {
                page.text(x + plan.width / 2.0, y - 78.0, text, 12.0, false)
            }
            _ => {
                let code = if matches!(nav, Navigation::Coda | Navigation::DoubleCoda) {
                    G::Coda
                } else {
                    G::Segno
                };
                page.glyph_at_center(x + plan.width / 2.0, y - 68.0, code, 10.0)?;
                if matches!(nav, Navigation::DoubleCoda | Navigation::DoubleSegno) {
                    page.glyph_at_center(x + plan.width / 2.0 + 20.0, y - 68.0, code, 10.0)?;
                }
            }
        }
    }
    for f in &m.fermatas {
        let ci = plan
            .columns
            .partition_point(|c| c.0 < f.start - 1e-8)
            .min(plan.columns.len().saturating_sub(1));
        let fx = x
            + plan.header
            + plan.columns[..ci].iter().map(|c| c.1).sum::<f32>()
            + plan.columns.get(ci).map_or(0.0, |c| c.1 / 2.0);
        page.glyph_at_center(
            fx - 6.0,
            y - 12.0,
            match f.kind {
                FermataKind::Normal => G::FermataAbove,
                FermataKind::Short => G::FermataShortAbove,
                FermataKind::Long => G::FermataLongAbove,
            },
            9.0,
        )?;
    }
    if options.elements.repeat_counts {
        if let Some(count) = m.repeat_count {
            page.text(
                x + plan.width - 22.0,
                y - 35.0,
                format!("{count}×"),
                11.0,
                false,
            );
        }
    }
    Ok(())
}

/// Draws time and key information, tempo, markers, and alternate endings.
fn draw_signatures(page: &mut Layout, frame: &MeasureFrame<'_>) -> Result<(), RenderError> {
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
    if mi == 0 || x == 44.0 || track.measures[mi - 1].time_signature != m.time_signature {
        if tab || options.display == DisplayMode::Numbered {
            page.text(
                x + 38.0,
                ty - 20.0,
                format!("{}/{}", m.time_signature.0, m.time_signature.1),
                12.0,
                false,
            );
        }
        if staff {
            page.text(
                x + plan.header - 17.0,
                y + 12.0,
                m.time_signature.0,
                15.0,
                true,
            );
            page.text(
                x + plan.header - 17.0,
                y + 30.0,
                m.time_signature.1,
                15.0,
                true,
            );
        }
    }
    if options.display == DisplayMode::Numbered
        && (mi == 0 || track.measures[mi - 1].key_signature != m.key_signature)
    {
        let tonic = [
            "Cb", "Gb", "Db", "Ab", "Eb", "Bb", "F", "C", "G", "D", "A", "E", "B", "F#", "C#",
        ][(m.key_signature + 7) as usize];
        page.text(x + 42.0, y - 42.0, format!("1 = {tonic}"), 11.0, false);
    }
    if let Some(tempo) = m.tempo {
        page.glyph_at_center(x + 60.0, y - 36.0, G::NoteQuarterUp, 6.0)?;
        page.text(x + 92.0, y - 43.0, format!("= {tempo}"), 11.0, false);
    }
    if !m.marker.is_empty() {
        page.text(x + plan.width / 2.0, y - 65.0, &m.marker, 13.0, false);
    }
    if !m.alternate_endings.is_empty() {
        page.line(x, y - 12.0, x + plan.width, y - 12.0, 1.0);
        page.line(x, y - 12.0, x, y - 4.0, 1.0);
        page.text(
            x + 25.0,
            y - 22.0,
            m.alternate_endings
                .iter()
                .map(u8::to_string)
                .collect::<Vec<_>>()
                .join(","),
            11.0,
            false,
        );
    }
    Ok(())
}

/// Draws repeat barlines and the compact multi-measure rest symbol.
fn draw_repeats_and_rest(page: &mut Layout, frame: &MeasureFrame<'_>) -> Result<(), RenderError> {
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
    let bar_bottom = if tab { bottom } else { y + 40.0 };
    for (repeat, bx, dot) in [
        (m.repeat_start, x + 4.0, x + 10.0),
        (m.repeat_end, x + plan.width - 4.0, x + plan.width - 12.0),
    ] {
        if repeat {
            page.line(bx, y, bx, bar_bottom, 2.5);
            for dy in [-5.0, 5.0] {
                page.glyph_at_center(
                    dot,
                    (if tab { ty + tab_height / 2.0 } else { y + 20.0 }) + dy,
                    G::AugmentationDot,
                    7.0,
                )?;
            }
        }
    }
    if m.rest_count > 1 {
        let left = x + plan.header + 8.0;
        let right = x + plan.width - 22.0;
        for yy in [
            staff.then_some(y + 20.0),
            tab.then_some(ty + tab_height / 2.0),
        ]
        .into_iter()
        .flatten()
        {
            page.line(left, yy, right, yy, 5.0);
            page.line(left, yy - 9.0, left, yy + 9.0, 1.2);
            page.line(right, yy - 9.0, right, yy + 9.0, 1.2);
            page.text((left + right) / 2.0, yy - 24.0, m.rest_count, 13.0, true);
        }
    }
    Ok(())
}
