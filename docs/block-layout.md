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
- Nonnegative integer CSS-pixel margins, borders and padding; no vertical
  margin collapse. Each side can optionally be a whole percentage of the
  containing **content width**, including vertical margins. Selected
  horizontal `auto` margins can center a fixed or width-constrained block.
- Explicit `BoxSizing::ContentBox` (default) or `BoxSizing::BorderBox`
  for caller-resolved fixed and percentage widths/heights. Auto sizes keep
  their existing natural sizing behavior.
- Optional caller-resolved minimum/maximum widths and heights (absolute
  integer pixels or integral percentages of the containing content dimension).
  Explicit dimensions are constrained before descendants lay out; auto
  height is constrained after natural child layout. An explicit minimum wins
  an incompatible maximum.
- Explicit integer-percentage content widths relative to the containing content
  width (e.g. 50% of 210 px = 105 px), and heights relative to a definite
  containing content height (the viewport for the root, an explicit height for
  descendants). Fractional CSS-pixel results fail rather than being rounded.
  Values above 100% may overflow.
- Display:none removes an entire subtree. Changing the display value and
  running layout again restores boxes without modifying document ownership.
- Explicit boxless `display: contents` flattens a wrapper's descendants into
  its containing block's normal flow without removing DOM-tree identity or
  changing children's inherited visibility.
- Resolved inherited visibility controls painting and input separately:
  hidden boxes remain in geometry and flow, while an explicitly visible
  descendant can override a hidden ancestor without reflow.
- Resolved inherited `PointerEvents::Auto/None` controls hit eligibility
  independently from geometry, visibility and caller targetability. An
  explicitly auto descendant may override an inherited none policy.
- Optional caller-resolved padding-box descendant clipping restricts paint
  and hit tests through nested ancestor clips without moving or hiding boxes.
- Positive post-flow `RelativeOffset { x, y }` translations move the visible
  box and descendants without consuming extra vertical normal-flow space,
  moving following siblings or changing auto-sized ancestors' natural height.
- Checked arithmetic and bounds: at most 4,096 nodes, depth 64 and
  1–16,384 CSS pixels per viewport dimension. Malformed parent indices
  and overflowing dimensions fail instead of wrapping or silently truncating.

## Positive relative visual offsets (normal-flow-only experiment)

`Style::relative_offset: RelativeOffset { x, y }` is an optional, already
resolved **nonnegative integer CSS-pixel** visual displacement. Zero is the
default. Every boxed node is first sized and positioned in its existing
normal flow (including horizontal auto-margin resolution), then translated
to the right by `x` and downward by `y`. The translation applies to the
node's margin, border, padding and content rectangles and to all visible
descendants. A descendant's own positive translation composes with its
ancestors' translated origin.

The *consumed flow height* remains the original margin-box height, and
the parent advances to the next sibling as though no relative translation
occurred. An auto-height parent also measures its normal-flow children,
not the visual displacement. Translated margin boxes nevertheless
contribute to the explicit document scroll extents; checked arithmetic
rejects overflow rather than wrapping. The RGB painter, input hit tester,
viewport-scrolled projection, ancestor padding clips and semantic text
anchors receive the same translated geometry. An active `display: contents`
wrapper has no box and ignores **its own** offset; its laid-out children
may each have an offset. A `display: none` subtree is skipped entirely.

The independent `crates/grunkalunka-layout/tests/relative_offsets.rs`
tests literal nested rectangles, sibling flow invariance, positive visual
overflow, scrolling, clipping, automatic horizontal margins, boxless
wrappers, pixel/hit agreement and checked overflow. The selected
[source-checked relative-offset bridge](browser-native-relative-bridge.md)
compares a positive `position: relative; left/top` configuration and a
live parent mutation with Chromium, Firefox and WebKit.

This is **not** general CSS positioned layout. It does not model negative
offsets; right/bottom inset precedence; subpixel values; `z-index` or
stacking contexts; new positioned containing blocks; or absolute, fixed
and sticky layout. Offsets are caller-resolved values, not CSS declarations
parsed or computed by the native engine.

