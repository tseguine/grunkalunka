//! Independent exact geometry for positive post-flow visual displacements.
//! CSS parsing, negative insets, stacking contexts, and positioned containing
//! blocks remain outside this selected headless normal-flow experiment.
use grunkalunka_layout::hit_test::{HitNode, hit_test, hit_test_scrolled};
use grunkalunka_layout::paint::{PaintNode, Rgb, paint, paint_scrolled};
use grunkalunka_layout::{
    Block, DescendantClip, Display, Edges, HorizontalAutoMargins, LayoutError, Length, Rect,
    RelativeOffset, Style, layout,
};

const ROOT: Rgb = Rgb::new(190, 190, 190);
const RED: Rgb = Rgb::new(240, 30, 30);
const BLUE: Rgb = Rgb::new(20, 40, 230);
const GREEN: Rgb = Rgb::new(30, 185, 40);

fn rect(x: u32, y: u32, width: u32, height: u32) -> Rect {
    Rect {
        x,
        y,
        width,
        height,
    }
}

fn inputs(blocks: &[Block], colors: &[Rgb]) -> (Vec<HitNode>, Vec<PaintNode>) {
    assert_eq!(blocks.len(), colors.len());
    let hits = blocks
        .iter()
        .map(|block| HitNode::new(block.parent))
        .collect();
    let paints = blocks
        .iter()
        .zip(colors)
        .map(|(block, color)| PaintNode {
            parent: block.parent,
            background: Some(*color),
            border: (color == &RED).then_some(Rgb::BLACK),
        })
        .collect();
    (hits, paints)
}

#[test]
fn nested_visual_offsets_shift_descendants_but_not_following_siblings() {
    assert_eq!(Style::default().relative_offset, RelativeOffset::default());
    let blocks = [
        Block {
            style: Style {
                width: Length::Px(80),
                height: Length::Px(40),
                ..Style::default()
            },
            ..Block::default()
        },
        Block {
            parent: Some(0),
            style: Style {
                width: Length::Px(30),
                height: Length::Px(12),
                border: Edges::all(1),
                padding: Edges::all(2),
                margin: Edges {
                    left: 4,
                    top: 2,
                    bottom: 3,
                    ..Edges::default()
                },
                relative_offset: RelativeOffset { x: 10, y: 5 },
                ..Style::default()
            },
            ..Block::default()
        },
        Block {
            parent: Some(1),
            style: Style {
                width: Length::Px(10),
                height: Length::Px(4),
                relative_offset: RelativeOffset { x: 2, y: 3 },
                ..Style::default()
            },
            ..Block::default()
        },
        Block {
            parent: Some(0),
            style: Style {
                width: Length::Px(20),
                height: Length::Px(6),
                margin: Edges {
                    left: 3,
                    ..Edges::default()
                },
                ..Style::default()
            },
            ..Block::default()
        },
    ];
    let frame = layout(&blocks, 100, 70).unwrap();
    assert_eq!(frame.boxes[0].unwrap().content, rect(0, 0, 80, 40));
    assert_eq!(frame.boxes[1].unwrap().margin, rect(10, 5, 40, 23));
    assert_eq!(frame.boxes[1].unwrap().border, rect(14, 7, 36, 18));
    assert_eq!(frame.boxes[1].unwrap().content, rect(17, 10, 30, 12));
    assert_eq!(frame.boxes[2].unwrap().border, rect(19, 13, 10, 4));
    assert_eq!(frame.boxes[3].unwrap().border, rect(3, 23, 20, 6));
    assert_eq!((frame.scroll_width, frame.scroll_height), (100, 70));
    assert_eq!(layout(&blocks, 100, 70), Ok(frame.clone()));

    let (hits, paints) = inputs(&blocks, &[ROOT, RED, BLUE, GREEN]);
    let canvas = paint(&frame, &paints, Rgb::WHITE).unwrap();
    for ((x, y), color, target) in [
        ((4, 2), ROOT, Some(0)),
        ((14, 7), Rgb::BLACK, Some(1)),
        ((19, 13), BLUE, Some(2)),
        ((27, 23), RED, Some(1)),
        ((19, 23), GREEN, Some(3)),
    ] {
        assert_eq!(canvas.pixel(x, y), Some(color), "paint at ({x},{y})");
        assert_eq!(
            hit_test(&frame, &hits, x, y),
            Ok(target),
            "hit at ({x},{y})"
        );
    }
}

