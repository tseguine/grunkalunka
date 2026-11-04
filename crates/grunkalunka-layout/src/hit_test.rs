//! Bounded, shell-independent hit testing for already-laid-out block boxes.
//!
//! Reverse shared paint order includes ordinary block traversal and selected
//! non-positioned flex-item scopes. Resolved visibility/pointer eligibility,
//! padding clips and document scrolling are supported; general CSS stacking,
//! arbitrary clipping paths, transforms and browser event dispatch are not.

use crate::paint_order::{PaintTree, TraversalError, validate_box};
use crate::{Frame, MAX_BLOCKS, MAX_VIEWPORT, Rect};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct HitNode {
    /// Same parent index as the source layout tree.
    pub parent: Option<usize>,
    /// An independent caller-supplied input policy, not CSS visibility or
    /// inherited pointer-events. All eligibility filters must agree.
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

impl From<TraversalError> for HitError {
    fn from(error: TraversalError) -> Self {
        match error {
            TraversalError::NodeCountMismatch => Self::NodeCountMismatch,
            TraversalError::InvalidParent { node } => Self::InvalidParent { node },
            TraversalError::TooDeep { node } => Self::TooDeep { node },
            TraversalError::VisibleChildOfHiddenParent { node } => {
                Self::VisibleChildOfHiddenParent { node }
            }
            TraversalError::InvalidBox { node } => Self::InvalidBox { node },
            TraversalError::Overflow => Self::Overflow,
        }
    }
}

fn contains_point(border: Rect, x: u32, y: u32) -> Result<bool, HitError> {
    Ok(x >= border.x
        && y >= border.y
        && x < border.right().ok_or(HitError::Overflow)?
        && y < border.bottom().ok_or(HitError::Overflow)?)
}

/// Return the topmost caller-targetable element at one viewport coordinate.
///
/// Active display:contents wrappers have no hit box of their own, but their
/// children retain their generated-box paint roles and ancestor clips.
/// The selected toy hit region is each visible element's border rectangle:
/// margins are excluded; overflow children can receive hits beyond parents.
/// Hidden elements are skipped without changing layout; descendants explicitly
/// overridden to visible can still be targeted. Descendant padding-box clips
/// restrict each target through every clipping ancestor.
/// Selection reverses the RGB painter's shared plan: ordinary boxes follow
/// tree order within a scope, and actual flex-item subtrees form deferred
/// scopes completed in source order. Flat input need not be preorder.
/// A targetable element may be visually transparent. Independent
/// caller input policy cannot override resolved inherited visibility or
/// pointer-events:none. An explicitly auto descendant of a none ancestor can
/// receive a hit; the parent does not become a hit target.
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
        || frame.pointer_events.len() != frame.boxes.len()
        || frame.clip_descendants.len() != frame.boxes.len()
        || frame.contents.len() != frame.boxes.len()
    {
        return Err(HitError::NodeCountMismatch);
    }
    if nodes.len() > MAX_BLOCKS {
        return Err(HitError::TooManyNodes);
    }
    let tree = PaintTree::new(frame, nodes.iter().map(|node| node.parent))?;
    for node in 0..frame.boxes.len() {
        validate_box(frame, node)?;
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

    let order = tree.plan(frame, |_| Ok(()))?;
    for entry in order.into_iter().rev() {
        let index = entry.node;
        let inherited_clip = entry.ancestor_clip;
        if nodes[index].targetable
            && frame.visibility[index]
            && frame.pointer_events[index]
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