## Containing-width-relative percentage margins

`Style::percentage_margins: PercentageMargins` has optional whole-percent
`top`, `right`, `bottom` and `left` fields. All default to `None`, which
preserves existing positive `Style::margin: Edges` pixel values. A selected
percentage overrides its corresponding fixed edge. **All four sides use the
immediate containing block's original CONTENT width**, not its border width,
the current block's used width, or the containing height. This intentionally
follows the selected horizontal writing-mode CSS margin percentage rule: for
example, `margin-top: 10%` under a 200px-wide content box is 20px even if
that parent is only 40px tall.

The resolution occurs before auto/fixed/percentage box-width calculation,
min/max bounds and post-flow relative offsets. A horizontally `auto` side
takes precedence over its supplied fixed and percentage values, even if
those ignored inputs would overflow or require fractional pixels.
Unselected fixed/percentage sides are still independently resolved and
checked. Top/bottom percentage margins contribute to vertical normal flow
and following siblings' y positions, while horizontal margins contribute
to border positions and margin rectangles. The native input/hit/paint
and scroll projections consume the same resulting geometry.

As elsewhere in this experimental engine, each used percentage must
produce an **exact nonnegative integer CSS-pixel result**; otherwise layout
returns `FractionalPercentageMargin { node }`. An exact oversized result
returns `Overflow`. Percentages above 100% are admitted when resolvable.
A hidden `display:none` subtree or boxless `display:contents` wrapper
ignores its own percentage declarations without resolving them; boxless
children still use the appropriate original containing content width.

The independent `crates/grunkalunka-layout/tests/percentage_margins.rs`
checks exact four-sided and nested geometry, vertical percentage margins
based on parent width, auto width with max bounds, horizontal auto
precedence, relative positioning, paint/hits, ignored wrapper declarations,
sibling flow and fail-closed arithmetic. The separate
[real-engine percent-margin bridge](browser-native-percentage-margin-bridge.md)
checks integral margin values, boxes and input hits through live CSS
mutations and viewport scrolling in Chromium, Firefox and WebKit.

This is **not** complete CSS margin layout. Negative margins, margin
collapsing, arbitrary writing modes, mixed calc expressions and fractional
subpixels remain outside this selected geometry model.

## Horizontal auto margins for normal-flow blocks

`Style::horizontal_auto_margins` admits
`HorizontalAutoMargins::{None, Left, Right, Both}`. The default `None`
preserves both explicit nonnegative `Style::margin.left/right` values.
An auto side overrides its corresponding numerical margin edge, without
changing vertical margins or box sizing. The used horizontal auto margins
resolve **after** width, `min_width`, `max_width` and padding/border
edges, against the original containing **content width**. A single auto
side consumes all nonnegative remaining space; two auto sides divide it
equally. A `width: auto` box fills first, so its auto margins generally
resolve to zero unless a width bound leaves free space.

The coordinate domain remains deliberately exact integer CSS pixels and
nonnegative. If two auto sides would need half an integer pixel,
`LayoutError::FractionalAutoMargin` rejects the layout; an overwide border
box gives its auto sides zero remaining width rather than generating
negative margins. Resolved auto margins participate in the emitted margin
rectangle and scroll extents, but are not part of the hit-test border
region. `display: none` suppresses them, while a `display: contents`
wrapper does not create margins of its own. Descendant percentage widths
still use the laid-out parent's content width. The independent
`tests/horizontal_auto_margins.rs` covers centered and one-sided boxes,
width limits, overconstraints, overflow, hidden/boxless wrappers and hits.

This is **not** complete CSS 2.1 width/margin equation solving: signed
overconstraint, direction-dependent surplus allocation, negative
margins, percentage margins and fractional subpixel rounding are not
modeled. The [source-checked three-engine auto-margin witness](browser-native-auto-margin-bridge.md)
compares selected integral cases against actual Chromium, Firefox and
WebKit CSS computed margins, rectangles and input hit targets.

