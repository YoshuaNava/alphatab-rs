//! egui selection, hit-testing, playback cursors, and scroll-following.
use super::EguiRenderer;
use crate::{BeatAddress, Scene};

/// Inclusive musical range; addresses remain stable across zoom and reflow.
#[derive(Clone, Copy, Debug)]
pub struct Selection {
    /// Fixed endpoint where selection began.
    pub anchor: BeatAddress,
    /// Movable endpoint most recently selected.
    pub end: BeatAddress,
}
impl Selection {
    /// Returns whether an address lies within this selection's musical range.
    pub fn contains(self, scene: &Scene, address: BeatAddress) -> bool {
        let key = |a: BeatAddress| {
            scene
                .beats
                .iter()
                .find(|b| (b.measure, b.voice, b.beat) == (a.measure, a.voice, a.beat))
                .map(|b| (b.measure, b.start))
        };
        match (key(self.anchor), key(self.end), key(address)) {
            (Some(a), Some(b), Some(c)) => {
                let (lo, hi) = if a <= b { (a, b) } else { (b, a) };
                c >= lo && c <= hi
            }
            _ => false,
        }
    }
}
/// Result of one interactive egui scene pass.
pub struct Interaction {
    /// The allocated egui response for the whole scene.
    pub response: egui::Response,
    /// Beat clicked during this pass, if any.
    pub clicked: Option<BeatAddress>,
    /// Beat under the pointer during this pass, if any.
    pub hovered: Option<BeatAddress>,
}
/// egui input adapter for a completed backend-neutral [`Scene`].
#[derive(Clone, Copy, Debug)]
pub struct EguiInteraction<'scene> {
    scene: &'scene Scene,
}

impl<'scene> EguiInteraction<'scene> {
    /// Creates an input adapter for `scene`.
    pub fn new(scene: &'scene Scene) -> Self {
        Self { scene }
    }

    /// Use click and shift-click for beat range selection. The caller owns state.
    pub fn handle(
        &self,
        ui: &mut egui::Ui,
        active: &[BeatAddress],
        selection: &mut Option<Selection>,
    ) -> Interaction {
        let response = EguiRenderer::new(self.scene).paint_with_playback_cursor(ui, active);
        let at = response
            .hover_pos()
            .and_then(|p| {
                let p = p - response.rect.min;
                self.hit_test(p.x, p.y).or_else(|| {
                    self.beats.iter().find(|b| {
                        p.x >= b.cursor_rect[0]
                            && p.x < b.cursor_rect[2]
                            && p.y >= b.cursor_rect[1]
                            && p.y < b.cursor_rect[3]
                    })
                })
            })
            .map(|b| BeatAddress {
                measure: b.measure,
                voice: b.voice,
                beat: b.beat,
            });
        let clicked = if response.clicked() { at } else { None };
        if let Some(address) = clicked {
            if ui.input(|i| i.modifiers.shift) {
                if let Some(s) = selection {
                    s.end = address;
                } else {
                    *selection = Some(Selection {
                        anchor: address,
                        end: address,
                    });
                }
            } else {
                *selection = Some(Selection {
                    anchor: address,
                    end: address,
                });
            }
        }
        if response.drag_started() {
            if let Some(address) = at {
                *selection = Some(Selection {
                    anchor: address,
                    end: address,
                });
            }
        }
        if response.dragged() {
            if let (Some(s), Some(address)) = (selection.as_mut(), at) {
                s.end = address;
            }
        }
        if let Some(selected) = selection {
            let painter = ui.painter_at(response.rect);
            for b in &self.beats {
                if selected.contains(
                    self,
                    BeatAddress {
                        measure: b.measure,
                        voice: b.voice,
                        beat: b.beat,
                    },
                ) {
                    let rect = egui::Rect::from_min_max(
                        response.rect.min + egui::vec2(b.cursor_rect[0], b.cursor_rect[1]),
                        response.rect.min + egui::vec2(b.cursor_rect[2], b.cursor_rect[3]),
                    );
                    painter.rect_stroke(
                        rect,
                        0.0,
                        egui::Stroke::new(2.0, self.style.cursor.egui()),
                        egui::StrokeKind::Inside,
                    );
                }
            }
        }
        Interaction {
            response,
            clicked,
            hovered: at,
        }
    }

    /// Follow a beat in a caller-owned scroll area and interpolate its cursor.
    pub fn paint_following(
        &self,
        ui: &mut egui::Ui,
        address: BeatAddress,
        fraction: f32,
        follow: bool,
    ) -> egui::Response {
        let response = EguiRenderer::new(self.scene).paint(ui);
        self.paint_playback_cursor(ui, &response, address, fraction, follow);
        response
    }

    /// Add an interpolated cursor to an already painted (including interactive) layout.
    pub fn paint_playback_cursor(
        &self,
        ui: &mut egui::Ui,
        response: &egui::Response,
        address: BeatAddress,
        fraction: f32,
        follow: bool,
    ) {
        if let Some(b) = self.beats.iter().find(|b| {
            (b.measure, b.voice, b.beat) == (address.measure, address.voice, address.beat)
        }) {
            let rect = egui::Rect::from_min_max(
                response.rect.min + egui::vec2(b.cursor_rect[0], b.cursor_rect[1]),
                response.rect.min + egui::vec2(b.cursor_rect[2], b.cursor_rect[3]),
            );
            let progress = if fraction.is_finite() {
                fraction.clamp(0.0, 1.0)
            } else {
                0.0
            };
            ui.painter_at(response.rect).vline(
                egui::lerp(rect.x_range(), progress),
                rect.y_range(),
                egui::Stroke::new(2.0, self.style.cursor.egui()),
            );
            if follow {
                ui.scroll_to_rect(rect, Some(egui::Align::Center));
            }
        }
    }
}

impl std::ops::Deref for EguiInteraction<'_> {
    type Target = Scene;

    fn deref(&self) -> &Self::Target {
        self.scene
    }
}
