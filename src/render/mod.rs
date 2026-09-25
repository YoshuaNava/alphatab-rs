//! egui painting and interaction for backend-neutral scenes.
mod egui;
mod egui_interaction;

pub(crate) use egui::EguiRenderer;
pub use egui_interaction::{EguiInteraction, Interaction, Selection};
