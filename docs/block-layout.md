# Headless block-box layout experiment

This is an implementation-side browser experiment, separate from the Playwright
reference oracle suite. The dependency-free Rust crate grunkalunka-layout takes
a caller-built rooted tree and explicitly resolved CSS-pixel styles. It does
not access browser globals, files, font services, WIT capabilities or the app
workstream. The output is the content, padding, border and margin rectangles
of visible blocks, plus scroll extents.

## Supported now

- Normal-flow vertical block stacking in insertion order, one document root.
- Auto widths fill the available content width after edges; explicitly fixed
  content widths may overflow. Auto heights contain laid-out children and
  an optional caller-supplied intrinsic content height.
- Nonnegative integer CSS-pixel margins, borders and padding; no margin collapse.
- Explicit `BoxSizing::ContentBox` (default) or `BoxSizing::BorderBox`
  for caller-resolved fixed and percentage widths/heights. Auto sizes keep
  their existing natural sizing behavior.
- Explicit integer-percentage content widths relative to the containing content
  width (e.g. 50% of 210 px = 105 px), and heights relative to a definite
  containing content height (the viewport for the root, an explicit height for
  descendants). Fractional CSS-pixel results fail rather than being rounded.
  Values above 100% may overflow.
- Display:none removes an entire subtree. Changing the display value and
  running layout again restores boxes without modifying document ownership.
- Checked arithmetic and bounds: at most 4,096 nodes, depth 64 and
  1–16,384 CSS pixels per viewport dimension. Malformed parent indices
  and overflowing dimensions fail instead of wrapping or silently truncating.

## What is deliberately NOT implemented

This is not CSS 2.1 block formatting conformance. There is no HTML parser, CSS
tokenizer, selectors/cascade, text shaping, inline formatting, negative or auto
margins, collapsed margins, floats, positioning, fractional percent widths,
percentage heights under indefinite auto-height parents, scrollbars,
CSS painting, hit testing, event loop or native window. Input text heights are
provided by a separate caller, not guessed. All coordinates are unsigned
integers. This is a small, experimental implementation component, NOT an
A1 application or a commitment to the final renderer architecture.

## Reproduction

After installing the repository Rust toolchain and fetching locked dependencies:

```sh
cargo test -p grunkalunka-layout --locked --offline
cargo run -p grunkalunka-layout --example headless_outline --locked --offline
```

The tests exercise independent literal coordinates for an outline-like tree,
display toggling and sibling reflow, overflow beyond fixed parent height,
zero-width overconstraint, malformed trees and integer overflow. The example
prints rectangles; it does not paint pixels.

## Next independently verifiable unit

An explicit, separately owned adapter can parse a very small known semantic
HTML/CSS subset (for example main/h1/ol/li/p, display and fixed dimensions),
construct the tree, and pass in measured text heights from an explicit service.
Parser choice, glyph metrics and host presentation remain open. As required by
the browser roadmap, the crate is independent of a browser shell and can be
exercised by a headless caller. This step does not authorize editing the
compiler, Markdown/WIT code or the unselected application pilot.


## Implemented minimal semantic-tree projection

`crates/grunkalunka-layout/src/semantic.rs` accepts an **already structured**
parent-before-child tree of `Body`, `Main`, `Heading(1..6)`, `OrderedList`,
`ListItem`, `Paragraph` and `Div` nodes. It maps them to the existing block
geometry engine using explicitly documented toy defaults, or accepts a full
**caller-resolved** style override. A hidden flag hides a subtree regardless of
its supplied box style. Neither CSS selectors nor the HTML tokenizer are present.

A text-bearing node must be a leaf and supply a nonzero **measured text height**;
otherwise layout fails explicitly. Text remains an uninterpreted UTF-8 string:
angle brackets and entity-like input are *data*, never parsed/executed. Returned
`TextBox` records associate those borrowed strings with their content rectangles
for a future separate shaping/painting consumer. The module does not draw glyphs,
provide text width, or perform line breaking. It caps total UTF-8 text bytes at
256 KiB, alongside the block engine's node/depth/viewport/overflow checks.

```sh
cargo test -p grunkalunka-layout --locked --offline
cargo run -p grunkalunka-layout --example semantic_outline --locked --offline
```

The example projects an outline-shaped semantic tree, hides its ordered list,
then demonstrates paragraph reflow with no HTML parser, browser shell, WASI or
host capabilities. This is **not HTML-to-layout** yet: a future parser/sink must
produce this typed tree, and a text service must supply metrics. We reviewed
`html5ever` as a future HTML5 parser candidate (MIT/Apache-2.0, TreeSink API),
but deliberately added no new parser/dependency or public DOM ABI in this
separately reviewable step. See the browser roadmap for the reusable component
constraint and source-licensed dependency review requirements.

