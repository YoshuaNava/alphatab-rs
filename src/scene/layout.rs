//! Measurement, wrapping, and horizontal distribution of scene rows.

use crate::{Clef, DisplayMode, LayoutMode, Measure, RenderError, Track};

use super::parameters::SYSTEM_LAYOUT;
use super::planning::MeasurePlan;
use super::{compute_pitch_y, SceneOptions};

/// Vertical measurements shared by every system in a track layout.
#[derive(Clone, Copy)]
pub(super) struct NotationExtents {
    /// Whether tablature lines are visible.
    pub(super) tab: bool,
    /// Whether a standard staff is visible.
    pub(super) staff: bool,
    /// Height occupied by all tablature strings.
    pub(super) tab_height: f32,
    /// Lowest staff-relative drawing extent.
    pub(super) low_pitch: f32,
    /// Highest staff-relative drawing extent.
    pub(super) high_pitch: f32,
    /// Vertical distance from the staff origin to the tablature origin.
    pub(super) staff_offset: f32,
    /// Base height occupied by the selected notation modes.
    pub(super) height: f32,
    /// Largest number of voices found in a measure.
    pub(super) max_voices: usize,
    /// Vertical distance reserved for each voice's rhythm and lyrics.
    pub(super) voice_spacing: f32,
}

/// Resolves the page width and rejects content that violates strict-width mode.
pub(super) fn compute_scene_width(
    plans: &[MeasurePlan],
    options: SceneOptions,
) -> Result<f32, RenderError> {
    let width = if options.flow == LayoutMode::Horizontal {
        plans.iter().map(|plan| plan.width).sum::<f32>() + SYSTEM_LAYOUT.content_horizontal_inset
    } else {
        plans.iter().fold(options.width, |width, plan| {
            width.max(plan.width + SYSTEM_LAYOUT.content_horizontal_inset)
        })
    }
    .max(options.width);
    if options.strict_width && width > options.width {
        return Err(RenderError::invalid_input(
            "notation exceeds the requested width; increase width or reduce zoom".into(),
        ));
    }
    Ok(width)
}

/// Distributes unused horizontal space across beat columns in each system.
pub(super) fn justify_measure_plans(
    track: &Track,
    plans: &mut [MeasurePlan],
    options: SceneOptions,
    width: f32,
) {
    if !options.justify || options.flow != LayoutMode::Vertical {
        return;
    }
    let mut first = 0;
    while first < plans.len() {
        let end = find_row_end(first, track, plans, options, width);
        let total = plans[first..end].iter().map(|plan| plan.width).sum::<f32>();
        let columns = plans[first..end]
            .iter()
            .map(|plan| plan.columns.len().max(1))
            .sum::<usize>();
        let extra =
            (width - SYSTEM_LAYOUT.content_horizontal_inset - total).max(0.0) / columns as f32;
        for plan in &mut plans[first..end] {
            plan.width += extra * plan.columns.len().max(1) as f32;
            for column in &mut plan.columns {
                column.1 += extra;
            }
        }
        first = end;
    }
}

