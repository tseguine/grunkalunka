//! Hand-projected browser oracle slice: resolved boxes, not an HTML/CSS parser.
//! Run: cargo run -p grunkalunka-layout --example native_block_bridge --locked
use grunkalunka_layout::hit_test::{HitNode, hit_test, hit_test_scrolled};
use grunkalunka_layout::{
    Block, BoxSizing, Display, Edges, Frame, Length, Rect, Style, WidthBound, layout,
};

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

const PROBES: [(&str, u32, u32); 7] = [
    ("first-center", 20, 20),
    ("first-lower", 20, 34),
    ("second-original", 20, 60),
    ("left-margin", 13, 14),
    ("outer-border", 5, 5),
    ("outside", 300, 20),
    ("first-right", 133, 20),
];

fn hit_targets(frame: &Frame) -> Vec<Option<&'static str>> {
    let nodes = [
        HitNode::new(None),
        HitNode::new(Some(0)),
        HitNode::new(Some(1)),
        HitNode::new(Some(2)),
        HitNode::new(Some(2)),
    ];
    PROBES
        .iter()
        .map(
            |&(_, x, y)| match hit_test(frame, &nodes, x, y).expect("headless hit test") {
                Some(2) => Some("outer"),
                Some(3) => Some("first"),
                Some(4) => Some("second"),
                None | Some(0) | Some(1) => None,
                Some(other) => panic!("unexpected input node {other}"),
            },
        )
        .collect()
}

