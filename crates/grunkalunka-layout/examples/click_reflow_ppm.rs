//! Original headless click-to-visibility policy; outputs only the redrawn PPM.
//! Usage: cargo run -q -p grunkalunka-layout --example click_reflow_ppm --locked --offline > toggled.ppm

use grunkalunka_layout::hit_test::{HitNode, hit_test};
use grunkalunka_layout::paint::{PaintNode, Rgb, paint};
use grunkalunka_layout::semantic::{Element, ElementKind, layout_semantic};
use std::io::{self, Write};

fn main() -> io::Result<()> {
    let mut elements = [
        Element::new(None, ElementKind::Body, "", 0),
        Element::new(Some(0), ElementKind::Main, "", 0),
        Element::new(Some(1), ElementKind::Heading(1), "Heading", 24),
        Element::new(Some(1), ElementKind::OrderedList, "", 0),
        Element::new(Some(3), ElementKind::ListItem, "First", 18),
        Element::new(Some(3), ElementKind::ListItem, "Second", 18),
        Element::new(Some(1), ElementKind::Paragraph, "Closing", 16),
    ];
    let frame = layout_semantic(&elements, 320, 120).expect("bounded semantic layout");
    let targets: Vec<_> = elements
        .iter()
        .map(|element| HitNode::new(element.parent))
        .collect();
    let selected = hit_test(&frame.frame, &targets, 26, 42).expect("bounded hit tree");
    // A fixed host action for the demo, not a JS callback, DOM event listener
    // or general browser input dispatcher.
    if selected == Some(4) {
        elements[3].hidden = true;
    }
    let updated = layout_semantic(&elements, 320, 120).expect("recomputed layout");
    let palette = [
        Rgb::WHITE,
        Rgb::new(245, 245, 240),
        Rgb::new(54, 107, 178),
        Rgb::WHITE,
        Rgb::new(87, 164, 118),
        Rgb::new(237, 183, 84),
        Rgb::new(160, 160, 160),
    ];
    let layers: Vec<_> = elements
        .iter()
        .zip(palette)
        .map(|(element, color)| PaintNode {
            parent: element.parent,
            background: Some(color),
            border: None,
        })
        .collect();
    let image = paint(&updated.frame, &layers, Rgb::WHITE).expect("bounded RGB repaint");
    eprintln!(
        "selected={selected:?}; visible anchors={}",
        updated.text_boxes.len()
    );
    io::stdout().write_all(&image.ppm())
}