#[test]
fn auto_height_ignores_offset_while_visual_overflow_remains_scrollable() {
    let mut blocks = [
        Block {
            style: Style {
                width: Length::Px(40),
                padding: Edges::all(2),
                ..Style::default()
            },
            ..Block::default()
        },
        Block {
            parent: Some(0),
            intrinsic_height: 8,
            style: Style {
                width: Length::Px(15),
                margin: Edges {
                    top: 3,
                    bottom: 4,
                    ..Edges::default()
                },
                relative_offset: RelativeOffset { x: 40, y: 20 },
                ..Style::default()
            },
        },
        Block {
            parent: Some(0),
            style: Style {
                width: Length::Px(20),
                height: Length::Px(6),
                ..Style::default()
            },
            ..Block::default()
        },
    ];
    let frame = layout(&blocks, 60, 30).unwrap();
    assert_eq!(frame.boxes[0].unwrap().border, rect(0, 0, 44, 25));
    assert_eq!(frame.boxes[0].unwrap().content, rect(2, 2, 40, 21));
    assert_eq!(frame.boxes[1].unwrap().border, rect(42, 25, 15, 8));
    assert_eq!(frame.boxes[2].unwrap().border, rect(2, 17, 20, 6));
    assert_eq!((frame.scroll_width, frame.scroll_height), (60, 33));

    let (hits, paints) = inputs(&blocks, &[ROOT, RED, GREEN]);
    let scrolled = paint_scrolled(&frame, &paints, Rgb::WHITE, 0, 3).unwrap();
    assert_eq!(scrolled.pixel(43, 22), Some(RED));
    assert_eq!(hit_test_scrolled(&frame, &hits, 43, 22, 0, 3), Ok(Some(1)));
    assert_eq!(hit_test(&frame, &hits, 3, 18), Ok(Some(2)));

    blocks[1].style.relative_offset = RelativeOffset::default();
    let unshifted = layout(&blocks, 60, 30).unwrap();
    assert_eq!(unshifted.boxes[0], frame.boxes[0]);
    assert_eq!(unshifted.boxes[2], frame.boxes[2]);
    assert_eq!(unshifted.boxes[1].unwrap().border, rect(2, 5, 15, 8));
    assert_eq!(unshifted.scroll_height, 30);
}

#[test]
fn horizontal_auto_margins_resolve_before_visual_translation() {
    let blocks = [
        Block {
            style: Style {
                width: Length::Px(100),
                height: Length::Px(20),
                ..Style::default()
            },
            ..Block::default()
        },
        Block {
            parent: Some(0),
            style: Style {
                width: Length::Px(20),
                height: Length::Px(5),
                horizontal_auto_margins: HorizontalAutoMargins::Both,
                relative_offset: RelativeOffset { x: 5, y: 3 },
                ..Style::default()
            },
            ..Block::default()
        },
        Block::child(0, 7),
    ];
    let frame = layout(&blocks, 120, 30).unwrap();
    assert_eq!(frame.boxes[1].unwrap().margin, rect(5, 3, 100, 5));
    assert_eq!(frame.boxes[1].unwrap().border, rect(45, 3, 20, 5));
    assert_eq!(frame.boxes[2].unwrap().border, rect(0, 5, 100, 7));
}

#[test]
fn boxless_and_hidden_wrappers_ignore_offsets_without_leaking_to_children() {
    let blocks = [
        Block {
            style: Style {
                width: Length::Px(40),
                height: Length::Px(30),
                ..Style::default()
            },
            ..Block::default()
        },
        Block {
            parent: Some(0),
            style: Style {
                display: Display::Contents,
                relative_offset: RelativeOffset {
                    x: u32::MAX,
                    y: u32::MAX,
                },
                margin: Edges::all(u32::MAX),
                ..Style::default()
            },
            ..Block::default()
        },
        Block {
            parent: Some(1),
            style: Style {
                width: Length::Px(10),
                height: Length::Px(5),
                relative_offset: RelativeOffset { x: 4, y: 3 },
                ..Style::default()
            },
            ..Block::default()
        },
        Block {
            parent: Some(0),
            style: Style {
                display: Display::None,
                relative_offset: RelativeOffset {
                    x: u32::MAX,
                    y: u32::MAX,
                },
                ..Style::default()
            },
            ..Block::default()
        },
        Block {
            parent: Some(3),
            style: Style {
                relative_offset: RelativeOffset {
                    x: u32::MAX,
                    y: u32::MAX,
                },
                ..Style::default()
            },
            ..Block::default()
        },
        Block {
            parent: Some(0),
            style: Style {
                width: Length::Px(6),
                height: Length::Px(4),
                ..Style::default()
            },
            ..Block::default()
        },
    ];
    let frame = layout(&blocks, 50, 40).unwrap();
    assert_eq!(frame.boxes[1], None);
    assert!(frame.contents[1]);
    assert_eq!(frame.boxes[2].unwrap().border, rect(4, 3, 10, 5));
    assert_eq!(frame.boxes[3], None);
    assert_eq!(frame.boxes[4], None);
    assert_eq!(frame.boxes[5].unwrap().border, rect(0, 5, 6, 4));

    let boxless_root = [
        Block {
            style: Style {
                display: Display::Contents,
                relative_offset: RelativeOffset {
                    x: u32::MAX,
                    y: u32::MAX,
                },
                ..Style::default()
            },
            ..Block::default()
        },
        Block::child(0, 5),
    ];
    let root_frame = layout(&boxless_root, 30, 20).unwrap();
    assert_eq!(root_frame.boxes[0], None);
    assert_eq!(root_frame.boxes[1].unwrap().border, rect(0, 0, 30, 5));
}

