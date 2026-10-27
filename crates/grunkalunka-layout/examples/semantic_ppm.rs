//! A typed semantic tree, headless layout and RGB raster in one reusable pipeline.
//! Text remains inert; filled rectangles indicate boxes, not rendered glyphs.
//! Usage: cargo run -q -p grunkalunka-layout --example semantic_ppm > outline.ppm

use grunkalunka_layout::paint::{PaintNode, Rgb, paint};
use grunkalunka_layout::semantic::{Element, ElementKind, layout_semantic};
use std::io::{self, Write};

fn main() -> io::Result<()> {
    let elements = [
        Element::new(None, ElementKind::Body, "", 0),
        Element::new(Some(0), ElementKind::Main, "", 0),
        Element::new(Some(1), ElementKind::Heading(1), "A heading", 24),
        Element::new(Some(1), ElementKind::OrderedList, "", 0),
        Element::new(Some(3), ElementKind::ListItem, "First", 18),
        Element::new(Some(3), ElementKind::ListItem, "Second", 18),
        Element::new(Some(1), ElementKind::Paragraph, "End", 16),
    ];
    let semantic = layout_semantic(&elements, 320, 120).expect("bounded semantic tree");
    let colors = [
        Rgb::WHITE,
        Rgb::new(245, 245, 240),
        Rgb::new(54, 107, 178),
        Rgb::WHITE,
        Rgb::new(87, 164, 118),
        Rgb::new(237, 183, 84),
        Rgb::new(160, 160, 160),
    ];
    let paints: Vec<_> = elements
        .iter()
        .zip(colors)
        .map(|(element, color)| PaintNode {
            parent: element.parent,
            background: Some(color),
            border: None,
        })
        .collect();
    let canvas = paint(&semantic.frame, &paints, Rgb::WHITE).expect("bounded RGB output");
    io::stdout().write_all(&canvas.ppm())
}
