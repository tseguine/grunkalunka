//! Headless input targeting without a browser window or an event dispatcher.

use grunkalunka_layout::hit_test::{HitNode, hit_test};
use grunkalunka_layout::{Block, Length, Style, layout};

fn main() {
    let blocks = [
        Block {
            style: Style {
                height: Length::Px(20),
                ..Style::default()
            },
            ..Block::default()
        },
        Block {
            parent: Some(0),
            style: Style {
                height: Length::Px(8),
                ..Style::default()
            },
            ..Block::default()
        },
        Block::child(0, 8),
        Block::child(1, 20),
    ];
    let frame = layout(&blocks, 10, 20).expect("bounded headless layout");
    let nodes = [
        HitNode::new(None),
        HitNode::new(Some(0)),
        HitNode::new(Some(0)),
        HitNode::new(Some(1)),
    ];
    for (x, y) in [(1, 1), (1, 10), (1, 17), (11, 1)] {
        let target = hit_test(&frame, &nodes, x, y).expect("valid hit tree");
        println!("{x},{y}: {target:?}");
    }
}