## Boxless display:contents (headless normal-flow feature)

`Display::Contents` removes only the wrapper's own box. Its children are
laid out in the wrapper's containing block's flow at the current cursor,
with the containing content width and definite height passed through.
Nested boxless wrappers work identically, and both the document root and
ordinary descendants may be contents wrappers. The wrapper's own margins,
padding, border, width/height, min/max constraints, intrinsic height and
padding-box descendant clip are ignored because no principal box exists.
The next sibling follows the consumed heights of the flattened descendants.

`Frame::contents` marks **active** contents wrappers. The corresponding
`Frame::boxes` entry is `None`, which differs from the hidden subtree of
`display:none`: visible descendants under contents remain laid out,
painted and targetable. The RGB painter traverses through the boxless
wrapper without painting its own background/border; the hit tester does
the same without selecting the wrapper as a target. Both consumers
reject missing/inconsistent frame metadata and still reject children of
genuinely hidden parents. Inherited `visibility` continues across the
boxless wrapper, including explicit visible overrides.

The typed semantic-tree adapter admits only **text-free** contents
containers. Direct text or a measured text height on an active boxless
node is rejected as `SemanticError::BoxlessText`, because anonymous
inline/text-box construction is not implemented. Regular text-bearing
descendants keep their own paint anchors. This is not HTML parsing,
`display:contents` accessibility or event-bubbling semantics, CSS
pseudo-elements, replaced element behavior or a full formatting context.

The independently authored `tests/display_contents.rs` checks exact
flattened coordinates, nested contents, paint and hit order, visibility
inheritance, `display:none` isolation, a boxless root, semantic text
rejection and corrupt frame metadata.
A separate [source-checked display:contents bridge](browser-native-boxless-bridge.md)
compares the resulting child/sibling geometry and input hits against real
Chromium, Firefox and WebKit observations under a DOM style toggle.

## Inherited pointer-events eligibility for native input

`Style::pointer_events` supports `PointerEvents::Inherit` (default),
`PointerEvents::Auto` and `PointerEvents::None`. The root inherits
`Auto`. The layout pass computes `Frame::pointer_events` in parent-before-
child order, preserving ancestor `None` on descendants unless they
explicitly override it with `Auto`. This follows the selected HTML
`pointer-events` inheritance behavior in CSS UI, but does **not**
implement SVG-only values or general DOM pointer dispatch.

The headless hit tester requires **all three independent eligibility
conditions** before choosing a border-box target: caller-supplied
`HitNode::targetable`, resolved `Frame::visibility` and resolved
`Frame::pointer_events`. A parent with `None` is not a target, but its
`Auto` child remains eligible. `display:none` still removes all boxes;
`display:contents` wrappers can propagate inherited pointer eligibility
without having a hit box themselves. Existing ancestor padding-box clips
continue to restrict restored descendants.

This property does not change any layout box, scroll extent, RGB pixel,
semantic text anchor or bitmap/glyph content. Hit testing rejects missing
or mismatched pointer metadata instead of assuming `Auto`; painting
deliberately does not depend on that metadata. The independent
`tests/pointer_events.rs` checks exact targets, auto overrides,
non-targetable caller flags, root-none inheritance, nested contents,
visibility, `display:none`, clipping, viewport scrolling, pixel
invariance and malformed frame metadata.

This selected input eligibility does not model DOM event paths,
capturing/bubbling, `pointerenter`/`pointerleave`, pointer capture,
hover state, keyboard navigation or accessibility. A separate
[source-checked pointer-events witness](browser-native-pointer-events-bridge.md)
now compares a selected inherited-none/explicit-auto case with real
Chromium, Firefox and WebKit `elementFromPoint` observations. This remains
narrow reference evidence, not DOM event-dispatch conformance.

## Inherited visibility without reflow

