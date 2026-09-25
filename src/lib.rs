//! Small, purpose-built Guitar Pro tablature and staff display for `music_gym`.
//!
//! The crate deliberately supports only the three views used by the application:
//! tablature, staff notation, and both together. It is not a general engraving
//! engine; Guitar Pro decorations outside this model are reported as warnings.
#![deny(missing_docs)]

/// An invalid compact score or scene-layout request.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RenderError(String);

impl std::fmt::Display for RenderError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for RenderError {}

mod track;
pub use track::{Bar, Beat, BeatAddress, Duration, Fret, Note, Track, Voice};

mod scene;
pub use scene::{engrave, DisplayMode, Scene, SceneOptions};

mod render;
pub use render::{EguiInteraction, Interaction, Selection};

/// Guitar Pro adapter for already parsed `guitarpro` model values.
pub mod guitar_pro;
