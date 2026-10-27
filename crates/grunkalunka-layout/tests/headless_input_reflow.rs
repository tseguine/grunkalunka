//! Headless click target -> caller policy -> semantic reflow -> RGB pixels.
//! These are integration tests, not DOM Event dispatch or trusted OS input.

use grunkalunka_layout::hit_test::{HitNode, hit_test};
use grunkalunka_layout::paint::{Canvas, PaintNode, Rgb, paint};
use grunkalunka_layout::semantic::{Element, ElementKind, SemanticFrame, layout_semantic};

const CLEAR: Rgb = Rgb::WHITE;
const MAIN: Rgb = Rgb::new(245, 245, 240);
const HEADER: Rgb = Rgb::new(54, 107, 178);
const FIRST: Rgb = Rgb::new(87, 164, 118);
const SECOND: Rgb = Rgb::new(237, 183, 84);
const PARAGRAPH: Rgb = Rgb::new(160, 160, 160);

fn outline() -> [Element<'static>; 7] {
    [
        Element::new(None, ElementKind::Body, "", 0),
        Element::new(Some(0), ElementKind::Main, "", 0),
        Element::new(Some(1), ElementKind::Heading(1), "Heading", 24),
        Element::new(Some(1), ElementKind::OrderedList, "", 0),
        Element::new(Some(3), ElementKind::ListItem, "First", 18),
        Element::new(Some(3), ElementKind::ListItem, "Second", 18),
        Element::new(Some(1), ElementKind::Paragraph, "Closing", 16),
    ]
}

fn colors(elements: &[Element<'_>], semantic: &SemanticFrame<'_>) -> Canvas {
    let palette = [CLEAR, MAIN, HEADER, CLEAR, FIRST, SECOND, PARAGRAPH];
    let layers: Vec<_> = elements
        .iter()
        .zip(palette)
        .map(|(node, color)| PaintNode {
            parent: node.parent,
            background: Some(color),
            border: None,
        })
        .collect();
    paint(&semantic.frame, &layers, CLEAR).unwrap()
}

fn targets(elements: &[Element<'_>]) -> Vec<HitNode> {
    elements
        .iter()
        .map(|node| HitNode::new(node.parent))
        .collect()
}

#[test]
fn caller_action_on_selected_list_item_reflows_text_and_pixels() {
    let mut elements = outline();
    let first_layout = layout_semantic(&elements, 320, 120).unwrap();
    let first_targets = targets(&elements);
    let original = colors(&elements, &first_layout);
    assert_eq!(
        hit_test(&first_layout.frame, &first_targets, 26, 42),
        Ok(Some(4))
    );
    assert_eq!(
        hit_test(&first_layout.frame, &first_targets, 10, 35),
        Ok(Some(1))
    );
    assert_eq!(
        hit_test(&first_layout.frame, &first_targets, 26, 120),
        Ok(None)
    );
    assert_eq!(
        first_layout
            .text_boxes
            .iter()
            .map(|text| text.node)
            .collect::<Vec<_>>(),
        vec![2, 4, 5, 6]
    );
    for ((x, y), expected) in [
        ((26, 42), FIRST),
        ((26, 62), SECOND),
        ((10, 88), PARAGRAPH),
        ((8, 8), HEADER),
        ((0, 118), CLEAR),
    ] {
        assert_eq!(original.pixel(x, y), Some(expected), "expanded {x},{y}");
    }

    // Entirely caller-owned input policy: an already-identified first list
    // item triggers hiding its list. No JS execution or DOM dispatch exists.
    if hit_test(&first_layout.frame, &first_targets, 26, 42).unwrap() == Some(4) {
        elements[3].hidden = true;
    }
    let collapsed = layout_semantic(&elements, 320, 120).unwrap();
    let collapsed_targets = targets(&elements);
    let redrawn = colors(&elements, &collapsed);
    assert_eq!(
        hit_test(&collapsed.frame, &collapsed_targets, 26, 42),
        Ok(Some(6))
    );
    assert_eq!(collapsed.frame.boxes[6].unwrap().content.y, 40);
    assert_eq!(
        collapsed
            .text_boxes
            .iter()
            .map(|text| text.node)
            .collect::<Vec<_>>(),
        vec![2, 6]
    );
    for ((x, y), expected) in [
        ((26, 42), PARAGRAPH),
        ((26, 62), MAIN),
        ((10, 88), CLEAR),
        ((8, 8), HEADER),
        ((0, 118), CLEAR),
    ] {
        assert_eq!(redrawn.pixel(x, y), Some(expected), "collapsed {x},{y}");
    }
    assert_ne!(original.ppm(), redrawn.ppm());
}

#[test]
fn input_target_policy_does_not_change_the_painted_pixels() {
    let elements = outline();
    let semantic = layout_semantic(&elements, 320, 120).unwrap();
    let unchanged = colors(&elements, &semantic).ppm();
    let mut input = targets(&elements);
    input[4].targetable = false;
    assert_eq!(hit_test(&semantic.frame, &input, 26, 42), Ok(Some(3)));
    input[3].targetable = false;
    assert_eq!(hit_test(&semantic.frame, &input, 26, 42), Ok(Some(1)));
    assert_eq!(colors(&elements, &semantic).ppm(), unchanged);
}
