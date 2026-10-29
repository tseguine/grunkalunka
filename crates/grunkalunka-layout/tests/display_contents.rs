//! Blockless display:contents flattening in the headless normal-flow engine.
//! This tests geometry and paint/hit projection, not a DOM or CSS parser.
use grunkalunka_layout::hit_test::{HitError, HitNode, hit_test};
use grunkalunka_layout::paint::{PaintError, PaintNode, Rgb, paint};
use grunkalunka_layout::semantic::{
    Element, ElementKind, SemanticError, layout_semantic,
};
use grunkalunka_layout::{
    Block, DescendantClip, Display, Edges, Length, Rect, Style, Visibility, layout,
};

const ROOT: Rgb = Rgb::new(200, 200, 200);
const WRAPPER: Rgb = Rgb::new(255, 0, 0);
const FIRST: Rgb = Rgb::new(0, 40, 230);
const SECOND: Rgb = Rgb::new(0, 160, 40);
const SIBLING: Rgb = Rgb::new(220, 145, 0);

fn boxes() -> [Block; 5] {
    [
        Block {
            style: Style {
                width: Length::Px(30),
                height: Length::Px(25),
                padding: Edges::all(1),
                ..Style::default()
            },
            ..Block::default()
        },
        Block {
            parent: Some(0),
            style: Style {
                display: Display::Contents,
                width: Length::Px(120),
                height: Length::Px(80),
                margin: Edges::all(9),
                border: Edges::all(3),
                padding: Edges::all(5),
                descendant_clip: DescendantClip::PaddingBox,
                ..Style::default()
            },
            intrinsic_height: 40,
        },
        Block {
            parent: Some(1),
            style: Style {
                width: Length::Px(12),
                height: Length::Px(5),
                margin: Edges {
                    top: 2,
                    left: 3,
                    ..Edges::default()
                },
                ..Style::default()
            },
            ..Block::default()
        },
        Block {
            parent: Some(1),
            style: Style {
                width: Length::Px(6),
                height: Length::Px(9),
                ..Style::default()
            },
            ..Block::default()
        },
        Block {
            parent: Some(0),
            style: Style {
                width: Length::Px(8),
                height: Length::Px(4),
                ..Style::default()
            },
            ..Block::default()
        },
    ]
}

fn layers(blocks: &[Block]) -> (Vec<HitNode>, Vec<PaintNode>) {
    let colors = [ROOT, WRAPPER, FIRST, SECOND, SIBLING];
    let hits = blocks
        .iter()
        .map(|block| HitNode::new(block.parent))
        .collect();
    let paints = blocks
        .iter()
        .zip(colors)
        .map(|(block, color)| PaintNode {
            parent: block.parent,
            background: Some(color),
            border: None,
        })
        .collect();
    (hits, paints)
}

#[test]
fn contents_wrapper_has_no_box_and_children_flatten_into_parent_flow() {
    let original = boxes();
    let frame = layout(&original, 40, 30).unwrap();
    assert_eq!(frame.contents, [false, true, false, false, false]);
    assert!(frame.boxes[1].is_none());
    assert_eq!(
        frame.boxes[2].unwrap().border,
        Rect { x: 4, y: 3, width: 12, height: 5 }
    );
    assert_eq!(
        frame.boxes[3].unwrap().border,
        Rect { x: 1, y: 8, width: 6, height: 9 }
    );
    assert_eq!(
        frame.boxes[4].unwrap().border,
        Rect { x: 1, y: 17, width: 8, height: 4 }
    );

    let direct = [original[0],
        Block {
            parent: Some(0),
            ..original[2]
        },
        Block {
            parent: Some(0),
            ..original[3]
        },
        original[4],
    ];
    let flattened = layout(&direct, 40, 30).unwrap();
    assert_eq!(frame.boxes[0], flattened.boxes[0]);
    for (old_index, flat_index) in [(2, 1), (3, 2), (4, 3)] {
        assert_eq!(frame.boxes[old_index], flattened.boxes[flat_index]);
    }
    assert_eq!((frame.scroll_width, frame.scroll_height), (40, 30));
    assert_eq!(
        (frame.scroll_width, frame.scroll_height),
        (flattened.scroll_width, flattened.scroll_height)
    );
}

