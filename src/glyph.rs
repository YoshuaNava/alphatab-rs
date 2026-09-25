//! Bravura outlines shared by the egui and SVG backends. Glyphs are tessellated
//! once at staff-space units; holes retain the font's nonzero winding rule.
use crate::{music_font::GlyphMetrics, RenderError};
use lyon_tessellation::{
    path::{math::point, Path},
    BuffersBuilder, FillOptions, FillTessellator, FillVertex, VertexBuffers,
};
use std::{
    collections::HashMap,
    fmt::Write,
    sync::{Arc, Mutex, OnceLock},
};

const FONT_COLLECTION_INDEX: u32 = 0;
const SCENE_UNITS_PER_STAFF_SPACE: f32 = 4.0;
const TESSELLATION_TOLERANCE: f32 = 0.002;

#[derive(Debug)]
pub struct Glyph {
    pub codepoint: char,
    pub bounds: [f32; 4],
    pub(crate) metrics: GlyphMetrics,
    pub vertices: Vec<[f32; 2]>,
    pub indices: Vec<u32>,
    pub svg: String,
}

impl Glyph {
    /// Returns bounds translated so their visible center lies at the origin.
    pub(crate) fn centered_bounds(&self, scale: f32) -> [f32; 4] {
        let center_x = (self.bounds[0] + self.bounds[2]) / 2.0;
        let center_y = (self.bounds[1] + self.bounds[3]) / 2.0;
        [
            (self.bounds[0] - center_x) * scale,
            (self.bounds[1] - center_y) * scale,
            (self.bounds[2] - center_x) * scale,
            (self.bounds[3] - center_y) * scale,
        ]
    }

    /// Converts a visible-center anchor into the glyph's SMuFL origin.
    pub(crate) fn origin_at_center(&self, center: [f32; 2], scale: f32) -> [f32; 2] {
        [
            center[0] - (self.bounds[0] + self.bounds[2]) * scale / 2.0,
            center[1] - (self.bounds[1] + self.bounds[3]) * scale / 2.0,
        ]
    }
}
struct Outline {
    builder: lyon_tessellation::path::path::Builder,
    svg: String,
    open: bool,
    scale: f32,
}
impl ttf_parser::OutlineBuilder for Outline {
    fn move_to(&mut self, x: f32, y: f32) {
        if self.open {
            self.builder.end(false);
        }
        self.builder.begin(point(x * self.scale, -y * self.scale));
        write!(self.svg, "M{} {}", x * self.scale, -y * self.scale).unwrap();
        self.open = true;
    }
    fn line_to(&mut self, x: f32, y: f32) {
        self.builder.line_to(point(x * self.scale, -y * self.scale));
        write!(self.svg, "L{} {}", x * self.scale, -y * self.scale).unwrap();
    }
    fn quad_to(&mut self, x: f32, y: f32, x2: f32, y2: f32) {
        self.builder.quadratic_bezier_to(
            point(x * self.scale, -y * self.scale),
            point(x2 * self.scale, -y2 * self.scale),
        );
        write!(
            self.svg,
            "Q{} {} {} {}",
            x * self.scale,
            -y * self.scale,
            x2 * self.scale,
            -y2 * self.scale
        )
        .unwrap();
    }
    fn curve_to(&mut self, x: f32, y: f32, x2: f32, y2: f32, x3: f32, y3: f32) {
        self.builder.cubic_bezier_to(
            point(x * self.scale, -y * self.scale),
            point(x2 * self.scale, -y2 * self.scale),
            point(x3 * self.scale, -y3 * self.scale),
        );
        write!(
            self.svg,
            "C{} {} {} {} {} {}",
            x * self.scale,
            -y * self.scale,
            x2 * self.scale,
            -y2 * self.scale,
            x3 * self.scale,
            -y3 * self.scale
        )
        .unwrap();
    }
    fn close(&mut self) {
        self.builder.end(true);
        self.svg.push('Z');
        self.open = false;
    }
}

pub fn load(symbol: smufl::Glyph) -> Result<Arc<Glyph>, RenderError> {
    static CACHE: OnceLock<Mutex<HashMap<smufl::Glyph, Arc<Glyph>>>> = OnceLock::new();
    let mut cache = CACHE
        .get_or_init(Default::default)
        .lock()
        .map_err(|_| RenderError::internal("music glyph cache poisoned"))?;
    if let Some(glyph) = cache.get(&symbol) {
        return Ok(glyph.clone());
    }
    let code = symbol.codepoint();
    let face = ttf_parser::Face::parse(
        include_bytes!("../assets/Bravura.otf"),
        FONT_COLLECTION_INDEX,
    )
    .map_err(|e| RenderError::internal(format!("invalid bundled font: {e:?}")))?;
    let id = face.glyph_index(code).ok_or_else(|| {
        RenderError::internal(format!("missing music glyph U+{:04X}", code as u32))
    })?;
    let mut outline = Outline {
        builder: Path::builder(),
        svg: String::new(),
        open: false,
        scale: SCENE_UNITS_PER_STAFF_SPACE / f32::from(face.units_per_em()),
    };
    face.outline_glyph(id, &mut outline)
        .ok_or_else(|| RenderError::internal("music glyph has no outline"))?;
    if outline.open {
        outline.builder.end(false);
    }
    let path = outline.builder.build();
    let mut buffers: VertexBuffers<[f32; 2], u32> = VertexBuffers::new();
    FillTessellator::new()
        .tessellate_path(
            &path,
            &FillOptions::default().with_tolerance(TESSELLATION_TOLERANCE),
            &mut BuffersBuilder::new(&mut buffers, |v: FillVertex<'_>| v.position().to_array()),
        )
        .map_err(|e| RenderError::internal(format!("music glyph tessellation failed: {e:?}")))?;
    let bounds = buffers.vertices.iter().fold(
        [
            f32::INFINITY,
            f32::INFINITY,
            f32::NEG_INFINITY,
            f32::NEG_INFINITY,
        ],
        |b, v| {
            [
                b[0].min(v[0]),
                b[1].min(v[1]),
                b[2].max(v[0]),
                b[3].max(v[1]),
            ]
        },
    );
    // Visible alignment uses the geometry that both backends actually draw.
    // SMuFL metadata remains authoritative for semantic attachment anchors,
    // but substituting its bounding box here creates a second visual geometry
    // model and makes any metadata/outline discrepancy appear as an offset.
    let metrics = crate::music_font::metrics(symbol);
    let glyph = Arc::new(Glyph {
        codepoint: code,
        bounds,
        metrics,
        vertices: buffers.vertices,
        indices: buffers.indices,
        svg: outline.svg,
    });
    cache.insert(symbol, glyph.clone());
    Ok(glyph)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cache_keeps_outline_and_metadata_together() {
        let first = load(smufl::Glyph::NoteheadBlack).unwrap();
        let second = load(smufl::Glyph::NoteheadBlack).unwrap();
        assert!(Arc::ptr_eq(&first, &second));
        assert_eq!(first.codepoint, '\u{E0A4}');
        assert_eq!(first.metrics.stem_up, Some([1.18, -0.168]));
    }
}
