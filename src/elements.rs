//! Measured semantic elements used between notation and drawing primitives.
//!
//! Engravers construct elements without choosing final coordinates. A lane
//! stack then places their measured bounds above or below a notation anchor.
//! Backends continue to consume the same flat [`crate::Primitive`] geometry.

use crate::{text, BeatAnnotations, Pedal, RenderError, Scene};
use smufl::Glyph;

const DEFAULT_ANNOTATION_GAP: f32 = 5.0;
const DEFAULT_ABOVE_ANNOTATION_GAP: f32 = 12.0;
const DEFAULT_BELOW_RHYTHM_GAP: f32 = 25.0;
const DEFAULT_MUSIC_GLYPH_SIZE: f32 = 8.0;
const DEFAULT_ANNOTATION_TEXT_SIZE: f32 = 11.0;
const DEFAULT_LYRIC_TEXT_SIZE: f32 = 11.0;
const PICKUP_TEXT_SIZE: f32 = 14.0;
const TEMPO_NOTE_OFFSET_X: f32 = 18.0;
const TEMPO_NOTE_GLYPH_SIZE: f32 = 6.0;
const TEMPO_TEXT_OFFSET_X: f32 = 12.0;
const TEMPO_TEXT_SIZE: f32 = 11.0;
const TIMER_TEXT_SIZE: f32 = 10.0;
const FADE_HEIGHT: f32 = 10.0;
const TECHNIQUE_TEXT_SIZE: f32 = 12.0;
const WHAMMY_VERTICAL_SCALE: f32 = 4.0;
const WHAMMY_VERTICAL_PADDING: f32 = 30.0;
const CHORD_DIAGRAM_BASE_HEIGHT: f32 = 36.0;
const CHORD_DIAGRAM_FRET_SPACING: f32 = 8.0;
const DYNAMIC_TEXT_SIZE: f32 = 12.0;
const CRESCENDO_HEIGHT: f32 = 8.0;

/// Spacing choices shared by semantic annotation layout.
#[derive(Clone, Copy, Debug)]
pub(crate) struct EngravingMetrics {
    pub annotation_gap: f32,
    pub above_gap: f32,
    pub below_rhythm_gap: f32,
    pub music_size: f32,
    pub annotation_text_size: f32,
    pub lyric_text_size: f32,
}

impl Default for EngravingMetrics {
    fn default() -> Self {
        Self {
            annotation_gap: DEFAULT_ANNOTATION_GAP,
            above_gap: DEFAULT_ABOVE_ANNOTATION_GAP,
            below_rhythm_gap: DEFAULT_BELOW_RHYTHM_GAP,
            music_size: DEFAULT_MUSIC_GLYPH_SIZE,
            annotation_text_size: DEFAULT_ANNOTATION_TEXT_SIZE,
            lyric_text_size: DEFAULT_LYRIC_TEXT_SIZE,
        }
    }
}

#[derive(Clone, Debug)]
enum ElementContent {
    Glyph { glyph: Glyph, scale: f32 },
    Text { text: String, size: f32 },
    Group(Vec<([f32; 2], MeasuredElement)>),
}

/// An element with bounds relative to its centered horizontal anchor.
#[derive(Clone, Debug)]
pub(crate) struct MeasuredElement {
    content: ElementContent,
    bounds: [f32; 4],
}

impl MeasuredElement {
    pub(crate) fn glyph(glyph: Glyph, scale: f32) -> Result<Self, RenderError> {
        let outline = crate::glyph::load(glyph)?;
        Ok(Self {
            content: ElementContent::Glyph { glyph, scale },
            bounds: outline.centered_bounds(scale),
        })
    }

    pub(crate) fn text(value: impl Into<String>, size: f32) -> Self {
        let text = value.into();
        let measured = text::size(&text, size);
        Self {
            content: ElementContent::Text { text, size },
            bounds: [
                -measured[0] / 2.0,
                -measured[1] / 2.0,
                measured[0] / 2.0,
                measured[1] / 2.0,
            ],
        }
    }

