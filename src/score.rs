//! Multi-staff score models, geometry, selection, and egui interaction.
use crate::*;

/// A track with its own display and visibility settings inside a score.
/// Width, spacing and flow must match the other tracks in a call to
/// [`crate::layout_score_tracks`], because the score shares one horizontal grid.
#[derive(Clone, Debug)]
pub struct ScoreTrack {
    /// Musical contents of this staff-like track.
    pub track: Track,
    /// Per-track notation, visibility, and style settings.
    pub options: LayoutOptions,
}

/// Visual grouping for consecutive score tracks, such as a piano grand staff.
#[derive(Clone, Debug)]
pub struct InstrumentGroup {
    /// Label displayed beside the grouped tracks.
    pub name: String,
    /// Half-open range of indices in the `ScoreTrack` slice.
    pub tracks: std::ops::Range<usize>,
    /// Visual connector drawn beside the group.
    pub bracket: InstrumentBracket,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
/// Connector used to group consecutive instrument staves.
pub enum InstrumentBracket {
    /// Curved brace, commonly used for piano grand staves.
    #[default]
    Brace,
    /// Straight bracket with short horizontal terminals.
    Bracket,
    /// No connector.
    None,
}

/// One notation staff in an instrument. A staff retains the existing `Track`
/// model so single-staff callers do not need to migrate.
#[derive(Clone, Debug)]
pub struct Staff {
    /// Musical contents assigned to this staff.
    pub track: Track,
    /// Staff-specific display and engraving options.
    pub options: LayoutOptions,
    /// Optional mapping from local measures to score master-bar indices.
    /// Empty means one-to-one indexing.
    pub master_bar_map: Vec<usize>,
}

/// An instrument owns one or more staves and their visual accolade.
#[derive(Clone, Debug)]
pub struct Instrument {
    /// Name displayed beside this instrument's group.
    pub name: String,
    /// One or more staves belonging to the instrument.
    pub staves: Vec<Staff>,
    /// Connector placed beside those staves.
    pub bracket: InstrumentBracket,
}

/// A score-level bar provides the shared timeline independent of staff meter.
/// Staff measures at the same index occupy this interval.
#[derive(Clone, Copy, Debug)]
pub struct MasterBar {
    /// Absolute start on the score timeline, in quarter-note units.
    pub start_quarters: f64,
    /// Positive duration in quarter-note units.
    pub duration_quarters: f64,
}

/// A connection whose endpoints may belong to different staves.
#[derive(Clone, Debug)]
pub struct CrossStaffSpan {
    /// First beat endpoint.
    pub start: ScoreBeatAddress,
    /// Last beat endpoint.
    pub end: ScoreBeatAddress,
    /// Shape connecting the endpoints.
    pub kind: CrossStaffSpanKind,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
/// Supported connection shapes between different staves.
pub enum CrossStaffSpanKind {
    /// A shared beam with stems to both endpoints.
    Beam,
    /// A curved slur between the endpoints.
    Slur,
}

/// Staff-aware score model used when track slices are not expressive enough.
#[derive(Clone, Debug, Default)]
pub struct ScoreDocument {
    /// Shared timeline used to align differently partitioned staves.
    pub master_bars: Vec<MasterBar>,
    /// Instruments in top-to-bottom display order.
    pub instruments: Vec<Instrument>,
    /// Connections whose endpoints belong to different staves.
    pub cross_staff_spans: Vec<CrossStaffSpan>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
/// Identifies one beat in a flattened score track.
pub struct ScoreBeatAddress {
    /// Zero-based flattened track index.
    pub track: usize,
    /// Address within that track.
    pub beat: BeatAddress,
}
#[derive(Clone, Debug)]
/// Hit and cursor geometry associated with a score beat.
pub struct ScoreBeatBounds {
    /// Zero-based flattened track index.
    pub track: usize,
    /// Geometry and musical timing within that track.
    pub beat: BeatBounds,
}
#[derive(Clone, Debug)]
/// Synchronized multi-track geometry and track-aware beat bounds.
pub struct ScoreLayout {
    /// Shared geometry; `beats` live on ScoreLayout to retain track identities.
    pub geometry: Layout,
    /// Track-aware bounds used for selection and hit testing.
    pub beats: Vec<ScoreBeatBounds>,
}
impl ScoreLayout {
    /// Returns the beat containing the supplied layout-space point.
    pub fn hit_test(&self, x: f32, y: f32) -> Option<ScoreBeatAddress> {
        self.beats
            .iter()
            .find(|b| {
                [b.beat.rect, b.beat.cursor_rect]
                    .iter()
                    .any(|r| x >= r[0] && x < r[2] && y >= r[1] && y < r[3])
            })
            .map(|b| ScoreBeatAddress {
                track: b.track,
                beat: BeatAddress {
                    measure: b.beat.measure,
                    voice: b.beat.voice,
                    beat: b.beat.beat,
                },
            })
    }
    /// Serializes the score's shared geometry to SVG.
    pub fn to_svg(&self) -> String {
        self.geometry.to_svg()
    }
    /// Paints the score and highlights the supplied beat addresses.
    pub fn show(&self, ui: &mut egui::Ui, active: &[ScoreBeatAddress]) -> egui::Response {
        let (rect, response) = ui.allocate_exact_size(
            egui::vec2(self.geometry.width, self.geometry.height),
            egui::Sense::click_and_drag(),
        );
        let painter = ui.painter_at(rect);
        painter.rect_filled(rect, 0.0, self.geometry.style.background.egui());
        for b in &self.beats {
            let address = ScoreBeatAddress {
                track: b.track,
                beat: BeatAddress {
                    measure: b.beat.measure,
                    voice: b.beat.voice,
                    beat: b.beat.beat,
                },
            };
            if active.contains(&address) {
                let r = b.beat.cursor_rect;
                let cursor = egui::Rect::from_min_max(
                    rect.min + egui::vec2(r[0], r[1]),
                    rect.min + egui::vec2(r[2], r[3]),
                );
                painter.rect_filled(cursor, 0.0, self.geometry.style.selection.egui());
                painter.vline(
                    cursor.center().x,
                    cursor.y_range(),
                    egui::Stroke::new(2.0, self.geometry.style.cursor.egui()),
                );
            }
        }
        self.geometry.paint_primitives(&painter, rect.min);
        response
    }
}

#[derive(Clone, Copy, Debug)]
/// Inclusive range of beats selected in a score.
pub struct ScoreSelection {
    /// Fixed endpoint where the selection began.
    pub anchor: ScoreBeatAddress,
    /// Movable endpoint of the selection.
    pub end: ScoreBeatAddress,
}
impl ScoreSelection {
    /// Select a time range across all displayed tracks and voices.
    pub fn contains(self, layout: &ScoreLayout, address: ScoreBeatAddress) -> bool {
        let key = |a| layout.bounds(a).map(|b| (b.measure, b.start));
        match (key(self.anchor), key(self.end), key(address)) {
            (Some(a), Some(b), Some(c)) => {
                let (lo, hi) = if a <= b { (a, b) } else { (b, a) };
                c >= lo && c <= hi
            }
            _ => false,
        }
    }
}
/// Result of one interactive score-layout pass.
pub struct ScoreInteraction {
    /// egui response allocated for the score.
    pub response: egui::Response,
    /// Beat clicked during the interaction, if any.
    pub clicked: Option<ScoreBeatAddress>,
    /// Beat under the pointer during the interaction, if any.
    pub hovered: Option<ScoreBeatAddress>,
}
impl ScoreLayout {
    pub(crate) fn bounds(&self, a: ScoreBeatAddress) -> Option<&BeatBounds> {
        self.beats
            .iter()
            .find(|b| {
                b.track == a.track
                    && (b.beat.measure, b.beat.voice, b.beat.beat)
                        == (a.beat.measure, a.beat.voice, a.beat.beat)
            })
            .map(|b| &b.beat)
    }
    /// Handles click, hover, drag, and Shift-click selection for the score.
    pub fn show_interactive(
        &self,
        ui: &mut egui::Ui,
        active: &[ScoreBeatAddress],
        selection: &mut Option<ScoreSelection>,
    ) -> ScoreInteraction {
        let response = self.show(ui, active);
        let at = response.hover_pos().and_then(|p| {
            let p = p - response.rect.min;
            self.hit_test(p.x, p.y)
        });
        let clicked = response.clicked().then_some(at).flatten();
        if let Some(address) = clicked.or_else(|| response.drag_started().then_some(at).flatten()) {
            if ui.input(|i| i.modifiers.shift) {
                if let Some(selection) = selection.as_mut() {
                    selection.end = address;
                } else {
                    *selection = Some(ScoreSelection {
                        anchor: address,
                        end: address,
                    });
                }
            } else {
                *selection = Some(ScoreSelection {
                    anchor: address,
                    end: address,
                });
            }
        }
        if response.dragged() {
            if let (Some(s), Some(a)) = (selection.as_mut(), at) {
                s.end = a;
            }
        }
        if let Some(selected) = selection {
            for b in &self.beats {
                let address = ScoreBeatAddress {
                    track: b.track,
                    beat: BeatAddress {
                        measure: b.beat.measure,
                        voice: b.beat.voice,
                        beat: b.beat.beat,
                    },
                };
                if selected.contains(self, address) {
                    let r = b.beat.cursor_rect;
                    ui.painter_at(response.rect).rect_stroke(
                        egui::Rect::from_min_max(
                            response.rect.min + egui::vec2(r[0], r[1]),
                            response.rect.min + egui::vec2(r[2], r[3]),
                        ),
                        0.0,
                        egui::Stroke::new(2.0, self.geometry.style.cursor.egui()),
                        egui::StrokeKind::Inside,
                    );
                }
            }
        }
        ScoreInteraction {
            response,
            clicked,
            hovered: at,
        }
    }
    /// Paints an interpolated playback cursor for a score beat.
    pub fn paint_playback_cursor(
        &self,
        ui: &mut egui::Ui,
        response: &egui::Response,
        address: ScoreBeatAddress,
        fraction: f32,
        follow: bool,
    ) {
        if let Some(b) = self.bounds(address) {
            let r = b.cursor_rect;
            let rect = egui::Rect::from_min_max(
                response.rect.min + egui::vec2(r[0], r[1]),
                response.rect.min + egui::vec2(r[2], r[3]),
            );
            let fraction = if fraction.is_finite() {
                fraction.clamp(0.0, 1.0)
            } else {
                0.0
            };
            ui.painter_at(response.rect).vline(
                egui::lerp(rect.x_range(), fraction),
                rect.y_range(),
                egui::Stroke::new(2.0, self.geometry.style.cursor.egui()),
            );
            if follow {
                ui.scroll_to_rect(rect, Some(egui::Align::Center));
            }
        }
    }
    /// Returns a uniformly scaled copy of this score layout.
    pub fn scaled(&self, zoom: f32) -> Result<Self, RenderError> {
        let mut result = self.clone();
        result.geometry = self.geometry.scaled(zoom)?;
        for b in &mut result.beats {
            for value in b.beat.rect.iter_mut().chain(b.beat.cursor_rect.iter_mut()) {
                *value *= zoom;
            }
        }
        Ok(result)
    }
    /// Splits the score into page-sized layouts while preserving beat addresses.
    pub fn paginate(&self, page_height: f32) -> Result<Vec<Self>, RenderError> {
        let pages = self.geometry.paginate(page_height)?;
        let mut first_system = 0;
        let mut result = Vec::new();
        for geometry in pages {
            let range = if geometry.systems.is_empty() {
                [0.0, self.geometry.height]
            } else {
                let start = self.geometry.systems[first_system][0];
                first_system += geometry.systems.len();
                [start, self.geometry.systems[first_system - 1][1]]
            };
            let beats = self
                .beats
                .iter()
                .filter(|b| b.beat.cursor_rect[1] >= range[0] && b.beat.cursor_rect[1] < range[1])
                .cloned()
                .map(|mut b| {
                    for r in [&mut b.beat.rect, &mut b.beat.cursor_rect] {
                        r[1] -= range[0];
                        r[3] -= range[0];
                    }
                    b
                })
                .collect();
            result.push(Self { geometry, beats });
        }
        Ok(result)
    }
}
