//! Shared bounded tree validation, paint order and original-ancestry clips.
//! Eligibility and rendering remain the responsibility of each consumer.

use crate::{Frame, MAX_DEPTH, Rect};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum TraversalError {
    NodeCountMismatch,
    InvalidParent { node: usize },
    TooDeep { node: usize },
    VisibleChildOfHiddenParent { node: usize },
    InvalidBox { node: usize },
    Overflow,
}

#[derive(Clone, Copy)]
pub(crate) struct PaintEntry {
    pub node: usize,
    pub ancestor_clip: Option<Rect>,
}

pub(crate) struct PaintTree {
    children: Vec<Vec<usize>>,
}

fn encloses(outer: Rect, inner: Rect) -> Result<bool, TraversalError> {
    Ok(inner.x >= outer.x
        && inner.y >= outer.y
        && inner.right().ok_or(TraversalError::Overflow)?
            <= outer.right().ok_or(TraversalError::Overflow)?
        && inner.bottom().ok_or(TraversalError::Overflow)?
            <= outer.bottom().ok_or(TraversalError::Overflow)?)
}

pub(crate) fn validate_box(frame: &Frame, node: usize) -> Result<(), TraversalError> {
    if frame.contents[node] && frame.boxes[node].is_some() {
        return Err(TraversalError::InvalidBox { node });
    }
    if let Some(geometry) = frame.boxes[node]
        && (!encloses(geometry.margin, geometry.border)?
            || !encloses(geometry.border, geometry.padding)?
            || !encloses(geometry.padding, geometry.content)?)
    {
        return Err(TraversalError::InvalidBox { node });
    }
    Ok(())
}

impl PaintTree {
    // Consumer-specific viewport, allocation and metadata checks stay at the
    // public entry points. Both consumers validate the same original tree.
    pub fn new(
        frame: &Frame,
        parents: impl ExactSizeIterator<Item = Option<usize>>,
    ) -> Result<Self, TraversalError> {
        if parents.len() != frame.boxes.len() || frame.boxes.is_empty() {
            return Err(TraversalError::NodeCountMismatch);
        }
        let mut children = vec![Vec::new(); parents.len()];
        let mut depths = vec![0usize; parents.len()];
        for (node, parent) in parents.enumerate() {
            if node == 0 {
                if parent.is_some() {
                    return Err(TraversalError::InvalidParent { node });
                }
                continue;
            }
            let Some(parent) = parent else {
                return Err(TraversalError::InvalidParent { node });
            };
            if parent >= node {
                return Err(TraversalError::InvalidParent { node });
            }
            depths[node] = depths[parent] + 1;
            if depths[node] > MAX_DEPTH {
                return Err(TraversalError::TooDeep { node });
            }
            if frame.boxes[node].is_some()
                && frame.boxes[parent].is_none()
                && !frame.contents[parent]
            {
                return Err(TraversalError::VisibleChildOfHiddenParent { node });
            }
            children[parent].push(node);
        }
        Ok(Self { children })
    }

    // An eager plan keeps reverse hit selection consistent with forward paint.
    // Paint validates geometry in traversal order; hit testing validates the
    // complete frame first, even for off-viewport queries. The callback keeps
    // those existing diagnostic orders without duplicating traversal or clips.
    pub fn plan(
        &self,
        frame: &Frame,
        mut before_node: impl FnMut(usize) -> Result<(), TraversalError>,
    ) -> Result<Vec<PaintEntry>, TraversalError> {
        let mut order = Vec::with_capacity(frame.boxes.len());
        let mut pending = vec![PaintEntry {
            node: 0,
            ancestor_clip: None,
        }];
        while let Some(entry) = pending.pop() {
            let node = entry.node;
            before_node(node)?;
            let Some(geometry) = frame.boxes[node] else {
                if frame.contents[node] {
                    for &child in self.children[node].iter().rev() {
                        pending.push(PaintEntry {
                            node: child,
                            ..entry
                        });
                    }
                }
                continue;
            };
            order.push(entry);
            // A node's own clip affects its descendants, not its own border.
            // Empty intersections remain active so descendants cannot escape.
            let ancestor_clip = if frame.clip_descendants[node] {
                Some(if let Some(ancestor) = entry.ancestor_clip {
                    ancestor
                        .intersection(geometry.padding)
                        .ok_or(TraversalError::Overflow)?
                } else {
                    geometry.padding
                })
            } else {
                entry.ancestor_clip
            };
            for &child in self.children[node].iter().rev() {
                pending.push(PaintEntry {
                    node: child,
                    ancestor_clip,
                });
            }
        }
        Ok(order)
    }
}
