//! Geometry owned by backend-neutral output primitives.
use super::Primitive;

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
