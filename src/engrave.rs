//! Validation, rhythmic planning, system layout, and notation engraving.
use crate::*;
use smufl::Glyph as G;

mod annotations;
mod measure;
mod numbered;
mod planning;
mod rhythm;
mod score_layout;
mod staff;
mod systems;
mod voices;

use annotations::*;
use measure::{render_measure_frame, MeasureFrame};
use numbered::numbered_beat;
pub(crate) use numbered::voice_offset;
use planning::{create_measure_plans, MeasurePlan};
use rhythm::*;
pub use score_layout::{layout_document, layout_instruments, layout_score, layout_score_tracks};
pub(crate) use staff::pitch_y;
use staff::{accidental_marks, draw_staff, key_accidental, staff_beat, StaffStyle};
use systems::{
    justify_measure_plans, layout_width, measure_depth, notation_extents, system_headroom,
};
use voices::{render_measure_voices, MeasureVoices, RenderState};

/// Selects the SMuFL flag glyph for a stem direction and subdivision level.
pub(crate) fn flag_glyph(levels: u32, down: bool) -> G {
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
fn label(note: &Note) -> String {
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
fn validate(track: &Track, options: LayoutOptions) -> Result<Vec<MeasurePlan>, RenderError> {
    create_measure_plans(track, options)
}

/// Performs every validation required for a single-track layout without
/// emitting geometry. This is the implementation behind `Track::validate_for`.
pub(crate) fn validate_for_layout(
    track: &Track,
    options: LayoutOptions,
) -> Result<(), RenderError> {
    let prepared = crate::spans::prepare(track, options)?;
    validate(&prepared, options).map(|_| ())
}

/// Lay out notation in vertical systems or a horizontally scrolling strip.
/// Pitch spelling is explicit; missing pitches are errors in staff modes.
pub fn layout(track: &Track, options: LayoutOptions) -> Result<Layout, RenderError> {
    let track = crate::spans::prepare(track, options)?;
    let plans = validate(&track, options)?;
    if options.multi_measure_rests {
        return crate::rests::single(&track, options);
    }
    layout_planned(&track, options, plans)
}

/// Converts validated measure plans into systems and drawing primitives.
fn layout_planned(
    track: &Track,
    options: LayoutOptions,
    mut plans: Vec<MeasurePlan>,
) -> Result<Layout, RenderError> {
    let width = layout_width(&plans, options)?;
    justify_measure_plans(track, &mut plans, options, width);
    let mut page = Layout {
        width,
        height: 80.0,
        primitives: vec![],
        beats: vec![],
        systems: vec![],
        style: options.style,
    };
    if options.elements.track_names {
        page.text(width / 2.0, 26.0, &track.name, 20.0, false);
    }
    let extents = notation_extents(track, options);
    let systems::NotationExtents {
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
    let mut top = system_headroom(0, track, &plans, options, width)?;
    // Reserve one annotation lane per string, plus separate rhythm lanes per voice.
    let mut x = 44.0;
    let metadata_height = crate::spans::metadata(&mut page, track, options);
    let mut y = top + 40.0 - high_pitch + metadata_height;
    let mut row_bottom = y + height + 80.0;
    let mut system_start = 0.0;
    let mut render_state = RenderState::default();
    let mut row_measures = 0;
    for (mi, (m, plan)) in track.measures.iter().zip(&plans).enumerate() {
        if options.flow == LayoutMode::Vertical
            && x > 44.0
            && (x + plan.width > width - 20.0
                || m.break_before
                || options.bars_per_system.is_some_and(|n| row_measures >= n))
        {
            x = 44.0;
            row_measures = 0;
            render_state
                .primitive_systems
                .resize(page.primitives.len(), page.systems.len());
            page.systems
                .push([system_start, row_bottom + options.engraving.system_gap]);
            system_start = row_bottom + options.engraving.system_gap;
            top = system_headroom(mi, track, &plans, options, width)?;
            y = row_bottom + options.engraving.system_gap + top - high_pitch;
            row_bottom = y + height + 80.0;
        }
        let current_clef = track.measures[..=mi]
            .iter()
            .rev()
            .find_map(|m| m.clef)
            .unwrap_or(track.clef);
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
        row_bottom = row_bottom.max(bottom + max_voices as f32 * voice_spacing + measure_depth(m)?);
        page.height = row_bottom + 15.0;
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
        options,
        &mut render_state.primitive_systems,
    )?;
    Ok(page)
}
