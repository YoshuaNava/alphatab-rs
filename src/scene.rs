//! Backend-neutral scene layout and geometry.

use crate::{BeatAddress, Fret, RenderError, Track};

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

/// A two-dimensional coordinate frame positioned within its parent frame.
///
/// Use [`CoordinateFrame::convert_point_to_parent`] to turn a point in this
/// frame into its parent's coordinates, and
/// [`CoordinateFrame::convert_point_from_parent`] for the inverse operation.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct CoordinateFrame {
    /// This frame's origin in its parent's coordinate system.
    pub(crate) origin: [f32; 2],
}

impl CoordinateFrame {
    /// Creates a frame whose origin is `origin` in its parent coordinate system.
    pub(crate) fn create_at(origin: [f32; 2]) -> Self {
        Self { origin }
    }

    /// Converts `point` from this frame into its parent coordinate system.
    pub(crate) fn convert_point_to_parent(self, point: [f32; 2]) -> [f32; 2] {
        [point[0] + self.origin[0], point[1] + self.origin[1]]
    }

    /// Converts `point` from the parent coordinate system into this frame.
    pub(crate) fn convert_point_from_parent(self, point: [f32; 2]) -> [f32; 2] {
        [point[0] - self.origin[0], point[1] - self.origin[1]]
    }
}

/// The position and extent of one bar within a scene.
#[derive(Clone, Debug)]
pub(crate) struct BarGeometry {
    /// Zero-based bar index in the track.
    pub(crate) bar: usize,
    /// Coordinate frame that maps bar-local points into scene coordinates.
    pub(crate) frame: CoordinateFrame,
    /// Bar extent in scene points as `[width, height]`.
    pub(crate) size: [f32; 2],
    /// Beat geometry expressed in this bar's local frame.
    pub(crate) beats: Vec<BeatGeometry>,
    pub(crate) draw: Vec<Draw>,
}

/// Timing and hit-test rectangle for one laid-out beat.
#[derive(Clone, Debug)]
pub(crate) struct BeatGeometry {
    /// Voice index.
    pub(crate) voice: usize,
    /// Beat index.
    pub(crate) beat: usize,
    /// Click and highlight rectangle as `[left, top, right, bottom]` in bar-local points.
    ///
    /// Its top edge extends slightly above the bar so notes near the top staff
    /// line remain easy to select.
    pub(crate) rect: [f32; 4],
    /// Playback-cursor rectangle as `[left, top, right, bottom]` in bar-local points.
    ///
    /// Unlike [`Self::rect`], this starts at the top of the notation area so
    /// the cursor remains within the bar while it moves across the beat.
    pub(crate) cursor_rect: [f32; 4],
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
    /// Position and extent of every bar.
    pub(crate) bars: Vec<BarGeometry>,
}

impl Scene {
    /// Finds geometry from a musical address.
    ///
    /// This answers “where is this beat?” for playback cursors and active-beat
    /// highlights.
    pub(crate) fn find_beat(&self, address: BeatAddress) -> Option<(&BarGeometry, &BeatGeometry)> {
        self.bars.iter().find_map(|bar| {
            (bar.bar == address.bar)
                .then(|| {
                    bar.beats
                        .iter()
                        .find(|beat| beat.voice == address.voice && beat.beat == address.beat)
                        .map(|beat| (bar, beat))
                })
                .flatten()
        })
    }

