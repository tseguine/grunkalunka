//! Bounded, shell-independent hit testing for already-laid-out block boxes.
//!
//! Parent-first, child-over-parent hit order mirrors the toy RGB painter's
//! depth-first traversal. This does NOT implement CSS pointer-events,
//! stacking contexts, arbitrary clipping paths, transforms, or browser event dispatch.

use crate::{Frame, MAX_BLOCKS, MAX_DEPTH, MAX_VIEWPORT, Rect};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct HitNode {
    /// Same parent index as the source layout tree.
    pub parent: Option<usize>,
    /// An independent caller-supplied input policy, not CSS visibility.
    pub targetable: bool,
}

impl HitNode {
    pub const fn new(parent: Option<usize>) -> Self {
        Self {
            parent,
            targetable: true,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum HitError {
    InvalidViewport,
    InvalidScrollOffset,
    NodeCountMismatch,
    TooManyNodes,
    InvalidParent { node: usize },
    TooDeep { node: usize },
    VisibleChildOfHiddenParent { node: usize },
    InvalidBox { node: usize },
    Overflow,
}

fn encloses(outer: Rect, inner: Rect) -> Result<bool, HitError> {
    Ok(inner.x >= outer.x
        && inner.y >= outer.y
        && inner.right().ok_or(HitError::Overflow)? <= outer.right().ok_or(HitError::Overflow)?
        && inner.bottom().ok_or(HitError::Overflow)? <= outer.bottom().ok_or(HitError::Overflow)?)
}

fn contains_point(border: Rect, x: u32, y: u32) -> Result<bool, HitError> {
    Ok(x >= border.x
        && y >= border.y
        && x < border.right().ok_or(HitError::Overflow)?
        && y < border.bottom().ok_or(HitError::Overflow)?)
}

/// Return the topmost caller-targetable element at one viewport coordinate.
///
/// The selected toy hit region is each visible element's border rectangle:
/// margins are excluded; overflow children can receive hits beyond parents.
/// Hidden elements are skipped without changing layout; descendants explicitly
/// overridden to visible can still be targeted. Descendant padding-box clips
/// restrict each target through every clipping ancestor.
/// Children cover parents and later siblings cover earlier siblings, following
/// the RGB painter's depth-first tree traversal even if a flat input is not
/// preorder. A targetable element may be visually transparent. Independent
/// caller input policy cannot override resolved inherited visibility.
///
/// The entire frame/tree is checked before any hit is accepted, including
/// bad parents, tree depth and nested box rectangles. Point coordinates beyond
/// the viewport return None rather than selecting offscreen content.
pub fn hit_test(
    frame: &Frame,
    nodes: &[HitNode],
    x: u32,
    y: u32,
) -> Result<Option<usize>, HitError> {
    hit_test_scrolled(frame, nodes, x, y, 0, 0)
}

/// Hit test a viewport point after translating it by the caller's global
/// document-space scroll offset. No nested scrolling, clipping contexts or
/// scroll-event dispatch are modeled. Invalid offsets fail instead of clamping.
pub fn hit_test_scrolled(
    frame: &Frame,
    nodes: &[HitNode],
    x: u32,
    y: u32,
    scroll_x: u32,
    scroll_y: u32,
) -> Result<Option<usize>, HitError> {
    if frame.viewport_width == 0
        || frame.viewport_height == 0
        || frame.viewport_width > MAX_VIEWPORT
        || frame.viewport_height > MAX_VIEWPORT
    {
        return Err(HitError::InvalidViewport);
    }
    if nodes.is_empty()
        || nodes.len() != frame.boxes.len()
        || frame.visibility.len() != frame.boxes.len()
        || frame.clip_descendants.len() != frame.boxes.len()
    {
        return Err(HitError::NodeCountMismatch);
    }
    if nodes.len() > MAX_BLOCKS {
        return Err(HitError::TooManyNodes);
    }
    if nodes[0].parent.is_some() {
        return Err(HitError::InvalidParent { node: 0 });
    }

    let mut children = vec![Vec::new(); nodes.len()];
    let mut depths = vec![0usize; nodes.len()];
    for (index, node) in nodes.iter().enumerate().skip(1) {
        let Some(parent) = node.parent else {
            return Err(HitError::InvalidParent { node: index });
        };
        if parent >= index {
            return Err(HitError::InvalidParent { node: index });
        }
        depths[index] = depths[parent] + 1;
        if depths[index] > MAX_DEPTH {
            return Err(HitError::TooDeep { node: index });
        }
        if frame.boxes[index].is_some() && frame.boxes[parent].is_none() {
            return Err(HitError::VisibleChildOfHiddenParent { node: index });
        }
        children[parent].push(index);
    }

    for (index, geometry) in frame.boxes.iter().enumerate() {
        if let Some(geometry) = geometry
            && (!encloses(geometry.margin, geometry.border)?
                || !encloses(geometry.border, geometry.padding)?
                || !encloses(geometry.padding, geometry.content)?)
        {
            return Err(HitError::InvalidBox { node: index });
        }
    }

    let max_x = frame
        .scroll_width
        .checked_sub(frame.viewport_width)
        .ok_or(HitError::InvalidScrollOffset)?;
    let max_y = frame
        .scroll_height
        .checked_sub(frame.viewport_height)
        .ok_or(HitError::InvalidScrollOffset)?;
    if scroll_x > max_x || scroll_y > max_y {
        return Err(HitError::InvalidScrollOffset);
    }
    if x >= frame.viewport_width || y >= frame.viewport_height {
        return Ok(None);
    }
    let x = x.checked_add(scroll_x).ok_or(HitError::Overflow)?;
    let y = y.checked_add(scroll_y).ok_or(HitError::Overflow)?;

    // Compute the painter's depth-first order and effective *ancestor* clip.
    // The node's own padding-box rule clips its children, not its own border.
    // Preserve a zero-sized intersection as an active clip so later descendants
    // cannot escape an earlier clipping ancestor.
    let mut order = Vec::with_capacity(nodes.len());
    let mut stack = vec![(0usize, None::<Rect>)];
    while let Some((index, inherited_clip)) = stack.pop() {
        order.push((index, inherited_clip));
        let child_clip = if frame.clip_descendants[index] {
            if let Some(geometry) = frame.boxes[index] {
                Some(if let Some(ancestor) = inherited_clip {
                    ancestor
                        .intersection(geometry.padding)
                        .ok_or(HitError::Overflow)?
                } else {
                    geometry.padding
                })
            } else {
                inherited_clip
            }
        } else {
            inherited_clip
        };
        for &child in children[index].iter().rev() {
            stack.push((child, child_clip));
        }
    }
    for (index, inherited_clip) in order.into_iter().rev() {
        if nodes[index].targetable
            && frame.visibility[index]
            && let Some(geometry) = frame.boxes[index]
            && contains_point(geometry.border, x, y)?
            && (if let Some(clip) = inherited_clip {
                contains_point(clip, x, y)?
            } else {
                true
            })
        {
            return Ok(Some(index));
        }
    }
    Ok(None)
}
