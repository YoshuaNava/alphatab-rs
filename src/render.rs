//! Egui painting and interaction for a laid-out scene.

use egui::{Color32, Rect, Sense, Stroke, Vec2};

use crate::{
    scene::{CoordinateFrame, Draw},
    BeatAddress, Scene,
};

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
        if let Some((bar, beat)) = self.scene.find_beat(address) {
            let origin = Vec2::from(bar.frame.convert_point_to_parent([0.0, 0.0]));
            let rect = Rect::from_min_max(
                response.rect.min + origin + Vec2::new(beat.cursor_rect[0], beat.cursor_rect[1]),
                response.rect.min + origin + Vec2::new(beat.cursor_rect[2], beat.cursor_rect[3]),
            );
            let x = egui::lerp(rect.x_range(), fraction.clamp(0.0, 1.0));
            ui.painter().vline(
                x,
                rect.y_range(),
                Stroke::new(
                    self.scene.render.cursor_stroke_width,
                    Color32::from_rgb(
                        self.scene.render.cursor_color[0],
                        self.scene.render.cursor_color[1],
                        self.scene.render.cursor_color[2],
                    ),
                ),
            );
            if follow {
                ui.scroll_to_rect(rect, Some(egui::Align::Center));
            }
        }
    }

    fn hit_test(&self, position: Vec2) -> Option<BeatAddress> {
        self.scene
            .find_beat_at_scene_point([position.x, position.y])
    }

    fn paint(&self, ui: &mut egui::Ui, active: &[BeatAddress]) -> egui::Response {
        let (rect, response) = ui.allocate_exact_size(
            Vec2::new(self.scene.width, self.scene.height),
            Sense::click_and_drag(),
        );
        let painter = ui.painter_at(rect);
        let foreground = ui.visuals().text_color();
        let voice_symbol_color = ui.visuals().strong_text_color();
        let score_color = Color32::from_rgba_unmultiplied(
            foreground.r(),
            foreground.g(),
            foreground.b(),
            self.scene.render.score_line_opacity,
        );
        for bar in &self.scene.bars {
            let origin = rect.min + Vec2::from(bar.frame.convert_point_to_parent([0.0, 0.0]));
            for draw in &bar.draw {
                match draw {
                    Draw::Line(start, end) => {
                        painter.line_segment(
                            [origin + Vec2::from(*start), origin + Vec2::from(*end)],
                            Stroke::new(self.scene.render.notation_stroke_width, score_color),
                        );
                    }
                    Draw::Text(position, text, size) => {
                        painter.text(
                            origin + Vec2::from(*position),
                            egui::Align2::CENTER_CENTER,
                            text,
                            egui::FontId::proportional(*size),
                            voice_symbol_color,
                        );
                    }
                    Draw::BarNumber(position, text, size) => {
                        painter.text(
                            origin + Vec2::from(*position),
                            egui::Align2::CENTER_CENTER,
                            text,
                            egui::FontId::proportional(*size),
                            score_color,
                        );
                    }
                    Draw::Note(position, stem_down) => {
                        let center = origin + Vec2::from(*position);
                        painter.circle_filled(
                            center,
                            self.scene.render.note_head_radius,
                            voice_symbol_color,
                        );
                        let stem = if *stem_down {
                            -self.scene.render.stem_length
                        } else {
                            self.scene.render.stem_length
                        };
                        painter.line_segment(
                            [center + Vec2::new(4.0, 0.0), center + Vec2::new(4.0, stem)],
                            Stroke::new(
                                self.scene.render.notation_stroke_width,
                                voice_symbol_color,
                            ),
                        );
                    }
                }
            }
        }
        for address in active {
            if let Some((bar, beat)) = self.scene.find_beat(*address) {
                let origin = rect.min + Vec2::from(bar.frame.convert_point_to_parent([0.0, 0.0]));
                painter.rect_filled(
                    Rect::from_min_max(
                        origin + Vec2::new(beat.rect[0], beat.rect[1]),
                        origin + Vec2::new(beat.rect[2], beat.rect[3]),
                    ),
                    0.0,
                    Color32::from_rgba_unmultiplied(
                        self.scene.render.active_beat_color[0],
                        self.scene.render.active_beat_color[1],
                        self.scene.render.active_beat_color[2],
                        self.scene.render.active_beat_color[3],
                    ),
                );
            }
        }
        response
    }
}
