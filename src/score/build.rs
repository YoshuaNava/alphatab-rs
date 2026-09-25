//! Builds coordinated scenes for synchronized scores.

use crate::scene::build::{engrave_planned_scene, validate};
use crate::scene::planning::MeasurePlan;
use crate::*;

/// Renders synchronized tracks with shared onset columns and system breaks.
pub fn layout_score(
    tracks: &[Track],
    options: LayoutOptions,
) -> Result<crate::ScoreLayout, RenderError> {
    if tracks.is_empty() {
        return Err(RenderError::invalid_input(
            "a score needs at least one track".into(),
        ));
    }
    let count = tracks[0].measures.len();
    if tracks.iter().any(|t| t.measures.len() != count) {
        return Err(RenderError::invalid_input(
            "score tracks need matching measure counts".into(),
        ));
    }
    for mi in 0..count {
        if tracks
            .iter()
            .any(|t| t.measures[mi].time_signature != tracks[0].measures[mi].time_signature)
        {
            return Err(RenderError::invalid_input(
                "score tracks need matching time signatures".into(),
            ));
        }
    }

    // Rest condensation has a score-specific implementation and therefore runs
    // before adapting the homogeneous API to the canonical per-track pipeline.
    if options.multi_measure_rests {
        for track in tracks {
            crate::spans::prepare(track, options)?;
        }
        return crate::rests::score(tracks, options);
    }

    let score_tracks = tracks
        .iter()
        .cloned()
        .map(|track| crate::ScoreTrack { track, options })
        .collect::<Vec<_>>();
    layout_score_tracks(&score_tracks)
}

/// Lay out a score with per-track notation settings.
///
/// Every track still shares one system grid, so width, spacing, flow, and system
/// limits must agree. Display mode and visibility settings may differ.
pub fn layout_score_tracks(
    tracks: &[crate::ScoreTrack],
) -> Result<crate::ScoreLayout, RenderError> {
    if tracks.is_empty() {
        return Err(RenderError::invalid_input(
            "a score needs at least one track".into(),
        ));
    }
    let base = tracks[0].options;
    if tracks.iter().any(|t| {
        let o = t.options;
        o.width != base.width
            || o.string_spacing != base.string_spacing
            || o.beat_spacing != base.beat_spacing
            || o.flow != base.flow
            || o.justify != base.justify
            || o.strict_width != base.strict_width
            || o.bars_per_system != base.bars_per_system
            || o.multi_measure_rests != base.multi_measure_rests
    }) {
        return Err(RenderError::invalid_input(
            "score tracks need matching geometry options".into(),
        ));
    }
    if tracks.iter().any(|t| t.options.multi_measure_rests) {
        return Err(RenderError::invalid_input("per-track score layout does not combine multi-measure rests; disable them or use layout_score".into()));
    }
    let count = tracks[0].track.measures.len();
    if tracks.iter().any(|t| t.track.measures.len() != count) {
        return Err(RenderError::invalid_input(
            "score tracks need matching measure counts".into(),
        ));
    }
    // Different meters may share a written bar. Plans merge their onset columns
    // and widths; callers still need a common measure partition for system
    // alignment and repeat/navigation structure.
    let rendered_states = tracks
        .iter()
        .map(|t| crate::spans::prepare(&t.track, t.options))
        .collect::<Result<Vec<_>, _>>()?;
    let individual = tracks
        .iter()
        .zip(tracks)
        .map(|(item, _)| validate(&item.track, item.options))
        .collect::<Result<Vec<_>, _>>()?;
    let plans = merge_score_plans(&individual, count);
    let mut rendered = vec![];
    for (index, (render, item)) in rendered_states.iter().zip(tracks).enumerate() {
        rendered.push(engrave_planned_scene(
            &item.track,
            render,
            LayoutOptions {
                show_metadata: item.options.show_metadata && index == 0,
                ..item.options
            },
            plans.clone(),
        )?);
    }
    stack_score_pages(
        &rendered,
        tracks.iter().map(|t| t.track.name.as_str()).collect(),
        base,
    )
}

/// Merge independently validated measure plans into the horizontal grid shared
/// by every staff. Keeping this operation separate prevents score entry points
/// from growing their own subtly different synchronization algorithms.
fn merge_score_plans(individual: &[Vec<MeasurePlan>], count: usize) -> Vec<MeasurePlan> {
    (0..count)
        .map(|mi| {
            let header = individual.iter().map(|p| p[mi].header).fold(0.0, f32::max);
            let mut columns: Vec<_> = individual
                .iter()
                .flat_map(|p| p[mi].columns.clone())
                .collect();
            columns.sort_by(|a, b| a.0.total_cmp(&b.0));
            let mut merged: Vec<(f64, f32)> = vec![];
            for column in columns {
                if let Some(last) = merged
                    .last_mut()
                    .filter(|last| (last.0 - column.0).abs() < 1e-8)
                {
                    last.1 = last.1.max(column.1);
                } else {
                    merged.push(column);
                }
            }
            let width = (header + 24.0 + merged.iter().map(|c| c.1).sum::<f32>())
                .max(individual.iter().map(|p| p[mi].width).fold(180.0, f32::max));
            MeasurePlan {
                header,
                width,
                columns: merged,
            }
        })
        .collect()
}

