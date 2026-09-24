//! Backend-neutral drawing primitives and their geometry.
use super::Color;

#[derive(Clone, Debug)]
/// One drawing command in a completed scene.
pub enum Primitive {
    /// A tessellated SMuFL glyph outline.
    Glyph {
        /// Glyph origin in scene coordinates.
        at: [f32; 2],
        /// Staff-space scale used to draw the glyph.
        space: f32,
        /// SMuFL character corresponding to the glyph.
        code: char,
        /// Loaded vector outline for the glyph.
        outline: std::sync::Arc<crate::glyph::Glyph>,
        /// Optional color override.
        color: Option<Color>,
    },
    /// A straight stroked segment.
    Line {
        /// Start point in scene coordinates.
        from: [f32; 2],
        /// End point in scene coordinates.
        to: [f32; 2],
        /// Stroke width in scene units.
        width: f32,
        /// Optional color override.
        color: Option<Color>,
    },
    /// A cubic Bézier stroke.
    Curve {
        /// The four cubic Bézier control points.
        points: [[f32; 2]; 4],
        /// Stroke width in scene units.
        width: f32,
        /// Optional color override.
        color: Option<Color>,
    },
    /// Centered text, optionally with a background mask.
    Text {
        /// Center point in scene coordinates.
        at: [f32; 2],
        /// Text content.
        text: String,
        /// Font size in scene units.
        size: f32,
        /// Whether to paint a background mask first.
        masked: bool,
        /// Optional color override.
        color: Option<Color>,
    },
}

impl Primitive {
    /// Axis-aligned bounds used for clipping and collision-aware span routing.
    pub(crate) fn compute_bounds(&self) -> [f32; 4] {
        match self {
            Self::Text { at, text, size, .. } => {
                let width = crate::text::width(text, *size) / 2.0;
                [
                    at[0] - width,
                    at[1] - size / 2.0,
                    at[0] + width,
                    at[1] + size / 2.0,
                ]
            }
            Self::Glyph {
                at, space, outline, ..
            } => {
                let bounds = outline.bounds;
                [
                    at[0] + bounds[0] * space,
                    at[1] + bounds[1] * space,
                    at[0] + bounds[2] * space,
                    at[1] + bounds[3] * space,
                ]
            }
            Self::Line {
                from, to, width, ..
            } => [
                from[0].min(to[0]) - width / 2.0,
                from[1].min(to[1]) - width / 2.0,
                from[0].max(to[0]) + width / 2.0,
                from[1].max(to[1]) + width / 2.0,
            ],
            Self::Curve { points, width, .. } => [
                points
                    .iter()
                    .map(|point| point[0])
                    .fold(f32::INFINITY, f32::min)
                    - width / 2.0,
                points
                    .iter()
                    .map(|point| point[1])
                    .fold(f32::INFINITY, f32::min)
                    - width / 2.0,
                points
                    .iter()
                    .map(|point| point[0])
                    .fold(f32::NEG_INFINITY, f32::max)
                    + width / 2.0,
                points
                    .iter()
                    .map(|point| point[1])
                    .fold(f32::NEG_INFINITY, f32::max)
                    + width / 2.0,
            ],
        }
    }
}
