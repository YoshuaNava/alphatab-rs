//! Native egui tablature used by `music_gym`. Build a [`Track`], call
//! [`engrave`], then paint it through [`EguiInteraction`]. String 1 is the top string.
#![deny(missing_docs)]

mod elements;
mod glyph;
pub mod guitar_pro;
mod music_font;
mod notation;
mod percussion;
mod render;
mod rests;
mod scene;
mod spans;
mod text;
mod validation;

pub use notation::*;
pub use render::{EguiInteraction, Interaction, Selection};
pub use scene::engrave;
pub use scene::*;
pub(crate) use smufl::Glyph as MusicGlyph;

const COMMON_TIME_NUMERATOR: u8 = 4;
const QUARTER_NOTE_DENOMINATOR: i16 = 4;
const EIGHTH_NOTE_DENOMINATOR: i16 = 8;
const LONGA_DURATION_VALUE: i16 = -4;
const BREVE_DURATION_VALUE: i16 = -2;
const MAXIMUM_DURATION_DENOMINATOR: i16 = 256;
const MAXIMUM_AUGMENTATION_DOTS: u8 = 3;
const ZERO_DURATION_COMPONENT: u8 = 0;
const QUARTER_BEATS_PER_WHOLE_NOTE: f64 = 4.0;
const AUGMENTATION_DOT_BASE: f64 = 2.0;

#[derive(Clone, Debug, Default)]
/// A single musical part, including its notation, tuning, metadata, and spans.
pub struct Track {
    /// Display name printed when track names are enabled.
    pub name: String,
    /// Top to bottom string labels, e.g. E B G D A E.
    pub strings: Vec<String>,
    /// Measures in written order.
    pub measures: Vec<Measure>,
    /// Initial clef; individual measures can override it.
    pub clef: Clef,
    /// Score-level descriptive text associated with this part.
    pub metadata: ScoreMetadata,
    /// Explicit connections and effect ranges between beats.
    pub spans: Vec<Span>,
    /// Guitar capo fret; zero means no capo.
    pub capo: u16,
}

impl Track {
    /// Creates an empty named track with default notation settings.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            ..Self::default()
        }
    }
}

#[derive(Clone, Debug)]
/// One written bar containing parallel voices and bar-level notation.
pub struct Measure {
    /// Numerator and denominator, such as `(4, 4)`.
    pub time_signature: (u8, u16),
    /// Voices start together; beats within each voice are sequential.
    pub voices: Vec<Vec<Beat>>,
    /// Whether this bar opens a repeated section.
    pub repeat_start: bool,
    /// Whether this bar closes a repeated section.
    pub repeat_end: bool,
    /// Total passes through this repeat section, when explicitly specified.
    pub repeat_count: Option<u8>,
    /// Conventional key signature.
    pub key_signature: KeySignature,
    /// Optional tempo change in quarter notes per minute.
    pub tempo: Option<u16>,
    /// Rehearsal mark or section label.
    pub marker: String,
    /// One-based repeat passes on which this ending is played.
    pub alternate_endings: Vec<u8>,
    /// Rendering hint: forces this measure to begin a new rendered system.
    /// It does not change the musical timeline.
    pub break_before: bool,
    /// Navigation symbol or textual direction placed at this bar.
    pub navigation: Option<Navigation>,
    /// Fermatas positioned in quarter-note units from the bar start.
    pub fermatas: Vec<Fermata>,
    /// Clef change at this bar, or `None` to retain the current clef.
    pub clef: Option<Clef>,
    /// Group sizes in denominator units; must sum to the meter numerator.
    pub beam_groups: Vec<u8>,
    /// Denominator of beam-group units; None uses the measure denominator.
    pub beam_unit: Option<u16>,
    /// Source notation for an explicit multi-measure rest. Engraving may project
    /// this into condensed geometry while preserving the original addresses.
    pub rest_count: usize,
    /// Presentation value for the printed bar number; it is not a stable
    /// measure identity. Collection position remains the address used by the
    /// layout and interaction APIs.
    pub display_number: Option<usize>,
    /// Written simile mark, rendered in place of ordinary contents.
    pub simile: Option<Simile>,
    /// Presentation instruction to draw a double barline at the end.
    pub double_bar: bool,
    /// Marks the measure as having no fixed meter.
    pub free_time: bool,
    /// Human-readable swing or triplet-feel indication.
    pub triplet_feel: Option<String>,
}

