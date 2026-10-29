//! Caller-resolved descendant clipping of paint and input, not CSS overflow.
//! Fixed integer geometry, nested padding-box clips, and scroll translation.
use grunkalunka_layout::hit_test::{HitError, HitNode, hit_test, hit_test_scrolled};
use grunkalunka_layout::paint::{PaintError, PaintNode, Rgb, paint, paint_scrolled};
use grunkalunka_layout::{Block, DescendantClip, Edges, Length, Rect, Style, Visibility, layout};

const CLEAR: Rgb = Rgb::WHITE;
const ROOT: Rgb = Rgb::new(180, 180, 180);
const PARENT: Rgb = Rgb::new(240, 30, 30);
const BLUE: Rgb = Rgb::new(30, 60, 220);
const GREEN: Rgb = Rgb::new(30, 180, 60);
const SIBLING: Rgb = Rgb::new(225, 140, 20);
const BLACK: Rgb = Rgb::BLACK;

fn sample() -> [Block; 5] {
    [
        Block {
            style: Style {
                width: Length::Px(30),
                height: Length::Px(26),
                ..Style::default()
            },
            ..Block::default()
        },
        Block {
            parent: Some(0),
            style: Style {
                descendant_clip: DescendantClip::PaddingBox,
                width: Length::Px(12),
                height: Length::Px(10),
                margin: Edges {
                    top: 2,
                    left: 4,
                    ..Edges::default()
                },
                border: Edges::all(1),
                padding: Edges::all(2),
                ..Style::default()
            },
            ..Block::default()
        },
        Block {
            parent: Some(1),
            style: Style {
                width: Length::Px(26),
                height: Length::Px(16),
                ..Style::default()
            },
            ..Block::default()
        },
        Block {
            parent: Some(2),
            style: Style {
                width: Length::Px(4),
                height: Length::Px(3),
                margin: Edges {
                    top: 14,
                    left: 10,
                    ..Edges::default()
                },
                ..Style::default()
            },
            ..Block::default()
        },
        Block {
            parent: Some(0),
            style: Style {
                width: Length::Px(6),
                height: Length::Px(5),
                ..Style::default()
            },
            ..Block::default()
        },
    ]
}

fn inputs(blocks: &[Block], colors: &[Rgb]) -> (Vec<HitNode>, Vec<PaintNode>) {
    let hits = blocks
        .iter()
        .map(|node| HitNode::new(node.parent))
        .collect();
    let paints = blocks
        .iter()
        .zip(colors)
        .map(|(node, color)| PaintNode {
            parent: node.parent,
            background: Some(*color),
            border: (color == &PARENT).then_some(BLACK),
        })
        .collect();
    (hits, paints)
}

#[test]
fn padding_clip_changes_only_descendant_pixels_and_hits_not_layout_geometry() {
    let original = sample();
    let clipped = layout(&original, 30, 26).unwrap();
    let mut without = original;
    without[1].style.descendant_clip = DescendantClip::None;
    let unclipped = layout(&without, 30, 26).unwrap();

    assert_eq!(
        clipped.boxes, unclipped.boxes,
        "no reflow or clipped box mutation"
    );
    assert_eq!(clipped.visibility, unclipped.visibility);
    assert_eq!((clipped.scroll_width, clipped.scroll_height), (33, 26));
    assert_eq!(clipped.clip_descendants, [false, true, false, false, false]);
    assert_eq!(unclipped.clip_descendants, [false; 5]);
    assert_eq!(
        clipped.boxes[1].unwrap().padding,
        Rect {
            x: 5,
            y: 3,
            width: 16,
            height: 14,
        }
    );
    assert_eq!(
        clipped.boxes[2].unwrap().border,
        Rect {
            x: 7,
            y: 5,
            width: 26,
            height: 16,
        }
    );
    assert_eq!(
        clipped.boxes[3].unwrap().border,
        Rect {
            x: 17,
            y: 19,
            width: 4,
            height: 3,
        }
    );
    assert_eq!(clipped.boxes[4].unwrap().border.y, 18);

    let (hits, paints) = inputs(&original, &[ROOT, PARENT, BLUE, GREEN, SIBLING]);
    let surface = paint(&clipped, &paints, CLEAR).unwrap();
    let baseline = paint(&unclipped, &paints, CLEAR).unwrap();
    for ((x, y), expected, target) in [
        ((4, 2), BLACK, Some(1)),
        ((5, 4), PARENT, Some(1)),
        ((8, 6), BLUE, Some(2)),
        ((20, 16), BLUE, Some(2)),
        ((21, 6), ROOT, Some(0)),
        ((8, 18), ROOT, Some(0)),
        ((17, 19), ROOT, Some(0)),
        ((2, 19), SIBLING, Some(4)),
    ] {
        assert_eq!(
            surface.pixel(x, y),
            Some(expected),
            "clipped pixel ({x},{y})"
        );
        assert_eq!(hit_test(&clipped, &hits, x, y), Ok(target), "hit ({x},{y})");
    }
    assert_eq!(baseline.pixel(22, 6), Some(BLUE));
    assert_eq!(baseline.pixel(17, 19), Some(GREEN));
    assert_eq!(hit_test(&unclipped, &hits, 22, 6), Ok(Some(2)));
    assert_eq!(hit_test(&unclipped, &hits, 17, 19), Ok(Some(3)));
}

