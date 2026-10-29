//! Bounded, headless RGB diagnostic painting of already-laid-out block boxes.
//!
//! The caller provides geometry and colors. No fonts, CSS paint semantics,
//! HTML parsing, window backend, GPU or filesystem access are involved.

use crate::{Frame, MAX_BLOCKS, MAX_DEPTH, MAX_VIEWPORT, Rect};

mod bitmap;

pub const MAX_CANVAS_PIXELS: usize = 4_000_000;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Rgb {
    pub red: u8,
    pub green: u8,
    pub blue: u8,
}

impl Rgb {
    pub const WHITE: Self = Self {
        red: 255,
        green: 255,
        blue: 255,
    };

    pub const BLACK: Self = Self {
        red: 0,
        green: 0,
        blue: 0,
    };

    pub const fn new(red: u8, green: u8, blue: u8) -> Self {
        Self { red, green, blue }
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct PaintNode {
    /// Parent index must match the corresponding layout input.
    pub parent: Option<usize>,
    /// The padding box, including its content box, receives this color.
    pub background: Option<Rgb>,
    /// Four strips between the border and padding box receive this color.
    pub border: Option<Rgb>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PaintError {
    InvalidViewport,
    InvalidScrollOffset,
    /// Malformed caller-supplied bitpacked monochrome mask.
    InvalidBitmap,
    TooManyPixels,
    NodeCountMismatch,
    InvalidParent {
        node: usize,
    },
    TooDeep {
        node: usize,
    },
    VisibleChildOfHiddenParent {
        node: usize,
    },
    InvalidBox {
        node: usize,
    },
    Overflow,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Canvas {
    width: u32,
    height: u32,
    scroll_x: u32,
    scroll_y: u32,
    pixels: Vec<Rgb>,
}

impl Canvas {
    pub const fn width(&self) -> u32 {
        self.width
    }

    pub const fn height(&self) -> u32 {
        self.height
    }

    pub fn pixel(&self, x: u32, y: u32) -> Option<Rgb> {
        if x < self.width && y < self.height {
            Some(self.pixels[y as usize * self.width as usize + x as usize])
        } else {
            None
        }
    }

    /// Opaque binary PPM (P6) bytes. No ambient file access.
    pub fn ppm(&self) -> Vec<u8> {
        let header = format!("P6\n{} {}\n255\n", self.width, self.height);
        let mut bytes = Vec::with_capacity(header.len() + self.pixels.len() * 3);
        bytes.extend_from_slice(header.as_bytes());
        for pixel in &self.pixels {
            bytes.extend_from_slice(&[pixel.red, pixel.green, pixel.blue]);
        }
        bytes
    }

    fn fill(
        &mut self,
        rect: Rect,
        color: Rgb,
        ancestor_clip: Option<Rect>,
    ) -> Result<(), PaintError> {
        // The caller's nested ancestor clip applies before viewport clipping.
        // None means no clip; a zero-sized intersection remains explicitly
        // empty and can never allow descendant paint back through.
        let rect = if let Some(clip) = ancestor_clip {
            rect.intersection(clip).ok_or(PaintError::Overflow)?
        } else {
            rect
        };
        // Clip document-space boxes to the scrolled viewport before indexing.
        // Subtract only after intersection; offscreen origins stay unsigned.
        let view_right = self
            .scroll_x
            .checked_add(self.width)
            .ok_or(PaintError::Overflow)?;
        let view_bottom = self
            .scroll_y
            .checked_add(self.height)
            .ok_or(PaintError::Overflow)?;
        let left = rect.x.max(self.scroll_x);
        let top = rect.y.max(self.scroll_y);
        let right = rect.right().ok_or(PaintError::Overflow)?.min(view_right);
        let bottom = rect.bottom().ok_or(PaintError::Overflow)?.min(view_bottom);
        if left >= right || top >= bottom {
            return Ok(());
        }
        for y in top..bottom {
            let row = (y - self.scroll_y) as usize * self.width as usize;
            for x in left..right {
                self.pixels[row + (x - self.scroll_x) as usize] = color;
            }
        }
        Ok(())
    }
}

fn contains(outer: Rect, inner: Rect) -> Result<bool, PaintError> {
    Ok(inner.x >= outer.x
        && inner.y >= outer.y
        && inner.right().ok_or(PaintError::Overflow)?
            <= outer.right().ok_or(PaintError::Overflow)?
        && inner.bottom().ok_or(PaintError::Overflow)?
            <= outer.bottom().ok_or(PaintError::Overflow)?)
}

fn paint_node(
    index: usize,
    children: &[Vec<usize>],
    frame: &Frame,
    nodes: &[PaintNode],
    canvas: &mut Canvas,
    ancestor_clip: Option<Rect>,
) -> Result<(), PaintError> {
    if frame.contents[index] && frame.boxes[index].is_some() {
        return Err(PaintError::InvalidBox { node: index });
    }
    let Some(geometry) = frame.boxes[index] else {
        // A boxless display:contents wrapper retains its children in normal
        // painting order. It has no own background, border, padding clip,
        // or principal box to paint, even when caller paint metadata exists.
        if frame.contents[index] {
            for &child in &children[index] {
                paint_node(child, children, frame, nodes, canvas, ancestor_clip)?;
            }
        }
        return Ok(());
    };
    if !contains(geometry.margin, geometry.border)?
        || !contains(geometry.border, geometry.padding)?
        || !contains(geometry.padding, geometry.content)?
    {
        return Err(PaintError::InvalidBox { node: index });
    }

    let border = geometry.border;
    let padding = geometry.padding;
    let border_right = border.right().ok_or(PaintError::Overflow)?;
    let border_bottom = border.bottom().ok_or(PaintError::Overflow)?;
    let padding_right = padding.right().ok_or(PaintError::Overflow)?;
    let padding_bottom = padding.bottom().ok_or(PaintError::Overflow)?;

    if frame.visibility[index]
        && let Some(color) = nodes[index].background
    {
        canvas.fill(padding, color, ancestor_clip)?;
    }
    if frame.visibility[index]
        && let Some(color) = nodes[index].border
    {
        canvas.fill(
            Rect {
                x: border.x,
                y: border.y,
                width: border.width,
                height: padding.y - border.y,
            },
            color,
            ancestor_clip,
        )?;
        canvas.fill(
            Rect {
                x: border.x,
                y: padding_bottom,
                width: border.width,
                height: border_bottom - padding_bottom,
            },
            color,
            ancestor_clip,
        )?;
        canvas.fill(
            Rect {
                x: border.x,
                y: padding.y,
                width: padding.x - border.x,
                height: padding.height,
            },
            color,
            ancestor_clip,
        )?;
        canvas.fill(
            Rect {
                x: padding_right,
                y: padding.y,
                width: border_right - padding_right,
                height: padding.height,
            },
            color,
            ancestor_clip,
        )?;
    }
    // The box's own background and border are not affected by its *own*
    // descendant clip; only its children inherit the clip at its padding edge.
    // Intersecting with a prior ancestor clip never permits a grandchild to
    // escape an older clipping boundary.
    let child_clip = if frame.clip_descendants[index] {
        Some(if let Some(previous) = ancestor_clip {
            previous
                .intersection(geometry.padding)
                .ok_or(PaintError::Overflow)?
        } else {
            geometry.padding
        })
    } else {
        ancestor_clip
    };
    // Explicit depth-first sibling order, even if the flat input is not preorder.
    // This is a toy painter's order, not an implementation of CSS stacking contexts.
    for &child in &children[index] {
        paint_node(child, children, frame, nodes, canvas, child_clip)?;
    }
    Ok(())
}

/// Fill already-calculated boxes in depth-first tree order with viewport clipping.
///
/// Active display:contents wrappers contribute no paint box, but their
/// children remain in depth-first paint order with the inherited ancestor
/// clip. Transparent/absent paints leave earlier pixels intact. Resolved hidden
/// elements suppress their own paint, not the paint of an explicitly visible
/// descendant; sibling flow and box geometry are unaffected.
/// Overflowing children can draw beyond their parents' borders unless the
/// caller enabled padding-box descendant clipping; ancestor clips intersect
/// and are also clipped to the output viewport.
/// No glyphs, images, alpha compositing, positioning or CSS stacking contexts.
pub fn paint(frame: &Frame, nodes: &[PaintNode], clear: Rgb) -> Result<Canvas, PaintError> {
    paint_scrolled(frame, nodes, clear, 0, 0)
}

/// Paint a viewport at an explicitly supplied document-space scroll offset.
///
/// No scrollbars or nested scrolling contexts are implemented. The offset must
/// fit the layout frame's scroll extents; caller UI chooses how to change it.
/// Coordinates passed to Canvas::blit_mono remain in document space.
pub fn paint_scrolled(
    frame: &Frame,
    nodes: &[PaintNode],
    clear: Rgb,
    scroll_x: u32,
    scroll_y: u32,
) -> Result<Canvas, PaintError> {
    let width = frame.viewport_width;
    let height = frame.viewport_height;
    if width == 0 || height == 0 || width > MAX_VIEWPORT || height > MAX_VIEWPORT {
        return Err(PaintError::InvalidViewport);
    }
    let max_x = frame
        .scroll_width
        .checked_sub(width)
        .ok_or(PaintError::InvalidScrollOffset)?;
    let max_y = frame
        .scroll_height
        .checked_sub(height)
        .ok_or(PaintError::InvalidScrollOffset)?;
    if scroll_x > max_x || scroll_y > max_y {
        return Err(PaintError::InvalidScrollOffset);
    }
    let pixel_count = (width as usize)
        .checked_mul(height as usize)
        .ok_or(PaintError::Overflow)?;
    if pixel_count > MAX_CANVAS_PIXELS {
        return Err(PaintError::TooManyPixels);
    }
    if nodes.is_empty()
        || nodes.len() > MAX_BLOCKS
        || nodes.len() != frame.boxes.len()
        || frame.visibility.len() != frame.boxes.len()
        || frame.clip_descendants.len() != frame.boxes.len()
        || frame.contents.len() != frame.boxes.len()
    {
        return Err(PaintError::NodeCountMismatch);
    }
    if nodes[0].parent.is_some() {
        return Err(PaintError::InvalidParent { node: 0 });
    }

    let mut children = vec![Vec::new(); nodes.len()];
    let mut depths = vec![0usize; nodes.len()];
    for (index, node) in nodes.iter().enumerate().skip(1) {
        let Some(parent) = node.parent else {
            return Err(PaintError::InvalidParent { node: index });
        };
        if parent >= index {
            return Err(PaintError::InvalidParent { node: index });
        }
        depths[index] = depths[parent] + 1;
        if depths[index] > MAX_DEPTH {
            return Err(PaintError::TooDeep { node: index });
        }
        if frame.boxes[index].is_some() && frame.boxes[parent].is_none() && !frame.contents[parent]
        {
            return Err(PaintError::VisibleChildOfHiddenParent { node: index });
        }
        children[parent].push(index);
    }

    let mut canvas = Canvas {
        width,
        height,
        scroll_x,
        scroll_y,
        pixels: vec![clear; pixel_count],
    };
    paint_node(0, &children, frame, nodes, &mut canvas, None)?;
    Ok(canvas)
}

#[cfg(test)]
mod tests;
