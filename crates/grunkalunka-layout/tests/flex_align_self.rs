//! Per-item alignment against literal geometry and the shared output consumers.
use grunkalunka_layout::hit_test::{HitNode, hit_test, hit_test_scrolled};
use grunkalunka_layout::paint::{PaintNode, Rgb, paint, paint_scrolled};
use grunkalunka_layout::semantic::{Element, ElementKind, layout_semantic};
use grunkalunka_layout::{
    Block, DescendantClip, Display, Edges, FlexAlign, FlexWrap, LayoutError, Length, MAX_BLOCKS,
    MAX_DEPTH, Rect, Style, Visibility, layout,
};

fn rect(x: u32, y: u32, width: u32, height: u32) -> Rect {
    Rect {
        x,
        y,
        width,
        height,
    }
}

fn row(parent: Option<usize>, width: u32, height: u32, align: FlexAlign) -> Block {
    Block {
        parent,
        style: Style {
            display: Display::FlexRow,
            width: Length::Px(width),
            height: Length::Px(height),
            align_items: align,
            ..Style::default()
        },
        ..Block::default()
    }
}

fn item(parent: usize, width: u32, height: u32, align: Option<FlexAlign>) -> Block {
    Block {
        parent: Some(parent),
        intrinsic_height: height,
        style: Style {
            width: Length::Px(width),
            align_self: align,
            ..Style::default()
        },
    }
}

#[test]
fn overrides_coexist_with_auto_and_reset_without_changing_sizing_or_flow() {
    assert_eq!(Style::default().align_self, None);
    let mut blocks = [
        Block::default(),
        row(Some(0), 60, 30, FlexAlign::SafeEnd),
        item(1, 10, 6, Some(FlexAlign::Start)),
        item(1, 10, 8, Some(FlexAlign::SafeEnd)),
        item(1, 10, 10, Some(FlexAlign::SafeCenter)),
        item(1, 10, 12, None),
        Block::child(0, 2),
    ];
    blocks[1].style.margin = Edges {
        top: 2,
        left: 1,
        bottom: 3,
        right: 0,
    };
    blocks[1].style.border = Edges::all(1);
    blocks[1].style.padding = Edges::all(2);
    let initial = layout(&blocks, 80, 60).unwrap();
    assert_eq!(initial.boxes[1].unwrap().content, rect(4, 5, 60, 30));
    for (node, expected) in [
        (2, rect(4, 5, 10, 6)),
        (3, rect(14, 27, 10, 8)),
        (4, rect(24, 15, 10, 10)),
        (5, rect(34, 23, 10, 12)),
        (6, rect(0, 41, 80, 2)),
    ] {
        assert_eq!(initial.boxes[node].unwrap().content, expected);
    }
    blocks[1].style.align_items = FlexAlign::Start;
    let changed_default = layout(&blocks, 80, 60).unwrap();
    for node in [1, 2, 3, 4, 6] {
        assert_eq!(changed_default.boxes[node], initial.boxes[node]);
    }
    assert_eq!(
        changed_default.boxes[5].unwrap().content,
        rect(34, 5, 10, 12)
    );
    blocks[3].style.align_self = None;
    let reset = layout(&blocks, 80, 60).unwrap();
    assert_eq!(reset.boxes[3].unwrap().content, rect(14, 5, 10, 8));
    assert_eq!(reset.boxes[6], initial.boxes[6]);
}

#[test]
fn fractional_rejection_and_safe_overflow_use_only_the_effective_item_mode() {
    let mut blocks = [
        row(None, 20, 9, FlexAlign::SafeCenter),
        item(0, 10, 4, Some(FlexAlign::Start)),
        item(0, 10, 6, Some(FlexAlign::SafeEnd)),
    ];
    let frame = layout(&blocks, 20, 9).unwrap();
    assert_eq!(frame.boxes[1].unwrap().content, rect(0, 0, 10, 4));
    assert_eq!(frame.boxes[2].unwrap().content, rect(10, 3, 10, 6));
    blocks[1].style.align_self = None;
    assert_eq!(
        layout(&blocks, 20, 9),
        Err(LayoutError::FractionalFlexAlign { node: 0 })
    );
    blocks[0].style.align_items = FlexAlign::Start;
    blocks[1].style.align_self = Some(FlexAlign::SafeCenter);
    assert_eq!(
        layout(&blocks, 20, 9),
        Err(LayoutError::FractionalFlexAlign { node: 0 })
    );
    blocks[1].style.visibility = Visibility::Hidden;
    assert_eq!(
        layout(&blocks, 20, 9),
        Err(LayoutError::FractionalFlexAlign { node: 0 })
    );
    blocks[1].style.display = Display::None;
    assert!(layout(&blocks, 20, 9).unwrap().boxes[1].is_none());
    blocks[1].style.display = Display::Block;
    blocks[0].style.height = Length::Px(3);
    let overflow = layout(&blocks, 20, 3).unwrap();
    assert_eq!(overflow.boxes[1].unwrap().content, rect(0, 0, 10, 4));
    assert_eq!(overflow.boxes[2].unwrap().content, rect(10, 0, 10, 6));
    assert_eq!(overflow.scroll_height, 6);
    blocks[1].intrinsic_height = 0;
    assert_eq!(
        layout(&blocks, 20, 3),
        Err(LayoutError::FractionalFlexAlign { node: 0 })
    );
}

