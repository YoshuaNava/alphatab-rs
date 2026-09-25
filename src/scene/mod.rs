//! Backend-neutral scene options, geometry, and construction support.
mod annotations;
pub(crate) mod build;
mod layout;
mod measure;
mod numbered;
mod parameters;
pub(crate) mod planning;
mod primitive;
mod rhythm;
mod staff;
mod voices;

pub use primitive::Primitive;

pub(crate) use build::select_flag_glyph;
pub(crate) use numbered::compute_voice_offset;
pub(crate) use staff::compute_pitch_y;

pub use build::{engrave, validate_scene};


use crate::{DisplayMode, LayoutMode, RenderError};

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
}

/// Global visual treatment for a completed layout.
/// Per-track treatments are available through [`crate::ScoreTrack`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RenderStyle {
    /// Default color for notation that has no class-specific override.
    pub foreground: Color,
    /// Page background color.
    pub background: Color,
    /// Color used to highlight selected beats.
    pub selection: Color,
    /// Color used for playback cursors.
    pub cursor: Color,
    /// Optional overrides for individual primitive classes.
    pub music_glyphs: Option<Color>,
    /// The staff and effect lines value.
    pub staff_and_effect_lines: Option<Color>,
    /// The text value.
    pub text: Option<Color>,
}

/// Fine-grained visibility settings for rendered notation elements.
///
/// These settings supplement the category switches on [`SceneOptions`].
/// A category switch still hides every element in that category.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ElementVisibility {
    /// Shows the score title.
    pub title: bool,
    /// Shows the score subtitle.
    pub subtitle: bool,
    /// Shows the artist credit.
    pub artist: bool,
    /// Shows the album credit.
    pub album: bool,
    /// Shows the lyricist credit.
    pub words: bool,
    /// Shows the composer credit.
    pub music: bool,
    /// Shows the copyright notice.
    pub copyright: bool,
    /// Shows score instructions.
    pub instructions: bool,
    /// Shows tuning labels.
    pub tuning: bool,
    /// Shows the capo indication.
    pub capo: bool,
    /// Shows track names.
    pub track_names: bool,
    /// Shows chord diagrams.
    pub chord_diagrams: bool,
    /// Shows dynamic markings.
    pub dynamics: bool,
    /// Shows lyric text.
    pub lyrics: bool,
    /// Shows performance effects.
    pub effects: bool,
    /// Shows displayed measure numbers.
    pub bar_numbers: bool,
    /// Shows repeat pass counts.
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
    /// Convention used when rendering fingering labels.
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
    pub(crate) fn glyph_color(self) -> Color {
        self.music_glyphs.unwrap_or(self.foreground)
    }
    pub(crate) fn line_color(self) -> Color {
        self.staff_and_effect_lines.unwrap_or(self.foreground)
    }
    pub(crate) fn text_color(self) -> Color {
        self.text.unwrap_or(self.foreground)
    }
}

#[derive(Clone, Copy, Debug)]
/// Controls page geometry, notation mode, visibility, and visual style.
pub struct SceneOptions {
    /// Preferred page width; a dense measure may expand it to avoid collisions.
    pub width: f32,
    /// Vertical distance between tablature strings.
    pub string_spacing: f32,
    /// Minimum horizontal allocation for a rhythmic beat.
    pub beat_spacing: f32,
    /// Notation staff or staffs to render.
    pub display: DisplayMode,
    /// Page wrapping or horizontal scrolling behavior.
    pub flow: LayoutMode,
    /// Stretches systems to the available width.
    pub justify: bool,
    /// Rejects rather than expands notation exceeding `width`.
    pub strict_width: bool,
    /// Rhythm-stem style for tablature.
    pub tab_rhythm: TabRhythm,
    /// Master switch for score metadata.
    pub show_metadata: bool,
    /// Master switch for chord diagrams.
    pub show_chords: bool,
    /// Master switch for dynamics.
    pub show_dynamics: bool,
    /// Master switch for lyrics.
    pub show_lyrics: bool,
    /// Master switch for performance effects.
    pub show_effects: bool,
    /// Master switch for bar numbers.
    pub show_bar_numbers: bool,
    /// Master switch for tuning and capo labels.
    pub show_tuning: bool,
    /// Optional maximum number of measures in one system.
    pub bars_per_system: Option<usize>,
    /// Collapse consecutive full-measure rests, preserving all beat addresses.
    pub multi_measure_rests: bool,
    /// Colours stored in the returned layout and shared by egui and SVG output.
    pub style: RenderStyle,
    /// Per-element visibility overrides.
    pub elements: ElementVisibility,
    /// Engraving choices that affect notation and geometry.
    pub engraving: EngravingSettings,
}
impl Default for SceneOptions {
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

impl SceneOptions {
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
    /// The hidden option.
    Hidden,
    /// The individual option.
    Individual,
    #[default]
    /// Connect eligible tablature stems with beams.
    Connected,
    /// The automatic option.
    Automatic,
}

#[derive(Clone, Debug)]
/// Musical identity, timing, and interaction geometry for one beat.
pub struct BeatBounds {
    /// The start value.
    pub start: f64,
    /// The duration value.
    pub duration: f64,
    /// The measure value.
    pub measure: usize,
    /// The voice value.
    pub voice: usize,
    /// The beat value.
    pub beat: usize,
    /// The rect value.
    pub rect: [f32; 4],
    /// The staff-sized rectangle used when showing the playback cursor.
    pub cursor_rect: [f32; 4],
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
/// Zero-based measure, voice, and beat indices within one track.
pub struct BeatAddress {
    /// The measure value.
    pub measure: usize,
    /// The voice value.
    pub voice: usize,
    /// The beat value.
    pub beat: usize,
}

#[derive(Clone, Debug)]
/// Completed backend-neutral page geometry and interaction metadata.
pub struct Scene {
    /// The width value.
    pub width: f32,
    /// The height value.
    pub height: f32,
    /// The primitives value.
    pub primitives: Vec<Primitive>,
    /// The beats value.
    pub beats: Vec<BeatBounds>,
    /// Vertical extents of complete systems, used for pagination.
    pub systems: Vec<[f32; 2]>,
    /// The style value.
    pub style: RenderStyle,
}

/// Caller-owned cache for synchronous layout work.
///
/// Supply an application revision that changes whenever its score model or any
/// layout input changes. The cache deliberately does not hash the model: callers
/// often already have a document revision and hashing every note would itself be
/// expensive on large scores.
#[derive(Clone, Debug, Default)]
pub struct SceneCache {
    revision: Option<u64>,
    scene: Option<Scene>,
}
impl SceneCache {
    /// Discards the cached revision and layout.
    pub fn clear(&mut self) {
        self.revision = None;
        self.scene = None;
    }
    /// Reuses the cached layout for `revision`, or invokes `build` once.
    pub fn scene_or_try_build<F>(&mut self, revision: u64, build: F) -> Result<&Scene, RenderError>
    where
        F: FnOnce() -> Result<Scene, RenderError>,
    {
        if self.revision != Some(revision) {
            self.scene = Some(build()?);
            self.revision = Some(revision);
        }
        self.scene
            .as_ref()
            .ok_or_else(|| RenderError::internal("layout cache lost its completed value"))
    }
}
impl Scene {
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

    /// Returns the first beat whose interaction rectangle contains the point.
    pub fn hit_test(&self, x: f32, y: f32) -> Option<&BeatBounds> {
        self.beats
            .iter()
            .find(|b| x >= b.rect[0] && x < b.rect[2] && y >= b.rect[1] && y < b.rect[3])
    }

    /// Scales geometry, glyph outlines and hit regions together.
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
