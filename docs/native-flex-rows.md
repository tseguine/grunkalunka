# Native fixed-width rows

`grunkalunka-layout` provides a caller-resolved `Display::FlexRow` formatting
context for simple horizontal arrangements. It shares the existing block
geometry, semantic text anchors, RGB painter, clipping, hit tester and document
scroll projection. Application hosts and compiler interfaces do not change.

The selected profile corresponds to a horizontal row with nonflexing items,
optional greedy wrapping and start alignment. It does not implement general CSS flex
layout, intrinsic width measurement, an HTML/CSS parser or a cascade.

## Sizing and placement

| Input | Behavior |
| --- | --- |
| Row container width | Existing block sizing; an auto width fills its containing content width after edges and constraints. |
| Item width | Explicit `Length::Px` or exact-integer `Length::Percent`; percentages resolve against the row's full content width. |
| Item width bounds and edges | Existing min/max widths, content/border-box sizing, fixed or integral-percentage margins and padding apply before placement. |
| Item height | Existing intrinsic, explicit or definite-percentage height; no cross-axis stretching. |
| `Style::column_gap` | Nonnegative integer pixels between successive active items. Default zero; no leading or trailing gap. |
| `Style::flex_wrap` | `FlexWrap::NoWrap` by default; `Wrap` greedily collects lines against the row's content width. |
| `Style::row_gap` | Nonnegative integer pixels between actual lines. Default zero; unused for a single line. |
| Row auto height | The sum of line heights and intervening row gaps, subject to the existing intrinsic-height minimum and explicit height bounds. Each line uses its maximum normal-flow item margin-box height. |
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

## Explicit admission limits

`LayoutError::UnsupportedFlexItemWidth { node }` rejects an active row item whose
width is `Auto`, even if it has a minimum or maximum width. Supporting intrinsic
flex bases and automatic minimum sizing needs a separate measurement contract.

`LayoutError::UnsupportedFlexItemAutoMargins { node }` rejects horizontal auto
margins on row items. Ordinary block auto margins and auto margins on a row
container in a block context keep their existing behavior. Boxless wrappers and
display-none subtrees have no row-item sizing obligations.

Flexible growth/shrinkage, cross-axis stretching, baseline alignment,
main-axis free-space distribution, reverse directions, reordering and
percentage gaps are outside this profile. The public enum names the
specific resolved row profile instead of implying that arbitrary `display:flex`
declarations are supported. All row settings are ignored by other display modes.

## Verification and cost boundaries

`crates/grunkalunka-layout/tests/flex_row.rs` specifies literal geometry for
outer sizes, percentages, bounds, box sizing, boxless tree order, nested
formatting contexts and zero-sized items. It separately checks selected pixels,
topmost hits, document-scroll projection, padding clips, inherited policies and
semantic text anchors. Browser captures do not supply these expectations.
`tests/flex_wrap.rs` separately checks exact fits, line maxima, zero-width and
oversized items, nested wrapping, hidden/boxless nodes, toggling and resizing,
semantic reflow, scrolled clipping and discarded-versus-actual gap overflow.

Both row-item discovery and nested layout use heap-owned work lists. The
existing 4,096-node and depth-64 bounds remain in force, with just-past-limit
errors and admitted nested rows exercised on a 2 MiB native stack. All dimension,
cursor and actual gap arithmetic is checked. A row never shrinks, truncates or
silently rounds a value to make the operation succeed.

Replay with `cargo test -p grunkalunka-layout --locked --offline`. Independent
browser comparison evidence, when available, remains distinct from these native
tests and does not establish general flexbox conformance.
