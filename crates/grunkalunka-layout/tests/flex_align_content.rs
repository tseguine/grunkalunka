//! Literal line-packing geometry and shared paint/input/semantic consequences.
use grunkalunka_layout::hit_test::{HitNode, hit_test_scrolled};
use grunkalunka_layout::paint::{PaintNode, Rgb, paint_scrolled};
use grunkalunka_layout::semantic::{Element, ElementKind, layout_semantic};
use grunkalunka_layout::{
    Block, DescendantClip, Display, Edges, FlexAlign, FlexAlignContent, FlexWrap, HeightBound,
    LayoutError, Length, MAX_BLOCKS, MAX_DEPTH, Rect, Style, Visibility, layout,
};

fn rect(x: u32, y: u32, width: u32, height: u32) -> Rect {
    Rect {
        x,
        y,
        width,
        height,
    }
}

fn row(parent: Option<usize>, width: u32, height: u32, mode: FlexAlignContent) -> Block {
    Block {
        parent,
        style: Style {
            display: Display::FlexRow,
            flex_wrap: FlexWrap::Wrap,
            width: Length::Px(width),
            height: Length::Px(height),
            align_content: mode,
            ..Style::default()
        },
        ..Block::default()
    }
}

fn item(parent: usize, width: u32, height: u32) -> Block {
    Block {
        parent: Some(parent),
        intrinsic_height: height,
        style: Style {
            width: Length::Px(width),
            ..Style::default()
        },
    }
}

#[test]
fn line_packing_preserves_box_edges_gaps_item_alignment_and_following_flow() {
    assert_eq!(Style::default().align_content, FlexAlignContent::Start);
    let mut blocks = [
        Block::default(),
        row(Some(0), 60, 100, FlexAlignContent::Start),
        item(1, 24, 8),
        item(1, 26, 10),
        item(1, 40, 20),
        item(1, 30, 6),
        Block::child(0, 2),
    ];
    blocks[1].style.margin = Edges {
        top: 2,
        bottom: 3,
        left: 1,
        right: 0,
    };
    blocks[1].style.border = Edges::all(1);
    blocks[1].style.padding = Edges::all(2);
    blocks[1].style.column_gap = 2;
    blocks[1].style.row_gap = 4;
    blocks[1].style.align_items = FlexAlign::SafeEnd;
    blocks[2].style.margin = Edges::all(1);
    blocks[2].style.border = Edges::all(1);
    blocks[2].style.padding = Edges::all(1);
    blocks[3].style.align_self = Some(FlexAlign::SafeCenter);

    // Three natural lines are 14, 20 and 6 pixels tall, plus two 4px gaps.
    for (mode, [first_y, second_y, third_y, fourth_y]) in [
        (FlexAlignContent::Start, [5, 7, 23, 47]),
        (FlexAlignContent::SafeEnd, [57, 59, 75, 99]),
        (FlexAlignContent::SafeCenter, [31, 33, 49, 73]),
        (FlexAlignContent::SpaceBetween, [5, 7, 49, 99]),
        (FlexAlignContent::SpaceEvenly, [18, 20, 49, 86]),
    ] {
        blocks[1].style.align_content = mode;
        let frame = layout(&blocks, 100, 120).unwrap();
        let first = frame.boxes[2].unwrap();
        assert_eq!(first.margin, rect(4, first_y, 30, 14), "{mode:?}");
        assert_eq!(first.border, rect(5, first_y + 1, 28, 12));
        assert_eq!(first.padding, rect(6, first_y + 2, 26, 10));
        assert_eq!(first.content, rect(7, first_y + 3, 24, 8));
        assert_eq!(frame.boxes[3].unwrap().content, rect(36, second_y, 26, 10));
        assert_eq!(frame.boxes[4].unwrap().content, rect(4, third_y, 40, 20));
        assert_eq!(frame.boxes[5].unwrap().content, rect(4, fourth_y, 30, 6));
        assert_eq!(frame.boxes[1].unwrap().content, rect(4, 5, 60, 100));
        assert_eq!(frame.boxes[6].unwrap().content, rect(0, 111, 100, 2));
    }
}

