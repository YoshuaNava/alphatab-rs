//! Construct a tablature study in code and render it to SVG.
use alphatab_rs::{engrave, Beat, Duration, Fret, SceneOptions, Measure, Note, SvgRenderer, Track};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Accept an optional output path so the example is convenient in scripts.
    let path = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "tablature.svg".into());

    // Build eight measures. Each has one voice containing eight eighth-note
    // beats: seven two-note shapes followed by a rest.
    let mut measures = Vec::new();
    for index in 0..8 {
        measures.push(Measure {
            voices: vec![(0..8)
                .map(|beat| Beat {
                    duration: Duration {
                        value: 8,
                        dots: 0,
                        tuplet: None,
                    },
                    // An empty note list represents a rest.
                    notes: if beat == 7 {
                        vec![]
                    } else {
                        vec![
                            Note {
                                string: 1 + beat % 3,
                                fret: Fret::Number((index + beat) as u16),
                                ..Default::default()
                            },
                            Note {
                                string: 6,
                                fret: Fret::Number(0),
                                ..Default::default()
                            },
                        ]
                    },
                    ..Default::default()
                })
                .collect()],
            repeat_start: index == 0,
            repeat_end: index == 7,
            ..Default::default()
        });
    }

    // Supply a name and high-to-low string tuning for tablature rendering.
    let track = Track {
        name: "alphatab-rs • Study in E".into(),
        strings: ["E", "B", "G", "D", "A", "E"].map(String::from).to_vec(),
        measures,
        ..Default::default()
    };

    // Default layout options select tablature; the resulting page can emit SVG.
    std::fs::write(
        path,
        SvgRenderer::new(&engrave(&track, SceneOptions::default())?).render(),
    )?;
    Ok(())
}
