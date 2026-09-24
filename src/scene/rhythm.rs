//! Rhythmic scene construction helpers.
use super::*;

pub(super) fn rest(page: &mut Layout, value: i16, x: f32, y: f32) -> Result<(), RenderError> {
    let symbol = match value {
        -4 => G::RestLonga,
        -2 => G::RestDoubleWhole,
        1 => G::RestWhole,
        2 => G::RestHalf,
        4 => G::RestQuarter,
        8 => G::Rest8th,
        16 => G::Rest16th,
        32 => G::Rest32nd,
        64 => G::Rest64th,
        _ => G::Rest128th,
    };
    page.glyph_at_center(x, y, symbol, 9.0).map(|_| ())
}
pub(super) struct VoiceLayout<'a> {
    pub(super) beats: &'a [Beat],
    pub(super) xs: &'a [f32],
    pub(super) times: &'a [f64],
    pub(super) meter: (u8, u16),
    pub(super) groups: &'a [u8],
    pub(super) group_unit: u16,
}
impl VoiceLayout<'_> {
    pub(super) fn connects(&self, i: usize, j: usize) -> bool {
        let group = if self.meter.1 == 8 && self.meter.0 > 3 && self.meter.0.is_multiple_of(3) {
            1.5
        } else {
            4.0 / f64::from(self.meter.1)
        };
        let later = i.max(j);
        let same_group = if self.groups.is_empty() {
            (self.times[i] / group + 1e-8).floor() == (self.times[j] / group + 1e-8).floor()
        } else {
            let index = |time: f64| {
                let mut end = 0.0;
                self.groups.iter().position(|g| {
                    end += f64::from(*g) * 4.0 / f64::from(self.group_unit);
                    time < end - 1e-8
                })
            };
            index(self.times[i]) == index(self.times[j])
        };
        self.beats[later].annotations.beaming != Beaming::Break
            && self.beats[i].annotations.stem == self.beats[j].annotations.stem
            && !self.beats[i].notes.is_empty()
            && self.beats[i].duration.value >= 8
            && !self.beats[j].notes.is_empty()
            && self.beats[j].duration.value >= 8
            && (same_group || self.beats[later].annotations.beaming == Beaming::Join)
            && self.beats[i].duration.tuplet == self.beats[j].duration.tuplet
    }
    pub(super) fn grouped(&self, i: usize) -> bool {
        (i > 0 && self.connects(i, i - 1)) || (i + 1 < self.beats.len() && self.connects(i, i + 1))
    }
}
pub(super) fn draw_beams(
    page: &mut Layout,
    voice: &VoiceLayout<'_>,
    i: usize,
    end: f32,
    down: bool,
) -> Result<(), RenderError> {
    let beam_width = crate::music_font::thickness(
        crate::music_font::metadata()
            .engraving_defaults
            .beam_thickness,
        10.0,
        3.5,
    );
    let x = voice.xs[i];
    let levels = voice.beats[i].duration.beam_level_count();
    let left = i > 0 && voice.connects(i, i - 1);
    let right = i + 1 < voice.beats.len() && voice.connects(i, i + 1);
    if levels > 0 && !left && !right {
        page.glyph_at_origin(x, end, flag_glyph(levels, down), 8.0)?;
    }
    let direction = if down { -1.0 } else { 1.0 };
    for level in 0..levels {
        let yy = end + direction * level as f32 * 5.0;
        if right
            && voice.beats[i + 1].duration.beam_level_count() > level
            && (voice.beats[i + 1].annotations.break_secondary == 0
                || level < u32::from(voice.beats[i + 1].annotations.break_secondary))
        {
            page.line(x, yy, voice.xs[i + 1], yy, beam_width);
        } else if left || right {
            let left_has = left
                && voice.beats[i - 1].duration.beam_level_count() > level
                && (voice.beats[i].annotations.break_secondary == 0
                    || level < u32::from(voice.beats[i].annotations.break_secondary));
            if !left_has {
                page.line(x, yy, x + if right { 9.0 } else { -9.0 }, yy, beam_width);
            }
        }
    }
    Ok(())
}
pub(super) fn rhythm(
    page: &mut Layout,
    context: &VoiceLayout<'_>,
    i: usize,
    y: f32,
) -> Result<(), RenderError> {
    let beat = &context.beats[i];
    let xs = context.xs;
    let x = xs[i];
    let value = beat.duration.value;
    if beat.notes.is_empty() {
        rest(page, value, x, y + 12.0)?;
    } else if value <= 1 {
        page.glyph_at_center(x, y + 12.0, NoteHead::Normal.resolve_glyph(value), 8.0)?;
    } else {
        page.line(x, y, x, y + 25.0, 1.2);
        if value == 2 {
            page.glyph_at_center(x, y, G::NoteheadHalf, 8.0)?;
        }
        draw_beams(page, context, i, y + 25.0, true)?;
    }
    for dot in 0..beat.duration.dots {
        page.glyph_at_center(
            x + 10.0 + dot as f32 * 5.0,
            y + 8.0,
            G::AugmentationDot,
            7.0,
        )?;
    }
    draw_tuplets(page, context.beats, xs, i, y + 40.0)?;
    Ok(())
}

