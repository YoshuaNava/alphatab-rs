//! Source-independent data structures for one tablature-oriented track.
//!
//! # Score hierarchy
//!
//! ```text
//! Track
//! ├── name and open-string tuning
//! └── Bar 0..n
//!     ├── time signature
//!     └── Voice 0..n                 (parallel rhythmic lines)
//!         └── Beat 0..n              (sequential rhythmic positions)
//!             └── Note 0..n          (simultaneous chord notes)
//!                 ├── string + fret  (tablature)
//!                 └── MIDI pitch      (staff view, when pitched)
//! ```
//!
//! A [`Track`] is one playable part, such as a guitar part. Its [`Bar`]s
//! appear in written order. A bar may have several [`Voice`]s: each voice
//! starts at the same bar origin and describes an independent rhythm. This
//! represents music such as a melody above sustained bass notes without forcing
//! both rhythms into one beat sequence.
//!
//! ```text
//! One 4/4 bar
//!
//! Voice 0:  | Beat(C, quarter) | Beat(D, quarter) | Beat(E, quarter) | Beat(F, quarter) |
//! Voice 1:  | Beat(C, half)                    | Beat(G, half)                    |
//!            0                 1                2                 3                4
//!                              quarter-note time →
//! ```
//!
//! Within one voice, beats are normally sequential: each next beat begins when
//! the preceding beat ends. [`Beat::start`] can explicitly place a beat at a
//! particular quarter-note offset instead. A beat's notes are simultaneous, so
//! a three-note guitar chord is one beat containing three [`Note`] values.
//!
//! ## Tab and staff information
//!
//! [`Note::string`] and [`Fret`] are sufficient to draw tablature. During Guitar
//! Pro import, [`Note::midi`] is derived from the open-string tuning, fret, capo,
//! and source transposition; it enables the staff view. Percussion notes leave
//! `midi` unset because they have no pitched staff placement.

use crate::RenderError;

/// A playable part represented as tablature-oriented bars.
///
/// See the module-level score hierarchy above for how tracks own bars,
/// voices, beats, and notes.
#[derive(Clone, Debug, Default)]
pub struct Track {
    /// Display name for the part.
    pub name: String,
    /// MIDI pitches, highest string first.
    pub strings: Vec<u8>,
    /// Bars in written order.
    pub bars: Vec<Bar>,
}

/// One written bar containing one or more concurrent voices.
///
/// See the module-level score hierarchy above for a timing example.
#[derive(Clone, Debug)]
pub struct Bar {
    /// Written meter as numerator and denominator, such as `(4, 4)`.
    pub time_signature: (u8, u16),
    /// Voices begin together. Beats in a voice are sequential unless `start` is set.
    pub voices: Vec<Voice>,
}
impl Default for Bar {
    fn default() -> Self {
        Self {
            time_signature: (4, 4),
            voices: vec![],
        }
    }
}

/// One independent rhythmic line in a bar.
///
/// Every voice starts at the bar origin. Its beats are sequential unless a
/// beat supplies an explicit onset.
#[derive(Clone, Debug, Default)]
pub struct Voice {
    /// Beats written sequentially in this voice.
    pub beats: Vec<Beat>,
}

/// A rhythmic instant containing a chord or a rest.
#[derive(Clone, Debug, Default)]
pub struct Beat {
    /// Onset in quarter notes relative to its bar.
    pub start: Option<f64>,
    /// Written duration.
    pub duration: Duration,
    /// Simultaneous notes; an empty list is a rest.
    pub notes: Vec<Note>,
}
impl Beat {
    /// Computes the performed duration in quarter notes.
    pub fn compute_quarter_beats(&self) -> Result<f64, RenderError> {
        self.duration.compute_quarter_beats()
    }
}

/// The tab-specific value shown on a string.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Fret {
    /// A fretted or open note.
    Number(u16),
    /// A muted or percussive note.
    Dead,
    /// A continuation retaining the prior fret label.
    Tied(u16),
}
impl Default for Fret {
    fn default() -> Self {
        Self::Number(0)
    }
}

/// One tab note and its optional staff pitch.
#[derive(Clone, Debug, Default)]
pub struct Note {
    /// One-based, from the top string.
    pub string: usize,
    /// The tablature value to display.
    pub fret: Fret,
    /// Derived during import so staff view never depends on optional spelling data.
    pub midi: Option<u8>,
}

/// A conventional duration denominator, dot count, and optional tuplet ratio.
#[derive(Clone, Copy, Debug)]
pub struct Duration {
    /// Whole-note denominator (`4` is a quarter); `-2` and `-4` are breve and longa.
    pub value: i16,
    /// Number of augmentation dots, from zero through three.
    pub dots: u8,
    /// `(played, in_time_of)`, such as `(3, 2)` for triplets.
    pub tuplet: Option<(u8, u8)>,
}
impl Default for Duration {
    fn default() -> Self {
        Self::QUARTER
    }
}
impl Duration {
    /// An undotted quarter note.
    pub const QUARTER: Self = Self {
        value: 4,
        dots: 0,
        tuplet: None,
    };
    /// Computes this written duration in quarter-note units.
    pub fn compute_quarter_beats(self) -> Result<f64, RenderError> {
        if self.dots > 3 || self.tuplet.is_some_and(|(a, b)| a == 0 || b == 0) {
            return Err(RenderError("invalid duration".into()));
        }
        let base = match self.value {
            -4 => 16.0,
            -2 => 8.0,
            n if n > 0 && (n as u16).is_power_of_two() && n <= 256 => 4.0 / f64::from(n),
            _ => return Err(RenderError("invalid duration".into())),
        };
        let dotted = base * (2.0 - 2.0_f64.powi(-i32::from(self.dots)));
        Ok(dotted
            * self
                .tuplet
                .map_or(1.0, |(a, b)| f64::from(b) / f64::from(a)))
    }
}

/// Stable zero-based position of a beat in a track.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct BeatAddress {
    /// Bar index.
    pub bar: usize,
    /// Voice index inside the bar.
    pub voice: usize,
    /// Beat index inside the voice.
    pub beat: usize,
}
