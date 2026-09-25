//! Opens the first track in a Guitar Pro file in a small native window.
//!
//! Run with `cargo run --example guitar_pro_viewer -- path/to/song.gp5`.

use std::{env, path::Path};

use alphatab_rs::{engrave, guitar_pro, DisplayMode, EguiInteraction, SceneOptions, Track};
use eframe::egui;

struct ViewerApp {
    // The source-independent track used by the renderer.
    track: Track,
    // Non-fatal omissions reported while converting the Guitar Pro source.
    warnings: Vec<String>,
}

impl eframe::App for ViewerApp {
    // This function is called by eframe every time the window needs repainting
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        // Show the imported track identity and any source-conversion warnings.
        ui.heading(&self.track.name);
        for warning in &self.warnings {
            ui.colored_label(egui::Color32::YELLOW, warning);
        }

        // Show a separator
        ui.separator();

        // Rebuild a scene for the current window width. This example always
        // shows staff notation above tablature and keeps a usable minimum width.
        let options = SceneOptions {
            width: ui.available_width().max(400.0),
            display: DisplayMode::Both,
            ..Default::default()
        };

        // Layout converts musical time and note positions into a paintable Scene.
        match engrave(&self.track, options) {
            Ok(scene) => {
                // Paint the scene in a scrollable region. The empty active-beat
                // list means this viewer has no playback cursor. Selection is
                // intentionally temporary in this minimal example.
                egui::ScrollArea::both().show(ui, |ui| {
                    let mut selection = None;
                    EguiInteraction::create_for_scene(&scene).handle(ui, &[], &mut selection);
                });
            }
            Err(error) => {
                ui.colored_label(egui::Color32::LIGHT_RED, error.to_string());
            }
        }
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Read the first argument after `--` as the Guitar Pro file to open.
    let path = env::args()
        .nth(1)
        .ok_or("usage: cargo run --example guitar_pro_viewer -- <file.gp5>")?;

    // Read and parse the file according to its .gp3/.gp4/.gp5/.gpx/.gp extension.
    let song = guitar_pro::load_guitar_pro_song(Path::new(&path))?;

    // This viewer displays only the first source track, then converts it to
    // the compact Track model used by layout and rendering.
    let source_track = song
        .tracks
        .first()
        .ok_or("the Guitar Pro file contains no tracks")?;
    let imported = guitar_pro::convert_track(&song, source_track)?;

    // Start a small native egui window and move the converted data into its app state.
    eframe::run_native(
        "Guitar Pro Viewer",
        eframe::NativeOptions {
            viewport: egui::ViewportBuilder::default().with_inner_size([760.0, 420.0]),
            ..Default::default()
        },
        Box::new(move |_creation_context| {
            Ok(Box::new(ViewerApp {
                track: imported.track,
                warnings: imported.warnings,
            }))
        }),
    )?;
    Ok(())
}