    /// Finds a musical address from a scene point.
    ///
    /// This answers “what beat is here?” for mouse hover and click handling.
    pub(crate) fn find_beat_at_scene_point(&self, point: [f32; 2]) -> Option<BeatAddress> {
        self.bars.iter().find_map(|bar| {
            let point = bar.frame.convert_point_from_parent(point);
            bar.beats
                .iter()
                .find(|beat| {
                    point[0] >= beat.rect[0]
                        && point[0] <= beat.rect[2]
                        && point[1] >= beat.rect[1]
                        && point[1] <= beat.rect[3]
                })
                .map(|beat| BeatAddress {
                    bar: bar.bar,
                    voice: beat.voice,
                    beat: beat.beat,
                })
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

        // Apply scaling to every bar frame and its local geometry.
        for bar in &mut self.bars {
            for value in &mut bar.frame.origin {
                *value *= zoom;
            }
            for value in &mut bar.size {
                *value *= zoom;
            }
            for b in &mut bar.beats {
                for value in &mut b.rect {
                    *value *= zoom;
                }
                for value in &mut b.cursor_rect {
                    *value *= zoom;
                }
            }
            for command in &mut bar.draw {
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
        }

        Ok(self)
    }
}

/// Builds a scene graph from a track without mutating the score.
///
/// Each bar is first laid out in its own local coordinate frame. The returned
/// scene then records where each completed bar frame belongs in the larger score.
pub fn engrave(track: &Track, options: SceneOptions) -> Result<Scene, RenderError> {
    // Reject dimensions that cannot produce a useful layout before creating any geometry.
    if !options.width.is_finite() || options.width < 160.0 {
        return Err(RenderError("width must be at least 160".into()));
    }
    // Derive the fixed vertical measurements shared by every bar in this scene.
    let strings = track.strings.len().max(1);
    let staff_height = 4.0 * STRING_GAP;
    let tab_height = (strings.saturating_sub(1) as f32) * STRING_GAP;
    let system_height = match options.display {
        DisplayMode::Tablature => tab_height,
        DisplayMode::Staff => staff_height,
        DisplayMode::Both => staff_height + 24.0 + tab_height,
    } + 30.0;
    // Create the scene composer. `x` and `y` place bar frames in scene coordinates.
    let available = (options.width - LEFT_MARGIN - RIGHT_MARGIN).max(80.0);
    let mut scene = Scene {
        width: options.width,
        height: 0.0,
        bars: vec![],
    };
    let mut x = LEFT_MARGIN;
    let mut y = 28.0;
    for (bar_index, bar) in track.bars.iter().enumerate() {
        // Give each bar one third of a system and start a new system when it would overflow.
        let bar_width = available / 3.0;
        if x > LEFT_MARGIN && x + bar_width > options.width - RIGHT_MARGIN {
            x = LEFT_MARGIN;
            y += system_height + SYSTEM_GAP;
        }
        // Start a new bar node. Everything added below uses this bar's local origin `(0, 0)`.
        let bar_frame = CoordinateFrame::create_at([x, y]);
        let mut geometry = BarGeometry {
            bar: bar_index,
            frame: bar_frame,
            size: [bar_width, system_height - 12.0],
            beats: vec![],
            draw: vec![],
        };
        // Add the staff and/or tab staff lines in bar-local coordinates.
        let tab_y = y + if options.display == DisplayMode::Both {
            staff_height + 24.0
        } else {
            0.0
        };
        if matches!(options.display, DisplayMode::Tablature | DisplayMode::Both) {
            for string in 0..strings {
                let sy = tab_y + string as f32 * STRING_GAP;
                geometry
                    .draw
                    .push(Draw::Line([0.0, sy - y], [bar_width, sy - y]));
            }
        }
        if matches!(options.display, DisplayMode::Staff | DisplayMode::Both) {
            for line in 0..5 {
                let sy = y + line as f32 * STRING_GAP;
                geometry
                    .draw
                    .push(Draw::Line([0.0, sy - y], [bar_width, sy - y]));
            }
        }
        // Label the bar and calculate its musical duration in quarter-note units.
        geometry
            .draw
            .push(Draw::Text([4.0, -11.0], (bar_index + 1).to_string(), 11.0));
        let bar_beats =
            f64::from(bar.time_signature.0) * 4.0 / f64::from(bar.time_signature.1.max(1));
        for (voice_index, voice) in bar.voices.iter().enumerate() {
            // Lay out each voice independently; voices share the same bar-local frame.
            let mut onset = 0.0;
            for (beat_index, beat) in voice.beats.iter().enumerate() {
                onset = beat.start.unwrap_or(onset);
                // Convert musical timing into a horizontal beat rectangle within this bar.
                let duration = beat.compute_quarter_beats()?;
                let bx = (onset / bar_beats.max(0.01)) as f32 * bar_width;
                let next_x = ((onset + duration) / bar_beats.max(0.01)) as f32 * bar_width;
                let top = -8.0;
                let bottom = system_height - 12.0;
                geometry.beats.push(BeatGeometry {
                    voice: voice_index,
                    beat: beat_index,
                    rect: [bx, top, next_x.max(bx + 8.0), bottom],
                    cursor_rect: [bx, 0.0, next_x.max(bx + 8.0), bottom],
                });
                // Add local tab labels and staff notes for every note in the beat.
                for note in &beat.notes {
                    if matches!(options.display, DisplayMode::Tablature | DisplayMode::Both)
                        && note.string > 0
                        && note.string <= strings
                    {
                        let label = match note.fret {
                            Fret::Number(n) | Fret::Tied(n) => n.to_string(),
                            Fret::Dead => "x".into(),
                        };
                        geometry.draw.push(Draw::Text(
                            [bx + 5.0, tab_y - y + (note.string - 1) as f32 * STRING_GAP],
                            label,
                            12.0,
                        ));
                    }
                    if matches!(options.display, DisplayMode::Staff | DisplayMode::Both) {
                        if let Some(midi) = note.midi {
                            let py =
                                4.0 * STRING_GAP - (f32::from(midi) - 60.0) * (STRING_GAP / 2.0);
                            geometry
                                .draw
                                .push(Draw::Note([bx + 5.0, py], voice_index % 2 == 1));
                        }
                    }
                }
                onset += duration;
            }
        }
        // Close the bar with vertical border lines, then compose the finished node into the scene.
        geometry
            .draw
            .push(Draw::Line([0.0, 0.0], [0.0, system_height - 12.0]));
        geometry.draw.push(Draw::Line(
            [bar_width, 0.0],
            [bar_width, system_height - 12.0],
        ));
        scene.bars.push(geometry);
        x += bar_width;
    }
    // Include the final system and bottom padding in the scene's total height.
    scene.height = (y + system_height + 20.0).max(80.0);
    Ok(scene)
}

#[cfg(test)]
mod tests {
    use super::{engrave, DisplayMode, SceneOptions};
    use crate::{Bar, Beat, Fret, Note, Track, Voice};

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
        assert_eq!(scene.bars[0].beats.len(), 2);
        assert_eq!(scene.bars[0].beats[1].voice, 0);
        assert_eq!(scene.bars[0].beats[1].beat, 1);
        assert!(scene.bars[0].beats[1].rect[0] > scene.bars[0].beats[0].rect[0]);
    }
}
