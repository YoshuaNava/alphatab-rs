//! egui backend for completed scenes.
use crate::*;

const CURSOR_STROKE_WIDTH: f32 = 2.0;
const MASK_HORIZONTAL_PADDING: f32 = 3.0;

impl Color {
    pub(crate) fn egui(self) -> egui::Color32 {
        egui::Color32::from_rgba_unmultiplied(self.r, self.g, self.b, self.a)
    }
}

/// egui adapter for a completed backend-neutral [`Scene`].
#[derive(Clone, Copy, Debug)]
pub struct EguiRenderer<'scene> {
    scene: &'scene Scene,
}

impl<'scene> EguiRenderer<'scene> {
    /// Creates an egui adapter for `scene`.
    pub fn new(scene: &'scene Scene) -> Self {
        Self { scene }
    }

    /// Paint the page, marking every active beat with a blue playback cursor.
    pub fn paint_with_playback_cursor(
        &self,
        ui: &mut egui::Ui,
        active_beats: &[BeatAddress],
    ) -> egui::Response {
        let (rect, response) = ui.allocate_exact_size(
            egui::vec2(self.width, self.height),
            egui::Sense::click_and_drag(),
        );
        let painter = ui.painter_at(rect);
        painter.rect_filled(rect, 0.0, self.style.background.egui());
        let pos = |p: [f32; 2]| rect.min + egui::vec2(p[0], p[1]);

        for beat in &self.beats {
            let address = BeatAddress {
                measure: beat.measure,
                voice: beat.voice,
                beat: beat.beat,
            };
            if active_beats.contains(&address) {
                let cursor = egui::Rect::from_min_max(
                    pos([beat.cursor_rect[0], beat.cursor_rect[1]]),
                    pos([beat.cursor_rect[2], beat.cursor_rect[3]]),
                );
                painter.rect_filled(cursor, 0.0, self.style.selection.egui());
                painter.vline(
                    cursor.center().x,
                    cursor.y_range(),
                    egui::Stroke::new(CURSOR_STROKE_WIDTH, self.style.cursor.egui()),
                );
            }
        }

        self.paint_primitives(&painter, rect.min);
        response
    }

    pub(crate) fn paint_primitives(&self, painter: &egui::Painter, origin: egui::Pos2) {
        let pos = |p: [f32; 2]| origin + egui::vec2(p[0], p[1]);
        for primitive in &self.primitives {
            let [_, top, _, bottom] = primitive.compute_bounds();
            if origin.y + bottom < painter.clip_rect().top()
                || origin.y + top > painter.clip_rect().bottom()
            {
                continue;
            }

            match primitive {
                Primitive::Glyph {
                    at,
                    space,
                    outline,
                    color,
                    ..
                } => {
                    let color = color.unwrap_or(self.style.glyph_color()).egui();
                    let mesh = egui::Mesh {
                        vertices: outline
                            .vertices
                            .iter()
                            .map(|v| egui::epaint::Vertex {
                                pos: pos([at[0] + v[0] * space, at[1] + v[1] * space]),
                                uv: egui::epaint::WHITE_UV,
                                color,
                            })
                            .collect(),
                        indices: outline.indices.clone(),
                        ..Default::default()
                    };
                    painter.add(egui::Shape::mesh(mesh));
                }

                Primitive::Line {
                    from,
                    to,
                    width,
                    color,
                } => {
                    painter.line_segment(
                        [pos(*from), pos(*to)],
                        egui::Stroke::new(*width, color.unwrap_or(self.style.line_color()).egui()),
                    );
                }
                Primitive::Curve {
                    points,
                    width,
                    color,
                } => {
                    painter.add(egui::Shape::CubicBezier(
                        egui::epaint::CubicBezierShape::from_points_stroke(
                            points.map(pos),
                            false,
                            egui::Color32::TRANSPARENT,
                            egui::Stroke::new(
                                *width,
                                color.unwrap_or(self.style.line_color()).egui(),
                            ),
                        ),
                    ));
                }
                Primitive::Text {
                    at,
                    text,
                    size,
                    masked,
                    color,
                } => {
                    let color = color.unwrap_or(self.style.text_color()).egui();
                    let galley = painter.layout_no_wrap(
                        text.clone(),
                        egui::FontId::proportional(*size),
                        color,
                    );
                    let bounds = egui::Rect::from_center_size(pos(*at), galley.size());
                    if *masked {
                        painter.rect_filled(
                            bounds.expand2(egui::vec2(MASK_HORIZONTAL_PADDING, 0.0)),
                            0.0,
                            self.style.background.egui(),
                        );
                    }
                    painter.galley(bounds.min, galley, color);
                }
            }
        }
    }
    /// Allocates and paints the scene inside a caller-owned scroll area.
    pub fn paint(&self, ui: &mut egui::Ui) -> egui::Response {
        self.paint_with_playback_cursor(ui, &[])
    }
}

impl std::ops::Deref for EguiRenderer<'_> {
    type Target = Scene;

    fn deref(&self) -> &Self::Target {
        self.scene
    }
}
