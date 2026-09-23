//! Rest condensation keeps a mapping back to every original musical address.
use crate::*;

fn silent(m: &Measure) -> bool {
    let length = f64::from(m.time_signature.0) * 4.0 / f64::from(m.time_signature.1);
    m.simile.is_none()
        && m.voices.iter().all(|voice| {
            let mut time = 0.0;
            voice.iter().all(|b| {
                let a = &b.annotations;
                time = b.start.unwrap_or(time);
                time += b.quarter_beats().expect("validated duration");
                b.notes.is_empty()
                    && a.text.is_empty()
                    && a.lyrics.is_empty()
                    && a.chord.is_none()
                    && a.dynamic.is_none()
                    && a.tempo.is_none()
                    && a.whammy.is_empty()
                    && a.technique.is_none()
                    && a.fade.is_none()
                    && a.ottava.is_none()
                    && a.pedal.is_none()
                    && a.wah_open.is_none()
                    && !a.golpe
                    && !a.left_hand_tap
                    && !a.rasgueado
                    && a.barre.is_none()
                    && a.timer_seconds.is_none()
                    && a.crescendo.is_none()
            }) && time <= length + 1e-8
        })
}
fn boundary(track: &Track, mi: usize) -> bool {
    let m = &track.measures[mi];
    m.break_before
        || m.clef.is_some()
        || m.tempo.is_some()
        || !m.marker.is_empty()
        || m.repeat_start
        || m.repeat_end
        || m.double_bar
        || !m.alternate_endings.is_empty()
        || m.navigation.is_some()
        || !m.fermatas.is_empty()
        || m.free_time
        || m.triplet_feel.is_some()
        || (mi > 0
            && (track.measures[mi - 1].time_signature != m.time_signature
                || track.measures[mi - 1].key_signature != m.key_signature))
}
fn compact(tracks: &[Track]) -> (Vec<Track>, Vec<Vec<usize>>) {
    let mut groups = Vec::new();
    let mut first = 0;
    while first < tracks[0].measures.len() {
        let eligible = |mi: usize| {
            tracks.iter().all(|t| {
                silent(&t.measures[mi])
                    && !t
                        .spans
                        .iter()
                        .any(|s| s.start.measure <= mi && s.end.measure >= mi)
            })
        };
        let mut end = first + 1;
        if eligible(first)
            && tracks.iter().all(|t| {
                !t.measures[first].repeat_end
                    && !t.measures[first].double_bar
                    && t.measures[first].fermatas.is_empty()
                    && t.measures[first].navigation.is_none()
            })
        {
            while end < tracks[0].measures.len()
                && eligible(end)
                && tracks.iter().all(|t| !boundary(t, end))
            {
                end += 1;
            }
        }
        groups.push((first..end).collect::<Vec<_>>());
        first = end;
    }
    let compacted = tracks
        .iter()
        .map(|t| {
            let mut result = t.clone();
            result.measures = groups
                .iter()
                .map(|g| {
                    let mut m = t.measures[g[0]].clone();
                    m.display_number = Some(g[0] + 1);
                    if g.len() > 1 {
                        m.rest_count = g.len();
                        m.voices = vec![vec![Beat {
                            duration: Duration {
                                value: 1,
                                ..Default::default()
                            },
                            ..Default::default()
                        }]];
                    }
                    m
                })
                .collect();
            for s in &mut result.spans {
                s.start.measure = groups
                    .iter()
                    .position(|g| g.contains(&s.start.measure))
                    .unwrap();
                s.end.measure = groups
                    .iter()
                    .position(|g| g.contains(&s.end.measure))
                    .unwrap();
            }
            result
        })
        .collect();
    (compacted, groups)
}
fn expand(bounds: &[BeatBounds], source: &Track, groups: &[Vec<usize>]) -> Vec<BeatBounds> {
    let mut result = Vec::new();
    for b in bounds {
        let group = &groups[b.measure];
        if group.len() == 1 {
            let mut b = b.clone();
            b.measure = group[0];
            result.push(b);
            continue;
        }
        for (i, &mi) in group.iter().enumerate() {
            let m = &source.measures[mi];
            let length = f64::from(m.time_signature.0) * 4.0 / f64::from(m.time_signature.1);
            for (vi, voice) in m.voices.iter().enumerate() {
                let mut time = 0.0;
                for (bi, beat) in voice.iter().enumerate() {
                    time = beat.start.unwrap_or(time);
                    let duration = beat.quarter_beats().expect("validated duration");
                    let mut next = b.clone();
                    next.measure = mi;
                    next.voice = vi;
                    next.beat = bi;
                    next.start = time;
                    next.duration = duration;
                    for rect in [&mut next.rect, &mut next.cursor_rect] {
                        let step = (rect[2] - rect[0]) / group.len() as f32;
                        let left = rect[0] + i as f32 * step;
                        rect[0] = left + step * (time / length) as f32;
                        rect[2] = left + step * ((time + duration) / length).min(1.0) as f32;
                    }
                    result.push(next);
                    time += duration;
                }
            }
        }
    }
    result
}
pub(crate) fn single(track: &Track, mut options: LayoutOptions) -> Result<Layout, RenderError> {
    let (tracks, groups) = compact(std::slice::from_ref(track));
    options.multi_measure_rests = false;
    let mut page = layout(&tracks[0], options)?;
    page.beats = expand(&page.beats, track, &groups);
    Ok(page)
}
pub(crate) fn score(
    tracks: &[Track],
    mut options: LayoutOptions,
) -> Result<ScoreLayout, RenderError> {
    let (compacted, groups) = compact(tracks);
    options.multi_measure_rests = false;
    let mut page = layout_score(&compacted, options)?;
    page.beats = tracks
        .iter()
        .enumerate()
        .flat_map(|(ti, t)| {
            let bounds: Vec<_> = page
                .beats
                .iter()
                .filter(|b| b.track == ti)
                .map(|b| b.beat.clone())
                .collect();
            expand(&bounds, t, &groups)
                .into_iter()
                .map(move |beat| ScoreBeatBounds { track: ti, beat })
        })
        .collect();
    Ok(page)
}
