//! Render options, output primitives, hit regions, and backend-neutral layouts.
use crate::{DisplayMode, LayoutMode, RenderError};
use std::fmt::Write;

mod primitive;

/// RGBA colour used by the renderer and SVG export.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Color {
    /// Red channel.
    pub r: u8,
    /// Green channel.
    pub g: u8,
    /// Blue channel.
    pub b: u8,
    /// Alpha channel, where zero is transparent.
    pub a: u8,
}
impl Color {
    /// Opaque black.
    pub const BLACK: Self = Self::opaque(0, 0, 0);
    /// Opaque white.
    pub const WHITE: Self = Self::opaque(255, 255, 255);
    /// Constructs an opaque RGB colour.
    pub const fn opaque(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b, a: 255 }
    }
    pub(crate) fn egui(self) -> egui::Color32 {
        egui::Color32::from_rgba_unmultiplied(self.r, self.g, self.b, self.a)
    }
    pub(crate) fn svg(self) -> String {
        if self.a == 255 {
            format!("rgb({} {} {})", self.r, self.g, self.b)
        } else {
            format!(
                "rgba({} {} {} / {:.3})",
                self.r,
                self.g,
                self.b,
                f32::from(self.a) / 255.0
            )
        }
    }
}

/// Global visual treatment for a completed layout.
/// Per-track treatments are available through [`crate::ScoreTrack`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RenderStyle {
    pub foreground: Color,
    pub background: Color,
    pub selection: Color,
    pub cursor: Color,
    /// Optional overrides for individual primitive classes.
    pub music_glyphs: Option<Color>,
    pub staff_and_effect_lines: Option<Color>,
    pub text: Option<Color>,
}

/// Fine-grained visibility settings for rendered notation elements.
///
/// These settings supplement the older category switches on [`LayoutOptions`].
/// A category switch still hides every element in that category.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ElementVisibility {
    pub title: bool,
    pub subtitle: bool,
    pub artist: bool,
    pub album: bool,
    pub words: bool,
    pub music: bool,
    pub copyright: bool,
    pub instructions: bool,
    pub tuning: bool,
    pub capo: bool,
    pub track_names: bool,
    pub chord_diagrams: bool,
    pub dynamics: bool,
    pub lyrics: bool,
    pub effects: bool,
    pub bar_numbers: bool,
    pub repeat_counts: bool,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
/// Selects the convention used for fingering labels.
pub enum FingeringMode {
    #[default]
    /// Guitar left- and right-hand notation.
    Guitar,
    /// Piano finger numbers.
    Piano,
}

/// Engraving choices that affect geometry or notation semantics.
#[derive(Clone, Copy, Debug)]
pub struct EngravingSettings {
    /// Minimum slur arch in layout units.
    pub slur_height: f32,
    /// Extra vertical distance inserted between systems.
    pub system_gap: f32,
    /// Written-pitch transposition applied before staff engraving.
    pub display_transposition: i8,
    pub fingering_mode: FingeringMode,
}
impl Default for EngravingSettings {
    fn default() -> Self {
        Self {
            slur_height: 12.0,
            system_gap: 15.0,
            display_transposition: 0,
            fingering_mode: FingeringMode::Guitar,
        }
    }
}
impl Default for ElementVisibility {
    fn default() -> Self {
        Self {
            title: true,
            subtitle: true,
            artist: true,
            album: true,
            words: true,
            music: true,
            copyright: true,
            instructions: true,
            tuning: true,
            capo: true,
            track_names: true,
            chord_diagrams: true,
            dynamics: true,
            lyrics: true,
            effects: true,
            bar_numbers: true,
            repeat_counts: true,
        }
    }
}
impl Default for RenderStyle {
    fn default() -> Self {
        Self {
            foreground: Color::BLACK,
            background: Color::WHITE,
            selection: Color {
                r: 90,
                g: 170,
                b: 255,
                a: 48,
            },
            cursor: Color::opaque(30, 105, 190),
            music_glyphs: None,
            staff_and_effect_lines: None,
            text: None,
        }
    }
}

impl RenderStyle {
    fn glyph_color(self) -> Color {
        self.music_glyphs.unwrap_or(self.foreground)
    }
    fn line_color(self) -> Color {
        self.staff_and_effect_lines.unwrap_or(self.foreground)
    }
    fn text_color(self) -> Color {
        self.text.unwrap_or(self.foreground)
    }
}

