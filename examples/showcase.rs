//! Build a one-measure guitar score containing many supported effects.
use alphatab_rs::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Start with eight eighth notes ascending chromatically on the first string.
    let mut notes: Vec<_> = (0..8)
        .map(|i| Beat {
            duration: Duration {
                value: 8,
                ..Default::default()
            },
            notes: vec![Note {
                string: 1,
                fret: Fret::Number(i),
                pitch: Some(Pitch::from_midi(64 + i as u8, false)),
                ..Default::default()
            }],
            ..Default::default()
        })
        .collect();

    // Attach a chord diagram to the first beat. Strings are ordered high to low,
    // matching the tuning below; `None` would mark a muted string.
    notes[0].annotations.chord = Some(ChordDiagram {
        name: "Em".into(),
        frets: vec![Some(0), Some(0), Some(0), Some(2), Some(2), Some(0)],
        first_fret: 1,
        barres: vec![Barre {
            fret: 2,
            first_string: 4,
            last_string: 5,
        }],
        ..Default::default()
    });

    // Add beat- and note-level techniques. Curves use normalized time (0..1)
    // and pitch offsets measured in semitones.
    notes[1].notes[0].effects.slide_in = Some(SlideDirection::Up);
    notes[2].notes[0].effects.slide_out = Some(SlideDirection::Down);
    notes[3].annotations.whammy = vec![[0.0, 0.0], [0.5, -2.0], [1.0, 0.0]];
    notes[4].annotations.technique = Some(PluckingTechnique::Tap);
    notes[5].annotations.strum_up = Some(false);
    notes[0].annotations.dynamic = Some("mf".into());
    notes[0].notes[0].effects.hammer_on = true;
    notes[1].notes[0].effects.slide = true;
    notes[2].notes[0].effects.vibrato = true;
    notes[3].notes[0].effects.bend = vec![[0.0, 0.0], [0.5, 2.0], [1.0, 0.0]];
    notes[4].notes[0].effects.grace_fret = Some(2);
    notes[5].notes[0].effects.harmonic = Some("N.H.".into());
    notes[6].notes[0].effects.palm_mute = true;
    // A beat with no notes is rendered as a rest.
    notes[7].notes.clear();

    // Put the beats into one voice and one repeating measure on a guitar track.
    let track = Track {
        name: "Engraving study".into(),
        strings: ["E", "B", "G", "D", "A", "E"].map(String::from).to_vec(),
        measures: vec![Measure {
            voices: vec![notes],
            tempo: Some(120),
            repeat_start: true,
            repeat_end: true,
            repeat_count: Some(3),
            ..Default::default()
        }],
        ..Default::default()
    };

    // Engrave standard notation and tablature together at a fixed content width.
    let page = engrave(
        &track,
        SceneOptions {
            display: DisplayMode::Both,
            width: 1100.0,
            ..Default::default()
        },
    )?;

    // Serialize the vector scene as SVG. The first argument overrides the
    // default output filename.
    std::fs::write(
        std::env::args()
            .nth(1)
            .unwrap_or_else(|| "showcase.svg".into()),
        page.to_svg(),
    )?;
    Ok(())
}
