//! Literal main-axis geometry and shared-consumer checks for nonflexing rows.
use grunkalunka_layout::hit_test::{HitNode, hit_test, hit_test_scrolled};
use grunkalunka_layout::paint::{PaintNode, Rgb, paint, paint_scrolled};
use grunkalunka_layout::semantic::{Element, ElementKind, layout_semantic};
use grunkalunka_layout::{
    Block, BoxSizing, DescendantClip, Display, Edges, FlexJustify, FlexWrap, LayoutError, Length,
    MAX_BLOCKS, MAX_DEPTH, PercentagePadding, PointerEvents, Rect, RelativeOffset, Style,
    Visibility, WidthBound, layout,
};

fn rect(x: u32, y: u32, width: u32, height: u32) -> Rect {
    Rect {
        x,
        y,
        width,
        height,
    }
}

fn row(parent: Option<usize>, width: u32, justify: FlexJustify) -> Block {
    Block {
        parent,
        style: Style {
            display: Display::FlexRow,
            width: Length::Px(width),
            justify_content: justify,
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

fn policies(blocks: &[Block], colors: &[Rgb]) -> (Vec<HitNode>, Vec<PaintNode>) {
    (
        blocks.iter().map(|b| HitNode::new(b.parent)).collect(),
        blocks
            .iter()
            .zip(colors)
            .map(|(b, color)| PaintNode {
                parent: b.parent,
                background: Some(*color),
                border: Some(Rgb::BLACK),
            })
            .collect(),
    )
}

#[test]
fn center_moves_every_box_rectangle_without_changing_vertical_flow() {
    assert_eq!(Style::default().justify_content, FlexJustify::Start);
    let mut blocks = [
        Block::default(),
        row(Some(0), 60, FlexJustify::SafeCenter),
        item(1, 10, 5),
        item(1, 12, 4),
        Block::child(0, 2),
    ];
    blocks[1].style.column_gap = 4;
    blocks[1].style.margin = Edges {
        top: 2,
        bottom: 3,
        left: 1,
        right: 0,
    };
    blocks[1].style.padding = Edges::all(2);
    blocks[1].style.border = Edges::all(1);
    blocks[2].style.margin = Edges {
        top: 1,
        right: 3,
        bottom: 2,
        left: 1,
    };
    blocks[2].style.border = Edges::all(1);
    blocks[2].style.padding = Edges::all(2);
    blocks[3].style.margin.left = 2;
    blocks[3].style.margin.right = 2;
    let frame = layout(&blocks, 100, 30).unwrap();
    let first = frame.boxes[2].unwrap();
    assert_eq!(first.margin, rect(14, 5, 20, 14));
    assert_eq!(first.border, rect(15, 6, 16, 11));
    assert_eq!(first.padding, rect(16, 7, 14, 9));
    assert_eq!(first.content, rect(18, 9, 10, 5));
    assert_eq!(frame.boxes[3].unwrap().margin, rect(38, 5, 16, 4));
    assert_eq!(frame.boxes[1].unwrap().content, rect(4, 5, 60, 14));
    assert_eq!(frame.boxes[4].unwrap().content, rect(0, 25, 100, 2));
}

#[test]
fn each_main_axis_mode_uses_residual_space_after_existing_gaps() {
    let mut blocks = [
        row(None, 100, FlexJustify::Start),
        item(0, 10, 3),
        item(0, 20, 5),
        item(0, 10, 4),
    ];
    blocks[0].style.column_gap = 5;
    for (justify, positions) in [
        (FlexJustify::Start, [0, 15, 40]),
        (FlexJustify::SafeCenter, [25, 40, 65]),
        (FlexJustify::SafeEnd, [50, 65, 90]),
        (FlexJustify::SpaceBetween, [0, 40, 90]),
    ] {
        blocks[0].style.justify_content = justify;
        let frame = layout(&blocks, 100, 10).unwrap();
        for (node, x) in (1..=3).zip(positions) {
            assert_eq!(frame.boxes[node].unwrap().content.x, x, "{justify:?}");
        }
        assert_eq!(frame.boxes[0].unwrap().content.height, 5);
    }
}

#[test]
fn percentage_edges_and_constrained_border_boxes_keep_the_full_width_reference() {
    let mut blocks = [
        row(None, 200, FlexJustify::SafeEnd),
        item(0, 1, 4),
        item(0, 1, 4),
    ];
    blocks[0].style.column_gap = 10;
    blocks[1].style.width = Length::Percent(25);
    blocks[1].style.percentage_padding = PercentagePadding {
        left: Some(5),
        right: Some(5),
        ..PercentagePadding::default()
    };
    blocks[2].style.width = Length::Percent(50);
    blocks[2].style.max_width = Some(WidthBound::Px(90));
    blocks[2].style.box_sizing = BoxSizing::BorderBox;
    blocks[2].style.border = Edges::all(2);
    blocks[2].style.padding = Edges::all(3);
    let frame = layout(&blocks, 200, 20).unwrap();
    assert_eq!(frame.boxes[1].unwrap().margin, rect(30, 0, 70, 4));
    assert_eq!(frame.boxes[1].unwrap().content, rect(40, 0, 50, 4));
    assert_eq!(frame.boxes[2].unwrap().border, rect(110, 0, 90, 14));
    assert_eq!(frame.boxes[2].unwrap().content, rect(115, 5, 80, 4));
}

#[test]
fn every_wrapped_line_is_justified_independently() {
    let mut blocks = [
        row(None, 50, FlexJustify::SafeEnd),
        item(0, 20, 5),
        item(0, 20, 5),
        item(0, 30, 5),
        item(0, 10, 5),
        item(0, 20, 5),
    ];
    blocks[0].style.flex_wrap = FlexWrap::Wrap;
    blocks[0].style.column_gap = 5;
    blocks[0].style.row_gap = 3;
    for (justify, positions) in [
        (FlexJustify::SafeEnd, [5, 30, 5, 40, 30]),
        (FlexJustify::SpaceBetween, [0, 30, 0, 40, 0]),
    ] {
        blocks[0].style.justify_content = justify;
        let frame = layout(&blocks, 50, 30).unwrap();
        for ((node, x), y) in (1..=5).zip(positions).zip([0, 0, 8, 8, 16]) {
            let content = frame.boxes[node].unwrap().content;
            assert_eq!((content.x, content.y), (x, y));
        }
        assert_eq!(frame.boxes[0].unwrap().content.height, 21);
    }
}

#[test]
fn empty_single_zero_width_and_overflow_lines_have_explicit_fallbacks() {
    assert!(layout(&[row(None, 1, FlexJustify::SafeCenter)], 1, 1).is_ok());
    let mut single = [row(None, 7, FlexJustify::SpaceBetween), item(0, 1, 1)];
    assert_eq!(
        layout(&single, 7, 1).unwrap().boxes[1].unwrap().content.x,
        0
    );
    single[0].style.justify_content = FlexJustify::SafeCenter;
    assert_eq!(
        layout(&single, 7, 1).unwrap().boxes[1].unwrap().content.x,
        3
    );
    let zeros = [
        row(None, 10, FlexJustify::SpaceBetween),
        item(0, 0, 0),
        item(0, 0, 0),
        item(0, 0, 0),
    ];
    let frame = layout(&zeros, 10, 1).unwrap();
    for (node, x) in [(1, 0), (2, 5), (3, 10)] {
        assert_eq!(frame.boxes[node].unwrap().content.x, x);
    }
    for justify in [
        FlexJustify::SafeEnd,
        FlexJustify::SafeCenter,
        FlexJustify::SpaceBetween,
    ] {
        let blocks = [row(None, 10, justify), item(0, 11, 1)];
        let frame = layout(&blocks, 10, 1).unwrap();
        assert_eq!(frame.boxes[1].unwrap().content, rect(0, 0, 11, 1));
        assert_eq!(frame.scroll_width, 11);
    }
    let mut huge_gap = [
        row(None, 10, FlexJustify::SafeEnd),
        item(0, 0, 0),
        item(0, 0, 0),
    ];
    huge_gap[0].style.flex_wrap = FlexWrap::Wrap;
    huge_gap[0].style.column_gap = u32::MAX;
    huge_gap[0].style.row_gap = 2;
    let frame = layout(&huge_gap, 10, 1).unwrap();
    assert_eq!(frame.boxes[1].unwrap().content, rect(10, 0, 0, 0));
    assert_eq!(frame.boxes[2].unwrap().content, rect(10, 2, 0, 0));
}

#[test]
fn fractions_are_rejected_after_safe_fallback_and_hidden_items_still_count() {
    let center = [row(None, 11, FlexJustify::SafeCenter), item(0, 10, 1)];
    assert_eq!(
        layout(&center, 11, 2),
        Err(LayoutError::FractionalFlexJustify { node: 0 })
    );
    let mut distributed = [
        row(None, 10, FlexJustify::SpaceBetween),
        item(0, 1, 1),
        item(0, 1, 1),
        item(0, 1, 1),
    ];
    distributed[3].style.visibility = Visibility::Hidden;
    assert_eq!(
        layout(&distributed, 10, 2),
        Err(LayoutError::FractionalFlexJustify { node: 0 })
    );
    distributed[3].style.display = Display::None;
    assert_eq!(
        layout(&distributed, 10, 2).unwrap().boxes[2]
            .unwrap()
            .content
            .x,
        9
    );
    distributed[3].style.display = Display::Block;
    distributed[0].style.width = Length::Px(2);
    let frame = layout(&distributed, 2, 2).unwrap();
    assert_eq!(frame.boxes[3].unwrap().content.x, 2);
    let mut ordinary = center;
    ordinary[0].style.display = Display::Block;
    assert_eq!(
        layout(&ordinary, 11, 2).unwrap().boxes[1]
            .unwrap()
            .content
            .x,
        0
    );
}

#[test]
fn projection_inherits_only_alignment_through_original_nonpreorder_parents() {
    let mut blocks = [
        row(None, 100, FlexJustify::SafeEnd),
        Block::child(0, 0),
        row(Some(0), 30, FlexJustify::SafeEnd),
        item(1, 20, 4),
        item(2, 10, 4),
        item(3, 4, 2),
        Block::child(0, 0),
        Block::child(6, 0),
    ];
    blocks[0].style.column_gap = 10;
    blocks[1].style.display = Display::Contents;
    blocks[1].style.visibility = Visibility::Hidden;
    blocks[1].style.justify_content = FlexJustify::SafeCenter;
    blocks[2].style.pointer_events = PointerEvents::None;
    blocks[3].style.relative_offset.x = 2;
    blocks[4].style.pointer_events = PointerEvents::Auto;
    blocks[5].style.visibility = Visibility::Visible;
    blocks[6].style.display = Display::None;
    let frame = layout(&blocks, 100, 8).unwrap();
    assert_eq!(frame.boxes[3].unwrap().content, rect(42, 0, 20, 4));
    assert_eq!(frame.boxes[2].unwrap().content, rect(70, 0, 30, 4));
    assert_eq!(frame.boxes[4].unwrap().content, rect(90, 0, 10, 4));
    assert_eq!(frame.boxes[5].unwrap().content, rect(42, 0, 4, 2));
    assert!(frame.boxes[1].is_none() && frame.boxes[6].is_none() && frame.boxes[7].is_none());
    let grey = Rgb::new(180, 180, 180);
    let blue = Rgb::new(20, 40, 220);
    let green = Rgb::new(20, 190, 30);
    let (hits, paints) = policies(&blocks, &[grey, blue, blue, blue, green, green, blue, blue]);
    let canvas = paint(&frame, &paints, Rgb::WHITE).unwrap();
    for (x, color, target) in [
        (43, green, Some(5)),
        (48, grey, Some(0)),
        (75, blue, Some(0)),
        (95, green, Some(4)),
    ] {
        assert_eq!(canvas.pixel(x, 1), Some(color));
        assert_eq!(hit_test(&frame, &hits, x, 1), Ok(target));
    }
}

#[test]
fn final_scroll_extents_padding_clips_and_hits_use_projected_boxes() {
    let blue = Rgb::new(20, 40, 220);
    let green = Rgb::new(20, 190, 30);
    let mut blocks = [
        row(None, 40, FlexJustify::SafeEnd),
        item(0, 10, 6),
        item(0, 10, 6),
    ];
    blocks[0].style.border = Edges::all(1);
    blocks[0].style.padding = Edges::all(2);
    blocks[0].style.descendant_clip = DescendantClip::PaddingBox;
    blocks[2].style.relative_offset.x = 10;
    let frame = layout(&blocks, 32, 12).unwrap();
    assert_eq!(frame.boxes[1].unwrap().content, rect(23, 3, 10, 6));
    assert_eq!(frame.boxes[2].unwrap().content, rect(43, 3, 10, 6));
    assert_eq!((frame.scroll_width, frame.scroll_height), (53, 12));
    let (hits, paints) = policies(&blocks, &[Rgb::WHITE, blue, green]);
    let canvas = paint_scrolled(&frame, &paints, Rgb::WHITE, 16, 0).unwrap();
    for (x, color, target) in [
        (11, blue, Some(1)),
        (27, green, Some(2)),
        (28, green, Some(2)),
        (29, Rgb::BLACK, Some(0)),
        (30, Rgb::WHITE, None),
    ] {
        assert_eq!(canvas.pixel(x, 4), Some(color));
        assert_eq!(hit_test_scrolled(&frame, &hits, x, 4, 16, 0), Ok(target));
    }
}

#[test]
fn semantic_anchors_are_created_from_final_aligned_content_boxes() {
    let mut elements = [
        Element::new(None, ElementKind::Body, "", 0),
        Element::new(Some(0), ElementKind::Paragraph, "one", 5),
        Element::new(Some(0), ElementKind::Paragraph, "two", 7),
    ];
    let mut root = row(None, 80, FlexJustify::SafeCenter);
    root.style.column_gap = 4;
    elements[0].resolved_style = Some(root.style);
    elements[1].resolved_style = Some(item(0, 30, 0).style);
    elements[2].resolved_style = Some(item(0, 20, 0).style);
    let semantic = layout_semantic(&elements, 80, 20).unwrap();
    assert_eq!(semantic.text_boxes[0].content, rect(13, 0, 30, 5));
    assert_eq!(semantic.text_boxes[1].content, rect(47, 0, 20, 7));
}

#[test]
fn deferred_alignment_checks_limits_and_final_extents_on_a_small_native_stack() {
    std::thread::Builder::new()
        .stack_size(2 * 1024 * 1024)
        .spawn(|| {
            let mut deep = vec![row(None, (2 * MAX_DEPTH + 1) as u32, FlexJustify::SafeEnd)];
            for level in 1..=MAX_DEPTH {
                let mut child = row(
                    Some(level - 1),
                    (2 * (MAX_DEPTH - level) + 1) as u32,
                    FlexJustify::SafeEnd,
                );
                if level == MAX_DEPTH {
                    child.style.display = Display::Block;
                    child.intrinsic_height = 1;
                }
                deep.push(child);
            }
            let frame = layout(&deep, 1, 1).unwrap();
            assert_eq!(frame.boxes[MAX_DEPTH].unwrap().content, rect(128, 0, 1, 1));
            deep.push(item(MAX_DEPTH, 1, 1));
            assert_eq!(
                layout(&deep, 1, 1),
                Err(LayoutError::TooDeep {
                    node: MAX_DEPTH + 1
                })
            );
            let mut wide = vec![row(
                None,
                (MAX_BLOCKS - 2) as u32,
                FlexJustify::SpaceBetween,
            )];
            wide.extend((1..MAX_BLOCKS).map(|_| item(0, 0, 0)));
            let frame = layout(&wide, 1, 1).unwrap();
            assert_eq!(frame.boxes[MAX_BLOCKS - 1].unwrap().content.x, 4094);
            wide.push(item(0, 0, 0));
            assert_eq!(layout(&wide, 1, 1), Err(LayoutError::TooManyBlocks));
            let mut overflow = [row(None, 2, FlexJustify::SafeEnd), item(0, 1, 1)];
            overflow[1].style.relative_offset = RelativeOffset {
                x: u32::MAX - 1,
                y: 0,
            };
            assert_eq!(layout(&overflow, 2, 1), Err(LayoutError::Overflow));
        })
        .unwrap()
        .join()
        .unwrap();
}
