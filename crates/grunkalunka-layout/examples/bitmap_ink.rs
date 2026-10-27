//! An original X bitmap drawn over a solid background.
//! Usage: cargo run -q -p grunkalunka-layout --example bitmap_ink --locked --offline > ink.ppm
//! The caller provides the exact 8x8 mask; there is no font or glyph engine.

use grunkalunka_layout::paint::{PaintNode, Rgb, paint};
use grunkalunka_layout::{Block, layout};
use std::io::{self, Write};

fn main() -> io::Result<()> {
    let frame = layout(
        &[Block {
            intrinsic_height: 16,
            ..Block::default()
        }],
        16,
        16,
    )
    .expect("bounded viewport");
    let mut canvas = paint(
        &frame,
        &[PaintNode {
            parent: None,
            background: Some(Rgb::new(238, 238, 238)),
            border: None,
        }],
        Rgb::WHITE,
    )
    .expect("bounded canvas");
    let x_mask = [
        0b1000_0001,
        0b0100_0010,
        0b0010_0100,
        0b0001_1000,
        0b0001_1000,
        0b0010_0100,
        0b0100_0010,
        0b1000_0001,
    ];
    canvas
        .blit_mono(4, 4, 8, 8, &x_mask, Rgb::new(12, 48, 156))
        .expect("original bitmap mask");
    io::stdout().write_all(&canvas.ppm())
}
