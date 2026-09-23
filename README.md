# alphatab-rs

Native Rust music engraving using egui. Guitar Pro file parsing is delegated to
`guitarpro`; the renderer consumes its own notation model. SVG output uses the
same geometry and bundled Bravura outlines as egui, so music fonts do not need to
be installed on the host. Font tessellation is cached and painting skips vertically
clipped primitives.

Documentation:

- [Features and validation](doc/FEATURES.md)
- [Architecture and integration](doc/ARCHITECTURE.md)
- [Differences from alphaTab and known limitations](doc/ALPHATAB_DELTA.md)

Rendering coverage is broad, but complete upstream parity is not established.

## API and validation

The public notation structs intentionally support direct construction. Their
existing public fields are stable throughout the 1.x series; additive notation
will use nested extension types or new APIs rather than adding mandatory fields
to those structs. `Track::new`, `Measure::new`, `Beat::rest`,
`Beat::with_notes`, and the `LayoutOptions::with_*` methods cover common cases.

Call `Track::validate_for(options)` to reject all single-track layout input
before scheduling layout; `Track::validate` and `LayoutOptions::validate` are
the lighter independent checks. Layout also validates automatically.
`RenderError::kind` provides stable categories for invalid input,
imports, exports, worker failures, resource limits, and internal failures.
`QuarterTime`, `NormalizedPosition`, `SemitoneOffset`, and `EffectPoint` provide
typed alternatives where raw floating-point values would be ambiguous.

## Use in egui

```rust
use alphatab_rs::*;
let track = Track {
    name: "Example".into(),
    strings: ["E", "B", "G", "D", "A", "E"].map(String::from).to_vec(),
    measures: vec![Measure {
        voices: vec![vec![Beat {
            notes: vec![Note {
                string: 1,
                fret: Fret::Number(5),
                pitch: Some(Pitch::from_midi(69, false)),
                ..Default::default()
            }],
            ..Default::default()
        }]],
        ..Default::default()
    }],
    ..Default::default()
};
let page = layout(&track, LayoutOptions {
    display: DisplayMode::Both,
    ..Default::default()
})?;
std::fs::write("tab.svg", page.to_svg())?;
# Ok::<(), Box<dyn std::error::Error>>(())
```

In an egui UI, cache the layout until the model, display mode, or width changes:

```rust,ignore
egui::ScrollArea::both().show(ui, |ui| {
    page.show_interactive(ui, &active_beats, &mut selection);
});
```

The caller owns playback positions and selection. Click selects a beat; shift-click
extends the selection. `Layout::show_with_playback_cursor` remains available for
playback-only views. `scaled(zoom)` transforms glyphs and hit regions together.
`layout_score(&tracks, options)` aligns tracks by measure and onset, preserving
track identities in `ScoreLayout::hit_test` and its playback cursors. It requires
matching measure counts and time signatures. For per-track display settings or
different meters within the same measure partition, use `layout_score_tracks`.
`layout_instruments` adds named brace/bracket groups to consecutive `ScoreTrack`
values. `RenderStyle` controls the shared egui/SVG colors, and `LayoutCache`
caches a single-track layout by a caller-provided revision. `Layout::to_png` and
`Layout::to_pdf` export the same SVG geometry as raster PNG and vector PDF.
`paginate_to_fit(height)` reduces an oversized complete system before pagination;
`paginate(height)` retains its strict error behavior.

`LayoutOptions::elements` provides independent visibility switches for score
credits, tuning/capo, track names, chord diagrams, dynamics, lyrics, effects,
bar numbers and repeat counts. Its category switches remain useful shortcuts.

For native multi-staff scores, use `ScoreDocument` with `Instrument`, `Staff`,
and `MasterBar`. `Staff::master_bar_map` aligns parts with omitted/local measure
partitions, while `CrossStaffSpan` draws cross-staff beams or slurs. Each staff
can select display, transposition, piano/guitar fingering, slur height, system
spacing and visibility through its `LayoutOptions`. `LayoutWorker` coalesces
revision-tagged layout requests on a background thread. For portable SVG files,
`to_svg_outlined_text` converts ordinary text to paths using fonts installed on
the exporting machine.

`paginate(height)` returns pages containing complete systems and rejects systems
that cannot fit. `LayoutMode::Horizontal` produces a single scrolling system.

`Beat::start` is an optional onset in quarter-note units within the measure; omitted
onsets follow the preceding beat. The adapter preserves explicit source onsets;
empty filler beats consume no time, and empty voices retain their indices. Invalid durations, strings, pitches, diagrams, bends, and dimensions
return `RenderError`. SVG export replaces XML-forbidden control characters with
U+FFFD before escaping text.

Documented resource limits bound measures, voices, beats, simultaneous notes,
effect-curve points, and score staves. Their exported `MAX_*` constants allow
editors and importers to apply the same limits before layout.

## Guitar Pro integration

Use `guitar_pro::convert_track(&song, &track)` with an already-parsed `guitarpro::Song`.
It returns `ImportReport { track, warnings }`. Present its warnings to the user:
unrecognized values and source inconsistencies are reported rather than silently hidden.
The `music_gym` consumer exposes these warnings, all five notation modes, rest
condensation, selection and interpolated/follow playback cursors.

