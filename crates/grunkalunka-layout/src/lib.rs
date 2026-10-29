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
    None,
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
    pub width: Length,
    pub min_width: Option<WidthBound>,
    pub max_width: Option<WidthBound>,
    pub height: Length,
    pub min_height: Option<HeightBound>,
    pub max_height: Option<HeightBound>,
    pub box_sizing: BoxSizing,
    pub margin: Edges,
    pub border: Edges,
    pub padding: Edges,
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
    InvalidParent { node: usize },
    TooDeep { node: usize },
    FractionalPercentageWidth { node: usize },
    FractionalPercentageHeight { node: usize },
    UnsupportedPercentageHeight { node: usize },
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

impl Engine<'_> {
    // Return the margin-box height occupied in the parent's vertical flow.
    fn place(
        &mut self,
        node: usize,
        containing_x: u32,
        cursor_y: u32,
        available_width: u32,
        containing_height: Option<u32>,
    ) -> Result<u32, LayoutError> {
        let block = self.nodes[node];
        let style = block.style;
        if style.display == Display::None {
            return Ok(0);
        }

        let (margin_x, _) = insets(style.margin)?;
        let (border_x, border_y) = insets(style.border)?;
        let (padding_x, padding_y) = insets(style.padding)?;
        let width_edges = add(border_x, padding_x)?;
        let height_edges = add(border_y, padding_y)?;
        let horizontal = add(margin_x, width_edges)?;
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

        let border_left = add(containing_x, style.margin.left)?;
        let padding_left = add(border_left, style.border.left)?;
        let content_x = add(padding_left, style.padding.left)?;
        let border_top = add(cursor_y, style.margin.top)?;
        let padding_top = add(border_top, style.border.top)?;
        let content_y = add(padding_top, style.padding.top)?;

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
        let max_content = style
            .max_height
            .map(&resolve_height_bound)
            .transpose()?;
        let min_content = style
            .min_height
            .map(&resolve_height_bound)
            .transpose()?;
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
        let definite_height = specified_height.map(&apply_bounds);
        let mut next_y = content_y;
        for index in 0..self.children[node].len() {
            let child = self.children[node][index];
            let consumed = self.place(child, content_x, next_y, content_width, definite_height)?;
            next_y = add(next_y, consumed)?;
        }
        let natural_height = (next_y - content_y).max(block.intrinsic_height);
        let content_height = definite_height.unwrap_or_else(|| apply_bounds(natural_height));
        let padding_width = add(content_width, padding_x)?;
        let padding_height = add(content_height, padding_y)?;
        let border_width = add(padding_width, border_x)?;
        let border_height = add(padding_height, border_y)?;
        let margin_width = add(border_width, margin_x)?;
        let margin_height = add(add(style.margin.top, border_height)?, style.margin.bottom)?;

        let geometry = BoxGeometry {
            margin: Rect {
                x: containing_x,
                y: cursor_y,
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
/// display:none, explicit content-box/border-box sizing, optional min/max
/// width/height constraints, and whole percentage dimensions with exact
/// integral results are admitted. Percentage heights
/// require a definite containing content
/// height; unresolved heights and fractional pixel results are rejected.
/// Positive margins do not collapse; inline layout, text shaping, floats,
/// CSS selectors and painting are not provided. Fixed heights may
/// overflow, and all descendants contribute to the resulting scroll extents.
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
    for (node, block) in blocks.iter().enumerate().skip(1) {
        let Some(parent) = block.parent else {
            return Err(LayoutError::InvalidParent { node });
        };
        if parent >= node {
            return Err(LayoutError::InvalidParent { node });
        }
        depths[node] = depths[parent] + 1;
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