impl Default for Measure {
    fn default() -> Self {
        Self {
            time_signature: (COMMON_TIME_NUMERATOR, QUARTER_NOTE_DENOMINATOR as u16),
            voices: vec![],
            repeat_start: false,
            repeat_end: false,
            repeat_count: None,
            key_signature: KeySignature::Natural,
            tempo: None,
            marker: String::new(),
            alternate_endings: vec![],
            break_before: false,
            navigation: None,
            fermatas: vec![],
            clef: None,
            beam_groups: vec![],
            beam_unit: None,
            rest_count: 0,
            display_number: None,
            simile: None,
            double_bar: false,
            free_time: false,
            triplet_feel: None,
        }
    }
}

impl Measure {
    /// Creates an empty measure in the supplied time signature.
    pub fn new(numerator: u8, denominator: u16) -> Self {
        Self {
            time_signature: (numerator, denominator),
            ..Self::default()
        }
    }
}

#[derive(Clone, Debug, Default)]
/// Simultaneous notes or a rest at one rhythmic position in a voice.
pub struct Beat {
    /// Optional onset in quarter-note units, relative to the measure.
    /// None places this beat immediately after its predecessor.
    pub start: Option<f64>,
    /// Written rhythmic duration.
    pub duration: Duration,
    /// An empty chord is a rest.
    pub notes: Vec<Note>,
    /// Text, dynamics, diagrams, and beat-wide performance markings.
    pub annotations: BeatAnnotations,
}

impl Beat {
    /// Returns this beat's duration in quarter notes, including nested tuplets.
    pub fn compute_quarter_beats(&self) -> Result<f64, RenderError> {
        let mut duration = self.duration.compute_quarter_beats()?;
        for &(a, b) in &self.annotations.tuplets {
            if a == 0 || b == 0 {
                return Err(RenderError::invalid_input(
                    "invalid nested tuplet ratio".into(),
                ));
            }
            duration *= f64::from(b) / f64::from(a);
        }
        Ok(duration)
    }

    /// Creates a rest of the supplied duration.
    pub fn rest(duration: Duration) -> Self {
        Self {
            duration,
            ..Self::default()
        }
    }

    /// Creates a beat containing simultaneous notes.
    pub fn with_notes(duration: Duration, notes: impl IntoIterator<Item = Note>) -> Self {
        Self {
            duration,
            notes: notes.into_iter().collect(),
            ..Self::default()
        }
    }