## Optional headless pixel diagnostics

The separate [RGB paint-surface experiment](block-paint.md) can draw solid
backgrounds and border strips from the block geometry into a bounded PPM
image. This does not add text/glyph rendering, CSS painting rules, a native
window or scripting to the block layout or semantic projection modules.

## Whole-percentage widths (small native geometry experiment)

The `Length::Percent(n)` representation denotes a **whole percentage of
the containing block's content width**. `Percent(50)` in a 210px containing
content width computes a 105px content width; edges are applied independently.
The native block engine performs checked `u64` arithmetic and rejects a
fractional CSS-pixel result with `FractionalPercentageWidth { node }`,
rather than inventing subpixel rounding that its integer `Rect` cannot
represent. Excessive percentages causing unrepresentable geometry return
`Overflow`. The same integer-only scaling can now apply to heights
whose containing content box is definite; an auto-height parent remains
`UnsupportedPercentageHeight`, and an inexact height is reported as
`FractionalPercentageHeight`. Neither dimension rounds to CSS subpixels.

`crates/grunkalunka-layout/tests/percentage_width.rs` includes separate
goldens for parent padding/borders, nested percentages and a following
sibling, over-100% widths and scroll extents, fractional/overflow failures,
and display:none isolation. The 210px/50%/105px/119px example deliberately
aligns with the known mathematical part of the independent baseline browser
fixture; it is not derived from browser output or evidence of general
CSS sizing conformance.

## Definite percentage heights (small native geometry experiment)

A root block's percentage content height uses the explicit viewport height.
Descendant percentage heights use their parent's **resolved, definite content
height**, from `height: Px(n)` or recursively resolved `Percent(n)`. A parent
with `height: Auto` remains indefinite for this experiment even if its own
containing block has a height: unlike a full browser layout, the engine
deliberately rejects that unresolved case rather than substituting an
unspecified CSS auto-height behavior.

Resolution occurs before laying out children so nested percentage heights have
an explicit numeric containing height. Fractions fail with
`FractionalPercentageHeight { node }`, and excessive magnitudes return
`Overflow`. Fixed/percentage parent heights may be exceeded by children:
normal sibling stacking and scroll extents still use laid-out child boxes,
without clipping or inventing an implicit scrollbar. `display:none` still
skips unsupported children and descendants.

The independent `crates/grunkalunka-layout/tests/percentage_height.rs`
checks viewport/root and nested definite height resolution, padding/border
exclusion, sibling positions, over-100% overflow, invalid resolution, exact
integer boundaries and hidden subtree isolation. No HTML/CSS parsing, margin
collapse, subpixel rounding, browser-engine comparison or broad CSS height
conformance is claimed.

## Explicit content-box and border-box sizing

The caller supplies `Style::box_sizing` as either
`BoxSizing::ContentBox` (the unchanged default) or
`BoxSizing::BorderBox`. This is **resolved layout input**, not CSS selector,
cascade or `box-sizing` declaration parsing.

For a non-auto fixed or integral percentage dimension, content-box sizing
treats the specified value as the content length. Border-box sizing instead
subtracts the corresponding **border and padding** edge sums from the
specified outer length; margins are never included. If the requested border
box is smaller than its own border/padding, its content length saturates at
zero and the *used* border box still contains those edges. Negative content
sizes or wrapped integer arithmetic are not introduced.

An integral percentage width is resolved against the containing **content
width** before subtracting the local border/padding. An integral percentage
height likewise uses the already **definite containing content height**,
allowing nested border-box percentages without importing the containing
border/padding. Indefinite height parents, fractional CSS-pixel percentage
results, checked overflow, normal-flow sibling placement, fixed-height
overflow and `display:none` retain their prior behavior. `Length::Auto`
ignores the sizing choice and follows the existing auto width/height rules.

`crates/grunkalunka-layout/tests/box_sizing.rs` checks exact independent
content/padding/border/margin coordinates, nested percentage dimensions and
sibling flow, overconstrained border boxes, whole-viewport percentages,
fractional-input rejections, default behavior, and hidden descendants.
The existing Rust layout tests remain unchanged to guard content-box output.
No subpixel rounding, min/max sizing, overflow clipping, layout containment,
margin collapse or general CSS box-model conformance is claimed. A separate
real-engine source-hashed comparison would be needed for selected browser
parity evidence.
