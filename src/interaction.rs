//! Selection, hit-testing, playback cursors, zoom, and pagination helpers.
use crate::*;

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
/// Result of one interactive egui layout pass.
pub struct Interaction {
    /// The allocated egui response for the whole layout.
    pub response: egui::Response,
    /// Beat clicked during this pass, if any.
    pub clicked: Option<BeatAddress>,
    /// Beat under the pointer during this pass, if any.
    pub hovered: Option<BeatAddress>,
}
impl Scene {
    /// Use click and shift-click for beat range selection. The caller owns state.
    pub fn show_interactive(
        &self,
        ui: &mut egui::Ui,
        active: &[BeatAddress],
        selection: &mut Option<Selection>,
    ) -> Interaction {
        let response = self.show_with_playback_cursor(ui, active);
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
    pub fn show_following(
        &self,
        ui: &mut egui::Ui,
        address: BeatAddress,
        fraction: f32,
        follow: bool,
    ) -> egui::Response {
        let response = self.show(ui);
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

    /// Scale geometry, glyph outlines and hit regions together.
    pub fn scaled(&self, zoom: f32) -> Result<Self, RenderError> {
        if !zoom.is_finite() || !(0.1..=8.0).contains(&zoom) {
            return Err(RenderError::invalid_input(
                "zoom must be between 0.1 and 8".into(),
            ));
        }
        let mut result = self.clone();
        result.width *= zoom;
        result.height *= zoom;
        for p in &mut result.primitives {
            match p {
                Primitive::Line {
                    from, to, width, ..
                } => {
                    for v in from.iter_mut().chain(to.iter_mut()) {
                        *v *= zoom;
                    }
                    *width *= zoom;
                }
                Primitive::Curve { points, width, .. } => {
                    for value in points.iter_mut().flatten() {
                        *value *= zoom;
                    }
                    *width *= zoom;
                }
                Primitive::Text { at, size, .. } => {
                    at[0] *= zoom;
                    at[1] *= zoom;
                    *size *= zoom;
                }
                Primitive::Glyph { at, space, .. } => {
                    at[0] *= zoom;
                    at[1] *= zoom;
                    *space *= zoom;
                }
            }
        }
        for b in &mut result.beats {
            for v in b.rect.iter_mut().chain(b.cursor_rect.iter_mut()) {
                *v *= zoom;
            }
        }
        for s in &mut result.systems {
            s[0] *= zoom;
            s[1] *= zoom;
        }
        Ok(result)
    }

    /// Divide complete systems among pages. Oversized systems are explicit errors;
    /// lower zoom or increase page height instead of clipping notation.
    pub fn paginate(&self, page_height: f32) -> Result<Vec<Self>, RenderError> {
        if !page_height.is_finite() || page_height < 100.0 {
            return Err(RenderError::invalid_input(
                "page height must be at least 100".into(),
            ));
        }
        if self.systems.is_empty() {
            return Ok(vec![self.clone()]);
        }
        let mut ranges: Vec<[f32; 2]> = vec![];
        for s in &self.systems {
            if s[1] - s[0] > page_height {
                return Err(RenderError::invalid_input(
                    "a system exceeds the page height".into(),
                ));
            }
            if let Some(range) = ranges.last_mut().filter(|r| s[1] - r[0] <= page_height) {
                range[1] = s[1];
            } else {
                ranges.push(*s);
            }
        }
        let mut pages = vec![];
        for range in ranges {
            let mut page = Self {
                width: self.width,
                height: page_height,
                primitives: vec![],
                beats: vec![],
                systems: vec![],
                style: self.style,
            };
            for p in &self.primitives {
                let y = match p {
                    Primitive::Line { from, to, .. } => (from[1] + to[1]) / 2.0,
                    Primitive::Curve { points, .. } => (points[0][1] + points[3][1]) / 2.0,
                    Primitive::Text { at, .. } | Primitive::Glyph { at, .. } => at[1],
                };
                if y >= range[0] && y < range[1] {
                    let mut p = p.clone();
                    match &mut p {
                        Primitive::Line { from, to, .. } => {
                            from[1] -= range[0];
                            to[1] -= range[0];
                        }
                        Primitive::Curve { points, .. } => {
                            for point in points {
                                point[1] -= range[0];
                            }
                        }
                        Primitive::Text { at, .. } | Primitive::Glyph { at, .. } => {
                            at[1] -= range[0]
                        }
                    };
                    page.primitives.push(p);
                }
            }
            for b in &self.beats {
                if b.cursor_rect[1] >= range[0] && b.cursor_rect[1] < range[1] {
                    let mut b = b.clone();
                    for rect in [&mut b.rect, &mut b.cursor_rect] {
                        rect[1] -= range[0];
                        rect[3] -= range[0];
                    }
                    page.beats.push(b);
                }
            }
            page.systems = self
                .systems
                .iter()
                .filter(|s| s[0] >= range[0] && s[1] <= range[1])
                .map(|s| [s[0] - range[0], s[1] - range[0]])
                .collect();
            pages.push(page);
        }
        Ok(pages)
    }

    /// Paginate complete systems, reducing the complete layout only when one
    /// system would otherwise exceed the requested page height.
    ///
    /// The returned pages retain the layout's beat addresses. This is useful for
    /// print/export callers that prefer a readable reduced score over a hard
    /// pagination error.
    pub fn paginate_to_fit(&self, page_height: f32) -> Result<Vec<Self>, RenderError> {
        if !page_height.is_finite() || page_height < 100.0 {
            return Err(RenderError::invalid_input(
                "page height must be at least 100".into(),
            ));
        }
        let tallest = self
            .systems
            .iter()
            .map(|s| s[1] - s[0])
            .fold(0.0_f32, f32::max);
        if tallest <= page_height || tallest == 0.0 {
            return self.paginate(page_height);
        }
        self.scaled(page_height / tallest)
            .and_then(|layout| layout.paginate(page_height))
    }
}

impl Scene {
    /// Paginate with a repeated running title, copyright footer and page numbers.
    /// Header/footer space is reserved before assigning complete systems to pages.
    pub fn paginate_with_headers(
        &self,
        page_height: f32,
        title: &str,
        copyright: &str,
    ) -> Result<Vec<Self>, RenderError> {
        const HEADER: f32 = 36.0;
        const FOOTER: f32 = 28.0;
        let mut pages = self.paginate(page_height - HEADER - FOOTER)?;
        let count = pages.len();
        for (i, page) in pages.iter_mut().enumerate() {
            page.translate_y(HEADER);
            page.height = page_height;
            page.text(page.width / 2.0, 16.0, title, 12.0, false);
            page.text(page.width / 2.0, page_height - 18.0, copyright, 9.0, false);
            page.text(
                page.width - 32.0,
                page_height - 18.0,
                format!("{} / {count}", i + 1),
                10.0,
                false,
            );
        }
        Ok(pages)
    }

    pub(crate) fn translate_y(&mut self, offset: f32) {
        for p in &mut self.primitives {
            match p {
                Primitive::Line { from, to, .. } => {
                    from[1] += offset;
                    to[1] += offset;
                }
                Primitive::Curve { points, .. } => {
                    for point in points {
                        point[1] += offset;
                    }
                }
                Primitive::Text { at, .. } | Primitive::Glyph { at, .. } => at[1] += offset,
            }
        }
        for b in &mut self.beats {
            for r in [&mut b.rect, &mut b.cursor_rect] {
                r[1] += offset;
                r[3] += offset;
            }
        }
        for s in &mut self.systems {
            s[0] += offset;
            s[1] += offset;
        }
    }
}
