//! Inherited visibility without reflow; contrasts with display:none.
//! Paint and hit decisions consume the same resolved per-node visibility.
use grunkalunka_layout::hit_test::{HitError, HitNode, hit_test};
use grunkalunka_layout::paint::{PaintError, PaintNode, Rgb, paint};
use grunkalunka_layout::semantic::{Element, ElementKind, layout_semantic};
use grunkalunka_layout::{Block, Display, Edges, Length, Style, Visibility, layout};

const CLEAR: Rgb = Rgb::WHITE;
const ROOT: Rgb = Rgb::new(220, 220, 220);
const HIDDEN: Rgb = Rgb::new(240, 20, 20);
const CHILD: Rgb = Rgb::new(30, 160, 30);
const RESTORED: Rgb = Rgb::new(20, 20, 200);
const SIBLING: Rgb = Rgb::new(225, 160, 20);
const GRANDCHILD: Rgb = Rgb::new(150, 30, 150);

fn blocks() -> [Block; 7] {
    [
        Block::default(),
        Block {
            parent: Some(0),
            style: Style {
                visibility: Visibility::Hidden,
                width: Length::Px(20),
                height: Length::Px(18),
                margin: Edges {
                    top: 2,
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
                width: Length::Px(8),
                height: Length::Px(6),
                ..Style::default()
            },
            ..Block::default()
        },
        Block {
            parent: Some(1),
            style: Style {
                visibility: Visibility::Visible,
                width: Length::Px(8),
                height: Length::Px(5),
                margin: Edges {
                    top: 2,
                    ..Edges::default()
                },
                ..Style::default()
            },
            ..Block::default()
        },
        Block {
            parent: Some(0),
            style: Style {
                width: Length::Px(12),
                height: Length::Px(7),
                ..Style::default()
            },
            ..Block::default()
        },
        Block {
            parent: Some(0),
            style: Style {
                visibility: Visibility::Hidden,
                width: Length::Px(4),
                height: Length::Px(3),
                ..Style::default()
            },
            ..Block::default()
        },
        Block {
            parent: Some(5),
            style: Style {
                visibility: Visibility::Visible,
                width: Length::Px(2),
                height: Length::Px(2),
                ..Style::default()
            },
            ..Block::default()
        },
    ]
}

fn inputs(blocks: &[Block]) -> (Vec<HitNode>, Vec<PaintNode>) {
    let paints = [ROOT, HIDDEN, CHILD, RESTORED, SIBLING, HIDDEN, GRANDCHILD];
    let hits = blocks.iter().map(|block| HitNode::new(block.parent)).collect();
    let paints = blocks
        .iter()
        .zip(paints)
        .map(|(block, color)| PaintNode {
            parent: block.parent,
            background: Some(color),
            border: None,
        })
        .collect();
    (hits, paints)
}

#[test]
fn visibility_inheritance_and_explicit_override_do_not_change_any_boxes() {
    let selected = blocks();
    let frame = layout(&selected, 60, 45).unwrap();
    assert_eq!(
        frame.visibility,
        [true, false, false, true, true, false, true]
    );
    assert_eq!(frame.boxes[1].unwrap().border.y, 2);
    assert_eq!(frame.boxes[2].unwrap().content.y, 5);
    assert_eq!(frame.boxes[3].unwrap().content.y, 13);
    assert_eq!(frame.boxes[4].unwrap().border.y, 26);
    assert_eq!(frame.boxes[6].unwrap().border.y, 33);
    assert_eq!((frame.scroll_width, frame.scroll_height), (60, 45));

    let mut all_visible = selected;
    all_visible[1].style.visibility = Visibility::Inherit;
    all_visible[3].style.visibility = Visibility::Inherit;
    all_visible[5].style.visibility = Visibility::Inherit;
    all_visible[6].style.visibility = Visibility::Inherit;
    let baseline = layout(&all_visible, 60, 45).unwrap();
    assert_eq!(baseline.visibility, vec![true; selected.len()]);
    assert_eq!(frame.boxes, baseline.boxes);
    assert_eq!(frame.scroll_width, baseline.scroll_width);
    assert_eq!(frame.scroll_height, baseline.scroll_height);
}

#[test]
fn paint_and_hit_respect_inherited_visibility_even_with_targetable_nodes() {
    let blocks = blocks();
    let frame = layout(&blocks, 60, 45).unwrap();
    let (hits, paints) = inputs(&blocks);

    assert_eq!(hit_test(&frame, &hits, 4, 6), Ok(Some(0)));
    assert_eq!(hit_test(&frame, &hits, 4, 14), Ok(Some(3)));
    assert_eq!(hit_test(&frame, &hits, 1, 27), Ok(Some(4)));
    assert_eq!(hit_test(&frame, &hits, 1, 34), Ok(Some(6)));
    assert_eq!(hit_test(&frame, &hits, 4, 20), Ok(Some(0)));
    assert_eq!(hit_test(&frame, &hits, 50, 40), Ok(None));

    let canvas = paint(&frame, &paints, CLEAR).unwrap();
    for ((x, y), expected) in [
        ((1, 3), ROOT), // Hidden ancestor border/background suppressed.
        ((4, 6), ROOT), // Inherited-hidden child retains geometry, not paint.
        ((4, 14), RESTORED), // Explicit visible descendant overrides parent.
        ((1, 27), SIBLING), // Following sibling has not reflowed.
        ((1, 34), GRANDCHILD), // Another visible override under hidden parent.
        ((4, 20), ROOT),
        ((50, 40), CLEAR),
    ] {
        assert_eq!(canvas.pixel(x, y), Some(expected), "pixel {x},{y}");
    }
}

#[test]
fn display_none_removes_boxes_even_when_descendant_overrides_visibility() {
    let mut blocks = blocks();
    blocks[1].style.display = Display::None;
    let frame = layout(&blocks, 60, 45).unwrap();
    assert_eq!(frame.boxes[1], None);
    assert_eq!(frame.boxes[2], None);
    assert_eq!(frame.boxes[3], None);
    assert!(frame.visibility[3], "explicit visible is not display:block");
    assert_eq!(frame.boxes[4].unwrap().border.y, 0);

    let (hits, paints) = inputs(&blocks);
    assert_eq!(hit_test(&frame, &hits, 4, 8), Ok(Some(0)));
    assert_eq!(hit_test(&frame, &hits, 4, 14), Ok(None));
    let canvas = paint(&frame, &paints, CLEAR).unwrap();
    assert_eq!(canvas.pixel(4, 8), Some(ROOT));
    assert_eq!(canvas.pixel(4, 14), Some(CLEAR));
}

#[test]
fn missing_visibility_metadata_fails_closed_for_paint_and_hit_test() {
    let blocks = blocks();
    let mut frame = layout(&blocks, 60, 45).unwrap();
    let (hits, paints) = inputs(&blocks);
    frame.visibility.pop();
    assert_eq!(hit_test(&frame, &hits, 4, 6), Err(HitError::NodeCountMismatch));
    assert_eq!(
        paint(&frame, &paints, CLEAR),
        Err(PaintError::NodeCountMismatch)
    );
}

#[test]
fn semantic_text_anchors_hide_inherited_text_but_allow_visible_override() {
    let mut elements = [
        Element::new(None, ElementKind::Body, "", 0),
        Element::new(Some(0), ElementKind::Main, "", 0),
        Element::new(Some(1), ElementKind::Paragraph, "Hidden", 5),
        Element::new(Some(1), ElementKind::Paragraph, "Shown", 5),
        Element::new(Some(0), ElementKind::Paragraph, "Sibling", 5),
    ];
    elements[1].resolved_style = Some(Style {
        visibility: Visibility::Hidden,
        ..Style::default()
    });
    elements[3].resolved_style = Some(Style {
        visibility: Visibility::Visible,
        ..Style::default()
    });
    let hidden = layout_semantic(&elements, 100, 60).unwrap();
    assert_eq!(hidden.frame.visibility, [true, false, false, true, true]);
    assert_eq!(
        hidden.text_boxes.iter().map(|box_| box_.node).collect::<Vec<_>>(),
        vec![3, 4]
    );
    assert!(hidden.frame.boxes[2].is_some());

    elements[1].resolved_style = Some(Style::default());
    let visible = layout_semantic(&elements, 100, 60).unwrap();
    assert_eq!(hidden.frame.boxes, visible.frame.boxes);
    assert_eq!(
        visible.text_boxes.iter().map(|box_| box_.node).collect::<Vec<_>>(),
        vec![2, 3, 4]
    );
}