#[test]
fn distributed_line_spacing_uses_outer_gaps_and_retains_safe_overflow() {
    let mut blocks = [
        row(None, 20, 94, FlexAlignContent::SpaceAround),
        item(0, 20, 10),
        item(0, 20, 10),
        item(0, 20, 10),
    ];
    blocks[0].style.row_gap = 2;
    // Three natural 10px lines and two 2px gaps use 34px, leaving 60px.
    for (mode, expected) in [
        (FlexAlignContent::SpaceAround, [10, 42, 74]),
        (FlexAlignContent::SpaceEvenly, [15, 42, 69]),
    ] {
        blocks[0].style.align_content = mode;
        let frame = layout(&blocks, 20, 110).unwrap();
        assert_eq!(frame.boxes[0].unwrap().content, rect(0, 0, 20, 94));
        for (node, y) in expected.into_iter().enumerate() {
            assert_eq!(
                frame.boxes[node + 1].unwrap().content,
                rect(0, y, 20, 10),
                "{mode:?}, item {node}"
            );
        }
    }

    // A fractional outer interval is unsupported, rather than rounded.
    blocks[0].style.height = Length::Px(95);
    for mode in [FlexAlignContent::SpaceAround, FlexAlignContent::SpaceEvenly] {
        blocks[0].style.align_content = mode;
        assert_eq!(
            layout(&blocks, 20, 110),
            Err(LayoutError::FractionalFlexAlignContent { node: 0 })
        );
    }

    // An overflowing line group falls back to start before divisibility tests.
    blocks[0].style.height = Length::Px(33);
    for mode in [FlexAlignContent::SpaceAround, FlexAlignContent::SpaceEvenly] {
        blocks[0].style.align_content = mode;
        let frame = layout(&blocks, 20, 110).unwrap();
        for (node, y) in [0, 12, 24].into_iter().enumerate() {
            assert_eq!(frame.boxes[node + 1].unwrap().content.y, y);
        }
    }
    // align-content never changes the single formatting line of NoWrap.
    blocks[0].style.flex_wrap = FlexWrap::NoWrap;
    blocks[0].style.height = Length::Px(95);
    for mode in [FlexAlignContent::SpaceAround, FlexAlignContent::SpaceEvenly] {
        blocks[0].style.align_content = mode;
        let frame = layout(&blocks, 20, 110).unwrap();
        for item in frame.boxes.iter().take(4).skip(1) {
            assert_eq!(item.as_ref().unwrap().content.y, 0);
        }
    }
}

#[test]
fn one_wrapped_line_centers_with_around_and_evenly_but_empty_rows_do_not_fail() {
    let mut blocks = [
        row(None, 20, 51, FlexAlignContent::SpaceAround),
        item(0, 10, 11),
    ];
    // A single actual wrapped line gets equal free space on both sides.
    for mode in [FlexAlignContent::SpaceAround, FlexAlignContent::SpaceEvenly] {
        blocks[0].style.align_content = mode;
        assert_eq!(
            layout(&blocks, 20, 60).unwrap().boxes[1].unwrap().content.y,
            20
        );
        blocks[0].style.height = Length::Px(50);
        assert_eq!(
            layout(&blocks, 20, 60),
            Err(LayoutError::FractionalFlexAlignContent { node: 0 })
        );
        blocks[0].style.height = Length::Px(51);
    }
    blocks[1].style.display = Display::None;
    blocks[0].style.height = Length::Px(1);
    for mode in [FlexAlignContent::SpaceAround, FlexAlignContent::SpaceEvenly] {
        blocks[0].style.align_content = mode;
        let frame = layout(&blocks, 20, 60).unwrap();
        assert!(frame.boxes[1].is_none());
        assert_eq!(frame.boxes[0].unwrap().content.height, 1);
    }
}

