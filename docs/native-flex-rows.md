# Native fixed-width rows

`grunkalunka-layout` provides a caller-resolved `Display::FlexRow` formatting
context for simple horizontal arrangements. It shares the existing block
geometry, semantic text anchors, RGB painter, clipping, hit tester and document
scroll projection. Application hosts and compiler interfaces do not change.

The selected profile corresponds to a horizontal row with nonflexing items,
optional greedy wrapping and selected safe alignment on both axes. It does not
implement general CSS flex layout, intrinsic width measurement, an HTML/CSS
parser or a cascade.

## Sizing and placement

| Input | Behavior |
| --- | --- |
| Row container width | Existing block sizing; an auto width fills its containing content width after edges and constraints. |
| Item width | Explicit `Length::Px` or exact-integer `Length::Percent`; percentages resolve against the row's full content width. |
| Item width bounds and edges | Existing min/max widths, content/border-box sizing, fixed or integral-percentage margins and padding apply before placement. |
| Item height | Existing intrinsic, explicit or definite-percentage height; no cross-axis stretching. |
| `Style::column_gap` | Nonnegative integer pixels between successive active items. Default zero; no leading or trailing gap. |
| `Style::justify_content` | `FlexJustify::Start` by default; `SafeEnd`, `SafeCenter`, `SpaceBetween`, `SpaceAround` and `SpaceEvenly` use each line's remaining content width. |
| `Style::align_items` | `FlexAlign::Start` by default; `SafeEnd` and `SafeCenter` align each item's outer margin box within its line without stretching. |
| `Style::align_self` | `None` uses the generated parent row's `align_items`; `Some(FlexAlign)` overrides it for this actual item without inheritance or stretching. |
| `Style::align_content` | `FlexAlignContent::Start` by default; `SafeEnd`, `SafeCenter` and `SpaceBetween` place natural lines within a wrapped row's used content height. No effect on nowrap rows. |
| `Style::flex_wrap` | `FlexWrap::NoWrap` by default; `Wrap` greedily collects lines against the row's content width. |
| `Style::row_gap` | Nonnegative integer pixels between actual lines. Default zero; unused for a single line. |
| Row auto height | Natural height starts from the tallest normal-flow item margin box per line plus intervening row gaps; the existing intrinsic-height minimum and explicit height bounds then apply. |
| Overflow | Items retain their widths; their boxes may overflow the container and contribute to document scroll extents. |

Items are placed by their outer margin boxes in source-tree order. A following
block starts after the row's own used margin-box height, rather than after the
sum of its item heights. Every row item retains its own formatting context:
a block item stacks its children vertically, while a nested `FlexRow` lays out
its children horizontally with its own wrapping and gap settings.

Width, margin and padding percentages use the **original full row content
width**, including top and bottom percentage edges. They do not use remaining
horizontal space or a preceding item's width. Both width bounds use that same
reference; the existing rule that an explicit minimum wins an incompatible
maximum remains unchanged. Fractional CSS-pixel results fail explicitly.

An item's percentage height still requires a definite containing content
height. The engine does not infer definiteness from a row's eventual auto height
or from its min/max height. Explicit item widths do not imply intrinsic flex
minimums: the profile admits caller-selected sizes without content-based
automatic minimum sizing.

`Style::relative_offset` displaces an item and its descendants visually after
normal-flow sizing. It does not change the horizontal slot consumed by that
item, the row's natural height, or any following sibling's flow position.
Positive visual overflow still contributes to document scroll extents.

## Line collection

With `FlexWrap::Wrap`, the next item joins the current line when its outer
width plus the intervening column gap fits the remaining content width.
Exact fits stay on the current line. Otherwise a nonempty line is finished,
and the new line starts at the row's content x coordinate below the preceding
line's tallest outer item plus `row_gap`.

The first item always starts a line, even when it is wider than the container;
there is no empty leading line and no shrinking to force a fit. A zero-width
item still counts as an item. It can join an exactly filled line when the
column gap is zero, while a positive gap can move it to the following line.
Zero-height lines remain actual lines, so a later line still consumes its
intervening row gap.

Only actual between-item and between-line gaps become coordinates. In
particular, line-fit comparison uses wider arithmetic before adding a column
gap: an enormous gap that causes a line break is discarded at that break,
not reported as a coordinate overflow. An actual overflowing coordinate or
row gap still returns `LayoutError::Overflow`. A single line ignores `row_gap`;
an empty row consumes neither kind of gap.

Each item's width, edges, bounds and height reference are resolved once before
line collection. Its prepared box is then positioned at the chosen slot and
its descendants are laid out there. Natural item heights flow back through
the existing iterative continuation; no repeated subtree layout or recursive
measurement is needed to choose a line.

## Main-axis alignment

