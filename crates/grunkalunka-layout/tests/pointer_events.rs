//! Caller-resolved, inherited pointer-events policy in native hit testing.
//! These are independent block geometry/input tests, not DOM event dispatch.
use grunkalunka_layout::hit_test::{HitError, HitNode, hit_test, hit_test_scrolled};
use grunkalunka_layout::paint::{PaintNode, Rgb, paint, paint_scrolled};
use grunkalunka_layout::{
    Block, DescendantClip, Display, Edges, Length, PointerEvents, Style, Visibility, layout,
};

const GREY: Rgb = Rgb::new(180, 180, 180);
const RED: Rgb = Rgb::new(235, 30, 30);
const BLUE: Rgb = Rgb::new(30, 45, 225);
const GREEN: Rgb = Rgb::new(30, 170, 55);
const YELLOW: Rgb = Rgb::new(230, 175, 20);
const PURPLE: Rgb = Rgb::new(170, 55, 185);
const CYAN: Rgb = Rgb::new(20, 175, 185);

fn sample() -> [Block; 7] {
    [
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
                pointer_events: PointerEvents::None,
                width: Length::Px(24),
                height: Length::Px(16),
                margin: Edges {
                    left: 3,
                    top: 2,
                    ..Edges::default()
                },
                ..Style::default()
            },
            ..Block::default()
        },
        Block {
            parent: Some(1),
            style: Style {
                width: Length::Px(10),
                height: Length::Px(5),
                ..Style::default()
            },
            ..Block::default()
        },
        Block {
            parent: Some(1),
            style: Style {
                pointer_events: PointerEvents::Auto,
                width: Length::Px(12),
                height: Length::Px(7),
                margin: Edges {
                    top: 1,
                    ..Edges::default()
                },
                ..Style::default()
            },
            ..Block::default()
        },
        Block {
            parent: Some(0),
            style: Style {
                width: Length::Px(15),
                height: Length::Px(6),
                ..Style::default()
            },
            ..Block::default()
        },
        Block {
            parent: Some(4),
            style: Style {
                pointer_events: PointerEvents::None,
                width: Length::Px(8),
                height: Length::Px(3),
                ..Style::default()
            },
            ..Block::default()
        },
        Block {
            parent: Some(5),
            style: Style {
                pointer_events: PointerEvents::Auto,
                width: Length::Px(4),
                height: Length::Px(2),
                ..Style::default()
            },
            ..Block::default()
        },
    ]
}

fn policies(blocks: &[Block]) -> (Vec<HitNode>, Vec<PaintNode>) {
    let colors = [GREY, RED, BLUE, GREEN, YELLOW, PURPLE, CYAN];
    let hits = blocks
        .iter()
        .map(|node| HitNode::new(node.parent))
        .collect();
    let paints = blocks
        .iter()
        .zip(colors)
        .map(|(node, color)| PaintNode {
            parent: node.parent,
            background: Some(color),
            border: None,
        })
        .collect();
    (hits, paints)
}

#[test]
fn inherited_none_skips_only_that_target_and_auto_descendants_restore_hits() {
    assert_eq!(Style::default().pointer_events, PointerEvents::Inherit);
    let blocks = sample();
    let frame = layout(&blocks, 40, 30).unwrap();
    assert_eq!(
        frame.pointer_events,
        [true, false, false, true, true, false, true]
    );
    assert_eq!(frame.boxes[1].unwrap().border.x, 3);
    assert_eq!(frame.boxes[1].unwrap().border.y, 2);
    assert_eq!(frame.boxes[3].unwrap().border.y, 8);
    assert_eq!(frame.boxes[4].unwrap().border.y, 18);
    assert_eq!((frame.scroll_width, frame.scroll_height), (40, 30));
    let (hits, paints) = policies(&blocks);
    for ((x, y), target, color) in [
        ((4, 3), Some(0), BLUE),
        ((4, 10), Some(3), GREEN),
        ((20, 10), Some(0), RED),
        ((2, 19), Some(6), CYAN),
        ((6, 19), Some(4), PURPLE),
        ((12, 20), Some(4), YELLOW),
        ((38, 28), Some(0), GREY),
    ] {
        assert_eq!(hit_test(&frame, &hits, x, y), Ok(target), "hit ({x},{y})");
        assert_eq!(
            paint(&frame, &paints, Rgb::WHITE).unwrap().pixel(x, y),
            Some(color),
            "paint ({x},{y})"
        );
    }
    assert_eq!(hit_test(&frame, &hits, 40, 0), Ok(None));

    let mut enabled = blocks;
    enabled[1].style.pointer_events = PointerEvents::Auto;
    enabled[5].style.pointer_events = PointerEvents::Inherit;
    let full = layout(&enabled, 40, 30).unwrap();
    assert_eq!(frame.boxes, full.boxes);
    assert_eq!(full.pointer_events, [true; 7]);
    assert_eq!(
        paint(&frame, &paints, Rgb::WHITE),
        paint(&full, &paints, Rgb::WHITE)
    );
    assert_eq!(hit_test(&full, &hits, 4, 3), Ok(Some(2)));
    assert_eq!(hit_test(&full, &hits, 6, 19), Ok(Some(5)));

    let mut disabled = hits;
    disabled[3].targetable = false;
    assert_eq!(hit_test(&frame, &disabled, 4, 10), Ok(Some(0)));
}

