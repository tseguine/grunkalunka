//! A deliberately small typed semantic-element adapter for headless block layout.
//!
//! An upstream HTML parser (and a future CSS resolver) must supply the tree and
//! resolved styles. Text heights come from a caller-owned measurement service;
//! this module never interprets markup, contacts a resource resolver, or shapes glyphs.

use crate::{Block, Display, Edges, Frame, LayoutError, MAX_BLOCKS, Rect, Style, layout};

pub const MAX_TEXT_BYTES: usize = 256 * 1024;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ElementKind {
    Body,
    Main,
    Heading(u8),
    OrderedList,
    ListItem,
    Paragraph,
    Div,
}

/// A node already constructed by a trusted caller/parser, in parent-before-child order.
///
/// Nodes with direct text must have no children in this selected subset. The text
/// is inert data: markup-like characters are not parsed or executed.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Element<'a> {
    pub parent: Option<usize>,
    pub kind: ElementKind,
    pub text: &'a str,
    /// Explicit CSS-pixel height measured by the caller, never inferred from text.
    pub measured_text_height: u32,
    /// Suppress this node and its descendants, regardless of the supplied style.
    pub hidden: bool,
    /// An entire already-resolved box style; no CSS selector/cascade interpretation.
    pub resolved_style: Option<Style>,
}

impl<'a> Element<'a> {
    pub const fn new(
        parent: Option<usize>,
        kind: ElementKind,
        text: &'a str,
        measured_text_height: u32,
    ) -> Self {
        Self {
            parent,
            kind,
            text,
            measured_text_height,
            hidden: false,
            resolved_style: None,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SemanticError {
    Layout(LayoutError),
    InvalidRootKind,
    NestedBody { node: usize },
    InvalidHeadingLevel { node: usize },
    MixedTextAndChildren { node: usize },
    UnmeasuredText { node: usize },
    TextQuotaExceeded,
}

impl From<LayoutError> for SemanticError {
    fn from(value: LayoutError) -> Self {
        Self::Layout(value)
    }
}

/// Unshaped text paired with the corresponding content rectangle.
/// Drawing and clipping glyphs require an independent text/paint implementation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TextBox<'a> {
    pub node: usize,
    pub text: &'a str,
    pub content: Rect,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SemanticFrame<'a> {
    pub frame: Frame,
    pub text_boxes: Vec<TextBox<'a>>,
}

/// Toy fixed defaults, not HTML user-agent CSS and not a CSS cascade.
pub fn default_style(kind: ElementKind) -> Style {
    match kind {
        ElementKind::Body | ElementKind::Div => Style::default(),
        ElementKind::Main => Style {
            padding: Edges::all(8),
            ..Style::default()
        },
        ElementKind::Heading(_) => Style {
            margin: Edges {
                bottom: 8,
                ..Edges::default()
            },
            ..Style::default()
        },
        ElementKind::OrderedList => Style {
            margin: Edges {
                bottom: 6,
                ..Edges::default()
            },
            padding: Edges {
                left: 16,
                ..Edges::default()
            },
            ..Style::default()
        },
        ElementKind::ListItem => Style {
            margin: Edges {
                bottom: 2,
                ..Edges::default()
            },
            ..Style::default()
        },
        ElementKind::Paragraph => Style {
            margin: Edges {
                bottom: 6,
                ..Edges::default()
            },
            ..Style::default()
        },
    }
}

/// Project a small, explicitly built semantic element tree into the existing
/// layout engine. The output includes inert text and its geometric anchor, not
/// painted pixels or actual glyph extents.
pub fn layout_semantic<'a>(
    elements: &[Element<'a>],
    viewport_width: u32,
    viewport_height: u32,
) -> Result<SemanticFrame<'a>, SemanticError> {
    if elements.is_empty() {
        return Err(SemanticError::Layout(LayoutError::EmptyDocument));
    }
    if elements.len() > MAX_BLOCKS {
        return Err(SemanticError::Layout(LayoutError::TooManyBlocks));
    }
    if elements[0].kind != ElementKind::Body {
        return Err(SemanticError::InvalidRootKind);
    }

    let mut text_bytes = 0usize;
    let mut has_children = vec![false; elements.len()];
    for (node, element) in elements.iter().enumerate() {
        if node != 0 && element.kind == ElementKind::Body {
            return Err(SemanticError::NestedBody { node });
        }
        if let ElementKind::Heading(level) = element.kind
            && !(1..=6).contains(&level)
        {
            return Err(SemanticError::InvalidHeadingLevel { node });
        }
        if !element.text.is_empty() && element.measured_text_height == 0 {
            return Err(SemanticError::UnmeasuredText { node });
        }
        text_bytes = text_bytes
            .checked_add(element.text.len())
            .ok_or(SemanticError::TextQuotaExceeded)?;
        if text_bytes > MAX_TEXT_BYTES {
            return Err(SemanticError::TextQuotaExceeded);
        }
        if let Some(parent) = element.parent
            && parent < node
        {
            has_children[parent] = true;
        }
    }
    for (node, element) in elements.iter().enumerate() {
        if has_children[node] && !element.text.is_empty() {
            return Err(SemanticError::MixedTextAndChildren { node });
        }
    }

    let blocks: Vec<_> = elements
        .iter()
        .map(|element| {
            let mut style = element
                .resolved_style
                .unwrap_or_else(|| default_style(element.kind));
            if element.hidden {
                style.display = Display::None;
            }
            Block {
                parent: element.parent,
                style,
                intrinsic_height: element.measured_text_height,
            }
        })
        .collect();
    let frame = layout(&blocks, viewport_width, viewport_height)?;
    let text_boxes = elements
        .iter()
        .enumerate()
        .filter_map(|(node, element)| {
            if element.text.is_empty() {
                return None;
            }
            frame.boxes[node].map(|geometry| TextBox {
                node,
                text: element.text,
                content: geometry.content,
            })
        })
        .collect();
    Ok(SemanticFrame { frame, text_boxes })
}

#[cfg(test)]
mod tests;