/// Add brace/bracket grouping around a score assembled from consecutive tracks.
pub fn layout_instruments(
    tracks: &[crate::ScoreTrack],
    groups: &[crate::InstrumentGroup],
) -> Result<crate::ScoreLayout, RenderError> {
    let mut score = layout_score_tracks(tracks)?;
    for group in groups {
        if group.tracks.start >= group.tracks.end || group.tracks.end > tracks.len() {
            return Err(RenderError::invalid_input(
                "instrument group has an invalid track range".into(),
            ));
        }
        for system in score.geometry.systems.clone() {
            let members: Vec<_> = score
                .beats
                .iter()
                .filter(|b| {
                    group.tracks.contains(&b.track)
                        && b.beat.cursor_rect[1] >= system[0]
                        && b.beat.cursor_rect[1] < system[1]
                })
                .collect();
            let Some(top) = members
                .iter()
                .map(|b| b.beat.cursor_rect[1])
                .reduce(f32::min)
            else {
                continue;
            };
            let bottom = members
                .iter()
                .map(|b| b.beat.cursor_rect[3])
                .reduce(f32::max)
                .unwrap();
            let x = 14.0;
            match group.bracket {
                crate::InstrumentBracket::None => {}
                crate::InstrumentBracket::Bracket => {
                    score.geometry.primitives.push(crate::Primitive::Line {
                        from: [x + 6.0, top],
                        to: [x, top],
                        width: 1.5,
                        color: None,
                    });
                    score.geometry.primitives.push(crate::Primitive::Line {
                        from: [x, top],
                        to: [x, bottom],
                        width: 1.5,
                        color: None,
                    });
                    score.geometry.primitives.push(crate::Primitive::Line {
                        from: [x, bottom],
                        to: [x + 6.0, bottom],
                        width: 1.5,
                        color: None,
                    });
                }
                crate::InstrumentBracket::Brace => {
                    score
                        .geometry
                        .curve([x + 8.0, top], [x, (top + bottom) / 2.0], 5.0);
                    score
                        .geometry
                        .curve([x, (top + bottom) / 2.0], [x + 8.0, bottom], -5.0);
                }
            }
            if !group.name.is_empty() {
                score
                    .geometry
                    .text(30.0, top - 10.0, &group.name, 12.0, false);
            }
        }
    }
    Ok(score)
}

fn stack_score_pages(
    pages: &[Layout],
    names: Vec<&str>,
    options: LayoutOptions,
) -> Result<crate::ScoreLayout, RenderError> {
    let mut geometry = Layout {
        width: pages.iter().map(|p| p.width).fold(options.width, f32::max),
        height: 0.0,
        primitives: vec![],
        beats: vec![],
        systems: vec![],
        style: options.style,
    };
    let mut beats = vec![];
    let systems = pages.first().map(|p| p.systems.len()).unwrap_or(0);
    if pages.iter().any(|page| page.systems.len() != systems) {
        return Err(RenderError::invalid_input(
            "score tracks produced incompatible system breaks".into(),
        ));
    }
    let mut y = 0.0;
    for si in 0..systems {
        let top = y;
        for (ti, page) in pages.iter().enumerate() {
            let range = page.systems[si];
            let offset = y - range[0] + 24.0;
            if options.elements.track_names {
                geometry.text(geometry.width / 2.0, y + 15.0, names[ti], 14.0, false);
            }
            for primitive in page.primitives.iter().skip(1) {
                let py = match primitive {
                    Primitive::Line { from, to, .. } => (from[1] + to[1]) / 2.0,
                    Primitive::Curve { points, .. } => (points[0][1] + points[3][1]) / 2.0,
                    Primitive::Text { at, .. } | Primitive::Glyph { at, .. } => at[1],
                };
                if py >= range[0] && py < range[1] {
                    let mut primitive = primitive.clone();
                    match &mut primitive {
                        Primitive::Line { from, to, .. } => {
                            from[1] += offset;
                            to[1] += offset;
                        }
                        Primitive::Curve { points, .. } => {
                            for point in points {
                                point[1] += offset;
                            }
                        }
                        Primitive::Text { at, .. } | Primitive::Glyph { at, .. } => at[1] += offset,
                    }
                    geometry.primitives.push(primitive);
                }
            }
            for bound in &page.beats {
                if bound.cursor_rect[1] >= range[0] && bound.cursor_rect[1] < range[1] {
                    let mut beat = bound.clone();
                    for rect in [&mut beat.rect, &mut beat.cursor_rect] {
                        rect[1] += offset;
                        rect[3] += offset;
                    }
                    beats.push(crate::ScoreBeatBounds { track: ti, beat });
                }
            }
            y += range[1] - range[0] + 24.0;
        }
        geometry.systems.push([top, y]);
    }
    geometry.height = y.max(80.0);
    Ok(crate::ScoreLayout { geometry, beats })
}
