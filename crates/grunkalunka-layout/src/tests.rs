use super::*;

fn rect(x: u32, y: u32, width: u32, height: u32) -> Rect {
    Rect {
        x,
        y,
        width,
        height,
    }
}

#[test]
fn nested_outline_boxes_have_independent_coordinates() {
    let blocks = [
        Block {
            style: Style {
                padding: Edges::all(8),
                border: Edges::all(1),
                ..Style::default()
            },
            ..Block::default()
        },
        Block {
            parent: Some(0),
            intrinsic_height: 24,
            style: Style {
                margin: Edges {
                    bottom: 6,
                    ..Edges::default()
                },
                ..Style::default()
            },
        },
        Block {
            parent: Some(0),
            style: Style {
                margin: Edges {
                    top: 2,
                    ..Edges::default()
                },
                ..Style::default()
            },
            ..Block::default()
        },
        Block::child(2, 18),
        Block::child(2, 18),
    ];
    let frame = layout(&blocks, 320, 200).unwrap();
    assert_eq!(frame.boxes[0].unwrap().border, rect(0, 0, 320, 86));
    assert_eq!(frame.boxes[0].unwrap().content, rect(9, 9, 302, 68));
    assert_eq!(frame.boxes[1].unwrap().border, rect(9, 9, 302, 24));
    assert_eq!(frame.boxes[2].unwrap().border, rect(9, 41, 302, 36));
    assert_eq!(frame.boxes[3].unwrap().border, rect(9, 41, 302, 18));
    assert_eq!(frame.boxes[4].unwrap().border, rect(9, 59, 302, 18));
    assert_eq!((frame.scroll_width, frame.scroll_height), (320, 200));
    assert_eq!(layout(&blocks, 320, 200).unwrap(), frame);
}

#[test]
fn hiding_one_subtree_leaves_following_siblings_in_normal_flow() {
    let blocks = [
        Block::default(),
        Block {
            parent: Some(0),
            style: Style {
                display: Display::None,
                ..Style::default()
            },
            ..Block::default()
        },
        Block::child(1, 300),
        Block::child(0, 17),
    ];
    let frame = layout(&blocks, 100, 40).unwrap();
    assert!(frame.boxes[1].is_none());
    assert!(frame.boxes[2].is_none());
    assert_eq!(frame.boxes[3].unwrap().content, rect(0, 0, 100, 17));
    assert_eq!(frame.boxes[0].unwrap().border, rect(0, 0, 100, 17));
    let mut expanded = blocks;
    expanded[1].style.display = Display::Block;
    let shown = layout(&expanded, 100, 40).unwrap();
    assert_eq!(shown.boxes[3].unwrap().border.y, 300);
    assert_eq!(shown.scroll_height, 317);
}

#[test]
fn fixed_height_overflow_preserves_descendant_geometry_and_scroll_size() {
    let blocks = [
        Block {
            style: Style {
                width: Length::Px(80),
                height: Length::Px(12),
                ..Style::default()
            },
            ..Block::default()
        },
        Block {
            parent: Some(0),
            intrinsic_height: 60,
            style: Style {
                width: Length::Px(150),
                margin: Edges {
                    left: 4,
                    top: 3,
                    ..Edges::default()
                },
                ..Style::default()
            },
        },
    ];
    let frame = layout(&blocks, 100, 20).unwrap();
    assert_eq!(frame.boxes[0].unwrap().content, rect(0, 0, 80, 12));
    assert_eq!(frame.boxes[1].unwrap().border, rect(4, 3, 150, 60));
    assert_eq!((frame.scroll_width, frame.scroll_height), (154, 63));
}

#[test]
fn overconstrained_auto_width_is_zero_but_edges_remain_visible() {
    let blocks = [Block {
        style: Style {
            padding: Edges::all(8),
            border: Edges::all(1),
            ..Style::default()
        },
        ..Block::default()
    }];
    let frame = layout(&blocks, 12, 10).unwrap();
    assert_eq!(frame.boxes[0].unwrap().content, rect(9, 9, 0, 0));
    assert_eq!(frame.boxes[0].unwrap().border, rect(0, 0, 18, 18));
    assert_eq!((frame.scroll_width, frame.scroll_height), (18, 18));
}

#[test]
fn input_tree_and_resource_quotas_fail_closed() {
    assert_eq!(layout(&[], 100, 100), Err(LayoutError::EmptyDocument));
    assert_eq!(
        layout(&[Block::default()], 0, 100),
        Err(LayoutError::InvalidViewport)
    );
    assert_eq!(
        layout(&[Block::default()], MAX_VIEWPORT + 1, 100),
        Err(LayoutError::InvalidViewport)
    );
    assert_eq!(
        layout(
            &[Block {
                parent: Some(0),
                ..Block::default()
            }],
            10,
            10
        ),
        Err(LayoutError::InvalidRoot)
    );
    assert_eq!(
        layout(&[Block::default(), Block::default()], 10, 10),
        Err(LayoutError::InvalidParent { node: 1 })
    );
    assert_eq!(
        layout(&[Block::default(), Block::child(1, 0)], 10, 10),
        Err(LayoutError::InvalidParent { node: 1 })
    );
    let chain: Vec<_> = (0..=MAX_DEPTH + 1)
        .map(|i| {
            if i == 0 {
                Block::default()
            } else {
                Block::child(i - 1, 0)
            }
        })
        .collect();
    assert_eq!(
        layout(&chain, 10, 10),
        Err(LayoutError::TooDeep {
            node: MAX_DEPTH + 1
        })
    );
    assert_eq!(
        layout(&vec![Block::default(); MAX_BLOCKS + 1], 10, 10),
        Err(LayoutError::TooManyBlocks)
    );
}

#[test]
fn checked_dimensions_reject_wraparound() {
    let blocks = [Block {
        style: Style {
            width: Length::Px(u32::MAX),
            border: Edges::all(1),
            ..Style::default()
        },
        ..Block::default()
    }];
    assert_eq!(layout(&blocks, 100, 100), Err(LayoutError::Overflow));
    let blocks = [Block {
        intrinsic_height: u32::MAX,
        style: Style {
            margin: Edges {
                bottom: 1,
                ..Edges::default()
            },
            ..Style::default()
        },
        ..Block::default()
    }];
    assert_eq!(layout(&blocks, 100, 100), Err(LayoutError::Overflow));
}

#[test]
fn mixed_depth_limit_uses_heap_frames_and_keeps_natural_height() {
    std::thread::Builder::new()
        .stack_size(2 * 1024 * 1024)
        .spawn(|| {
            let mut blocks = vec![Block::default()];
            for level in 1..=MAX_DEPTH {
                let mut child = Block::child(level - 1, u32::from(level == MAX_DEPTH));
                if level % 2 == 0 && level < MAX_DEPTH {
                    child.style.display = Display::Contents;
                }
                blocks.push(child);
            }
            let frame = layout(&blocks, 10, 10).unwrap();
            assert_eq!(frame.boxes[0].unwrap().content.height, 1);
            assert_eq!(frame.boxes[MAX_DEPTH].unwrap().content.height, 1);
            blocks.push(Block::child(MAX_DEPTH, 1));
            assert_eq!(
                layout(&blocks, 10, 10),
                Err(LayoutError::TooDeep {
                    node: MAX_DEPTH + 1
                })
            );
        })
        .unwrap()
        .join()
        .unwrap();
}
