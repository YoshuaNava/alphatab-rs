//! Concrete adapters that paint or export backend-neutral scenes.
mod egui;
mod egui_interaction;
mod pdf;
mod png;
mod svg;

pub use egui::EguiRenderer;
pub use egui_interaction::{EguiInteraction, Interaction, Selection};
pub use pdf::PdfExporter;
pub use png::{PngExporter, RasterOptions};
pub use svg::SvgRenderer;