pub(super) fn draw_tuplets(
    page: &mut Layout,
    voice: &[Beat],
    xs: &[f32],
    i: usize,
    y: f32,
) -> Result<(), RenderError> {
    let ratios = |b: &Beat| {
        b.annotations
            .tuplets
            .iter()
            .copied()
            .chain(b.duration.tuplet)
            .collect::<Vec<_>>()
    };
    let layers = ratios(&voice[i]);
    for (depth, &(a, b)) in layers.iter().enumerate() {
        let prefix = &layers[..=depth];
        let run_start = (0..i)
            .rev()
            .find(|j| {
                !ratios(&voice[*j]).starts_with(prefix)
                    || voice[*j].annotations.tuplet_end
                    || voice[*j + 1].annotations.tuplet_start
            })
            .map_or(0, |j| j + 1);
        // Count notated duration, rather than assuming every beat has equal length.
        let unit = voice[run_start..]
            .iter()
            .take_while(|b| ratios(b).starts_with(prefix))
            .map(|b| {
                b.duration.undotted_quarter_beats()
                    * ratios(b)
                        .iter()
                        .skip(depth + 1)
                        .map(|(_, normal)| f64::from(*normal))
                        .product::<f64>()
            })
            .fold(f64::INFINITY, f64::min);
        let group_length = unit * f64::from(a);
        let elapsed: f64 = voice[run_start..i]
            .iter()
            .map(|beat| {
                let r = ratios(beat);
                let inner: f64 = r
                    .iter()
                    .skip(depth + 1)
                    .map(|(a, b)| f64::from(*b) / f64::from(*a))
                    .product();
                beat.duration.undotted_quarter_beats()
                    * beat.duration.augmentation_dot_factor()
                    * inner
            })
            .sum();
        if (elapsed / group_length - (elapsed / group_length).round()).abs() > 1e-7 {
            continue;
        }
        let mut end = i;
        let mut length = 0.0;
        for (j, beat) in voice.iter().enumerate().skip(i) {
            let r = ratios(beat);
            if !r.starts_with(prefix) {
                break;
            }
            length += beat.duration.undotted_quarter_beats()
                * beat.duration.augmentation_dot_factor()
                * r.iter()
                    .skip(depth + 1)
                    .map(|(a, b)| f64::from(*b) / f64::from(*a))
                    .product::<f64>();
            end = j;
            if length >= group_length - 1e-8 || beat.annotations.tuplet_end {
                break;
            }
        }
        let yy = y - depth as f32 * 16.0;
        let bracket = voice[i..=end]
            .iter()
            .any(|beat| beat.annotations.force_tuplet_bracket || beat.notes.is_empty())
            || voice[i..=end]
                .iter()
                .any(|beat| beat.duration.beam_level_count() == 0);
        if bracket {
            page.line(xs[i] - 8.0, yy, xs[end] + 8.0, yy, 1.0);
            for xx in [xs[i] - 8.0, xs[end] + 8.0] {
                page.line(xx, yy, xx, yy - 5.0, 1.0);
            }
        }
        page.text(
            (xs[i] + xs[end]) / 2.0,
            yy,
            if b == 2 || b == 4 {
                a.to_string()
            } else {
                format!("{a}:{b}")
            },
            11.0,
            true,
        );
    }
    Ok(())
}

pub(super) fn stem_down(beat: &Beat, voice: usize) -> bool {
    match beat.annotations.stem {
        StemDirection::Down => true,
        StemDirection::Up => false,
        StemDirection::Auto => voice % 2 == 1,
    }
}
