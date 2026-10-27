//! Print an original headless RGB PPM image to standard output.
//! Usage: cargo run -q -p grunkalunka-layout --example headless_ppm > boxes.ppm

use grunkalunka_layout::paint::{PaintNode, Rgb, paint};
use grunkalunka_layout::{Block, Edges, Style, layout};
use std::io::{self, Write};

fn main() -> io::Result<()> {
    let blocks = [
        Block {
            style: Style {
                border: Edges::all(1),
                padding: Edges::all(4),
                ..Style::default()
            },
            ..Block::default()
        },
        Block::child(0, 10),
        Block::child(0, 10),
    ];
    let frame = layout(&blocks, 64, 48).expect("bounded example layout");
    let color = |red, green, blue| Rgb::new(red, green, blue);
    let nodes = [
        PaintNode {
            parent: None,
            background: Some(Rgb::WHITE),
            border: Some(Rgb::BLACK),
        },
        PaintNode {
            parent: Some(0),
            background: Some(color(52, 108, 170)),
            border: None,
        },
        PaintNode {
            parent: Some(0),
            background: Some(color(235, 151, 68)),
            border: None,
        },
    ];
    let canvas = paint(&frame, &nodes, color(224, 224, 224)).expect("valid paint tree");
    io::stdout().write_all(&canvas.ppm())
}
