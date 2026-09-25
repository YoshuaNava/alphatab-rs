//! Small, purpose-built Guitar Pro tablature and staff display for `music_gym`.
//!
//! The crate deliberately supports only the three views used by the application:
//! tablature, staff notation, and both together.  It is not a general engraving
//! engine; Guitar Pro decorations outside this model are reported as warnings.
#![deny(missing_docs)]

use egui::{Color32, Rect, Sense, Stroke, Vec2};

mod track;
pub use track::{Bar, Beat, BeatAddress, Duration, Fret, Note, Track, Voice};

const LEFT_MARGIN: f32 = 46.0;
const RIGHT_MARGIN: f32 = 18.0;
const SYSTEM_GAP: f32 = 36.0;
const STRING_GAP: f32 = 13.0;

/// The notation view to lay out.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum DisplayMode {
    #[default]
    /// Six-string-style tablature only.
    Tablature,
    /// Five-line staff only.
    Staff,
    /// Staff followed by tablature.
    Both,
}
/// Whether systems wrap or form a horizontal strip.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum LayoutMode {
    #[default]
    /// Wrap systems to the next row.
    Vertical,
    /// Keep all bars on one scrolling row.
    Horizontal,
}

/// Inputs that affect the deterministic layout.
#[derive(Clone, Copy, Debug)]
pub struct SceneOptions {
    /// Requested layout width in egui points.
    pub width: f32,
    /// Notation view to display.
    pub display: DisplayMode,
    /// Wrapping policy.
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

/// Timing and hit-test rectangle for one laid-out beat.
#[derive(Clone, Debug)]
pub struct BeatBounds {
    /// Onset in quarter notes relative to the bar.
    pub start: f64,
    /// Performed duration in quarter notes.
    pub duration: f64,
    /// Bar index.
    pub bar: usize,
    /// Voice index.
    pub voice: usize,
    /// Beat index.
    pub beat: usize,
    /// Inclusive drawing rectangle as `[left, top, right, bottom]`.
    pub rect: [f32; 4],
    /// Cursor rectangle as `[left, top, right, bottom]`.
    pub cursor_rect: [f32; 4],
}

#[derive(Clone, Debug)]
enum Draw {
    Line([f32; 2], [f32; 2]),
    Text([f32; 2], String, f32),
    Note([f32; 2], bool),
}

/// Completed geometry that can be painted and interacted with through egui.
#[derive(Clone, Debug)]
pub struct Scene {
    /// Scene width in egui points.
    pub width: f32,
    /// Scene height in egui points.
    pub height: f32,
    /// Hit-test information for every beat.
    pub beats: Vec<BeatBounds>,
    draw: Vec<Draw>,
}
impl Scene {
    /// Scales this scene uniformly by a positive finite factor.
    pub fn scale_by(mut self, zoom: f32) -> Result<Self, RenderError> {
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

/// An invalid score, import, or layout request.
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
    for (bar_index, bar) in track.bars.iter().enumerate() {
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
        scene.draw.push(Draw::Text(
            [x + 4.0, y - 11.0],
            (bar_index + 1).to_string(),
            11.0,
        ));
        let bar_beats =
            f64::from(bar.time_signature.0) * 4.0 / f64::from(bar.time_signature.1.max(1));
        for (vi, voice) in bar.voices.iter().enumerate() {
            let mut onset = 0.0;
            for (bi, beat) in voice.beats.iter().enumerate() {
                onset = beat.start.unwrap_or(onset);
                let duration = beat.compute_quarter_beats()?;
                let bx = x + (onset / bar_beats.max(0.01)) as f32 * measure_width;
                let next_x = x + ((onset + duration) / bar_beats.max(0.01)) as f32 * measure_width;
                let top = y - 8.0;
                let bottom = y + system_height - 12.0;
                scene.beats.push(BeatBounds {
                    start: onset,
                    duration,
                    bar: bar_index,
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

/// Inclusive range selected by the user.
#[derive(Clone, Copy, Debug)]
pub struct Selection {
    /// Fixed endpoint where selection began.
    pub anchor: BeatAddress,
    /// Most recently selected endpoint.
    pub end: BeatAddress,
}
/// Result of one interactive paint pass.
pub struct Interaction {
    /// Egui response allocated for the scene.
    pub response: egui::Response,
    /// Beat clicked in this pass, if any.
    pub clicked: Option<BeatAddress>,
    /// Beat under the pointer, if any.
    pub hovered: Option<BeatAddress>,
}
/// Egui painting and interaction adapter for a completed scene.
pub struct EguiInteraction<'a> {
    scene: &'a Scene,
}
impl<'a> EguiInteraction<'a> {
    /// Creates an egui adapter for `scene`.
    pub fn create_for_scene(scene: &'a Scene) -> Self {
        Self { scene }
    }
    /// Paints the scene, performs hit testing, and updates click selection.
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
    /// Paints an interpolated playback cursor and optionally scrolls it into view.
    pub fn paint_playback_cursor(
        &self,
        ui: &mut egui::Ui,
        response: &egui::Response,
        address: BeatAddress,
        fraction: f32,
        follow: bool,
    ) {
        if let Some(b) = self
            .scene
            .beats
            .iter()
            .find(|b| (b.bar, b.voice, b.beat) == (address.bar, address.voice, address.beat))
        {
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
                bar: b.bar,
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
                .find(|b| (b.bar, b.voice, b.beat) == (a.bar, a.voice, a.beat))
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

/// Guitar Pro adapter for already parsed `guitarpro` model values.
pub mod guitar_pro;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lays_out_every_beat_with_its_original_address() {
        let track = Track {
            strings: vec![64, 59],
            bars: vec![Bar {
                voices: vec![Voice {
                    beats: vec![
                        Beat {
                            notes: vec![Note {
                                string: 1,
                                fret: Fret::Number(3),
                                midi: Some(67),
                            }],
                            ..Default::default()
                        },
                        Beat::default(),
                    ],
                }],
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
        assert_eq!(scene.beats[1].bar, 0);
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
        assert!(duration.compute_quarter_beats().is_err());
    }
}
