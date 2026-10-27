use super::*;
use crate::{Block, Display, Edges, LayoutError, Length, Style, layout};

const RED: Rgb = Rgb::new(255, 0, 0);
const BLUE: Rgb = Rgb::new(0, 0, 255);
const GREEN: Rgb = Rgb::new(0, 255, 0);

fn painted(parent: Option<usize>, background: Option<Rgb>, border: Option<Rgb>) -> PaintNode {
    PaintNode {
        parent,
        background,
        border,
    }
}

#[test]
fn draws_border_strips_and_backgrounds_without_painting_margins() {
    let blocks = [
        Block {
            style: Style {
                border: Edges::all(1),
                padding: Edges::all(2),
                ..Style::default()
            },
            ..Block::default()
        },
        Block::child(0, 2),
    ];
    let frame = layout(&blocks, 12, 8).unwrap();
    assert_eq!(frame.boxes[0].unwrap().content.width, 6);
    let paints = [
        painted(None, Some(Rgb::WHITE), Some(Rgb::BLACK)),
        painted(Some(0), Some(RED), None),
    ];
    let surface = paint(&frame, &paints, BLUE).unwrap();
    assert_eq!(surface.width(), 12);
    assert_eq!(surface.height(), 8);
    assert_eq!(surface.pixel(0, 0), Some(Rgb::BLACK));
    assert_eq!(surface.pixel(1, 1), Some(Rgb::WHITE));
    assert_eq!(surface.pixel(3, 3), Some(RED));
    assert_eq!(surface.pixel(8, 4), Some(RED));
    assert_eq!(surface.pixel(9, 4), Some(Rgb::WHITE));
    assert_eq!(surface.pixel(11, 6), Some(Rgb::BLACK));
    assert_eq!(surface.pixel(12, 0), None);
    assert_eq!(surface.pixel(0, 8), None);
    assert_eq!(paint(&frame, &paints, BLUE).unwrap(), surface);
}

#[test]
fn paints_depth_first_even_when_flat_input_is_not_preorder() {
    let blocks = [
        Block {
            style: Style {
                height: Length::Px(12),
                ..Style::default()
            },
            ..Block::default()
        },
        Block {
            parent: Some(0),
            style: Style {
                height: Length::Px(4),
                ..Style::default()
            },
            ..Block::default()
        },
        Block::child(0, 8),
        Block::child(1, 12),
    ];
    let frame = layout(&blocks, 10, 12).unwrap();
    let paints = [
        painted(None, Some(Rgb::WHITE), None),
        painted(Some(0), Some(RED), None),
        painted(Some(0), Some(BLUE), None),
        painted(Some(1), Some(GREEN), None),
    ];
    let canvas = paint(&frame, &paints, Rgb::BLACK).unwrap();
    assert_eq!(canvas.pixel(0, 2), Some(GREEN));
    assert_eq!(canvas.pixel(0, 6), Some(BLUE));
    assert_eq!(canvas.pixel(9, 11), Some(BLUE));
}

#[test]
fn clips_overflow_to_viewport_and_drops_hidden_subtrees() {
    let blocks = [
        Block {
            style: Style {
                width: Length::Px(80),
                height: Length::Px(6),
                ..Style::default()
            },
            ..Block::default()
        },
        Block {
            parent: Some(0),
            style: Style {
                display: Display::None,
                ..Style::default()
            },
            ..Block::default()
        },
        Block::child(1, 20),
        Block::child(0, 2),
    ];
    let frame = layout(&blocks, 20, 10).unwrap();
    let paints = [
        painted(None, Some(RED), None),
        painted(Some(0), Some(GREEN), None),
        painted(Some(1), Some(GREEN), None),
        painted(Some(0), Some(BLUE), None),
    ];
    assert!(frame.boxes[1].is_none() && frame.boxes[2].is_none());
    let surface = paint(&frame, &paints, Rgb::WHITE).unwrap();
    assert_eq!(surface.pixel(19, 0), Some(BLUE));
    assert_eq!(surface.pixel(19, 4), Some(RED));
    assert_eq!(surface.pixel(19, 8), Some(Rgb::WHITE));
    assert_eq!(frame.scroll_width, 80);
}