#[derive(Clone, Copy, Debug)]
/// Controls page geometry, notation mode, visibility, and visual style.
pub struct LayoutOptions {
    /// Preferred page width; a dense measure may expand it to avoid collisions.
    pub width: f32,
    pub string_spacing: f32,
    pub beat_spacing: f32,
    pub display: DisplayMode,
    pub flow: LayoutMode,
    pub justify: bool,
    pub strict_width: bool,
    pub tab_rhythm: TabRhythm,
    pub show_metadata: bool,
    pub show_chords: bool,
    pub show_dynamics: bool,
    pub show_lyrics: bool,
    pub show_effects: bool,
    pub show_bar_numbers: bool,
    pub show_tuning: bool,
    pub bars_per_system: Option<usize>,
    /// Collapse consecutive full-measure rests, preserving all beat addresses.
    pub multi_measure_rests: bool,
    /// Colours stored in the returned layout and shared by egui and SVG output.
    pub style: RenderStyle,
    pub elements: ElementVisibility,
    pub engraving: EngravingSettings,
}
impl Default for LayoutOptions {
    fn default() -> Self {
        Self {
            width: 900.0,
            string_spacing: 22.0,
            beat_spacing: 40.0,
            display: DisplayMode::Tablature,
            flow: LayoutMode::Vertical,
            justify: false,
            strict_width: false,
            tab_rhythm: TabRhythm::Connected,
            show_metadata: true,
            show_chords: true,
            show_dynamics: true,
            show_lyrics: true,
            show_effects: true,
            show_bar_numbers: true,
            show_tuning: false,
            bars_per_system: None,
            multi_measure_rests: false,
            style: RenderStyle::default(),
            elements: ElementVisibility::default(),
            engraving: EngravingSettings::default(),
        }
    }
}

impl LayoutOptions {
    /// Returns options configured for the requested notation mode.
    pub fn with_display(mut self, display: DisplayMode) -> Self {
        self.display = display;
        self
    }

    /// Returns options configured for the requested page width.
    pub fn with_width(mut self, width: f32) -> Self {
        self.width = width;
        self
    }