fn hits_json(values: &[Option<&str>]) -> String {
    assert_eq!(values.len(), PROBES.len());
    PROBES
        .iter()
        .zip(values)
        .map(|(&(name, _, _), target)| match target {
            Some(id) => format!(r#""{name}":"{id}""#),
            None => format!(r#""{name}":null"#),
        })
        .collect::<Vec<_>>()
        .join(",")
}

// A separately resolved percentage-height witness. The first sibling stands
// in for the original 86px-high outer box so the independent host begins at
// document y=96 after its 10px top margin. No CSS is parsed here.
fn percentage_heights_json() -> String {
    let blocks = [
        Block::default(),
        Block {
            parent: Some(0),
            style: Style {
                width: Length::Px(284),
                height: Length::Px(86),
                ..Style::default()
            },
            ..Block::default()
        },
        Block {
            parent: Some(0),
            style: Style {
                width: Length::Px(120),
                height: Length::Px(80),
                margin: Edges {
                    top: 10,
                    ..Edges::default()
                },
                padding: Edges::all(4),
                border: Edges::all(2),
                ..Style::default()
            },
            ..Block::default()
        },
        Block {
            parent: Some(2),
            style: Style {
                width: Length::Px(30),
                height: Length::Percent(50),
                margin: Edges {
                    top: 3,
                    bottom: 5,
                    ..Edges::default()
                },
                padding: Edges::all(2),
                border: Edges::all(1),
                ..Style::default()
            },
            ..Block::default()
        },
        Block {
            parent: Some(2),
            style: Style {
                width: Length::Px(20),
                height: Length::Percent(25),
                ..Style::default()
            },
            ..Block::default()
        },
    ];
    let frame = layout(&blocks, 800, 600).expect("definite percentage heights must resolve");
    let expected = [
        (
            "height-host",
            2,
            Rect {
                x: 0,
                y: 96,
                width: 132,
                height: 92,
            },
        ),
        (
            "height-first",
            3,
            Rect {
                x: 6,
                y: 105,
                width: 36,
                height: 46,
            },
        ),
        (
            "height-second",
            4,
            Rect {
                x: 6,
                y: 156,
                width: 20,
                height: 20,
            },
        ),
    ];
    for &(id, index, golden) in &expected {
        assert_eq!(
            frame.boxes[index]
                .expect("visible percentage-height box")
                .border,
            golden,
            "independent native percentage-height geometry for {id}"
        );
    }
    expected
        .into_iter()
        .map(|(id, index, _)| {
            let rect = frame.boxes[index].unwrap().border;
            format!(
                r#""{id}":{{"x":{},"y":{},"width":{},"height":{}}}"#,
                rect.x, rect.y, rect.width, rect.height
            )
        })
        .collect::<Vec<_>>()
        .join(",")
}

// Independently resolved boxes for the appended CSS border-box group. The
// preceding fixture groups are represented only by their reviewed fixed border
// heights; no browser boxes or parsed CSS become Rust layout inputs.
fn border_boxes_json() -> String {
    let blocks = [
        Block::default(),
        Block {
            parent: Some(0),
            style: Style {
                width: Length::Px(284),
                height: Length::Px(86),
                ..Style::default()
            },
            ..Block::default()
        },
        Block {
            parent: Some(0),
            style: Style {
                width: Length::Px(132),
                height: Length::Px(92),
                margin: Edges {
                    top: 10,
                    ..Edges::default()
                },
                ..Style::default()
            },
            ..Block::default()
        },
        Block {
            parent: Some(0),
            style: Style {
                box_sizing: BoxSizing::BorderBox,
                width: Length::Px(120),
                height: Length::Px(80),
                margin: Edges {
                    top: 10,
                    ..Edges::default()
                },
                border: Edges::all(2),
                padding: Edges::all(4),
                ..Style::default()
            },
            ..Block::default()
        },
        Block {
            parent: Some(3),
            style: Style {
                box_sizing: BoxSizing::BorderBox,
                width: Length::Percent(50),
                height: Length::Percent(50),
                margin: Edges {
                    top: 3,
                    bottom: 5,
                    ..Edges::default()
                },
                border: Edges::all(1),
                padding: Edges::all(2),
                ..Style::default()
            },
            ..Block::default()
        },
        Block {
            parent: Some(3),
            style: Style {
                width: Length::Percent(25),
                height: Length::Percent(25),
                ..Style::default()
            },
            ..Block::default()
        },
    ];
    let frame = layout(&blocks, 800, 600).expect("border-box geometry must resolve");
    assert_eq!(
        frame.boxes[3].unwrap().content,
        Rect {
            x: 6,
            y: 204,
            width: 108,
            height: 68,
        }
    );
    assert_eq!(
        frame.boxes[4].unwrap().content,
        Rect {
            x: 9,
            y: 210,
            width: 48,
            height: 28,
        }
    );
    let expected = [
        (
            "border-host",
            3,
            Rect {
                x: 0,
                y: 198,
                width: 120,
                height: 80,
            },
        ),
        (
            "border-first",
            4,
            Rect {
                x: 6,
                y: 207,
                width: 54,
                height: 34,
            },
        ),
        (
            "border-second",
            5,
            Rect {
                x: 6,
                y: 246,
                width: 27,
                height: 17,
            },
        ),
    ];
    for &(id, index, golden) in &expected {
        assert_eq!(
            frame.boxes[index]
                .expect("visible border-box sample")
                .border,
            golden,
            "independent native box-sizing geometry for {id}"
        );
    }
    expected
        .into_iter()
        .map(|(id, index, _)| {
            let rect = frame.boxes[index].unwrap().border;
            format!(
                r#""{id}":{{"x":{},"y":{},"width":{},"height":{}}}"#,
                rect.x, rect.y, rect.width, rect.height
            )
        })
        .collect::<Vec<_>>()
        .join(",")
}

// A document-level viewport-scroll witness. Its leading fixed boxes reproduce
// the *reviewed* occupied heights of the other fixture groups; the scroll
// target itself is laid out independently, not derived from browser evidence.
fn scroll_viewport_json() -> String {
    let blocks = [
        Block::default(),
        Block {
            parent: Some(0),
            style: Style {
                width: Length::Px(284),
                height: Length::Px(86),
                ..Style::default()
            },
            ..Block::default()
        },
        Block {
            parent: Some(0),
            style: Style {
                width: Length::Px(132),
                height: Length::Px(92),
                margin: Edges {
                    top: 10,
                    ..Edges::default()
                },
                ..Style::default()
            },
            ..Block::default()
        },
        Block {
            parent: Some(0),
            style: Style {
                width: Length::Px(120),
                height: Length::Px(80),
                margin: Edges {
                    top: 10,
                    ..Edges::default()
                },
                ..Style::default()
            },
            ..Block::default()
        },
        Block {
            parent: Some(0),
            style: Style {
                height: Length::Px(650),
                ..Style::default()
            },
            ..Block::default()
        },
        Block {
            parent: Some(0),
            style: Style {
                width: Length::Px(80),
                height: Length::Px(30),
                ..Style::default()
            },
            ..Block::default()
        },
    ];
    let frame = layout(&blocks, 800, 600).expect("scroll witness must lay out");
    let border = frame.boxes[5].expect("visible offscreen target").border;
    assert_eq!(
        border,
        Rect {
            x: 0,
            y: 928,
            width: 80,
            height: 30,
        }
    );
    assert_eq!((frame.scroll_width, frame.scroll_height), (800, 958));

    // Caller-supplied targetability deliberately excludes unrelated boxes.
    // Hit selection translates the same viewport point into document space.
    let nodes = [
        HitNode {
            parent: None,
            targetable: false,
        },
        HitNode {
            parent: Some(0),
            targetable: false,
        },
        HitNode {
            parent: Some(0),
            targetable: false,
        },
        HitNode {
            parent: Some(0),
            targetable: false,
        },
        HitNode {
            parent: Some(0),
            targetable: false,
        },
        HitNode::new(Some(0)),
    ];
    let sample = |scroll_y: u32, expected: Option<usize>| {
        let actual = hit_test_scrolled(&frame, &nodes, 10, 590, 0, scroll_y)
            .expect("valid document-space target");
        assert_eq!(actual, expected, "independent viewport-scroll hit golden");
        let rect = format!(
            r#"{{"x":{},"y":{},"width":{},"height":{}}}"#,
            border.x,
            border.y - scroll_y,
            border.width,
            border.height
        );
        let hit_json = if actual == Some(5) {
            r#""scroll-target""#
        } else {
            "null"
        };
        format!(
            r#"{{"scrollX":0,"scrollY":{scroll_y},"targetRect":{rect},"hitTarget":{hit_json}}}"#
        )
    };
    let before = sample(0, None);
    let scrolled = sample(350, Some(5));
    let restored = sample(0, None);
    format!(
        r#"{{"documentHeight":{},"before":{before},"scrolled":{scrolled},"restored":{restored}}}"#,
        frame.scroll_height
    )
}

// This additional group lives inside the fixture's pre-existing 650px fixed
// scroll spacer. The preceding fixed-height placeholders preserve its original
// y position, document scroll extent and unrelated real-engine observations.
fn width_bounds_json() -> String {
    let blocks = [
        Block::default(),
        Block {
            parent: Some(0),
            style: Style {
                width: Length::Px(284),
                height: Length::Px(86),
                ..Style::default()
            },
            ..Block::default()
        },
        Block {
            parent: Some(0),
            style: Style {
                width: Length::Px(132),
                height: Length::Px(92),
                margin: Edges {
                    top: 10,
                    ..Edges::default()
                },
                ..Style::default()
            },
            ..Block::default()
        },
        Block {
            parent: Some(0),
            style: Style {
                width: Length::Px(120),
                height: Length::Px(80),
                margin: Edges {
                    top: 10,
                    ..Edges::default()
                },
                ..Style::default()
            },
            ..Block::default()
        },
        Block {
            parent: Some(0),
            style: Style {
                height: Length::Px(650),
                ..Style::default()
            },
            ..Block::default()
        },
        Block {
            parent: Some(4),
            style: Style {
                width: Length::Px(400),
                height: Length::Px(80),
                ..Style::default()
            },
            ..Block::default()
        },
        Block {
            parent: Some(5),
            style: Style {
                box_sizing: BoxSizing::BorderBox,
                width: Length::Px(400),
                min_width: Some(WidthBound::Percent(45)),
                max_width: Some(WidthBound::Percent(30)),
                height: Length::Px(60),
                padding: Edges::all(4),
                border: Edges::all(2),
                ..Style::default()
            },
            ..Block::default()
        },
        Block {
            parent: Some(6),
            style: Style {
                width: Length::Percent(50),
                height: Length::Px(10),
                ..Style::default()
            },
            ..Block::default()
        },
        Block {
            parent: Some(6),
            style: Style {
                width: Length::Px(250),
                max_width: Some(WidthBound::Px(100)),
                height: Length::Px(8),
                ..Style::default()
            },
            ..Block::default()
        },
        Block {
            parent: Some(0),
            style: Style {
                width: Length::Px(80),
                height: Length::Px(30),
                ..Style::default()
            },
            ..Block::default()
        },
    ];
    let frame = layout(&blocks, 800, 600).expect("native width limits must lay out");
    assert_eq!(
        frame.boxes[5].unwrap().content,
        Rect {
            x: 0,
            y: 278,
            width: 400,
            height: 80,
        }
    );
    assert_eq!(
        frame.boxes[6].unwrap().content,
        Rect {
            x: 6,
            y: 284,
            width: 168,
            height: 48,
        }
    );
    assert_eq!((frame.scroll_width, frame.scroll_height), (800, 958));
    assert_eq!(
        frame.boxes[9].unwrap().border,
        Rect {
            x: 0,
            y: 928,
            width: 80,
            height: 30,
        }
    );
    let expected = [
        (
            "bounds-host",
            6,
            Rect {
                x: 0,
                y: 278,
                width: 180,
                height: 60,
            },
        ),
        (
            "bounds-first",
            7,
            Rect {
                x: 6,
                y: 284,
                width: 84,
                height: 10,
            },
        ),
        (
            "bounds-second",
            8,
            Rect {
                x: 6,
                y: 294,
                width: 100,
                height: 8,
            },
        ),
    ];
    for &(id, index, golden) in &expected {
        assert_eq!(
            frame.boxes[index]
                .expect("visible min/max-width witness")
                .border,
            golden,
            "independent native constrained-width rectangle for {id}"
        );
    }
    expected
        .into_iter()
        .map(|(id, index, _)| {
            let rect = frame.boxes[index].unwrap().border;
            format!(
                r#""{id}":{{"x":{},"y":{},"width":{},"height":{}}}"#,
                rect.x, rect.y, rect.width, rect.height
            )
        })
        .collect::<Vec<_>>()
        .join(",")
}

fn main() {
    // Synthetic html/body ancestors have zero margins and padding. The first
    // group avoids text shaping and percentage lengths; appended groups use
    // separate, explicitly caller-resolved dimensions.
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

    // These host-chosen sample positions use only visible border rectangles
    // with no z-index, clipping, scrolling, text, pointer-events or transforms.
    // Check exact targets independently of browser DOM hit-testing.
    let original_hits = hit_targets(&frame);
    let hidden_hits = hit_targets(&hidden);
    let restored_hits = hit_targets(&restored);
    assert_eq!(
        original_hits,
        vec![
            Some("first"),
            Some("first"),
            Some("second"),
            Some("outer"),
            Some("outer"),
            None,
            Some("first"),
        ]
    );
    assert_eq!(
        hidden_hits,
        vec![
            Some("second"),
            Some("outer"),
            None,
            Some("second"),
            Some("outer"),
            None,
            Some("outer"),
        ]
    );
    assert_eq!(restored_hits, original_hits);

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
    output.push_str(r#"}},"hitTargets":{"original":{"#);
    output.push_str(&hits_json(&original_hits));
    output.push_str(r#"},"hidden":{"#);
    output.push_str(&hits_json(&hidden_hits));
    output.push_str(r#"},"restored":{"#);
    output.push_str(&hits_json(&restored_hits));
    output.push_str(r#"}},"percentageHeights":{"#);
    output.push_str(&percentage_heights_json());
    output.push_str(r#"},"borderBoxes":{"#);
    output.push_str(&border_boxes_json());
    output.push_str(r#"},"scrollViewport":"#);
    output.push_str(&scroll_viewport_json());
    output.push_str(r#","widthBounds":{"#);
    output.push_str(&width_bounds_json());
    output.push_str(r#"}}"#);
    println!("{output}");
}
