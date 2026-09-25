//! Shared fixed engraving parameters in scene-coordinate units.

/// Thin notation line width for staff lines, stems, and ordinary connectors.
pub(crate) const THIN_STROKE_WIDTH: f32 = 1.0;
/// Emphasized notation line width for barlines and effect strokes.
pub(crate) const EMPHASIZED_STROKE_WIDTH: f32 = 1.2;
/// Distance between adjacent staff lines.
pub(crate) const STAFF_LINE_SPACING: f32 = 10.0;
/// Height of a five-line staff.
pub(crate) const STAFF_HEIGHT: f32 = STAFF_LINE_SPACING * 4.0;
/// Vertical offset of the staff middle line from its top.
pub(crate) const STAFF_MIDDLE_LINE_OFFSET: f32 = STAFF_LINE_SPACING * 2.0;
/// Standard music-glyph scale used for ordinary noteheads and flags.
pub(crate) const NOTE_GLYPH_SIZE: f32 = 8.0;
/// Standard scale used for augmentation dots and small accidental glyphs.
pub(crate) const SMALL_GLYPH_SIZE: f32 = 7.0;
/// Text size for compact labels such as tuplet and effect markings.
pub(crate) const SMALL_TEXT_SIZE: f32 = 9.0;
/// Text size for ordinary notation labels.
pub(crate) const STANDARD_TEXT_SIZE: f32 = 10.0;
/// Text size for annotation labels.
pub(crate) const ANNOTATION_TEXT_SIZE: f32 = 11.0;
/// Text size for emphasized notation labels.
pub(crate) const EMPHASIZED_TEXT_SIZE: f32 = 12.0;
/// Text size for prominent labels.
pub(crate) const LARGE_TEXT_SIZE: f32 = 14.0;
/// Reusable system spacing shared by layout, metadata, and numbered notation.
pub(crate) const SYSTEM_LAYOUT: SystemLayout = SystemLayout {
    numbered_voice_spacing: 60.0,
    numbered_stack_spacing: 24.0,
    staff_note_clearance: 30.0,
    staff_to_tab_gap: 50.0,
    minimum_row_headroom: 90.0,
    annotation_headroom_padding: 20.0,
    span_lane_height: 18.0,
    maximum_span_lanes: 8,
    numbered_top_padding: 20.0,
    base_voice_spacing: 58.0,
    lyric_line_spacing: 14.0,
    effect_base_depth: 40.0,
    effect_string_depth_step: 16.0,
};

/// System-level layout geometry.
pub(crate) struct SystemLayout {
    pub(crate) numbered_voice_spacing: f32,
    pub(crate) numbered_stack_spacing: f32,
    pub(crate) staff_note_clearance: f32,
    pub(crate) staff_to_tab_gap: f32,
    pub(crate) minimum_row_headroom: f32,
    pub(crate) annotation_headroom_padding: f32,
    pub(crate) span_lane_height: f32,
    pub(crate) maximum_span_lanes: usize,
    pub(crate) numbered_top_padding: f32,
    pub(crate) base_voice_spacing: f32,
    pub(crate) lyric_line_spacing: f32,
    pub(crate) effect_base_depth: f32,
    pub(crate) effect_string_depth_step: f32,
}

/// Rhythm-stem, beam, and tuplet geometry.
pub(crate) const RHYTHM: RhythmParameters = RhythmParameters {
    compound_meter_denominator: 8,
    compound_meter_minimum_numerator: 3,
    compound_meter_group_beats: 1.5,
    quarter_beats_per_whole_note: 4.0,
    beamable_duration_value: 8,
    tuplet_rounding_epsilon: 1e-7,
};

/// Parameters that govern rhythmic notation geometry and grouping.
pub(crate) struct RhythmParameters {
    pub(crate) compound_meter_denominator: u16,
    pub(crate) compound_meter_minimum_numerator: u8,
    pub(crate) compound_meter_group_beats: f64,
    pub(crate) quarter_beats_per_whole_note: f64,
    pub(crate) beamable_duration_value: i16,
    pub(crate) tuplet_rounding_epsilon: f64,
}

/// Horizontal separation between successive augmentation dots.
pub(crate) const DOT_SPACING: f32 = 5.0;
/// Diatonic scale steps in one octave, used by staff and numbered notation.
pub(crate) const DIATONIC_STEPS_PER_OCTAVE: i16 = 7;
/// Tolerance for matching independently accumulated beat onsets.
pub(crate) const TIMELINE_EPSILON: f64 = 1e-8;
