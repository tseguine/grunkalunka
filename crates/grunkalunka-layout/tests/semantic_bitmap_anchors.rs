//! An original caller-rasterized ink integration with typed semantic box layout.
//! Independent exact samples ensure that masks follow anchors after reflow.

use grunkalunka_layout::paint::{Canvas, PaintNode, Rgb, paint};
use grunkalunka_layout::semantic::{Element, ElementKind, SemanticFrame, layout_semantic};

const CLEAR: Rgb = Rgb::WHITE;
const MAIN: Rgb = Rgb::new(245, 245, 240);
const HEADING: Rgb = Rgb::new(54, 107, 178);
const FIRST: Rgb = Rgb::new(87, 164, 118);
const SECOND: Rgb = Rgb::new(237, 183, 84);
const PARAGRAPH: Rgb = Rgb::new(160, 160, 160);

fn outline() -> [Element<'static>; 7] {
    [
        Element::new(None, ElementKind::Body, "", 0),
        Element::new(Some(0), ElementKind::Main, "", 0),
        Element::new(Some(1), ElementKind::Heading(1), "HI", 24),
        Element::new(Some(1), ElementKind::OrderedList, "", 0),
        Element::new(Some(3), ElementKind::ListItem, "First", 18),
        Element::new(Some(3), ElementKind::ListItem, "Second", 18),
        Element::new(Some(1), ElementKind::Paragraph, "End", 16),
    ]
}

// These original masks are supplied explicitly by the caller; no glyph lookup,
// text shaping or font metric inference is performed by the layout/paint crates.
fn draw_inert_masks(semantic: &SemanticFrame<'_>, canvas: &mut Canvas) {
    const H: [u8; 5] = [
        0b1010_0000, 0b1010_0000, 0b1110_0000, 0b1010_0000, 0b1010_0000,
    ];
    const I: [u8; 5] = [
        0b1110_0000, 0b0100_0000, 0b0100_0000, 0b0100_0000, 0b1110_0000,
    ];
    const X: [u8; 3] = [0b1010_0000, 0b0100_0000, 0b1010_0000];

    for anchor in &semantic.text_boxes {
        let x = anchor.content.x + 2;
        let y = anchor.content.y + 2;
        match anchor.node {
            2 => {
                canvas.blit_mono(x, y, 3, 5, &H, Rgb::BLACK).unwrap();
                // The caller explicitly supplies an advance of 5 pixels.
                canvas
                    .blit_mono(x + 5, y, 3, 5, &I, Rgb::BLACK)
                    .unwrap();
            }
            4..=6 => canvas.blit_mono(x, y, 3, 3, &X, Rgb::BLACK).unwrap(),
            other => panic!("unexpected text anchor {other}"),
        }
    }
}

fn paint_document<'a>(elements: &[Element<'a>]) -> (SemanticFrame<'a>, Canvas) {
    let semantic = layout_semantic(elements, 320, 120).unwrap();
    let colors = [CLEAR, MAIN, HEADING, CLEAR, FIRST, SECOND, PARAGRAPH];
    let layers: Vec<_> = elements
        .iter()
        .zip(colors)
        .map(|(element, color)| PaintNode {
            parent: element.parent,
            background: Some(color),
            border: None,
        })
        .collect();
    let mut canvas = paint(&semantic.frame, &layers, CLEAR).unwrap();
    draw_inert_masks(&semantic, &mut canvas);
    (semantic, canvas)
}

#[test]
fn supplied_bitmap_ink_follows_semantic_text_anchors_across_reflow() {
    let mut elements = outline();
    let (expanded, full) = paint_document(&elements);
    assert_eq!(
        expanded.text_boxes.iter().map(|a| a.node).collect::<Vec<_>>(),
        vec![2, 4, 5, 6]
    );
    assert_eq!(expanded.frame.boxes[6].unwrap().content.y, 86);

    // The independently specified visible pixels include on-ink, off-ink
    // transparent gaps, list rows, paragraph ink and untouched viewport.
    for ((x, y), color) in [
        ((10, 10), Rgb::BLACK), // H top-left stroke
        ((11, 10), HEADING),    // H's transparent center
        ((12, 10), Rgb::BLACK), // H right stroke
        ((15, 10), Rgb::BLACK), // I top-left stroke
        ((15, 11), HEADING),    // I's transparent left gap
        ((16, 11), Rgb::BLACK), // I central stroke
        ((26, 42), Rgb::BLACK), // first row X
        ((27, 42), FIRST),      // first row X transparent middle
        ((26, 62), Rgb::BLACK), // second row X
        ((27, 62), SECOND),     // second row X transparent middle
        ((10, 88), Rgb::BLACK), // paragraph X at original y
        ((11, 88), PARAGRAPH),  // paragraph transparent middle
        ((10, 50), CLEAR),     // list's unpainted text gap
        ((0, 118), CLEAR),     // outside document
    ] {
        assert_eq!(full.pixel(x, y), Some(color), "expanded {x},{y}");
    }

    elements[3].hidden = true;
    let (collapsed, smaller) = paint_document(&elements);
    assert_eq!(
        collapsed.text_boxes.iter().map(|a| a.node).collect::<Vec<_>>(),
        vec![2, 6]
    );
    assert_eq!(collapsed.frame.boxes[6].unwrap().content.y, 40);
    for ((x, y), color) in [
        ((10, 10), Rgb::BLACK), // unchanged H glyph
        ((10, 42), Rgb::BLACK), // paragraph X moved to y=40+2
        ((11, 42), PARAGRAPH),  // transparent portion of the X
        ((26, 42), PARAGRAPH),  // former first-list ink is gone
        ((26, 62), MAIN),      // former second-list ink is gone
        ((10, 88), CLEAR),     // former paragraph paint is gone
        ((0, 118), CLEAR),
    ] {
        assert_eq!(smaller.pixel(x, y), Some(color), "collapsed {x},{y}");
    }
    assert_ne!(full.ppm(), smaller.ppm());
}

#[test]
fn supplied_ink_is_not_implicitly_derived_from_source_text() {
    let mut elements = outline();
    let (_, original) = paint_document(&elements);
    // Text is inert data. Replacing it does not choose a new glyph or execute
    // markup; a real external text service must supply different masks.
    elements[2].text = "<script>not executed</script>";
    let (projection, same_ink) = paint_document(&elements);
    assert_eq!(projection.text_boxes[0].text, "<script>not executed</script>");
    assert_eq!(original.ppm(), same_ink.ppm());
}
