//! Independent exact hit targets for the headless block tree.
//! No browser event dispatcher or painted pixel buffer supplies expectations.

use grunkalunka_layout::hit_test::{HitError, HitNode, hit_test};
use grunkalunka_layout::{
    Block, Display, Edges, Frame, LayoutError, Length, MAX_BLOCKS, MAX_DEPTH, Style, layout,
};

fn overlapping() -> (Frame, [HitNode; 4]) {
    let blocks = [
        Block {
            style: Style {
                height: Length::Px(20),
                ..Style::default()
            },
            ..Block::default()
        },
        Block {
            parent: Some(0),
            style: Style {
                height: Length::Px(8),
                ..Style::default()
            },
            ..Block::default()
        },
        Block {
            parent: Some(0),
            intrinsic_height: 8,
            ..Block::default()
        },
        Block::child(1, 20),
    ];
    (
        layout(&blocks, 10, 20).unwrap(),
        [
            HitNode::new(None),
            HitNode::new(Some(0)),
            HitNode::new(Some(0)),
            HitNode::new(Some(1)),
        ],
    )
}

#[test]
fn reverse_paint_order_is_based_on_the_tree_not_flat_indices() {
    let (frame, nodes) = overlapping();
    // Paint order: root 0, child 1, grandchild 3, then sibling 2.
    assert_eq!(hit_test(&frame, &nodes, 1, 1), Ok(Some(3)));
    assert_eq!(hit_test(&frame, &nodes, 1, 10), Ok(Some(2)));
    assert_eq!(hit_test(&frame, &nodes, 1, 17), Ok(Some(3)));
    assert_eq!(hit_test(&frame, &nodes, 10, 1), Ok(None));
    assert_eq!(hit_test(&frame, &nodes, 1, 20), Ok(None));
}

#[test]
fn untargetable_elements_do_not_occlude_targetable_descendants_or_ancestors() {
    let (frame, mut nodes) = overlapping();
    nodes[2].targetable = false;
    assert_eq!(hit_test(&frame, &nodes, 1, 10), Ok(Some(3)));
    nodes[3].targetable = false;
    assert_eq!(hit_test(&frame, &nodes, 1, 10), Ok(Some(0)));
    assert_eq!(hit_test(&frame, &nodes, 1, 1), Ok(Some(1)));
    nodes[1].targetable = false;
    nodes[0].targetable = false;
    assert_eq!(hit_test(&frame, &nodes, 1, 1), Ok(None));
}

#[test]
fn border_is_hittable_but_margin_is_not() {
    let blocks = [
        Block {
            style: Style {
                height: Length::Px(10),
                ..Style::default()
            },
            ..Block::default()
        },
        Block {
            parent: Some(0),
            style: Style {
                width: Length::Px(5),
                height: Length::Px(5),
                margin: Edges {
                    top: 2,
                    left: 2,
                    ..Edges::default()
                },
                border: Edges::all(1),
                ..Style::default()
            },
            ..Block::default()
        },
    ];
    let frame = layout(&blocks, 10, 10).unwrap();
    let nodes = [HitNode::new(None), HitNode::new(Some(0))];
    assert_eq!(hit_test(&frame, &nodes, 1, 3), Ok(Some(0)));
    assert_eq!(hit_test(&frame, &nodes, 2, 2), Ok(Some(1)));
    assert_eq!(hit_test(&frame, &nodes, 8, 8), Ok(Some(1)));
    assert_eq!(hit_test(&frame, &nodes, 9, 2), Ok(Some(0)));
}

#[test]
fn hidden_subtrees_do_not_hit_and_later_siblings_reflow() {
    let blocks = [
        Block {
            style: Style {
                height: Length::Px(20),
                ..Style::default()
            },
            ..Block::default()
        },
        Block {
            parent: Some(0),
            style: Style {
                display: Display::None,
                ..Style::default()
            },
            ..Block::default()
        },
        Block::child(1, 30),
        Block::child(0, 7),
    ];
    let frame = layout(&blocks, 10, 20).unwrap();
    assert!(frame.boxes[1].is_none() && frame.boxes[2].is_none());
    let nodes = [
        HitNode::new(None),
        HitNode::new(Some(0)),
        HitNode::new(Some(1)),
        HitNode::new(Some(0)),
    ];
    assert_eq!(hit_test(&frame, &nodes, 1, 1), Ok(Some(3)));
    assert_eq!(hit_test(&frame, &nodes, 1, 8), Ok(Some(0)));
    assert_eq!(hit_test(&frame, &nodes, 1, 19), Ok(Some(0)));
}

