//! Explicit notation data, independent of file parsing.

/// A finite musical time measured in quarter notes.
#[derive(Clone, Copy, Debug, PartialEq, PartialOrd)]
pub struct QuarterTime(f64);

impl QuarterTime {
    /// Creates a nonnegative finite time value.
    pub fn new(value: f64) -> Result<Self, crate::RenderError> {
        if value.is_finite() && value >= 0.0 {
            Ok(Self(value))
        } else {
            Err(crate::RenderError::new(
                crate::ErrorKind::InvalidInput,
                "quarter time must be finite and nonnegative",
            ))
        }
    }

    /// Returns the value in quarter notes.
    pub const fn get(self) -> f64 {
        self.0
    }
}

/// A finite position along an effect curve, restricted to `0.0..=1.0`.
#[derive(Clone, Copy, Debug, PartialEq, PartialOrd)]
pub struct NormalizedPosition(f32);

impl NormalizedPosition {
    /// Creates a normalized position.
    pub fn new(value: f32) -> Result<Self, crate::RenderError> {
        if value.is_finite() && (0.0..=1.0).contains(&value) {
            Ok(Self(value))
        } else {
            Err(crate::RenderError::new(
                crate::ErrorKind::InvalidInput,
                "normalized position must be between zero and one",
            ))
        }
    }

    /// Returns the unitless position.
    pub const fn get(self) -> f32 {
        self.0
    }
}

/// A finite pitch displacement measured in semitones.
#[derive(Clone, Copy, Debug, PartialEq, PartialOrd)]
pub struct SemitoneOffset(f32);

impl SemitoneOffset {
    /// Creates a finite semitone displacement.
    pub fn new(value: f32) -> Result<Self, crate::RenderError> {
        if value.is_finite() {
            Ok(Self(value))
        } else {
            Err(crate::RenderError::new(
                crate::ErrorKind::InvalidInput,
                "semitone offset must be finite",
            ))
        }
    }

    /// Returns the displacement in semitones.
    pub const fn get(self) -> f32 {
        self.0
    }
}

/// A validated point on a bend or tremolo-bar curve.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct EffectPoint {
    /// Position from the beginning to the end of the effect.
    pub position: NormalizedPosition,
    /// Signed pitch displacement.
    pub offset: SemitoneOffset,
}

impl From<EffectPoint> for [f32; 2] {
    fn from(point: EffectPoint) -> Self {
        [point.position.get(), point.offset.get()]
    }
}

impl NoteEffects {
    /// Replaces the bend curve with validated, typed points.
    pub fn set_bend_points(&mut self, points: impl IntoIterator<Item = EffectPoint>) {
        self.bend = points.into_iter().map(Into::into).collect();
    }
}

