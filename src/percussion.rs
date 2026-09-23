//! Percussion staff positions and SMuFL noteheads from the factual articulation
//! table in CoderLine/alphaTab, revision 0a3af85833133a0e450223ae286fb681dea8bdda.

use crate::{MusicGlyph as G, Pitch, RenderError};

/// Resolves a General MIDI percussion id to a written pitch and named SMuFL
/// noteheads for quarter, half, and whole durations.
pub fn resolve(id: u16) -> Result<(Pitch, [G; 3]), RenderError> {
    let normal = [G::NoteheadBlack, G::NoteheadHalf, G::NoteheadWhole];
    let cross = [G::NoteheadXBlack; 3];
    let cross_durations = [G::NoteheadXBlack, G::NoteheadXHalf, G::NoteheadXWhole];
    let triangle = [
        G::NoteheadTriangleUpBlack,
        G::NoteheadTriangleUpHalf,
        G::NoteheadTriangleUpWhole,
    ];
    let diamond = [G::NoteheadDiamondWhite; 3];
    let (line, heads) = match id {
        38..=40 => (3, normal), // Snare, clap
        37 => (3, cross),       // Side stick
        91 => (3, diamond),     // Snare rim
        42 => (-1, cross),      // Closed hi-hat
        92 => (-1, [G::NoteheadCircleSlash; 3]),
        46 => (-1, [G::NoteheadCircleX; 3]),
        44 => (9, cross),
        35 => (8, normal),
        36 => (7, normal),
        50 => (1, normal),
        48 => (2, normal),
        47 => (4, normal),
        45 | 41 => (5, normal),
        43 => (6, normal),
        93 | 51 | 94 => (0, cross),
        53 => (0, diamond),
        55 | 95 => (-2, cross),
        52 | 96 => (-3, [G::NoteheadHeavyXHat; 3]),
        49 | 97 => (-2, [G::NoteheadHeavyX; 3]),
        57 | 98 => (-1, [G::NoteheadHeavyX; 3]),
        99 => (1, triangle),
        100 => (1, cross_durations),
        56 => (0, triangle),
        101 => (0, cross_durations),
        102 => (-1, triangle),
        103 => (-1, cross_durations),
        77 => (-9, [G::NoteheadTriangleUpBlack; 3]),
        76 => (-10, [G::NoteheadTriangleUpBlack; 3]),
        60 => (-4, normal),
        104 => (-5, normal),
        105 => (-6, cross),
        61 => (-7, normal),
        106 => (-8, normal),
        107 => (-16, cross),
        66 => (10, normal),
        65 => (9, normal),
        68 => (12, normal),
        67 => (11, normal),
        64 => (17, normal),
        108 => (16, cross),
        109 => (15, normal),
        63 => (14, normal),
        110 => (13, cross),
        62 => (19, normal),
        72 => (-11, normal),
        71 => (-17, normal),
        73 => (38, normal),
        74 => (37, normal),
        86 => (36, normal),
        87 => (35, cross),
        54 => (3, [G::NoteheadTriangleUpBlack; 3]),
        111 => (2, [G::NoteheadTriangleUpBlack; 3]),
        112 => (1, [G::NoteheadTriangleUpBlack; 3]),
        113 => (-7, cross),
        79 => (30, normal),
        78 => (29, cross),
        58 => (28, normal),
        81 => (27, normal),
        80 => (26, cross),
        114 => (25, normal),
        115 => (18, normal),
        116 => (24, cross),
        69 => (23, normal),
        117 => (22, normal),
        85 => (21, normal),
        75 => (20, normal),
        70 => (-12, normal),
        118 => (-13, normal),
        119 => (-14, normal),
        120 => (-15, normal),
        82 => (-23, normal),
        122 => (-24, normal),
        84 => (-18, normal),
        123 => (-19, normal),
        83 => (-20, normal),
        124 => (-21, [G::NoteheadNull; 3]),
        125 => (-22, [G::NoteheadNull; 3]),
        31 => (3, [G::NoteheadSlashedBlack2; 3]),
        59 | 126 | 29 => (2, cross),
        127 => (2, diamond),
        30 => (-3, cross),
        33 => (3, cross),
        34 => (3, [G::NoteheadBlack; 3]),
        _ => {
            return Err(RenderError::invalid_input(format!(
                "unsupported percussion articulation {id}"
            )))
        }
    };
    let step = 38_i16 - line;
    Ok((
        Pitch {
            step: step.rem_euclid(7) as u8,
            octave: step.div_euclid(7) as i8,
            accidental: 0,
        },
        heads,
    ))
}
