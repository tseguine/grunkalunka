//! Caller-rasterized symbols at semantic text anchors; no font is selected.
//! Usage: cargo run -q -p grunkalunka-layout --example semantic_ink_ppm --locked --offline > anchors.ppm

use grunkalunka_layout::paint::{PaintNode, Rgb, paint};
use grunkalunka_layout::semantic::{Element, ElementKind, layout_semantic};
use std::io::{self, Write};

fn main() -> io::Result<()> {
    let elements = [
        Element::new(None, ElementKind::Body, "", 0),
        Element::new(Some(0), ElementKind::Main, "", 0),
        Element::new(Some(1), ElementKind::Heading(1), "HI", 24),
        Element::new(Some(1), ElementKind::OrderedList, "", 0),
        Element::new(Some(3), ElementKind::ListItem, "First", 18),
        Element::new(Some(3), ElementKind::ListItem, "Second", 18),
        Element::new(Some(1), ElementKind::Paragraph, "End", 16),
    ];
    let colors = [
        Rgb::WHITE,
        Rgb::new(245, 245, 240),
        Rgb::new(54, 107, 178),
        Rgb::WHITE,
        Rgb::new(87, 164, 118),
        Rgb::new(237, 183, 84),
        Rgb::new(160, 160, 160),
    ];
    let semantic = layout_semantic(&elements, 320, 120).expect("bounded semantic tree");
    let layers: Vec<_> = elements
        .iter()
        .zip(colors)
        .map(|(element, color)| PaintNode {
            parent: element.parent,
            background: Some(color),
            border: None,
        })
        .collect();
    let mut canvas = paint(&semantic.frame, &layers, Rgb::WHITE).expect("bounded RGB output");

    // Original hand-supplied pixel patterns, not Unicode glyph lookup, font
    // shaping, text measurement or implicit association with the UTF-8 strings.
    let h = [
        0b1010_0000,
        0b1010_0000,
        0b1110_0000,
        0b1010_0000,
        0b1010_0000,
    ];
    let i = [
        0b1110_0000,
        0b0100_0000,
        0b0100_0000,
        0b0100_0000,
        0b1110_0000,
    ];
    let marker = [0b1010_0000, 0b0100_0000, 0b1010_0000];
    for anchor in &semantic.text_boxes {
        let x = anchor.content.x + 2;
        let y = anchor.content.y + 2;
        if anchor.node == 2 {
            canvas
                .blit_mono(x, y, 3, 5, &h, Rgb::BLACK)
                .expect("original H mask");
            canvas
                .blit_mono(x + 5, y, 3, 5, &i, Rgb::BLACK)
                .expect("original I mask");
        } else {
            canvas
                .blit_mono(x, y, 3, 3, &marker, Rgb::BLACK)
                .expect("original marker mask");
        }
    }
    io::stdout().write_all(&canvas.ppm())
}
