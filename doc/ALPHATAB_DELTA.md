# Differences from alphaTab

This is an independent Rust renderer, not a source port, drop-in replacement or
verified complete implementation of alphaTab. The implemented rendering inventory
is in [FEATURES.md](FEATURES.md); internal design is in
[ARCHITECTURE.md](ARCHITECTURE.md).

## Comparison baseline

The repository baseline is alphaTab commit
[`0a3af85833133a0e450223ae286fb681dea8bdda`](https://github.com/CoderLine/alphaTab/tree/0a3af85833133a0e450223ae286fb681dea8bdda).
Upstream descriptions below refer to its
[README](https://github.com/CoderLine/alphaTab/blob/0a3af85833133a0e450223ae286fb681dea8bdda/README.md),
[model](https://github.com/CoderLine/alphaTab/tree/0a3af85833133a0e450223ae286fb681dea8bdda/packages/alphatab/src/model),
and [rendering implementation](https://github.com/CoderLine/alphaTab/tree/0a3af85833133a0e450223ae286fb681dea8bdda/packages/alphatab/src/rendering).
This document is a capability and architecture comparison, not an exhaustive
symbol-by-symbol or image comparison. It does not track every later upstream change.

## Structure and integration

| Area | Original alphaTab | alphatab-rs |
| --- | --- | --- |
| Language/platform | TypeScript-based project with browser, .NET and Android integrations. | Rust crate with egui integration and SVG strings; no corresponding platform wrappers or JavaScript API. |
| Scope | Notation model, importers, rendering and MIDI/SoundFont playback. | Rendering model, adapter, engraving, export and interaction helpers. Audio belongs to the application. |
| Model | Score/Track/Staff/Bar/Voice/Beat/Note hierarchy, including multiple staves per instrument. | `ScoreDocument`/`Instrument`/`Staff`/`MasterBar` wraps the existing Track/Measure/voice/Beat/Note model. Staff maps align local measures to shared master bars. Public names and serialization differ. |
| Engraving | Dedicated renderer/glyph/layout classes and platform rendering infrastructure. | Synchronous preparation and layout producing flat glyph/line/text primitives and addressable beat bounds. |
| Files | Built-in importers including Guitar Pro, MusicXML and alphaTex. | Mandatory guitarpro dependency plus an adapter for already-parsed values; no local file decoder or textual notation parser. |
| Drawing | SVG and platform raster backends. | egui meshes, SVG, raster PNG and vector PDF from shared geometry. No canvas, Skia, GDI or Android backend. |
| Styling | Element style types and a broader settings surface. | Configurable background/selection/cursor plus independent glyph, line and text colors, per-staff engraving settings and common-element visibility. Upstream's complete style matrix is not API-compatible. |
| Lifecycle | Platform APIs handle rendering and interaction integration. | Caller owns selection/playback and revision numbers; `LayoutWorker` performs coalesced background engraving. No platform resize observer is included. |

The upstream multi-staff and style model can be inspected in
[Track.ts](https://github.com/CoderLine/alphaTab/blob/0a3af85833133a0e450223ae286fb681dea8bdda/packages/alphatab/src/model/Track.ts).
Public names and serialized models are not API-compatible.

## Rendering coverage and remaining differences

Both cover standard notation, tablature, percussion, multi-track presentation,
common rhythms, repeats, lyrics, chords and guitar techniques. The Rust renderer
also exposes numbered/slash modes, explicit span and beam controls, rest
condensation, hit testing, selection and follow cursors. These are local
capabilities; their existence does not establish equivalence for every upstream
variant, placement rule or setting. See the feature table for concrete behavior.

Known rendering differences remain:

- **Multi-staff instruments:** the Rust model now has instruments, staves,
  master bars, braces/brackets and explicit cross-staff beams/slurs. It remains
  structurally different from upstream and does not deserialize its model.
  Cross-staff spans currently have to remain within one rendered system.
- **Settings and styling:** `layout_score_tracks` supports independently
  configured display/visibility/engraving settings per staff. `RenderStyle`
  supplies class defaults and `set_primitive_color` styles any individual output
  element. There is no one-to-one naming or serialization mapping for upstream
  settings.
- **Display hints:** forced tuplet brackets now differ from automatic beamed
  tuplets. Import fidelity still depends on which hints `guitarpro` exposes.
- **Engraving:** beat annotations use measured semantic elements and
  bounds-driven stacks; system extents derive from the same measurements. Curves
  remain cubic vector primitives through all backends. Spacing, beam construction,
  accidental columns and curve routing still use Rust-native algorithms rather
  than alphaTab's class hierarchy. Dense combinations can still need visual review.
- **Pages:** strict pagination preserves complete systems and errors on an
  oversized system. `paginate_to_fit` can uniformly reduce the complete layout
  first, but there is no upstream-equivalent print/layout policy surface.
- **Score timeline:** `Staff::master_bar_map` aligns local measures and permits
  omitted master bars. A single local measure still cannot visually span several
  master bars without being split by the caller.
- **Fonts and appearance:** music glyphs use the bundled Bravura 1.482 outlines
  together with its standard font-specific SMuFL metadata, matching upstream's
  metadata-driven model for bounds, advance widths, stem anchors and engraving
  defaults. Beat annotations center their visible outline bounds because this
  crate paints paths directly; alphaTab's canvas text backend centers the font
  advance and can retain a small visible offset for asymmetric glyphs. alphaTab
  can load configurable SMuFL font sources; this crate fixes
  the matched Bravura pair at compile time. Ordinary SVG text can be converted
  to paths with `to_svg_outlined_text`; editable-text SVG still depends on font
  substitution.
- **Performance:** glyphs and completed layouts can be cached, painting is
  vertically culled, and `LayoutWorker` moves coalesced revisions off the UI
  thread. Engraving itself remains a full-layout operation rather than a partial
  system recomputation.

## File-import differences

The example/application loads GP3/4/5, GPX and GP through `guitarpro` 0.4.3.
Renderer support is broader than what any particular imported file can supply.
The adapter maps exposed source values and reports known inconsistencies; it
cannot reconstruct fields the parser drops. Import warning coverage is not a
complete audit of every source field.

GP6/7 GPIF `<Whammy>` data omitted by that parser is an accepted limitation.
Explicit tremolo-bar curves supplied through the Rust model still render.
Advanced spans can likewise be supplied directly when source data is unavailable.
No alternate parser or alphaTab parser has been embedded to fill these gaps.
There is no Guitar Pro writer or lossless file round-trip contract.

## Functionality outside the agreed rendering scope

- MusicXML/alphaTex and other non-Guitar-Pro input adapters are absent.
- alphaSynth, MIDI generation, SoundFont loading and integrated audio playback
  are absent from the crate. music_gym audio is a separate application feature.
- Repeat/navigation marks are drawn; they do not create a repeat-expanded
  playback timeline. Cursor helpers visualize caller-supplied timing.
- Upstream browser, .NET and Android controls, event APIs and packaging are absent.

These are differences from the complete alphaTab product even though they are
outside this project's rendering-only scope.

## Rust-specific facilities

The local integration provides Rust-owned notation values, `Result` errors,
structured import reports, native egui interaction and a reusable flat primitive
list. Beat addresses are preserved through rest condensation, scaling and
pagination; caller-owned playback can be layered over selection without a second
notation painting pass. These are integration choices, not claims that alphaTab
lacks analogous capabilities or that the Rust renderer exceeds its feature set.

## Verification and accepted limitations

Upstream visual verification and parser-missing GP6/7 whammy data were explicitly
waived. Neither waiver proves parity or removes other documented limitations.
Local tests cover selected behaviors and SVG/native rendering paths; they are not
an upstream conformance suite. The last completed results are recorded in
[FEATURES.md](FEATURES.md#local-validation). No tests were rerun for this docs update.
