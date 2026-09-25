# How rendering works

`alphatab-rs` renders in two separate phases: it first calculates geometry,
then gives that geometry to egui for painting and interaction.

```text
Track
  ↓ engrave(track, options)
Scene
  ├── private drawing commands
  └── public beat rectangles
  ↓ EguiInteraction
egui painter + mouse selection + playback cursor
```

This separation is intentional. Layout understands musical time and tablature;
painting understands egui coordinates and colours. Neither phase needs to do
the other's work.

## 1. Input: a track

The renderer receives a [`Track`](../src/track.rs), which follows this hierarchy:

```text
Track → Bar → Voice → Beat → Note
```

Bars are written in order. Voices are parallel rhythmic lines inside a bar.
Beats are sequential positions within a voice, and notes inside one beat are
simultaneous—for example, a guitar chord.

## 2. Layout: `engrave`

Call layout with a score and a small set of display options:

```rust
let scene = alphatab_rs::engrave(
    &track,
    alphatab_rs::SceneOptions {
        width: 760.0,
        display: alphatab_rs::DisplayMode::Both,
    },
)?;
```

`engrave` always lays out vertically. Bars fill a row until the next bar would
exceed `width`; it then starts a new row.

```text
| bar 1 | bar 2 | bar 3 |
| bar 4 | bar 5 | ...   |
```

For every bar, layout performs the following work:

1. Draw tablature string lines, staff lines, or both, based on `DisplayMode`.
2. Convert each beat's musical onset and duration into horizontal coordinates.
3. Add a public `BeatGeometry` entry for the beat.
4. Add private commands for fret labels, staff noteheads, stems, and bar lines.

### Time becomes an x-coordinate

For a beat inside a bar, layout uses the beat's time relative to the bar:

```text
x = bar_left + (beat_start / bar_duration) × bar_width
```

The same calculation with `beat_start + beat_duration` produces its right
edge. For example, in 4/4, a beat at quarter-note time `2.0` starts halfway
through the bar.

### Tab and staff output

For tablature, the renderer draws a text label on the note's string:

```text
Fret::Number(7) → "7"
Fret::Dead      → "x"
Fret::Tied(7)   → "7"
```

For staff display, an imported note's MIDI pitch determines a simple vertical
position. The renderer draws a filled notehead and a stem. It deliberately does
not yet implement accidentals, ledger lines, key signatures, beaming, or
collision avoidance.

## 3. Scene: drawing commands and interaction geometry

The output is a `Scene`:

```text
Scene
├── width and height
├── beats: Vec<BeatGeometry>
└── private Draw commands
    ├── Line
    ├── Text
    └── Note
```

The private drawing commands are an implementation detail. Applications use
the scene's dimensions and `BeatGeometry`, but do not need to understand how a
staff line or notehead is represented.

Each `BeatGeometry` has a stable musical address:

```rust
BeatAddress { bar, voice, beat }
```

It also has a hit-test rectangle and a cursor rectangle. These remain tied to
the musical beat even when the scene is scaled.

## 4. Egui painting and interaction

Use `EguiInteraction` to paint a completed scene:

```rust
let mut selection = None;
let interaction = alphatab_rs::EguiInteraction::create_for_scene(&scene)
    .handle(ui, &active_beats, &mut selection);
```

`handle` allocates egui space for the scene, paints all commands, and converts
pointer coordinates back to scene coordinates. It uses `BeatGeometry` to find the
beat under the pointer. A click starts a selection; Shift-click extends it.

The renderer uses egui's active text colour, so notation remains visible in
both light and dark themes.

## 5. Playback cursor

For playback, the application identifies the active beat with `BeatAddress`
and supplies progress through that beat from `0.0` to `1.0`:

```rust
alphatab_rs::EguiInteraction::create_for_scene(&scene)
    .paint_playback_cursor(ui, &interaction.response, address, fraction, true);
```

The cursor is a blue vertical line interpolated across the beat's cursor
rectangle. Passing `true` as the last argument scrolls the active beat into
view.

```text
| beat starts                 beat ends |
|              cursor →                 |
```

## Why this design is useful

The design keeps responsibilities narrow:

```text
track.rs       Music data only
guitar_pro.rs  Guitar Pro → Track conversion
scene.rs       Track → backend-neutral Scene layout
render.rs      Scene → egui painting and interaction
lib.rs         Public API facade and shared error type
```

This makes it possible to test timing and geometry without opening a window,
and to change egui painting without changing how musical time is calculated.