Supported imported guitar effects now include directional slide-in/out marks,
strum strokes, tap/slap/pop, chord barres and repeat counts. Tremolo-bar curves
render when present in the parsed model, but `guitarpro` 0.4.3 drops the GPIF
`<Whammy>` representation used by some GP6/7 files; this import limitation has been accepted. See [Features](doc/FEATURES.md) for the distinction between renderer support
and file-import coverage.

Examples, run from this directory:

```sh
cargo test
cargo run --example render_svg -- tab.svg
cargo run --example showcase -- showcase.svg
cargo run --example advanced -- advanced.svg
cargo run --example render_gp -- song.gp3 song.svg 0 both
```

The `render_gp` example uses the `guitarpro` crate for GP3/4/5, GPX and GP input. Its
track index is zero-based; display choices are `tab`, `staff`, `both`, `numbered`, and `slash`.

## Acknowledgements and attribution

alphatab-rs is derived from the design, behavior, and notation model of
[alphaTab](https://github.com/CoderLine/alphaTab). We gratefully thank alphaTab's
author, [Daniel Kuschny](https://github.com/Danielku15), and all of its
[contributors](https://github.com/CoderLine/alphaTab/graphs/contributors) for
creating and maintaining the project that made this Rust implementation
possible.

This crate has its own Rust architecture and uses the `guitarpro` crate for file
loading. Its rendering goals are informed by alphaTab's
[rendering overview](https://docs.alphatab.net/docs/introduction) and
[notation settings](https://docs.alphatab.net/docs/reference/types/notationsettings/).

alphatab-rs source code is licensed under the
[Mozilla Public License 2.0](LICENSE), matching the original alphaTab project.

Bravura's matched OTF outlines and standard SMuFL metadata are embedded at
compile time. Metadata drives glyph bounds, optical centers, stem attachment
points, and font-specific engraving defaults in the same manner as alphaTab.
Bravura is copyright Steinberg Media Technologies GmbH, distributed under the
SIL Open Font License; see [the included license](assets/Bravura-LICENSE.txt) and
[asset provenance](assets/SOURCE.md). No alphaTab source code is vendored.

## Additional notation and layout controls

Percussion tracks use `Clef::Percussion` and `Note::percussion` articulation IDs;
`percussion::resolve` provides written staff positions and notehead mappings.
Use `Measure::clef`, `navigation`, and `fermatas` for clef changes and measure
symbols. `NoteEffects` includes ornaments and pitched grace-note information.
The Guitar Pro adapter imports these where the crate exposes them, plus song lyrics.

`LayoutOptions::justify` fills rows with available spacing. `strict_width` returns
an error when minimum readable notation cannot fit the requested width.
`show_interactive` supports drag selection and uses musical onsets across voices.
`show_following` accepts a beat, fractional progress, and caller-controlled
scroll-follow flag. It can be used inside the existing egui scroll area.

```sh
cargo run --example percussion -- percussion.svg
```


## Connections, beam groups and nested tuplets

`Track::spans` contains explicit `Span` values with stable `BeatAddress` endpoints.
Use `SpanKind::Slur` for note-indexed slurs; `Beam` for beam groups that may cross
barlines; or the effect variants for hairpins, ottava, pedal, trill, palm mute,
let ring and text ranges. Spans split at system breaks. Repeated palm-mute,
let-ring, rasgueado, ottava, barre and crescendo annotations are grouped
automatically. Invalid endpoints and overlapping explicit beam groups are errors.

`Measure::beam_groups` specifies custom groups; `beam_unit` selects their unit
(default: the meter denominator). `BeatAnnotations::beaming`, `stem`, and
`break_secondary` provide individual engraving overrides.
`BeatAnnotations::tuplets` adds outer tuplet ratios to `Duration::tuplet`;
`Beat::quarter_beats()` includes every nesting level.

`Duration::value` is signed: `-4` is longa, `-2` breve, and positive powers of two
run from whole (`1`) to 256th (`256`). Up to three dots are accepted. `Pitch::in_key`
chooses key-aware spelling; explicit `Pitch` values always retain their spelling.

## Page and score controls

`LayoutOptions::multi_measure_rests` condenses shared silence while preserving
original beat addresses, hit regions and cursor timing. With multiple tracks,
a rest run condenses only when all tracks are silent and no structural change
or explicit span intervenes. `bars_per_system` provides a fixed upper limit.
`tab_rhythm` selects hidden, individual, connected or automatic tab rhythms.
Visibility controls cover metadata, tuning, chords, lyrics, dynamics, effects
and measure numbers.

`Track::metadata` holds the score credits; `capo` and optional tuning labels
provide instrument information. `ChordDiagram::fingers` follows the same string
order as `frets`; diagrams automatically expand to include wide voicings.

Both `Layout` and `ScoreLayout` support selection, scaling and pagination.
`Layout::paginate_with_headers` reserves space for repeated page titles and
footers. To combine selection and a moving cursor without painting twice:

```rust,ignore
let interaction = page.show_interactive(ui, &[], &mut selection);
page.paint_playback_cursor(ui, &interaction.response, address, fraction, follow);
```

`ScoreLayout` exposes the analogous methods with `ScoreBeatAddress` and
`ScoreSelection`, retaining track identity after zoom and pagination.
