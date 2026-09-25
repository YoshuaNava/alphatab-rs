//! Load with guitarpro and export one selected track. Usage: render_gp FILE OUT [TRACK] [tab|staff|both|numbered|slash]
use alphatab_rs::{engrave, guitar_pro::convert_track, DisplayMode, SceneOptions};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Require an input Guitar Pro file and an output SVG path.
    let args: Vec<_> = std::env::args().collect();
    if args.len() < 3 {
        return Err(
            "usage: render_gp INPUT OUTPUT.svg [TRACK_INDEX] [tab|staff|both|numbered|slash]"
                .into(),
        );
    }

    // The guitarpro crate owns file parsing. Select its parser from the suffix.
    let data = std::fs::read(&args[1])?;
    let mut song = guitarpro::Song::default();
    match std::path::Path::new(&args[1])
        .extension()
        .and_then(|e| e.to_str())
        .map(str::to_ascii_lowercase)
        .as_deref()
    {
        Some("gp3") => song.read_gp3(&data)?,
        Some("gp4") => song.read_gp4(&data)?,
        Some("gp5") => song.read_gp5(&data)?,
        Some("gpx") => song.read_gpx(&data)?,
        Some("gp") => song.read_gp(&data)?,
        _ => return Err("unsupported file extension".into()),
    }

    // Select a source track (zero by default), then adapt guitarpro's model to
    // alphatab-rs's rendering model. Recoverable import losses are warnings.
    let index: usize = args.get(3).map(|s| s.parse()).transpose()?.unwrap_or(0);
    let source = song.tracks.get(index).ok_or("track index out of range")?;
    let imported = convert_track(&song, source)?;
    for warning in imported.warnings {
        eprintln!("Warning: {warning}");
    }

    // Choose which notation stave(s) to engrave from the optional fifth argument.
    let display = match args.get(4).map(String::as_str).unwrap_or("tab") {
        "tab" => DisplayMode::Tablature,
        "staff" => DisplayMode::Standard,
        "both" => DisplayMode::Both,
        "numbered" => DisplayMode::Numbered,
        "slash" => DisplayMode::Slash,
        _ => return Err("display must be tab, staff, both, numbered or slash".into()),
    };

    // Lay out the imported track, convert its vector primitives to SVG, and save it.
    std::fs::write(
        &args[2],
        engrave(
            &imported.track,
            SceneOptions {
                display,
                ..Default::default()
            },
        )?
        .to_svg(),
    )?;
    Ok(())
}
