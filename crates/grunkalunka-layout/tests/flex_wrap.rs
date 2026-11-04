//! Independent literal line-collection, geometry and input expectations for
//! the selected fixed-width wrapping-row profile.
use grunkalunka_layout::hit_test::{HitNode, hit_test, hit_test_scrolled};
use grunkalunka_layout::paint::{PaintNode, Rgb, paint, paint_scrolled};
use grunkalunka_layout::semantic::{Element, ElementKind, layout_semantic};
use grunkalunka_layout::{
    Block, BoxSizing, DescendantClip, Display, Edges, FlexWrap, LayoutError, Length, MAX_BLOCKS,
    MAX_DEPTH, PointerEvents, Rect, RelativeOffset, Style, Visibility, WidthBound, layout,
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

fn row(parent: Option<usize>, width: u32, column_gap: u32, row_gap: u32) -> Block {
    Block {
        parent,
        style: Style {
            display: Display::FlexRow,
            width: Length::Px(width),
            flex_wrap: FlexWrap::Wrap,
            column_gap,
            row_gap,
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
fn exact_fits_share_a_line_and_line_maxima_advance_following_blocks() {
    let mut blocks = [
        Block::default(),
        row(Some(0), 50, 4, 3),
        item(1, 20, 5),
        item(1, 26, 9),
        item(1, 12, 4),
        item(1, 30, 6),
        item(1, 10, 7),
        Block::child(0, 2),
    ];
    blocks[1].style.margin = Edges {
        top: 2,
        bottom: 1,
        left: 1,
        ..Edges::default()
    };
    blocks[1].style.border = Edges::all(1);
    blocks[1].style.padding = Edges::all(2);
    let frame = layout(&blocks, 100, 20).unwrap();
    assert_eq!(frame.boxes[1].unwrap().content, rect(4, 5, 50, 28));
    assert_eq!(frame.boxes[2].unwrap().content, rect(4, 5, 20, 5));
    assert_eq!(frame.boxes[3].unwrap().content, rect(28, 5, 26, 9));
    assert_eq!(frame.boxes[4].unwrap().content, rect(4, 17, 12, 4));
    assert_eq!(frame.boxes[5].unwrap().content, rect(20, 17, 30, 6));
    assert_eq!(frame.boxes[6].unwrap().content, rect(4, 26, 10, 7));
    assert_eq!(frame.boxes[7].unwrap().content, rect(0, 37, 100, 2));
    assert_eq!(frame.boxes[0].unwrap().content.height, 39);
    assert_eq!((frame.scroll_width, frame.scroll_height), (100, 39));
}

#[test]
fn disabling_wrap_and_resizing_recompute_only_the_resolved_geometry() {
    assert_eq!(Style::default().flex_wrap, FlexWrap::NoWrap);
    assert_eq!(Style::default().row_gap, 0);
    let mut blocks = [
        row(None, 40, 2, 7),
        item(0, 20, 5),
        item(0, 20, 8),
        item(0, 10, 4),
    ];
    let wrapped = layout(&blocks, 50, 20).unwrap();
    assert_eq!(wrapped.boxes[1].unwrap().content, rect(0, 0, 20, 5));
    assert_eq!(wrapped.boxes[2].unwrap().content, rect(0, 12, 20, 8));
    assert_eq!(wrapped.boxes[3].unwrap().content, rect(22, 12, 10, 4));
    assert_eq!(wrapped.boxes[0].unwrap().content.height, 20);
    blocks[0].style.flex_wrap = FlexWrap::NoWrap;
    blocks[0].style.row_gap = u32::MAX;
    let single = layout(&blocks, 50, 20).unwrap();
    assert_eq!(single.boxes[2].unwrap().content, rect(22, 0, 20, 8));
    assert_eq!(single.boxes[3].unwrap().content, rect(44, 0, 10, 4));
    assert_eq!(single.boxes[0].unwrap().content.height, 8);
    assert_eq!(single.scroll_width, 54);
    blocks[0].style.flex_wrap = FlexWrap::Wrap;
    blocks[0].style.row_gap = 7;
    assert_eq!(layout(&blocks, 50, 20), Ok(wrapped));
    blocks[0].style.width = Length::Px(54);
    let wider = layout(&blocks, 60, 20).unwrap();
    assert_eq!(wider.boxes[3].unwrap().content, rect(44, 0, 10, 4));
    assert_eq!(wider.boxes[0].unwrap().content.height, 8);
}

#[test]
fn oversized_first_items_and_zero_width_items_do_not_create_phantom_lines() {
    let blocks = [
        row(None, 10, 0, 2),
        item(0, 15, 3),
        item(0, 0, 5),
        item(0, 10, 2),
        item(0, 0, 4),
        item(0, 1, 1),
    ];
    let frame = layout(&blocks, 20, 20).unwrap();
    assert_eq!(frame.boxes[1].unwrap().content, rect(0, 0, 15, 3));
    assert_eq!(frame.boxes[2].unwrap().content, rect(0, 5, 0, 5));
    assert_eq!(frame.boxes[3].unwrap().content, rect(0, 5, 10, 2));
    assert_eq!(frame.boxes[4].unwrap().content, rect(10, 5, 0, 4));
    assert_eq!(frame.boxes[5].unwrap().content, rect(0, 12, 1, 1));
    assert_eq!(frame.boxes[0].unwrap().content.height, 13);

    let mut exact = [row(None, 10, 0, 2), item(0, 10, 3), item(0, 0, 5)];
    let no_gap = layout(&exact, 20, 20).unwrap();
    assert_eq!(no_gap.boxes[2].unwrap().content, rect(10, 0, 0, 5));
    assert_eq!(no_gap.boxes[0].unwrap().content.height, 5);
    exact[0].style.column_gap = 1;
    let gap = layout(&exact, 20, 20).unwrap();
    assert_eq!(gap.boxes[2].unwrap().content, rect(0, 5, 0, 5));
    assert_eq!(gap.boxes[0].unwrap().content.height, 10);

    let zero = [
        row(None, 0, 0, 3),
        item(0, 0, 2),
        item(0, 0, 1),
        item(0, 1, 4),
        item(0, 0, 2),
    ];
    let zero_frame = layout(&zero, 1, 20).unwrap();
    assert_eq!(zero_frame.boxes[2].unwrap().content, rect(0, 0, 0, 1));
    assert_eq!(zero_frame.boxes[3].unwrap().content, rect(0, 5, 1, 4));
    assert_eq!(zero_frame.boxes[4].unwrap().content, rect(0, 12, 0, 2));
    assert_eq!(zero_frame.boxes[0].unwrap().content.height, 14);
}

#[test]
fn line_fit_uses_constrained_outer_sizes_and_ignores_visual_displacements() {
    let mut blocks = [row(None, 100, 4, 3), item(0, 0, 6), item(0, 0, 5)];
    for block in &mut blocks[1..] {
        block.style.width = Length::Percent(50);
        block.style.box_sizing = BoxSizing::BorderBox;
    }
    blocks[1].style.padding = Edges::all(4);
    blocks[1].style.border = Edges::all(1);
    blocks[1].style.margin = Edges::all(2);
    blocks[2].style.padding = Edges::all(1);
    blocks[2].style.border = Edges::all(1);
    blocks[2].style.margin = Edges::all(1);
    blocks[2].style.max_width = Some(WidthBound::Px(45));
    blocks[2].style.min_width = Some(WidthBound::Px(40));
    let frame = layout(&blocks, 100, 40).unwrap();
    assert_eq!(frame.boxes[1].unwrap().margin, rect(0, 0, 54, 20));
    assert_eq!(frame.boxes[1].unwrap().content, rect(7, 7, 40, 6));
    assert_eq!(frame.boxes[2].unwrap().margin, rect(0, 23, 47, 11));
    assert_eq!(frame.boxes[2].unwrap().content, rect(3, 26, 41, 5));
    assert_eq!(frame.boxes[0].unwrap().content.height, 34);
    blocks[1].style.relative_offset = RelativeOffset { x: 100, y: 100 };
    let shifted = layout(&blocks, 100, 40).unwrap();
    assert_eq!(shifted.boxes[1].unwrap().margin, rect(100, 100, 54, 20));
    assert_eq!(shifted.boxes[0], frame.boxes[0]);
    assert_eq!(shifted.boxes[2], frame.boxes[2]);
    assert_eq!((shifted.scroll_width, shifted.scroll_height), (154, 120));
}

#[test]
fn flattened_contents_keep_tree_order_visibility_slots_and_pointer_overrides() {
    let mut blocks = [
        row(None, 12, 2, 1),
        Block::child(0, 0),
        item(0, 6, 4),
        Block::child(1, 0),
        Block::child(1, 0),
        item(3, 6, 3),
        item(1, 4, 5),
        Block::child(4, u32::MAX),
        Block::child(0, 0),
    ];
    blocks[0].style.pointer_events = PointerEvents::None;
    for node in [1, 3, 8] {
        blocks[node].style.display = Display::Contents;
        blocks[node].style.flex_wrap = FlexWrap::Wrap;
        blocks[node].style.row_gap = u32::MAX;
    }
    blocks[1].style.visibility = Visibility::Hidden;
    blocks[2].style.pointer_events = PointerEvents::Auto;
    blocks[4].style.display = Display::None;
    blocks[5].style.visibility = Visibility::Visible;
    blocks[5].style.pointer_events = PointerEvents::Auto;
    let frame = layout(&blocks, 14, 12).unwrap();
    assert_eq!(frame.boxes[5].unwrap().content, rect(0, 0, 6, 3));
    assert_eq!(frame.boxes[6].unwrap().content, rect(8, 0, 4, 5));
    assert_eq!(frame.boxes[2].unwrap().content, rect(0, 6, 6, 4));
    assert_eq!(frame.boxes[0].unwrap().content.height, 10);
    assert!(frame.boxes[7].is_none());
    let (hits, paints) = policies(&blocks, &[GREY, RED, GREEN, RED, RED, BLUE, RED, RED, RED]);
    let canvas = paint(&frame, &paints, Rgb::WHITE).unwrap();
    for ((x, y), color, target) in [
        ((1, 1), BLUE, Some(5)),
        ((9, 1), GREY, None),
        ((1, 7), GREEN, Some(2)),
    ] {
        assert_eq!(canvas.pixel(x, y), Some(color));
        assert_eq!(hit_test(&frame, &hits, x, y), Ok(target));
    }
    blocks[6].style.display = Display::None;
    let pruned = layout(&blocks, 14, 12).unwrap();
    assert_eq!(pruned.boxes[2].unwrap().content, rect(0, 4, 6, 4));
    assert_eq!(pruned.boxes[0].unwrap().content.height, 8);
}

#[test]
fn nested_wrapping_rows_report_their_complete_height_to_the_parent_line() {
    let blocks = [
        Block::default(),
        row(Some(0), 25, 2, 3),
        row(Some(1), 14, 2, 1),
        item(1, 9, 4),
        item(2, 8, 2),
        item(2, 8, 5),
        item(1, 12, 3),
        Block::child(0, 2),
    ];
    let frame = layout(&blocks, 40, 20).unwrap();
    assert_eq!(frame.boxes[2].unwrap().content, rect(0, 0, 14, 8));
    assert_eq!(frame.boxes[3].unwrap().content, rect(16, 0, 9, 4));
    assert_eq!(frame.boxes[4].unwrap().content, rect(0, 0, 8, 2));
    assert_eq!(frame.boxes[5].unwrap().content, rect(0, 3, 8, 5));
    assert_eq!(frame.boxes[6].unwrap().content, rect(0, 11, 12, 3));
    assert_eq!(frame.boxes[1].unwrap().content.height, 14);
    assert_eq!(frame.boxes[7].unwrap().content, rect(0, 14, 40, 2));
}

#[test]
fn wrapped_overflow_keeps_document_scroll_and_padding_clip_input_consistent() {
    let mut blocks = [
        row(None, 14, 2, 3),
        item(0, 6, 4),
        item(0, 6, 6),
        item(0, 6, 8),
    ];
    blocks[0].style.height = Length::Px(12);
    blocks[0].style.padding = Edges::all(2);
    blocks[0].style.border = Edges::all(1);
    blocks[0].style.descendant_clip = DescendantClip::PaddingBox;
    let frame = layout(&blocks, 18, 12).unwrap();
    assert_eq!(frame.boxes[3].unwrap().content, rect(3, 12, 6, 8));
    assert_eq!(frame.boxes[0].unwrap().border, rect(0, 0, 20, 18));
    assert_eq!((frame.scroll_width, frame.scroll_height), (20, 20));
    let (hits, paints) = policies(&blocks, &[GREY, RED, GREEN, BLUE]);
    let canvas = paint(&frame, &paints, Rgb::WHITE).unwrap();
    assert_eq!(canvas.pixel(12, 4), Some(GREEN));
    assert_eq!(hit_test(&frame, &hits, 12, 4), Ok(Some(2)));
    let scrolled = paint_scrolled(&frame, &paints, Rgb::WHITE, 0, 8).unwrap();
    for (y, color, target) in [
        (4, BLUE, Some(3)),
        (8, BLUE, Some(3)),
        (9, Rgb::BLACK, Some(0)),
        (10, Rgb::WHITE, None),
    ] {
        assert_eq!(scrolled.pixel(4, y), Some(color));
        assert_eq!(hit_test_scrolled(&frame, &hits, 4, y, 0, 8), Ok(target));
    }
}

#[test]
fn semantic_anchors_reflow_on_wrapped_lines_and_after_hiding_an_item() {
    let mut elements = [
        Element::new(None, ElementKind::Body, "", 0),
        Element::new(Some(0), ElementKind::Paragraph, "alpha", 4),
        Element::new(Some(0), ElementKind::Paragraph, "beta", 6),
        Element::new(Some(0), ElementKind::Paragraph, "gamma", 5),
    ];
    elements[0].resolved_style = Some(row(None, 20, 2, 3).style);
    elements[1].resolved_style = Some(item(0, 9, 0).style);
    elements[2].resolved_style = Some(item(0, 9, 0).style);
    elements[3].resolved_style = Some(item(0, 20, 0).style);
    let frame = layout_semantic(&elements, 30, 20).unwrap();
    assert_eq!(frame.text_boxes[0].content, rect(0, 0, 9, 4));
    assert_eq!(frame.text_boxes[1].content, rect(11, 0, 9, 6));
    assert_eq!(frame.text_boxes[2].content, rect(0, 9, 20, 5));
    elements[2].hidden = true;
    let hidden = layout_semantic(&elements, 30, 20).unwrap();
    assert_eq!(hidden.text_boxes.len(), 2);
    assert_eq!(hidden.text_boxes[1].text, "gamma");
    assert_eq!(hidden.text_boxes[1].content, rect(0, 7, 20, 5));
}

#[test]
fn discarded_and_inactive_gaps_do_not_overflow_but_actual_line_positions_do() {
    let oversized = [
        row(None, 1, u32::MAX, 0),
        item(0, u32::MAX, 1),
        item(0, u32::MAX, 1),
    ];
    let frame = layout(&oversized, 1, 1).unwrap();
    assert_eq!(frame.boxes[1].unwrap().content, rect(0, 0, u32::MAX, 1));
    assert_eq!(frame.boxes[2].unwrap().content, rect(0, 1, u32::MAX, 1));
    assert_eq!((frame.scroll_width, frame.scroll_height), (u32::MAX, 2));
    let mut blocks = [row(None, 1, u32::MAX, u32::MAX), item(0, 1, 1)];
    assert_eq!(
        layout(&blocks, 1, 1).unwrap().boxes[0]
            .unwrap()
            .content
            .height,
        1
    );
    blocks[1].style.display = Display::None;
    assert_eq!(
        layout(&blocks, 1, 1).unwrap().boxes[0]
            .unwrap()
            .content
            .height,
        0
    );
    let overflow = [row(None, 1, 0, u32::MAX), item(0, 1, 1), item(0, 1, 1)];
    assert_eq!(layout(&overflow, 1, 1), Err(LayoutError::Overflow));
    let mut vertical = [
        row(None, 1, u32::MAX, u32::MAX),
        Block::child(0, 2),
        Block::child(0, 3),
    ];
    vertical[0].style.display = Display::Block;
    let block_frame = layout(&vertical, 1, 10).unwrap();
    assert_eq!(block_frame.boxes[2].unwrap().content, rect(0, 2, 1, 3));
}

#[test]
fn deep_and_wide_wrapping_preserve_existing_quotas_on_a_small_native_stack() {
    std::thread::Builder::new()
        .stack_size(2 * 1024 * 1024)
        .spawn(|| {
            let mut deep = vec![row(None, 1, 0, 1)];
            for level in 1..MAX_DEPTH {
                deep.push(row(Some(level - 1), 1, 0, 1));
            }
            deep.push(item(MAX_DEPTH - 1, 1, 1));
            for parent in 0..MAX_DEPTH {
                deep.push(item(parent, 1, 1));
            }
            let frame = layout(&deep, 1, 1).unwrap();
            assert_eq!(frame.boxes[MAX_DEPTH].unwrap().content, rect(0, 0, 1, 1));
            assert_eq!(frame.boxes[0].unwrap().content.height, 129);
            assert_eq!(frame.scroll_height, 129);
            let invalid_node = deep.len();
            deep.push(item(MAX_DEPTH, 1, 1));
            assert_eq!(
                layout(&deep, 1, 1),
                Err(LayoutError::TooDeep { node: invalid_node })
            );
            let mut wide = vec![row(None, 1, 0, 1)];
            wide.extend((1..MAX_BLOCKS).map(|_| item(0, 1, 1)));
            let frame = layout(&wide, 1, 1).unwrap();
            assert_eq!(
                frame.boxes[MAX_BLOCKS - 1].unwrap().content,
                rect(0, 8188, 1, 1)
            );
            assert_eq!(frame.scroll_height, 8189);
            wide.push(item(0, 1, 1));
            assert_eq!(layout(&wide, 1, 1), Err(LayoutError::TooManyBlocks));
        })
        .unwrap()
        .join()
        .unwrap();
}
