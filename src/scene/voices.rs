//! Scene construction for voices, beats, note connections, and hit regions.

use crate::{
    Beaming, Beat, BeatAddress, BeatBounds, Clef, DisplayMode, Fret, Measure, Note, RenderError,
    SpanKind, TabRhythm, Track,
};

use super::annotations::{draw_beat_annotations, draw_note_effects};
use super::build::format_fret_label;
use super::numbered::draw_numbered_beat;
use super::parameters::{
    DOT_SPACING, EMPHASIZED_STROKE_WIDTH, LARGE_TEXT_SIZE, NOTE_GLYPH_SIZE, SMALL_GLYPH_SIZE,
    SMALL_TEXT_SIZE, STAFF_HEIGHT, STAFF_MIDDLE_LINE_OFFSET, SYSTEM_LAYOUT, THIN_STROKE_WIDTH,
    TIMELINE_EPSILON,
};
use super::planning::MeasurePlan;
use super::rhythm::{
    draw_beams, draw_tablature_rhythm, draw_tuplets, stem_points_down, VoiceLayout,
};
use super::staff::{collect_accidental_marks, draw_staff_beat, StaffStyle};
use super::{compute_pitch_y, compute_voice_offset, Scene, SceneOptions};
use std::collections::HashMap;

/// Cross-measure state needed to continue ties, slides, and spans.
#[derive(Default)]
pub(super) struct RenderState {
    /// System index assigned to every emitted primitive.
    pub(super) primitive_systems: Vec<usize>,
    /// Last standard-notation position for each voice and string.
    previous_staff: HashMap<(usize, usize), ([f32; 2], f32)>,
    /// Last tablature position and outgoing hammer/slide flags.
    previous_tab: HashMap<(usize, usize), ([f32; 2], bool, bool)>,
}

/// Immutable inputs and resolved geometry used to render one measure's voices.
pub(super) struct MeasureVoices<'a> {
    /// Display-only transformations for this engraving request.
    pub(super) render: &'a crate::spans::RenderState,
    /// Track that owns the measure.
    pub(super) track: &'a Track,
    /// Measure containing the voices.
    pub(super) measure: &'a Measure,
    /// Horizontal measure plan.
    pub(super) plan: &'a MeasurePlan,
    /// Zero-based measure index.
    pub(super) index: usize,
    /// Scene options controlling notation output.
    pub(super) options: SceneOptions,
    /// Left edge of the measure.
    pub(super) x: f32,
    /// Staff origin of the current system.
    pub(super) y: f32,
    /// Tablature origin of the current system.
    pub(super) tab_y: f32,
    /// Lowest notation line in the system.
    pub(super) bottom: f32,
    /// Whether tablature is visible.
    pub(super) tab: bool,
    /// Whether standard notation is visible.
    pub(super) staff: bool,
    /// Page width used for connection continuations.
    pub(super) width: f32,
    /// Maximum voice count in the track.
    pub(super) max_voices: usize,
    /// Vertical distance reserved for each voice.
    pub(super) voice_spacing: f32,
    /// Clef active in the measure.
    pub(super) clef: Clef,
}

/// Beat-local geometry shared by staff, tablature, and rhythm rendering.
#[derive(Clone, Copy)]
struct BeatRender<'a> {
    voice: &'a [Beat],
    beat: &'a Beat,
    positions: &'a [f32],
    times: &'a [f64],
    staff_positions: &'a [f32],
    accidentals: &'a std::collections::HashSet<(usize, usize, usize)>,
    voice_index: usize,
    beat_index: usize,
    x: f32,
    rhythm_y: f32,
    column_width: f32,
    explicit_beam: bool,
}