#[test]
fn wrap_mode_not_actual_line_count_controls_packing_and_empty_rows_ignore_it() {
    let mut blocks = [
        row(None, 60, 51, FlexAlignContent::SafeEnd),
        item(0, 10, 11),
    ];
    assert_eq!(
        layout(&blocks, 60, 60).unwrap().boxes[1].unwrap().content.y,
        40
    );
    blocks[0].style.align_content = FlexAlignContent::SafeCenter;
    assert_eq!(
        layout(&blocks, 60, 60).unwrap().boxes[1].unwrap().content.y,
        20
    );
    blocks[0].style.align_content = FlexAlignContent::SpaceBetween;
    assert_eq!(
        layout(&blocks, 60, 60).unwrap().boxes[1].unwrap().content.y,
        0
    );

    blocks[0].style.height = Length::Px(50);
    blocks[0].style.align_content = FlexAlignContent::SafeCenter;
    assert_eq!(
        layout(&blocks, 60, 60),
        Err(LayoutError::FractionalFlexAlignContent { node: 0 })
    );
    blocks[0].style.flex_wrap = FlexWrap::NoWrap;
    assert_eq!(
        layout(&blocks, 60, 60).unwrap().boxes[1].unwrap().content.y,
        0
    );
    blocks[0].style.display = Display::Block;
    blocks[0].style.flex_wrap = FlexWrap::Wrap;
    assert_eq!(
        layout(&blocks, 60, 60).unwrap().boxes[1].unwrap().content.y,
        0
    );
    blocks[0].style.display = Display::Contents;
    assert_eq!(
        layout(&blocks, 60, 60).unwrap().boxes[1].unwrap().content.y,
        0
    );

    let mut empty = [
        row(None, 60, 51, FlexAlignContent::SafeCenter),
        item(0, 10, 11),
    ];
    empty[1].style.display = Display::None;
    let frame = layout(&empty, 60, 60).unwrap();
    assert!(frame.boxes[1].is_none());
    assert_eq!(frame.boxes[0].unwrap().content.height, 51);
    empty[0].style.row_gap = u32::MAX;
    empty[1].style.display = Display::Block;
    assert_eq!(
        layout(&empty, 60, 60).unwrap().boxes[1].unwrap().content.y,
        20
    );
    empty[0].style.align_content = FlexAlignContent::Start;
    assert_eq!(
        layout(&empty, 60, 60).unwrap().boxes[1].unwrap().content.y,
        0
    );
}

#[test]
fn safe_overflow_precedes_fraction_checks_and_zero_height_lines_still_count() {
    let mut blocks = [
        row(None, 30, 24, FlexAlignContent::Start),
        item(0, 20, 5),
        item(0, 20, 7),
        item(0, 20, 9),
    ];
    blocks[0].style.row_gap = 2;
    for height in [24, 25] {
        blocks[0].style.height = Length::Px(height);
        for mode in [
            FlexAlignContent::SafeEnd,
            FlexAlignContent::SafeCenter,
            FlexAlignContent::SpaceBetween,
        ] {
            blocks[0].style.align_content = mode;
            let frame = layout(&blocks, 30, 30).unwrap();
            for (node, y) in [(1, 0), (2, 7), (3, 16)] {
                assert_eq!(
                    frame.boxes[node].unwrap().content.y,
                    y,
                    "{height}, {mode:?}"
                );
            }
        }
    }
    blocks[0].style.height = Length::Px(26);
    for mode in [FlexAlignContent::SafeCenter, FlexAlignContent::SpaceBetween] {
        blocks[0].style.align_content = mode;
        assert_eq!(
            layout(&blocks, 30, 30),
            Err(LayoutError::FractionalFlexAlignContent { node: 0 })
        );
    }
    blocks[0].style.align_content = FlexAlignContent::SafeEnd;
    let frame = layout(&blocks, 30, 30).unwrap();
    assert_eq!(frame.boxes[1].unwrap().content.y, 1);
    assert_eq!(frame.boxes[3].unwrap().content.y, 17);

    blocks[0].style.height = Length::Px(4);
    blocks[0].style.row_gap = 0;
    blocks[0].style.align_content = FlexAlignContent::SpaceBetween;
    for block in &mut blocks[1..] {
        block.intrinsic_height = 0;
    }
    let frame = layout(&blocks, 30, 10).unwrap();
    for (node, y) in [(1, 0), (2, 2), (3, 4)] {
        assert_eq!(frame.boxes[node].unwrap().content.y, y);
    }
    blocks[0].style.height = Length::Px(3);
    assert_eq!(
        layout(&blocks, 30, 10),
        Err(LayoutError::FractionalFlexAlignContent { node: 0 })
    );
}

