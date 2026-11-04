//! Independent resolved input for the fixed-row browser witness. No HTML/CSS
//! parser or browser receipt is used to construct these native boxes or hits.

use grunkalunka_layout::hit_test::{HitNode, hit_test_scrolled};
use grunkalunka_layout::{
    Block, DescendantClip, Display, Edges, FlexWrap, Frame, Length, Rect, RelativeOffset, Style,
    layout,
};

const IDS: [(&str, usize); 6] = [
    ("row-host", 1),
    ("row-a", 2),
    ("row-b", 4),
    ("row-inner", 5),
    ("row-c", 6),
    ("row-after", 7),
];
const PROBES: [(&str, u32, u32); 9] = [
    ("a", 30, 30),
    ("b", 110, 32),
    ("nowrap", 180, 30),
    ("wrap", 30, 80),
    ("gap-wrap", 118, 62),
    ("after", 10, 110),
    ("host-border", 21, 21),
    ("gap", 95, 30),
    ("outside", 250, 50),
];
const PHASES: [&str; 10] = [
    "nowrap",
    "wrap",
    "wide",
    "columnGap",
    "rowGap",
    "hidden",
    "relative",
    "clipped",
    "scrolled",
    "restored",
];

fn source() -> [Block; 8] {
    [
        Block {
            intrinsic_height: 1200,
            ..Block::default()
        },
        Block {
            parent: Some(0),
            style: Style {
                display: Display::FlexRow,
                width: Length::Px(150),
                column_gap: 10,
                row_gap: 8,
                margin: Edges {
                    top: 20,
                    left: 20,
                    ..Edges::default()
                },
                padding: Edges::all(4),
                border: Edges::all(2),
                ..Style::default()
            },
            ..Block::default()
        },
        Block {
            parent: Some(1),
            style: Style {
                width: Length::Px(60),
                height: Length::Px(20),
                padding: Edges::all(1),
                border: Edges::all(1),
                ..Style::default()
            },
            ..Block::default()
        },
        Block {
            parent: Some(1),
            style: Style {
                display: Display::Contents,
                width: Length::Percent(33),
                padding: Edges::all(99),
                ..Style::default()
            },
            ..Block::default()
        },
        Block {
            parent: Some(3),
            style: Style {
                width: Length::Px(50),
                height: Length::Px(30),
                margin: Edges {
                    top: 2,
                    right: 3,
                    bottom: 4,
                    left: 5,
                },
                padding: Edges::all(2),
                border: Edges::all(1),
                ..Style::default()
            },
            ..Block::default()
        },
        Block {
            parent: Some(4),
            style: Style {
                width: Length::Percent(50),
                height: Length::Px(8),
                ..Style::default()
            },
            ..Block::default()
        },
        Block {
            parent: Some(3),
            style: Style {
                width: Length::Px(40),
                height: Length::Px(16),
                border: Edges::all(1),
                ..Style::default()
            },
            ..Block::default()
        },
        Block {
            parent: Some(0),
            style: Style {
                width: Length::Px(180),
                height: Length::Px(12),
                margin: Edges {
                    top: 6,
                    ..Edges::default()
                },
                ..Style::default()
            },
            ..Block::default()
        },
    ]
}

fn phase_source(phase: &str) -> ([Block; 8], u32) {
    let mut blocks = source();
    if phase != "nowrap" && phase != "restored" {
        blocks[1].style.flex_wrap = FlexWrap::Wrap;
    }
    match phase {
        "wide" => blocks[1].style.width = Length::Px(220),
        "columnGap" => blocks[1].style.column_gap = 24,
        "rowGap" => blocks[1].style.row_gap = 18,
        "hidden" => blocks[4].style.display = Display::None,
        "relative" => blocks[4].style.relative_offset = RelativeOffset { x: 8, y: 9 },
        "clipped" => {
            blocks[1].style.height = Length::Px(36);
            blocks[1].style.descendant_clip = DescendantClip::PaddingBox;
        }
        "nowrap" | "wrap" | "scrolled" | "restored" => {}
        _ => panic!("unknown native flex phase"),
    }
    (blocks, if phase == "scrolled" { 10 } else { 0 })
}

fn rect(x: u32, y: u32, width: u32, height: u32) -> Option<Rect> {
    Some(Rect {
        x,
        y,
        width,
        height,
    })
}

