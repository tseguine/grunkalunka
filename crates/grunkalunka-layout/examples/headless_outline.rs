//! Headless use without a browser shell, HTML parser or WIT host.

use grunkalunka_layout::{Block, Edges, Style, layout};

fn main() {
    let blocks = [
        Block {
            style: Style {
                padding: Edges::all(8),
                ..Style::default()
            },
            ..Block::default()
        },
        Block::child(0, 24), // A heading, with height from a separate text service.
        Block::child(0, 18), // First outline row.
        Block::child(0, 18), // Second outline row.
    ];
    let frame = layout(&blocks, 320, 200).expect("bounded example layout");
    for (index, geometry) in frame.boxes.iter().enumerate() {
        println!(
            "{index}: {:?}",
            geometry.expect("all example blocks are visible").border
        );
    }
}
