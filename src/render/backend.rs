//! egui and SVG backends for completed layouts.
use super::*;

impl Layout {
    /// Paint the page, marking every active beat with a blue playback cursor.
    pub fn show_with_playback_cursor(
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
                    egui::Stroke::new(2.0, self.style.cursor.egui()),
                );
            }
        }

        self.paint_primitives(&painter, rect.min);
        response
    }

    pub(crate) fn paint_primitives(&self, painter: &egui::Painter, origin: egui::Pos2) {
        let pos = |p: [f32; 2]| origin + egui::vec2(p[0], p[1]);
        for primitive in &self.primitives {
            let [_, top, _, bottom] = primitive.bounds();
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
                            bounds.expand2(egui::vec2(3.0, 0.0)),
                            0.0,
                            self.style.background.egui(),
                        );
                    }
                    painter.galley(bounds.min, galley, color);
                }
            }
        }
    }
    /// Serializes the shared primitive geometry to SVG.
    pub fn to_svg(&self) -> String {
        let mut svg = format!("<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{}\" height=\"{}\" viewBox=\"0 0 {} {}\"><rect width=\"100%\" height=\"100%\" fill=\"{}\"/>", self.width, self.height, self.width, self.height, self.style.background.svg());
        for primitive in &self.primitives {
            match primitive {
                Primitive::Glyph {
                    at,
                    space,
                    outline,
                    code,
                    color,
                } => {
                    write!(svg, "<path data-smufl=\"{:04X}\" transform=\"translate({} {}) scale({})\" d=\"{}\" fill=\"{}\"/>", *code as u32, at[0], at[1], space, outline.svg, color.unwrap_or(self.style.glyph_color()).svg()).unwrap();
                }

                Primitive::Line {
                    from,
                    to,
                    width,
                    color,
                } => {
                    write!(svg, "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"{}\" stroke-width=\"{}\"/>", from[0], from[1], to[0], to[1], color.unwrap_or(self.style.line_color()).svg(), width).unwrap();
                }
                Primitive::Curve {
                    points,
                    width,
                    color,
                } => {
                    write!(svg, "<path d=\"M{} {} C{} {},{} {},{} {}\" fill=\"none\" stroke=\"{}\" stroke-width=\"{}\"/>", points[0][0], points[0][1], points[1][0], points[1][1], points[2][0], points[2][1], points[3][0], points[3][1], color.unwrap_or(self.style.line_color()).svg(), width).unwrap();
                }
                Primitive::Text {
                    at,
                    text,
                    size,
                    masked,
                    color,
                } => {
                    if *masked {
                        let w = crate::text::width(text, *size) + 6.0;
                        write!(
                            svg,
                            "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" fill=\"{}\"/>",
                            at[0] - w / 2.0,
                            at[1] - size * 0.6,
                            w,
                            size * 1.2,
                            self.style.background.svg()
                        )
                        .unwrap();
                    }
                    write!(svg, "<text x=\"{}\" y=\"{}\" font-family=\"sans-serif\" font-size=\"{}\" textLength=\"{}\" lengthAdjust=\"spacingAndGlyphs\" text-anchor=\"middle\" dominant-baseline=\"central\" fill=\"{}\">{}</text>", at[0], at[1], size, crate::text::width(text, *size), color.unwrap_or(self.style.text_color()).svg(), escape_xml(text)).unwrap();
                }
            }
        }
        svg.push_str("</svg>");
        svg
    }

    /// Allocate and paint this layout inside a caller-owned scroll area.
    pub fn show(&self, ui: &mut egui::Ui) -> egui::Response {
        self.show_with_playback_cursor(ui, &[])
    }
}

fn escape_xml(text: &str) -> String {
    text.chars().map(|c| if matches!(c, '\t' | '\n' | '\r' | '\u{20}'..='\u{D7FF}' | '\u{E000}'..='\u{FFFD}' | '\u{10000}'..='\u{10FFFF}') { c } else { '\u{FFFD}' }).collect::<String>().replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}