#[test]
fn ancestor_padding_clip_still_restricts_relatively_shifted_descendants() {
    let mut blocks = [
        Block {
            style: Style {
                width: Length::Px(24),
                height: Length::Px(16),
                border: Edges::all(2),
                padding: Edges::all(2),
                descendant_clip: DescendantClip::PaddingBox,
                ..Style::default()
            },
            ..Block::default()
        },
        Block {
            parent: Some(0),
            style: Style {
                width: Length::Px(20),
                height: Length::Px(10),
                relative_offset: RelativeOffset { x: 10, y: 3 },
                ..Style::default()
            },
            ..Block::default()
        },
    ];
    let clipped = layout(&blocks, 32, 24).unwrap();
    assert_eq!(clipped.boxes[0].unwrap().padding, rect(2, 2, 28, 20));
    assert_eq!(clipped.boxes[1].unwrap().border, rect(14, 7, 20, 10));
    assert_eq!((clipped.scroll_width, clipped.scroll_height), (34, 24));
    let (hits, paints) = inputs(&blocks, &[ROOT, BLUE]);
    let pixels = paint(&clipped, &paints, Rgb::WHITE).unwrap();
    assert_eq!(pixels.pixel(29, 9), Some(BLUE));
    assert_eq!(pixels.pixel(30, 9), Some(ROOT));
    assert_eq!(hit_test(&clipped, &hits, 29, 9), Ok(Some(1)));
    assert_eq!(hit_test(&clipped, &hits, 30, 9), Ok(Some(0)));

    blocks[0].style.descendant_clip = DescendantClip::None;
    let unclipped = layout(&blocks, 32, 24).unwrap();
    assert_eq!(clipped.boxes, unclipped.boxes);
    assert_eq!(
        paint(&unclipped, &paints, Rgb::WHITE).unwrap().pixel(30, 9),
        Some(BLUE)
    );
    assert_eq!(hit_test(&unclipped, &hits, 30, 9), Ok(Some(1)));

    let scrolled = paint_scrolled(&clipped, &paints, Rgb::WHITE, 2, 0).unwrap();
    assert_eq!(scrolled.pixel(27, 9), Some(BLUE));
    assert_eq!(hit_test_scrolled(&clipped, &hits, 27, 9, 2, 0), Ok(Some(1)));
    assert_eq!(hit_test_scrolled(&clipped, &hits, 29, 9, 2, 0), Ok(Some(0)));
}

#[test]
fn visual_origin_overflow_is_rejected_instead_of_wrapping() {
    let root = [Block {
        style: Style {
            width: Length::Px(3),
            height: Length::Px(1),
            relative_offset: RelativeOffset { x: u32::MAX, y: 0 },
            ..Style::default()
        },
        ..Block::default()
    }];
    assert_eq!(layout(&root, 20, 20), Err(LayoutError::Overflow));

    let mut oversized = root;
    oversized[0].style.relative_offset = RelativeOffset { x: 0, y: u32::MAX };
    assert_eq!(layout(&oversized, 20, 20), Err(LayoutError::Overflow));

    let nested = [
        Block {
            style: Style {
                width: Length::Px(0),
                relative_offset: RelativeOffset { x: u32::MAX, y: 0 },
                ..Style::default()
            },
            ..Block::default()
        },
        Block {
            parent: Some(0),
            style: Style {
                relative_offset: RelativeOffset { x: 2, y: 0 },
                ..Style::default()
            },
            ..Block::default()
        },
    ];
    assert_eq!(layout(&nested, 20, 20), Err(LayoutError::Overflow));
}