/// Measures the vertical space required by staff, tablature, voices, and lyrics.
pub(super) fn compute_notation_extents(track: &Track, options: SceneOptions) -> NotationExtents {
    let tab = options.display.renders_tab() && track.clef != Clef::Percussion;
    let staff = options.display.renders_staff() || track.clef == Clef::Percussion;
    let tab_height = (track.strings.len().saturating_sub(1)) as f32 * options.string_spacing;
    let mut low_pitch = SYSTEM_LAYOUT.default_staff_low_pitch;
    let mut high_pitch = SYSTEM_LAYOUT.default_staff_high_pitch;
    let mut measure_clef = if options.display == DisplayMode::Slash {
        Clef::Treble
    } else {
        track.clef
    };
    if staff {
        for measure in &track.measures {
            if options.display != DisplayMode::Slash {
                measure_clef = measure.clef.unwrap_or(measure_clef);
            }
            for note in measure.voices.iter().flatten().flat_map(|beat| &beat.notes) {
                for pitch in [
                    note.pitch,
                    note.effects.grace_pitch,
                    note.effects.harmonic_pitch,
                ]
                .into_iter()
                .flatten()
                {
                    let y = compute_pitch_y(pitch, measure_clef, 0.0);
                    low_pitch = low_pitch.max(y + SYSTEM_LAYOUT.staff_note_clearance);
                    high_pitch = high_pitch.min(y - SYSTEM_LAYOUT.staff_note_clearance);
                }
            }
        }
    }
    if options.display == DisplayMode::Numbered {
        let notes = track
            .measures
            .iter()
            .flat_map(|measure| &measure.voices)
            .flatten()
            .map(|beat| beat.notes.len())
            .max()
            .unwrap_or(1);
        high_pitch = -((notes.saturating_sub(1)) as f32 * SYSTEM_LAYOUT.numbered_stack_spacing
            + SYSTEM_LAYOUT.numbered_top_padding);
    }
    let staff_offset = if staff {
        low_pitch + SYSTEM_LAYOUT.staff_to_tab_gap
    } else {
        0.0
    };
    let max_voices = track
        .measures
        .iter()
        .map(|measure| measure.voices.len())
        .max()
        .unwrap_or(1)
        .max(1);
    let lyric_lines = track
        .measures
        .iter()
        .flat_map(|measure| &measure.voices)
        .flatten()
        .map(|beat| beat.annotations.lyrics.lines().count().saturating_sub(1))
        .max()
        .unwrap_or(0) as f32;
    NotationExtents {
        tab,
        staff,
        tab_height,
        low_pitch,
        high_pitch,
        staff_offset,
        height: if tab {
            staff_offset + tab_height
        } else {
            low_pitch + SYSTEM_LAYOUT.staff_note_clearance
        },
        max_voices,
        voice_spacing: SYSTEM_LAYOUT.base_voice_spacing
            + lyric_lines * SYSTEM_LAYOUT.lyric_line_spacing,
    }
}

/// Returns the first measure after the system that starts at `start`.
pub(super) fn find_row_end(
    start: usize,
    track: &Track,
    plans: &[MeasurePlan],
    options: SceneOptions,
    width: f32,
) -> usize {
    let mut end = start;
    let mut row_width = 0.0;
    while end < plans.len() {
        if end > start
            && options.flow == LayoutMode::Vertical
            && (track.measures[end].break_before
                || row_width + plans[end].width > width - SYSTEM_LAYOUT.content_horizontal_inset
                || options
                    .bars_per_system
                    .is_some_and(|count| end - start >= count))
        {
            break;
        }
        row_width += plans[end].width;
        end += 1;
    }
    end
}

/// Measures space above a system for annotations and crossing spans.
pub(super) fn compute_row_headroom(
    start: usize,
    track: &Track,
    plans: &[MeasurePlan],
    options: SceneOptions,
    width: f32,
) -> Result<f32, RenderError> {
    let end = find_row_end(start, track, plans, options, width);
    let annotation_height = track.measures[start..end]
        .iter()
        .flat_map(|measure| &measure.voices)
        .flatten()
        .map(|beat| crate::elements::annotation_extents(&beat.annotations).map(|extent| extent[0]))
        .collect::<Result<Vec<_>, _>>()?
        .into_iter()
        .fold(0.0_f32, f32::max);
    let spans = track
        .spans
        .iter()
        .filter(|span| span.start.measure < end && span.end.measure >= start)
        .count();
    Ok(SYSTEM_LAYOUT
        .minimum_row_headroom
        .max(annotation_height + SYSTEM_LAYOUT.annotation_headroom_padding)
        + spans.min(SYSTEM_LAYOUT.maximum_span_lanes) as f32 * SYSTEM_LAYOUT.span_lane_height)
}

/// Measures the space below a measure for note effects and beat annotations.
pub(super) fn compute_measure_depth(measure: &Measure) -> Result<f32, RenderError> {
    let mut effect_height = 0.0_f32;
    for note in measure.voices.iter().flatten().flat_map(|beat| &beat.notes) {
        let effect = &note.effects;
        if effect.trill_fret.is_some()
            || effect.palm_mute
            || effect.let_ring
            || effect.harmonic.is_some()
            || effect.fingering.is_some()
            || effect.vibrato
            || !effect.bend.is_empty()
        {
            effect_height = effect_height.max(
                SYSTEM_LAYOUT.effect_base_depth
                    + note.string as f32 * SYSTEM_LAYOUT.effect_string_depth_step,
            );
        }
    }
    let annotation_depth = measure
        .voices
        .iter()
        .flatten()
        .map(|beat| crate::elements::annotation_extents(&beat.annotations).map(|extent| extent[1]))
        .collect::<Result<Vec<_>, _>>()?
        .into_iter()
        .fold(0.0_f32, f32::max);
    Ok(effect_height + annotation_depth)
}
