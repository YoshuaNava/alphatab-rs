//! Adapter for already-parsed `guitarpro` values. No file-format parser lives here.
use crate::*;
mod helpers;
mod measures;
use measures::convert_measures;

#[derive(Debug)]
/// Converted track plus recoverable source-fidelity warnings.
pub struct ImportReport {
    /// Native rendering model produced by the adapter.
    pub track: Track,
    /// Deduplicated warnings for unsupported or inconsistent source values.
    pub warnings: Vec<String>,
}

type WarningSet = std::collections::BTreeSet<String>;

fn display_hints(
    song: &guitarpro::Song,
    source: &guitarpro::Track,
) -> Result<guitarpro::model::optimized::LoadedScore, RenderError> {
    let hints_song = guitarpro::Song {
        tracks: vec![source.clone()],
        measure_headers: song.measure_headers.clone(),
        ..Default::default()
    };
    Ok(guitarpro::convert::optimized::legacy::legacy_song_to_loaded_score(&hints_song))
}

fn build_track(
    song: &guitarpro::Song,
    source: &guitarpro::Track,
    measures: Vec<Measure>,
) -> Result<Track, RenderError> {
    Ok(Track {
        metadata: ScoreMetadata {
            title: song.name.clone(),
            subtitle: song.subtitle.clone(),
            artist: song.artist.clone(),
            album: song.album.clone(),
            words: song.words.clone(),
            music: song.author.clone(),
            copyright: song.copyright.clone(),
            instructions: song.instructions.clone(),
        },
        spans: vec![],
        capo: u16::try_from(source.offset)
            .map_err(|_| RenderError::invalid_input("invalid capo fret".into()))?,
        name: source.name.clone(),
        strings: source
            .strings
            .iter()
            .map(|(_, midi)| {
                [
                    "C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B",
                ][midi.rem_euclid(12) as usize]
                    .into()
            })
            .collect(),
        measures,
        clef: if source.percussion_track {
            Clef::Percussion
        } else if source.strings.iter().map(|s| s.1).max().unwrap_or(64) < 50 {
            Clef::Bass
        } else {
            Clef::Treble
        },
    })
}

fn apply_lyrics(
    song: &guitarpro::Song,
    source: &guitarpro::Track,
    measures: &mut [Measure],
    warnings: &mut WarningSet,
) {
    if i32::from(song.lyrics.track_choice) != source.number || song.lyrics.track_choice == 0 {
        return;
    }
    for (_, start, text) in &song.lyrics.lines {
        let visible = text
            .chars()
            .scan(false, |hidden, c| match c {
                '[' => {
                    *hidden = true;
                    Some(None)
                }
                ']' => {
                    *hidden = false;
                    Some(None)
                }
                _ if *hidden => Some(None),
                c => Some(Some(c)),
            })
            .flatten()
            .collect::<String>();
        let mut syllables = visible.split_whitespace().map(|s| s.replace('+', " "));
        for measure in measures
            .iter_mut()
            .skip(usize::from(start.saturating_sub(1)))
        {
            if let Some(voice) = measure.voices.first_mut() {
                for beat in voice {
                    if beat.notes.is_empty()
                        || beat.notes.iter().all(|n| matches!(n.fret, Fret::Tied(_)))
                    {
                        continue;
                    }
                    if let Some(syllable) = syllables.next() {
                        if !beat.annotations.lyrics.is_empty() {
                            beat.annotations.lyrics.push('\n');
                        }
                        beat.annotations.lyrics.push_str(&syllable);
                    }
                }
            }
        }
        if syllables.next().is_some() {
            warnings.insert("Lyrics extend beyond the available beats".into());
        }
    }
}

/// Converts one parsed Guitar Pro track into the native rendering model.
///
/// The `song` supplies shared headers and metadata. File decoding remains the
/// responsibility of the `guitarpro` crate.
pub fn convert_track(
    song: &guitarpro::Song,
    source: &guitarpro::Track,
) -> Result<ImportReport, RenderError> {
    convert_track_inner(song, source)
        .map_err(|error| RenderError::new(crate::ErrorKind::Import, error.message().to_owned()))
}

fn convert_track_inner(
    song: &guitarpro::Song,
    source: &guitarpro::Track,
) -> Result<ImportReport, RenderError> {
    let mut warnings = WarningSet::new();
    // The optimized conversion supplies display hints which are not public on the source model.
    let hints = display_hints(song, source)?;
    let hint_track = hints.score.tracks.first().ok_or_else(|| {
        RenderError::invalid_input("guitarpro display conversion returned no track".into())
    })?;
    let mut measures = convert_measures(song, source, hint_track, &mut warnings)?;
    apply_lyrics(song, source, &mut measures, &mut warnings);
    let track = build_track(song, source, measures)?;
    layout(&track, LayoutOptions::default())?;
    Ok(ImportReport {
        track,
        warnings: warnings.into_iter().collect(),
    })
}
