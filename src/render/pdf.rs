//! PDF backend for completed scenes.
use super::SvgRenderer;
use crate::{RenderError, Scene};

/// Produces vector PDF bytes from backend-neutral scene geometry.
#[derive(Clone, Copy, Debug, Default)]
pub struct PdfExporter;

impl PdfExporter {
    /// Converts the SVG representation of `scene` to a vector PDF.
    /// Text is resolved through the exporting machine's installed fonts.
    pub fn export(scene: &Scene) -> Result<Vec<u8>, RenderError> {
        let svg = SvgRenderer::new(scene).render();
        let mut options = svg2pdf::usvg::Options::default();
        options.fontdb_mut().load_system_fonts();
        let tree = svg2pdf::usvg::Tree::from_str(&svg, &options).map_err(|error| {
            RenderError::export(format!("could not parse generated SVG for PDF: {error}"))
        })?;
        svg2pdf::to_pdf(
            &tree,
            svg2pdf::ConversionOptions::default(),
            svg2pdf::PageOptions::default(),
        )
        .map_err(|error| RenderError::export(format!("could not create PDF: {error}")))
    }
}
