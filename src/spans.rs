//! Connections and effect bands are laid out after beat geometry is known.
use crate::*;

pub(crate) fn contains(span: &Span, address: BeatAddress) -> bool {
    address.voice == span.start.voice
        && (address.measure, address.beat) >= (span.start.measure, span.start.beat)
        && (address.measure, address.beat) <= (span.end.measure, span.end.beat)
}

fn beat(track: &Track, address: BeatAddress) -> Result<&Beat, RenderError> {
    track
        .measures
        .get(address.measure)
        .and_then(|m| m.voices.get(address.voice))
        .and_then(|v| v.get(address.beat))
        .ok_or_else(|| RenderError::invalid_input(format!("invalid span endpoint {address:?}")))
}

#[derive(Clone, Copy)]
struct OutgoingLegato {
    address: BeatAddress,
    note: usize,
    fret: u16,
    hammer: bool,
    slide: bool,
}

/// A beat-local effect which becomes one contiguous span during preparation.
/// Keeping extraction and clearing together makes adding a new range effect a
/// single, reviewable change rather than another numeric branch in the pass.
#[derive(Clone, Copy)]
enum AutoRange {
    PalmMute,
    LetRing,
    Rasgueado,
    Ottava,
    Crescendo,
    Barre,
}

impl AutoRange {
    const ALL: [Self; 6] = [
        Self::PalmMute,
        Self::LetRing,
        Self::Rasgueado,
        Self::Ottava,
        Self::Crescendo,
        Self::Barre,
    ];

    /// Removes this local mark and returns the range kind it contributes.
    fn take(self, beat: &mut Beat) -> Option<SpanKind> {
        match self {
            Self::PalmMute => {
                let active = beat.notes.iter().any(|note| note.effects.palm_mute);
                for note in &mut beat.notes {
                    note.effects.palm_mute = false;
                }
                active.then_some(SpanKind::PalmMute)
            }
            Self::LetRing => {
                let active = beat.notes.iter().any(|note| note.effects.let_ring);
                for note in &mut beat.notes {
                    note.effects.let_ring = false;
                }
                active.then_some(SpanKind::LetRing)
            }
            Self::Rasgueado => beat
                .annotations
                .rasgueado
                .then_some(SpanKind::Rasgueado)
                .inspect(|_| {
                    beat.annotations.rasgueado = false;
                }),
            Self::Ottava => beat.annotations.ottava.take().map(SpanKind::Ottava),
            Self::Crescendo => beat.annotations.crescendo.take().map(|growing| {
                if growing {
                    SpanKind::Crescendo
                } else {
                    SpanKind::Diminuendo
                }
            }),
            Self::Barre => beat.annotations.barre.take().map(SpanKind::Text),
        }
    }
}

fn same_range_kind(left: &SpanKind, right: &SpanKind) -> bool {
    match (left, right) {
        (SpanKind::Ottava(a), SpanKind::Ottava(b)) => a == b,
        (SpanKind::Text(a), SpanKind::Text(b)) => a == b,
        (a, b) => std::mem::discriminant(a) == std::mem::discriminant(b),
    }
}

fn range_placement(kind: &SpanKind) -> Placement {
    if matches!(kind, SpanKind::Crescendo | SpanKind::Diminuendo) {
        Placement::Below
    } else {
        Placement::Above
    }
}