    /// Combines measured children whose offsets are relative to the group center.
    pub(crate) fn group(children: Vec<([f32; 2], Self)>) -> Self {
        let mut bounds = [
            f32::INFINITY,
            f32::INFINITY,
            f32::NEG_INFINITY,
            f32::NEG_INFINITY,
        ];
        for (offset, child) in &children {
            bounds[0] = bounds[0].min(offset[0] + child.bounds[0]);
            bounds[1] = bounds[1].min(offset[1] + child.bounds[1]);
            bounds[2] = bounds[2].max(offset[0] + child.bounds[2]);
            bounds[3] = bounds[3].max(offset[1] + child.bounds[3]);
        }
        if children.is_empty() {
            bounds = [0.0; 4];
        }
        Self {
            content: ElementContent::Group(children),
            bounds,
        }
    }

    fn place(&self, page: &mut Scene, x: f32, center_y: f32) -> Result<(), RenderError> {
        match &self.content {
            ElementContent::Glyph { glyph, scale } => {
                page.glyph_at_center(x, center_y, *glyph, *scale)?;
                Ok(())
            }
            ElementContent::Text { text, size } => {
                page.text(x, center_y, text, *size, false);
                Ok(())
            }
            ElementContent::Group(children) => {
                for (offset, child) in children {
                    child.place(page, x + offset[0], center_y + offset[1])?;
                }
                Ok(())
            }
        }
    }

    fn height(&self) -> f32 {
        self.bounds[3] - self.bounds[1]
    }
}

/// Returns space needed above and below a beat's notation anchors.
pub(crate) fn annotation_extents(a: &BeatAnnotations) -> Result<[f32; 2], RenderError> {
    let metrics = EngravingMetrics::default();
    let mut above = Vec::new();
    let mut below = Vec::new();
    if !a.text.is_empty() {
        above.push(MeasuredElement::text(&a.text, metrics.annotation_text_size).height());
    }
    if a.pick_up.is_some() {
        above.push(MeasuredElement::text("↑", PICKUP_TEXT_SIZE).height());
    }
    for glyph in [
        a.golpe.then_some(Glyph::GuitarGolpe),
        a.left_hand_tap.then_some(Glyph::GuitarLeftHandTapping),
        a.wah_open.map(|open| {
            if open {
                Glyph::GuitarOpenPedal
            } else {
                Glyph::GuitarClosePedal
            }
        }),
    ]
    .into_iter()
    .flatten()
    {
        above.push(MeasuredElement::glyph(glyph, metrics.music_size)?.height());
    }
    if let Some(tempo) = a.tempo {
        above.push(
            MeasuredElement::group(vec![
                (
                    [-TEMPO_NOTE_OFFSET_X, 0.0],
                    MeasuredElement::glyph(Glyph::NoteQuarterUp, TEMPO_NOTE_GLYPH_SIZE)?,
                ),
                (
                    [TEMPO_TEXT_OFFSET_X, 0.0],
                    MeasuredElement::text(format!("= {tempo}"), TEMPO_TEXT_SIZE),
                ),
            ])
            .height(),
        );
    }
    if a.timer_seconds.is_some() {
        above.push(MeasuredElement::text("00:00", TIMER_TEXT_SIZE).height());
    }
    if a.fade.is_some() {
        above.push(FADE_HEIGHT);
    }
    if a.technique.is_some() {
        above.push(MeasuredElement::text("T", TECHNIQUE_TEXT_SIZE).height());
    }
    if !a.whammy.is_empty() {
        let (min, max) = a
            .whammy
            .iter()
            .map(|point| -point[1] * WHAMMY_VERTICAL_SCALE)
            .fold((f32::INFINITY, f32::NEG_INFINITY), |(min, max), value| {
                (min.min(value), max.max(value))
            });
        above.push(max - min + WHAMMY_VERTICAL_PADDING);
    }
    if let Some(chord) = &a.chord {
        above.push(
            CHORD_DIAGRAM_BASE_HEIGHT
                + f32::from(chord.compute_rows()) * CHORD_DIAGRAM_FRET_SPACING,
        );
    }

    if let Some(dynamic) = &a.dynamic {
        below.push(if let Some(glyph) = dynamic_glyph(dynamic) {
            MeasuredElement::glyph(glyph, metrics.music_size)?.height()
        } else {
            MeasuredElement::text(dynamic, DYNAMIC_TEXT_SIZE).height()
        });
    }
    below.extend(
        a.lyrics
            .lines()
            .map(|line| MeasuredElement::text(line, metrics.lyric_text_size).height()),
    );
    if let Some(pedal) = a.pedal {
        below.push(
            MeasuredElement::glyph(
                if pedal == Pedal::Down {
                    Glyph::KeyboardPedalPed
                } else {
                    Glyph::KeyboardPedalUp
                },
                metrics.music_size,
            )?
            .height(),
        );
    }
    if a.crescendo.is_some() {
        below.push(CRESCENDO_HEIGHT);
    }
    let stacked = |items: &[f32], initial: f32| {
        if items.is_empty() {
            0.0
        } else {
            initial
                + items.iter().sum::<f32>()
                + metrics.annotation_gap * items.len().saturating_sub(1) as f32
        }
    };
    Ok([
        stacked(&above, metrics.above_gap),
        stacked(&below, metrics.below_rhythm_gap),
    ])
}

