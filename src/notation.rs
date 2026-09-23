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
    Tablature,
    Standard,
    Both,
    Numbered,
    Slash,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
/// Selects pagelike wrapping or a single horizontal strip.
pub enum LayoutMode {
    #[default]
    Vertical,
    Horizontal,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
/// A written clef, including octave-transposing forms and percussion.
pub enum Clef {
    #[default]
    Treble,
    Bass,
    Alto,
    Tenor,
    Percussion,
    Treble8Above,
    Treble8Below,
    Treble15Above,
    Treble15Below,
    Bass8Above,
    Bass8Below,
    Bass15Above,
    Bass15Below,
    Alto8Above,
    Alto8Below,
    Alto15Above,
    Alto15Below,
    Tenor8Above,
    Tenor8Below,
    Tenor15Above,
    Tenor15Below,
}

/// Written pitch: C=0 through B=6, octave uses scientific pitch numbering.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Pitch {
    pub step: u8,
    pub octave: i8,
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
    pub ghost: bool,
    pub hammer_on: bool,
    pub slide: bool,
    pub slide_in: Option<SlideDirection>,
    pub slide_out: Option<SlideDirection>,
    pub vibrato: bool,
    pub palm_mute: bool,
    pub let_ring: bool,
    pub staccato: bool,
    pub accent: bool,
    pub heavy_accent: bool,
    pub harmonic: Option<String>,
    /// Bend points: normalized position 0..1 and displacement in semitones.
    pub bend: Vec<[f32; 2]>,
    pub grace_fret: Option<u16>,
    pub grace_pitch: Option<Pitch>,
    pub grace_on_beat: bool,
    pub grace_slide: bool,
    pub grace_dead: bool,
    pub grace_bend: bool,
    pub grace_slur: bool,
    pub grace_duration: u16,
    pub head: NoteHead,
    pub right_fingering: Option<String>,
    pub bend_vibrato: bool,
    pub slide_legato: bool,
    pub harmonic_pitch: Option<Pitch>,
    /// Quarter-tone offset relative to the written pitch: -1, 0 or 1.
    pub quarter_tone: i8,
    pub ornament: Option<Ornament>,
    pub trill_fret: Option<u16>,
    pub tremolo_slashes: u8,
    pub fingering: Option<String>,
}
#[derive(Clone, Debug, Default)]
/// Text, performance marks, and rhythmic overrides attached to one beat.
pub struct BeatAnnotations {
    /// Normalized beat positions and signed semitone offsets.
    pub whammy: Vec<[f32; 2]>,
    pub strum_up: Option<bool>,
    pub technique: Option<PluckingTechnique>,
    pub text: String,
    pub lyrics: String,
    pub dynamic: Option<String>,
    pub chord: Option<ChordDiagram>,
    pub pick_up: Option<bool>,
    /// true crescendo, false diminuendo over the beat's column.
    pub crescendo: Option<bool>,
    /// Additional tuplet ratios, outermost first. The duration ratio is innermost.
    pub tuplets: Vec<(u8, u8)>,
    pub beaming: Beaming,
    pub stem: StemDirection,
    pub break_secondary: u8,
    pub tuplet_start: bool,
    pub tuplet_end: bool,
    pub force_tuplet_bracket: bool,
    pub rasgueado: bool,
    pub fade: Option<Fade>,
    pub ottava: Option<Ottava>,
    pub pedal: Option<Pedal>,
    pub wah_open: Option<bool>,
    pub golpe: bool,
    pub left_hand_tap: bool,
    pub barre: Option<String>,
    pub timer_seconds: Option<u32>,
    pub tempo: Option<u16>,
}
#[derive(Clone, Debug, Default)]
/// A fretboard diagram displayed above a beat.
pub struct ChordDiagram {
    pub name: String,
    /// One entry per string, top string first: None muted; Some(0) open.
    pub frets: Vec<Option<u16>>,
    pub first_fret: u16,
    pub barres: Vec<Barre>,
    /// Optional one label per string, in the same order as frets.
    pub fingers: Vec<String>,
    /// Minimum number of frets. Wide voicings expand the diagram automatically.
    pub fret_count: u16,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
/// Direction of a slide entering or leaving a note.
pub enum SlideDirection {
    Up,
    Down,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
/// Right-hand bass or guitar plucking technique.
pub enum PluckingTechnique {
    Tap,
    Slap,
    Pop,
}
#[derive(Clone, Copy, Debug)]
/// A barre drawn across an inclusive range of strings in a chord diagram.
pub struct Barre {
    pub fret: u16,
    /// Inclusive one-based string indices, counted from the top string.
    pub first_string: usize,
    pub last_string: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
/// A conventional melodic ornament.
pub enum Ornament {
    Trill,
    Turn,
    InvertedTurn,
    UpperMordent,
    LowerMordent,
}
impl Ornament {
    pub(crate) fn glyph(self) -> smufl::Glyph {
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
    pub start: f64,
    pub kind: FermataKind,
}
#[derive(Clone, Copy, Debug)]
/// Relative duration represented by a fermata glyph.
pub enum FermataKind {
    Normal,
    Short,
    Long,
}
#[derive(Clone, Debug)]
/// Repeat-navigation symbol or free-form navigation instruction.
pub enum Navigation {
    Coda,
    DoubleCoda,
    Segno,
    DoubleSegno,
    Instruction(String),
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
/// Visual shape used for a written notehead.
pub enum NoteHead {
    #[default]
    Normal,
    Diamond,
    Cross,
    Slash,
    Triangle,
    CircleCross,
}
impl NoteHead {
    pub(crate) fn glyph(self, value: i16) -> smufl::Glyph {
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
    Auto,
    Up,
    Down,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
/// Override for automatic beam grouping at a beat.
pub enum Beaming {
    #[default]
    Auto,
    Break,
    Join,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
/// Volume-envelope marking attached to a beat.
pub enum Fade {
    In,
    Out,
    Swell,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
/// Piano pedal transition.
pub enum Pedal {
    Down,
    Up,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
/// One- or two-octave transposition line above or below the staff.
pub enum Ottava {
    Above8,
    Below8,
    Above15,
    Below15,
}
impl Ottava {
    pub(crate) fn label(self) -> &'static str {
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
    Single,
    DoubleFirst,
    DoubleSecond,
}

/// A connection with explicit beat endpoints, including across measures/systems.
#[derive(Clone, Debug)]
pub struct Span {
    pub start: crate::BeatAddress,
    pub end: crate::BeatAddress,
    pub kind: SpanKind,
    pub placement: Placement,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
/// Preferred side of the staff for a span.
pub enum Placement {
    #[default]
    Above,
    Below,
}
#[derive(Clone, Debug)]
/// Shape or continuous musical effect represented by a [`Span`].
pub enum SpanKind {
    /// Note indices in the endpoint chords. None attaches outside the chord.
    Slur {
        start_note: Option<usize>,
        end_note: Option<usize>,
    },
    Legato {
        start_note: Option<usize>,
        end_note: Option<usize>,
        text: String,
    },
    Crescendo,
    Diminuendo,
    Ottava(Ottava),
    PalmMute,
    LetRing,
    Pedal,
    Trill,
    Vibrato,
    Rasgueado,
    /// Caller-defined effect band (for example a beat barre or pick slide).
    Text(String),
    /// Explicit beam group; can cross barlines, and breaks at system boundaries.
    Beam,
}

#[derive(Clone, Debug, Default)]
/// Descriptive score text rendered above the first system.
pub struct ScoreMetadata {
    pub title: String,
    pub subtitle: String,
    pub artist: String,
    pub album: String,
    pub words: String,
    pub music: String,
    pub copyright: String,
    pub instructions: String,
}
impl ChordDiagram {
    pub(crate) fn rows(&self) -> u16 {
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
    pub fn in_key(midi: u8, key: i8) -> Self {
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
    pub(crate) fn staff(self) -> bool {
        matches!(self, Self::Standard | Self::Both | Self::Slash)
    }
    pub(crate) fn tab(self) -> bool {
        matches!(self, Self::Tablature | Self::Both)
    }
}

impl Pitch {
    /// Converts this written spelling to a MIDI note number.
    pub fn midi(self) -> Result<u8, crate::RenderError> {
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