/// Renders every beat in a measure and records its interactive bounds.
pub(super) fn render_measure_voices(
    page: &mut Scene,
    state: &mut RenderState,
    context: &MeasureVoices<'_>,
) -> Result<(), RenderError> {
    let MeasureVoices {
        track,
        measure: m,
        plan,
        index: mi,
        x,
        y,
        bottom,
        staff,
        voice_spacing,
        ..
    } = *context;
    let accidentals = collect_accidental_marks(m);
    for (vi, voice) in m.voices.iter().enumerate() {
        // Resolve this voice against the shared measure columns before emitting
        // its notation, interactive bounds, and cross-measure connections.
        let (positions, times) = compute_voice_positions(voice, plan, x)?;
        let staff_positions: Vec<_> = positions
            .iter()
            .zip(&times)
            .map(|(x, t)| x + compute_voice_offset(m, vi, *t))
            .collect();
        let ry = bottom + STAFF_MIDDLE_LINE_OFFSET + SMALL_GLYPH_SIZE + vi as f32 * voice_spacing;
        for (bi, source_beat) in voice.iter().enumerate() {
            let displayed = context.render.beat(source_beat, context.options)?;
            let beat = displayed.as_ref();
            let address = BeatAddress {
                measure: mi,
                voice: vi,
                beat: bi,
            };
            let explicit_beam = track.spans.iter().any(|span| {
                matches!(span.kind, SpanKind::Beam) && crate::spans::contains(span, address)
            });
            let bx = positions[bi];
            let ci = plan
                .columns
                .iter()
                .position(|c| (c.0 - times[bi]).abs() < TIMELINE_EPSILON)
                .unwrap();
            let cw = plan.columns[ci].1;
            let beat_render = BeatRender {
                voice,
                beat,
                positions: &positions,
                times: &times,
                staff_positions: &staff_positions,
                accidentals: &accidentals,
                voice_index: vi,
                beat_index: bi,
                x: bx,
                rhythm_y: ry,
                column_width: cw,
                explicit_beam,
            };
            if m.simile.is_none() && m.rest_count <= 1 {
                render_notes(page, state, context, beat, vi, bx, cw)?;
                if staff {
                    render_staff_rhythm(page, context, &beat_render)?;
                }
                render_secondary_rhythm(page, context, &beat_render)?;
                draw_strum(page, context, &beat_render);
                draw_beat_annotations(page, &beat.annotations, bx, y, ry, cw, context.options)?;
            }
            record_beat_bounds(
                page,
                beat,
                BeatAddress {
                    measure: mi,
                    voice: vi,
                    beat: bi,
                },
                times[bi],
                [
                    bx - cw / 2.0,
                    ry - DOT_SPACING,
                    bx + cw / 2.0,
                    ry + STAFF_HEIGHT + DOT_SPACING,
                ],
                [
                    bx - cw / 2.0,
                    y - NOTE_GLYPH_SIZE,
                    bx + cw / 2.0,
                    bottom + NOTE_GLYPH_SIZE,
                ],
            )?;
            if beat.notes.is_empty() {
                state.previous_tab.retain(|(voice, _), _| *voice != vi);
                state.previous_staff.retain(|(voice, _), _| *voice != vi);
            }
        }
    }
    Ok(())
}

/// Records the hit target and playback cursor region for one rendered beat.
fn record_beat_bounds(
    page: &mut Scene,
    beat: &Beat,
    address: BeatAddress,
    start: f64,
    rect: [f32; 4],
    cursor_rect: [f32; 4],
) -> Result<(), RenderError> {
    page.beats.push(BeatBounds {
        start,
        duration: beat.compute_quarter_beats()?,
        measure: address.measure,
        voice: address.voice,
        beat: address.beat,
        rect,
        cursor_rect,
    });
    Ok(())
}

