//! Backend-neutral scene layout and geometry.

use crate::{BeatAddress, Fret, RenderError, Track};

/// Values controlling score layout and egui painting.
#[derive(Clone, Copy, Debug)]
pub struct RenderOptions {
    /// Smallest supported scene width.
    pub minimum_scene_width: f32,
    /// Smallest usable width after margins are removed.
    pub minimum_content_width: f32,
    /// Top offset of the first row of bars.
    pub first_bar_row_y: f32,
    /// Extra vertical space included in every bar row.
    pub bar_row_padding: f32,
    /// Bottom inset between a bar's extent and its row extent.
    pub bar_bottom_inset: f32,
    /// Left and right score margins.
    pub horizontal_margin: f32,
    /// Vertical gap between consecutive rows of bars.
    pub bar_row_gap: f32,
    /// Distance between staff lines or tab strings.
    pub line_gap: f32,
    /// Number of bars per system.
    pub bars_per_system: usize,
    /// Number of staff lines.
    pub staff_line_count: usize,
    /// Quarter notes in a whole note.
    pub quarter_notes_per_whole: f64,
    /// Vertical gap between staff and tablature when both display modes are shown.
    pub display_modes_gap: f32,
    /// Horizontal note and fret-label inset.
    pub note_x_inset: f32,
    /// Fret-label font size.
    pub fret_label_font_size: f32,
    /// MIDI pitch placed on the staff reference line.
    pub staff_reference_midi: u8,
    /// Vertical distance between adjacent staff pitch steps.
    pub staff_pitch_step: f32,
    /// Bar-number horizontal inset, vertical offset, and font size.
    pub bar_number_x_inset: f32,
    /// Bar-number vertical offset.
    pub bar_number_y_offset: f32,
    /// Bar-number font size.
    pub bar_number_font_size: f32,
    /// Extra hit area above a bar.
    pub beat_hit_top_padding: f32,
    /// Minimum width of a beat hit area.
    pub minimum_beat_width: f32,
    /// Smallest bar duration used to avoid division by zero during layout.
    pub minimum_bar_duration: f64,
    /// Note-head radius.
    pub note_head_radius: f32,
    /// Note-stem length.
    pub stem_length: f32,
    /// Notation line width.
    pub notation_stroke_width: f32,
    /// Opacity of staff, tab, and bar lines relative to the egui foreground colour.
    pub score_line_opacity: u8,
    /// Playback-cursor line width.
    pub cursor_stroke_width: f32,
    /// Playback cursor colour.
    pub cursor_color: [u8; 3],
    /// Active-beat highlight colour.
    pub active_beat_color: [u8; 4],
}

impl Default for RenderOptions {
    fn default() -> Self {
        Self {
            minimum_scene_width: 160.0,
            minimum_content_width: 80.0,
            first_bar_row_y: 28.0,
            bar_row_padding: 30.0,
            bar_bottom_inset: 12.0,
            horizontal_margin: 46.0,
            bar_row_gap: 36.0,
            line_gap: 13.0,
            bars_per_system: 3,
            staff_line_count: 5,
            quarter_notes_per_whole: 4.0,
            display_modes_gap: 24.0,
            note_x_inset: 5.0,
            fret_label_font_size: 12.0,
            staff_reference_midi: 60,
            staff_pitch_step: 6.5,
            bar_number_x_inset: 4.0,
            bar_number_y_offset: -11.0,
            bar_number_font_size: 11.0,
            beat_hit_top_padding: 8.0,
            minimum_beat_width: 8.0,
            minimum_bar_duration: 0.01,
            note_head_radius: 4.0,
            stem_length: 20.0,
            notation_stroke_width: 1.0,
            score_line_opacity: 120,
            cursor_stroke_width: 2.0,
            cursor_color: [30, 105, 190],
            active_beat_color: [90, 170, 255, 40],
        }
    }
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
    /// Layout and visual values.
    pub render: RenderOptions,
}

