//! Font-specific SMuFL data for the bundled Bravura font.
//!
//! Engraving code uses [`smufl::Glyph`] as its canonical symbol identity. This
//! module only translates Bravura's staff-space metadata into screen
//! coordinates; Unicode conversion belongs to the font outline loader.

use std::sync::OnceLock;

use smufl::{Glyph, Metadata};

/// Metadata-derived placement data cached with one rendered glyph.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct GlyphMetrics {
    /// Distance from this glyph's origin to the next one, in staff spaces.
    pub advance: Option<f32>,
    pub stem_up: Option<[f32; 2]>,
    pub stem_down: Option<[f32; 2]>,
}

/// Parses the bundled metadata once. The JSON and OTF are a matched pair, so
/// failure indicates a broken crate package.
pub(crate) fn metadata() -> &'static Metadata {
    static METADATA: OnceLock<Metadata> = OnceLock::new();
    METADATA.get_or_init(|| {
        Metadata::from_reader(include_bytes!("../assets/Bravura.json").as_slice())
            .expect("bundled Bravura metadata must be valid")
    })
}

/// Returns Bravura metrics with the SMuFL upward Y axis converted to screen Y.
pub(crate) fn metrics(glyph: Glyph) -> GlyphMetrics {
    let metadata = metadata();
    let anchors = metadata.anchors.get(glyph);
    let coord = |value: Option<smufl::Coord>| {
        value.map(|value| [f64::from(value.x()) as f32, -f64::from(value.y()) as f32])
    };
    GlyphMetrics {
        advance: metadata
            .advance_widths
            .get(glyph)
            .map(|value| f64::from(value) as f32),
        stem_up: anchors.and_then(|value| coord(value.stem_up_se)),
        stem_down: anchors.and_then(|value| coord(value.stem_down_nw)),
    }
}

/// Converts a recommended staff-space thickness to layout units.
pub(crate) fn thickness(value: Option<smufl::StaffSpaces>, staff_space: f32, fallback: f32) -> f32 {
    value.map_or(fallback, |value| f64::from(value) as f32 * staff_space)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loads_matching_bravura_metadata() {
        assert_eq!(metadata().font_name, "Bravura");
        let black = metrics(Glyph::NoteheadBlack);
        assert_eq!(black.stem_up, Some([1.18, -0.168]));
        assert_eq!(black.stem_down, Some([0.0, 0.168]));
        assert_eq!(Glyph::NoteheadBlack.codepoint(), '\u{E0A4}');
    }
}
