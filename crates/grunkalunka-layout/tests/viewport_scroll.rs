//! Explicit viewport-origin scroll translation across box paint, bitmap ink,
//! and hit testing. These tests do not claim CSS scrolling conformance.

use grunkalunka_layout::hit_test::{HitError, HitNode, hit_test, hit_test_scrolled};
use grunkalunka_layout::paint::{PaintError, PaintNode, Rgb, paint, paint_scrolled};
use grunkalunka_layout::{Block, Edges, Frame, Length, Style, layout};

const ROOT: Rgb = Rgb::new(220, 30, 30);
const FIRST: Rgb = Rgb::new(30, 50, 220);
const SECOND: Rgb = Rgb::new(30, 190, 60);
const INK: Rgb = Rgb::BLACK;

fn document() -> (Frame, [HitNode; 3], [PaintNode; 3]) {
    // A 40x30 viewport contains a fixed-height root. Its first child starts
    // below that root, extends to x=55, and the second continues down to y=70.
    let blocks = [
        Block {
            style: Style {
                width: Length::Px(40),
                height: Length::Px(20),
                ..Style::default()
            },
            ..Block::default()
        },
        Block {
            parent: Some(0),
            style: Style {
                width: Length::Px(35),
                height: Length::Px(20),
                margin: Edges {
                    top: 25,
                    left: 20,
                    ..Edges::default()
                },
                ..Style::default()
            },
            ..Block::default()
        },
        Block {
            parent: Some(0),
            style: Style {
                width: Length::Px(10),
                height: Length::Px(25),
                margin: Edges {
                    left: 5,
                    ..Edges::default()
                },
                ..Style::default()
            },
            ..Block::default()
        },
    ];
    let frame = layout(&blocks, 40, 30).unwrap();
    assert_eq!((frame.scroll_width, frame.scroll_height), (55, 70));
    let hits = [
        HitNode::new(None),
        HitNode::new(Some(0)),
        HitNode::new(Some(0)),
    ];
    let paints = [
        PaintNode {
            parent: None,
            background: Some(ROOT),
            border: None,
        },
        PaintNode {
            parent: Some(0),
            background: Some(FIRST),
            border: None,
        },
        PaintNode {
            parent: Some(0),
            background: Some(SECOND),
            border: None,
        },
    ];
    (frame, hits, paints)
}

#[test]
fn zero_offsets_preserve_existing_pixels_and_hit_targets() {
    let (frame, hits, paints) = document();
    assert_eq!(
        paint(&frame, &paints, Rgb::WHITE),
        paint_scrolled(&frame, &paints, Rgb::WHITE, 0, 0)
    );
    for (x, y) in [(0, 0), (12, 3), (21, 26), (39, 29), (40, 0)] {
        assert_eq!(
            hit_test(&frame, &hits, x, y),
            hit_test_scrolled(&frame, &hits, x, y, 0, 0)
        );
    }
    assert_eq!(hit_test(&frame, &hits, 12, 3), Ok(Some(0)));
}

#[test]
fn scrolled_box_pixels_and_targets_use_the_same_document_coordinates() {
    let (frame, hits, paints) = document();
    let scrolled = paint_scrolled(&frame, &paints, Rgb::WHITE, 10, 30).unwrap();
    for ((x, y), pixel, target) in [
        ((12, 3), FIRST, Some(1)),
        ((39, 3), FIRST, Some(1)),
        ((1, 18), SECOND, Some(2)),
        ((1, 3), Rgb::WHITE, None),
        ((10, 15), Rgb::WHITE, None),
    ] {
        assert_eq!(scrolled.pixel(x, y), Some(pixel), "pixel ({x},{y})");
        assert_eq!(
            hit_test_scrolled(&frame, &hits, x, y, 10, 30),
            Ok(target),
            "hit ({x},{y})"
        );
    }
    assert_eq!(hit_test_scrolled(&frame, &hits, 40, 3, 10, 30), Ok(None));
    assert_eq!(scrolled.pixel(40, 3), None);
    // Independent zero-y offset retains visible root paint and target.
    let top = paint_scrolled(&frame, &paints, Rgb::WHITE, 10, 0).unwrap();
    assert_eq!(top.pixel(29, 10), Some(ROOT));
    assert_eq!(top.pixel(30, 10), Some(Rgb::WHITE));
    assert_eq!(hit_test_scrolled(&frame, &hits, 29, 10, 10, 0), Ok(Some(0)));
    assert_eq!(hit_test_scrolled(&frame, &hits, 30, 10, 10, 0), Ok(None));
}

#[test]
fn scrolled_bitmap_ink_clips_left_top_right_and_bottom() {
    let (frame, _, paints) = document();
    let mut canvas = paint_scrolled(&frame, &paints, Rgb::WHITE, 10, 30).unwrap();
    // The source mask is 2x2 at document x=9. Its first column is offscreen,
    // and the second column lands on viewport x=0.
    canvas
        .blit_mono(9, 30, 2, 2, &[0b1100_0000, 0b0100_0000], INK)
        .unwrap();
    assert_eq!(canvas.pixel(0, 0), Some(INK));
    assert_eq!(canvas.pixel(0, 1), Some(INK));
    assert_eq!(canvas.pixel(1, 0), Some(Rgb::WHITE));

    // Three mask rows/columns at the far right/bottom only expose 2x2 pixels.
    canvas
        .blit_mono(48, 58, 3, 3, &[0b1110_0000; 3], INK)
        .unwrap();
    for (x, y) in [(38, 28), (39, 28), (38, 29), (39, 29)] {
        assert_eq!(canvas.pixel(x, y), Some(INK));
    }
    assert_eq!(canvas.pixel(37, 28), Some(Rgb::WHITE));
    assert_eq!(canvas.pixel(39, 27), Some(Rgb::WHITE));

    let unchanged = canvas.clone();
    assert_eq!(
        canvas.blit_mono(9, 30, 2, 2, &[], INK),
        Err(PaintError::InvalidBitmap)
    );
    assert_eq!(canvas, unchanged, "invalid input must not mutate paint");
}

#[test]
fn offsets_beyond_document_extents_fail_instead_of_clamping() {
    let (frame, hits, paints) = document();
    for (x, y) in [(16, 0), (0, 41), (u32::MAX, 0), (0, u32::MAX)] {
        assert_eq!(
            paint_scrolled(&frame, &paints, Rgb::WHITE, x, y),
            Err(PaintError::InvalidScrollOffset)
        );
        assert_eq!(
            hit_test_scrolled(&frame, &hits, 1, 1, x, y),
            Err(HitError::InvalidScrollOffset)
        );
    }
    assert!(paint_scrolled(&frame, &paints, Rgb::WHITE, 15, 40).is_ok());
    assert!(hit_test_scrolled(&frame, &hits, 1, 1, 15, 40).is_ok());
}
