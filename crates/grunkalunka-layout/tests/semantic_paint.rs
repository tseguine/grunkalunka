//! Selected semantic-tree -> box-layout -> RGB-paint integration.
//! Expected coordinates and colors are written independently of the painter.

use grunkalunka_layout::paint::{PaintNode, Rgb, paint};
use grunkalunka_layout::semantic::{Element, ElementKind, layout_semantic};

const CLEAR: Rgb = Rgb::WHITE;
const MAIN: Rgb = Rgb::new(245, 245, 240);
const HEADER: Rgb = Rgb::new(54, 107, 178);
const FIRST: Rgb = Rgb::new(87, 164, 118);
const SECOND: Rgb = Rgb::new(237, 183, 84);
const PARAGRAPH: Rgb = Rgb::new(160, 160, 160);

#[test]
fn semantic_outline_paints_exact_boxes_and_reflows_when_hidden() {
    let mut elements = [
        Element::new(None, ElementKind::Body, "", 0),
        Element::new(Some(0), ElementKind::Main, "", 0),
        Element::new(Some(1), ElementKind::Heading(1), "A heading", 24),
        Element::new(Some(1), ElementKind::OrderedList, "", 0),
        Element::new(Some(3), ElementKind::ListItem, "First", 18),
        Element::new(Some(3), ElementKind::ListItem, "Second", 18),
        Element::new(Some(1), ElementKind::Paragraph, "End", 16),
    ];
    let colors = [CLEAR, MAIN, HEADER, CLEAR, FIRST, SECOND, PARAGRAPH];
    let paints: Vec<_> = elements
        .iter()
        .zip(colors)
        .map(|(element, color)| PaintNode {
            parent: element.parent,
            background: Some(color),
            border: None,
        })
        .collect();

    let expanded = layout_semantic(&elements, 320, 120).unwrap();
    let painted = paint(&expanded.frame, &paints, CLEAR).unwrap();
    for ((x, y), expected) in [
        ((0, 0), MAIN),
        ((8, 8), HEADER),
        ((9, 35), MAIN),
        ((10, 50), CLEAR),
        ((24, 40), FIRST),
        ((24, 60), SECOND),
        ((25, 59), CLEAR),
        ((8, 86), PARAGRAPH),
        ((8, 104), MAIN),
        ((8, 116), CLEAR),
    ] {
        assert_eq!(painted.pixel(x, y), Some(expected), "pixel {x},{y}");
    }
    assert_eq!(
        expanded.text_boxes.iter().map(|x| x.node).collect::<Vec<_>>(),
        vec![2, 4, 5, 6]
    );

    elements[3].hidden = true;
    let collapsed = layout_semantic(&elements, 320, 120).unwrap();
    let smaller = paint(&collapsed.frame, &paints, CLEAR).unwrap();
    assert_eq!(
        collapsed.text_boxes.iter().map(|x| x.node).collect::<Vec<_>>(),
        vec![2, 6]
    );
    for ((x, y), expected) in [
        ((24, 40), PARAGRAPH),
        ((24, 60), MAIN),
        ((24, 70), CLEAR),
        ((8, 86), CLEAR),
    ] {
        assert_eq!(smaller.pixel(x, y), Some(expected), "collapsed pixel {x},{y}");
    }
    assert_ne!(painted.ppm(), smaller.ppm());
}
