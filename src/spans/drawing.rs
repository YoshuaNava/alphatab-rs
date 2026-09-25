//! Span routing and drawing after beat geometry is known.
use super::{anchor, contains, RenderState};
use crate::scene::parameters::{
    DOT_SPACING, EMPHASIZED_STROKE_WIDTH, EMPHASIZED_TEXT_SIZE, LARGE_TEXT_SIZE, NOTE_GLYPH_SIZE,
    SMALL_GLYPH_SIZE, SMALL_TEXT_SIZE, STAFF_HEIGHT, STAFF_LINE_SPACING, STAFF_MIDDLE_LINE_OFFSET,
    STANDARD_TEXT_SIZE, SYSTEM_LAYOUT, THIN_STROKE_WIDTH,
};
use crate::{
    BeatAddress, BeatBounds, Clef, DisplayMode, Placement, Primitive, RenderError, Scene,
    SceneOptions, SpanKind, StemDirection, TabRhythm, Track,
};

pub(crate) fn draw(
    page: &mut Scene,
    track: &Track,
    render: &RenderState,
    options: SceneOptions,
    owners: &mut Vec<usize>,
) -> Result<(), RenderError> {
    let staff = options.display.renders_staff()
        || options.display == DisplayMode::Numbered
        || track.clef == Clef::Percussion;
    let tab = options.display.renders_tab() && track.clef != Clef::Percussion;
    for span in &render.spans(track, options) {
        let selected: Vec<_> = page
            .beats
            .iter()
            .filter(|b| {
                contains(
                    span,
                    BeatAddress {
                        measure: b.measure,
                        voice: b.voice,
                        beat: b.beat,
                    },
                )
            })
            .cloned()
            .collect();
        for (si, system) in page.systems.clone().into_iter().enumerate() {
            let part: Vec<_> = selected
                .iter()
                .filter(|b| b.cursor_rect[1] >= system[0] && b.cursor_rect[1] < system[1])
                .collect();
            let (Some(first), Some(last)) = (part.first(), part.last()) else {
                continue;
            };
            let starts = (first.measure, first.beat) == (span.start.measure, span.start.beat);
            let ends = (last.measure, last.beat) == (span.end.measure, span.end.beat);
            if matches!(span.kind, SpanKind::Beam) {
                for is_staff in [true, false].into_iter().filter(|s| {
                    if *s {
                        staff
                    } else {
                        tab && options.tab_rhythm != TabRhythm::Hidden
                    }
                }) {
                    draw_beam(page, track, &part, is_staff, options)?;
                }
                owners.resize(page.primitives.len(), si);
                continue;
            }
            let above = span.placement == Placement::Above;
            let sign = if above { -1.0 } else { 1.0 };
            let slur = match &span.kind {
                SpanKind::Slur {
                    start_note,
                    end_note,
                } => Some((*start_note, *end_note, None)),
                SpanKind::Legato {
                    start_note,
                    end_note,
                    text,
                } => Some((*start_note, *end_note, Some(text.as_str()))),
                _ => None,
            };
            if let Some((start_note, end_note, label)) = slur {
                for is_staff in [true, false]
                    .into_iter()
                    .filter(|s| if *s { staff } else { tab })
                {
                    let mut from = anchor(
                        track,
                        first,
                        if starts { start_note } else { None },
                        is_staff,
                        above,
                        options,
                    );
                    let mut to = anchor(
                        track,
                        last,
                        if ends { end_note } else { None },
                        is_staff,
                        above,
                        options,
                    );
                    from[0] = if starts {
                        from[0] + SMALL_GLYPH_SIZE
                    } else {
                        STAFF_HEIGHT + SMALL_GLYPH_SIZE - STAFF_MIDDLE_LINE_OFFSET
                    };
                    to[0] = if ends {
                        to[0] - SMALL_GLYPH_SIZE
                    } else {
                        page.width - STAFF_MIDDLE_LINE_OFFSET + SMALL_GLYPH_SIZE
                    };
                    if to[0] <= from[0] {
                        to[0] = from[0] + LARGE_TEXT_SIZE;
                    }
                    from[1] += sign * NOTE_GLYPH_SIZE;
                    to[1] += sign * NOTE_GLYPH_SIZE;
                    let mut arch = options.engraving.slur_height;
                    // Solve the parabola's required height at each intervening obstacle.
                    for p in &page.primitives {
                        let r = p.compute_bounds();
                        let xx = ((r[0] + r[2]) / 2.0).clamp(from[0], to[0]);
                        let t = (xx - from[0]) / (to[0] - from[0]);
                        let staff_top = if is_staff {
                            first.cursor_rect[1] + NOTE_GLYPH_SIZE
                        } else {
                            first.cursor_rect[3]
                                - NOTE_GLYPH_SIZE
                                - track.strings.len().saturating_sub(1) as f32
                                    * options.string_spacing
                        };
                        let staff_bottom = if is_staff {
                            staff_top + STAFF_HEIGHT
                        } else {
                            first.cursor_rect[3] - NOTE_GLYPH_SIZE
                        };
                        if !((THIN_STROKE_WIDTH / EMPHASIZED_TEXT_SIZE)
                            ..=(1.0 - THIN_STROKE_WIDTH / EMPHASIZED_TEXT_SIZE))
                            .contains(&t)
                            || r[3] < staff_top - STAFF_HEIGHT + DOT_SPACING
                            || r[1] > staff_bottom + STAFF_MIDDLE_LINE_OFFSET
                        {
                            continue;
                        }
                        if matches!(p, Primitive::Line { from,to,width, .. } if *width <= 1.2 && ((from[1]-to[1]).abs()<0.1 && (from[0]-to[0]).abs()>100.0 || (from[0]-to[0]).abs()<0.1 && (from[1]-to[1]).abs()>45.0))
                        {
                            continue;
                        }
                        let base = from[1] + t * (to[1] - from[1]);
                        let edge = if above {
                            r[1] - DOT_SPACING
                        } else {
                            r[3] + DOT_SPACING
                        };
                        arch = arch.max(
                            ((edge - base) * sign)
                                / ((STAFF_HEIGHT / STAFF_LINE_SPACING) * t * (1.0 - t)),
                        );
                    }
                    page.curve(from, to, sign * arch);
                    if starts {
                        if let Some(label) = label {
                            page.text(
                                (from[0] + to[0]) / 2.0,
                                (from[1] + to[1]) / 2.0 + sign * (arch + NOTE_GLYPH_SIZE),
                                label,
                                SMALL_TEXT_SIZE,
                                false,
                            );
                        }
                    }
                }
                owners.resize(page.primitives.len(), si);
                continue;
            }
            let mut left = if starts {
                first.cursor_rect[0] + SMALL_GLYPH_SIZE
            } else {
                STAFF_HEIGHT + SMALL_GLYPH_SIZE - STAFF_MIDDLE_LINE_OFFSET
            };
            let mut right = if ends {
                last.cursor_rect[2] - SMALL_GLYPH_SIZE
            } else {
                page.width - STAFF_MIDDLE_LINE_OFFSET + SMALL_GLYPH_SIZE
            };
            let mut yy = if above {
                first.cursor_rect[1] - STAFF_HEIGHT
            } else {
                first.rect[3] + STAFF_MIDDLE_LINE_OFFSET + SMALL_GLYPH_SIZE
            };
            if !above && matches!(span.kind, SpanKind::Crescendo | SpanKind::Diminuendo) {
                yy = first.rect[1] + STAFF_HEIGHT;
                if let Some(d) = &track.measures[first.measure].voices[first.voice][first.beat]
                    .annotations
                    .dynamic
                {
                    left = (first.cursor_rect[0] + first.cursor_rect[2]) / 2.0
                        + crate::text::width(d, LARGE_TEXT_SIZE) / 2.0
                        + STAFF_LINE_SPACING;
                }
                if part.len() > 1 {
                    if let Some(d) = &track.measures[last.measure].voices[last.voice][last.beat]
                        .annotations
                        .dynamic
                    {
                        right = (last.cursor_rect[0] + last.cursor_rect[2]) / 2.0
                            - crate::text::width(d, LARGE_TEXT_SIZE) / 2.0
                            - STAFF_LINE_SPACING;
                    }
                }
            }
            // Place the entire effect band in a free lane, keeping its label and line together.
            loop {
                let collision = page.primitives.iter().any(|p| {
                    let r = p.compute_bounds();
                    r[0] < right
                        && r[2] > left
                        && r[1] < yy + SMALL_TEXT_SIZE
                        && r[3] > yy - SMALL_TEXT_SIZE
                });
                if !collision {
                    break;
                }
                yy += sign * STAFF_MIDDLE_LINE_OFFSET;
            }
            match &span.kind {
                SpanKind::Crescendo | SpanKind::Diminuendo => {
                    let growing = matches!(span.kind, SpanKind::Crescendo);
                    let total = selected.len().max(1) as f32;
                    let offset = selected
                        .iter()
                        .position(|b| b.measure == first.measure && b.beat == first.beat)
                        .unwrap() as f32;
                    let a = if growing {
                        offset / total
                    } else {
                        1.0 - offset / total
                    } * DOT_SPACING;
                    let b = if growing {
                        (offset + part.len() as f32) / total
                    } else {
                        1.0 - (offset + part.len() as f32) / total
                    } * DOT_SPACING;
                    page.line(left, yy - a, right, yy - b, THIN_STROKE_WIDTH);
                    page.line(left, yy + a, right, yy + b, THIN_STROKE_WIDTH);
                }
                SpanKind::Vibrato | SpanKind::Trill => {
                    let start = if matches!(span.kind, SpanKind::Trill) {
                        page.glyph_at_origin(
                            left,
                            yy,
                            smufl::Glyph::OrnamentTrill,
                            NOTE_GLYPH_SIZE,
                        )?;
                        left + STAFF_MIDDLE_LINE_OFFSET
                    } else {
                        left
                    };
                    let mut x = start;
                    while x + SMALL_GLYPH_SIZE <= right {
                        page.line(
                            x,
                            yy,
                            x + (SMALL_GLYPH_SIZE / 2.0),
                            yy - (SMALL_GLYPH_SIZE / 2.0),
                            THIN_STROKE_WIDTH,
                        );
                        page.line(
                            x + (SMALL_GLYPH_SIZE / 2.0),
                            yy - (SMALL_GLYPH_SIZE / 2.0),
                            x + SMALL_GLYPH_SIZE,
                            yy,
                            THIN_STROKE_WIDTH,
                        );
                        x += SMALL_GLYPH_SIZE;
                    }
                }
                kind => {
                    let text = match kind {
                        SpanKind::Ottava(o) => o.format_label(),
                        SpanKind::PalmMute => "P.M.",
                        SpanKind::LetRing => "let ring",
                        SpanKind::Pedal => "Ped.",
                        SpanKind::Rasgueado => "Rasg.",
                        SpanKind::Text(t) => t,
                        _ => unreachable!(),
                    };
                    let label = if starts {
                        text.to_string()
                    } else {
                        format!("({text})")
                    };
                    let w = crate::text::width(&label, STANDARD_TEXT_SIZE);
                    page.text(left + w / 2.0, yy, label, STANDARD_TEXT_SIZE, false);
                    let mut x = left + w + DOT_SPACING;
                    while x < right {
                        page.line(
                            x,
                            yy,
                            (x + (DOT_SPACING - THIN_STROKE_WIDTH)).min(right),
                            yy,
                            THIN_STROKE_WIDTH,
                        );
                        x += SMALL_GLYPH_SIZE;
                    }
                    if ends {
                        page.line(
                            right,
                            yy,
                            right,
                            yy - sign * SMALL_GLYPH_SIZE,
                            THIN_STROKE_WIDTH,
                        );
                    }
                }
            }
            owners.resize(page.primitives.len(), si);
        }
    }
    pack_systems(page, owners);
    Ok(())
}

