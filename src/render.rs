//! Egui painting and interaction for a laid-out scene.

use egui::{Color32, Rect, Sense, Stroke, Vec2};

use crate::{scene::Draw, BeatAddress, CoordinateFrame, Scene};

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
        let window_frame = CoordinateFrame::create_at([response.rect.min.x, response.rect.min.y]);
        let address = response.hover_pos().and_then(|point| {
            self.hit_test(Vec2::from(
                window_frame.convert_point_from_parent([point.x, point.y]),
            ))
        });
        if response.clicked() {
            if let Some(address) = address {
                if ui.input(|input| input.modifiers.shift) {
                    if let Some(selection) = selection {
                        selection.end = address;
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
        }
        Interaction {
            response,
            clicked: if ui.ctx().input(|input| input.pointer.any_released()) {
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
        if let Some(beat) = self.scene.beats.iter().find(|beat| {
            (beat.bar, beat.voice, beat.beat) == (address.bar, address.voice, address.beat)
        }) {
            let rect = Rect::from_min_max(
                response.rect.min + Vec2::new(beat.cursor_rect[0], beat.cursor_rect[1]),
                response.rect.min + Vec2::new(beat.cursor_rect[2], beat.cursor_rect[3]),
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

    fn hit_test(&self, position: Vec2) -> Option<BeatAddress> {
        self.scene
            .beats
            .iter()
            .find(|beat| {
                position.x >= beat.rect[0]
                    && position.x <= beat.rect[2]
                    && position.y >= beat.rect[1]
                    && position.y <= beat.rect[3]
            })
            .map(|beat| BeatAddress {
                bar: beat.bar,
                voice: beat.voice,
                beat: beat.beat,
            })
    }

    fn paint(&self, ui: &mut egui::Ui, active: &[BeatAddress]) -> egui::Response {
        let (rect, response) = ui.allocate_exact_size(
            Vec2::new(self.scene.width, self.scene.height),
            Sense::click_and_drag(),
        );
        let painter = ui.painter_at(rect);
        let foreground = ui.visuals().text_color();
        for draw in &self.scene.draw {
            match draw {
                Draw::Line(start, end) => {
                    painter.line_segment(
                        [rect.min + Vec2::from(*start), rect.min + Vec2::from(*end)],
                        Stroke::new(1.0, foreground),
                    );
                }
                Draw::Text(position, text, size) => {
                    painter.text(
                        rect.min + Vec2::from(*position),
                        egui::Align2::CENTER_CENTER,
                        text,
                        egui::FontId::proportional(*size),
                        foreground,
                    );
                }
                Draw::Note(position, stem_down) => {
                    let center = rect.min + Vec2::from(*position);
                    painter.circle_filled(center, 4.0, foreground);
                    let stem = if *stem_down { -20.0 } else { 20.0 };
                    painter.line_segment(
                        [center + Vec2::new(4.0, 0.0), center + Vec2::new(4.0, stem)],
                        Stroke::new(1.0, foreground),
                    );
                }
            }
        }
        for address in active {
            if let Some(beat) = self.scene.beats.iter().find(|beat| {
                (beat.bar, beat.voice, beat.beat) == (address.bar, address.voice, address.beat)
            }) {
                painter.rect_filled(
                    Rect::from_min_max(
                        rect.min + Vec2::new(beat.rect[0], beat.rect[1]),
                        rect.min + Vec2::new(beat.rect[2], beat.rect[3]),
                    ),
                    0.0,
                    Color32::from_rgba_unmultiplied(90, 170, 255, 40),
                );
            }
        }
        response
    }
}