pub(crate) fn prepare(track: &Track, options: LayoutOptions) -> Result<Track, RenderError> {
    let mut track = track.clone();
    if options.engraving.display_transposition != 0 {
        let transpose = |pitch: &mut Option<Pitch>| -> Result<(), RenderError> {
            if let Some(value) = pitch {
                let midi =
                    i16::from(value.midi()?) + i16::from(options.engraving.display_transposition);
                if !(0..=127).contains(&midi) {
                    return Err(RenderError::invalid_input(
                        "display transposition moves a pitch outside MIDI range".into(),
                    ));
                }
                *value = Pitch::from_midi(midi as u8, value.accidental < 0);
            }
            Ok(())
        };
        for note in track
            .measures
            .iter_mut()
            .flat_map(|m| &mut m.voices)
            .flatten()
            .flat_map(|beat| &mut beat.notes)
        {
            transpose(&mut note.pitch)?;
            transpose(&mut note.effects.grace_pitch)?;
            transpose(&mut note.effects.harmonic_pitch)?;
        }
    }
    if options.engraving.fingering_mode == FingeringMode::Piano {
        let piano = |value: &mut Option<String>| {
            if let Some(label) = value {
                *label = match label.as_str() {
                    "p" | "T" => "1",
                    "i" | "1" => "2",
                    "m" | "2" => "3",
                    "a" | "3" => "4",
                    "c" | "4" => "5",
                    other => other,
                }
                .to_string();
            }
        };
        for note in track
            .measures
            .iter_mut()
            .flat_map(|m| &mut m.voices)
            .flatten()
            .flat_map(|beat| &mut beat.notes)
        {
            piano(&mut note.effects.fingering);
            piano(&mut note.effects.right_fingering);
        }
    }
    for span in &track.spans {
        let first = beat(&track, span.start)?;
        let last = beat(&track, span.end)?;
        if span.start.voice != span.end.voice
            || (span.start.measure, span.start.beat) > (span.end.measure, span.end.beat)
        {
            return Err(RenderError::invalid_input(
                "span endpoints must be ordered in the same voice".into(),
            ));
        }
        if let SpanKind::Slur {
            start_note,
            end_note,
        }
        | SpanKind::Legato {
            start_note,
            end_note,
            ..
        } = span.kind
        {
            if first.notes.is_empty()
                || last.notes.is_empty()
                || start_note.is_some_and(|i| i >= first.notes.len())
                || end_note.is_some_and(|i| i >= last.notes.len())
            {
                return Err(RenderError::invalid_input(
                    "slur endpoints must reference notes".into(),
                ));
            }
        }
        if matches!(span.kind, SpanKind::Beam) {
            if span.start == span.end {
                return Err(RenderError::invalid_input(
                    "a beam group needs at least two beats".into(),
                ));
            }
            for (mi, m) in track.measures.iter().enumerate() {
                if let Some(voice) = m.voices.get(span.start.voice) {
                    for (bi, b) in voice.iter().enumerate() {
                        if contains(
                            span,
                            BeatAddress {
                                measure: mi,
                                voice: span.start.voice,
                                beat: bi,
                            },
                        ) && (b.notes.is_empty()
                            || b.duration.value < 8
                            || b.annotations.stem != first.annotations.stem)
                        {
                            return Err(RenderError::invalid_input("explicit beams require short notes with a consistent stem direction".into()));
                        }
                    }
                }
            }
        }
    }
    for (i, a) in track
        .spans
        .iter()
        .enumerate()
        .filter(|(_, s)| matches!(s.kind, SpanKind::Beam))
    {
        if track.spans[..i].iter().any(|b| {
            matches!(b.kind, SpanKind::Beam) && (contains(a, b.start) || contains(b, a.start))
        }) {
            return Err(RenderError::invalid_input(
                "explicit beam groups cannot overlap".into(),
            ));
        }
    }
    let mut previous: std::collections::HashMap<(usize, usize), OutgoingLegato> =
        std::collections::HashMap::new();
    for (mi, m) in track.measures.iter_mut().enumerate() {
        previous.retain(|(vi, _), _| m.voices.get(*vi).is_some_and(|v| !v.is_empty()));
        for (vi, voice) in m.voices.iter_mut().enumerate() {
            for (bi, b) in voice.iter_mut().enumerate() {
                if b.notes.is_empty() {
                    previous.retain(|(v, _), _| *v != vi);
                }
                let address = BeatAddress {
                    measure: mi,
                    voice: vi,
                    beat: bi,
                };
                for (ni, n) in b.notes.iter_mut().enumerate() {
                    let fret = match n.fret {
                        Fret::Number(f) | Fret::Tied(f) => f,
                        Fret::Dead => 0,
                    };
                    if let Some(&OutgoingLegato {
                        address: start,
                        note: start_note,
                        fret: prior_fret,
                        hammer,
                        slide,
                    }) = previous.get(&(vi, n.string))
                    {
                        if hammer || slide {
                            let text = if hammer {
                                if fret > prior_fret {
                                    "H"
                                } else {
                                    "P"
                                }
                            } else {
                                "sl."
                            };
                            track.spans.push(Span {
                                start,
                                end: address,
                                kind: SpanKind::Legato {
                                    start_note: Some(start_note),
                                    end_note: Some(ni),
                                    text: text.into(),
                                },
                                placement: Placement::Above,
                            });
                        }
                    }
                    previous.insert(
                        (vi, n.string),
                        OutgoingLegato {
                            address,
                            note: ni,
                            fret,
                            hammer: n.effects.hammer_on,
                            slide: n.effects.slide_legato,
                        },
                    );
                    n.effects.hammer_on = false;
                    n.effects.slide_legato = false;
                }
            }
        }
    }
    if options.display == DisplayMode::Slash {
        for span in &mut track.spans {
            if let SpanKind::Slur {
                start_note,
                end_note,
            }
            | SpanKind::Legato {
                start_note,
                end_note,
                ..
            } = &mut span.kind
            {
                *start_note = None;
                *end_note = None;
            }
        }
        track.clef = Clef::Treble;
        for m in &mut track.measures {
            m.clef = None;
            m.key_signature = 0;
            for b in m.voices.iter_mut().flatten() {
                b.notes.truncate(1);
                if let Some(note) = b.notes.first_mut() {
                    note.pitch = Some(Pitch {
                        step: 6,
                        octave: 4,
                        accidental: 0,
                    });
                    note.effects.head = NoteHead::Slash;
                }
            }
        }
    }
    let voices = track
        .measures
        .iter()
        .map(|m| m.voices.len())
        .max()
        .unwrap_or(0);
    // Group repeated per-beat effects into a single range; silence terminates a run.
    for vi in 0..voices {
        for effect in AutoRange::ALL {
            let mut run: Option<Span> = None;
            for (mi, m) in track.measures.iter_mut().enumerate() {
                let Some(voice) = m.voices.get_mut(vi) else {
                    if let Some(span) = run.take() {
                        track.spans.push(span);
                    }
                    continue;
                };
                if voice.is_empty() {
                    if let Some(span) = run.take() {
                        track.spans.push(span);
                    }
                }
                for (bi, b) in voice.iter_mut().enumerate() {
                    let kind = effect.take(b);
                    let address = BeatAddress {
                        measure: mi,
                        voice: vi,
                        beat: bi,
                    };
                    if let Some(kind) = kind {
                        let same = run
                            .as_ref()
                            .is_some_and(|span| same_range_kind(&span.kind, &kind));
                        if same {
                            run.as_mut().unwrap().end = address;
                        } else {
                            if let Some(span) = run.take() {
                                track.spans.push(span);
                            }
                            run = Some(Span {
                                start: address,
                                end: address,
                                placement: range_placement(&kind),
                                kind,
                            });
                        }
                    } else if let Some(span) = run.take() {
                        track.spans.push(span);
                    }
                }
            }
            if let Some(span) = run {
                track.spans.push(span);
            }
        }
    }
    if !options.show_effects || !options.elements.effects {
        track.spans.retain(|s| {
            matches!(
                s.kind,
                SpanKind::Beam | SpanKind::Slur { .. } | SpanKind::Legato { .. }
            )
        });
    }
    for m in &mut track.measures {
        for b in m.voices.iter_mut().flatten() {
            if !options.show_chords || !options.elements.chord_diagrams {
                b.annotations.chord = None;
            }
            if !options.show_dynamics || !options.elements.dynamics {
                b.annotations.dynamic = None;
            }
            if !options.show_lyrics || !options.elements.lyrics {
                b.annotations.lyrics.clear();
            }
            if !options.show_effects || !options.elements.effects {
                b.annotations.whammy.clear();
                b.annotations.technique = None;
                b.annotations.fade = None;
                b.annotations.pedal = None;
                b.annotations.wah_open = None;
                b.annotations.golpe = false;
                b.annotations.left_hand_tap = false;
                for n in &mut b.notes {
                    let head = n.effects.head;
                    n.effects = NoteEffects {
                        head,
                        ..Default::default()
                    };
                }
            }
        }
    }
    Ok(track)
}