    /// Returns options with an optional maximum number of bars per system.
    pub fn with_bars_per_system(mut self, bars: Option<usize>) -> Self {
        self.bars_per_system = bars;
        self
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
/// Controls whether and how rhythm stems are shown below tablature.
pub enum TabRhythm {
    Hidden,
    Individual,
    #[default]
    Connected,
    Automatic,
}

#[derive(Clone, Debug)]
/// Backend-neutral drawing command produced by engraving.
pub enum Primitive {
    Glyph {
        at: [f32; 2],
        space: f32,
        code: char,
        outline: std::sync::Arc<crate::glyph::Glyph>,
        color: Option<Color>,
    },
    Line {
        from: [f32; 2],
        to: [f32; 2],
        width: f32,
        color: Option<Color>,
    },
    /// Cubic Bézier retained as vector geometry through every backend.
    Curve {
        points: [[f32; 2]; 4],
        width: f32,
        color: Option<Color>,
    },
    /// Centered text; masked text has a white rectangle behind it.
    Text {
        at: [f32; 2],
        text: String,
        size: f32,
        masked: bool,
        color: Option<Color>,
    },
}

#[derive(Clone, Debug)]
/// Musical identity, timing, and interaction geometry for one beat.
pub struct BeatBounds {
    pub start: f64,
    pub duration: f64,
    pub measure: usize,
    pub voice: usize,
    pub beat: usize,
    pub rect: [f32; 4],
    /// The staff-sized rectangle used when showing the playback cursor.
    pub cursor_rect: [f32; 4],
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
/// Zero-based measure, voice, and beat indices within one track.
pub struct BeatAddress {
    pub measure: usize,
    pub voice: usize,
    pub beat: usize,
}

#[derive(Clone, Debug)]
/// Completed backend-neutral page geometry and interaction metadata.
pub struct Layout {
    pub width: f32,
    pub height: f32,
    pub primitives: Vec<Primitive>,
    pub beats: Vec<BeatBounds>,
    /// Vertical extents of complete systems, used for pagination.
    pub systems: Vec<[f32; 2]>,
    pub style: RenderStyle,
}

/// Caller-owned cache for synchronous layout work.
///
/// Supply an application revision that changes whenever its score model or any
/// layout input changes. The cache deliberately does not hash the model: callers
/// often already have a document revision and hashing every note would itself be
/// expensive on large scores.
#[derive(Clone, Debug, Default)]
pub struct LayoutCache {
    revision: Option<u64>,
    layout: Option<Layout>,
}
impl LayoutCache {
    /// Discards the cached revision and layout.
    pub fn clear(&mut self) {
        self.revision = None;
        self.layout = None;
    }
    /// Reuses the cached layout for `revision`, or invokes `build` once.
    pub fn layout_or_try_build<F>(
        &mut self,
        revision: u64,
        build: F,
    ) -> Result<&Layout, RenderError>
    where
        F: FnOnce() -> Result<Layout, RenderError>,
    {
        if self.revision != Some(revision) {
            self.layout = Some(build()?);
            self.revision = Some(revision);
        }
        self.layout
            .as_ref()
            .ok_or_else(|| RenderError::internal("layout cache lost its completed value"))
    }
}
impl Layout {
    /// Places a glyph by its SMuFL origin.
    ///
    /// Most engraving code should use [`Self::glyph_at_center`]. This method is
    /// reserved for attachment geometry such as flags whose origin is meaningful.
    pub(crate) fn glyph_at_origin(
        &mut self,
        x: f32,
        y: f32,
        symbol: smufl::Glyph,
        space: f32,
    ) -> Result<(), RenderError> {
        let outline = crate::glyph::load(symbol)?;
        self.primitives.push(Primitive::Glyph {
            at: [x, y],
            space,
            code: outline.codepoint,
            outline,
            color: None,
        });
        Ok(())
    }

    /// Places the center of a glyph's visible outline at a notation anchor and
    /// returns the resulting SMuFL origin for stem/attachment calculations.
    pub(crate) fn glyph_at_center(
        &mut self,
        x: f32,
        y: f32,
        symbol: smufl::Glyph,
        space: f32,
    ) -> Result<[f32; 2], RenderError> {
        let glyph = crate::glyph::load(symbol)?;
        let origin = glyph.origin_at_center([x, y], space);
        self.primitives.push(Primitive::Glyph {
            at: origin,
            space,
            code: glyph.codepoint,
            outline: glyph,
            color: None,
        });
        Ok(origin)
    }

    /// Places a glyph with its visible right edge and vertical center at an anchor.
    pub(crate) fn glyph_at_right_center(
        &mut self,
        right: f32,
        y: f32,
        symbol: smufl::Glyph,
        space: f32,
    ) -> Result<[f32; 2], RenderError> {
        let glyph = crate::glyph::load(symbol)?;
        let origin = [
            right - glyph.bounds[2] * space,
            y - (glyph.bounds[1] + glyph.bounds[3]) * space / 2.0,
        ];
        self.primitives.push(Primitive::Glyph {
            at: origin,
            space,
            code: glyph.codepoint,
            outline: glyph,
            color: None,
        });
        Ok(origin)
    }

    /// Places a glyph with its visible left edge and vertical center at an anchor.
    pub(crate) fn glyph_at_left_center(
        &mut self,
        left: f32,
        y: f32,
        symbol: smufl::Glyph,
        space: f32,
    ) -> Result<[f32; 2], RenderError> {
        let glyph = crate::glyph::load(symbol)?;
        let origin = [
            left - glyph.bounds[0] * space,
            y - (glyph.bounds[1] + glyph.bounds[3]) * space / 2.0,
        ];
        self.primitives.push(Primitive::Glyph {
            at: origin,
            space,
            code: glyph.codepoint,
            outline: glyph,
            color: None,
        });
        Ok(origin)
    }

    pub(crate) fn curve(&mut self, from: [f32; 2], to: [f32; 2], arch: f32) {
        let delta = [to[0] - from[0], to[1] - from[1]];
        self.primitives.push(Primitive::Curve {
            points: [
                from,
                [
                    from[0] + delta[0] / 3.0,
                    from[1] + delta[1] / 3.0 + arch * 4.0 / 3.0,
                ],
                [
                    from[0] + delta[0] * 2.0 / 3.0,
                    from[1] + delta[1] * 2.0 / 3.0 + arch * 4.0 / 3.0,
                ],
                to,
            ],
            width: 1.2,
            color: None,
        });
    }

    pub(crate) fn line(&mut self, x: f32, y: f32, x2: f32, y2: f32, width: f32) {
        self.primitives.push(Primitive::Line {
            from: [x, y],
            to: [x2, y2],
            width,
            color: None,
        });
    }
    pub(crate) fn text(&mut self, x: f32, y: f32, text: impl ToString, size: f32, masked: bool) {
        self.primitives.push(Primitive::Text {
            at: [x, y],
            text: text.to_string(),
            size,
            masked,
            color: None,
        });
    }
    /// Override the color of one output element by primitive index.
    pub fn set_primitive_color(
        &mut self,
        index: usize,
        color: Option<Color>,
    ) -> Result<(), RenderError> {
        let primitive = self.primitives.get_mut(index).ok_or_else(|| {
            RenderError::invalid_input("primitive style index is out of range".into())
        })?;
        match primitive {
            Primitive::Glyph { color: value, .. }
            | Primitive::Line { color: value, .. }
            | Primitive::Curve { color: value, .. }
            | Primitive::Text { color: value, .. } => *value = color,
        }
        Ok(())
    }
    pub fn hit_test(&self, x: f32, y: f32) -> Option<&BeatBounds> {
        self.beats
            .iter()
            .find(|b| x >= b.rect[0] && x < b.rect[2] && y >= b.rect[1] && y < b.rect[3])
    }

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
