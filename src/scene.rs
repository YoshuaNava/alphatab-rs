//! Backend-neutral scene layout and geometry.

use crate::{Fret, RenderError, Track};

const LEFT_MARGIN: f32 = 46.0;
const RIGHT_MARGIN: f32 = 18.0;
const SYSTEM_GAP: f32 = 36.0;
const STRING_GAP: f32 = 13.0;

/// A two-dimensional coordinate frame positioned within its parent frame.
///
/// Use [`CoordinateFrame::convert_point_to_parent`] to turn a point in this
/// frame into its parent's coordinates, and
/// [`CoordinateFrame::convert_point_from_parent`] for the inverse operation.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct CoordinateFrame {
    /// This frame's origin in its parent's coordinate system.
    pub origin: [f32; 2],
}

impl CoordinateFrame {
    /// Creates a frame whose origin is `origin` in its parent coordinate system.
    pub fn create_at(origin: [f32; 2]) -> Self {
        Self { origin }
    }

    /// Converts `point` from this frame into its parent coordinate system.
    pub fn convert_point_to_parent(self, point: [f32; 2]) -> [f32; 2] {
        [point[0] + self.origin[0], point[1] + self.origin[1]]
    }

    /// Converts `point` from the parent coordinate system into this frame.
    pub fn convert_point_from_parent(self, point: [f32; 2]) -> [f32; 2] {
        [point[0] - self.origin[0], point[1] - self.origin[1]]
    }
}

/// The position and extent of one bar within a scene.
#[derive(Clone, Debug)]
pub struct BarGeometry {
    /// Zero-based bar index in the track.
    pub bar: usize,
    /// Coordinate frame that maps bar-local points into scene coordinates.
    pub frame: CoordinateFrame,
    /// Bar extent in scene points as `[width, height]`.
    pub size: [f32; 2],
}

impl BarGeometry {
    /// Returns whether `scene_point` lies inside this bar.
    pub fn contains_scene_point(&self, scene_point: [f32; 2]) -> bool {
        let point = self.frame.convert_point_from_parent(scene_point);
        point[0] >= 0.0 && point[0] <= self.size[0] && point[1] >= 0.0 && point[1] <= self.size[1]
    }
}

/// A scene point expressed in a particular bar's local coordinate frame.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BarPoint {
    /// Zero-based bar index.
    pub bar: usize,
    /// Point relative to the bar's top-left origin.
    pub point: [f32; 2],
}

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

/// Inputs that affect the deterministic layout.
#[derive(Clone, Copy, Debug)]
pub struct SceneOptions {
    /// Requested layout width in egui points.
    pub width: f32,
    /// Notation view to display.
    pub display: DisplayMode,
}

impl Default for SceneOptions {
    fn default() -> Self {
        Self {
            width: 900.0,
            display: DisplayMode::Tablature,
        }
    }
}

/// Timing and hit-test rectangle for one laid-out beat.
#[derive(Clone, Debug)]
pub struct BeatGeometry {
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
pub(crate) enum Draw {
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
    pub beats: Vec<BeatGeometry>,
    /// Position and extent of every bar.
    pub bars: Vec<BarGeometry>,
    pub(crate) draw: Vec<Draw>,
}

impl Scene {
    /// Finds the bar containing `scene_point` and expresses it in that bar's frame.
    pub fn find_bar_point(&self, scene_point: [f32; 2]) -> Option<BarPoint> {
        self.bars
            .iter()
            .find(|bar| bar.contains_scene_point(scene_point))
            .map(|bar| BarPoint {
                bar: bar.bar,
                point: bar.frame.convert_point_from_parent(scene_point),
            })
    }

    /// Scales this scene uniformly by a positive finite factor.
    pub fn scale_by(mut self, zoom: f32) -> Result<Self, RenderError> {
        if !zoom.is_finite() || zoom <= 0.0 {
            return Err(RenderError("zoom must be positive".into()));
        }

        // Apply scaling to window size
        self.width *= zoom;
        self.height *= zoom;

        // Apply scaling to every bar frame before its child geometry.
        for bar in &mut self.bars {
            for value in &mut bar.frame.origin {
                *value *= zoom;
            }
            for value in &mut bar.size {
                *value *= zoom;
            }
        }

        // Apply scaling to every beat.
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
        bars: vec![],
        draw: vec![],
    };
    let mut x = LEFT_MARGIN;
    let mut y = 28.0;
    for (bar_index, bar) in track.bars.iter().enumerate() {
        let bar_width = available / 3.0;
        if x > LEFT_MARGIN && x + bar_width > options.width - RIGHT_MARGIN {
            x = LEFT_MARGIN;
            y += system_height + SYSTEM_GAP;
        }
        let bar_frame = CoordinateFrame::create_at([x, y]);
        scene.bars.push(BarGeometry {
            bar: bar_index,
            frame: bar_frame,
            size: [bar_width, system_height - 12.0],
        });
        let tab_y = y + if options.display == DisplayMode::Both {
            staff_height + 24.0
        } else {
            0.0
        };
        if matches!(options.display, DisplayMode::Tablature | DisplayMode::Both) {
            for string in 0..strings {
                let sy = tab_y + string as f32 * STRING_GAP;
                scene.draw.push(Draw::Line([x, sy], [x + bar_width, sy]));
            }
        }
        if matches!(options.display, DisplayMode::Staff | DisplayMode::Both) {
            for line in 0..5 {
                let sy = y + line as f32 * STRING_GAP;
                scene.draw.push(Draw::Line([x, sy], [x + bar_width, sy]));
            }
        }
        scene.draw.push(Draw::Text(
            [x + 4.0, y - 11.0],
            (bar_index + 1).to_string(),
            11.0,
        ));
        let bar_beats =
            f64::from(bar.time_signature.0) * 4.0 / f64::from(bar.time_signature.1.max(1));
        for (voice_index, voice) in bar.voices.iter().enumerate() {
            let mut onset = 0.0;
            for (beat_index, beat) in voice.beats.iter().enumerate() {
                onset = beat.start.unwrap_or(onset);
                let duration = beat.compute_quarter_beats()?;
                let bx = bar_frame.convert_point_to_parent([
                    (onset / bar_beats.max(0.01)) as f32 * bar_width,
                    0.0,
                ])[0];
                let next_x = bar_frame.convert_point_to_parent([
                    ((onset + duration) / bar_beats.max(0.01)) as f32 * bar_width,
                    0.0,
                ])[0];
                let top = y - 8.0;
                let bottom = y + system_height - 12.0;
                scene.beats.push(BeatGeometry {
                    start: onset,
                    duration,
                    bar: bar_index,
                    voice: voice_index,
                    beat: beat_index,
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
                            scene
                                .draw
                                .push(Draw::Note([bx + 5.0, py], voice_index % 2 == 1));
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
            [x + bar_width, y],
            [x + bar_width, y + system_height - 12.0],
        ));
        x += bar_width;
    }
    scene.height = (y + system_height + 20.0).max(80.0);
    Ok(scene)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Bar, Beat, Note, Voice};

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
        let origin = scene.bars[0].frame.origin;
        let bar_point = scene
            .find_bar_point([origin[0] + 10.0, origin[1] + 10.0])
            .unwrap();
        assert_eq!(bar_point.bar, 0);
        assert_eq!(bar_point.point, [10.0, 10.0]);
    }
}