    /// Assigns an explicit onset in quarter-note units.
    pub fn at(mut self, onset: QuarterTime) -> Self {
        self.start = Some(onset.get());
        self
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
/// A tablature fret or special fret state.
pub enum Fret {
    /// A normally fretted or open note; zero is an open string.
    Number(u16),
    /// A muted or percussive dead note.
    Dead,
    /// Continuation of a previous note, retaining its displayed fret.
    Tied(u16),
}

#[derive(Clone, Debug, Default)]
/// One note inside a beat, with tablature, pitch, and effect information.
pub struct Note {
    /// One-based index, counted from the top string.
    pub string: usize,
    /// Fret displayed on the tablature staff.
    pub fret: Fret,
    /// Written pitch required by standard and numbered notation modes.
    pub pitch: Option<Pitch>,
    /// Note-specific articulations and guitar techniques.
    pub effects: NoteEffects,
    /// General MIDI percussion id for unpitched notation.
    pub percussion: Option<u16>,
}

#[derive(Clone, Copy, Debug)]
/// A written duration expressed as a denominator, dots, and optional tuplet.
pub struct Duration {
    /// -4 = longa, -2 = breve, 1 = whole, 2 = half, 4 = quarter, through 256.
    pub value: i16,
    /// Number of augmentation dots, from zero through three.
    pub dots: u8,
    /// (notes played, in the time of), e.g. (3, 2) for triplets.
    pub tuplet: Option<(u8, u8)>,
}

impl Duration {
    /// An undotted quarter-note duration.
    pub const QUARTER: Self = Self {
        value: QUARTER_NOTE_DENOMINATOR,
        dots: 0,
        tuplet: None,
    };

    /// Returns the number of rhythmic flag or beam levels required by this
    /// duration.
    ///
    /// An isolated short note displays these levels as flags; adjacent notes
    /// may connect the same levels into beams. An eighth note has one flag,
    /// a sixteenth note has two, and a thirty-second note has three.
    /// When consecutive short notes occur, those flags are usually joined into
    /// horizontal beams. Quarter notes and longer values return zero. This
    /// method reports rhythmic depth only; the engraving stage decides which
    /// neighboring notes can be beamed together.
    pub fn beam_level_count(self) -> u32 {
        if self.value >= EIGHTH_NOTE_DENOMINATOR {
            self.value.ilog2() - 2
        } else {
            0
        }
    }
    /// Returns the undotted duration measured in quarter notes.
    pub fn undotted_quarter_beats(self) -> f64 {
        if self.value < 0 {
            -QUARTER_BEATS_PER_WHOLE_NOTE * f64::from(self.value)
        } else {
            QUARTER_BEATS_PER_WHOLE_NOTE / f64::from(self.value)
        }
    }
    /// Returns the duration multiplier introduced by augmentation dots.
    pub fn augmentation_dot_factor(self) -> f64 {
        AUGMENTATION_DOT_BASE - AUGMENTATION_DOT_BASE.powi(-i32::from(self.dots))
    }

    /// Validates the duration and returns its length in quarter notes.
    ///
    /// The calculation proceeds in three stages:
    ///
    /// 1. The denominator is converted to an undotted base length: a quarter
    ///    note (`4`) is `1.0`, a half note (`2`) is `2.0`, and an eighth note
    ///    (`8`) is `0.5` quarter notes. Longa (`-4`) and breve (`-2`) use the
    ///    special negative values documented on [`Duration::value`].
    /// 2. Augmentation dots multiply that base by `1.5`, `1.75`, or `1.875`
    ///    for one, two, or three dots.
    /// 3. A tuplet `(played, in_time_of)` multiplies the result by
    ///    `in_time_of / played`; for example, `(3, 2)` turns three notes into
    ///    the time normally occupied by two.
    ///
    /// Invalid denominators, too many dots, or zero-valued tuplet components
    /// return [`RenderError`] instead of producing a non-musical duration.
    pub fn compute_quarter_beats(self) -> Result<f64, RenderError> {
        if (!matches!(self.value, LONGA_DURATION_VALUE | BREVE_DURATION_VALUE)
            && (self.value <= 0
                || !(self.value as u16).is_power_of_two()
                || self.value > MAXIMUM_DURATION_DENOMINATOR))
            || self.dots > MAXIMUM_AUGMENTATION_DOTS
            || self
                .tuplet
                .is_some_and(|(a, b)| a == ZERO_DURATION_COMPONENT || b == ZERO_DURATION_COMPONENT)
        {
            return Err(RenderError::invalid_input("invalid note duration".into()));
        }
        let dots = self.augmentation_dot_factor();
        let ratio = self
            .tuplet
            .map_or(1.0, |(a, b)| f64::from(b) / f64::from(a));
        Ok(self.undotted_quarter_beats() * dots * ratio)
    }
}

/// Stable category for programmatic error handling.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ErrorKind {
    /// The supplied notation model or layout request is invalid.
    InvalidInput,
    /// Parsed source data could not be converted safely.
    Import,
    /// Input exceeded a documented memory or complexity limit.
    ResourceLimit,
    /// A bundled asset or internal invariant failed.
    Internal,
}

/// An invalid model, import, layout, or export operation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RenderError {
    kind: ErrorKind,
    message: String,
}

impl RenderError {
    /// Creates an error with a stable category and human-readable message.
    pub fn new(kind: ErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
        }
    }

    /// Returns the category suitable for branching in application code.
    pub const fn kind(&self) -> ErrorKind {
        self.kind
    }

    /// Returns the human-readable error detail.
    pub fn message(&self) -> &str {
        &self.message
    }

    pub(crate) fn invalid_input(message: String) -> Self {
        Self::new(ErrorKind::InvalidInput, message)
    }

    pub(crate) fn internal(message: impl Into<String>) -> Self {
        Self::new(ErrorKind::Internal, message)
    }

    pub(crate) fn resource_limit(message: impl Into<String>) -> Self {
        Self::new(ErrorKind::ResourceLimit, message)
    }
}

impl std::fmt::Display for RenderError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)
    }
}
impl std::error::Error for RenderError {}

impl Default for Fret {
    fn default() -> Self {
        Self::Number(0)
    }
}
impl Default for Duration {
    fn default() -> Self {
        Self::QUARTER
    }
}
