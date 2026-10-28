//! Hand-projected browser oracle slice: resolved boxes, not an HTML/CSS parser.
//! Run: cargo run -p grunkalunka-layout --example native_block_bridge --locked
use grunkalunka_layout::{Block, Edges, Length, Rect, Style, layout};

fn main() {
    // Synthetic html/body ancestors have zero margins and padding. The fixture
    // deliberately avoids text shaping, percentage lengths and margin collapse.
    let blocks = [
        Block::default(),
        Block::child(0, 0),
        Block {
            parent: Some(1),
            style: Style {
                width: Length::Px(260),
                padding: Edges::all(10),
                border: Edges::all(2),
                ..Style::default()
            },
            ..Block::default()
        },
        Block {
            parent: Some(2),
            style: Style {
                width: Length::Px(100),
                height: Length::Px(24),
                margin: Edges {
                    top: 4,
                    left: 4,
                    ..Edges::default()
                },
                padding: Edges::all(6),
                border: Edges::all(3),
                ..Style::default()
            },
            ..Block::default()
        },
        Block {
            parent: Some(2),
            style: Style {
                width: Length::Px(80),
                height: Length::Px(10),
                padding: Edges::all(2),
                border: Edges::all(1),
                ..Style::default()
            },
            ..Block::default()
        },
    ];
    let frame = layout(&blocks, 800, 600).expect("native block bridge must lay out");
    let expected = [
        (
            "outer",
            2,
            Rect {
                x: 0,
                y: 0,
                width: 284,
                height: 86,
            },
        ),
        (
            "first",
            3,
            Rect {
                x: 16,
                y: 16,
                width: 118,
                height: 42,
            },
        ),
        (
            "second",
            4,
            Rect {
                x: 12,
                y: 58,
                width: 86,
                height: 16,
            },
        ),
    ];
    let entries: Vec<_> = expected
        .iter()
        .map(|(id, index, golden)| {
            let rect = frame.boxes[*index].expect("visible sample box").border;
            assert_eq!(rect, *golden, "independent native geometry for {id}");
            format!(
                r#""{id}":{{"x":{},"y":{},"width":{},"height":{}}}"#,
                rect.x, rect.y, rect.width, rect.height
            )
        })
        .collect();
    println!(
        r#"{{"schemaVersion":1,"kind":"grunkalunka-native-block-geometry","fixtureId":"native-block-bridge-v1","viewport":{{"width":800,"height":600,"devicePixelRatio":1}},"geometry":{{{}}}}}"#,
        entries.join(",")
    );
}