#[test]
fn line_packing_uses_finished_height_without_changing_percentage_definiteness() {
    let mut blocks = [
        row(None, 40, 80, FlexAlignContent::SafeEnd),
        item(0, 40, 10),
        item(0, 40, 20),
        item(1, 20, 5),
    ];
    blocks[0].style.height = Length::Auto;
    blocks[0].style.min_height = Some(HeightBound::Px(80));
    blocks[0].style.max_height = Some(HeightBound::Px(30));
    blocks[0].style.row_gap = 4;
    blocks[1].style.width = Length::Percent(100);
    blocks[1].style.relative_offset.y = 8;
    let frame = layout(&blocks, 40, 160).unwrap();
    assert_eq!(frame.boxes[0].unwrap().content.height, 80);
    assert_eq!(frame.boxes[1].unwrap().content, rect(0, 54, 40, 10));
    assert_eq!(frame.boxes[2].unwrap().content, rect(0, 60, 40, 20));
    assert_eq!(frame.boxes[3].unwrap().content, rect(0, 54, 20, 5));

    blocks[1].style.height = Length::Percent(25);
    assert_eq!(
        layout(&blocks, 40, 160),
        Err(LayoutError::UnsupportedPercentageHeight { node: 1 })
    );
    blocks[0].style.height = Length::Percent(50);
    let frame = layout(&blocks, 40, 160).unwrap();
    assert_eq!(frame.boxes[1].unwrap().content, rect(0, 44, 40, 20));
    assert_eq!(frame.boxes[2].unwrap().content, rect(0, 60, 40, 20));
    assert_eq!(frame.boxes[3].unwrap().content, rect(0, 44, 20, 5));
}

#[test]
fn nested_rows_and_boxless_tree_order_compose_independent_line_offsets() {
    let mut blocks = [
        row(None, 60, 100, FlexAlignContent::SafeCenter),
        Block::child(0, 0),
        item(1, 30, 10),
        row(Some(0), 40, 40, FlexAlignContent::SafeCenter),
        item(3, 30, 8),
        item(1, 30, 20),
        item(3, 30, 8),
        item(2, 20, 2),
    ];
    blocks[0].style.row_gap = 10;
    blocks[0].style.align_items = FlexAlign::SafeEnd;
    blocks[1].style.display = Display::Contents;
    blocks[1].style.align_content = FlexAlignContent::SafeEnd;
    blocks[1].style.row_gap = u32::MAX;
    blocks[3].style.row_gap = 4;
    blocks[5].style.visibility = Visibility::Hidden;
    blocks[7].style.align_content = FlexAlignContent::SafeEnd;
    let frame = layout(&blocks, 60, 110).unwrap();
    for (node, expected) in [
        (2, rect(0, 25, 30, 10)),
        (5, rect(30, 15, 30, 20)),
        (3, rect(0, 45, 40, 40)),
        (4, rect(0, 55, 30, 8)),
        (6, rect(0, 67, 30, 8)),
        (7, rect(0, 25, 20, 2)),
    ] {
        assert_eq!(frame.boxes[node].unwrap().content, expected, "node {node}");
    }
    assert!(frame.boxes[1].is_none());
    assert!(!frame.visibility[5]);

    blocks[5].style.display = Display::None;
    let frame = layout(&blocks, 60, 110).unwrap();
    assert_eq!(frame.boxes[2].unwrap().content.y, 20);
    assert_eq!(frame.boxes[3].unwrap().content.y, 40);
    assert_eq!(frame.boxes[4].unwrap().content.y, 50);
    assert_eq!(frame.boxes[6].unwrap().content.y, 62);
}

#[test]
fn projected_lines_share_scrolled_paint_hit_and_padding_clip_coordinates() {
    let mut blocks = [
        row(None, 20, 40, FlexAlignContent::SafeEnd),
        item(0, 20, 6),
        item(0, 20, 6),
    ];
    blocks[0].style.row_gap = 4;
    blocks[0].style.border = Edges::all(1);
    blocks[0].style.padding = Edges::all(2);
    blocks[0].style.descendant_clip = DescendantClip::PaddingBox;
    blocks[2].style.relative_offset.y = 6;
    let frame = layout(&blocks, 26, 28).unwrap();
    assert_eq!(frame.boxes[1].unwrap().content, rect(3, 27, 20, 6));
    assert_eq!(frame.boxes[2].unwrap().content, rect(3, 43, 20, 6));
    assert_eq!((frame.scroll_width, frame.scroll_height), (26, 49));
    let blue = Rgb::new(20, 40, 220);
    let green = Rgb::new(20, 190, 30);
    let hits: Vec<_> = blocks.iter().map(|b| HitNode::new(b.parent)).collect();
    let paints: Vec<_> = blocks
        .iter()
        .zip([Rgb::WHITE, blue, green])
        .map(|(b, color)| PaintNode {
            parent: b.parent,
            background: Some(color),
            border: Some(Rgb::BLACK),
        })
        .collect();
    let canvas = paint_scrolled(&frame, &paints, Rgb::WHITE, 0, 20).unwrap();
    for ((x, y), color, target) in [
        ((4, 8), blue, Some(1)),
        ((4, 23), green, Some(2)),
        ((4, 24), green, Some(2)),
        ((4, 25), Rgb::BLACK, Some(0)),
        ((4, 26), Rgb::WHITE, None),
    ] {
        assert_eq!(canvas.pixel(x, y), Some(color));
        assert_eq!(hit_test_scrolled(&frame, &hits, x, y, 0, 20), Ok(target));
    }
}

