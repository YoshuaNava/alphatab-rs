//! Portable SVG, PNG, and PDF export built from shared scene geometry.
use crate::Layout;
use crate::RenderError;

/// Resolution used by [`Layout::to_png`].
#[derive(Clone, Copy, Debug)]
pub struct RasterOptions {
    /// Multiplier applied to SVG user units. `1.0` corresponds to 96 DPI.
    pub scale: f32,
}

impl Default for RasterOptions {
    fn default() -> Self {
        Self { scale: 1.0 }
    }
}

impl RasterOptions {
    fn compute_dimensions(self, layout: &Layout) -> Result<(u32, u32), RenderError> {
        if !self.scale.is_finite() || !(0.1..=16.0).contains(&self.scale) {
            return Err(RenderError::export(
                "export scale must be between 0.1 and 16",
            ));
        }
        let width = (layout.width * self.scale).ceil();
        let height = (layout.height * self.scale).ceil();
        if !(1.0..=32768.0).contains(&width) || !(1.0..=32768.0).contains(&height) {
            return Err(RenderError::export(
                "export dimensions must be between 1 and 32768 pixels",
            ));
        }
        Ok((width as u32, height as u32))
    }
}

fn parse_svg(svg: &str) -> Result<resvg::usvg::Tree, RenderError> {
    let mut options = resvg::usvg::Options::default();
    options.fontdb_mut().load_system_fonts();
    resvg::usvg::Tree::from_str(svg, &options)
        .map_err(|error| RenderError::export(format!("could not parse generated SVG: {error}")))
}

impl Layout {
    /// Export SVG with ordinary text converted to font outlines.
    ///
    /// The outline shapes are selected from fonts installed on the exporting
    /// machine, so this removes downstream substitution but does not make
    /// source-font selection reproducible across machines.
    pub fn to_svg_outlined_text(&self) -> Result<String, RenderError> {
        let tree = parse_svg(&self.to_svg())?;
        Ok(tree.to_string(&resvg::usvg::WriteOptions {
            preserve_text: false,
            ..Default::default()
        }))
    }

    /// Rasterize the same SVG geometry used by [`Layout::to_svg`] to PNG bytes.
    pub fn to_png(&self, options: RasterOptions) -> Result<Vec<u8>, RenderError> {
        let (width, height) = options.compute_dimensions(self)?;
        let tree = parse_svg(&self.to_svg())?;
        let mut pixmap = resvg::tiny_skia::Pixmap::new(width, height)
            .ok_or_else(|| RenderError::export("could not allocate PNG bitmap"))?;
        resvg::render(
            &tree,
            resvg::tiny_skia::Transform::from_scale(options.scale, options.scale),
            &mut pixmap.as_mut(),
        );
        pixmap
            .encode_png()
            .map_err(|error| RenderError::export(format!("could not encode PNG: {error}")))
    }

    /// Convert the same SVG geometry used by [`Layout::to_svg`] to a vector PDF.
    /// Text is resolved through the exporting machine's installed fonts.
    pub fn to_pdf(&self) -> Result<Vec<u8>, RenderError> {
        let svg = self.to_svg();
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