pub(crate) fn metadata(page: &mut Layout, track: &Track, options: LayoutOptions) -> f32 {
    let mut y = 48.0;
    if options.show_metadata {
        let m = &track.metadata;
        let authors = if m.words == m.music && !m.words.is_empty() {
            format!("Words and music: {}", m.words)
        } else {
            [
                (!m.words.is_empty()).then(|| format!("Words: {}", m.words)),
                (!m.music.is_empty()).then(|| format!("Music: {}", m.music)),
            ]
            .into_iter()
            .flatten()
            .collect::<Vec<_>>()
            .join(" · ")
        };
        for (visible, text, size) in [
            (options.elements.title, &m.title, 24.0),
            (options.elements.subtitle, &m.subtitle, 16.0),
            (options.elements.artist, &m.artist, 14.0),
            (options.elements.album, &m.album, 12.0),
            (
                options.elements.words || options.elements.music,
                &authors,
                12.0,
            ),
            (options.elements.copyright, &m.copyright, 10.0),
            (options.elements.instructions, &m.instructions, 11.0),
        ] {
            if !visible {
                continue;
            }
            for line in text.lines().filter(|l| !l.is_empty()) {
                let mut wrapped = String::new();
                for word in line.split_whitespace() {
                    let candidate = if wrapped.is_empty() {
                        word.into()
                    } else {
                        format!("{wrapped} {word}")
                    };
                    if crate::text::width(&candidate, size) > page.width - 64.0
                        && !wrapped.is_empty()
                    {
                        page.text(page.width / 2.0, y, &wrapped, size, false);
                        y += size + 8.0;
                        wrapped = word.into();
                    } else {
                        wrapped = candidate;
                    }
                }
                page.text(page.width / 2.0, y, &wrapped, size, false);
                y += size + 8.0;
            }
        }
    }
    if options.show_tuning && options.elements.tuning && !track.strings.is_empty() {
        page.text(
            page.width / 2.0,
            y,
            format!(
                "Tuning: {}",
                track
                    .strings
                    .iter()
                    .rev()
                    .cloned()
                    .collect::<Vec<_>>()
                    .join(" – ")
            ),
            11.0,
            false,
        );
        y += 20.0;
    }
    if options.elements.capo && track.capo > 0 {
        page.text(
            page.width / 2.0,
            y,
            format!("Capo: {}", track.capo),
            11.0,
            false,
        );
        y += 20.0;
    }
    y - 48.0
}