`Style::visibility` admits `Visibility::Inherit` (the default),
`Visibility::Visible` and `Visibility::Hidden`. The root's inherited
value resolves to visible. A descendant inherits its parent's *resolved*
visibility unless explicitly overridden; unlike `display:none`, a
hidden ancestor retains geometry, occupies its normal-flow height and
still contributes to scroll extents. An explicit visible descendant
under a hidden ancestor keeps its own paint and input targetability.
An explicit visible descendant under `display:none` cannot resurrect
an absent box.

The `Frame::visibility` vector contains these resolved per-node Booleans
alongside the existing optional layout boxes. The RGB painter suppresses
background and border paint for hidden nodes but still visits all present
descendants, allowing an explicit visible override. Hit testing first
requires both caller targetability and resolved visibility. A hidden
topmost node permits fallthrough to the next visible target. Both consumers
reject a malformed visibility vector rather than guessing values.

The typed semantic adapter filters invisible text *paint anchors* while
retaining all present block geometry. This does not delete the source text
or model accessibility, selection, opacity, CSS animation, DOM mutation,
`visibility: collapse`, pointer-events inheritance or hit-test retargeting.
The independent `tests/visibility.rs` exercises exact layout equality,
nested inheritance and explicit overrides, RGB pixels, input hits,
`display:none` isolation, malformed metadata and semantic text anchors.
A separate [source-checked visibility bridge](browser-native-visibility-bridge.md)
compares the selected geometry, inherited computed visibility and DOM hit
targets with real Chromium, Firefox and WebKit evidence.

## Padding-box descendant clipping (native renderer)

`Style::descendant_clip` accepts `DescendantClip::None` (default)
or `DescendantClip::PaddingBox`. The latter clips **descendants'** own
painting and hit regions to the node's resolved padding rectangle.
The clipping node's own border/background remain unaffected by its
own rule, but are still subject to any earlier ancestor clips.
Nested clips intersect; an empty intersection never resets to unrestricted
painting. An explicitly visible child of a hidden ancestor is still
subject to their shared geometry clipping, independently of hit-target policy.

`Frame::clip_descendants` retains the explicit per-node choice after
layout. Both the optional RGB painter and hit tester consume that same
metadata and reject missing or mismatched entries. They account for
global viewport scrolling using the original document-space coordinates.
Clipping does **not** alter the layout boxes or normal-flow sibling positions.

This intentionally only clips **paint and hits**. The current document
`scroll_width`/`scroll_height` still measure the entire child overflow,
including visually clipped descendants, and external `Canvas::blit_mono`
calls remain caller-controlled; they do not automatically inherit a clip.
This is therefore **not a CSS `overflow: clip` or `overflow: hidden`
implementation** with all their scroll-container, clip-edge,
scroll-extent and text effects. It has no border radii, transforms, nested
scrollers or CSS parsing.

The independent `crates/grunkalunka-layout/tests/descendant_clip.rs`
verifies padding-edge clipping and paint/hit agreement, nested and disjoint
clips, visibility overrides, `display:none`, viewport scrolling,
geometry and extents preservation, and malformed clip metadata.
A separate [source-checked browser hit witness](browser-native-padding-clip-bridge.md)
compares one CSS `overflow: clip` reference shape with the native
padding-box clipping policy without claiming full CSS overflow handling.

## What is deliberately NOT implemented

This is not CSS 2.1 block formatting conformance. There is no HTML parser, CSS
tokenizer, selectors/cascade, text shaping, inline formatting, negative margins,
fractional percentage margins, collapsed vertical margins,
floats, general CSS positioned layout (negative relative offsets,
absolute/fixed/sticky, stacking contexts), fractional percent widths,
percentage heights under indefinite auto-height parents, scrollbars,
CSS paint/style processing, DOM event dispatch or native window. Input text heights are
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
No subpixel rounding, intrinsic min-/max-content sizing, overflow clipping,
layout containment, margin collapse or general CSS box-model conformance is claimed. The separate
[source-checked native border-box bridge](browser-native-box-sizing-bridge.md)
supplies selected real-engine geometry evidence, not a general CSS compliance claim.