This subset follows the selected [flexbox main-axis packing rules](https://drafts.csswg.org/css-flexbox-1/#justify-content-property)
and explicit [safe overflow alignment](https://drafts.csswg.org/css-align-3/#overflow-values).
The engine's exact-integer admission remains narrower than browser CSS geometry.

`Style::justify_content` chooses placement independently for each completed
line. It uses the row's full content width minus the normal-flow outer widths
of the items and the existing between-item column gaps. Alignment never changes
item widths, percentage references, greedy line collection or source order.

| Mode | Selected behavior |
| --- | --- |
| `FlexJustify::Start` | Keep the line at the content start. |
| `FlexJustify::SafeEnd` | Put all nonnegative remaining space before the line. |
| `FlexJustify::SafeCenter` | Put half the nonnegative remaining space before the line. An odd remainder is rejected. |
| `FlexJustify::SpaceBetween` | Add an equal share of remaining space to every existing between-item gap. With at most one item, use start placement. |
| `FlexJustify::SpaceAround` | Add half-size outer gaps and twice that share between items; one item centers. |
| `FlexJustify::SpaceEvenly` | Add equal shares to both outer gaps and every between-item gap; one item centers. |

Every selected distribution falls back to start when a line overflows; none
squeeze or replace declared gaps. Empty lines require no alignment calculation.
Zero-width and visibility-hidden items still participate. Around divides free
space by twice the item count for integral outer half-gaps; evenly divides it
by one more than the item count. Each completed wrapped line uses its own
item count and available width.

`LayoutError::FractionalFlexJustify { node }` identifies the row whose selected
center offset or distributed interval needs fractional CSS pixels. Overflow
fallback happens before that check, so an overflowing line does not fail merely
because the negative remainder is odd. Positive fractional values are rejected
without rounding.

The engine records per-item alignment offsets after each line is measured.
A final parent-before-child pass accumulates these offsets through the original
source tree, moves all four rectangles of every box once, and computes document
scroll extents from the projected geometry. Existing relative offsets are
already present in the measured boxes and are not applied a second time.
Nested rows and boxless ancestors therefore share the same final positions
with semantic anchors, the painter and hit testing, without subtree relayout.

## Cross-axis alignment

`Style::align_items` follows the selected [cross-size determination](https://drafts.csswg.org/css-flexbox-1/#cross-sizing)
and [cross-axis alignment](https://drafts.csswg.org/css-flexbox-1/#cross-alignment)
rules with explicit safe overflow handling. The alignment height depends on
`Style::flex_wrap`, not on how many lines happen to form:

| Row mode | Height used for item alignment |
| --- | --- |
| `FlexWrap::NoWrap` | The row's final used content height, including explicit sizing, min/max bounds and the caller's intrinsic-height minimum. |
| `FlexWrap::Wrap` | Each line's maximum normal-flow outer item height, even when there is only one actual line or the row's own used height is larger or smaller. |

Wrapped lines retain their natural heights and declared row gaps. Their
positions follow `align_content`, whose default keeps start packing.
For either mode, `FlexAlign::SafeEnd` places the available vertical space before
each item, while `SafeCenter` places half before it. `Start` preserves the
original top placement. All modes preserve item dimensions and row flow sizes.
An oversized item falls back to start independently of its smaller neighbors.

`LayoutError::FractionalFlexAlign { node }` identifies the row when any admitted
item's center offset is fractional. As on the main axis, safe overflow fallback
happens first. For example, a nowrap row of height 41 with items of outer heights
20 and 40 rejects half-pixel offsets. A wrap row with the same inputs has one
natural line of height 40 and admits offsets 10 and 0. A zero-height item still
requires its own integral offset; an empty row has no item-alignment obligation.

Cross-axis offsets join the same final projection as main-axis offsets. Each
item's normal-flow margin-box height is measured before alignment; positive
relative displacements do not enlarge the line or change following flow.
Reading a completed nowrap row's used height for alignment does not make an
auto height definite for percentage-height descendants. Existing percentage
admission and full-width references for percentage margins remain unchanged.

### Per-item overrides

`Style::align_self` selects the same three resolved modes for one actual flex
item. `None` represents the [CSS `align-self: auto` behavior](https://drafts.csswg.org/css-flexbox-1/#align-items-property):
use `align_items` on the item's generated parent row. `Some(FlexAlign::Start)`,
`Some(FlexAlign::SafeEnd)` and `Some(FlexAlign::SafeCenter)` override that default.
Resetting the field to `None` restores the current row default on the next layout.
This property is not inherited. A `Contents` wrapper's own override is ignored;
its generated descendants select their own override or the actual row's default.
The root and ordinary blocks that are not flex items ignore `align_self`.

A nested row's `align_self` controls its position as an item of its outer row;
its separate `align_items` still controls its own items. Overrides preserve the
existing used-height versus natural-line-height distinction, margin-box sizing,
safe overflow fallback, source order and final geometry projection. Only the
effective mode can require fractional-center rejection: an explicit `Start` or
`SafeEnd` does not inherit an unused row-center divisibility requirement. A hidden
item retains that requirement for its selected center mode; a display-none
subtree has no item obligation. Cross-axis auto margins, stretch and baseline
alignment remain outside this resolved profile.

## Cross-axis line packing

`Style::align_content` follows selected [flex-line packing](https://www.w3.org/TR/css-flexbox-1/#align-content-property)
rules with [explicit safe overflow alignment](https://www.w3.org/TR/css-align-3/#overflow-values).
It applies to `FlexWrap::Wrap`, including a row that collected only one actual
line. `NoWrap`, ordinary blocks and boxless wrappers ignore it. The property is
not inherited. This caller-resolved profile defaults to start packing; it does
not implement CSS's initial `stretch` value.

The available space is the finished row's content height minus its natural
line heights and actual intervening `row_gap` values. Explicit height,
min/max constraints and the caller's intrinsic-height minimum affect that used
height. Relative offsets, item alignment and visual overflow do not change the
natural line measurements or the following block's flow position.

| Mode | Selected behavior |
| --- | --- |
| `FlexAlignContent::Start` | Retain start packing and declared row gaps. |
| `FlexAlignContent::SafeEnd` | Put nonnegative remaining space before the first line. |
| `FlexAlignContent::SafeCenter` | Put half the nonnegative remaining space before the first line. |
| `FlexAlignContent::SpaceBetween` | Add an equal share to each between-line gap; a single line stays at the start. |
| `FlexAlignContent::SpaceAround` | Give each line a half-size outer gap on both sides; interior gaps receive two shares. A single line centers. |
| `FlexAlignContent::SpaceEvenly` | Divide remaining space equally among the two outer gaps and every between-line gap. A single line centers. |

All selected modes use start packing when the natural line group overflows.
`LayoutError::FractionalFlexAlignContent { node }` reports a positive center
offset or distributed interval that is not an integer. Around uses twice
the line count as its denominator to retain integral outer half-gaps;
evenly uses one more than the line count. Safe fallback occurs before this
check. Zero-height lines count in the distribution; an empty row has no
line-placement obligation. A single line ignores `row_gap`.

Packing changes neither line sizes nor item sizes. Each item's `align_items`
or `align_self` offset still uses its unchanged natural line height. The
implementation stores line-end indices during the existing iterative layout,
adds each line's shift to its items' offsets after sizing, and uses the existing
single final projection for descendants, paint, hit testing and text anchors.
There is no subtree relayout. Used height does not become a new definite
percentage-height reference merely because line packing reads it.

## Boxless and hidden nodes

Active `Display::Contents` wrappers are flattened into the row's item sequence.
Their own dimensions, margins, offsets and gaps are ignored. The implementation
preserves depth-first tree order even when the flat input is only
parent-before-child, and retains original node indices and parent relationships
for inherited visibility, pointer eligibility, paint order and hit testing.
Nested rows inside these wrappers remain single items of the enclosing row.

`Display::None` prunes an entire subtree and contributes no slot or gap.
`Visibility::Hidden` retains its slot and sizing obligations; descendants may
explicitly restore visibility and input eligibility under the existing rules.
A zero-width or zero-height item is still an item and participates in gaps.
Empty rows and rows with only one active item never consume a column gap.

## Paint and input order

The selected non-positioned profile follows the [flex-item painting rule](https://drafts.csswg.org/css-flexbox-1/#painting):
actual flex items participate where inline blocks would paint. The native engine
resolves `Frame::paint_roles` as `PaintRole::Ordinary` or `PaintRole::FlexItem`.
A generated box is an item when its nearest generated parent is a `FlexRow`;
boxless `Contents` ancestors pass that relationship through. The document root,
contents wrappers and display-none nodes are always ordinary. A nested row can
itself be an item while also making its own generated children items.

Painting uses bounded root and item scopes. Within each scope, the root and
ordinary descendants paint in depth-first source-tree order. Actual item
subtrees are deferred, then each deferred item scope completes in source order
before the next one begins. This lets an overflowing item cover a later ordinary
block outside its row, while a later sibling item still covers an earlier item's
nested row descendants. Ordinary descendants inside an item paint above that
item's own background. Flat input need only be parent-before-child; numeric node
order does not replace source-tree traversal.

The painter consumes one shared iterative plan forward, and the hit tester
consumes it in reverse. Each entry retains the intersections of its original
ancestors' padding clips, including empty intersections, even when its scope is
deferred. A node's own descendant clip does not clip its own border. Document
scrolling applies to the same final geometry and clip coordinates in both
consumers. Scope order changes neither layout rectangles nor scroll extents.

Visibility, pointer eligibility and caller targetability remain per-node filters;
they do not change scope membership. Hidden items can still contain explicitly
visible descendants. Transparent paint does not remove a hit target, and an
untargetable item does not suppress an eligible descendant. `Display::None`
continues to prune an entire subtree. `Style::relative_offset` remains a geometry
translation and does not create CSS positioned-element paint behavior.

This is not general CSS stacking: there is no `z-index`, `order`, positioned
out-of-scope descendant handling, opacity grouping or arbitrary stacking context.
It covers the resolved non-positioned block/row profile described here. Consumers
reject missing role entries and impossible item roles on the root, a contents
wrapper or an absent box. Hit testing validates every box and all role metadata
before selecting a target, including for an off-viewport query.

## Explicit admission limits

`LayoutError::UnsupportedFlexItemWidth { node }` rejects an active row item whose
width is `Auto`, even if it has a minimum or maximum width. Supporting intrinsic
flex bases and automatic minimum sizing needs a separate measurement contract.

`LayoutError::UnsupportedFlexItemAutoMargins { node }` rejects horizontal auto
margins on row items. Ordinary block auto margins and auto margins on a row
container in a block context keep their existing behavior. Boxless wrappers and
display-none subtrees have no row-item sizing obligations.

Flexible growth/shrinkage, stretching, baseline alignment, other line and
main-axis distributions, reverse directions,
item reordering and percentage gaps are outside this profile. The public enum names the
specific resolved row profile instead of implying that arbitrary `display:flex`
declarations are supported. Container row settings are ignored by other display
modes; `align_self` applies to generated flex items regardless of their own
block or row display mode.

## Verification and cost boundaries

`crates/grunkalunka-layout/tests/flex_row.rs` specifies literal geometry for
outer sizes, percentages, bounds, box sizing, boxless tree order, nested
formatting contexts and zero-sized items. It separately checks selected pixels,
topmost hits, document-scroll projection, padding clips, inherited policies and
semantic text anchors. Browser captures do not supply these expectations.
`tests/flex_wrap.rs` separately checks exact fits, line maxima, zero-width and
oversized items, nested wrapping, hidden/boxless nodes, toggling and resizing,
semantic reflow, scrolled clipping and discarded-versus-actual gap overflow.
`tests/flex_justify.rs` covers each main-axis mode with literal margin, border,
padding and content rectangles, per-line distribution, integral percentages,
safe overflow and fractional rejection, nested/boxless inheritance, shared
paint/input/semantic behavior and final projection overflow.
`tests/flex_align.rs` checks used versus natural line heights, fixed/auto/bounded
nowrap sizing, the one-line wrap distinction, per-item safe overflow, fractional
and zero-height cases, nested axes and percentage references, hidden policies,
semantic reflow, vertical scrolled clipping and projected-coordinate limits.
`tests/flex_align_self.rs` checks mixed overrides and auto reset, effective-mode
fractional admission, safe overflow, wrapped versus nowrap placement, boxless and
nested-row defaults, ordinary descendants, semantic anchors, paint/scrolled hits
and the same native-stack/node/depth/coordinate bounds.
`tests/flex_align_content.rs` checks whole-line placement and the around/evenly
outer-gap distributions (including single-line, overflow and fractional cases),
with all box edges,
existing gaps and item overrides; single-line wrap versus nowrap, safe overflow,
fractional and zero-height distributions, height bounds and percentages,
nested/boxless order, hidden reflow, text anchors, scrolled paint and padding
clips, and admitted/just-past node/depth limits on a 2 MiB native stack.
`tests/flex_paint_order.rs` asserts literal pixels and topmost targets for an
item overflowing onto a following block, nested scope atomicity, ordinary
subtrees, boxless and hidden nodes, transparent and untargetable boxes, inherited
and empty clips, own borders, document scrolling and relative offsets.
`tests/flex_paint_metadata.rs` checks generated-parent role resolution and display
changes, malformed roles and late geometry before hit selection, and admitted
node/depth boundaries on a 2 MiB native stack.

Both row-item discovery and nested layout use heap-owned work lists. The final
alignment projection visits every input node once and uses the already-bounded
parent-before-child order. Paint planning also uses heap-owned work lists; each
node is visited a bounded number of times, with linear time and storage. The
existing 4,096-node and depth-64 bounds remain in force, with just-past-limit
errors and admitted nested rows exercised on a 2 MiB native stack. All dimension,
cursor and actual gap arithmetic is checked. A row never shrinks, truncates or
silently rounds a value to make the operation succeed.

Replay with `cargo test -p grunkalunka-layout --locked --offline`. Independent
browser comparison evidence, when available, remains distinct from these native
tests and does not establish general flexbox conformance.
