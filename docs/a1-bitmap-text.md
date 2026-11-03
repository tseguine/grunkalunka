# Bounded text in the native A1 frame

The private `a1_note_scene/text.rs` helper measures and paints an original
fixed-cell bitmap font in the existing native A1 scene. This turns retained text
anchors into readable pixels for printable ASCII without adding dependencies or
selecting a general platform font stack.

The font covers U+0020–U+007E, including distinct lowercase letters. Glyphs occupy
five by seven pixels, with six-pixel horizontal and nine-pixel line advances and
a two-pixel content inset. Space has no ink but occupies a cell. Every other
Unicode scalar, including controls, produces a visible replacement box. Original
Unicode text remains intact in HTML and native `A1FRAME1` reports. This is not
Unicode shaping, grapheme segmentation, word wrapping or complete note typography.

The helper accepts at most 4,096 UTF-8 bytes per label. A shared retained glyph
plan determines measurement and painting. The adapter first obtains actual native
content widths, measures labels for those widths, raises text heights where
needed, and reflows before drawing or hit testing. Short title/row minimum heights
remain 24/18 pixels. Long labels wrap at fixed scalar cells and move subsequent
rows and their input targets together.

Ink is confined to each measured content rectangle and the existing RGB canvas
clips it to the viewport. The private adapter uses nonoverlapping normal-flow
rows without ancestor clipping. This does not change general layout, CSS, text
stacking or the library's paint implementation. The scene wire and frame-report
formats remain unchanged. Existing packages pick up the new text service inside
the native scene executable without an extra payload.

## Executable checks

`cargo test -p grunkalunka-layout --test a1_text --locked --offline` checks exact
glyph pixels, all printable cells, replacement boxes, scalar/space-preserving
wraps, viewport clipping, invalid geometry before paint and native reflow.

The source-service application suite also sends actual Markdown/WIT labels
through the native executable and checks ink pixels, preserved source text,
wrapped row coordinates and pointer selection against the reflowed geometry:
`node --test tests/applications/a1-service-interaction.test.mjs`.

These tests participate in the existing Cargo and A1 canonical selection. See
PR receipts for actual execution results and tested revisions. A1 still needs
broader text support, a persistent usable interaction, typed application views
and completed application acceptance; readable ASCII alone does not close it.
