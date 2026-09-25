//! Scene construction: validation, planning, and notation engraving.
use crate::{Clef, DisplayMode, Fret, LayoutMode, Note, RenderError, Scene, SceneOptions, Track};
use smufl::Glyph as G;

use super::layout;
use super::layout::{
    compute_measure_depth, compute_notation_extents, compute_row_headroom, compute_scene_width,
    justify_measure_plans,
};
use super::measure::{render_measure_frame, MeasureFrame};
use super::parameters::{
    DOT_SPACING, EMPHASIZED_TEXT_SIZE, LARGE_TEXT_SIZE, SMALL_GLYPH_SIZE, STAFF_HEIGHT,
    STAFF_LINE_SPACING, STAFF_MIDDLE_LINE_OFFSET,
};
use super::planning::{create_measure_plans, MeasurePlan};
use super::voices::{render_measure_voices, MeasureVoices, RenderState};

/// Selects the SMuFL flag glyph for a stem direction and subdivision level.
pub(crate) fn select_flag_glyph(levels: u32, down: bool) -> G {
    match (levels, down) {
        (1, false) => G::Flag8thUp,
        (1, true) => G::Flag8thDown,
        (2, false) => G::Flag16thUp,
        (2, true) => G::Flag16thDown,
        (3, false) => G::Flag32ndUp,
        (3, true) => G::Flag32ndDown,
        (4, false) => G::Flag64thUp,
        (4, true) => G::Flag64thDown,
        (_, false) => G::Flag128thUp,
        (_, true) => G::Flag128thDown,
    }
}

/// Formats the fret label shown for a tablature note.
pub(super) fn format_fret_label(note: &Note) -> String {
    let s = match note.fret {
        Fret::Number(n) | Fret::Tied(n) => n.to_string(),
        Fret::Dead => "x".into(),
    };
    if note.effects.ghost {
        format!("({s})")
    } else if note.effects.harmonic.is_some() {
        format!("<{s}>")
    } else {
        s
    }
}
/// Validates a layout request and builds its per-measure horizontal plans.
pub(crate) fn validate(
    track: &Track,
    options: SceneOptions,
) -> Result<Vec<MeasurePlan>, RenderError> {
    create_measure_plans(track, options)
}

/// Lay out notation in vertical systems or a horizontally scrolling strip.
/// Pitch spelling is explicit; missing pitches are errors in staff modes.
pub fn engrave(track: &Track, options: SceneOptions) -> Result<Scene, RenderError> {
    let render = crate::spans::prepare(track, options)?;
    let plans = validate(track, options)?;
    if options.multi_measure_rests {
        return crate::rests::single(track, options);
    }
    engrave_planned_scene(track, &render, options, plans)
}

/// Converts validated measure plans into systems and drawing primitives.
pub(crate) fn engrave_planned_scene(
    track: &Track,
    render: &crate::spans::RenderState,
    options: SceneOptions,
    mut plans: Vec<MeasurePlan>,
) -> Result<Scene, RenderError> {
    let width = compute_scene_width(&plans, options)?;
    justify_measure_plans(track, &mut plans, options, width);
    let mut page = Scene {
        width,
        height: STAFF_HEIGHT * 2.0,
        primitives: vec![],
        beats: vec![],
        systems: vec![],
        style: options.style,
    };
    if options.elements.track_names {
        page.text(
            width / 2.0,
            STAFF_HEIGHT - EMPHASIZED_TEXT_SIZE,
            &track.name,
            LARGE_TEXT_SIZE,
            false,
        );
    }
    let extents = compute_notation_extents(track, options);
    let layout::NotationExtents {
        tab,
        staff,
        tab_height,
        low_pitch,
        high_pitch,
        staff_offset,
        height,
        max_voices,
        voice_spacing,
    } = extents;
    let mut top = compute_row_headroom(0, track, &plans, options, width)?;
    // Reserve one annotation lane per string, plus separate rhythm lanes per voice.
    let mut x = STAFF_HEIGHT + SMALL_GLYPH_SIZE;
    let metadata_height = crate::spans::metadata(&mut page, track, options);
    let mut y = top + STAFF_HEIGHT - high_pitch + metadata_height;
    let mut row_bottom = y + height + STAFF_HEIGHT * 2.0;
    let mut system_start = 0.0;
    let mut render_state = RenderState::default();
    let mut row_measures = 0;
    for (mi, (m, plan)) in track.measures.iter().zip(&plans).enumerate() {
        if options.flow == LayoutMode::Vertical
            && x > STAFF_HEIGHT + SMALL_GLYPH_SIZE
            && (x + plan.width > width - STAFF_MIDDLE_LINE_OFFSET
                || m.break_before
                || options.bars_per_system.is_some_and(|n| row_measures >= n))
        {
            x = STAFF_HEIGHT + SMALL_GLYPH_SIZE;
            row_measures = 0;
            render_state
                .primitive_systems
                .resize(page.primitives.len(), page.systems.len());
            page.systems
                .push([system_start, row_bottom + options.engraving.system_gap]);
            system_start = row_bottom + options.engraving.system_gap;
            top = compute_row_headroom(mi, track, &plans, options, width)?;
            y = row_bottom + options.engraving.system_gap + top - high_pitch;
            row_bottom = y + height + STAFF_HEIGHT * 2.0;
        }
        let current_clef = if options.display == DisplayMode::Slash {
            Clef::Treble
        } else {
            track.measures[..=mi]
                .iter()
                .rev()
                .find_map(|m| m.clef)
                .unwrap_or(track.clef)
        };
        let ty = y + staff_offset;
        let bottom = if tab { ty + tab_height } else { y + low_pitch };
        render_measure_frame(
            &mut page,
            &MeasureFrame {
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
                tab_height,
                clef: current_clef,
            },
        )?;
        render_measure_voices(
            &mut page,
            &mut render_state,
            &MeasureVoices {
                render,
                track,
                measure: m,
                plan,
                index: mi,
                options,
                x,
                y,
                tab_y: ty,
                bottom,
                tab,
                staff,
                width,
                max_voices,
                voice_spacing,
                clef: current_clef,
            },
        )?;
        x += plan.width;
        row_measures += 1;
        row_bottom =
            row_bottom.max(bottom + max_voices as f32 * voice_spacing + compute_measure_depth(m)?);
        page.height = row_bottom + STAFF_LINE_SPACING + DOT_SPACING;
    }
    if !plans.is_empty() {
        render_state
            .primitive_systems
            .resize(page.primitives.len(), page.systems.len());
        page.systems.push([system_start, page.height]);
    }
    crate::spans::draw(
        &mut page,
        track,
        render,
        options,
        &mut render_state.primitive_systems,
    )?;
    Ok(page)
}