/// Draws note labels, continuations, and note-specific effect glyphs.
fn render_notes(
    page: &mut Scene,
    state: &mut RenderState,
    context: &MeasureVoices<'_>,
    beat: &Beat,
    voice: usize,
    beat_x: f32,
    column_width: f32,
) -> Result<(), RenderError> {
    for note in &beat.notes {
        let note_y = if context.options.display == DisplayMode::Numbered {
            context.y
                + STAFF_MIDDLE_LINE_OFFSET
                + voice as f32 * SYSTEM_LAYOUT.numbered_voice_spacing
        } else if context.tab {
            context.tab_y + (note.string.saturating_sub(1)) as f32 * context.options.string_spacing
        } else {
            compute_pitch_y(note.pitch.unwrap(), context.clef, context.y)
        };
        if context.tab {
            page.text(
                beat_x,
                note_y,
                format_fret_label(note),
                LARGE_TEXT_SIZE,
                true,
            );
        }
        draw_tab_connection(page, state, context, note, voice, beat_x, note_y);
        if context.staff && context.tab {
            draw_staff_tie(page, state, context, note, voice, beat_x);
        }
        state.previous_tab.insert(
            (voice, note.string),
            ([beat_x, note_y], note.effects.hammer_on, note.effects.slide),
        );
        let effect_y = context.bottom
            + context.max_voices as f32 * context.voice_spacing
            + STAFF_HEIGHT
            + (note.string.saturating_sub(1)) as f32 * STAFF_MIDDLE_LINE_OFFSET;
        draw_note_effects(
            page,
            note,
            [beat_x, note_y, column_width, effect_y],
            context.tab,
        )?;
    }
    Ok(())
}

/// Continues tablature ties, hammer-ons, pull-offs, and slides across systems.
fn draw_tab_connection(
    page: &mut Scene,
    state: &mut RenderState,
    context: &MeasureVoices<'_>,
    note: &Note,
    voice: usize,
    beat_x: f32,
    note_y: f32,
) {
    let Some((from, hammer, slide)) = state.previous_tab.get(&(voice, note.string)).copied() else {
        return;
    };
    let tied = matches!(note.fret, Fret::Tied(_));
    if !(tied || hammer || slide) {
        return;
    }
    if from[1] < note_y - STAFF_HEIGHT * 2.0 {
        state
            .primitive_systems
            .resize(page.primitives.len(), page.systems.len());
        draw_note_connection(
            page,
            [from[0] + NOTE_GLYPH_SIZE, from[1] + SMALL_GLYPH_SIZE],
            [
                context.width - STAFF_MIDDLE_LINE_OFFSET,
                from[1] + SMALL_GLYPH_SIZE,
            ],
            slide,
            false,
        );
        state
            .primitive_systems
            .resize(page.primitives.len(), page.systems.len().saturating_sub(1));
        draw_note_connection(
            page,
            [context.x + DOT_SPACING, note_y + SMALL_GLYPH_SIZE],
            [beat_x - NOTE_GLYPH_SIZE, note_y + SMALL_GLYPH_SIZE],
            slide,
            !tied && hammer,
        );
    } else {
        draw_note_connection(
            page,
            [from[0] + NOTE_GLYPH_SIZE, from[1] + SMALL_GLYPH_SIZE],
            [beat_x - NOTE_GLYPH_SIZE, note_y + SMALL_GLYPH_SIZE],
            slide,
            !tied && hammer,
        );
    }
}

/// Continues the standard-notation half of a tie across system boundaries.
fn draw_staff_tie(
    page: &mut Scene,
    state: &mut RenderState,
    context: &MeasureVoices<'_>,
    note: &Note,
    voice: usize,
    beat_x: f32,
) {
    let staff_y = compute_pitch_y(note.pitch.unwrap(), context.clef, context.y);
    if matches!(note.fret, Fret::Tied(_)) {
        if let Some((from, system_y)) = state.previous_staff.get(&(voice, note.string)).copied() {
            if system_y == context.y {
                page.curve(
                    [from[0] + SMALL_GLYPH_SIZE, from[1] + SMALL_GLYPH_SIZE],
                    [beat_x - SMALL_GLYPH_SIZE, staff_y + SMALL_GLYPH_SIZE],
                    NOTE_GLYPH_SIZE,
                );
            } else {
                state
                    .primitive_systems
                    .resize(page.primitives.len(), page.systems.len());
                page.curve(
                    [from[0] + SMALL_GLYPH_SIZE, from[1] + SMALL_GLYPH_SIZE],
                    [
                        context.width - STAFF_MIDDLE_LINE_OFFSET,
                        from[1] + SMALL_GLYPH_SIZE,
                    ],
                    NOTE_GLYPH_SIZE,
                );
                state
                    .primitive_systems
                    .resize(page.primitives.len(), page.systems.len().saturating_sub(1));
                page.curve(
                    [context.x + SMALL_GLYPH_SIZE, staff_y + SMALL_GLYPH_SIZE],
                    [beat_x - SMALL_GLYPH_SIZE, staff_y + SMALL_GLYPH_SIZE],
                    NOTE_GLYPH_SIZE,
                );
            }
        }
    }
    state
        .previous_staff
        .insert((voice, note.string), ([beat_x, staff_y], context.y));
}

