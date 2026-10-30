//! A bounded, host-independent normal-flow block-box layout experiment.
//! HTML parsing, CSS cascade, font metrics and scripting are external.
//! A separate optional module provides bounded opaque RGB diagnostic painting.

#![forbid(unsafe_code)]

pub mod hit_test;
pub mod paint;
pub mod semantic;

pub const MAX_BLOCKS: usize = 4096;
pub const MAX_DEPTH: usize = 64;
pub const MAX_VIEWPORT: u32 = 16_384;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Edges {
    pub top: u32,
    pub right: u32,
    pub bottom: u32,
    pub left: u32,
}

impl Edges {
    pub const fn all(value: u32) -> Self {
        Self {
            top: value,
            right: value,
            bottom: value,
            left: value,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum Length {
    #[default]
    Auto,
    Px(u32),
    /// Whole percent of the containing content width or height. Fractional
    /// results are rejected. Heights need a definite containing content
    /// height (viewport for the root, explicit height for descendants).
    Percent(u32),
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum Display {
    #[default]
    Block,
    /// Remove the element's own box while laying out its children directly
    /// in the containing block's normal flow.
    Contents,
    None,
}

/// Already-resolved visibility declaration. Children inherit unless explicitly
/// overridden; unlike display:none, a hidden ancestor retains its boxes and
/// an explicitly visible descendant can be painted and hit.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum Visibility {
    #[default]
    Inherit,
    Visible,
    Hidden,
}

impl Visibility {
    fn resolve(self, inherited: bool) -> bool {
        match self {
            Self::Inherit => inherited,
            Self::Visible => true,
            Self::Hidden => false,
        }
    }
}

/// Resolved pointer-target eligibility. This mirrors only HTML auto/none
/// inheritance for source-independent hit testing, not DOM event dispatch,
/// pointer capture, SVG-specific values or keyboard focus.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum PointerEvents {
    #[default]
    Inherit,
    Auto,
    None,
}

impl PointerEvents {
    fn resolve(self, inherited: bool) -> bool {
        match self {
            Self::Inherit => inherited,
            Self::Auto => true,
            Self::None => false,
        }
    }
}

/// An explicit clipping policy for a node's descendants. This deliberately
/// models only padding-box clipping of paint/hits, not CSS scroll containers,
/// overflow propagation, scrollbars or clipping of the node's own border.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum DescendantClip {
    #[default]
    None,
    PaddingBox,
}

/// Explicitly resolved box sizing, not a CSS declaration parser.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum BoxSizing {
    #[default]
    ContentBox,
    BorderBox,
}

impl BoxSizing {
    fn content_size(self, specified: u32, border_padding: u32) -> u32 {
        match self {
            Self::ContentBox => specified,
            // A requested border box cannot be smaller than its edges.
            // Keep the content box nonnegative, enlarging the used border box.
            Self::BorderBox => specified.saturating_sub(border_padding),
        }
    }
}

/// Per-side, explicitly selected CSS percentage margins. All four sides,
/// INCLUDING top and bottom, resolve from the containing CONTENT width
/// (horizontal writing-mode model). A None side uses `Style::margin`.
/// Horizontal auto margins take priority over both when selected.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct PercentageMargins {
    pub top: Option<u32>,
    pub right: Option<u32>,
    pub bottom: Option<u32>,
    pub left: Option<u32>,
}

/// Caller-resolved percentage padding for a horizontal writing-mode
/// normal-flow block. As in CSS, ALL sides (including top/bottom) use
/// the containing block's original CONTENT width, not its height.
/// Unspecified sides retain `Style::padding` integer pixel values.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct PercentagePadding {
    pub top: Option<u32>,
    pub right: Option<u32>,
    pub bottom: Option<u32>,
    pub left: Option<u32>,
}

