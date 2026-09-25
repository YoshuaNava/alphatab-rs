//! SVG backend for completed scenes.
use crate::{Color, Primitive, RenderError, Scene};
use std::fmt::Write;

const SMUFL_CODEPOINT_HEX_WIDTH: usize = 4;
const OPAQUE_ALPHA: u8 = 255;
const ALPHA_NORMALIZATION_DENOMINATOR: f32 = 255.0;
const ALPHA_DECIMAL_PLACES: usize = 3;
const MASK_HORIZONTAL_PADDING: f32 = 6.0;
const MASK_ASCENT_FRACTION: f32 = 0.6;
const MASK_HEIGHT_FACTOR: f32 = 1.2;

impl Color {
    pub(crate) fn svg(self) -> String {
        if self.a == OPAQUE_ALPHA {
            format!("rgb({} {} {})", self.r, self.g, self.b)
        } else {
            format!(
                "rgba({} {} {} / {:.precision$})",
                self.r,
                self.g,
                self.b,
                f32::from(self.a) / ALPHA_NORMALIZATION_DENOMINATOR,
                precision = ALPHA_DECIMAL_PLACES,
            )
        }
    }
}

/// SVG adapter for a completed backend-neutral [`Scene`].
#[derive(Clone, Copy, Debug)]
pub struct SvgRenderer<'scene> {
    scene: &'scene Scene,
}

impl<'scene> SvgRenderer<'scene> {
    /// Creates an SVG adapter for `scene`.
    pub fn new(scene: &'scene Scene) -> Self {
        Self { scene }
    }

    /// Serializes the shared primitive geometry to SVG.
    pub fn render(&self) -> String {
        let mut svg = format!("<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{}\" height=\"{}\" viewBox=\"0 0 {} {}\"><rect width=\"100%\" height=\"100%\" fill=\"{}\"/>", self.width, self.height, self.width, self.height, self.style.background.svg());
        for primitive in &self.primitives {
            match primitive {
                Primitive::Glyph {
                    at,
                    space,
                    outline,
                    code,
                    color,
                } => {
                    write!(svg, "<path data-smufl=\"{:0width$X}\" transform=\"translate({} {}) scale({})\" d=\"{}\" fill=\"{}\"/>", *code as u32, at[0], at[1], space, outline.svg, color.unwrap_or(self.style.glyph_color()).svg(), width = SMUFL_CODEPOINT_HEX_WIDTH).unwrap();
                }

                Primitive::Line {
                    from,
                    to,
                    width,
                    color,
                } => {
                    write!(svg, "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"{}\" stroke-width=\"{}\"/>", from[0], from[1], to[0], to[1], color.unwrap_or(self.style.line_color()).svg(), width).unwrap();
                }
                Primitive::Curve {
                    points,
                    width,
                    color,
                } => {
                    write!(svg, "<path d=\"M{} {} C{} {},{} {},{} {}\" fill=\"none\" stroke=\"{}\" stroke-width=\"{}\"/>", points[0][0], points[0][1], points[1][0], points[1][1], points[2][0], points[2][1], points[3][0], points[3][1], color.unwrap_or(self.style.line_color()).svg(), width).unwrap();
                }
                Primitive::Text {
                    at,
                    text,
                    size,
                    masked,
                    color,
                } => {
                    if *masked {
                        let w = crate::text::width(text, *size) + MASK_HORIZONTAL_PADDING;
                        write!(
                            svg,
                            "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" fill=\"{}\"/>",
                            at[0] - w / 2.0,
                            at[1] - size * MASK_ASCENT_FRACTION,
                            w,
                            size * MASK_HEIGHT_FACTOR,
                            self.style.background.svg()
                        )
                        .unwrap();
                    }
                    write!(svg, "<text x=\"{}\" y=\"{}\" font-family=\"sans-serif\" font-size=\"{}\" textLength=\"{}\" lengthAdjust=\"spacingAndGlyphs\" text-anchor=\"middle\" dominant-baseline=\"central\" fill=\"{}\">{}</text>", at[0], at[1], size, crate::text::width(text, *size), color.unwrap_or(self.style.text_color()).svg(), escape_xml(text)).unwrap();
                }
            }
        }
        svg.push_str("</svg>");
        svg
    }
}

impl std::ops::Deref for SvgRenderer<'_> {
    type Target = Scene;

    fn deref(&self) -> &Self::Target {
        self.scene
    }
}

impl SvgRenderer<'_> {
    /// Renders SVG with ordinary text converted to font outlines.
    ///
    /// The outline shapes are selected from fonts installed on the exporting
    /// machine, so this removes downstream substitution but does not make
    /// source-font selection reproducible across machines.
    pub fn render_outlined_text(&self) -> Result<String, RenderError> {
        let mut options = resvg::usvg::Options::default();
        options.fontdb_mut().load_system_fonts();
        let tree = resvg::usvg::Tree::from_str(&self.render(), &options).map_err(|error| {
            RenderError::export(format!("could not parse generated SVG: {error}"))
        })?;
        Ok(tree.to_string(&resvg::usvg::WriteOptions {
            preserve_text: false,
            ..Default::default()
        }))
    }
}

fn escape_xml(text: &str) -> String {
    text.chars().map(|c| if matches!(c, '\t' | '\n' | '\r' | '\u{20}'..='\u{D7FF}' | '\u{E000}'..='\u{FFFD}' | '\u{10000}'..='\u{10FFFF}') { c } else { '\u{FFFD}' }).collect::<String>().replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}
