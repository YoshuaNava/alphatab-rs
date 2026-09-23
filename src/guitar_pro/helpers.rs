//! Parser-version normalization and small value conversions.
use crate::{Duration, RenderError};

pub(super) fn convert_duration(
    source: &guitarpro::model::legacy::key_signature::Duration,
) -> Result<Duration, RenderError> {
    Ok(Duration {
        value: i16::try_from(source.value)
            .map_err(|_| RenderError::invalid_input("duration denominator out of range".into()))?,
        dots: if source.double_dotted {
            2
        } else {
            u8::from(source.dotted)
        },
        tuplet: (source.tuplet_enters != 1 || source.tuplet_times != 1)
            .then_some((source.tuplet_enters, source.tuplet_times)),
    })
}

pub(super) fn repeat_count(source: i8, major_version: u8) -> Option<u8> {
    (source >= 0).then(|| source as u8 + u8::from(major_version < 6))
}

pub(super) fn beam_groups(
    meter: &guitarpro::TimeSignature,
    major_version: u8,
) -> Result<Vec<u8>, RenderError> {
    let numerator = u32::try_from(meter.numerator)
        .map_err(|_| RenderError::invalid_input("negative meter".into()))?;
    let complete = meter.beams.iter().map(|n| u32::from(*n)).sum::<u32>()
        * u32::from(meter.denominator.value)
        == numerator * 8;
    Ok(if major_version == 5 && complete {
        meter.beams.iter().copied().filter(|n| *n > 0).collect()
    } else {
        vec![]
    })
}

pub(super) fn convert_fret(value: i16) -> Result<u16, RenderError> {
    u16::try_from(value).map_err(|_| RenderError::invalid_input(format!("negative fret {value}")))
}

pub(super) fn fingering(
    fingering: &guitarpro::Fingering,
    right: bool,
    warnings: &mut std::collections::BTreeSet<String>,
) -> Option<String> {
    use guitarpro::Fingering::*;
    let index = match fingering {
        Open => return None,
        Thumb => 0,
        Index => 1,
        Middle => 2,
        Annular => 3,
        Little => 4,
        Unknown(value) => {
            warnings.insert(format!("Unknown fingering value: {value}"));
            return None;
        }
    };
    Some(
        (if right {
            ["p", "i", "m", "a", "c"]
        } else {
            ["T", "1", "2", "3", "4"]
        })[index]
            .into(),
    )
}

pub(super) fn natural_harmonic(fret: i32) -> i32 {
    match fret {
        3 => 31,
        4 | 9 | 16 => 28,
        5 | 24 => 24,
        6 | 10 | 14 | 15 => 34,
        7 | 19 => 19,
        8 | 17 | 22 => 36,
        12 => 12,
        _ => 0,
    }
}