impl BeatAnnotations {
    /// Replaces the tremolo-bar curve with validated, typed points.
    pub fn set_whammy_points(&mut self, points: impl IntoIterator<Item = EffectPoint>) {
        self.whammy = points.into_iter().map(Into::into).collect();
    }
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
/// Selects the notation staff or combination of staves to render.
pub enum DisplayMode {
    #[default]
    /// Render tablature only.
    Tablature,
    /// Render standard notation only.
    Standard,
    /// Render standard notation and tablature together.
    Both,
    /// Render numbered notation.
    Numbered,
    /// Render slash notation.
    Slash,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
/// Selects pagelike wrapping or a single horizontal strip.
pub enum LayoutMode {
    #[default]
    /// Wrap systems vertically like pages.
    Vertical,
    /// Keep systems in one horizontal strip.
    Horizontal,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
/// A written clef, including octave-transposing forms and percussion.
pub enum Clef {
    #[default]
    /// The treble (G) clef.
    Treble,
    /// The bass option.
    Bass,
    /// The alto option.
    Alto,
    /// The tenor option.
    Tenor,
    /// The percussion option.
    Percussion,
    /// The treble8above option.
    Treble8Above,
    /// The treble8below option.
    Treble8Below,
    /// The treble15above option.
    Treble15Above,
    /// The treble15below option.
    Treble15Below,
    /// The bass8above option.
    Bass8Above,
    /// The bass8below option.
    Bass8Below,
    /// The bass15above option.
    Bass15Above,
    /// The bass15below option.
    Bass15Below,
    /// The alto8above option.
    Alto8Above,
    /// The alto8below option.
    Alto8Below,
    /// The alto15above option.
    Alto15Above,
    /// The alto15below option.
    Alto15Below,
    /// The tenor8above option.
    Tenor8Above,
    /// The tenor8below option.
    Tenor8Below,
    /// The tenor15above option.
    Tenor15Above,
    /// The tenor15below option.
    Tenor15Below,
}

/// Written pitch: C=0 through B=6, octave uses scientific pitch numbering.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Pitch {
    /// Diatonic step, where C is zero and B is six.
    pub step: u8,
    /// Scientific-pitch octave number.
    pub octave: i8,
    /// Chromatic alteration in semitones.
    pub accidental: i8,
}
impl Pitch {
    /// Spells a MIDI pitch using either a flat- or sharp-biased chromatic map.
    pub fn from_midi(midi: u8, flats: bool) -> Self {
        let (step, accidental) = if flats {
            [
                (0, 0),
                (1, -1),
                (1, 0),
                (2, -1),
                (2, 0),
                (3, 0),
                (4, -1),
                (4, 0),
                (5, -1),
                (5, 0),
                (6, -1),
                (6, 0),
            ][usize::from(midi % 12)]
        } else {
            [
                (0, 0),
                (0, 1),
                (1, 0),
                (1, 1),
                (2, 0),
                (3, 0),
                (3, 1),
                (4, 0),
                (4, 1),
                (5, 0),
                (5, 1),
                (6, 0),
            ][usize::from(midi % 12)]
        };
        Self {
            step,
            accidental,
            octave: (midi / 12) as i8 - 1,
        }
    }
}
#[derive(Clone, Debug, Default)]
/// Articulations, ornaments, and guitar techniques attached to one note.
pub struct NoteEffects {
    /// Ghost-note articulation.
    pub ghost: bool,
    /// Hammer-on articulation.
    pub hammer_on: bool,
    /// Slide articulation.
    pub slide: bool,
    /// Optional slide entering the note.
    pub slide_in: Option<SlideDirection>,
    /// Optional slide leaving the note.
    pub slide_out: Option<SlideDirection>,
    /// Vibrato articulation.
    pub vibrato: bool,
    /// Palm-mute articulation.
    pub palm_mute: bool,
    /// Let-ring articulation.
    pub let_ring: bool,
    /// Staccato articulation.
    pub staccato: bool,
    /// Accent articulation.
    pub accent: bool,
    /// Heavy-accent articulation.
    pub heavy_accent: bool,
    /// Harmonic description, when present.
    pub harmonic: Option<String>,
    /// Bend points: normalized position 0..1 and displacement in semitones.
    pub bend: Vec<[f32; 2]>,
    /// The grace fret value.
    pub grace_fret: Option<u16>,
    /// The grace pitch value.
    pub grace_pitch: Option<Pitch>,
    /// The grace on beat value.
    pub grace_on_beat: bool,
    /// The grace slide value.
    pub grace_slide: bool,
    /// The grace dead value.
    pub grace_dead: bool,
    /// The grace bend value.
    pub grace_bend: bool,
    /// The grace slur value.
    pub grace_slur: bool,
    /// The grace duration value.
    pub grace_duration: u16,
    /// The head value.
    pub head: NoteHead,
    /// The right fingering value.
    pub right_fingering: Option<String>,
    /// The bend vibrato value.
    pub bend_vibrato: bool,
    /// The slide legato value.
    pub slide_legato: bool,
    /// The harmonic pitch value.
    pub harmonic_pitch: Option<Pitch>,
    /// Quarter-tone offset relative to the written pitch: -1, 0 or 1.
    pub quarter_tone: i8,
    /// The ornament value.
    pub ornament: Option<Ornament>,
    /// The trill fret value.
    pub trill_fret: Option<u16>,
    /// The tremolo slashes value.
    pub tremolo_slashes: u8,
    /// The fingering value.
    pub fingering: Option<String>,
}
#[derive(Clone, Debug, Default)]
/// Text, performance marks, and rhythmic overrides attached to one beat.
pub struct BeatAnnotations {
    /// Normalized beat positions and signed semitone offsets.
    pub whammy: Vec<[f32; 2]>,
    /// Strum direction; true is up.
    pub strum_up: Option<bool>,
    /// Plucking technique marking.
    pub technique: Option<PluckingTechnique>,
    /// Free-form text attached to the beat.
    pub text: String,
    /// Lyric syllables attached to the beat.
    pub lyrics: String,
    /// Dynamic marking text.
    pub dynamic: Option<String>,
    /// Chord diagram shown above the beat.
    pub chord: Option<ChordDiagram>,
    /// The pick up value.
    pub pick_up: Option<bool>,
    /// true crescendo, false diminuendo over the beat's column.
    pub crescendo: Option<bool>,
    /// Additional tuplet ratios, outermost first. The duration ratio is innermost.
    pub tuplets: Vec<(u8, u8)>,
    /// The beaming value.
    pub beaming: Beaming,
    /// The stem value.
    pub stem: StemDirection,
    /// The break secondary value.
    pub break_secondary: u8,
    /// The tuplet start value.
    pub tuplet_start: bool,
    /// The tuplet end value.
    pub tuplet_end: bool,
    /// The force tuplet bracket value.
    pub force_tuplet_bracket: bool,
    /// The rasgueado value.
    pub rasgueado: bool,
    /// The fade value.
    pub fade: Option<Fade>,
    /// The ottava value.
    pub ottava: Option<Ottava>,
    /// The pedal value.
    pub pedal: Option<Pedal>,
    /// The wah open value.
    pub wah_open: Option<bool>,
    /// The golpe value.
    pub golpe: bool,
    /// The left hand tap value.
    pub left_hand_tap: bool,
    /// The barre value.
    pub barre: Option<String>,
    /// The timer seconds value.
    pub timer_seconds: Option<u32>,
    /// The tempo value.
    pub tempo: Option<u16>,
}
#[derive(Clone, Debug, Default)]
/// A fretboard diagram displayed above a beat.
pub struct ChordDiagram {
    /// The name value.
    pub name: String,
    /// One entry per string, top string first: None muted; Some(0) open.
    pub frets: Vec<Option<u16>>,
    /// The first fret value.
    pub first_fret: u16,
    /// The barres value.
    pub barres: Vec<Barre>,
    /// Optional one label per string, in the same order as frets.
    pub fingers: Vec<String>,
    /// Minimum number of frets. Wide voicings expand the diagram automatically.
    pub fret_count: u16,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
/// Direction of a slide entering or leaving a note.
pub enum SlideDirection {
    /// The up option.
    Up,
    /// The down option.
    Down,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
/// Right-hand bass or guitar plucking technique.
pub enum PluckingTechnique {
    /// The tap option.
    Tap,
    /// The slap option.
    Slap,
    /// The pop option.
    Pop,
}
#[derive(Clone, Copy, Debug)]
/// A barre drawn across an inclusive range of strings in a chord diagram.
pub struct Barre {
    /// The fret value.
    pub fret: u16,
    /// Inclusive one-based string indices, counted from the top string.
    pub first_string: usize,
    /// The last string value.
    pub last_string: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
/// A conventional melodic ornament.
pub enum Ornament {
    /// The trill option.
    Trill,
    /// The turn option.
    Turn,
    /// The inverted turn option.
    InvertedTurn,
    /// The upper mordent option.
    UpperMordent,
    /// The lower mordent option.
    LowerMordent,
}
impl Ornament {
    pub(crate) fn resolve_glyph(self) -> smufl::Glyph {
        use smufl::Glyph::*;
        match self {
            Self::Trill => OrnamentTrill,
            Self::Turn => OrnamentTurn,
            Self::InvertedTurn => OrnamentTurnInverted,
            Self::UpperMordent => OrnamentShortTrill,
            Self::LowerMordent => OrnamentMordent,
        }
    }
}
#[derive(Clone, Debug)]
/// A fermata positioned within a measure.
pub struct Fermata {
    /// The start value.
    pub start: f64,
    /// The kind value.
    pub kind: FermataKind,
}
#[derive(Clone, Copy, Debug)]
/// Relative duration represented by a fermata glyph.
pub enum FermataKind {
    /// The normal option.
    Normal,
    /// The short option.
    Short,
    /// The long option.
    Long,
}
#[derive(Clone, Debug)]
/// Repeat-navigation symbol or free-form navigation instruction.
pub enum Navigation {
    /// The coda option.
    Coda,
    /// The double coda option.
    DoubleCoda,
    /// The segno option.
    Segno,
    /// The double segno option.
    DoubleSegno,
    /// The instruction option.
    Instruction(String),
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
/// Visual shape used for a written notehead.
pub enum NoteHead {
    #[default]
    /// The conventional oval notehead.
    Normal,
    /// The diamond option.
    Diamond,
    /// The cross option.
    Cross,
    /// The slash option.
    Slash,
    /// The triangle option.
    Triangle,
    /// The circle cross option.
    CircleCross,
}
impl NoteHead {
    pub(crate) fn resolve_glyph(self, value: i16) -> smufl::Glyph {
        use smufl::Glyph::*;
        if value == -2 || value == -4 {
            return match self {
                Self::Normal => {
                    if value == -4 {
                        NoteheadDoubleWholeSquare
                    } else {
                        NoteheadDoubleWhole
                    }
                }
                Self::Diamond => NoteheadDiamondDoubleWhole,
                Self::Cross => NoteheadXDoubleWhole,
                Self::Slash => NoteheadSlashWhiteDoubleWhole,
                Self::Triangle => NoteheadTriangleUpDoubleWhole,
                Self::CircleCross => NoteheadCircleXDoubleWhole,
            };
        }
        let index = match value {
            1 => 0,
            2 => 1,
            _ => 2,
        };
        match self {
            Self::Normal => [NoteheadWhole, NoteheadHalf, NoteheadBlack][index],
            Self::Diamond => [
                NoteheadDiamondWhole,
                NoteheadDiamondHalf,
                NoteheadDiamondBlack,
            ][index],
            Self::Cross => [NoteheadXWhole, NoteheadXHalf, NoteheadXBlack][index],
            Self::Slash => [
                NoteheadSlashWhiteWhole,
                NoteheadSlashWhiteHalf,
                NoteheadSlashVerticalEnds,
            ][index],
            Self::Triangle => [
                NoteheadTriangleUpWhole,
                NoteheadTriangleUpHalf,
                NoteheadTriangleUpBlack,
            ][index],
            Self::CircleCross => {
                [NoteheadCircleXWhole, NoteheadCircleXHalf, NoteheadCircleX][index]
            }
        }
    }
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
/// Explicit stem direction or automatic voice-based placement.
pub enum StemDirection {
    #[default]
    /// Select the direction automatically from the voice and pitch.
    Auto,
    /// The up option.
    Up,
    /// The down option.
    Down,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
/// Override for automatic beam grouping at a beat.
pub enum Beaming {
    #[default]
    /// Select beam connections automatically.
    Auto,
    /// The break option.
    Break,
    /// The join option.
    Join,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
/// Volume-envelope marking attached to a beat.
pub enum Fade {
    /// The in option.
    In,
    /// The out option.
    Out,
    /// The swell option.
    Swell,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
/// Piano pedal transition.
pub enum Pedal {
    /// The down option.
    Down,
    /// The up option.
    Up,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
/// One- or two-octave transposition line above or below the staff.
pub enum Ottava {
    /// The above8 option.
    Above8,
    /// The below8 option.
    Below8,
    /// The above15 option.
    Above15,
    /// The below15 option.
    Below15,
}
impl Ottava {
    pub(crate) fn format_label(self) -> &'static str {
        match self {
            Self::Above8 => "8va",
            Self::Below8 => "8vb",
            Self::Above15 => "15ma",
            Self::Below15 => "15mb",
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
/// One- or two-bar measure-repeat symbol.
pub enum Simile {
    /// The single option.
    Single,
    /// The double first option.
    DoubleFirst,
    /// The double second option.
    DoubleSecond,
}

/// A connection with explicit beat endpoints, including across measures/systems.
#[derive(Clone, Debug)]
pub struct Span {
    /// The start value.
    pub start: crate::BeatAddress,
    /// The end value.
    pub end: crate::BeatAddress,
    /// The kind value.
    pub kind: SpanKind,
    /// The placement value.
    pub placement: Placement,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
/// Preferred side of the staff for a span.
pub enum Placement {
    #[default]
    /// Place the span above the staff.
    Above,
    /// The below option.
    Below,
}
#[derive(Clone, Debug)]
/// Shape or continuous musical effect represented by a [`Span`].
pub enum SpanKind {
    /// Note indices in the endpoint chords. None attaches outside the chord.
    Slur {
        /// Optional note index in the starting chord.
        start_note: Option<usize>,
        /// Optional note index in the ending chord.
        end_note: Option<usize>,
    },
    /// The legato option.
    Legato {
        /// Optional note index in the starting chord.
        start_note: Option<usize>,
        /// Optional note index in the ending chord.
        end_note: Option<usize>,
        /// Text shown alongside the legato span.
        text: String,
    },
    /// The crescendo option.
    Crescendo,
    /// The diminuendo option.
    Diminuendo,
    /// The ottava option.
    Ottava(Ottava),
    /// The palm mute option.
    PalmMute,
    /// The let ring option.
    LetRing,
    /// The pedal option.
    Pedal,
    /// The trill option.
    Trill,
    /// The vibrato option.
    Vibrato,
    /// The rasgueado option.
    Rasgueado,
    /// Caller-defined effect band (for example a beat barre or pick slide).
    Text(String),
    /// Explicit beam group; can cross barlines, and breaks at system boundaries.
    Beam,
}

#[derive(Clone, Debug, Default)]
/// Descriptive score text rendered above the first system.
pub struct ScoreMetadata {
    /// The title value.
    pub title: String,
    /// The subtitle value.
    pub subtitle: String,
    /// The artist value.
    pub artist: String,
    /// The album value.
    pub album: String,
    /// The words value.
    pub words: String,
    /// The music value.
    pub music: String,
    /// The copyright value.
    pub copyright: String,
    /// The instructions value.
    pub instructions: String,
}
impl ChordDiagram {
    pub(crate) fn compute_rows(&self) -> u16 {
        self.frets
            .iter()
            .flatten()
            .copied()
            .chain(self.barres.iter().map(|b| b.fret))
            .max()
            .unwrap_or(0)
            .saturating_sub(self.first_fret)
            .saturating_add(1)
            .max(self.fret_count)
            .max(5)
    }
}

impl Pitch {
    /// Spell MIDI pitches in the given key, including B-sharp/C-flat in remote keys.
    pub fn spell_in_key(midi: u8, key: i8) -> Self {
        let order = if key >= 0 {
            [3, 0, 4, 1, 5, 2, 6]
        } else {
            [6, 2, 5, 1, 4, 0, 3]
        };
        let mut best = Self::from_midi(midi, key < 0);
        let mut cost = i16::MAX;
        for octave in -1..=9 {
            for (step, natural) in [0, 2, 4, 5, 7, 9, 11].into_iter().enumerate() {
                let accidental = i16::from(midi) - ((octave + 1) * 12 + natural);
                if !(-2..=2).contains(&accidental) {
                    continue;
                }
                let expected = if order[..usize::from(key.unsigned_abs().min(7))].contains(&step) {
                    i16::from(key.signum())
                } else {
                    0
                };
                let c = (accidental - expected).abs() * 3
                    + accidental.abs()
                    + i16::from(accidental.signum() != i16::from(key.signum()) && accidental != 0);
                if c < cost {
                    cost = c;
                    best = Self {
                        step: step as u8,
                        octave: octave as i8,
                        accidental: accidental as i8,
                    };
                }
            }
        }
        best
    }
}

impl DisplayMode {
    pub(crate) fn renders_staff(self) -> bool {
        matches!(self, Self::Standard | Self::Both | Self::Slash)
    }
    pub(crate) fn renders_tab(self) -> bool {
        matches!(self, Self::Tablature | Self::Both)
    }
}

impl Pitch {
    /// Converts this written spelling to a MIDI note number.
    pub fn to_midi(self) -> Result<u8, crate::RenderError> {
        let step = [0_i16, 2, 4, 5, 7, 9, 11]
            .get(usize::from(self.step))
            .ok_or_else(|| crate::RenderError::invalid_input("invalid pitch step".into()))?;
        let midi = (i16::from(self.octave) + 1) * 12 + step + i16::from(self.accidental);
        u8::try_from(midi)
            .ok()
            .filter(|v| *v <= 127)
            .ok_or_else(|| crate::RenderError::invalid_input("pitch outside MIDI range".into()))
    }
}