fn clef(track: &Track, measure: usize) -> Clef {
    track.measures[..=measure]
        .iter()
        .rev()
        .find_map(|m| m.clef)
        .unwrap_or(track.clef)
}
fn anchor(
    track: &Track,
    b: &BeatBounds,
    note: Option<usize>,
    staff: bool,
    above: bool,
    options: LayoutOptions,
) -> [f32; 2] {
    let beat = &track.measures[b.measure].voices[b.voice][b.beat];
    if options.display == DisplayMode::Numbered {
        let i = note.unwrap_or(if above {
            beat.notes.len().saturating_sub(1)
        } else {
            0
        });
        return [
            (b.cursor_rect[0] + b.cursor_rect[2]) / 2.0,
            b.cursor_rect[1] + 28.0 + b.voice as f32 * 60.0 - i as f32 * 24.0,
        ];
    }
    let y = |n: &Note| {
        if staff {
            crate::engrave::pitch_y(
                n.pitch.expect("validated pitch"),
                clef(track, b.measure),
                b.cursor_rect[1] + 8.0,
            )
        } else {
            b.cursor_rect[3]
                - 8.0
                - track.strings.len().saturating_sub(n.string) as f32 * options.string_spacing
        }
    };
    let yy = if let Some(n) = note {
        y(&beat.notes[n])
    } else {
        beat.notes
            .iter()
            .map(y)
            .reduce(if above { f32::min } else { f32::max })
            .unwrap_or(b.cursor_rect[1])
    };
    [
        (b.cursor_rect[0] + b.cursor_rect[2]) / 2.0
            + if staff {
                crate::engrave::voice_offset(&track.measures[b.measure], b.voice, b.start)
            } else {
                0.0
            },
        yy,
    ]
}

