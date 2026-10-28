//! Hand-projected browser oracle slice: resolved boxes, not an HTML/CSS parser.
//! Run: cargo run -p grunkalunka-layout --example native_block_bridge --locked
use grunkalunka_layout::{Block, Display, Edges, Frame, Length, Rect, Style, layout};

// Only native border boxes: this receipt never claims HTML/CSS layout parsing.
fn boxes_json(frame: &Frame) -> String {
    [("outer", 2), ("first", 3), ("second", 4)]
        .into_iter()
        .map(|(id, index)| match frame.boxes[index] {
            Some(geometry) => {
                let rect = geometry.border;
                format!(
                    r#""{id}":{{"x":{},"y":{},"width":{},"height":{}}}"#,
                    rect.x, rect.y, rect.width, rect.height
                )
            }
            None => format!(r#""{id}":null"#),
        })
        .collect::<Vec<_>>()
        .join(",")
}

fn main() {
    // Synthetic html/body ancestors have zero margins and padding. The fixture
    // deliberately avoids text shaping, percentage lengths and margin collapse.
    let mut blocks = [
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
    for &(id, index, golden) in &expected {
        assert_eq!(
            frame.boxes[index].expect("visible sample box").border,
            golden,
            "independent native geometry for {id}"
        );
    }

    // Display:none removes the entire first box; the following sibling
    // reflows upward and the auto-sized parent shrinks. No cache/state from
    // the previous layout may leak into the new frame.
    blocks[3].style.display = Display::None;
    let hidden = layout(&blocks, 800, 600).expect("native hide must lay out");
    assert_eq!(hidden.boxes[3], None);
    assert_eq!(
        hidden.boxes[2].expect("visible outer box").border,
        Rect {
            x: 0,
            y: 0,
            width: 284,
            height: 40,
        }
    );
    assert_eq!(
        hidden.boxes[4].expect("reflowed sibling").border,
        Rect {
            x: 12,
            y: 12,
            width: 86,
            height: 16,
        }
    );

    // Restore the original style and prove every node returns to its original
    // geometry, not merely the three rectangles selected for comparison.
    blocks[3].style.display = Display::Block;
    let restored = layout(&blocks, 800, 600).expect("native restore must lay out");
    assert_eq!(restored.boxes, frame.boxes);

    let baseline = boxes_json(&frame);
    let hidden_json = boxes_json(&hidden);
    let restored_json = boxes_json(&restored);
    let mut output = String::from(
        r#"{"schemaVersion":1,"kind":"grunkalunka-native-block-geometry","fixtureId":"native-block-bridge-v1","viewport":{"width":800,"height":600,"devicePixelRatio":1},"geometry":{"#,
    );
    output.push_str(&baseline);
    output.push_str(r#"},"reflow":{"hidden":{"#);
    output.push_str(&hidden_json);
    output.push_str(r#"},"restored":{"#);
    output.push_str(&restored_json);
    output.push_str(r#"}}}"#);
    println!("{output}");
}