#[test]
fn wrapped_overrides_use_line_height_and_nowrap_uses_the_used_container_height() {
    let mut blocks = [
        Block::default(),
        row(Some(0), 25, 50, FlexAlign::SafeEnd),
        item(1, 10, 4, Some(FlexAlign::Start)),
        item(1, 10, 10, Some(FlexAlign::SafeCenter)),
        item(1, 10, 8, None),
        item(1, 10, 2, Some(FlexAlign::SafeCenter)),
        Block::child(0, 2),
    ];
    blocks[1].style.flex_wrap = FlexWrap::Wrap;
    blocks[1].style.column_gap = 5;
    blocks[1].style.row_gap = 3;
    let wrapped = layout(&blocks, 60, 60).unwrap();
    for (node, expected) in [
        (2, rect(0, 0, 10, 4)),
        (3, rect(15, 0, 10, 10)),
        (4, rect(0, 13, 10, 8)),
        (5, rect(15, 16, 10, 2)),
        (6, rect(0, 50, 60, 2)),
    ] {
        assert_eq!(wrapped.boxes[node].unwrap().content, expected);
    }
    blocks[1].style.flex_wrap = FlexWrap::NoWrap;
    let nowrap = layout(&blocks, 60, 60).unwrap();
    for (node, expected) in [
        (2, rect(0, 0, 10, 4)),
        (3, rect(15, 20, 10, 10)),
        (4, rect(30, 42, 10, 8)),
        (5, rect(45, 24, 10, 2)),
    ] {
        assert_eq!(nowrap.boxes[node].unwrap().content, expected);
    }
    assert_eq!(nowrap.boxes[6], wrapped.boxes[6]);
}

#[test]
fn boxless_wrappers_and_ordinary_children_do_not_inherit_item_overrides() {
    let mut blocks = [
        row(None, 60, 30, FlexAlign::SafeEnd),
        item(0, 10, 0, Some(FlexAlign::SafeCenter)),
        item(1, 10, 6, None),
        item(2, 4, 2, Some(FlexAlign::SafeCenter)),
        row(Some(0), 20, 10, FlexAlign::Start),
        item(4, 5, 4, None),
        item(4, 5, 2, Some(FlexAlign::SafeEnd)),
    ];
    blocks[0].style.align_self = Some(FlexAlign::SafeCenter);
    blocks[1].style.display = Display::Contents;
    blocks[1].style.align_items = FlexAlign::SafeCenter;
    blocks[4].style.align_self = Some(FlexAlign::SafeCenter);
    let frame = layout(&blocks, 60, 40).unwrap();
    assert!(frame.boxes[1].is_none());
    for (node, expected) in [
        (0, rect(0, 0, 60, 30)),
        (2, rect(0, 24, 10, 6)),
        (3, rect(0, 24, 4, 2)),
        (4, rect(10, 10, 20, 10)),
        (5, rect(10, 10, 5, 4)),
        (6, rect(15, 18, 5, 2)),
    ] {
        assert_eq!(frame.boxes[node].unwrap().content, expected);
    }
    blocks[1].style.display = Display::Block;
    let boxed = layout(&blocks, 60, 40).unwrap();
    assert_eq!(boxed.boxes[1].unwrap().content, rect(0, 12, 10, 6));
    assert_eq!(boxed.boxes[2].unwrap().content, rect(0, 12, 10, 6));
    assert_eq!(boxed.boxes[3].unwrap().content, rect(0, 12, 4, 2));
    assert_eq!(boxed.boxes[4], frame.boxes[4]);
}