#[test]
fn overflow_children_can_be_hit_beyond_a_parent_border() {
    let blocks = [
        Block {
            style: Style {
                height: Length::Px(5),
                ..Style::default()
            },
            ..Block::default()
        },
        Block::child(0, 10),
    ];
    let frame = layout(&blocks, 10, 12).unwrap();
    let nodes = [HitNode::new(None), HitNode::new(Some(0))];
    assert_eq!(hit_test(&frame, &nodes, 1, 8), Ok(Some(1)));
    assert_eq!(hit_test(&frame, &nodes, 1, 11), Ok(None));
    assert_eq!(hit_test(&frame, &nodes, 1, 12), Ok(None));
}

#[test]
fn corrupt_frames_parents_depth_and_viewports_fail_closed() {
    let frame = layout(
        &[
            Block {
                intrinsic_height: 10,
                ..Block::default()
            },
            Block::child(0, 3),
        ],
        10,
        10,
    )
    .unwrap();
    let nodes = [HitNode::new(None), HitNode::new(Some(0))];
    assert_eq!(hit_test(&frame, &nodes, 1, 1), Ok(Some(1)));

    let mut invalid = frame.clone();
    invalid.viewport_width = 0;
    assert_eq!(
        hit_test(&invalid, &nodes, 1, 1),
        Err(HitError::InvalidViewport)
    );
    let mut invalid = frame.clone();
    invalid.boxes.pop();
    assert_eq!(
        hit_test(&invalid, &nodes, 1, 1),
        Err(HitError::NodeCountMismatch)
    );
    let mut invalid_nodes = nodes;
    invalid_nodes[0].parent = Some(0);
    assert_eq!(
        hit_test(&frame, &invalid_nodes, 1, 1),
        Err(HitError::InvalidParent { node: 0 })
    );
    invalid_nodes = nodes;
    invalid_nodes[1].parent = Some(1);
    assert_eq!(
        hit_test(&frame, &invalid_nodes, 1, 1),
        Err(HitError::InvalidParent { node: 1 })
    );

    let mut invalid = frame.clone();
    invalid.boxes[0] = None;
    assert_eq!(
        hit_test(&invalid, &nodes, 1, 1),
        Err(HitError::VisibleChildOfHiddenParent { node: 1 })
    );

    let mut invalid = frame.clone();
    invalid.boxes[1].as_mut().unwrap().content.x = 9;
    assert_eq!(
        hit_test(&invalid, &nodes, 1, 1),
        Err(HitError::InvalidBox { node: 1 })
    );

    let mut invalid = frame.clone();
    invalid.boxes[1].as_mut().unwrap().content.x = u32::MAX;
    assert_eq!(hit_test(&invalid, &nodes, 1, 1), Err(HitError::Overflow));
    // Check malformed geometry even for an off-viewport query.
    assert_eq!(
        hit_test(&invalid, &nodes, 1000, 1000),
        Err(HitError::Overflow)
    );

    let mut invalid = frame.clone();
    invalid.boxes = vec![None; MAX_BLOCKS + 1];
    invalid.visibility = vec![true; MAX_BLOCKS + 1];
    invalid.clip_descendants = vec![false; MAX_BLOCKS + 1];
    let many = vec![HitNode::new(None); MAX_BLOCKS + 1];
    assert_eq!(hit_test(&invalid, &many, 1, 1), Err(HitError::TooManyNodes));

    let mut invalid = frame;
    invalid.boxes = vec![None; MAX_DEPTH + 2];
    invalid.visibility = vec![true; MAX_DEPTH + 2];
    invalid.clip_descendants = vec![false; MAX_DEPTH + 2];
    let mut deep = vec![HitNode::new(None); MAX_DEPTH + 2];
    for (index, node) in deep.iter_mut().enumerate().skip(1) {
        node.parent = Some(index - 1);
    }
    assert_eq!(
        hit_test(&invalid, &deep, 1, 1),
        Err(HitError::TooDeep {
            node: MAX_DEPTH + 1
        })
    );
    assert_eq!(layout(&[], 10, 10), Err(LayoutError::EmptyDocument));
}