#[test]
fn emits_deterministic_binary_ppm_from_pixels() {
    let frame = layout(
        &[Block {
            intrinsic_height: 1,
            ..Block::default()
        }],
        2,
        1,
    )
    .unwrap();
    let surface = paint(&frame, &[painted(None, Some(RED), None)], Rgb::WHITE).unwrap();
    let mut expected = b"P6\n2 1\n255\n".to_vec();
    expected.extend_from_slice(&[255, 0, 0, 255, 0, 0]);
    assert_eq!(surface.ppm(), expected);
}

#[test]
fn rejects_corrupt_geometry_and_tree_metadata() {
    let frame = layout(&[Block::default(), Block::child(0, 3)], 10, 10).unwrap();
    let paints = [
        painted(None, Some(RED), None),
        painted(Some(0), Some(BLUE), None),
    ];
    assert_eq!(
        paint(&frame, &paints[..1], Rgb::WHITE),
        Err(PaintError::NodeCountMismatch)
    );
    let mut wrong = paints;
    wrong[1].parent = Some(1);
    assert_eq!(
        paint(&frame, &wrong, Rgb::WHITE),
        Err(PaintError::InvalidParent { node: 1 })
    );
    let mut wrong = paints;
    wrong[0].parent = Some(0);
    assert_eq!(
        paint(&frame, &wrong, Rgb::WHITE),
        Err(PaintError::InvalidParent { node: 0 })
    );
    let mut hidden_parent = frame.clone();
    hidden_parent.boxes[0] = None;
    assert_eq!(
        paint(&hidden_parent, &paints, Rgb::WHITE),
        Err(PaintError::VisibleChildOfHiddenParent { node: 1 })
    );

    let mut invalid = frame.clone();
    invalid.boxes[0].as_mut().unwrap().padding.x = u32::MAX;
    assert_eq!(
        paint(&invalid, &paints, Rgb::WHITE),
        Err(PaintError::Overflow)
    );
    let mut invalid = frame.clone();
    invalid.boxes[0].as_mut().unwrap().content.x = 3;
    assert_eq!(
        paint(&invalid, &paints, Rgb::WHITE),
        Err(PaintError::InvalidBox { node: 0 })
    );
    let mut invalid = frame.clone();
    invalid.boxes[0].as_mut().unwrap().margin.x = 1;
    assert_eq!(
        paint(&invalid, &paints, Rgb::WHITE),
        Err(PaintError::InvalidBox { node: 0 })
    );
}

#[test]
fn caps_allocation_and_rejects_bad_viewports_or_excessive_depth() {
    let frame = layout(&[Block::default()], 2001, 2001).unwrap();
    assert_eq!(
        paint(&frame, &[PaintNode::default()], Rgb::WHITE),
        Err(PaintError::TooManyPixels)
    );
    let small = layout(
        &[Block {
            intrinsic_height: 1,
            ..Block::default()
        }],
        1,
        1,
    )
    .unwrap();
    assert_eq!(
        paint(&small, &[painted(None, Some(RED), None)], Rgb::WHITE)
            .unwrap()
            .pixel(0, 0),
        Some(RED)
    );
    assert_eq!(
        layout(&[Block::default()], 0, 1),
        Err(LayoutError::InvalidViewport)
    );

    let mut malformed = small.clone();
    malformed.boxes = vec![small.boxes[0]; MAX_DEPTH + 2];
    let mut nodes = vec![PaintNode::default(); MAX_DEPTH + 2];
    for (i, node) in nodes.iter_mut().enumerate().skip(1) {
        node.parent = Some(i - 1);
    }
    assert_eq!(
        paint(&malformed, &nodes, Rgb::WHITE),
        Err(PaintError::TooDeep {
            node: MAX_DEPTH + 1
        })
    );
}
