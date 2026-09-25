//! Opens the first track in a Guitar Pro file in a small native window.
//!
//! Run with `cargo run --example guitar_pro_viewer -- path/to/song.gp5`.

use std::{env, path::Path};

use alphatab_rs::{engrave, guitar_pro, DisplayMode, EguiInteraction, SceneOptions, Track};
use eframe::egui;

/// Loads one Guitar Pro file using the parser selected by its filename extension.
fn load_guitar_pro_song(path: &Path) -> Result<guitarpro::Song, Box<dyn std::error::Error>> {
    let extension = path
        .extension()
        .and_then(|extension| extension.to_str())
        .map(str::to_ascii_lowercase)
        .ok_or("the tab file needs a Guitar Pro extension")?;
    let data = std::fs::read(path)?;
    let mut song = guitarpro::Song::default();
    match extension.as_str() {
        "gp3" => song.read_gp3(&data)?,
        "gp4" => song.read_gp4(&data)?,
        "gp5" => song.read_gp5(&data)?,
        "gpx" => song.read_gpx(&data)?,
        "gp" => song.read_gp(&data)?,
        _ => return Err(format!("unsupported Guitar Pro extension .{extension}").into()),
    }
    Ok(song)
}

struct ViewerApp {
    track: Track,
    warnings: Vec<String>,
}

impl eframe::App for ViewerApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        ui.heading(&self.track.name);
        for warning in &self.warnings {
            ui.colored_label(egui::Color32::YELLOW, warning);
        }
        ui.separator();

        let options = SceneOptions {
            width: ui.available_width().max(400.0),
            display: DisplayMode::Both,
            ..Default::default()
        };
        match engrave(&self.track, options) {
            Ok(scene) => {
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
    let path = env::args()
        .nth(1)
        .ok_or("usage: cargo run --example guitar_pro_viewer -- <file.gp5>")?;
    let song = load_guitar_pro_song(Path::new(&path))?;
    let source_track = song
        .tracks
        .first()
        .ok_or("the Guitar Pro file contains no tracks")?;
    let imported = guitar_pro::convert_track(&song, source_track)?;

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
