# Rendering coverage

The reference is [CoderLine/alphaTab](https://github.com/CoderLine/alphaTab), pinned to
[`0a3af85833133a0e450223ae286fb681dea8bdda`](https://github.com/CoderLine/alphaTab/tree/0a3af85833133a0e450223ae286fb681dea8bdda).
The scope is native Rust notation rendering with egui and SVG output. Guitar Pro
parsing stays in `guitarpro`; there is no second file parser in this library.

Upstream visual verification and GP6/7 `<Whammy>` data missing from `guitarpro`
are the two agreed exclusions. Other file importers, alphaTex, audio synthesis,
and upstream's browser/platform wrappers are outside the rendering-only scope.
This is an independent engraver, not a line-by-line port or a claim of
pixel-identical output.

## Status and boundaries

This inventory describes implemented code, not certified alphaTab feature parity.
See [Architecture](ARCHITECTURE.md) for the implementation and
[alphaTab differences](ALPHATAB_DELTA.md) for missing functionality and limitations.
The staff-aware API, primitive-class and individual primitive colors, expanded
engraving settings, background layout and portable exports are implemented.
Public settings and models remain Rust-native rather than API-compatible with
alphaTab; see the delta document for the remaining structural distinctions.

## Implemented rendering features

The former gap-candidate list has been replaced with concrete capabilities and
local regression coverage. “Collision handling” below names the algorithms
actually implemented, rather than promising an unspecified universal engraver.

| Area | Implemented behavior | Main regression coverage |
| --- | --- | --- |
| Output | Native egui glyph meshes, SVG music outlines, portable SVG with text outlines, raster PNG and vector PDF export from shared geometry; semantic measured elements and bounds-driven annotation stacks; native cubic Bézier curves; canonical named SMuFL glyph identities; cached Bravura outlines and matched metadata for bounds, advance widths, stem anchors and engraving defaults; viewport culling; measured text; independent glyph/line/text defaults and per-primitive color overrides | `elements.rs`: measurement and lanes; `music_font.rs` and `glyph.rs`: Bravura metadata and unified cache; `rendering.rs`: native egui mesh smoke test, SVG escaping, outlined text and PNG/PDF signatures; `advanced.rs`: metadata, style and cache |
| Notation modes | 1–16-string tablature, standard notation, combined staves, numbered notation, one-line slash notation, 95 percussion articulations | `rendering.rs`: percussion; `advanced.rs`: numbered/slash without guitar strings |
| Rhythm | Longa, breve, whole through 256th, up to three dots, rests, nested tuplet ratios/brackets in tab and staff; custom meter groups with selectable units; explicit beam joins/breaks and secondary breaks; stem direction; explicit beam spans across barlines and systems | `advanced.rs`: nested onsets and bracket endpoints, custom grouping, cross-bar beams, long/short durations |
| Standard notation | Treble, bass, alto and tenor clefs with 8va/8vb/15ma/15mb variants; key changes/cancellations; explicit and key-aware pitch spelling; quarter-tone accidentals; ledger lines; normal, diamond, cross, slash, triangle and circled-cross heads; ghost parentheses | `advanced.rs`: every clef/head outline, remote key spelling, quarter-tone cancellation |
| Voice collisions | Opposing stems, displaced adjacent/unison noteheads where voices conflict, separate rest positions, accidental columns, simultaneous conflicting accidentals and subsequent cancellation | `advanced.rs`: conflicting simultaneous accidentals; `rendering.rs`: voice alignment |
| Connections | Ties; explicit note-indexed slurs; H/P and legato-slide slurs on both staves; directional slide-in/out; continuation across systems; curves routed around notation on their own staff | `advanced.rs`: routed spans, H/P/slide labels on both staves, invalid endpoints; `rendering.rs`: combined ties |
| Techniques | Natural/artificial/tapped/pinch/semi harmonics with written pitches; grace duration, dead grace, on/off-beat slash and bend/slide/slur transitions; prebends, holds, releases, bend arrows and tone labels, pitched staff bend targets, bend vibrato, tremolo picking, trills, ornaments, accents and fingerings for both hands | `import.rs`: harmonic/prebend pitches and grace details; `rendering.rs`: ornaments and guitar effects |
| Effect ranges | Grouped palm mute, let ring, rasgueado, ottava, crescendo/diminuendo and barre/text spans; explicit pedal/trill/vibrato ranges; fade-in/out/swell, wah, golpe, left-hand tap, beat timers; signed whammy curves when supplied | `advanced.rs`: grouped ranges across systems, effect symbols; `rendering.rs`: signed whammy geometry |
| Text and metadata | Measured text/lyric widths, song title/subtitle/artist/album/words/music/copyright/instructions, tuning/capo, beat text, lyrics, tempo changes and music-glyph dynamics; individual visibility controls for credits, tuning/capo, track names, chord diagrams, dynamics, lyrics, effects, bar numbers and repeat counts; hairpins share the dynamic band; repeated page titles, copyright footers and page numbers | `advanced.rs`: metadata/element visibility and running headers; `import.rs`: metadata and notation annotations |
| Chords | Variable string count, named diagrams, open/muted strings, barres, per-string fingerings; automatically expanded fret ranges for wide voicings | `advanced.rs`: wide diagrams/fingerings and invalid diagrams |
| Measure structure | Numbers, meter changes, repeat barlines/counts, alternate endings, final/double barlines, coda/segno and textual jumps, fermatas, free time/swing labels, single/double similes, explicit and automatic multi-measure rests | `advanced.rs`: rest condensation and address preservation; `import.rs`: structural import |
| Layout | Horizontal/vertical flow, explicit breaks, bars per system, dense-measure expansion, optional justification/strict width, aligned multi-track columns; `ScoreDocument` instrument/staff/master-bar hierarchy; staff measure maps; per-staff display, visibility, transposition, fingering, slur and spacing settings; brace/bracket groups and cross-staff beams/slurs; separate effect lanes and system expansion after routing | `rendering.rs`: wrapping, density, justification and score alignment; `advanced.rs`: staff hierarchy, mapped parts, cross-staff spans, settings and rest condensation |
| Interaction | Beat hit testing; click, shift-click and drag ranges by musical onset; interpolated/follow cursors; solo and multi-track selection, zoom and complete-system pagination; fit-to-page scaling; original addresses retained in condensed rests; revision-aware background layout with queued-request coalescing | `advanced.rs`: score pages/zoom/selection, background layout and condensed hit regions; `rendering.rs`: onset selection and pagination |
| Application | `music_gym` exposes all five notation modes, zoom, horizontal flow, rest condensation and follow playback; audio events and cursors share imported beat timing | `music_gym` tab-playback tests, including empty voices and explicit onsets |

## Guitar Pro boundary

`guitar_pro::convert_track` accepts already-parsed `guitarpro::Song`/`Track`
values and returns the renderer model plus warnings. The example loader delegates
GP3/4/5, GPX and GP input entirely to `guitarpro`.

The adapter maps the exposed rendering data: voice identity and beat onsets,
durations, written transposition and enharmonic hints, percussion, clefs, keys,
metadata/lyrics, repeats/navigation/fermatas, similes, free time, swing, line
breaks, double bars, chord voicings/barres/fingerings, both-hand fingerings,
harmonic/grace/bend details, slide distinctions, ornaments, rasgueado, fades,
ottava, strokes, tempo and wah changes. Its public optimized conversion is used
to access Guitar Pro's otherwise-private beam/stem/tuplet display hints; it does
not decode file bytes itself.

Unknown ornament, fingering and simile values, incomplete chord diagrams,
inconsistent source timestamps and excess lyrics produce explicit warnings.
Invalid model dimensions, pitches, durations, curves, diagrams, groups and span
endpoints return `RenderError`. Playback-only mixer/RSE parameters are outside
this renderer. Features such as explicit slurs or custom effect spans can also
be supplied directly through the notation model; the renderer cannot reconstruct
information a file parser has discarded.

The accepted GP6/7 whammy limitation remains: `guitarpro` 0.4.3 does not expose
the GPIF `<Whammy>` representation used by some files. Explicit tremolo-bar
curves do render and have local regression coverage. No unconditional warning
is emitted merely because a file is GP6/7.

## Local validation

The current library validation reports 62 tests (25 advanced, 5 hardening,
7 import, 23 rendering), clean Clippy with warnings denied, two focused application
integration tests, and successful application binary checks. These are local
regressions, not upstream conformance tests.

- `cargo test`: model, import, layout, interaction geometry and native egui tests.
- Property tests cover duration arithmetic, finite geometry, address retention,
  resource limits, structured errors, and background-worker lifecycle behavior.
- `cargo clippy --all-targets -- -D warnings`.
- Focused `music_gym` integration tests and application compilation.
- `examples/advanced.rs`: an SVG showcase with wide diagrams, multi-system
  connections, explicit beams, grace notes, bends and condensed rests.
- Local Guitar Pro files and generated SVG previews exercise the complete
  parser → adapter → layout → export path.

Useful upstream source entry points remain
[rendering](https://github.com/CoderLine/alphaTab/tree/0a3af85833133a0e450223ae286fb681dea8bdda/packages/alphatab/src/rendering),
[notation settings](https://github.com/CoderLine/alphaTab/blob/0a3af85833133a0e450223ae286fb681dea8bdda/packages/alphatab/src/NotationSettings.ts),
and [notation models](https://github.com/CoderLine/alphaTab/tree/0a3af85833133a0e450223ae286fb681dea8bdda/packages/alphatab/src/model).