/// Allocates non-overlapping element rows away from a notation anchor.
pub(crate) struct LaneStack {
    cursor: f32,
    direction: f32,
    gap: f32,
}

impl LaneStack {
    pub(crate) fn above(anchor: f32, gap_from_notation: f32, lane_gap: f32) -> Self {
        Self {
            cursor: anchor - gap_from_notation,
            direction: -1.0,
            gap: lane_gap,
        }
    }

    pub(crate) fn below(anchor: f32, gap_from_notation: f32, lane_gap: f32) -> Self {
        Self {
            cursor: anchor + gap_from_notation,
            direction: 1.0,
            gap: lane_gap,
        }
    }

    pub(crate) fn place(
        &mut self,
        page: &mut Scene,
        x: f32,
        element: &MeasuredElement,
    ) -> Result<(), RenderError> {
        let center_y = self.cursor + self.direction * element.height() / 2.0;
        element.place(page, x, center_y)?;
        self.cursor += self.direction * (element.height() + self.gap);
        Ok(())
    }

    /// Reserves a row for geometry composed from lines rather than one element.
    pub(crate) fn reserve(&mut self, height: f32) -> f32 {
        let center = self.cursor + self.direction * height / 2.0;
        self.cursor += self.direction * (height + self.gap);
        center
    }

    #[cfg(test)]
    fn cursor(&self) -> f32 {
        self.cursor
    }
}

pub(crate) fn dynamic_glyph(value: &str) -> Option<Glyph> {
    Some(match value {
        "pppp" => Glyph::DynamicPppp,
        "ppp" => Glyph::DynamicPpp,
        "pp" => Glyph::DynamicPp,
        "p" => Glyph::DynamicPiano,
        "mp" => Glyph::DynamicMp,
        "mf" => Glyph::DynamicMf,
        "f" => Glyph::DynamicForte,
        "ff" => Glyph::DynamicFf,
        "fff" => Glyph::DynamicFff,
        "ffff" => Glyph::DynamicFfff,
        "sfz" => Glyph::DynamicSforzato,
        "fp" => Glyph::DynamicFortePiano,
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::{LaneStack, MeasuredElement};
    use crate::Scene;

    #[test]
    fn lane_stack_advances_by_measured_height_and_gap() {
        let element = MeasuredElement::text("line", 12.0);
        let height = element.height();
        let mut below = LaneStack::below(100.0, 10.0, 4.0);
        let mut page = Scene {
            width: 200.0,
            height: 200.0,
            primitives: vec![],
            beats: vec![],
            systems: vec![],
            style: Default::default(),
        };
        below.place(&mut page, 50.0, &element).unwrap();
        assert!((below.cursor() - (110.0 + height + 4.0)).abs() < 0.001);
    }
}