// Literal independent geometry, in IDS order. The row's normal-flow outer
// widths are 64, 64 and 42 pixels; its item heights are 24, 42 and 18 pixels.
// These expected values are not derived by the engine under test.
fn expected_boxes(phase: &str) -> [Option<Rect>; 6] {
    let mut expected = [
        rect(20, 20, 162, 54),
        rect(26, 26, 64, 24),
        rect(105, 28, 56, 36),
        rect(108, 31, 25, 8),
        rect(174, 26, 42, 18),
        rect(0, 80, 180, 12),
    ];
    match phase {
        "nowrap" | "restored" => {}
        "wide" => expected[0] = rect(20, 20, 232, 54),
        "wrap" | "relative" | "clipped" | "scrolled" => {
            expected[0] = rect(20, 20, 162, 80);
            expected[4] = rect(26, 76, 42, 18);
            expected[5] = rect(0, 106, 180, 12);
            if phase == "relative" {
                expected[2] = rect(113, 37, 56, 36);
                expected[3] = rect(116, 40, 25, 8);
            } else if phase == "clipped" {
                expected[0] = rect(20, 20, 162, 48);
                expected[5] = rect(0, 74, 180, 12);
            }
        }
        "columnGap" => {
            expected[0] = rect(20, 20, 162, 86);
            expected[2] = rect(31, 60, 56, 36);
            expected[3] = rect(34, 63, 25, 8);
            expected[4] = rect(114, 58, 42, 18);
            expected[5] = rect(0, 112, 180, 12);
        }
        "rowGap" => {
            expected[0] = rect(20, 20, 162, 90);
            expected[4] = rect(26, 86, 42, 18);
            expected[5] = rect(0, 116, 180, 12);
        }
        "hidden" => {
            expected[0] = rect(20, 20, 162, 36);
            expected[2] = None;
            expected[3] = None;
            expected[4] = rect(100, 26, 42, 18);
            expected[5] = rect(0, 62, 180, 12);
        }
        _ => panic!("unknown native flex phase"),
    }
    expected
}

fn expected_hits(phase: &str) -> [Option<&'static str>; 9] {
    let mut expected = [
        Some("row-a"),
        Some("row-inner"),
        Some("row-c"),
        Some("row-after"),
        Some("row-b"),
        None,
        Some("row-host"),
        Some("row-host"),
        None,
    ];
    match phase {
        "nowrap" | "restored" => {}
        "wide" => expected[8] = Some("row-host"),
        "wrap" | "relative" | "scrolled" => {
            expected[2] = Some("row-host");
            expected[3] = Some("row-c");
            expected[5] = Some("row-after");
            if phase == "relative" {
                expected[1] = Some("row-host");
            } else if phase == "scrolled" {
                expected[1] = Some("row-b");
                expected[4] = Some("row-host");
                expected[5] = None;
            }
        }
        "columnGap" => {
            expected[1] = Some("row-host");
            expected[2] = Some("row-host");
            expected[3] = Some("row-host");
            expected[4] = Some("row-c");
        }
        "rowGap" => {
            expected[2] = Some("row-host");
            expected[3] = Some("row-host");
        }
        "hidden" => {
            expected[1] = Some("row-c");
            expected[2] = Some("row-host");
            expected[3] = None;
            expected[4] = Some("row-after");
        }
        "clipped" => expected[2] = Some("row-host"),
        _ => panic!("unknown native flex phase"),
    }
    expected
}

fn box_json(value: Option<Rect>) -> String {
    match value {
        Some(value) => format!(
            r#"{{"x":{},"y":{},"width":{},"height":{}}}"#,
            value.x, value.y, value.width, value.height
        ),
        None => String::from("null"),
    }
}

fn receipt(phase: &str) -> (Frame, String) {
    let (blocks, scroll_y) = phase_source(phase);
    let frame = layout(&blocks, 800, 600).expect("native fixed-row layout");
    assert!(frame.contents[3]);
    assert_eq!(frame.boxes[3], None, "contents wrapper has no box");
    let expected = expected_boxes(phase);
    let boxes = IDS
        .iter()
        .zip(expected)
        .map(|(&(id, node), expected)| {
            let observed = frame.boxes[node].map(|geometry| geometry.border);
            assert_eq!(observed, expected, "{phase}: {id}");
            format!(r#""{id}":{}"#, box_json(observed))
        })
        .collect::<Vec<_>>()
        .join(",");
    let targets: Vec<_> = blocks
        .iter()
        .map(|block| HitNode::new(block.parent))
        .collect();
    let expected = expected_hits(phase);
    let hits = PROBES
        .iter()
        .zip(expected)
        .map(|(&(probe, x, y), expected)| {
            let node = hit_test_scrolled(&frame, &targets, x, y, 0, scroll_y)
                .expect("native fixed-row hit");
            let id = node.and_then(|node| {
                IDS.iter()
                    .find_map(|&(id, index)| (node == index).then_some(id))
            });
            assert_eq!(id, expected, "{phase}: hit {probe}");
            match id {
                Some(id) => format!(r#""{probe}":"{id}""#),
                None => format!(r#""{probe}":null"#),
            }
        })
        .collect::<Vec<_>>()
        .join(",");
    (
        frame,
        format!(r#""{phase}":{{"scrollY":{scroll_y},"boxes":{{{boxes}}},"hits":{{{hits}}}}}"#),
    )
}

fn main() {
    let mut baseline = None;
    let states = PHASES
        .into_iter()
        .map(|phase| {
            let (frame, state) = receipt(phase);
            if phase == "nowrap" {
                baseline = Some(frame);
            } else if phase == "restored" {
                assert_eq!(baseline.as_ref(), Some(&frame), "full frame restoration");
            }
            state
        })
        .collect::<Vec<_>>()
        .join(",");
    println!(
        r#"{{"schemaVersion":1,"kind":"grunkalunka-native-flex-geometry","fixtureId":"native-flex-bridge-v1","viewport":{{"width":800,"height":600,"devicePixelRatio":1}},"states":{{{states}}}}}"#
    );
}