## Optional minimum and maximum widths

`Style::min_width` and `Style::max_width` accept
`Option<WidthBound>`, where the bound is `Px(u32)` or
`Percent(u32)`. `None` means no constraint. There is deliberately no
`auto` width-bound variant, negative dimension, CSS `min-content`/
`max-content` intrinsic sizing or CSS declaration/cascade parser. The
existing `Style::default()` has neither constraint, preserving the prior
unconstrained `Length::Auto`, fixed and percentage width behavior.

Each bound is resolved **independently against the containing block's
original content width**. Fractional percentages are rejected with the
existing `FractionalPercentageWidth { node }` error, and unrepresentable
integer results fail with `Overflow`, even if another bound would have made
the first one irrelevant. Bounds are expressed in the same box-sizing
convention as the node's specified width: a `ContentBox` limit applies to
the content width, and a `BorderBox` limit includes the node's own border
and padding but excludes margins. A border-box bound smaller than its edge
sum leaves a zero-width content area without generating negative geometry.

The engine first calculates the tentative content width using the existing
sizing rules, then applies the maximum and finally the minimum. If
`min_width > max_width`, the minimum wins. The final content width feeds
child percentage widths, horizontal flow/scroll extents and the existing
checked arithmetic. These constraints affect *width only*; height limits
are an independent feature described below. No scrollbar or clipping
semantics are inferred from a maximum width.

The independent `crates/grunkalunka-layout/tests/width_bounds.rs`
module verifies auto/fixed sizing, nested percentage widths, min/max
precedence, border-box constraints and tiny overconstrained boxes,
fractional/overflow fail-closed behavior and hidden-subtree isolation.
These are native literal-coordinate tests. The separate
[source-checked width-bounds bridge](browser-width-bounds-bridge.md) compares
one selected min/max-width configuration with real browser measurements; it
does not establish broad CSS conformance.

## Optional minimum and maximum heights

`Style::min_height` and `Style::max_height` are optional
`HeightBound::{Px(u32),Percent(u32)}` values. They use the node's chosen
`ContentBox` or `BorderBox` convention, subtracting only local border and
padding for border-box sizing. They do not include margins and cannot
produce negative content dimensions. The default has no limits.

Each percentage height bound resolves independently against the
**definite containing content height**. As with the existing percentage
height declaration, a parent with auto height provides no definite
containing height, so the engine rejects percentage bounds with
`UnsupportedPercentageHeight { node }` instead of inventing browser auto
fallback semantics. Fractional CSS pixels fail with
`FractionalPercentageHeight { node }`, and excessive exact magnitudes
fail with `Overflow` even when the other constraint would win.

An explicitly fixed or resolved-percentage `height` is clamped by the
maximum, then the minimum (minimum wins contradictions) **before** laying
out descendants. Children with percentage heights therefore use the
parent's final definite content height. By contrast, `height: Auto`
first measures its descendants and caller-provided intrinsic height,
then clamps that natural result. This used height remains *indefinite*
for child percentage resolution: a minimum alone does not turn an auto
height into a definite height. A maximum does not clip descendants;
overflowing boxes continue to contribute to document scroll extents.

The independent `crates/grunkalunka-layout/tests/height_bounds.rs`
suite checks fixed/auto limits, post-child natural sizing, percentage
containing boxes, nested descendant heights, contradictory constraints,
border-box edges, fractional/overflow rejection, visible overflow and
hidden-subtree isolation. No CSS margin collapse, viewport fitting,
intrinsic text sizing, auto-percentage fallback or real-engine parity
is claimed by these native tests alone. The separate
[source-checked height-bounds witness](browser-height-bounds-bridge.md)
compares a reviewed definite percentage-bound case with real browser
geometry without widening that claim.