#[test]
fn nested_clips_intersect_and_an_empty_intersection_stays_empty() {
    let blocks = [
        Block {
            style: Style {
                width: Length::Px(50),
                height: Length::Px(30),
                ..Style::default()
            },
            ..Block::default()
        },
        Block {
            parent: Some(0),
            style: Style {
                descendant_clip: DescendantClip::PaddingBox,
                width: Length::Px(12),
                height: Length::Px(8),
                margin: Edges {
                    left: 10,
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
                descendant_clip: DescendantClip::PaddingBox,
                width: Length::Px(15),
                height: Length::Px(12),
                margin: Edges {
                    left: 5,
                    top: 3,
                    ..Edges::default()
                },
                ..Style::default()
            },
            ..Block::default()
        },
        Block {
            parent: Some(2),
            style: Style {
                width: Length::Px(20),
                height: Length::Px(12),
                ..Style::default()
            },
            ..Block::default()
        },
    ];
    let frame = layout(&blocks, 50, 30).unwrap();
    let (hits, paints) = inputs(&blocks, &[ROOT, PARENT, BLUE, GREEN]);
    let canvas = paint(&frame, &paints, CLEAR).unwrap();
    for ((x, y), expected, target) in [
        ((15, 5), GREEN, Some(3)),
        ((21, 9), GREEN, Some(3)),
        ((14, 7), PARENT, Some(1)),
        ((22, 9), ROOT, Some(0)),
        ((20, 10), ROOT, Some(0)),
        ((23, 6), ROOT, Some(0)),
    ] {
        assert_eq!(canvas.pixel(x, y), Some(expected), "nested clip ({x},{y})");
        assert_eq!(hit_test(&frame, &hits, x, y), Ok(target));
    }

    // A second clip wholly outside the first may not reset an empty
    // inherited intersection and revive its descendants.
    let mut disjoint = blocks;
    disjoint[2].style.margin.left = 20;
    let frame = layout(&disjoint, 50, 30).unwrap();
    let canvas = paint(&frame, &paints, CLEAR).unwrap();
    assert_eq!(canvas.pixel(31, 6), Some(ROOT));
    assert_eq!(hit_test(&frame, &hits, 31, 6), Ok(Some(0)));
}

#[test]
fn clipping_is_independent_of_inherited_visibility_and_input_targetability() {
    let mut blocks = sample();
    blocks[1].style.visibility = Visibility::Hidden;
    blocks[2].style.visibility = Visibility::Visible;
    let frame = layout(&blocks, 30, 26).unwrap();
    let (mut hits, paints) = inputs(&blocks, &[ROOT, PARENT, BLUE, GREEN, SIBLING]);
    assert_eq!(frame.visibility, [true, false, true, true, true]);
    assert_eq!(hit_test(&frame, &hits, 8, 6), Ok(Some(2)));
    assert_eq!(hit_test(&frame, &hits, 22, 6), Ok(Some(0)));
    let canvas = paint(&frame, &paints, CLEAR).unwrap();
    assert_eq!(canvas.pixel(8, 6), Some(BLUE));
    assert_eq!(canvas.pixel(4, 2), Some(ROOT));
    hits[2].targetable = false;
    assert_eq!(hit_test(&frame, &hits, 8, 6), Ok(Some(0)));
    assert_eq!(paint(&frame, &paints, CLEAR), Ok(canvas));

    blocks[1].style.display = grunkalunka_layout::Display::None;
    let hidden = layout(&blocks, 30, 26).unwrap();
    assert_eq!(hidden.boxes[2], None);
    assert_eq!(hidden.boxes[3], None);
    assert_eq!(hidden.boxes[4].unwrap().border.y, 0);
}

#[test]
fn document_scroll_translation_applies_to_the_same_padding_box_clip() {
    let blocks = sample();
    let frame = layout(&blocks, 30, 26).unwrap();
    let (hits, paints) = inputs(&blocks, &[ROOT, PARENT, BLUE, GREEN, SIBLING]);
    // The old extents deliberately remain independent of this paint/input-only
    // clipping slice. The child extends to document x=33; max scroll x is 3.
    let canvas = paint_scrolled(&frame, &paints, CLEAR, 3, 0).unwrap();
    assert_eq!(canvas.pixel(4, 6), Some(BLUE));
    assert_eq!(canvas.pixel(19, 6), Some(ROOT));
    assert_eq!(hit_test_scrolled(&frame, &hits, 4, 6, 3, 0), Ok(Some(2)));
    assert_eq!(hit_test_scrolled(&frame, &hits, 19, 6, 3, 0), Ok(Some(0)));
}

#[test]
fn absent_clip_metadata_and_overflowing_intersections_fail_closed() {
    let blocks = sample();
    let mut frame = layout(&blocks, 30, 26).unwrap();
    let (hits, paints) = inputs(&blocks, &[ROOT, PARENT, BLUE, GREEN, SIBLING]);
    frame.clip_descendants.pop();
    assert_eq!(
        hit_test(&frame, &hits, 8, 6),
        Err(HitError::NodeCountMismatch)
    );
    assert_eq!(
        paint(&frame, &paints, CLEAR),
        Err(PaintError::NodeCountMismatch)
    );

    let a = Rect {
        x: 10,
        y: 10,
        width: 3,
        height: 3,
    };
    let b = Rect {
        x: 20,
        y: 11,
        width: 2,
        height: 2,
    };
    assert_eq!(
        a.intersection(b),
        Some(Rect {
            x: 20,
            y: 11,
            width: 0,
            height: 2,
        })
    );
    assert_eq!(
        a.intersection(Rect {
            x: u32::MAX,
            y: 0,
            width: 1,
            height: 1,
        }),
        None
    );

    let mut invalid = layout(&blocks, 30, 26).unwrap();
    invalid.boxes[1].as_mut().unwrap().padding.x = u32::MAX;
    assert_eq!(hit_test(&invalid, &hits, 8, 6), Err(HitError::Overflow));
    assert_eq!(paint(&invalid, &paints, CLEAR), Err(PaintError::Overflow));
}
