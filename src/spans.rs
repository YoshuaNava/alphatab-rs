//! Connections and effect bands are laid out after beat geometry is known.
mod drawing;
mod preparation;

use crate::*;
pub(crate) use drawing::draw;
pub(crate) use preparation::prepare;

pub(crate) fn contains(span: &Span, address: BeatAddress) -> bool {
    address.voice == span.start.voice
        && (address.measure, address.beat) >= (span.start.measure, span.start.beat)
        && (address.measure, address.beat) <= (span.end.measure, span.end.beat)
}

fn beat(track: &Track, address: BeatAddress) -> Result<&Beat, RenderError> {
    track
        .measures
        .get(address.measure)
        .and_then(|m| m.voices.get(address.voice))
        .and_then(|v| v.get(address.beat))
        .ok_or_else(|| RenderError::invalid_input(format!("invalid span endpoint {address:?}")))
}

#[derive(Clone, Copy)]
struct OutgoingLegato {
    address: BeatAddress,
    note: usize,
    fret: u16,
    hammer: bool,
    slide: bool,
}

/// A beat-local effect which becomes one contiguous span during preparation.
/// Keeping extraction and clearing together makes adding a new range effect a
/// single, reviewable change rather than another numeric branch in the pass.
#[derive(Clone, Copy)]
enum AutoRange {
    PalmMute,
    LetRing,
    Rasgueado,
    Ottava,
    Crescendo,
    Barre,
}

impl AutoRange {
    const ALL: [Self; 6] = [
        Self::PalmMute,
        Self::LetRing,
        Self::Rasgueado,
        Self::Ottava,
        Self::Crescendo,
        Self::Barre,
    ];

    /// Removes this local mark and returns the range kind it contributes.
    fn take(self, beat: &mut Beat) -> Option<SpanKind> {
        match self {
            Self::PalmMute => {
                let active = beat.notes.iter().any(|note| note.effects.palm_mute);
                for note in &mut beat.notes {
                    note.effects.palm_mute = false;
                }
                active.then_some(SpanKind::PalmMute)
            }
            Self::LetRing => {
                let active = beat.notes.iter().any(|note| note.effects.let_ring);
                for note in &mut beat.notes {
                    note.effects.let_ring = false;
                }
                active.then_some(SpanKind::LetRing)
            }
            Self::Rasgueado => beat
                .annotations
                .rasgueado
                .then_some(SpanKind::Rasgueado)
                .inspect(|_| {
                    beat.annotations.rasgueado = false;
                }),
            Self::Ottava => beat.annotations.ottava.take().map(SpanKind::Ottava),
            Self::Crescendo => beat.annotations.crescendo.take().map(|growing| {
                if growing {
                    SpanKind::Crescendo
                } else {
                    SpanKind::Diminuendo
                }
            }),
            Self::Barre => beat.annotations.barre.take().map(SpanKind::Text),
        }
    }
}

fn same_range_kind(left: &SpanKind, right: &SpanKind) -> bool {
    match (left, right) {
        (SpanKind::Ottava(a), SpanKind::Ottava(b)) => a == b,
        (SpanKind::Text(a), SpanKind::Text(b)) => a == b,
        (a, b) => std::mem::discriminant(a) == std::mem::discriminant(b),
    }
}

fn range_placement(kind: &SpanKind) -> Placement {
    if matches!(kind, SpanKind::Crescendo | SpanKind::Diminuendo) {
        Placement::Below
    } else {
        Placement::Above
    }
}

pub(crate) fn metadata(page: &mut Layout, track: &Track, options: LayoutOptions) -> f32 {
    let mut y = 48.0;
    if options.show_metadata {
        let m = &track.metadata;
        let authors = if m.words == m.music && !m.words.is_empty() {
            format!("Words and music: {}", m.words)
        } else {
            [
                (!m.words.is_empty()).then(|| format!("Words: {}", m.words)),
                (!m.music.is_empty()).then(|| format!("Music: {}", m.music)),
            ]
            .into_iter()
            .flatten()
            .collect::<Vec<_>>()
            .join(" · ")
        };
        for (visible, text, size) in [
            (options.elements.title, &m.title, 24.0),
            (options.elements.subtitle, &m.subtitle, 16.0),
            (options.elements.artist, &m.artist, 14.0),
            (options.elements.album, &m.album, 12.0),
            (
                options.elements.words || options.elements.music,
                &authors,
                12.0,
            ),
            (options.elements.copyright, &m.copyright, 10.0),
            (options.elements.instructions, &m.instructions, 11.0),
        ] {
            if !visible {
                continue;
            }
            for line in text.lines().filter(|l| !l.is_empty()) {
                let mut wrapped = String::new();
                for word in line.split_whitespace() {
                    let candidate = if wrapped.is_empty() {
                        word.into()
                    } else {
                        format!("{wrapped} {word}")
                    };
                    if crate::text::width(&candidate, size) > page.width - 64.0
                        && !wrapped.is_empty()
                    {
                        page.text(page.width / 2.0, y, &wrapped, size, false);
                        y += size + 8.0;
                        wrapped = word.into();
                    } else {
                        wrapped = candidate;
                    }
                }
                page.text(page.width / 2.0, y, &wrapped, size, false);
                y += size + 8.0;
            }
        }
    }
    if options.show_tuning && options.elements.tuning && !track.strings.is_empty() {
        page.text(
            page.width / 2.0,
            y,
            format!(
                "Tuning: {}",
                track
                    .strings
                    .iter()
                    .rev()
                    .cloned()
                    .collect::<Vec<_>>()
                    .join(" – ")
            ),
            11.0,
            false,
        );
        y += 20.0;
    }
    if options.elements.capo && track.capo > 0 {
        page.text(
            page.width / 2.0,
            y,
            format!("Capo: {}", track.capo),
            11.0,
            false,
        );
        y += 20.0;
    }
    y - 48.0
}

fn clef(track: &Track, measure: usize) -> Clef {
    track.measures[..=measure]
        .iter()
        .rev()
        .find_map(|m| m.clef)
        .unwrap_or(track.clef)
}
fn anchor(
    track: &Track,
    b: &BeatBounds,
    note: Option<usize>,
    staff: bool,
    above: bool,
    options: LayoutOptions,
) -> [f32; 2] {
    let beat = &track.measures[b.measure].voices[b.voice][b.beat];
    if options.display == DisplayMode::Numbered {
        let i = note.unwrap_or(if above {
            beat.notes.len().saturating_sub(1)
        } else {
            0
        });
        return [
            (b.cursor_rect[0] + b.cursor_rect[2]) / 2.0,
            b.cursor_rect[1] + 28.0 + b.voice as f32 * 60.0 - i as f32 * 24.0,
        ];
    }
    let y = |n: &Note| {
        if staff {
            crate::engrave::pitch_y(
                n.pitch.expect("validated pitch"),
                clef(track, b.measure),
                b.cursor_rect[1] + 8.0,
            )
        } else {
            b.cursor_rect[3]
                - 8.0
                - track.strings.len().saturating_sub(n.string) as f32 * options.string_spacing
        }
    };
    let yy = if let Some(n) = note {
        y(&beat.notes[n])
    } else {
        beat.notes
            .iter()
            .map(y)
            .reduce(if above { f32::min } else { f32::max })
            .unwrap_or(b.cursor_rect[1])
    };
    [
        (b.cursor_rect[0] + b.cursor_rect[2]) / 2.0
            + if staff {
                crate::engrave::voice_offset(&track.measures[b.measure], b.voice, b.start)
            } else {
                0.0
            },
        yy,
    ]
}