pub(crate) fn draw(
    page: &mut Layout,
    track: &Track,
    options: LayoutOptions,
    owners: &mut Vec<usize>,
) -> Result<(), RenderError> {
    let staff = options.display.staff()
        || options.display == DisplayMode::Numbered
        || track.clef == Clef::Percussion;
    let tab = options.display.tab() && track.clef != Clef::Percussion;
    for span in &track.spans {
        let selected: Vec<_> = page
            .beats
            .iter()
            .filter(|b| {
                contains(
                    span,
                    BeatAddress {
                        measure: b.measure,
                        voice: b.voice,
                        beat: b.beat,
                    },
                )
            })
            .cloned()
            .collect();
        for (si, system) in page.systems.clone().into_iter().enumerate() {
            let part: Vec<_> = selected
                .iter()
                .filter(|b| b.cursor_rect[1] >= system[0] && b.cursor_rect[1] < system[1])
                .collect();
            let (Some(first), Some(last)) = (part.first(), part.last()) else {
                continue;
            };
            let starts = (first.measure, first.beat) == (span.start.measure, span.start.beat);
            let ends = (last.measure, last.beat) == (span.end.measure, span.end.beat);
            if matches!(span.kind, SpanKind::Beam) {
                for is_staff in [true, false].into_iter().filter(|s| {
                    if *s {
                        staff
                    } else {
                        tab && options.tab_rhythm != TabRhythm::Hidden
                    }
                }) {
                    draw_beam(page, track, &part, is_staff, options)?;
                }
                owners.resize(page.primitives.len(), si);
                continue;
            }
            let above = span.placement == Placement::Above;
            let sign = if above { -1.0 } else { 1.0 };
            let slur = match &span.kind {
                SpanKind::Slur {
                    start_note,
                    end_note,
                } => Some((*start_note, *end_note, None)),
                SpanKind::Legato {
                    start_note,
                    end_note,
                    text,
                } => Some((*start_note, *end_note, Some(text.as_str()))),
                _ => None,
            };
            if let Some((start_note, end_note, label)) = slur {
                for is_staff in [true, false]
                    .into_iter()
                    .filter(|s| if *s { staff } else { tab })
                {
                    let mut from = anchor(
                        track,
                        first,
                        if starts { start_note } else { None },
                        is_staff,
                        above,
                        options,
                    );
                    let mut to = anchor(
                        track,
                        last,
                        if ends { end_note } else { None },
                        is_staff,
                        above,
                        options,
                    );
                    from[0] = if starts { from[0] + 7.0 } else { 48.0 };
                    to[0] = if ends { to[0] - 7.0 } else { page.width - 24.0 };
                    if to[0] <= from[0] {
                        to[0] = from[0] + 14.0;
                    }
                    from[1] += sign * 8.0;
                    to[1] += sign * 8.0;
                    let mut arch = options.engraving.slur_height;
                    // Solve the parabola's required height at each intervening obstacle.
                    for p in &page.primitives {
                        let r = p.bounds();
                        let xx = ((r[0] + r[2]) / 2.0).clamp(from[0], to[0]);
                        let t = (xx - from[0]) / (to[0] - from[0]);
                        let staff_top = if is_staff {
                            first.cursor_rect[1] + 8.0
                        } else {
                            first.cursor_rect[3]
                                - 8.0
                                - track.strings.len().saturating_sub(1) as f32
                                    * options.string_spacing
                        };
                        let staff_bottom = if is_staff {
                            staff_top + 40.0
                        } else {
                            first.cursor_rect[3] - 8.0
                        };
                        if !(0.08..=0.92).contains(&t)
                            || r[3] < staff_top - 45.0
                            || r[1] > staff_bottom + 20.0
                        {
                            continue;
                        }
                        if matches!(p, Primitive::Line { from,to,width, .. } if *width <= 1.2 && ((from[1]-to[1]).abs()<0.1 && (from[0]-to[0]).abs()>100.0 || (from[0]-to[0]).abs()<0.1 && (from[1]-to[1]).abs()>45.0))
                        {
                            continue;
                        }
                        let base = from[1] + t * (to[1] - from[1]);
                        let edge = if above { r[1] - 5.0 } else { r[3] + 5.0 };
                        arch = arch.max(((edge - base) * sign) / (4.0 * t * (1.0 - t)));
                    }
                    page.curve(from, to, sign * arch);
                    if starts {
                        if let Some(label) = label {
                            page.text(
                                (from[0] + to[0]) / 2.0,
                                (from[1] + to[1]) / 2.0 + sign * (arch + 8.0),
                                label,
                                9.0,
                                false,
                            );
                        }
                    }
                }
                owners.resize(page.primitives.len(), si);
                continue;
            }
            let mut left = if starts {
                first.cursor_rect[0] + 6.0
            } else {
                48.0
            };
            let mut right = if ends {
                last.cursor_rect[2] - 6.0
            } else {
                page.width - 24.0
            };
            let mut yy = if above {
                first.cursor_rect[1] - 36.0
            } else {
                first.rect[3] + 24.0
            };
            if !above && matches!(span.kind, SpanKind::Crescendo | SpanKind::Diminuendo) {
                yy = first.rect[1] + 41.0;
                if let Some(d) = &track.measures[first.measure].voices[first.voice][first.beat]
                    .annotations
                    .dynamic
                {
                    left = (first.cursor_rect[0] + first.cursor_rect[2]) / 2.0
                        + crate::text::width(d, 16.0) / 2.0
                        + 10.0;
                }
                if part.len() > 1 {
                    if let Some(d) = &track.measures[last.measure].voices[last.voice][last.beat]
                        .annotations
                        .dynamic
                    {
                        right = (last.cursor_rect[0] + last.cursor_rect[2]) / 2.0
                            - crate::text::width(d, 16.0) / 2.0
                            - 10.0;
                    }
                }
            }
            // Place the entire effect band in a free lane, keeping its label and line together.
            loop {
                let collision = page.primitives.iter().any(|p| {
                    let r = p.bounds();
                    r[0] < right && r[2] > left && r[1] < yy + 9.0 && r[3] > yy - 9.0
                });
                if !collision {
                    break;
                }
                yy += sign * 18.0;
            }
            match &span.kind {
                SpanKind::Crescendo | SpanKind::Diminuendo => {
                    let growing = matches!(span.kind, SpanKind::Crescendo);
                    let total = selected.len().max(1) as f32;
                    let offset = selected
                        .iter()
                        .position(|b| b.measure == first.measure && b.beat == first.beat)
                        .unwrap() as f32;
                    let a = if growing {
                        offset / total
                    } else {
                        1.0 - offset / total
                    } * 5.0;
                    let b = if growing {
                        (offset + part.len() as f32) / total
                    } else {
                        1.0 - (offset + part.len() as f32) / total
                    } * 5.0;
                    page.line(left, yy - a, right, yy - b, 1.0);
                    page.line(left, yy + a, right, yy + b, 1.0);
                }
                SpanKind::Vibrato | SpanKind::Trill => {
                    let start = if matches!(span.kind, SpanKind::Trill) {
                        page.glyph_at_origin(left, yy, smufl::Glyph::OrnamentTrill, 8.0)?;
                        left + 22.0
                    } else {
                        left
                    };
                    let mut x = start;
                    while x + 6.0 <= right {
                        page.line(x, yy, x + 3.0, yy - 3.0, 1.0);
                        page.line(x + 3.0, yy - 3.0, x + 6.0, yy, 1.0);
                        x += 6.0;
                    }
                }
                kind => {
                    let text = match kind {
                        SpanKind::Ottava(o) => o.label(),
                        SpanKind::PalmMute => "P.M.",
                        SpanKind::LetRing => "let ring",
                        SpanKind::Pedal => "Ped.",
                        SpanKind::Rasgueado => "Rasg.",
                        SpanKind::Text(t) => t,
                        _ => unreachable!(),
                    };
                    let label = if starts {
                        text.to_string()
                    } else {
                        format!("({text})")
                    };
                    let w = crate::text::width(&label, 10.0);
                    page.text(left + w / 2.0, yy, label, 10.0, false);
                    let mut x = left + w + 5.0;
                    while x < right {
                        page.line(x, yy, (x + 4.0).min(right), yy, 0.9);
                        x += 7.0;
                    }
                    if ends {
                        page.line(right, yy, right, yy - sign * 6.0, 1.0);
                    }
                }
            }
            owners.resize(page.primitives.len(), si);
        }
    }
    pack_systems(page, owners);
    Ok(())
}

