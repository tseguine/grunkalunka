//! A bounded, host-independent normal-flow block-box layout experiment.
//! HTML parsing, CSS cascade, font metrics and scripting are external.
//! A separate optional module provides bounded opaque RGB diagnostic painting.

#![forbid(unsafe_code)]

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
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum Display {
    #[default]
    Block,
    None,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Style {
    pub display: Display,
    pub width: Length,
    pub height: Length,
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
    Overflow,
}

fn add(a: u32, b: u32) -> Result<u32, LayoutError> {
    a.checked_add(b).ok_or(LayoutError::Overflow)
}

fn insets(edges: Edges) -> Result<(u32, u32), LayoutError> {
    Ok((add(edges.left, edges.right)?, add(edges.top, edges.bottom)?))
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
    ) -> Result<u32, LayoutError> {
        let block = self.nodes[node];
        let style = block.style;
        if style.display == Display::None {
            return Ok(0);
        }

        let (margin_x, _) = insets(style.margin)?;
        let (border_x, border_y) = insets(style.border)?;
        let (padding_x, padding_y) = insets(style.padding)?;
        let horizontal = add(add(margin_x, border_x)?, padding_x)?;
        let content_width = match style.width {
            Length::Auto => available_width.saturating_sub(horizontal),
            Length::Px(value) => value,
        };

        let border_left = add(containing_x, style.margin.left)?;
        let padding_left = add(border_left, style.border.left)?;
        let content_x = add(padding_left, style.padding.left)?;
        let border_top = add(cursor_y, style.margin.top)?;
        let padding_top = add(border_top, style.border.top)?;
        let content_y = add(padding_top, style.padding.top)?;

        let mut next_y = content_y;
        for index in 0..self.children[node].len() {
            let child = self.children[node][index];
            let consumed = self.place(child, content_x, next_y, content_width)?;
            next_y = add(next_y, consumed)?;
        }
        let natural_height = (next_y - content_y).max(block.intrinsic_height);
        let content_height = match style.height {
            Length::Auto => natural_height,
            Length::Px(value) => value,
        };
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
/// Only nonnegative resolved CSS pixels, vertical block flow and display:none
/// are admitted. Positive margins do not collapse; inline layout, text shaping,
/// floats, CSS selectors and painting are not provided. Fixed heights may
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
    engine.place(0, 0, 0, viewport_width)?;
    Ok(engine.frame)
}

#[cfg(test)]
mod tests;