impl Default for SceneOptions {
    fn default() -> Self {
        Self {
            width: 900.0,
            display: DisplayMode::Tablature,
            render: RenderOptions::default(),
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
    BarNumber([f32; 2], String, f32),
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
    /// Options used for rendering the scene
    pub(crate) render: RenderOptions,
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
                    Draw::BarNumber(p, _, size) | Draw::Text(p, _, size) => {
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
    if !options.width.is_finite() || options.width < options.render.minimum_scene_width {
        return Err(RenderError("width is below the configured minimum".into()));
    }

    // Derive the fixed vertical measurements shared by every bar in this scene.
    let render = options.render;
    let strings = track.strings.len().max(1);
    let staff_height = (render.staff_line_count.saturating_sub(1) as f32) * render.line_gap;
    let tab_height = (strings.saturating_sub(1) as f32) * render.line_gap;
    let system_height = match options.display {
        DisplayMode::Tablature => tab_height,
        DisplayMode::Staff => staff_height,
        DisplayMode::Both => staff_height + render.display_modes_gap + tab_height,
    } + render.bar_row_padding;

    // Create the scene composer. `x` and `y` place bar frames in scene coordinates.
    let available =
        (options.width - 2.0 * render.horizontal_margin).max(render.minimum_content_width);
    let mut scene = Scene {
        width: options.width,
        height: 0.0,
        render,
        bars: vec![],
    };
    let mut x = render.horizontal_margin;
    let mut y = render.first_bar_row_y;
    for (bar_index, bar) in track.bars.iter().enumerate() {
        // Give each bar one third of a system and start a new system when it would overflow.
        let bar_width = available / render.bars_per_system.max(1) as f32;
        if x > render.horizontal_margin && x + bar_width > options.width - render.horizontal_margin
        {
            x = render.horizontal_margin;
            y += system_height + render.bar_row_gap;
        }

        // Start a new bar node. Everything added below uses this bar's local origin `(0, 0)`.
        let bar_frame = CoordinateFrame::create_at([x, y]);
        let mut geometry = BarGeometry {
            bar: bar_index,
            frame: bar_frame,
            size: [bar_width, system_height - render.bar_bottom_inset],
            beats: vec![],
            draw: vec![],
        };

        // Add the staff and/or tab staff lines in bar-local coordinates.
        let tab_y = y + if options.display == DisplayMode::Both {
            staff_height + render.display_modes_gap
        } else {
            0.0
        };
        if matches!(options.display, DisplayMode::Tablature | DisplayMode::Both) {
            for string in 0..strings {
                let sy = tab_y + string as f32 * render.line_gap;
                geometry
                    .draw
                    .push(Draw::Line([0.0, sy - y], [bar_width, sy - y]));
            }
        }
        if matches!(options.display, DisplayMode::Staff | DisplayMode::Both) {
            for line in 0..render.staff_line_count {
                let sy = y + line as f32 * render.line_gap;
                geometry
                    .draw
                    .push(Draw::Line([0.0, sy - y], [bar_width, sy - y]));
            }
        }

        // Label the bar and calculate its musical duration in quarter-note units.
        geometry.draw.push(Draw::BarNumber(
            [render.bar_number_x_inset, render.bar_number_y_offset],
            (bar_index + 1).to_string(),
            render.bar_number_font_size,
        ));
        let bar_beats = f64::from(bar.time_signature.0) * render.quarter_notes_per_whole
            / f64::from(bar.time_signature.1.max(1));
        for (voice_index, voice) in bar.voices.iter().enumerate() {
            // Lay out each voice independently; voices share the same bar-local frame.
            let mut onset = 0.0;
            for (beat_index, beat) in voice.beats.iter().enumerate() {
                onset = beat.start.unwrap_or(onset);
                // Convert musical timing into a horizontal beat rectangle within this bar.
                let duration = beat.compute_quarter_beats()?;
                let bx = (onset / bar_beats.max(render.minimum_bar_duration)) as f32 * bar_width;
                let next_x = ((onset + duration) / bar_beats.max(render.minimum_bar_duration))
                    as f32
                    * bar_width;
                let top = -render.beat_hit_top_padding;
                let bottom = system_height - render.bar_bottom_inset;
                geometry.beats.push(BeatGeometry {
                    voice: voice_index,
                    beat: beat_index,
                    rect: [bx, top, next_x.max(bx + render.minimum_beat_width), bottom],
                    cursor_rect: [bx, 0.0, next_x.max(bx + render.minimum_beat_width), bottom],
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
                            [
                                bx + render.note_x_inset,
                                tab_y - y + (note.string - 1) as f32 * render.line_gap,
                            ],
                            label,
                            render.fret_label_font_size,
                        ));
                    }
                    if matches!(options.display, DisplayMode::Staff | DisplayMode::Both) {
                        if let Some(midi) = note.midi {
                            let py = staff_height
                                - (f32::from(midi) - f32::from(render.staff_reference_midi))
                                    * render.staff_pitch_step;
                            geometry.draw.push(Draw::Note(
                                [bx + render.note_x_inset, py],
                                voice_index % 2 == 1,
                            ));
                        }
                    }
                }
                onset += duration;
            }
        }
        // Close the bar with vertical border lines, then compose the finished node into the scene.
        geometry.draw.push(Draw::Line(
            [0.0, 0.0],
            [0.0, system_height - render.bar_bottom_inset],
        ));
        geometry.draw.push(Draw::Line(
            [bar_width, 0.0],
            [bar_width, system_height - render.bar_bottom_inset],
        ));
        scene.bars.push(geometry);
        x += bar_width;
    }
    // Include the final system and bottom padding in the scene's total height.
    scene.height = (y + system_height + render.bar_row_padding).max(render.minimum_content_width);
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