/// Expand systems to contain routed curves and effects; shift every hit region with them.
fn pack_systems(page: &mut Layout, owners: &[usize]) {
    if page.systems.is_empty() {
        return;
    }
    let original = page.systems.clone();
    let mut extents = original.clone();
    for (p, &si) in page.primitives.iter().zip(owners) {
        let r = p.bounds();
        extents[si][0] = extents[si][0].min(r[1] - 8.0);
        extents[si][1] = extents[si][1].max(r[3] + 8.0);
    }
    let mut shifts = Vec::new();
    let mut y = 0.0;
    for (si, range) in extents.iter().enumerate() {
        shifts.push(y - range[0]);
        page.systems[si] = [y, y + range[1] - range[0]];
        y = page.systems[si][1];
    }
    for (p, &si) in page.primitives.iter_mut().zip(owners) {
        match p {
            Primitive::Line { from, to, .. } => {
                from[1] += shifts[si];
                to[1] += shifts[si];
            }
            Primitive::Curve { points, .. } => {
                for point in points {
                    point[1] += shifts[si];
                }
            }
            Primitive::Text { at, .. } | Primitive::Glyph { at, .. } => at[1] += shifts[si],
        }
    }
    for b in &mut page.beats {
        let si = original
            .iter()
            .position(|r| b.cursor_rect[1] >= r[0] && b.cursor_rect[1] < r[1])
            .expect("beat belongs to a system");
        for r in [&mut b.rect, &mut b.cursor_rect] {
            r[1] += shifts[si];
            r[3] += shifts[si];
        }
    }
    page.height = y;
}