/// Draws the directional arrow attached to a strummed chord.
fn draw_strum(page: &mut Scene, context: &MeasureVoices<'_>, render: &BeatRender<'_>) {
    let beat = render.beat;
    let Some(up) = beat.annotations.strum_up else {
        return;
    };
    let note_y = |note: &Note| {
        if context.tab {
            context.tab_y + (note.string - 1) as f32 * context.options.string_spacing
        } else {
            compute_pitch_y(note.pitch.unwrap(), context.clef, context.y)
        }
    };
    let Some(first) = beat.notes.iter().map(note_y).reduce(f32::min) else {
        return;
    };
    let last = beat.notes.iter().map(note_y).reduce(f32::max).unwrap();
    let x = render.x - render.column_width / 2.0 + SMALL_GLYPH_SIZE;
    let (start, end) = if up {
        (first - SMALL_GLYPH_SIZE, last + SMALL_GLYPH_SIZE)
    } else {
        (last + SMALL_GLYPH_SIZE, first - SMALL_GLYPH_SIZE)
    };
    page.line(x, start, x, end, EMPHASIZED_STROKE_WIDTH);
    let tail = if end > start {
        end - DOT_SPACING
    } else {
        end + DOT_SPACING
    };
    page.line(x - THIN_STROKE_WIDTH, tail, x, end, EMPHASIZED_STROKE_WIDTH);
    page.line(x + THIN_STROKE_WIDTH, tail, x, end, EMPHASIZED_STROKE_WIDTH);
}

/// Draws staff noteheads, stems, flags, and beams for one beat.
fn render_staff_rhythm(
    page: &mut Scene,
    context: &MeasureVoices<'_>,
    render: &BeatRender<'_>,
) -> Result<(), RenderError> {
    let measure = context.measure;
    let BeatRender {
        voice,
        beat,
        staff_positions: positions,
        times,
        accidentals,
        voice_index,
        beat_index,
        column_width,
        explicit_beam,
        ..
    } = *render;
    let layout = VoiceLayout {
        beats: voice,
        xs: positions,
        times,
        meter: measure.time_signature,
        groups: &measure.beam_groups,
        group_unit: measure.beam_unit.unwrap_or(measure.time_signature.1),
    };
    let mut first = beat_index;
    let mut last = beat_index;
    while first > 0 && layout.connects(first, first - 1) {
        first -= 1;
    }
    while last + 1 < voice.len() && layout.connects(last, last + 1) {
        last += 1;
    }
    let down = stem_points_down(beat, voice_index);
    let stem_end = layout.grouped(beat_index).then(|| {
        let note_positions = voice[first..=last]
            .iter()
            .flat_map(|item| &item.notes)
            .map(|note| compute_pitch_y(note.pitch.unwrap(), context.clef, context.y));
        if down {
            note_positions.fold(f32::MIN, f32::max) + STAFF_HEIGHT
        } else {
            note_positions.fold(f32::MAX, f32::min) - STAFF_HEIGHT
        }
    });
    draw_staff_beat(
        page,
        beat,
        StaffStyle {
            clef: context.clef,
            voice: voice_index,
            stem_end,
            suppress_stem: explicit_beam,
            multiple_voices: measure
                .voices
                .iter()
                .filter(|item| !item.is_empty())
                .count()
                > 1,
            column_width,
            slash: context.options.display == DisplayMode::Slash,
        },
        positions[beat_index],
        context.y,
        accidentals,
        (voice_index, beat_index),
    )?;
    if let Some(end) = stem_end.filter(|_| !explicit_beam) {
        let shifted = positions
            .iter()
            .map(|x| x + if down { -DOT_SPACING } else { DOT_SPACING })
            .collect::<Vec<_>>();
        draw_beams(
            page,
            &VoiceLayout {
                xs: &shifted,
                ..layout
            },
            beat_index,
            end,
            down,
        )?;
    }
    Ok(())
}

