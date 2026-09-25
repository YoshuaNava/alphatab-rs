//! Shared fixed engraving parameters in scene-coordinate units.

/// Thin notation line width for staff lines, stems, and ordinary connectors.
pub(super) const THIN_STROKE_WIDTH: f32 = 1.0;
/// Emphasized notation line width for barlines and effect strokes.
pub(super) const EMPHASIZED_STROKE_WIDTH: f32 = 1.2;
/// Distance between adjacent staff lines.
pub(super) const STAFF_LINE_SPACING: f32 = 10.0;
/// Height of a five-line staff.
pub(super) const STAFF_HEIGHT: f32 = STAFF_LINE_SPACING * 4.0;
/// Vertical offset of the staff middle line from its top.
pub(super) const STAFF_MIDDLE_LINE_OFFSET: f32 = STAFF_LINE_SPACING * 2.0;
/// Standard music-glyph scale used for ordinary noteheads and flags.
pub(super) const NOTE_GLYPH_SIZE: f32 = 8.0;
/// Standard scale used for augmentation dots and small accidental glyphs.
pub(super) const SMALL_GLYPH_SIZE: f32 = 7.0;
/// Horizontal separation between successive augmentation dots.
pub(super) const DOT_SPACING: f32 = 5.0;
/// Diatonic scale steps in one octave, used by staff and numbered notation.
pub(super) const DIATONIC_STEPS_PER_OCTAVE: i16 = 7;
/// Tolerance for matching independently accumulated beat onsets.
pub(super) const TIMELINE_EPSILON: f64 = 1e-8;
