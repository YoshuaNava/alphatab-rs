//! PNG backend for completed scenes.
use super::SvgRenderer;
use crate::{RenderError, Scene};

/// Resolution used by [`PngExporter::export`].
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
    fn compute_dimensions(self, scene: &Scene) -> Result<(u32, u32), RenderError> {
        if !self.scale.is_finite() || !(0.1..=16.0).contains(&self.scale) {
            return Err(RenderError::export(
                "export scale must be between 0.1 and 16",
            ));
        }
        let width = (scene.width * self.scale).ceil();
        let height = (scene.height * self.scale).ceil();
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

/// Produces PNG bytes from backend-neutral scene geometry.
#[derive(Clone, Copy, Debug, Default)]
pub struct PngExporter;

impl PngExporter {
    /// Rasterizes the SVG representation of `scene` to PNG bytes.
    pub fn export(scene: &Scene, options: RasterOptions) -> Result<Vec<u8>, RenderError> {
        let (width, height) = options.compute_dimensions(scene)?;
        let tree = parse_svg(&SvgRenderer::new(scene).render())?;
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
}
