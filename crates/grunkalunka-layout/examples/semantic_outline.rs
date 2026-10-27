//! A headless semantic tree consumed without an HTML parser, CSS cascade, or UI.

use grunkalunka_layout::semantic::{Element, ElementKind, layout_semantic};

fn main() {
    let mut nodes = [
        Element::new(None, ElementKind::Body, "", 0),
        Element::new(Some(0), ElementKind::Main, "", 0),
        Element::new(Some(1), ElementKind::Heading(1), "Example note", 24),
        Element::new(Some(1), ElementKind::OrderedList, "", 0),
        Element::new(Some(3), ElementKind::ListItem, "First heading", 18),
        Element::new(Some(3), ElementKind::ListItem, "Second heading", 18),
        Element::new(Some(1), ElementKind::Paragraph, "End of outline", 16),
    ];

    let expanded = layout_semantic(&nodes, 320, 100).expect("bounded example tree");
    println!("expanded: {:?}", expanded.text_boxes);

    nodes[3].hidden = true;
    let collapsed = layout_semantic(&nodes, 320, 100).expect("hidden subtree");
    println!("collapsed: {:?}", collapsed.text_boxes);
}