/// Expand systems to contain routed curves and effects; shift every hit region with them.
fn pack_systems(page: &mut Scene, owners: &[usize]) {
    if page.systems.is_empty() {
        return;
    }
    let original = page.systems.clone();
    let mut extents = original.clone();
    for (p, &si) in page.primitives.iter().zip(owners) {
        let r = p.compute_bounds();
        extents[si][0] = extents[si][0].min(r[1] - NOTE_GLYPH_SIZE);
        extents[si][1] = extents[si][1].max(r[3] + NOTE_GLYPH_SIZE);
    }
    let mut shifts = Vec::new();
    let mut y = 0.0;
    for (si, range) in extents.iter().enumerate() {
        shifts.push(y - range[0]);
        page.systems[si] = [y, y + range[1] - range[0]];
        y = page.systems[si][1];
    }
    for (p, &si) in page.primitives.iter_mut().zip(owners) {
        match p {
            Primitive::Line { from, to, .. } => {
                from[1] += shifts[si];
                to[1] += shifts[si];
            }
            Primitive::Curve { points, .. } => {
                for point in points {
                    point[1] += shifts[si];
                }
            }
            Primitive::Text { at, .. } | Primitive::Glyph { at, .. } => at[1] += shifts[si],
        }
    }
    for b in &mut page.beats {
        let si = original
            .iter()
            .position(|r| b.cursor_rect[1] >= r[0] && b.cursor_rect[1] < r[1])
            .expect("beat belongs to a system");
        for r in [&mut b.rect, &mut b.cursor_rect] {
            r[1] += shifts[si];
            r[3] += shifts[si];
        }
    }
    page.height = y;
}

