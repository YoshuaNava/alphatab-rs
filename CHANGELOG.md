# Changelog

All notable changes to alphatab-rs are recorded here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and published versions
follow [Semantic Versioning](https://semver.org/).

## [Unreleased]

### Added

- MPL-2.0 licensing and public repository metadata.
- Native egui, SVG, outlined SVG, PNG, and vector PDF rendering.
- Guitar Pro model conversion through the `guitarpro` crate.
- Standard, tablature, combined, numbered, slash, and percussion notation.
- Multi-track and multi-staff layout, interaction, pagination, and background layout.
- Explicit model validation, resource limits, and structured error categories.
- Official Bravura SMuFL metadata for glyph bounds, advance widths, stem
  attachment, and font-specific engraving defaults.
- Correct system ownership for staff, tablature, and symbol primitives emitted
  before a cross-system tie continuation, preventing detached or displaced rows.
- Use named `smufl::Glyph` identities throughout engraving and cache every
  outline with its matched metadata, removing the duplicated reverse-codepoint
  table and ad hoc dynamic centering.
- Center dynamics and pedal marks by their visible Bravura bounds, keeping
  asymmetric outlines on the beat axis when painting paths directly.
- Measure semantic annotation elements before painting and allocate their
  dynamics, lyrics, techniques, pedal marks, diagrams and effects in
  bounds-driven stacks; derive system extents from the same measurements.
- Preserve ties, slurs and bends as cubic Bézier primitives through egui, SVG,
  scaling, pagination and collision calculations instead of flattening them
  into fixed line segments during engraving.
- Place primary/grace/rhythm noteheads, rests, accidentals, ghost parentheses,
  ornaments and articulations from exact Bravura bounds; use SMuFL stem anchors
  instead of approximate origin offsets.

### Known limitations

- See [doc/ALPHATAB_DELTA.md](doc/ALPHATAB_DELTA.md) for deliberate exclusions and
  differences from upstream alphaTab.

### Changed

- `percussion::resolve` now returns named `MusicGlyph` values instead of raw
  Unicode integers. `MusicGlyph` is re-exported by the crate.

## Release policy

Public fields in the 1.x model are stable. Breaking model or behavior changes
require a new major version. Additive methods, new enum-free helper types, and
bug fixes may be released in minor or patch versions as SemVer permits.