#[test]
fn paint_and_input_walk_through_a_boxless_wrapper_without_painting_it() {
    let blocks = boxes();
    let frame = layout(&blocks, 40, 30).unwrap();
    let (hits, paints) = layers(&blocks);
    let canvas = paint(&frame, &paints, Rgb::WHITE).unwrap();
    for ((x, y), color, target) in [
        ((4, 4), FIRST, Some(2)),
        ((1, 10), SECOND, Some(3)),
        ((1, 18), SIBLING, Some(4)),
        ((20, 20), ROOT, Some(0)),
    ] {
        assert_eq!(canvas.pixel(x, y), Some(color), "contents pixel ({x},{y})");
        assert_eq!(hit_test(&frame, &hits, x, y), Ok(target));
    }
    assert_eq!(hit_test(&frame, &hits, 1, 1), Ok(Some(0)));
    assert_eq!(canvas.pixel(1, 1), Some(ROOT));
    assert!(!canvas.ppm().is_empty());
}

#[test]
fn nested_contents_keep_visibility_inheritance_and_do_not_restore_display_none() {
    let mut blocks = [
        Block {
            style: Style {
                width: Length::Px(30),
                height: Length::Px(30),
                ..Style::default()
            },
            ..Block::default()
        },
        Block {
            parent: Some(0),
            style: Style {
                display: Display::Contents,
                visibility: Visibility::Hidden,
                ..Style::default()
            },
            ..Block::default()
        },
        Block {
            parent: Some(1),
            style: Style {
                display: Display::Contents,
                ..Style::default()
            },
            ..Block::default()
        },
        Block {
            parent: Some(2),
            style: Style {
                visibility: Visibility::Visible,
                width: Length::Px(10),
                height: Length::Px(7),
                ..Style::default()
            },
            ..Block::default()
        },
        Block {
            parent: Some(2),
            style: Style {
                width: Length::Px(8),
                height: Length::Px(6),
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
    let frame = layout(&blocks, 30, 30).unwrap();
    assert_eq!(frame.contents, [false, true, true, false, false, false]);
    assert_eq!(frame.visibility, [true, false, false, true, false, true]);
    assert!(frame.boxes[1].is_none() && frame.boxes[2].is_none());
    assert_eq!(frame.boxes[3].unwrap().border.y, 0);
    assert_eq!(frame.boxes[4].unwrap().border.y, 7);
    assert_eq!(frame.boxes[5].unwrap().border.y, 13);

    let hits: Vec<_> = blocks.iter().map(|node| HitNode::new(node.parent)).collect();
    let paints: Vec<_> = blocks.iter().enumerate().map(|(i, node)| PaintNode {
        parent: node.parent,
        background: Some([ROOT, WRAPPER, WRAPPER, FIRST, SECOND, SIBLING][i]),
        border: None,
    }).collect();
    let canvas = paint(&frame, &paints, Rgb::WHITE).unwrap();
    assert_eq!(canvas.pixel(1, 1), Some(FIRST));
    assert_eq!(canvas.pixel(1, 9), Some(ROOT));
    assert_eq!(canvas.pixel(1, 14), Some(SIBLING));
    assert_eq!(hit_test(&frame, &hits, 1, 1), Ok(Some(3)));
    assert_eq!(hit_test(&frame, &hits, 1, 9), Ok(Some(0)));
    assert_eq!(hit_test(&frame, &hits, 1, 14), Ok(Some(5)));

    blocks[1].style.display = Display::None;
    let hidden = layout(&blocks, 30, 30).unwrap();
    assert_eq!(hidden.contents, [false; 6]);
    assert_eq!(hidden.boxes[1], None);
    assert_eq!(hidden.boxes[2], None);
    assert_eq!(hidden.boxes[3], None);
    assert_eq!(hidden.boxes[4], None);
    assert_eq!(hidden.boxes[5].unwrap().border.y, 0);
    assert_eq!(hit_test(&hidden, &hits, 1, 9), Ok(Some(0)));
}

#[test]
fn contents_root_uses_viewport_for_children_and_never_gets_a_paint_box() {
    let blocks = [
        Block {
            style: Style {
                display: Display::Contents,
                width: Length::Px(999),
                height: Length::Px(999),
                padding: Edges::all(7),
                ..Style::default()
            },
            intrinsic_height: 500,
        },
        Block {
            parent: Some(0),
            style: Style {
                width: Length::Percent(50),
                height: Length::Percent(50),
                ..Style::default()
            },
            ..Block::default()
        },
        Block {
            parent: Some(0),
            style: Style {
                width: Length::Px(10),
                height: Length::Px(5),
                ..Style::default()
            },
            ..Block::default()
        },
    ];
    let frame = layout(&blocks, 80, 40).unwrap();
    assert_eq!(frame.contents, [true, false, false]);
    assert_eq!(frame.boxes[0], None);
    assert_eq!(frame.boxes[1].unwrap().content, Rect {
        x: 0, y: 0, width: 40, height: 20
    });
    assert_eq!(frame.boxes[2].unwrap().border.y, 20);
    assert_eq!((frame.scroll_width, frame.scroll_height), (80, 40));

    let hits = [HitNode::new(None), HitNode::new(Some(0)), HitNode::new(Some(0))];
    let paints = [
        PaintNode { parent: None, background: Some(WRAPPER), border: None },
        PaintNode { parent: Some(0), background: Some(FIRST), border: None },
        PaintNode { parent: Some(0), background: Some(SIBLING), border: None },
    ];
    let canvas = paint(&frame, &paints, Rgb::WHITE).unwrap();
    assert_eq!(canvas.pixel(2, 2), Some(FIRST));
    assert_eq!(canvas.pixel(2, 21), Some(SIBLING));
    assert_eq!(canvas.pixel(60, 2), Some(Rgb::WHITE));
    assert_eq!(hit_test(&frame, &hits, 60, 2), Ok(None));
}

#[test]
fn malformed_contents_metadata_fails_before_paint_and_hit_selection() {
    let blocks = boxes();
    let frame = layout(&blocks, 40, 30).unwrap();
    let (hits, paints) = layers(&blocks);
    let mut bad = frame.clone();
    bad.contents.pop();
    assert_eq!(paint(&bad, &paints, Rgb::WHITE), Err(PaintError::NodeCountMismatch));
    assert_eq!(hit_test(&bad, &hits, 4, 4), Err(HitError::NodeCountMismatch));

    let mut bad = frame.clone();
    bad.contents[0] = true;
    assert_eq!(paint(&bad, &paints, Rgb::WHITE), Err(PaintError::InvalidBox { node: 0 }));
    assert_eq!(hit_test(&bad, &hits, 4, 4), Err(HitError::InvalidBox { node: 0 }));

    let mut bad = frame.clone();
    bad.contents[1] = false;
    assert_eq!(
        paint(&bad, &paints, Rgb::WHITE),
        Err(PaintError::VisibleChildOfHiddenParent { node: 2 })
    );
    assert_eq!(
        hit_test(&bad, &hits, 4, 4),
        Err(HitError::VisibleChildOfHiddenParent { node: 2 })
    );
}

#[test]
fn semantic_tree_uses_boxless_div_containers_but_rejects_direct_text() {
    let mut elements = [
        Element::new(None, ElementKind::Body, "", 0),
        Element::new(Some(0), ElementKind::Div, "", 0),
        Element::new(Some(1), ElementKind::Paragraph, "Alpha", 5),
        Element::new(Some(1), ElementKind::Paragraph, "Beta", 7),
        Element::new(Some(0), ElementKind::Paragraph, "After", 3),
    ];
    elements[1].resolved_style = Some(Style {
        display: Display::Contents,
        margin: Edges::all(10),
        padding: Edges::all(5),
        ..Style::default()
    });
    let frame = layout_semantic(&elements, 100, 60).unwrap();
    assert_eq!(frame.frame.boxes[1], None);
    assert!(frame.frame.contents[1]);
    assert_eq!(frame.frame.boxes[2].unwrap().content.y, 0);
    assert_eq!(frame.frame.boxes[3].unwrap().content.y, 11);
    assert_eq!(frame.frame.boxes[4].unwrap().content.y, 24);
    assert_eq!(
        frame.text_boxes.iter().map(|box_| box_.node).collect::<Vec<_>>(),
        vec![2, 3, 4]
    );

    elements[1].text = "unsupported";
    elements[1].measured_text_height = 4;
    assert_eq!(
        layout_semantic(&elements, 100, 60),
        Err(SemanticError::BoxlessText { node: 1 })
    );

    // Remove the deliberately invalid direct text before hiding the wrapper.
    elements[1].text = "";
    elements[1].measured_text_height = 0;
    elements[1].hidden = true;
    let hidden = layout_semantic(&elements, 100, 60).unwrap();
    assert_eq!(hidden.frame.boxes[2], None);
    assert_eq!(hidden.frame.boxes[3], None);
    assert_eq!(hidden.frame.boxes[4].unwrap().content.y, 0);
}
