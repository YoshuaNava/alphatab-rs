//! Render the advanced notation regression showcase to SVG.
use alphatab_rs::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Build four measures of eighth notes, including score metadata and guitar
    // tuning. Supplying pitches lets the same notes appear on a standard staff.
    let mut track = Track {
        name: "Guitar".into(),
        strings: ["E", "B", "G", "D", "A", "E"].map(String::from).to_vec(),
        metadata: ScoreMetadata {
            title: "Notation and connections".into(),
            subtitle: "Beams · slurs · effects · condensed rests".into(),
            music: "alphatab-rs example".into(),
            ..Default::default()
        },
        measures: (0..4)
            .map(|mi| Measure {
                voices: vec![(0..4)
                    .map(|bi| Beat {
                        duration: Duration {
                            value: 8,
                            ..Default::default()
                        },
                        notes: vec![Note {
                            string: 1,
                            fret: Fret::Number((mi + bi) as u16),
                            pitch: Some(Pitch::spell_in_key((64 + mi + bi) as u8, 0)),
                            ..Default::default()
                        }],
                        ..Default::default()
                    })
                    .collect()],
                ..Default::default()
            })
            .collect(),
        ..Default::default()
    };

    // Add a chord diagram whose wide fingering also exercises diagram sizing.
    track.measures[0].voices[0][0].annotations.chord = Some(ChordDiagram {
        name: "Wide voicing".into(),
        first_fret: 1,
        frets: vec![Some(0), Some(9), Some(5), None, Some(3), Some(0)],
        fingers: ["", "4", "3", "", "1", ""].map(String::from).to_vec(),
        ..Default::default()
    });

    // Mark a p-to-f passage, with palm muting and crescendo on its first two bars.
    track.measures[0].voices[0][0].annotations.dynamic = Some("p".into());
    track.measures[1].voices[0][3].annotations.dynamic = Some("f".into());
    for mi in 0..2 {
        for beat in &mut track.measures[mi].voices[0] {
            beat.notes[0].effects.palm_mute = true;
            beat.annotations.crescendo = Some(true);
        }
    }

    // BeatAddress identifies endpoints independently of their current page or
    // system. Explicit spans may therefore connect beats in different measures.
    let a = |measure, beat| BeatAddress {
        measure,
        voice: 0,
        beat,
    };
    track.spans = vec![
        Span {
            start: a(0, 0),
            end: a(1, 3),
            kind: SpanKind::Slur {
                start_note: None,
                end_note: None,
            },
            placement: Placement::Above,
        },
        Span {
            start: a(2, 0),
            end: a(3, 3),
            kind: SpanKind::Beam,
            placement: Placement::Above,
        },
    ];

    // Add grace-note and bend data to individual notes. Bend points contain
    // normalized time followed by a semitone offset.
    track.measures[2].voices[0][1].notes[0].effects.grace_pitch = Some(Pitch::from_midi(66, false));
    track.measures[2].voices[0][1].notes[0].effects.grace_fret = Some(2);
    track.measures[2].voices[0][1].notes[0].effects.grace_slur = true;
    track.measures[2].voices[0][2].notes[0].effects.bend = vec![[0.0, 2.0], [0.5, 2.0], [1.0, 0.0]];

    // Append three whole-measure rests so multi-measure-rest condensation is visible.
    for _ in 0..3 {
        track.measures.push(Measure {
            voices: vec![vec![Beat {
                duration: Duration {
                    value: 1,
                    ..Default::default()
                },
                ..Default::default()
            }]],
            ..Default::default()
        });
    }

    // Render two bars per system with standard notation and tablature together.
    let page = engrave(
        &track,
        SceneOptions {
            width: 1100.0,
            bars_per_system: Some(2),
            display: DisplayMode::Both,
            multi_measure_rests: true,
            ..Default::default()
        },
    )?;

    // Write the SVG to the optional first argument or to advanced.svg.
    std::fs::write(
        std::env::args()
            .nth(1)
            .unwrap_or_else(|| "advanced.svg".into()),
        SvgRenderer::new(&page).render(),
    )?;
    Ok(())
}
