//! Small, purpose-built Guitar Pro tablature and staff display for `music_gym`.
//!
//! The crate deliberately supports only the three views used by the application:
//! tablature, staff notation, and both together.  It is not a general engraving
//! engine; Guitar Pro decorations outside this model are reported as warnings.

use egui::{Color32, Rect, Sense, Stroke, Vec2};

const QUARTER_TICKS: f64 = 960.0;
const BEAT_ORIGIN_TICKS: i64 = 960;
const LEFT_MARGIN: f32 = 46.0;
const RIGHT_MARGIN: f32 = 18.0;
const SYSTEM_GAP: f32 = 36.0;
const STRING_GAP: f32 = 13.0;

#[derive(Clone, Debug, Default)]
pub struct Track {
    pub name: String,
    /// MIDI pitches, highest string first.
    pub strings: Vec<u8>,
    pub measures: Vec<Measure>,
}

#[derive(Clone, Debug)]
pub struct Measure {
    pub time_signature: (u8, u16),
    /// Voices begin together. Beats in a voice are sequential unless `start` is set.
    pub voices: Vec<Vec<Beat>>,
}

impl Default for Measure {
    fn default() -> Self {
        Self {
            time_signature: (4, 4),
            voices: vec![],
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct Beat {
    /// Onset in quarter notes relative to its measure.
    pub start: Option<f64>,
    pub duration: Duration,
    pub notes: Vec<Note>,
}

impl Beat {
    pub fn compute_quarter_beats(&self) -> Result<f64, RenderError> {
        self.duration.quarter_beats()
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Fret {
    Number(u16),
    Dead,
    Tied(u16),
}
impl Default for Fret {
    fn default() -> Self {
        Self::Number(0)
    }
}

#[derive(Clone, Debug, Default)]
pub struct Note {
    /// One-based, from the top string.
    pub string: usize,
    pub fret: Fret,
    /// Derived during import so staff view never depends on optional spelling data.
    pub midi: Option<u8>,
}

#[derive(Clone, Copy, Debug)]
pub struct Duration {
    pub value: i16,
    pub dots: u8,
    pub tuplet: Option<(u8, u8)>,
}
impl Default for Duration {
    fn default() -> Self {
        Self::QUARTER
    }
}
impl Duration {
    pub const QUARTER: Self = Self {
        value: 4,
        dots: 0,
        tuplet: None,
    };
    pub fn quarter_beats(self) -> Result<f64, RenderError> {
        if self.dots > 3 || self.tuplet.is_some_and(|(a, b)| a == 0 || b == 0) {
            return Err(RenderError("invalid duration".into()));
        }
        let base = match self.value {
            -4 => 16.0,
            -2 => 8.0,
            n if n > 0 && (n as u16).is_power_of_two() && n <= 256 => 4.0 / f64::from(n),
            _ => return Err(RenderError("invalid duration".into())),
        };
        let dotted = base * (2.0 - 2.0_f64.powi(-i32::from(self.dots)));
        Ok(dotted
            * self
                .tuplet
                .map_or(1.0, |(a, b)| f64::from(b) / f64::from(a)))
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum DisplayMode {
    #[default]
    Tablature,
    Staff,
    Both,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum LayoutMode {
    #[default]
    Vertical,
    Horizontal,
}

#[derive(Clone, Copy, Debug)]
pub struct SceneOptions {
    pub width: f32,
    pub display: DisplayMode,
    pub flow: LayoutMode,
}
impl Default for SceneOptions {
    fn default() -> Self {
        Self {
            width: 900.0,
            display: DisplayMode::Tablature,
            flow: LayoutMode::Vertical,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct BeatAddress {
    pub measure: usize,
    pub voice: usize,
    pub beat: usize,
}

#[derive(Clone, Debug)]
pub struct BeatBounds {
    pub start: f64,
    pub duration: f64,
    pub measure: usize,
    pub voice: usize,
    pub beat: usize,
    pub rect: [f32; 4],
    pub cursor_rect: [f32; 4],
}

#[derive(Clone, Debug)]
enum Draw {
    Line([f32; 2], [f32; 2]),
    Text([f32; 2], String, f32),
    Note([f32; 2], bool),
}

#[derive(Clone, Debug)]
pub struct Scene {
    pub width: f32,
    pub height: f32,
    pub beats: Vec<BeatBounds>,
    draw: Vec<Draw>,
}
impl Scene {
    pub fn scaled(mut self, zoom: f32) -> Result<Self, RenderError> {
        if !zoom.is_finite() || zoom <= 0.0 {
            return Err(RenderError("zoom must be positive".into()));
        }
        self.width *= zoom;
        self.height *= zoom;
        for b in &mut self.beats {
            for value in &mut b.rect {
                *value *= zoom;
            }
            for value in &mut b.cursor_rect {
                *value *= zoom;
            }
        }
        for command in &mut self.draw {
            match command {
                Draw::Line(a, b) => {
                    for p in [a, b] {
                        p[0] *= zoom;
                        p[1] *= zoom;
                    }
                }
                Draw::Text(p, _, size) => {
                    p[0] *= zoom;
                    p[1] *= zoom;
                    *size *= zoom;
                }
                Draw::Note(p, _) => {
                    p[0] *= zoom;
                    p[1] *= zoom;
                }
            }
        }
        Ok(self)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RenderError(String);
impl std::fmt::Display for RenderError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}
impl std::error::Error for RenderError {}

/// Builds simple, deterministic display geometry. Layout never mutates the score.
pub fn engrave(track: &Track, options: SceneOptions) -> Result<Scene, RenderError> {
    if !options.width.is_finite() || options.width < 160.0 {
        return Err(RenderError("width must be at least 160".into()));
    }
    let strings = track.strings.len().max(1);
    let staff_height = 4.0 * STRING_GAP;
    let tab_height = (strings.saturating_sub(1) as f32) * STRING_GAP;
    let system_height = match options.display {
        DisplayMode::Tablature => tab_height,
        DisplayMode::Staff => staff_height,
        DisplayMode::Both => staff_height + 24.0 + tab_height,
    } + 30.0;
    let available = (options.width - LEFT_MARGIN - RIGHT_MARGIN).max(80.0);
    let mut scene = Scene {
        width: options.width,
        height: 0.0,
        beats: vec![],
        draw: vec![],
    };
    let mut x = LEFT_MARGIN;
    let mut y = 28.0;
    let mut system = 0usize;
    for (mi, measure) in track.measures.iter().enumerate() {
        let measure_width = available / 3.0;
        if options.flow == LayoutMode::Vertical
            && x > LEFT_MARGIN
            && x + measure_width > options.width - RIGHT_MARGIN
        {
            x = LEFT_MARGIN;
            y += system_height + SYSTEM_GAP;
            system += 1;
        }
        if options.flow == LayoutMode::Horizontal {
            y = 28.0;
        }
        let tab_y = y + if options.display == DisplayMode::Both {
            staff_height + 24.0
        } else {
            0.0
        };
        if matches!(options.display, DisplayMode::Tablature | DisplayMode::Both) {
            for string in 0..strings {
                let sy = tab_y + string as f32 * STRING_GAP;
                scene
                    .draw
                    .push(Draw::Line([x, sy], [x + measure_width, sy]));
            }
        }
        if matches!(options.display, DisplayMode::Staff | DisplayMode::Both) {
            for line in 0..5 {
                let sy = y + line as f32 * STRING_GAP;
                scene
                    .draw
                    .push(Draw::Line([x, sy], [x + measure_width, sy]));
            }
        }
        scene
            .draw
            .push(Draw::Text([x + 4.0, y - 11.0], (mi + 1).to_string(), 11.0));
        let bar_beats =
            f64::from(measure.time_signature.0) * 4.0 / f64::from(measure.time_signature.1.max(1));
        for (vi, voice) in measure.voices.iter().enumerate() {
            let mut onset = 0.0;
            for (bi, beat) in voice.iter().enumerate() {
                onset = beat.start.unwrap_or(onset);
                let duration = beat.compute_quarter_beats()?;
                let bx = x + (onset / bar_beats.max(0.01)) as f32 * measure_width;
                let next_x = x + ((onset + duration) / bar_beats.max(0.01)) as f32 * measure_width;
                let top = y - 8.0;
                let bottom = y + system_height - 12.0;
                scene.beats.push(BeatBounds {
                    start: onset,
                    duration,
                    measure: mi,
                    voice: vi,
                    beat: bi,
                    rect: [bx, top, next_x.max(bx + 8.0), bottom],
                    cursor_rect: [bx, y, next_x.max(bx + 8.0), bottom],
                });
                for note in &beat.notes {
                    if matches!(options.display, DisplayMode::Tablature | DisplayMode::Both)
                        && note.string > 0
                        && note.string <= strings
                    {
                        let label = match note.fret {
                            Fret::Number(n) | Fret::Tied(n) => n.to_string(),
                            Fret::Dead => "x".into(),
                        };
                        scene.draw.push(Draw::Text(
                            [bx + 5.0, tab_y + (note.string - 1) as f32 * STRING_GAP],
                            label,
                            12.0,
                        ));
                    }
                    if matches!(options.display, DisplayMode::Staff | DisplayMode::Both) {
                        if let Some(midi) = note.midi {
                            let py = y + 4.0 * STRING_GAP
                                - (f32::from(midi) - 60.0) * (STRING_GAP / 2.0);
                            scene.draw.push(Draw::Note([bx + 5.0, py], vi % 2 == 1));
                        }
                    }
                }
                onset += duration;
            }
        }
        scene
            .draw
            .push(Draw::Line([x, y], [x, y + system_height - 12.0]));
        scene.draw.push(Draw::Line(
            [x + measure_width, y],
            [x + measure_width, y + system_height - 12.0],
        ));
        x += measure_width;
        if options.flow == LayoutMode::Horizontal {
            x += 0.0;
        }
        let _ = system;
    }
    scene.height = (y + system_height + 20.0).max(80.0);
    Ok(scene)
}

#[derive(Clone, Copy, Debug)]
pub struct Selection {
    pub anchor: BeatAddress,
    pub end: BeatAddress,
}
pub struct Interaction {
    pub response: egui::Response,
    pub clicked: Option<BeatAddress>,
    pub hovered: Option<BeatAddress>,
}
pub struct EguiInteraction<'a> {
    scene: &'a Scene,
}
impl<'a> EguiInteraction<'a> {
    pub fn new(scene: &'a Scene) -> Self {
        Self { scene }
    }
    pub fn handle(
        &self,
        ui: &mut egui::Ui,
        active: &[BeatAddress],
        selection: &mut Option<Selection>,
    ) -> Interaction {
        let response = self.paint(ui, active);
        let address = response
            .hover_pos()
            .and_then(|p| self.hit_test(p - response.rect.min));
        if response.clicked() {
            if let Some(a) = address {
                if ui.input(|i| i.modifiers.shift) {
                    if let Some(s) = selection {
                        s.end = a;
                    } else {
                        *selection = Some(Selection { anchor: a, end: a });
                    }
                } else {
                    *selection = Some(Selection { anchor: a, end: a });
                }
            }
        }
        Interaction {
            response,
            clicked: if ui.ctx().input(|i| i.pointer.any_released()) {
                address
            } else {
                None
            },
            hovered: address,
        }
    }
    pub fn paint_playback_cursor(
        &self,
        ui: &mut egui::Ui,
        response: &egui::Response,
        address: BeatAddress,
        fraction: f32,
        follow: bool,
    ) {
        if let Some(b) = self.scene.beats.iter().find(|b| {
            (b.measure, b.voice, b.beat) == (address.measure, address.voice, address.beat)
        }) {
            let rect = Rect::from_min_max(
                response.rect.min + Vec2::new(b.cursor_rect[0], b.cursor_rect[1]),
                response.rect.min + Vec2::new(b.cursor_rect[2], b.cursor_rect[3]),
            );
            let x = egui::lerp(rect.x_range(), fraction.clamp(0.0, 1.0));
            ui.painter().vline(
                x,
                rect.y_range(),
                Stroke::new(2.0, Color32::from_rgb(30, 105, 190)),
            );
            if follow {
                ui.scroll_to_rect(rect, Some(egui::Align::Center));
            }
        }
    }
    fn hit_test(&self, p: Vec2) -> Option<BeatAddress> {
        self.scene
            .beats
            .iter()
            .find(|b| p.x >= b.rect[0] && p.x <= b.rect[2] && p.y >= b.rect[1] && p.y <= b.rect[3])
            .map(|b| BeatAddress {
                measure: b.measure,
                voice: b.voice,
                beat: b.beat,
            })
    }
    fn paint(&self, ui: &mut egui::Ui, active: &[BeatAddress]) -> egui::Response {
        let (rect, response) = ui.allocate_exact_size(
            Vec2::new(self.scene.width, self.scene.height),
            Sense::click_and_drag(),
        );
        let painter = ui.painter_at(rect);
        let foreground = ui.visuals().text_color();
        for d in &self.scene.draw {
            match d {
                Draw::Line(a, b) => {
                    painter.line_segment(
                        [rect.min + Vec2::from(*a), rect.min + Vec2::from(*b)],
                        Stroke::new(1.0, foreground),
                    );
                }
                Draw::Text(p, text, size) => {
                    painter.text(
                        rect.min + Vec2::from(*p),
                        egui::Align2::CENTER_CENTER,
                        text,
                        egui::FontId::proportional(*size),
                        foreground,
                    );
                }
                Draw::Note(p, down) => {
                    let c = rect.min + Vec2::from(*p);
                    painter.circle_filled(c, 4.0, foreground);
                    let stem = if *down { -20.0 } else { 20.0 };
                    painter.line_segment(
                        [c + Vec2::new(4.0, 0.0), c + Vec2::new(4.0, stem)],
                        Stroke::new(1.0, foreground),
                    );
                }
            }
        }
        for a in active {
            if let Some(b) = self
                .scene
                .beats
                .iter()
                .find(|b| (b.measure, b.voice, b.beat) == (a.measure, a.voice, a.beat))
            {
                painter.rect_filled(
                    Rect::from_min_max(
                        rect.min + Vec2::new(b.rect[0], b.rect[1]),
                        rect.min + Vec2::new(b.rect[2], b.rect[3]),
                    ),
                    0.0,
                    Color32::from_rgba_unmultiplied(90, 170, 255, 40),
                );
            }
        }
        response
    }
}

pub mod guitar_pro {
    use super::*;
    #[derive(Debug)]
    pub struct ImportReport {
        pub track: Track,
        pub warnings: Vec<String>,
    }
    pub fn convert_track(
        song: &guitarpro::Song,
        source: &guitarpro::Track,
    ) -> Result<ImportReport, RenderError> {
        let mut warnings = vec![];
        let strings = source
            .strings
            .iter()
            .map(|(_, midi)| {
                u8::try_from(*midi).map_err(|_| RenderError("invalid string tuning".into()))
            })
            .collect::<Result<Vec<_>, _>>()?;
        let mut measures = Vec::with_capacity(source.measures.len());
        for (mi, source_measure) in source.measures.iter().enumerate() {
            let header = song
                .measure_headers
                .get(mi)
                .ok_or_else(|| RenderError(format!("missing header for measure {}", mi + 1)))?;
            let numerator = u8::try_from(header.time_signature.numerator)
                .map_err(|_| RenderError("invalid meter".into()))?;
            let denominator = u16::try_from(header.time_signature.denominator.value)
                .map_err(|_| RenderError("invalid meter".into()))?;
            let mut voices = Vec::new();
            for source_voice in &source_measure.voices {
                let mut beats = Vec::new();
                let mut onset = 0.0;
                for source_beat in &source_voice.beats {
                    if source_beat.status == guitarpro::BeatStatus::Empty {
                        continue;
                    }
                    let duration = Duration {
                        value: i16::try_from(source_beat.duration.value)
                            .map_err(|_| RenderError("invalid duration".into()))?,
                        dots: if source_beat.duration.double_dotted {
                            2
                        } else {
                            u8::from(source_beat.duration.dotted)
                        },
                        tuplet: ((
                            source_beat.duration.tuplet_enters,
                            source_beat.duration.tuplet_times,
                        ) != (1, 1))
                            .then_some((
                                source_beat.duration.tuplet_enters,
                                source_beat.duration.tuplet_times,
                            )),
                    };
                    if let Some(ticks) = source_beat.start {
                        onset = ((ticks - BEAT_ORIGIN_TICKS) as f64 / QUARTER_TICKS).max(onset);
                    }
                    let mut notes = Vec::new();
                    for n in &source_beat.notes {
                        if n.kind == guitarpro::NoteType::Rest {
                            continue;
                        };
                        let string = usize::try_from(n.string)
                            .map_err(|_| RenderError("invalid string index".into()))?;
                        let fret = u16::try_from(n.value)
                            .map_err(|_| RenderError("negative fret".into()))?;
                        let value = match n.kind {
                            guitarpro::NoteType::Dead => Fret::Dead,
                            guitarpro::NoteType::Tie => Fret::Tied(fret),
                            guitarpro::NoteType::Normal => Fret::Number(fret),
                            _ => {
                                warnings.push("Unknown note kind omitted".into());
                                continue;
                            }
                        };
                        let midi = if source.percussion_track {
                            None
                        } else {
                            let open = strings
                                .get(
                                    string
                                        .checked_sub(1)
                                        .ok_or_else(|| RenderError("zero string index".into()))?,
                                )
                                .ok_or_else(|| RenderError("string index out of range".into()))?;
                            let midi =
                                i32::from(*open) + i32::from(fret) + i32::from(source.offset)
                                    - i32::from(source.transpose_chromatic)
                                    - i32::from(source.transpose_octave) * 12;
                            Some(u8::try_from(midi).map_err(|_| {
                                RenderError("written pitch outside MIDI range".into())
                            })?)
                        };
                        notes.push(Note {
                            string,
                            fret: value,
                            midi,
                        });
                    }
                    beats.push(Beat {
                        start: Some(onset),
                        duration,
                        notes,
                    });
                    onset += duration.quarter_beats()?;
                }
                voices.push(beats);
            }
            measures.push(Measure {
                time_signature: (numerator, denominator),
                voices,
            });
        }
        warnings.sort();
        warnings.dedup();
        Ok(ImportReport {
            track: Track {
                name: source.name.clone(),
                strings,
                measures,
            },
            warnings,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lays_out_every_beat_with_its_original_address() {
        let track = Track {
            strings: vec![64, 59],
            measures: vec![Measure {
                voices: vec![vec![
                    Beat {
                        notes: vec![Note {
                            string: 1,
                            fret: Fret::Number(3),
                            midi: Some(67),
                        }],
                        ..Default::default()
                    },
                    Beat::default(),
                ]],
                ..Default::default()
            }],
            ..Default::default()
        };
        let scene = engrave(
            &track,
            SceneOptions {
                display: DisplayMode::Both,
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(scene.beats.len(), 2);
        assert_eq!(scene.beats[1].measure, 0);
        assert_eq!(scene.beats[1].voice, 0);
        assert_eq!(scene.beats[1].beat, 1);
        assert!(scene.beats[1].rect[0] > scene.beats[0].rect[0]);
    }

    #[test]
    fn duration_rejects_invalid_tuplets() {
        let duration = Duration {
            value: 4,
            dots: 0,
            tuplet: Some((3, 0)),
        };
        assert!(duration.quarter_beats().is_err());
    }
}
