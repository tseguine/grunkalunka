//! Literal geometry and observable paint/input checks for the bounded,
//! nonflexing row profile. No browser capture supplies these expectations.
use grunkalunka_layout::hit_test::{HitNode, hit_test, hit_test_scrolled};
use grunkalunka_layout::paint::{PaintNode, Rgb, paint, paint_scrolled};
use grunkalunka_layout::semantic::{Element, ElementKind, layout_semantic};
use grunkalunka_layout::{
    Block, BoxSizing, DescendantClip, Display, Edges, HeightBound, HorizontalAutoMargins,
    LayoutError, Length, MAX_BLOCKS, MAX_DEPTH, PercentageMargins, PercentagePadding,
    PointerEvents, Rect, RelativeOffset, Style, Visibility, WidthBound, layout,
};

const GREY: Rgb = Rgb::new(180, 180, 180);
const RED: Rgb = Rgb::new(230, 30, 30);
const GREEN: Rgb = Rgb::new(30, 180, 40);
const BLUE: Rgb = Rgb::new(30, 40, 230);

fn rect(x: u32, y: u32, width: u32, height: u32) -> Rect {
    Rect {
        x,
        y,
        width,
        height,
    }
}

fn row(parent: Option<usize>, width: u32, gap: u32) -> Block {
    Block {
        parent,
        style: Style {
            display: Display::FlexRow,
            width: Length::Px(width),
            column_gap: gap,
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
    assert_eq!(blocks.len(), colors.len());
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
fn row_consumes_outer_widths_and_its_tallest_outer_height() {
    assert_eq!(Style::default().column_gap, 0);
    let mut blocks = [
        Block::default(),
        row(Some(0), 60, 4),
        item(1, 12, 5),
        item(1, 18, 10),
        Block::child(0, 7),
    ];
    blocks[1].style.padding = Edges::all(2);
    blocks[1].style.border = Edges::all(1);
    blocks[1].style.margin = Edges {
        top: 2,
        right: 2,
        bottom: 3,
        left: 1,
    };
    blocks[2].style.padding = Edges::all(1);
    blocks[2].style.border = Edges::all(1);
    blocks[2].style.margin = Edges {
        top: 1,
        right: 1,
        bottom: 2,
        left: 2,
    };
    let frame = layout(&blocks, 100, 20).unwrap();
    assert_eq!(frame.boxes[0].unwrap().content, rect(0, 0, 100, 30));
    assert_eq!(frame.boxes[1].unwrap().margin, rect(0, 0, 69, 23));
    assert_eq!(frame.boxes[1].unwrap().content, rect(4, 5, 60, 12));
    assert_eq!(frame.boxes[2].unwrap().margin, rect(4, 5, 19, 12));
    assert_eq!(frame.boxes[2].unwrap().border, rect(6, 6, 16, 9));
    assert_eq!(frame.boxes[2].unwrap().content, rect(8, 8, 12, 5));
    assert_eq!(frame.boxes[3].unwrap().content, rect(27, 5, 18, 10));
    assert_eq!(frame.boxes[4].unwrap().content, rect(0, 23, 100, 7));
    assert_eq!((frame.scroll_width, frame.scroll_height), (100, 30));
}

#[test]
fn percentage_edges_bounds_and_box_sizing_use_the_full_row_width() {
    let mut blocks = [row(None, 200, 8), item(0, 0, 0), item(0, 0, 10)];
    blocks[1].style.width = Length::Percent(25);
    blocks[1].style.height = Length::Px(12);
    blocks[1].style.box_sizing = BoxSizing::BorderBox;
    blocks[1].style.border = Edges::all(1);
    blocks[1].style.padding = Edges::all(2);
    blocks[1].style.percentage_margins = PercentageMargins {
        left: Some(5),
        top: Some(5),
        ..PercentageMargins::default()
    };
    blocks[1].style.margin.right = 2;
    blocks[1].style.margin.bottom = 1;
    blocks[2].style.width = Length::Percent(50);
    blocks[2].style.max_width = Some(WidthBound::Px(70));
    blocks[2].style.min_width = Some(WidthBound::Px(80));
    blocks[2].style.percentage_padding = PercentagePadding {
        left: Some(5),
        right: Some(5),
        ..PercentagePadding::default()
    };
    let frame = layout(&blocks, 200, 40).unwrap();
    assert_eq!(frame.boxes[1].unwrap().margin, rect(0, 0, 62, 23));
    assert_eq!(frame.boxes[1].unwrap().border, rect(10, 10, 50, 12));
    assert_eq!(frame.boxes[1].unwrap().content, rect(13, 13, 44, 6));
    assert_eq!(frame.boxes[2].unwrap().border, rect(70, 0, 100, 10));
    assert_eq!(frame.boxes[2].unwrap().content, rect(80, 0, 80, 10));
    assert_eq!(frame.boxes[0].unwrap().content, rect(0, 0, 200, 23));
}

#[test]
fn definite_heights_keep_existing_bounds_without_stretching_other_items() {
    let mut blocks = [row(None, 80, 0), item(0, 20, 0), item(0, 20, 8)];
    blocks[0].style.height = Length::Px(40);
    blocks[1].style.height = Length::Percent(50);
    blocks[1].style.max_height = Some(HeightBound::Percent(60));
    blocks[1].style.min_height = Some(HeightBound::Px(30));
    let frame = layout(&blocks, 80, 50).unwrap();
    assert_eq!(frame.boxes[0].unwrap().content, rect(0, 0, 80, 40));
    assert_eq!(frame.boxes[1].unwrap().content, rect(0, 0, 20, 30));
    assert_eq!(frame.boxes[2].unwrap().content, rect(20, 0, 20, 8));
}

#[test]
fn contents_flatten_in_tree_order_and_hidden_subtrees_leave_no_gap() {
    let mut blocks = [
        row(None, 50, 3),
        Block::child(0, 0),
        item(0, 7, 4),
        Block::child(1, 0),
        Block::child(1, 0),
        item(3, 8, 5),
        item(1, 6, 3),
        Block::child(4, u32::MAX),
        Block::child(0, 0),
    ];
    for node in [1, 3, 8] {
        blocks[node].style.display = Display::Contents;
        blocks[node].style.margin = Edges::all(u32::MAX);
        blocks[node].style.relative_offset = RelativeOffset {
            x: u32::MAX,
            y: u32::MAX,
        };
        blocks[node].style.column_gap = u32::MAX;
    }
    blocks[4].style.display = Display::None;
    let frame = layout(&blocks, 50, 10).unwrap();
    assert_eq!(frame.boxes[5].unwrap().content, rect(0, 0, 8, 5));
    assert_eq!(frame.boxes[6].unwrap().content, rect(11, 0, 6, 3));
    assert_eq!(frame.boxes[2].unwrap().content, rect(20, 0, 7, 4));
    assert_eq!(frame.boxes[0].unwrap().content.height, 5);
    for node in [1, 3, 4, 7, 8] {
        assert!(frame.boxes[node].is_none());
    }
    assert_eq!(
        frame.contents,
        [false, true, false, true, false, false, false, false, true]
    );
    blocks[5].style.display = Display::None;
    let hidden = layout(&blocks, 50, 10).unwrap();
    assert_eq!(hidden.boxes[6].unwrap().content, rect(0, 0, 6, 3));
    assert_eq!(hidden.boxes[2].unwrap().content, rect(9, 0, 7, 4));
    blocks[5].style.display = Display::Block;
    assert_eq!(layout(&blocks, 50, 10), Ok(frame));
}

#[test]
fn gaps_count_zero_sized_items_but_never_empty_or_trailing_slots() {
    let mut blocks = [
        row(None, 30, 5),
        item(0, 0, 0),
        item(0, 10, 3),
        item(0, 0, 0),
    ];
    let frame = layout(&blocks, 30, 10).unwrap();
    assert_eq!(frame.boxes[1].unwrap().content, rect(0, 0, 0, 0));
    assert_eq!(frame.boxes[2].unwrap().content, rect(5, 0, 10, 3));
    assert_eq!(frame.boxes[3].unwrap().content, rect(20, 0, 0, 0));
    for block in &mut blocks[1..] {
        block.style.display = Display::None;
    }
    blocks[0].style.column_gap = u32::MAX;
    let empty = layout(&blocks, 30, 10).unwrap();
    assert_eq!(empty.boxes[0].unwrap().content.height, 0);
    blocks[2].style.display = Display::Block;
    let single = layout(&blocks, 30, 10).unwrap();
    assert_eq!(single.boxes[2].unwrap().content, rect(0, 0, 10, 3));
}

#[test]
fn nested_rows_and_vertical_items_keep_their_own_formatting_contexts() {
    let blocks = [
        Block::default(),
        row(Some(0), 80, 3),
        row(Some(1), 30, 2),
        item(1, 20, 0),
        item(2, 10, 5),
        item(2, 12, 9),
        Block::child(3, 7),
        Block::child(3, 4),
        Block::child(0, 2),
    ];
    let frame = layout(&blocks, 80, 20).unwrap();
    assert_eq!(frame.boxes[1].unwrap().content, rect(0, 0, 80, 11));
    assert_eq!(frame.boxes[2].unwrap().content, rect(0, 0, 30, 9));
    assert_eq!(frame.boxes[3].unwrap().content, rect(33, 0, 20, 11));
    assert_eq!(frame.boxes[4].unwrap().content, rect(0, 0, 10, 5));
    assert_eq!(frame.boxes[5].unwrap().content, rect(12, 0, 12, 9));
    assert_eq!(frame.boxes[7].unwrap().content, rect(33, 7, 20, 4));
    assert_eq!(frame.boxes[8].unwrap().content, rect(0, 11, 80, 2));
    assert_eq!(frame.boxes[0].unwrap().content.height, 13);
}

#[test]
fn relative_offsets_do_not_move_siblings_or_change_source_tree_paint_order() {
    let mut blocks = [
        row(None, 30, 3),
        item(0, 10, 5),
        item(0, 10, 8),
        item(1, 4, 3),
    ];
    blocks[0].style.padding = Edges::all(2);
    blocks[1].style.relative_offset = RelativeOffset { x: 12, y: 3 };
    blocks[3].style.relative_offset = RelativeOffset { x: 1, y: 1 };
    let frame = layout(&blocks, 40, 20).unwrap();
    assert_eq!(frame.boxes[0].unwrap().content, rect(2, 2, 30, 8));
    assert_eq!(frame.boxes[1].unwrap().border, rect(14, 5, 10, 5));
    assert_eq!(frame.boxes[2].unwrap().border, rect(15, 2, 10, 8));
    assert_eq!(frame.boxes[3].unwrap().border, rect(15, 6, 4, 3));
    let (hits, paints) = policies(&blocks, &[GREY, RED, GREEN, BLUE]);
    let canvas = paint(&frame, &paints, Rgb::WHITE).unwrap();
    for ((x, y), color, target) in [
        ((14, 5), RED, Some(1)),
        ((16, 7), GREEN, Some(2)),
        ((25, 3), GREY, Some(0)),
        ((35, 3), Rgb::WHITE, None),
    ] {
        assert_eq!(canvas.pixel(x, y), Some(color));
        assert_eq!(hit_test(&frame, &hits, x, y), Ok(target));
    }
    blocks[1].style.relative_offset = RelativeOffset::default();
    let unshifted = layout(&blocks, 40, 20).unwrap();
    assert_eq!(unshifted.boxes[0], frame.boxes[0]);
    assert_eq!(unshifted.boxes[2], frame.boxes[2]);
}

#[test]
fn overflowing_rows_share_padding_clips_between_paint_and_scrolled_hits() {
    let mut blocks = [row(None, 20, 3), item(0, 12, 8), item(0, 12, 8)];
    blocks[0].style.height = Length::Px(6);
    blocks[0].style.padding = Edges::all(2);
    blocks[0].style.border = Edges::all(1);
    blocks[0].style.descendant_clip = DescendantClip::PaddingBox;
    let frame = layout(&blocks, 25, 12).unwrap();
    assert_eq!(frame.boxes[2].unwrap().content, rect(18, 3, 12, 8));
    assert_eq!((frame.scroll_width, frame.scroll_height), (30, 12));
    let (hits, paints) = policies(&blocks, &[GREY, RED, GREEN]);
    let canvas = paint(&frame, &paints, Rgb::WHITE).unwrap();
    assert_eq!(canvas.pixel(24, 5), Some(GREEN));
    assert_eq!(hit_test(&frame, &hits, 24, 5), Ok(Some(2)));
    let scrolled = paint_scrolled(&frame, &paints, Rgb::WHITE, 5, 0).unwrap();
    for (x, color, target) in [
        (19, GREEN, Some(2)),
        (20, Rgb::BLACK, Some(0)),
        (21, Rgb::WHITE, None),
    ] {
        assert_eq!(scrolled.pixel(x, 5), Some(color));
        assert_eq!(hit_test_scrolled(&frame, &hits, x, 5, 5, 0), Ok(target));
    }
}

#[test]
fn boxless_inheritance_keeps_hidden_slots_and_explicit_input_overrides() {
    let mut blocks = [
        row(None, 30, 2),
        Block::child(0, 0),
        item(1, 6, 4),
        Block::child(1, 0),
        item(3, 6, 4),
        item(0, 6, 4),
        item(5, 3, 2),
    ];
    blocks[0].style.pointer_events = PointerEvents::None;
    blocks[1].style.display = Display::Contents;
    blocks[1].style.visibility = Visibility::Hidden;
    blocks[3].style.display = Display::Contents;
    blocks[3].style.visibility = Visibility::Visible;
    blocks[3].style.pointer_events = PointerEvents::Auto;
    blocks[6].style.pointer_events = PointerEvents::Auto;
    let frame = layout(&blocks, 30, 8).unwrap();
    assert_eq!(frame.boxes[2].unwrap().content, rect(0, 0, 6, 4));
    assert_eq!(frame.boxes[4].unwrap().content, rect(8, 0, 6, 4));
    assert_eq!(frame.boxes[5].unwrap().content, rect(16, 0, 6, 4));
    let (hits, paints) = policies(&blocks, &[GREY, RED, RED, BLUE, BLUE, GREEN, RED]);
    let canvas = paint(&frame, &paints, Rgb::WHITE).unwrap();
    for ((x, y), color, target) in [
        ((1, 1), GREY, None),
        ((9, 1), BLUE, Some(4)),
        ((17, 1), RED, Some(6)),
        ((20, 3), GREEN, None),
    ] {
        assert_eq!(canvas.pixel(x, y), Some(color));
        assert_eq!(hit_test(&frame, &hits, x, y), Ok(target));
    }
}

#[test]
fn semantic_text_anchors_follow_row_geometry_without_an_application_adapter() {
    let mut elements = [
        Element::new(None, ElementKind::Body, "", 0),
        Element::new(Some(0), ElementKind::Paragraph, "one", 5),
        Element::new(Some(0), ElementKind::Paragraph, "two", 7),
    ];
    elements[0].resolved_style = Some(row(None, 80, 4).style);
    elements[1].resolved_style = Some(item(0, 30, 0).style);
    elements[2].resolved_style = Some(item(0, 20, 0).style);
    let semantic = layout_semantic(&elements, 80, 20).unwrap();
    assert_eq!(semantic.text_boxes.len(), 2);
    assert_eq!(semantic.text_boxes[0].text, "one");
    assert_eq!(semantic.text_boxes[0].content, rect(0, 0, 30, 5));
    assert_eq!(semantic.text_boxes[1].text, "two");
    assert_eq!(semantic.text_boxes[1].content, rect(34, 0, 20, 7));
}

#[test]
fn unsupported_item_sizing_and_fractional_inputs_fail_explicitly() {
    let mut blocks = [row(None, 51, 0), Block::child(0, 2)];
    blocks[1].style.min_width = Some(WidthBound::Px(10));
    assert_eq!(
        layout(&blocks, 60, 20),
        Err(LayoutError::UnsupportedFlexItemWidth { node: 1 })
    );
    blocks[1].style.width = Length::Px(10);
    for auto in [
        HorizontalAutoMargins::Left,
        HorizontalAutoMargins::Right,
        HorizontalAutoMargins::Both,
    ] {
        blocks[1].style.horizontal_auto_margins = auto;
        assert_eq!(
            layout(&blocks, 60, 20),
            Err(LayoutError::UnsupportedFlexItemAutoMargins { node: 1 })
        );
    }
    blocks[1].style.horizontal_auto_margins = HorizontalAutoMargins::None;
    blocks[1].style.width = Length::Percent(50);
    assert_eq!(
        layout(&blocks, 60, 20),
        Err(LayoutError::FractionalPercentageWidth { node: 1 })
    );
    blocks[1].style.width = Length::Px(10);
    blocks[1].style.height = Length::Percent(50);
    assert_eq!(
        layout(&blocks, 60, 20),
        Err(LayoutError::UnsupportedPercentageHeight { node: 1 })
    );
    blocks[1].style.display = Display::None;
    assert!(layout(&blocks, 60, 20).is_ok());
    blocks[1].style.display = Display::Block;
    blocks[1].style.visibility = Visibility::Hidden;
    assert_eq!(
        layout(&blocks, 60, 20),
        Err(LayoutError::UnsupportedPercentageHeight { node: 1 })
    );
}

#[test]
fn row_limits_and_arithmetic_fail_on_a_bounded_native_stack() {
    std::thread::Builder::new()
        .stack_size(2 * 1024 * 1024)
        .spawn(|| {
            let mut deep = vec![row(None, 1, 0)];
            for level in 1..=MAX_DEPTH {
                let mut child = row(Some(level - 1), 1, 0);
                if level == MAX_DEPTH {
                    child.style.display = Display::Block;
                    child.intrinsic_height = 1;
                } else if level % 2 == 0 {
                    child.style.display = Display::Contents;
                }
                deep.push(child);
            }
            let frame = layout(&deep, 1, 1).unwrap();
            assert_eq!(frame.boxes[MAX_DEPTH].unwrap().content, rect(0, 0, 1, 1));
            deep.push(item(MAX_DEPTH, 1, 1));
            assert_eq!(
                layout(&deep, 1, 1),
                Err(LayoutError::TooDeep {
                    node: MAX_DEPTH + 1
                })
            );
            let mut wide = vec![row(None, 1, 0)];
            wide.extend((1..MAX_BLOCKS).map(|_| item(0, 0, 0)));
            assert_eq!(layout(&wide, 1, 1).unwrap().boxes.len(), MAX_BLOCKS);
            wide.push(item(0, 0, 0));
            assert_eq!(layout(&wide, 1, 1), Err(LayoutError::TooManyBlocks));
            let overflow = [row(None, 1, 0), item(0, u32::MAX, 1), item(0, 1, 1)];
            assert_eq!(layout(&overflow, 1, 1), Err(LayoutError::Overflow));
            let gap_overflow = [row(None, 1, u32::MAX), item(0, 1, 1), item(0, 0, 0)];
            assert_eq!(layout(&gap_overflow, 1, 1), Err(LayoutError::Overflow));
        })
        .unwrap()
        .join()
        .unwrap();
}