/// Which horizontal margins use the CSS-inspired `auto` policy.
/// An auto side ignores the corresponding numerical side of `Style::margin`.
/// The default preserves both explicit integer margins. This is a selected
/// normal-flow block policy, not a CSS parser/cascade or direction model.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum HorizontalAutoMargins {
    #[default]
    None,
    Left,
    Right,
    Both,
}

impl HorizontalAutoMargins {
    fn sides(self) -> (bool, bool) {
        (
            matches!(self, Self::Left | Self::Both),
            matches!(self, Self::Right | Self::Both),
        )
    }
}

/// A selected, already-resolved positive post-flow displacement of an
/// element and its subtree. Coordinates are integer CSS pixels to the right
/// and down. This is NOT full CSS positioned layout: negative offsets, inset
/// precedence, stacking contexts, absolute/fixed/sticky and z-index remain
/// outside this headless normal-flow model.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct RelativeOffset {
    pub x: u32,
    pub y: u32,
}

/// Caller-resolved minimum/maximum width bound, with no `auto` value.
/// A percent is relative to the containing block's content width.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WidthBound {
    Px(u32),
    Percent(u32),
}

/// Caller-resolved minimum/maximum height bound. Percentages require a
/// definite containing content height; no CSS fallback is guessed.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum HeightBound {
    Px(u32),
    Percent(u32),
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Style {
    pub display: Display,
    pub visibility: Visibility,
    pub pointer_events: PointerEvents,
    pub descendant_clip: DescendantClip,
    pub width: Length,
    pub min_width: Option<WidthBound>,
    pub max_width: Option<WidthBound>,
    pub height: Length,
    pub min_height: Option<HeightBound>,
    pub max_height: Option<HeightBound>,
    pub box_sizing: BoxSizing,
    pub margin: Edges,
    /// Exact-integer percentage margins relative to the containing width.
    /// A selected side overrides its fixed `margin` value.
    pub percentage_margins: PercentageMargins,
    pub horizontal_auto_margins: HorizontalAutoMargins,
    /// A positive relative visual translation; ignored on boxless wrappers.
    /// Affects this node and descendants, but not following siblings' flow.
    pub relative_offset: RelativeOffset,
    pub border: Edges,
    pub padding: Edges,
    /// Optional per-side percentages overriding fixed padding edges.
    pub percentage_padding: PercentagePadding,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Block {
    /// A node's parent must precede it; only the first node has no parent.
    pub parent: Option<usize>,
    pub style: Style,
    /// Explicit caller-measured minimum height for auto-height content.
    pub intrinsic_height: u32,
}

impl Block {
    pub fn child(parent: usize, intrinsic_height: u32) -> Self {
        Self {
            parent: Some(parent),
            intrinsic_height,
            ..Self::default()
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Rect {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

impl Rect {
    pub fn right(self) -> Option<u32> {
        self.x.checked_add(self.width)
    }

    pub fn bottom(self) -> Option<u32> {
        self.y.checked_add(self.height)
    }

    /// Intersection of already-resolved, nonnegative rectangle coordinates.
    /// A disjoint intersection is an empty rectangle, NOT an absent clip.
    /// None means the caller's geometry overflowed its coordinate domain.
    pub fn intersection(self, other: Self) -> Option<Self> {
        let x = self.x.max(other.x);
        let y = self.y.max(other.y);
        let right = self.right()?.min(other.right()?);
        let bottom = self.bottom()?.min(other.bottom()?);
        Some(Self {
            x,
            y,
            width: right.saturating_sub(x),
            height: bottom.saturating_sub(y),
        })
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BoxGeometry {
    pub margin: Rect,
    pub border: Rect,
    pub padding: Rect,
    pub content: Rect,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Frame {
    /// None represents display:none and its descendants.
    pub boxes: Vec<Option<BoxGeometry>>,
    /// Active display:contents nodes. Their own box is absent, but their
    /// descendants still participate in layout, painting and input.
    /// Inactive descendants of display:none are not marked boxless.
    pub contents: Vec<bool>,
    /// Computed inherited visibility per node; independent of layout box
    /// existence. Explicit visible descendants can override hidden ancestors.
    pub visibility: Vec<bool>,
    /// Resolved inherited pointer-target eligibility, independent of
    /// caller HitNode::targetable, painted pixels or box visibility.
    pub pointer_events: Vec<bool>,
    /// Whether a node clips its descendants to its resolved padding box.
    /// An ancestor's clip still affects the node's own painting/hit tests.
    pub clip_descendants: Vec<bool>,
    pub viewport_width: u32,
    pub viewport_height: u32,
    pub scroll_width: u32,
    pub scroll_height: u32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LayoutError {
    InvalidViewport,
    EmptyDocument,
    TooManyBlocks,
    InvalidRoot,
    InvalidParent {
        node: usize,
    },
    TooDeep {
        node: usize,
    },
    FractionalPercentageWidth {
        node: usize,
    },
    FractionalPercentageHeight {
        node: usize,
    },
    FractionalPercentageMargin {
        node: usize,
    },
    FractionalPercentagePadding {
        node: usize,
    },
    /// Two auto margins would each need half an integer CSS pixel; this
    /// engine deliberately refuses fractional geometry instead of rounding.
    FractionalAutoMargin {
        node: usize,
    },
    UnsupportedPercentageHeight {
        node: usize,
    },
    Overflow,
}

fn add(a: u32, b: u32) -> Result<u32, LayoutError> {
    a.checked_add(b).ok_or(LayoutError::Overflow)
}

fn insets(edges: Edges) -> Result<(u32, u32), LayoutError> {
    Ok((add(edges.left, edges.right)?, add(edges.top, edges.bottom)?))
}

// Preserve the integer-box contract for either dimension. Rounding CSS
// subpixels is deliberately not part of this selected geometry experiment.
fn whole_percent(
    containing: u32,
    percent: u32,
    fractional: LayoutError,
) -> Result<u32, LayoutError> {
    let scaled = u64::from(containing)
        .checked_mul(u64::from(percent))
        .ok_or(LayoutError::Overflow)?;
    if scaled % 100 != 0 {
        return Err(fractional);
    }
    u32::try_from(scaled / 100).map_err(|_| LayoutError::Overflow)
}

struct Engine<'a> {
    nodes: &'a [Block],
    children: Vec<Vec<usize>>,
    frame: Frame,
}

// A normal-flow box has a pre-children geometry phase and a post-children
// height/scroll phase. Keeping these values in heap frames prevents native
// stack growth with layout tree depth.
#[derive(Clone, Copy)]
struct PreparedBox {
    node: usize,
    intrinsic_height: u32,
    margin: Edges,
    visual_x: u32,
    visual_y: u32,
    margin_left: u32,
    margin_right: u32,
    border_left: u32,
    border_top: u32,
    padding_left: u32,
    padding_top: u32,
    content_x: u32,
    content_y: u32,
    content_width: u32,
    definite_height: Option<u32>,
    max_content: Option<u32>,
    min_content: Option<u32>,
    border_width: u32,
    border_x: u32,
    border_y: u32,
    padding_x: u32,
    padding_y: u32,
}

struct PlaceFrame {
    node: usize,
    containing_x: u32,
    cursor_y: u32,
    available_width: u32,
    containing_height: Option<u32>,
    next_child: usize,
    next_y: u32,
    entered: bool,
    contents: bool,
    prepared: Option<PreparedBox>,
}

impl PlaceFrame {
    fn new(
        node: usize,
        containing_x: u32,
        cursor_y: u32,
        available_width: u32,
        containing_height: Option<u32>,
    ) -> Self {
        Self {
            node,
            containing_x,
            cursor_y,
            available_width,
            containing_height,
            next_child: 0,
            next_y: cursor_y,
            entered: false,
            contents: false,
            prepared: None,
        }
    }
}

impl Engine<'_> {
    // Each frame carries one subtree's sibling cursor. Completed child heights
    // flow back to the parent without calling place recursively.
    fn place(
        &mut self,
        node: usize,
        containing_x: u32,
        cursor_y: u32,
        available_width: u32,
        containing_height: Option<u32>,
    ) -> Result<u32, LayoutError> {
        let mut pending = vec![PlaceFrame::new(
            node,
            containing_x,
            cursor_y,
            available_width,
            containing_height,
        )];
        let mut completed = None;
        while let Some(current) = pending.last_mut() {
            if let Some(height) = completed.take() {
                current.next_y = add(current.next_y, height)?;
            }
            if !current.entered {
                current.entered = true;
                match self.nodes[current.node].style.display {
                    Display::None => {
                        pending.pop();
                        completed = Some(0);
                        continue;
                    }
                    Display::Contents => {
                        current.contents = true;
                        current.next_y = current.cursor_y;
                    }
                    Display::Block => {
                        let prepared = self.prepare(
                            current.node,
                            current.containing_x,
                            current.cursor_y,
                            current.available_width,
                            current.containing_height,
                        )?;
                        current.next_y = prepared.content_y;
                        current.prepared = Some(prepared);
                    }
                }
            }
            if let Some(&child) = self.children[current.node].get(current.next_child) {
                current.next_child += 1;
                let (x, width, height) = if let Some(prepared) = current.prepared {
                    (
                        prepared.content_x,
                        prepared.content_width,
                        prepared.definite_height,
                    )
                } else {
                    (
                        current.containing_x,
                        current.available_width,
                        current.containing_height,
                    )
                };
                let next_y = current.next_y;
                pending.push(PlaceFrame::new(child, x, next_y, width, height));
            } else {
                let height = if current.contents {
                    current.next_y - current.cursor_y
                } else {
                    self.finish(
                        &current.prepared.expect("entered normal-flow box"),
                        current.next_y,
                    )?
                };
                pending.pop();
                completed = Some(height);
            }
        }
        Ok(completed.expect("root placement completed"))
    }

    fn prepare(
        &self,
        node: usize,
        containing_x: u32,
        cursor_y: u32,
        available_width: u32,
        containing_height: Option<u32>,
    ) -> Result<PreparedBox, LayoutError> {
        let block = self.nodes[node];
        let style = block.style;
        // Relative positioning changes the *visual* origin of this box and
        // its descendants, without consuming additional normal-flow space.
        // A boxless/hidden wrapper was returned above and has no offset.
        let visual_x = add(containing_x, style.relative_offset.x)?;
        let visual_y = add(cursor_y, style.relative_offset.y)?;

        let (auto_left, auto_right) = style.horizontal_auto_margins.sides();
        // Percentage margins use the ORIGINAL containing content width,
        // including the vertical sides. The exact-integer policy rejects
        // fractional CSS pixels rather than silently rounding them.
        // A horizontally auto side ignores both its fixed and percent input.
        let resolve_margin = |percent: Option<u32>, fixed: u32| -> Result<u32, LayoutError> {
            match percent {
                Some(value) => whole_percent(
                    available_width,
                    value,
                    LayoutError::FractionalPercentageMargin { node },
                ),
                None => Ok(fixed),
            }
        };
        let margin = Edges {
            top: resolve_margin(style.percentage_margins.top, style.margin.top)?,
            right: if auto_right {
                0
            } else {
                resolve_margin(style.percentage_margins.right, style.margin.right)?
            },
            bottom: resolve_margin(style.percentage_margins.bottom, style.margin.bottom)?,
            left: if auto_left {
                0
            } else {
                resolve_margin(style.percentage_margins.left, style.margin.left)?
            },
        };
        // Auto sides do not participate in the tentative width calculation.
        // A width:auto block therefore fills the remaining space and uses
        // zero auto margins unless a width bound later makes room for them.
        let fixed_left = margin.left;
        let fixed_right = margin.right;
        let fixed_margins = add(fixed_left, fixed_right)?;
        // Resolve each used padding edge against the ORIGINAL containing
        // content width, even for top/bottom. Padding cannot be auto or
        // negative in this model. Exact-integer percentages fail closed.
        let resolve_padding = |percent: Option<u32>, fixed: u32| -> Result<u32, LayoutError> {
            match percent {
                Some(value) => whole_percent(
                    available_width,
                    value,
                    LayoutError::FractionalPercentagePadding { node },
                ),
                None => Ok(fixed),
            }
        };
        let padding = Edges {
            top: resolve_padding(style.percentage_padding.top, style.padding.top)?,
            right: resolve_padding(style.percentage_padding.right, style.padding.right)?,
            bottom: resolve_padding(style.percentage_padding.bottom, style.padding.bottom)?,
            left: resolve_padding(style.percentage_padding.left, style.padding.left)?,
        };
        let (border_x, border_y) = insets(style.border)?;
        let (padding_x, padding_y) = insets(padding)?;
        let width_edges = add(border_x, padding_x)?;
        let height_edges = add(border_y, padding_y)?;
        let horizontal = add(fixed_margins, width_edges)?;
        let tentative_width = match style.width {
            Length::Auto => available_width.saturating_sub(horizontal),
            Length::Px(value) => style.box_sizing.content_size(value, width_edges),
            Length::Percent(value) => style.box_sizing.content_size(
                whole_percent(
                    available_width,
                    value,
                    LayoutError::FractionalPercentageWidth { node },
                )?,
                width_edges,
            ),
        };
        // Width limits constrain the same used box selected by box_sizing.
        // Resolve both against the original containing content width, never
        // the candidate width or the newly constrained width. Apply max first
        // and min second: an explicit minimum wins a contradictory maximum.
        let resolve_bound = |bound: WidthBound| -> Result<u32, LayoutError> {
            let specified = match bound {
                WidthBound::Px(value) => value,
                WidthBound::Percent(value) => whole_percent(
                    available_width,
                    value,
                    LayoutError::FractionalPercentageWidth { node },
                )?,
            };
            Ok(style.box_sizing.content_size(specified, width_edges))
        };
        let max_content = style.max_width.map(resolve_bound).transpose()?;
        let min_content = style.min_width.map(resolve_bound).transpose()?;
        let content_width = tentative_width
            .min(max_content.unwrap_or(u32::MAX))
            .max(min_content.unwrap_or(0));
        let border_width = add(content_width, width_edges)?;
        // Resolve used auto margins AFTER min/max width and box-sizing.
        // In the nonnegative integer model, oversize boxes consume all
        // remaining space; negative CSS margins are not representable.
        let remaining_width = available_width.saturating_sub(add(border_width, fixed_margins)?);
        let (margin_left, margin_right) = match style.horizontal_auto_margins {
            HorizontalAutoMargins::None => (fixed_left, fixed_right),
            HorizontalAutoMargins::Left => (remaining_width, fixed_right),
            HorizontalAutoMargins::Right => (fixed_left, remaining_width),
            HorizontalAutoMargins::Both => {
                if remaining_width % 2 != 0 {
                    return Err(LayoutError::FractionalAutoMargin { node });
                }
                (remaining_width / 2, remaining_width / 2)
            }
        };

        let border_left = add(visual_x, margin_left)?;
        let padding_left = add(border_left, style.border.left)?;
        let content_x = add(padding_left, padding.left)?;
        let border_top = add(visual_y, margin.top)?;
        let padding_top = add(border_top, style.border.top)?;
        let content_y = add(padding_top, padding.top)?;

        // Resolve both height limits independently against the parent's
        // definite content height. No percentage fallback is invented for an
        // auto-height containing block; hidden subtrees were skipped above.
        let resolve_height_bound = |bound: HeightBound| -> Result<u32, LayoutError> {
            let specified = match bound {
                HeightBound::Px(value) => value,
                HeightBound::Percent(value) => whole_percent(
                    containing_height.ok_or(LayoutError::UnsupportedPercentageHeight { node })?,
                    value,
                    LayoutError::FractionalPercentageHeight { node },
                )?,
            };
            Ok(style.box_sizing.content_size(specified, height_edges))
        };
        let max_content = style.max_height.map(resolve_height_bound).transpose()?;
        let min_content = style.min_height.map(resolve_height_bound).transpose()?;
        let apply_bounds = |height: u32| {
            height
                .min(max_content.unwrap_or(u32::MAX))
                .max(min_content.unwrap_or(0))
        };

        // A child's percentage height uses this box's *used definite*
        // content height. Clamped fixed/percentage heights remain definite;
        // auto heights stay indefinite even if their used size is bounded.
        let specified_height = match style.height {
            Length::Auto => None,
            Length::Px(value) => Some(style.box_sizing.content_size(value, height_edges)),
            Length::Percent(value) => Some(style.box_sizing.content_size(
                whole_percent(
                    containing_height.ok_or(LayoutError::UnsupportedPercentageHeight { node })?,
                    value,
                    LayoutError::FractionalPercentageHeight { node },
                )?,
                height_edges,
            )),
        };
        let definite_height = specified_height.map(apply_bounds);
        Ok(PreparedBox {
            node,
            intrinsic_height: block.intrinsic_height,
            margin,
            visual_x,
            visual_y,
            margin_left,
            margin_right,
            border_left,
            border_top,
            padding_left,
            padding_top,
            content_x,
            content_y,
            content_width,
            definite_height,
            max_content,
            min_content,
            border_width,
            border_x,
            border_y,
            padding_x,
            padding_y,
        })
    }

    fn finish(&mut self, prepared: &PreparedBox, next_y: u32) -> Result<u32, LayoutError> {
        let PreparedBox {
            node,
            intrinsic_height,
            margin,
            visual_x,
            visual_y,
            margin_left,
            margin_right,
            border_left,
            border_top,
            padding_left,
            padding_top,
            content_x,
            content_y,
            content_width,
            definite_height,
            max_content,
            min_content,
            border_width,
            border_x,
            border_y,
            padding_x,
            padding_y,
        } = *prepared;
        let natural_height = (next_y - content_y).max(intrinsic_height);
        let content_height = definite_height.unwrap_or_else(|| {
            natural_height
                .min(max_content.unwrap_or(u32::MAX))
                .max(min_content.unwrap_or(0))
        });
        let padding_width = add(content_width, padding_x)?;
        let padding_height = add(content_height, padding_y)?;
        let border_height = add(padding_height, border_y)?;
        let margin_width = add(border_width, add(margin_left, margin_right)?)?;
        let margin_height = add(add(margin.top, border_height)?, margin.bottom)?;

        let geometry = BoxGeometry {
            margin: Rect {
                x: visual_x,
                y: visual_y,
                width: margin_width,
                height: margin_height,
            },
            border: Rect {
                x: border_left,
                y: border_top,
                width: border_width,
                height: border_height,
            },
            padding: Rect {
                x: padding_left,
                y: padding_top,
                width: padding_width,
                height: padding_height,
            },
            content: Rect {
                x: content_x,
                y: content_y,
                width: content_width,
                height: content_height,
            },
        };
        self.frame.scroll_width = self
            .frame
            .scroll_width
            .max(geometry.margin.right().ok_or(LayoutError::Overflow)?);
        self.frame.scroll_height = self
            .frame
            .scroll_height
            .max(geometry.margin.bottom().ok_or(LayoutError::Overflow)?);
        self.frame.boxes[node] = Some(geometry);
        Ok(margin_height)
    }

}

/// Arrange a flat, parent-before-child tree from the viewport origin.
///
/// Only nonnegative resolved integer CSS pixels, vertical block flow,
/// display:none, boxless display:contents, inherited visibility, resolved
/// pointer-events input policy, optional padding-box descendant clipping,
/// content-box/border-box sizing, min/max width/height constraints,
/// nonnegative horizontal auto margins, containing-width-relative percentage
/// margins and padding on all four sides, positive post-flow relative offsets,
/// and whole
/// percentage dimensions with exact integral results are admitted.
/// Relative displacements move an element and descendants for geometry,
/// painting, hit testing and scroll extent but do not advance sibling flow
/// or change natural parent heights. Auto margins consume remaining
/// width after min/max constraints; both auto sides share it equally.
/// Fractional halves are rejected, and overconstrained auto margins become
/// zero (negative CSS margins are outside this nonnegative model).
/// Percentage heights need a definite containing content height;
/// unresolved heights and fractional pixel results are rejected.
/// Positive vertical margins do not collapse; inline layout, text shaping, floats,
/// CSS selectors and painting are not provided. Fixed heights may
/// overflow, and all descendants contribute to scroll extents even when
/// clipped for painting and input. Document overflow accounting is unchanged.
pub fn layout(
    blocks: &[Block],
    viewport_width: u32,
    viewport_height: u32,
) -> Result<Frame, LayoutError> {
    if viewport_width == 0
        || viewport_height == 0
        || viewport_width > MAX_VIEWPORT
        || viewport_height > MAX_VIEWPORT
    {
        return Err(LayoutError::InvalidViewport);
    }
    if blocks.is_empty() {
        return Err(LayoutError::EmptyDocument);
    }
    if blocks.len() > MAX_BLOCKS {
        return Err(LayoutError::TooManyBlocks);
    }
    if blocks[0].parent.is_some() {
        return Err(LayoutError::InvalidRoot);
    }

    let mut children = vec![Vec::new(); blocks.len()];
    let mut depths = vec![0usize; blocks.len()];
    let mut visibility = vec![true; blocks.len()];
    let mut pointer_events = vec![true; blocks.len()];
    // The boxless bit only marks contents wrappers in active subtrees.
    // Nodes beneath display:none remain absent and cannot revive children.
    let mut active = vec![true; blocks.len()];
    let mut contents = vec![false; blocks.len()];
    active[0] = blocks[0].style.display != Display::None;
    contents[0] = blocks[0].style.display == Display::Contents;
    let clip_descendants: Vec<bool> = blocks
        .iter()
        .map(|block| block.style.descendant_clip == DescendantClip::PaddingBox)
        .collect();
    visibility[0] = blocks[0].style.visibility.resolve(true);
    pointer_events[0] = blocks[0].style.pointer_events.resolve(true);
    for (node, block) in blocks.iter().enumerate().skip(1) {
        let Some(parent) = block.parent else {
            return Err(LayoutError::InvalidParent { node });
        };
        if parent >= node {
            return Err(LayoutError::InvalidParent { node });
        }
        depths[node] = depths[parent] + 1;
        active[node] = active[parent] && block.style.display != Display::None;
        contents[node] = active[node] && block.style.display == Display::Contents;
        visibility[node] = block.style.visibility.resolve(visibility[parent]);
        pointer_events[node] = block.style.pointer_events.resolve(pointer_events[parent]);
        if depths[node] > MAX_DEPTH {
            return Err(LayoutError::TooDeep { node });
        }
        children[parent].push(node);
    }

    let mut engine = Engine {
        nodes: blocks,
        children,
        frame: Frame {
            boxes: vec![None; blocks.len()],
            contents,
            visibility,
            pointer_events,
            clip_descendants,
            viewport_width,
            viewport_height,
            scroll_width: viewport_width,
            scroll_height: viewport_height,
        },
    };
    engine.place(0, 0, 0, viewport_width, Some(viewport_height))?;
    Ok(engine.frame)
}

#[cfg(test)]
mod tests;
