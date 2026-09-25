//! Connections and effect bands are laid out after beat geometry is known.
mod drawing;
mod preparation;

use crate::scene::parameters::{
    ANNOTATION_TEXT_SIZE, EMPHASIZED_TEXT_SIZE, LARGE_TEXT_SIZE, NOTE_GLYPH_SIZE, SPAN_LAYOUT,
    STAFF_MIDDLE_LINE_OFFSET, STANDARD_TEXT_SIZE, SYSTEM_LAYOUT,
};
use crate::{
    Beat, BeatAddress, BeatBounds, Clef, DisplayMode, Note, Placement, RenderError, Scene,
    SceneOptions, Span, SpanKind, Track,
};

pub(crate) use drawing::draw;
pub(crate) use preparation::{prepare, RenderState};

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

/// A beat-local effect which becomes one contiguous span during preparation.
/// Keeping extraction and clearing together makes adding a new range effect a
/// single, reviewable change rather than another numeric branch in the pass.
#[derive(Clone, Copy)]
pub(crate) enum AutoRange {
    PalmMute,
    LetRing,
    Rasgueado,
    Ottava,
    Crescendo,
    Barre,
}

impl AutoRange {
    pub(crate) const ALL: [Self; 6] = [
        Self::PalmMute,
        Self::LetRing,
        Self::Rasgueado,
        Self::Ottava,
        Self::Crescendo,
        Self::Barre,
    ];

    /// Reads the local mark which contributes to this derived range.
    pub(crate) fn kind(self, beat: &Beat) -> Option<SpanKind> {
        match self {
            Self::PalmMute => {
                let active = beat.notes.iter().any(|note| note.effects.palm_mute);
                active.then_some(SpanKind::PalmMute)
            }
            Self::LetRing => {
                let active = beat.notes.iter().any(|note| note.effects.let_ring);
                active.then_some(SpanKind::LetRing)
            }
            Self::Rasgueado => beat.annotations.rasgueado.then_some(SpanKind::Rasgueado),
            Self::Ottava => beat.annotations.ottava.map(SpanKind::Ottava),
            Self::Crescendo => beat.annotations.crescendo.map(|growing| {
                if growing {
                    SpanKind::Crescendo
                } else {
                    SpanKind::Diminuendo
                }
            }),
            Self::Barre => beat.annotations.barre.clone().map(SpanKind::Text),
        }
    }
}

pub(crate) fn same_range_kind(left: &SpanKind, right: &SpanKind) -> bool {
    match (left, right) {
        (SpanKind::Ottava(a), SpanKind::Ottava(b)) => a == b,
        (SpanKind::Text(a), SpanKind::Text(b)) => a == b,
        (a, b) => std::mem::discriminant(a) == std::mem::discriminant(b),
    }
}

pub(crate) fn range_placement(kind: &SpanKind) -> Placement {
    if matches!(kind, SpanKind::Crescendo | SpanKind::Diminuendo) {
        Placement::Below
    } else {
        Placement::Above
    }
}

pub(crate) fn metadata(page: &mut Scene, track: &Track, options: SceneOptions) -> f32 {
    let mut y = SPAN_LAYOUT.metadata_start_y;
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
            (
                options.elements.title,
                &m.title,
                SPAN_LAYOUT.title_text_size,
            ),
            (
                options.elements.subtitle,
                &m.subtitle,
                SPAN_LAYOUT.subtitle_text_size,
            ),
            (options.elements.artist, &m.artist, LARGE_TEXT_SIZE),
            (options.elements.album, &m.album, EMPHASIZED_TEXT_SIZE),
            (
                options.elements.words || options.elements.music,
                &authors,
                EMPHASIZED_TEXT_SIZE,
            ),
            (options.elements.copyright, &m.copyright, STANDARD_TEXT_SIZE),
            (
                options.elements.instructions,
                &m.instructions,
                ANNOTATION_TEXT_SIZE,
            ),
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
                    if crate::text::width(&candidate, size)
                        > page.width - SYSTEM_LAYOUT.content_horizontal_inset
                        && !wrapped.is_empty()
                    {
                        page.text(page.width / 2.0, y, &wrapped, size, false);
                        y += size + NOTE_GLYPH_SIZE;
                        wrapped = word.into();
                    } else {
                        wrapped = candidate;
                    }
                }
                page.text(page.width / 2.0, y, &wrapped, size, false);
                y += size + NOTE_GLYPH_SIZE;
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
            ANNOTATION_TEXT_SIZE,
            false,
        );
        y += STAFF_MIDDLE_LINE_OFFSET;
    }
    if options.elements.capo && track.capo > 0 {
        page.text(
            page.width / 2.0,
            y,
            format!("Capo: {}", track.capo),
            ANNOTATION_TEXT_SIZE,
            false,
        );
        y += STAFF_MIDDLE_LINE_OFFSET;
    }
    y - SPAN_LAYOUT.metadata_start_y
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
    options: SceneOptions,
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
            b.cursor_rect[1]
                + SPAN_LAYOUT.numbered_anchor_offset_y
                + b.voice as f32 * SYSTEM_LAYOUT.numbered_voice_spacing
                - i as f32 * SYSTEM_LAYOUT.numbered_stack_spacing,
        ];
    }
    let y = |n: &Note| {
        if staff {
            crate::scene::compute_pitch_y(
                n.pitch.expect("validated pitch"),
                clef(track, b.measure),
                b.cursor_rect[1] + SPAN_LAYOUT.staff_anchor_offset_y,
            )
        } else {
            b.cursor_rect[3]
                - SPAN_LAYOUT.staff_anchor_offset_y
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
                crate::scene::compute_voice_offset(&track.measures[b.measure], b.voice, b.start)
            } else {
                0.0
            },
        yy,
    ]
}
