//! ScoreDocument adaptation and cross-staff decoration.
use super::*;

/// Lays out a native staff document and decorates it with cross-staff spans.
pub fn layout_document(document: &crate::ScoreDocument) -> Result<crate::ScoreLayout, RenderError> {
    document.validate()?;
    if document.instruments.is_empty() {
        return Err(RenderError::invalid_input(
            "a score document needs at least one instrument".into(),
        ));
    }
    let mut tracks = vec![];
    let mut groups = vec![];
    for instrument in &document.instruments {
        if instrument.staves.is_empty() {
            return Err(RenderError::invalid_input(
                "an instrument needs at least one staff".into(),
            ));
        }
        let start = tracks.len();
        for staff in &instrument.staves {
            let mut track = staff.track.clone();
            if !document.master_bars.is_empty() && !staff.master_bar_map.is_empty() {
                if staff.master_bar_map.len() != track.measures.len()
                    || staff
                        .master_bar_map
                        .windows(2)
                        .any(|pair| pair[0] >= pair[1])
                    || staff
                        .master_bar_map
                        .iter()
                        .any(|index| *index >= document.master_bars.len())
                {
                    return Err(RenderError::invalid_input(
                        "staff master-bar map must be ordered and match its measures".into(),
                    ));
                }
                let source = track.measures;
                track.measures = (0..document.master_bars.len())
                    .map(|master| {
                        staff
                            .master_bar_map
                            .iter()
                            .position(|mapped| *mapped == master)
                            .map_or_else(Measure::default, |local| source[local].clone())
                    })
                    .collect();
                for span in &mut track.spans {
                    span.start.measure =
                        *staff
                            .master_bar_map
                            .get(span.start.measure)
                            .ok_or_else(|| {
                                RenderError::invalid_input(
                                    "span start is outside the staff master-bar map".into(),
                                )
                            })?;
                    span.end.measure =
                        *staff.master_bar_map.get(span.end.measure).ok_or_else(|| {
                            RenderError::invalid_input(
                                "span end is outside the staff master-bar map".into(),
                            )
                        })?;
                }
            }
            tracks.push(crate::ScoreTrack {
                track,
                options: staff.options,
            });
        }
        groups.push(crate::InstrumentGroup {
            name: instrument.name.clone(),
            tracks: start..tracks.len(),
            bracket: instrument.bracket,
        });
    }
    let count = tracks[0].track.measures.len();
    if !document.master_bars.is_empty() && document.master_bars.len() != count {
        return Err(RenderError::invalid_input(
            "master-bar count must match each staff's measure count".into(),
        ));
    }
    let mut previous_end = 0.0;
    for bar in &document.master_bars {
        if !bar.start_quarters.is_finite()
            || !bar.duration_quarters.is_finite()
            || bar.start_quarters < previous_end - 1e-8
            || bar.duration_quarters <= 0.0
        {
            return Err(RenderError::invalid_input(
                "master bars must be finite, positive and ordered".into(),
            ));
        }
        previous_end = bar.start_quarters + bar.duration_quarters;
    }
    let mut score = layout_instruments(&tracks, &groups)?;
    for span in &document.cross_staff_spans {
        let start = score.bounds(span.start).cloned().ok_or_else(|| {
            RenderError::invalid_input("cross-staff span start does not exist".into())
        })?;
        let end = score.bounds(span.end).cloned().ok_or_else(|| {
            RenderError::invalid_input("cross-staff span end does not exist".into())
        })?;
        let system = |bound: &BeatBounds| {
            score.geometry.systems.iter().position(|system| {
                bound.cursor_rect[1] >= system[0] && bound.cursor_rect[1] < system[1]
            })
        };
        if system(&start) != system(&end) {
            return Err(RenderError::invalid_input(
                "cross-staff spans cannot cross a system break".into(),
            ));
        }
        let from = [
            (start.cursor_rect[0] + start.cursor_rect[2]) / 2.0,
            (start.cursor_rect[1] + start.cursor_rect[3]) / 2.0,
        ];
        let to = [
            (end.cursor_rect[0] + end.cursor_rect[2]) / 2.0,
            (end.cursor_rect[1] + end.cursor_rect[3]) / 2.0,
        ];
        match span.kind {
            crate::CrossStaffSpanKind::Slur => score.geometry.curve(from, to, -18.0),
            crate::CrossStaffSpanKind::Beam => {
                let beam_y = (from[1] + to[1]) / 2.0;
                score.geometry.line(from[0], from[1], from[0], beam_y, 1.2);
                score.geometry.line(to[0], to[1], to[0], beam_y, 1.2);
                let beam_width = crate::music_font::thickness(
                    crate::music_font::metadata()
                        .engraving_defaults
                        .beam_thickness,
                    10.0,
                    3.5,
                );
                score
                    .geometry
                    .line(from[0], beam_y, to[0], beam_y, beam_width);
            }
        }
    }
    Ok(score)
}