/// Draws numbered notation, tuplets, and optional tablature rhythm stems.
fn render_secondary_rhythm(
    page: &mut Scene,
    context: &MeasureVoices<'_>,
    render: &BeatRender<'_>,
) -> Result<(), RenderError> {
    let measure = context.measure;
    let options = context.options;
    let BeatRender {
        voice,
        beat,
        positions,
        times,
        voice_index,
        beat_index,
        x: beat_x,
        rhythm_y,
        column_width,
        explicit_beam,
        ..
    } = *render;
    if options.display == DisplayMode::Numbered {
        draw_numbered_beat(
            page,
            beat,
            measure.key_signature,
            beat_x,
            context.y + voice_index as f32 * SYSTEM_LAYOUT.numbered_voice_spacing,
            column_width,
            explicit_beam,
        )?;
        draw_tuplets(
            page,
            voice,
            positions,
            beat_index,
            context.y + STAFF_HEIGHT + voice_index as f32 * SYSTEM_LAYOUT.numbered_voice_spacing,
        )?;
    }
    if context.staff && !context.tab {
        draw_tuplets(page, voice, positions, beat_index, context.y - STAFF_HEIGHT)?;
    }
    if context.tab
        && !explicit_beam
        && options.tab_rhythm != TabRhythm::Hidden
        && (options.tab_rhythm != TabRhythm::Automatic || !context.staff)
    {
        let individual;
        let rhythm_voice = if options.tab_rhythm == TabRhythm::Individual {
            individual = voice
                .iter()
                .cloned()
                .map(|mut item| {
                    item.annotations.beaming = Beaming::Break;
                    item
                })
                .collect::<Vec<_>>();
            &individual
        } else {
            voice
        };
        draw_tablature_rhythm(
            page,
            &VoiceLayout {
                beats: rhythm_voice,
                xs: positions,
                times,
                meter: measure.time_signature,
                groups: &measure.beam_groups,
                group_unit: measure.beam_unit.unwrap_or(measure.time_signature.1),
            },
            beat_index,
            rhythm_y,
        )?;
    }
    Ok(())
}

/// Resolves rhythmic onsets to the horizontal centers of their measure columns.
fn compute_voice_positions(
    voice: &[Beat],
    plan: &MeasurePlan,
    measure_x: f32,
) -> Result<(Vec<f32>, Vec<f64>), RenderError> {
    let mut time = 0.0;
    let mut positions = Vec::with_capacity(voice.len());
    let mut times = Vec::with_capacity(voice.len());
    for beat in voice {
        time = beat.start.unwrap_or(time);
        let column = plan
            .columns
            .iter()
            .position(|entry| (entry.0 - time).abs() < TIMELINE_EPSILON)
            .expect("validated onset");
        positions.push(
            measure_x
                + plan.header
                + plan.columns[..column]
                    .iter()
                    .map(|entry| entry.1)
                    .sum::<f32>()
                + plan.columns[column].1 / 2.0,
        );
        times.push(time);
        time += beat.compute_quarter_beats()?;
    }
    Ok((positions, times))
}

/// Draws a tie, hammer-on/pull-off arc, or slide between two note positions.
fn draw_note_connection(page: &mut Scene, from: [f32; 2], to: [f32; 2], slide: bool, hammer: bool) {
    if slide {
        page.line(
            from[0],
            from[1] + SMALL_GLYPH_SIZE,
            to[0],
            to[1] - SMALL_GLYPH_SIZE,
            EMPHASIZED_STROKE_WIDTH,
        );
    } else {
        page.curve(from, to, NOTE_GLYPH_SIZE);
    }
    if hammer {
        page.text(
            (from[0] + to[0]) / 2.0,
            from[1] + SMALL_TEXT_SIZE + SMALL_GLYPH_SIZE,
            "H/P",
            SMALL_TEXT_SIZE,
            false,
        );
    }
}
