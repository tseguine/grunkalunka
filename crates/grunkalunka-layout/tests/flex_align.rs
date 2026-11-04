//! Literal cross-axis geometry, safe overflow and shared-consumer checks.
use grunkalunka_layout::hit_test::{HitNode, hit_test, hit_test_scrolled};
use grunkalunka_layout::paint::{PaintNode, Rgb, paint, paint_scrolled};
use grunkalunka_layout::semantic::{Element, ElementKind, layout_semantic};
use grunkalunka_layout::{
    Block, BoxSizing, DescendantClip, Display, Edges, FlexAlign, FlexJustify, FlexWrap,
    HeightBound, LayoutError, Length, MAX_BLOCKS, MAX_DEPTH, PercentageMargins, PointerEvents,
    Rect, RelativeOffset, Style, Visibility, layout,
};

fn rect(x: u32, y: u32, width: u32, height: u32) -> Rect {
    Rect {
        x,
        y,
        width,
        height,
    }
}

fn row(parent: Option<usize>, width: u32, align: FlexAlign) -> Block {
    Block {
        parent,
        style: Style {
            display: Display::FlexRow,
            width: Length::Px(width),
            align_items: align,
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
fn cross_alignment_moves_all_outer_rectangles_without_stretching_or_reflow() {
    assert_eq!(Style::default().align_items, FlexAlign::Start);
    let mut blocks = [
        Block::default(),
        row(Some(0), 60, FlexAlign::SafeEnd),
        item(1, 10, 5),
        item(1, 12, 4),
        Block::child(0, 2),
    ];
    blocks[1].style.height = Length::Px(40);
    blocks[1].style.column_gap = 4;
    blocks[1].style.margin = Edges {
        top: 2,
        bottom: 3,
        left: 1,
        right: 0,
    };
    blocks[1].style.border = Edges::all(1);
    blocks[1].style.padding = Edges::all(2);
    blocks[2].style.margin = Edges {
        top: 1,
        right: 3,
        bottom: 2,
        left: 1,
    };
    blocks[2].style.border = Edges::all(1);
    blocks[2].style.padding = Edges::all(2);
    let frame = layout(&blocks, 100, 60).unwrap();
    let first = frame.boxes[2].unwrap();
    assert_eq!(first.margin, rect(4, 31, 20, 14));
    assert_eq!(first.border, rect(5, 32, 16, 11));
    assert_eq!(first.padding, rect(6, 33, 14, 9));
    assert_eq!(first.content, rect(8, 35, 10, 5));
    assert_eq!(frame.boxes[3].unwrap().content, rect(28, 41, 12, 4));
    assert_eq!(frame.boxes[1].unwrap().content, rect(4, 5, 60, 40));
    assert_eq!(frame.boxes[4].unwrap().content, rect(0, 51, 100, 2));
    blocks[1].style.align_items = FlexAlign::SafeCenter;
    let centered = layout(&blocks, 100, 60).unwrap();
    assert_eq!(centered.boxes[2].unwrap().margin, rect(4, 18, 20, 14));
    assert_eq!(centered.boxes[3].unwrap().content, rect(28, 23, 12, 4));
    assert_eq!(centered.boxes[4], frame.boxes[4]);
}

#[test]
fn nowrap_uses_the_finished_content_height_including_bounds_and_intrinsic_minimum() {
    for (height, minimum, maximum, intrinsic, used, positions) in [
        (Length::Px(20), None, None, 0, 20, [12, 16]),
        (Length::Auto, None, None, 0, 8, [0, 4]),
        (
            Length::Auto,
            Some(HeightBound::Px(20)),
            None,
            0,
            20,
            [12, 16],
        ),
        (Length::Auto, None, Some(HeightBound::Px(6)), 0, 6, [0, 2]),
        (
            Length::Auto,
            Some(HeightBound::Px(12)),
            Some(HeightBound::Px(6)),
            0,
            12,
            [4, 8],
        ),
        (Length::Auto, None, None, 18, 18, [10, 14]),
        (
            Length::Percent(50),
            None,
            Some(HeightBound::Percent(40)),
            0,
            16,
            [8, 12],
        ),
    ] {
        let mut blocks = [
            row(None, 30, FlexAlign::SafeEnd),
            item(0, 10, 8),
            item(0, 10, 4),
        ];
        blocks[0].style.height = height;
        blocks[0].style.min_height = minimum;
        blocks[0].style.max_height = maximum;
        blocks[0].intrinsic_height = intrinsic;
        let frame = layout(&blocks, 30, 40).unwrap();
        assert_eq!(frame.boxes[0].unwrap().content.height, used);
        for (node, y) in (1..=2).zip(positions) {
            assert_eq!(
                frame.boxes[node].unwrap().content.y,
                y,
                "{height:?}, {minimum:?}, {maximum:?}"
            );
        }
    }
}

#[test]
fn wrap_with_one_actual_line_keeps_its_natural_height_and_fraction_policy() {
    let mut blocks = [
        row(None, 30, FlexAlign::SafeCenter),
        item(0, 10, 20),
        item(0, 10, 40),
    ];
    blocks[0].style.height = Length::Px(41);
    assert_eq!(
        layout(&blocks, 30, 50),
        Err(LayoutError::FractionalFlexAlign { node: 0 })
    );
    blocks[0].style.flex_wrap = FlexWrap::Wrap;
    let wrapped = layout(&blocks, 30, 50).unwrap();
    assert_eq!(wrapped.boxes[0].unwrap().content.height, 41);
    assert_eq!(wrapped.boxes[1].unwrap().content, rect(0, 10, 10, 20));
    assert_eq!(wrapped.boxes[2].unwrap().content, rect(10, 0, 10, 40));

    blocks[0].style.height = Length::Auto;
    blocks[0].style.max_height = Some(HeightBound::Px(20));
    blocks[0].style.align_items = FlexAlign::SafeEnd;
    blocks[1].intrinsic_height = 30;
    for (wrap, expected) in [(FlexWrap::NoWrap, [0, 0]), (FlexWrap::Wrap, [10, 0])] {
        blocks[0].style.flex_wrap = wrap;
        let frame = layout(&blocks, 30, 50).unwrap();
        assert_eq!(frame.boxes[0].unwrap().content.height, 20);
        for (node, y) in (1..=2).zip(expected) {
            assert_eq!(frame.boxes[node].unwrap().content.y, y);
        }
    }
}

#[test]
fn wrapped_lines_align_independently_without_distributing_extra_container_height() {
    let mut blocks = [
        Block::default(),
        row(Some(0), 30, FlexAlign::SafeEnd),
        item(1, 10, 4),
        item(1, 10, 8),
        item(1, 10, 6),
        item(1, 10, 2),
        Block::child(0, 2),
    ];
    blocks[1].style.height = Length::Px(30);
    blocks[1].style.flex_wrap = FlexWrap::Wrap;
    blocks[1].style.column_gap = 5;
    blocks[1].style.row_gap = 3;
    blocks[1].style.justify_content = FlexJustify::SpaceBetween;
    for (align, positions) in [
        (FlexAlign::SafeEnd, [4, 0, 11, 15]),
        (FlexAlign::SafeCenter, [2, 0, 11, 13]),
    ] {
        blocks[1].style.align_items = align;
        let frame = layout(&blocks, 40, 40).unwrap();
        for ((node, x), y) in (2..=5).zip([0, 20, 0, 20]).zip(positions) {
            let content = frame.boxes[node].unwrap().content;
            assert_eq!((content.x, content.y), (x, y));
        }
        assert_eq!(frame.boxes[1].unwrap().content.height, 30);
        assert_eq!(frame.boxes[6].unwrap().content, rect(0, 30, 40, 2));
    }
}

#[test]
fn nested_percentages_use_original_definite_sizes_before_both_alignment_axes() {
    let mut blocks = [
        row(None, 46, FlexAlign::SafeCenter),
        row(Some(0), 10, FlexAlign::SafeEnd),
        item(0, 10, 10),
        item(1, 5, 0),
    ];
    blocks[0].style.box_sizing = BoxSizing::BorderBox;
    blocks[0].style.height = Length::Px(46);
    blocks[0].style.border = Edges::all(1);
    blocks[0].style.padding = Edges::all(2);
    blocks[0].style.justify_content = FlexJustify::SafeCenter;
    blocks[1].style.height = Length::Percent(50);
    blocks[1].style.justify_content = FlexJustify::SafeEnd;
    blocks[2].style.percentage_margins = PercentageMargins {
        top: Some(5),
        bottom: Some(15),
        ..PercentageMargins::default()
    };
    blocks[3].style.height = Length::Percent(50);
    let frame = layout(&blocks, 60, 60).unwrap();
    assert_eq!(frame.boxes[0].unwrap().content, rect(3, 3, 40, 40));
    assert_eq!(frame.boxes[1].unwrap().content, rect(13, 13, 10, 20));
    assert_eq!(frame.boxes[2].unwrap().margin, rect(23, 14, 10, 18));
    assert_eq!(frame.boxes[2].unwrap().content, rect(23, 16, 10, 10));
    assert_eq!(frame.boxes[3].unwrap().content, rect(18, 23, 5, 10));
    blocks[0].style.height = Length::Auto;
    blocks[0].style.min_height = Some(HeightBound::Px(46));
    assert_eq!(
        layout(&blocks, 60, 60),
        Err(LayoutError::UnsupportedPercentageHeight { node: 1 })
    );
}

#[test]
fn safe_overflow_is_per_item_and_precedes_fractional_rejection() {
    let mut blocks = [
        row(None, 20, FlexAlign::SafeCenter),
        item(0, 10, 9),
        item(0, 10, 2),
    ];
    blocks[0].style.height = Length::Px(6);
    let frame = layout(&blocks, 20, 6).unwrap();
    assert_eq!(frame.boxes[1].unwrap().content, rect(0, 0, 10, 9));
    assert_eq!(frame.boxes[2].unwrap().content, rect(10, 2, 10, 2));
    assert_eq!(frame.scroll_height, 9);
    blocks[0].style.align_items = FlexAlign::SafeEnd;
    assert_eq!(
        layout(&blocks, 20, 6).unwrap().boxes[2].unwrap().content.y,
        4
    );
    blocks[0].style.align_items = FlexAlign::SafeCenter;
    blocks[2].intrinsic_height = 5;
    blocks[2].style.visibility = Visibility::Hidden;
    assert_eq!(
        layout(&blocks, 20, 6),
        Err(LayoutError::FractionalFlexAlign { node: 0 })
    );
    blocks[2].style.display = Display::None;
    assert_eq!(
        layout(&blocks, 20, 6).unwrap().boxes[1].unwrap().content.y,
        0
    );
    blocks[2].style.display = Display::Block;
    blocks[0].style.display = Display::Block;
    let ordinary = layout(&blocks, 20, 6).unwrap();
    assert_eq!(ordinary.boxes[2].unwrap().content.y, 9);
    blocks[0].style.display = Display::Contents;
    assert_eq!(
        layout(&blocks, 20, 6).unwrap().boxes[2].unwrap().content.y,
        9
    );
}

#[test]
fn empty_single_and_zero_height_lines_do_not_add_phantom_alignment_or_gaps() {
    let mut empty = row(None, 1, FlexAlign::SafeCenter);
    empty.style.height = Length::Px(1);
    empty.style.row_gap = u32::MAX;
    assert!(layout(&[empty], 1, 1).is_ok());
    assert_eq!(
        layout(&[empty, item(0, 0, 0)], 1, 1),
        Err(LayoutError::FractionalFlexAlign { node: 0 })
    );
    let mut single = [row(None, 1, FlexAlign::SafeCenter), item(0, 1, 1)];
    single[0].style.height = Length::Px(7);
    assert_eq!(
        layout(&single, 1, 7).unwrap().boxes[1].unwrap().content.y,
        3
    );
    let mut zeros = [
        row(None, 1, FlexAlign::SafeEnd),
        item(0, 1, 0),
        item(0, 1, 0),
    ];
    zeros[0].style.height = Length::Px(10);
    let nowrap = layout(&zeros, 2, 10).unwrap();
    assert_eq!(nowrap.boxes[1].unwrap().content, rect(0, 10, 1, 0));
    assert_eq!(nowrap.boxes[2].unwrap().content, rect(1, 10, 1, 0));
    zeros[0].style.flex_wrap = FlexWrap::Wrap;
    zeros[0].style.row_gap = 2;
    let wrapped = layout(&zeros, 2, 10).unwrap();
    assert_eq!(wrapped.boxes[1].unwrap().content, rect(0, 0, 1, 0));
    assert_eq!(wrapped.boxes[2].unwrap().content, rect(0, 2, 1, 0));
}

#[test]
fn nested_boxless_projection_keeps_relative_offsets_and_inherited_policies() {
    let mut blocks = [
        row(None, 60, FlexAlign::SafeEnd),
        Block::child(0, 0),
        row(Some(0), 20, FlexAlign::SafeEnd),
        item(1, 10, 6),
        item(2, 4, 2),
        item(3, 3, 2),
        Block::child(0, 0),
        Block::child(6, 0),
    ];
    blocks[0].style.height = Length::Px(30);
    blocks[0].style.column_gap = 10;
    blocks[0].style.justify_content = FlexJustify::SafeEnd;
    blocks[1].style.display = Display::Contents;
    blocks[1].style.visibility = Visibility::Hidden;
    blocks[1].style.pointer_events = PointerEvents::None;
    blocks[1].style.align_items = FlexAlign::SafeCenter;
    blocks[2].style.height = Length::Px(10);
    blocks[2].style.justify_content = FlexJustify::SafeEnd;
    blocks[2].style.pointer_events = PointerEvents::None;
    blocks[3].style.relative_offset = RelativeOffset { x: 2, y: 3 };
    blocks[4].style.pointer_events = PointerEvents::Auto;
    blocks[5].style.visibility = Visibility::Visible;
    blocks[5].style.pointer_events = PointerEvents::Auto;
    blocks[6].style.display = Display::None;
    let frame = layout(&blocks, 60, 35).unwrap();
    assert_eq!(frame.boxes[3].unwrap().content, rect(22, 27, 10, 6));
    assert_eq!(frame.boxes[2].unwrap().content, rect(40, 20, 20, 10));
    assert_eq!(frame.boxes[4].unwrap().content, rect(56, 28, 4, 2));
    assert_eq!(frame.boxes[5].unwrap().content, rect(22, 27, 3, 2));
    assert!(frame.boxes[1].is_none() && frame.boxes[6].is_none() && frame.boxes[7].is_none());
    let grey = Rgb::new(180, 180, 180);
    let blue = Rgb::new(20, 40, 220);
    let green = Rgb::new(20, 190, 30);
    let (hits, paints) = policies(&blocks, &[grey, blue, blue, blue, green, green, blue, blue]);
    let canvas = paint(&frame, &paints, Rgb::WHITE).unwrap();
    for ((x, y), color, target) in [
        ((23, 28), green, Some(5)),
        ((26, 28), grey, Some(0)),
        ((45, 24), blue, Some(0)),
        ((57, 29), green, Some(4)),
    ] {
        assert_eq!(canvas.pixel(x, y), Some(color));
        assert_eq!(hit_test(&frame, &hits, x, y), Ok(target));
    }
}

#[test]
fn wrapped_subtree_natural_height_is_measured_before_outer_alignment() {
    let mut blocks = [
        row(None, 40, FlexAlign::SafeCenter),
        row(Some(0), 10, FlexAlign::SafeEnd),
        item(0, 10, 10),
        item(1, 10, 4),
        item(1, 10, 8),
    ];
    blocks[0].style.min_height = Some(HeightBound::Px(30));
    blocks[1].style.flex_wrap = FlexWrap::Wrap;
    blocks[1].style.row_gap = 2;
    blocks[1].style.relative_offset.y = 3;
    let frame = layout(&blocks, 40, 30).unwrap();
    assert_eq!(frame.boxes[1].unwrap().content, rect(0, 11, 10, 14));
    assert_eq!(frame.boxes[2].unwrap().content, rect(10, 10, 10, 10));
    assert_eq!(frame.boxes[3].unwrap().content, rect(0, 11, 10, 4));
    assert_eq!(frame.boxes[4].unwrap().content, rect(0, 17, 10, 8));
    assert_eq!(frame.boxes[0].unwrap().content, rect(0, 0, 40, 30));
}

#[test]
fn projected_vertical_extents_and_padding_clips_agree_for_paint_and_scrolled_hits() {
    let blue = Rgb::new(20, 40, 220);
    let green = Rgb::new(20, 190, 30);
    let mut blocks = [
        row(None, 20, FlexAlign::SafeEnd),
        item(0, 10, 6),
        item(0, 10, 6),
    ];
    blocks[0].style.height = Length::Px(20);
    blocks[0].style.border = Edges::all(1);
    blocks[0].style.padding = Edges::all(2);
    blocks[0].style.descendant_clip = DescendantClip::PaddingBox;
    blocks[2].style.relative_offset.y = 6;
    let frame = layout(&blocks, 26, 14).unwrap();
    assert_eq!(frame.boxes[1].unwrap().content, rect(3, 17, 10, 6));
    assert_eq!(frame.boxes[2].unwrap().content, rect(13, 23, 10, 6));
    assert_eq!((frame.scroll_width, frame.scroll_height), (26, 29));
    let (hits, paints) = policies(&blocks, &[Rgb::WHITE, blue, green]);
    let canvas = paint_scrolled(&frame, &paints, Rgb::WHITE, 0, 14).unwrap();
    for ((x, y), color, target) in [
        ((4, 4), blue, Some(1)),
        ((14, 9), green, Some(2)),
        ((14, 10), green, Some(2)),
        ((14, 11), Rgb::BLACK, Some(0)),
        ((14, 12), Rgb::WHITE, None),
    ] {
        assert_eq!(canvas.pixel(x, y), Some(color));
        assert_eq!(hit_test_scrolled(&frame, &hits, x, y, 0, 14), Ok(target));
    }
}

#[test]
fn semantic_anchors_follow_both_axes_wrap_mode_and_hidden_item_reflow() {
    let mut elements = [
        Element::new(None, ElementKind::Body, "", 0),
        Element::new(Some(0), ElementKind::Paragraph, "one", 6),
        Element::new(Some(0), ElementKind::Paragraph, "two", 10),
    ];
    let mut root = row(None, 40, FlexAlign::SafeEnd);
    root.style.height = Length::Px(20);
    root.style.column_gap = 4;
    root.style.justify_content = FlexJustify::SafeCenter;
    elements[0].resolved_style = Some(root.style);
    elements[1].resolved_style = Some(item(0, 10, 0).style);
    elements[2].resolved_style = Some(item(0, 10, 0).style);
    let nowrap = layout_semantic(&elements, 40, 20).unwrap();
    assert_eq!(nowrap.text_boxes[0].content, rect(8, 14, 10, 6));
    assert_eq!(nowrap.text_boxes[1].content, rect(22, 10, 10, 10));
    root.style.flex_wrap = FlexWrap::Wrap;
    elements[0].resolved_style = Some(root.style);
    let wrapped = layout_semantic(&elements, 40, 20).unwrap();
    assert_eq!(wrapped.text_boxes[0].content, rect(8, 4, 10, 6));
    assert_eq!(wrapped.text_boxes[1].content, rect(22, 0, 10, 10));
    elements[2].hidden = true;
    let hidden = layout_semantic(&elements, 40, 20).unwrap();
    assert_eq!(hidden.text_boxes.len(), 1);
    assert_eq!(hidden.text_boxes[0].content, rect(15, 0, 10, 6));
}

#[test]
fn cross_alignment_limits_and_projected_overflow_fail_on_a_small_native_stack() {
    std::thread::Builder::new()
        .stack_size(2 * 1024 * 1024)
        .spawn(|| {
            let mut deep = vec![row(None, 1, FlexAlign::SafeEnd)];
            deep[0].style.height = Length::Px((2 * MAX_DEPTH + 1) as u32);
            for level in 1..=MAX_DEPTH {
                let mut child = row(Some(level - 1), 1, FlexAlign::SafeEnd);
                child.style.height = Length::Px((2 * (MAX_DEPTH - level) + 1) as u32);
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
            let mut wide = vec![row(None, 1, FlexAlign::SafeCenter)];
            wide[0].style.height = Length::Px(2);
            wide.extend((1..MAX_BLOCKS).map(|_| item(0, 0, 0)));
            let frame = layout(&wide, 1, 1).unwrap();
            assert_eq!(
                frame.boxes[MAX_BLOCKS - 1].unwrap().content,
                rect(0, 1, 0, 0)
            );
            wide.push(item(0, 0, 0));
            assert_eq!(layout(&wide, 1, 1), Err(LayoutError::TooManyBlocks));
            let mut overflow = [row(None, 1, FlexAlign::SafeEnd), item(0, 1, 1)];
            overflow[0].style.height = Length::Px(2);
            overflow[1].style.relative_offset.y = u32::MAX - 1;
            assert_eq!(layout(&overflow, 1, 2), Err(LayoutError::Overflow));
        })
        .unwrap()
        .join()
        .unwrap();
}