#[test]
fn item_overrides_share_relative_projection_clipping_paint_and_scrolled_hits() {
    let blue = Rgb::new(20, 40, 220);
    let green = Rgb::new(20, 190, 30);
    let mut blocks = [
        row(None, 20, 20, FlexAlign::SafeEnd),
        item(0, 10, 6, Some(FlexAlign::Start)),
        item(0, 10, 6, Some(FlexAlign::SafeCenter)),
    ];
    blocks[0].style.border = Edges::all(1);
    blocks[0].style.padding = Edges::all(2);
    blocks[0].style.descendant_clip = DescendantClip::PaddingBox;
    blocks[2].style.relative_offset.y = 10;
    let frame = layout(&blocks, 26, 14).unwrap();
    assert_eq!(frame.boxes[1].unwrap().content, rect(3, 3, 10, 6));
    assert_eq!(frame.boxes[2].unwrap().content, rect(13, 20, 10, 6));
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
    let initial = paint(&frame, &paints, Rgb::WHITE).unwrap();
    assert_eq!(initial.pixel(4, 4), Some(blue));
    assert_eq!(hit_test(&frame, &hits, 4, 4), Ok(Some(1)));
    let scrolled = paint_scrolled(&frame, &paints, Rgb::WHITE, 0, 12).unwrap();
    for ((x, y), color, target) in [
        ((14, 9), green, Some(2)),
        ((14, 12), green, Some(2)),
        ((14, 13), Rgb::BLACK, Some(0)),
    ] {
        assert_eq!(scrolled.pixel(x, y), Some(color));
        assert_eq!(hit_test_scrolled(&frame, &hits, x, y, 0, 12), Ok(target));
    }
}

#[test]
fn semantic_text_anchors_follow_item_modes_and_auto_reset() {
    let mut elements = [
        Element::new(None, ElementKind::Body, "", 0),
        Element::new(Some(0), ElementKind::Paragraph, "one", 6),
        Element::new(Some(0), ElementKind::Paragraph, "two", 10),
    ];
    elements[0].resolved_style = Some(row(None, 40, 20, FlexAlign::SafeEnd).style);
    elements[1].resolved_style = Some(item(0, 10, 0, Some(FlexAlign::Start)).style);
    elements[2].resolved_style = Some(item(0, 10, 0, Some(FlexAlign::SafeCenter)).style);
    let frame = layout_semantic(&elements, 40, 20).unwrap();
    assert_eq!(frame.text_boxes[0].content, rect(0, 0, 10, 6));
    assert_eq!(frame.text_boxes[1].content, rect(10, 5, 10, 10));
    elements[1].resolved_style.as_mut().unwrap().align_self = None;
    let reset = layout_semantic(&elements, 40, 20).unwrap();
    assert_eq!(reset.text_boxes[0].content, rect(0, 14, 10, 6));
    assert_eq!(reset.text_boxes[1].content, frame.text_boxes[1].content);
}

#[test]
fn override_projection_retains_node_depth_and_overflow_limits_on_a_small_stack() {
    std::thread::Builder::new()
        .stack_size(2 * 1024 * 1024)
        .spawn(|| {
            let mut deep = vec![row(None, 1, (2 * MAX_DEPTH + 1) as u32, FlexAlign::Start)];
            for level in 1..=MAX_DEPTH {
                let mut child = row(
                    Some(level - 1),
                    1,
                    (2 * (MAX_DEPTH - level) + 1) as u32,
                    FlexAlign::Start,
                );
                child.style.align_self = Some(FlexAlign::SafeEnd);
                if level == MAX_DEPTH {
                    child.style.display = Display::Block;
                }
                deep.push(child);
            }
            let frame = layout(&deep, 1, 1).unwrap();
            assert_eq!(frame.boxes[MAX_DEPTH].unwrap().content, rect(0, 128, 1, 1));
            deep.push(item(MAX_DEPTH, 1, 1, Some(FlexAlign::SafeEnd)));
            assert_eq!(
                layout(&deep, 1, 1),
                Err(LayoutError::TooDeep {
                    node: MAX_DEPTH + 1
                })
            );
            let mut wide = vec![row(None, 1, 2, FlexAlign::Start)];
            wide.extend((1..MAX_BLOCKS).map(|_| item(0, 0, 0, Some(FlexAlign::SafeCenter))));
            assert_eq!(
                layout(&wide, 1, 1).unwrap().boxes[MAX_BLOCKS - 1]
                    .unwrap()
                    .content,
                rect(0, 1, 0, 0)
            );
            wide.push(item(0, 0, 0, None));
            assert_eq!(layout(&wide, 1, 1), Err(LayoutError::TooManyBlocks));
            let mut overflow = [
                row(None, 1, 2, FlexAlign::Start),
                item(0, 1, 1, Some(FlexAlign::SafeEnd)),
            ];
            overflow[1].style.relative_offset.y = u32::MAX - 1;
            assert_eq!(layout(&overflow, 1, 2), Err(LayoutError::Overflow));
        })
        .unwrap()
        .join()
        .unwrap();
}
