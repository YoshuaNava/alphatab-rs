//! Render a small General MIDI drum groove on a percussion staff.
use alphatab_rs::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Alternate kick/closed-hi-hat and snare/open-hi-hat pairs for eight beats.
    // `percussion::resolve` maps each General MIDI id to its written staff pitch.
    let beats = (0..8)
        .map(|i| {
            let ids = if i % 2 == 0 { [36, 42] } else { [38, 46] };
            let notes = ids
                .into_iter()
                .map(|id| {
                    Ok(Note {
                        percussion: Some(id),
                        pitch: Some(percussion::resolve(id)?.0),
                        ..Default::default()
                    })
                })
                .collect::<Result<Vec<_>, RenderError>>()?;
            Ok(Beat {
                duration: Duration {
                    value: 8,
                    ..Default::default()
                },
                notes,
                ..Default::default()
            })
        })
        .collect::<Result<Vec<_>, RenderError>>()?;

    // Use a percussion clef and demonstrate navigation and timed fermata marks.
    // Fermata positions are measured in quarter-note units within the measure.
    let track = Track {
        name: "Drum groove".into(),
        clef: Clef::Percussion,
        measures: vec![Measure {
            voices: vec![beats],
            navigation: Some(Navigation::Segno),
            fermatas: vec![Fermata {
                start: 3.5,
                kind: FermataKind::Normal,
            }],
            ..Default::default()
        }],
        ..Default::default()
    };

    // Justification expands the system to use the available layout width.
    let page = layout(
        &track,
        LayoutOptions {
            justify: true,
            ..Default::default()
        },
    )?;

    // Write SVG to the optional first argument or to percussion.svg.
    std::fs::write(
        std::env::args()
            .nth(1)
            .unwrap_or_else(|| "percussion.svg".into()),
        page.to_svg(),
    )?;
    Ok(())
}