#[test]
fn semantic_text_anchors_follow_line_packing_and_hidden_item_reflow() {
    let mut elements = [
        Element::new(None, ElementKind::Body, "", 0),
        Element::new(Some(0), ElementKind::Paragraph, "one", 6),
        Element::new(Some(0), ElementKind::Paragraph, "two", 10),
    ];
    let mut root = row(None, 20, 40, FlexAlignContent::SafeEnd);
    root.style.row_gap = 4;
    elements[0].resolved_style = Some(root.style);
    elements[1].resolved_style = Some(item(0, 20, 0).style);
    elements[2].resolved_style = Some(item(0, 20, 0).style);
    let end = layout_semantic(&elements, 20, 40).unwrap();
    assert_eq!(end.text_boxes[0].content, rect(0, 20, 20, 6));
    assert_eq!(end.text_boxes[1].content, rect(0, 30, 20, 10));
    root.style.align_content = FlexAlignContent::SafeCenter;
    elements[0].resolved_style = Some(root.style);
    let centered = layout_semantic(&elements, 20, 40).unwrap();
    assert_eq!(centered.text_boxes[0].content, rect(0, 10, 20, 6));
    assert_eq!(centered.text_boxes[1].content, rect(0, 20, 20, 10));
    elements[2].hidden = true;
    let hidden = layout_semantic(&elements, 20, 40).unwrap();
    assert_eq!(hidden.text_boxes.len(), 1);
    assert_eq!(hidden.text_boxes[0].content, rect(0, 17, 20, 6));
}

#[test]
fn line_packing_retains_node_depth_and_coordinate_bounds_on_a_small_stack() {
    std::thread::Builder::new()
        .stack_size(2 * 1024 * 1024)
        .spawn(|| {
            let mut deep = vec![row(
                None,
                1,
                (2 * MAX_DEPTH + 1) as u32,
                FlexAlignContent::SafeEnd,
            )];
            for level in 1..=MAX_DEPTH {
                let mut child = row(
                    Some(level - 1),
                    1,
                    (2 * (MAX_DEPTH - level) + 1) as u32,
                    FlexAlignContent::SafeEnd,
                );
                if level == MAX_DEPTH {
                    child.style.display = Display::Block;
                }
                deep.push(child);
            }
            let frame = layout(&deep, 1, 1).unwrap();
            assert_eq!(frame.boxes[MAX_DEPTH].unwrap().content, rect(0, 128, 1, 1));
            deep.push(item(MAX_DEPTH, 1, 1));
            assert_eq!(
                layout(&deep, 1, 1),
                Err(LayoutError::TooDeep {
                    node: MAX_DEPTH + 1
                })
            );

            let mut wide = vec![row(
                None,
                0,
                (MAX_BLOCKS - 2) as u32,
                FlexAlignContent::SpaceBetween,
            )];
            wide.extend((1..MAX_BLOCKS).map(|_| item(0, 1, 0)));
            let frame = layout(&wide, 1, 1).unwrap();
            assert_eq!(
                frame.boxes[MAX_BLOCKS - 1].unwrap().content,
                rect(0, (MAX_BLOCKS - 2) as u32, 1, 0)
            );
            wide.push(item(0, 1, 0));
            assert_eq!(layout(&wide, 1, 1), Err(LayoutError::TooManyBlocks));

            let mut overflow = [row(None, 1, 2, FlexAlignContent::SafeEnd), item(0, 1, 1)];
            overflow[1].style.relative_offset.y = u32::MAX - 1;
            assert_eq!(layout(&overflow, 1, 2), Err(LayoutError::Overflow));
        })
        .unwrap()
        .join()
        .unwrap();
}