fn draw_beam(
    page: &mut Scene,
    track: &Track,
    part: &[&BeatBounds],
    staff: bool,
    options: SceneOptions,
) -> Result<(), RenderError> {
    let first = part[0];
    if options.display == DisplayMode::Numbered {
        for (i, bounds) in part.iter().enumerate() {
            let b = &track.measures[bounds.measure].voices[bounds.voice][bounds.beat];
            let x = (bounds.cursor_rect[0] + bounds.cursor_rect[2]) / 2.0;
            for level in 0..b.duration.beam_level_count() {
                let end = part
                    .get(i + 1)
                    .filter(|n| {
                        track.measures[n.measure].voices[n.voice][n.beat]
                            .duration
                            .beam_level_count()
                            > level
                    })
                    .map_or(x + NOTE_GLYPH_SIZE, |n| {
                        (n.cursor_rect[0] + n.cursor_rect[2]) / 2.0
                    });
                let yy = bounds.cursor_rect[1]
                    + STAFF_HEIGHT
                    + SMALL_GLYPH_SIZE
                    + bounds.voice as f32 * SYSTEM_LAYOUT.numbered_voice_spacing
                    + level as f32 * (DOT_SPACING - THIN_STROKE_WIDTH);
                page.line(x - NOTE_GLYPH_SIZE, yy, end, yy, EMPHASIZED_STROKE_WIDTH);
            }
        }
        return Ok(());
    }
    let defaults = &crate::music_font::metadata().engraving_defaults;
    let stem_width = crate::music_font::thickness(
        defaults.stem_thickness,
        STAFF_LINE_SPACING,
        THIN_STROKE_WIDTH,
    );
    let beam_width = crate::music_font::thickness(
        defaults.beam_thickness,
        STAFF_LINE_SPACING,
        SMALL_GLYPH_SIZE / 2.0,
    );
    let b = &track.measures[first.measure].voices[first.voice][first.beat];
    let down = !staff
        || b.annotations.stem == StemDirection::Down
        || (b.annotations.stem == StemDirection::Auto && first.voice % 2 == 1);
    let sign = if down { 1.0 } else { -1.0 };
    let end = if staff {
        part.iter()
            .map(|b| anchor(track, b, None, true, !down, options)[1])
            .reduce(if down { f32::max } else { f32::min })
            .unwrap()
            + sign * STAFF_HEIGHT
    } else {
        first.rect[1] + STAFF_HEIGHT
    };
    for (i, bounds) in part.iter().enumerate() {
        let b = &track.measures[bounds.measure].voices[bounds.voice][bounds.beat];
        let a = anchor(track, bounds, None, staff, down, options);
        let x = a[0]
            + if staff {
                if down {
                    -DOT_SPACING
                } else {
                    DOT_SPACING
                }
            } else {
                0.0
            };
        page.line(
            x,
            if staff {
                a[1]
            } else {
                bounds.rect[1] + DOT_SPACING
            },
            x,
            end,
            stem_width,
        );
        let levels = b.duration.beam_level_count();
        if part.len() == 1 {
            page.glyph_at_origin(
                x,
                end,
                crate::scene::select_flag_glyph(levels, down),
                NOTE_GLYPH_SIZE,
            )?;
        }
        for level in 0..levels {
            let yy = end - sign * level as f32 * DOT_SPACING;
            let right = part.get(i + 1).filter(|next| {
                track.measures[next.measure].voices[next.voice][next.beat]
                    .duration
                    .beam_level_count()
                    > level
            });
            if let Some(next) = right {
                page.line(
                    x,
                    yy,
                    (next.cursor_rect[0] + next.cursor_rect[2]) / 2.0
                        + if staff {
                            if down {
                                -DOT_SPACING
                            } else {
                                DOT_SPACING
                            }
                        } else {
                            0.0
                        },
                    yy,
                    beam_width,
                );
            } else if part.len() > 1
                && (i == 0
                    || track.measures[part[i - 1].measure].voices[part[i - 1].voice]
                        [part[i - 1].beat]
                        .duration
                        .beam_level_count()
                        <= level)
            {
                page.line(
                    x,
                    yy,
                    x + if i + 1 < part.len() {
                        SMALL_TEXT_SIZE
                    } else {
                        -SMALL_TEXT_SIZE
                    },
                    yy,
                    beam_width,
                );
            }
        }
    }
    Ok(())
}
