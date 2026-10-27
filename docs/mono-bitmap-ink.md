# Caller-supplied monochrome bitmap ink

The optional headless RGB painter can draw a caller-rasterized **1-bit mask** into
an already allocated Canvas. It does not choose fonts, lay out text, decode
UTF-8, shape glyphs, perform line wrapping, or paint a browser UI. In particular,
this is not a built-in font and a shaped Unicode string is not an accepted input.

## Binary contract

Canvas::blit_mono(x, y, width, height, bytes, ink) accepts nonnegative
integer CSS-pixel coordinates and positive mask dimensions up to MAX_VIEWPORT.
Each mask row begins at the next byte boundary and occupies
ceil(width / 8) bytes. The most-significant bit of the first byte is column 0,
the next bit is column 1, and so on. Unused **low** bits in the final byte of a
row do not belong to the mask and are ignored.

A one bit overwrites the corresponding RGB pixel with opaque ink; a zero bit
leaves the previously painted color unchanged. Blits are clipped against the
right and bottom of the Canvas. Masks completely beyond those edges are valid
no-ops when otherwise well formed; negative origins are not admitted.

The API validates all inputs **before** modifying pixels. Zero or oversized
dimensions and incorrect byte lengths return InvalidBitmap. More than
4,000,000 mask pixels return TooManyPixels, and addition/multiplication overflow
returns Overflow. A rejected mask leaves the original canvas unchanged, even
when the requested origin is completely outside the viewport. There are no
font/glyph-specific authority grants or filesystem accesses.

## Reproduction

After building the locked Rust workspace:

    cargo test -p grunkalunka-layout --locked --offline
    cargo run -q -p grunkalunka-layout --example bitmap_ink --locked --offline > ink.ppm

The original 16x16 P6 example draws an 8x8 X bitmap over a solid grey canvas.
It is a hand-supplied icon pattern, not a font glyph.

Independent Rust integration tests verify bit order across multiple bytes,
ignored row padding, transparent zeros, opaque ink layering, clipping, fully
offscreen behavior, exact PPM pixels, invalid mask lengths and dimensions,
resource limits, integer overflow and no partial writes on failure.

## Possible follow-on

A separate font/glyph service might provide bitmap masks together with
advance widths, bearings, ascent/descent and a line-breaking policy to the
caller. Do not infer those from bitmap dimensions or embed a single font engine
into the layout core. Any such service needs its own bounded measurement and
provenance contract, appropriate licensing, and evidence that at least two
consumers can use it independently of a browser shell.

This is a narrow implementation building block, not Unicode shaping, CSS Text,
SVG, Web Platform Test coverage, browser compatibility or A1 acceptance.


## Inert masks at semantic text anchors

The additional headless `semantic_ink_ppm` example composes the existing
`layout_semantic` text anchors, RGB background painter and caller-supplied
`blit_mono` masks. It paints original, hand-defined 3x5 pixel patterns
resembling H and I, plus small row markers, into a 320x120 PPM:

    cargo run -q -p grunkalunka-layout --example semantic_ink_ppm --locked --offline > anchors.ppm
    cargo test -p grunkalunka-layout --locked --offline

The standalone `tests/semantic_bitmap_anchors.rs` supplies manual expected
coordinates for opaque strokes, transparent mask bits, both list rows,
the paragraph and untouched viewport pixels. It hides the list and verifies
that the paragraph's ink moves with its new content rectangle, the removed
list ink disappears, and the PPM output changes.

**This is not automatically rendered text.** The caller chooses masks and
an explicit five-pixel advance independent of the semantic string. The second
test deliberately replaces text with inert markup-like characters while
leaving the supplied masks unchanged, documenting the missing font/shaping
boundary rather than pretending to implement it. There is no implicit
glyph lookup, font metric inference, CSS Text conformance or DOM scripting.