fn draw_beam(
    page: &mut Layout,
    track: &Track,
    part: &[&BeatBounds],
    staff: bool,
    options: LayoutOptions,
) -> Result<(), RenderError> {
    let first = part[0];
    if options.display == DisplayMode::Numbered {
        for (i, bounds) in part.iter().enumerate() {
            let b = &track.measures[bounds.measure].voices[bounds.voice][bounds.beat];
            let x = (bounds.cursor_rect[0] + bounds.cursor_rect[2]) / 2.0;
            for level in 0..b.duration.beam_levels() {
                let end = part
                    .get(i + 1)
                    .filter(|n| {
                        track.measures[n.measure].voices[n.voice][n.beat]
                            .duration
                            .beam_levels()
                            > level
                    })
                    .map_or(x + 8.0, |n| (n.cursor_rect[0] + n.cursor_rect[2]) / 2.0);
                let yy =
                    bounds.cursor_rect[1] + 42.0 + bounds.voice as f32 * 60.0 + level as f32 * 4.0;
                page.line(x - 8.0, yy, end, yy, 1.2);
            }
        }
        return Ok(());
    }
    let defaults = &crate::music_font::metadata().engraving_defaults;
    let stem_width = crate::music_font::thickness(defaults.stem_thickness, 10.0, 1.1);
    let beam_width = crate::music_font::thickness(defaults.beam_thickness, 10.0, 3.5);
    let b = &track.measures[first.measure].voices[first.voice][first.beat];
    let down = !staff
        || b.annotations.stem == StemDirection::Down
        || (b.annotations.stem == StemDirection::Auto && first.voice % 2 == 1);
    let sign = if down { 1.0 } else { -1.0 };
    let end = if staff {
        part.iter()
            .map(|b| anchor(track, b, None, true, !down, options)[1])
            .reduce(if down { f32::max } else { f32::min })
            .unwrap()
            + sign * 32.0
    } else {
        first.rect[1] + 30.0
    };
    for (i, bounds) in part.iter().enumerate() {
        let b = &track.measures[bounds.measure].voices[bounds.voice][bounds.beat];
        let a = anchor(track, bounds, None, staff, down, options);
        let x = a[0]
            + if staff {
                if down {
                    -5.0
                } else {
                    5.0
                }
            } else {
                0.0
            };
        page.line(
            x,
            if staff { a[1] } else { bounds.rect[1] + 5.0 },
            x,
            end,
            stem_width,
        );
        let levels = b.duration.beam_levels();
        if part.len() == 1 {
            page.glyph_at_origin(x, end, crate::engrave::flag_glyph(levels, down), 8.0)?;
        }
        for level in 0..levels {
            let yy = end - sign * level as f32 * 5.0;
            let right = part.get(i + 1).filter(|next| {
                track.measures[next.measure].voices[next.voice][next.beat]
                    .duration
                    .beam_levels()
                    > level
            });
            if let Some(next) = right {
                page.line(
                    x,
                    yy,
                    (next.cursor_rect[0] + next.cursor_rect[2]) / 2.0
                        + if staff {
                            if down {
                                -5.0
                            } else {
                                5.0
                            }
                        } else {
                            0.0
                        },
                    yy,
                    beam_width,
                );
            } else if part.len() > 1
                && (i == 0
                    || track.measures[part[i - 1].measure].voices[part[i - 1].voice]
                        [part[i - 1].beat]
                        .duration
                        .beam_levels()
                        <= level)
            {
                page.line(
                    x,
                    yy,
                    x + if i + 1 < part.len() { 9.0 } else { -9.0 },
                    yy,
                    beam_width,
                );
            }
        }
    }
    Ok(())
}