#[test]
fn root_none_is_inherited_without_preventing_explicit_auto_descendants() {
    let mut blocks = sample();
    blocks[0].style.pointer_events = PointerEvents::None;
    let frame = layout(&blocks, 40, 30).unwrap();
    assert_eq!(
        frame.pointer_events,
        [false, false, false, true, false, false, true]
    );
    let (hits, _) = policies(&blocks);
    assert_eq!(hit_test(&frame, &hits, 4, 3), Ok(None));
    assert_eq!(hit_test(&frame, &hits, 4, 10), Ok(Some(3)));
    assert_eq!(hit_test(&frame, &hits, 2, 19), Ok(Some(6)));
    assert_eq!(hit_test(&frame, &hits, 6, 19), Ok(None));
    assert_eq!(hit_test(&frame, &hits, 38, 28), Ok(None));
}

#[test]
fn contents_and_hidden_ancestors_pass_down_pointer_policy_without_new_boxes() {
    let mut blocks = sample();
    blocks[1].style.display = Display::Contents;
    blocks[1].style.visibility = Visibility::Hidden;
    blocks[3].style.visibility = Visibility::Visible;
    let frame = layout(&blocks, 40, 30).unwrap();
    assert_eq!(frame.contents[1], true);
    assert_eq!(frame.boxes[1], None);
    assert_eq!(
        frame.pointer_events,
        [true, false, false, true, true, false, true]
    );
    assert_eq!(
        frame.visibility,
        [true, false, false, true, true, true, true]
    );
    assert_eq!(frame.boxes[2].unwrap().border.y, 0);
    assert_eq!(frame.boxes[3].unwrap().border.y, 6);
    assert_eq!(frame.boxes[4].unwrap().border.y, 13);
    let (hits, paints) = policies(&blocks);
    assert_eq!(hit_test(&frame, &hits, 1, 1), Ok(Some(0)));
    assert_eq!(hit_test(&frame, &hits, 1, 8), Ok(Some(3)));
    assert_eq!(hit_test(&frame, &hits, 1, 14), Ok(Some(6)));
    let canvas = paint(&frame, &paints, Rgb::WHITE).unwrap();
    assert_eq!(canvas.pixel(1, 1), Some(GREY));
    assert_eq!(canvas.pixel(1, 8), Some(GREEN));

    blocks[1].style.display = Display::None;
    let removed = layout(&blocks, 40, 30).unwrap();
    assert_eq!(removed.contents, [false; 7]);
    assert_eq!(removed.boxes[1], None);
    assert_eq!(removed.boxes[2], None);
    assert_eq!(removed.boxes[3], None);
    assert_eq!(removed.boxes[4].unwrap().border.y, 0);
    assert_eq!(removed.pointer_events[3], true);
    assert_eq!(hit_test(&removed, &hits, 1, 8), Ok(Some(0)));
}

#[test]
fn pointer_events_auto_does_not_escape_a_padding_clip_or_a_scrolled_viewport() {
    let mut blocks = sample();
    blocks[1].style.descendant_clip = DescendantClip::PaddingBox;
    blocks[1].style.border = Edges::all(1);
    blocks[1].style.padding = Edges::all(2);
    blocks[3].style.width = Length::Px(40);
    let frame = layout(&blocks, 40, 30).unwrap();
    let (hits, paints) = policies(&blocks);
    assert_eq!((frame.scroll_width, frame.scroll_height), (46, 30));
    assert_eq!(frame.boxes[3].unwrap().border.x, 6);
    assert_eq!(hit_test(&frame, &hits, 7, 12), Ok(Some(3)));
    assert_eq!(hit_test(&frame, &hits, 35, 12), Ok(Some(0)));
    assert_eq!(hit_test_scrolled(&frame, &hits, 1, 12, 6, 0), Ok(Some(3)));
    assert_eq!(hit_test_scrolled(&frame, &hits, 29, 12, 6, 0), Ok(Some(0)));
    let scrolled = paint_scrolled(&frame, &paints, Rgb::WHITE, 6, 0).unwrap();
    assert_eq!(scrolled.pixel(1, 12), Some(GREEN));
    assert_eq!(scrolled.pixel(29, 12), Some(GREY));
}

#[test]
fn pointer_events_do_not_change_visibility_text_anchor_or_pixel_results() {
    let mut blocks = sample();
    blocks[1].style.visibility = Visibility::Hidden;
    blocks[3].style.visibility = Visibility::Visible;
    let frame = layout(&blocks, 40, 30).unwrap();
    let (hits, paints) = policies(&blocks);
    assert_eq!(hit_test(&frame, &hits, 4, 10), Ok(Some(3)));
    assert_eq!(hit_test(&frame, &hits, 4, 3), Ok(Some(0)));
    let before = paint(&frame, &paints, Rgb::WHITE).unwrap();
    assert_eq!(before.pixel(4, 10), Some(GREEN));
    assert_eq!(before.pixel(4, 3), Some(GREY));

    blocks[3].style.pointer_events = PointerEvents::None;
    let after_frame = layout(&blocks, 40, 30).unwrap();
    assert_eq!(frame.boxes, after_frame.boxes);
    assert_eq!(frame.visibility, after_frame.visibility);
    assert_eq!(hit_test(&after_frame, &hits, 4, 10), Ok(Some(0)));
    assert_eq!(paint(&after_frame, &paints, Rgb::WHITE), Ok(before));

    let mut invalid = frame;
    invalid.pointer_events.pop();
    assert_eq!(
        hit_test(&invalid, &hits, 4, 10),
        Err(HitError::NodeCountMismatch)
    );
    assert!(
        paint(&invalid, &paints, Rgb::WHITE).is_ok(),
        "pointer eligibility is irrelevant to painting"
    );
}
