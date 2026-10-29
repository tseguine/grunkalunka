//! Hand-projected browser oracle slice: resolved boxes, not an HTML/CSS parser.
//! Run: cargo run -p grunkalunka-layout --example native_block_bridge --locked
use grunkalunka_layout::hit_test::{HitNode, hit_test, hit_test_scrolled};
use grunkalunka_layout::{
    Block, BoxSizing, DescendantClip, Display, Edges, Frame, HeightBound, HorizontalAutoMargins,
    Length, PercentageMargins, PointerEvents, Rect, RelativeOffset, Style, Visibility, WidthBound,
    layout,
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
fn width_and_height_bounds_json() -> (String, String) {
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
        Block {
            parent: Some(5),
            style: Style {
                box_sizing: BoxSizing::BorderBox,
                width: Length::Px(80),
                height: Length::Px(90),
                min_height: Some(HeightBound::Percent(75)),
                max_height: Some(HeightBound::Percent(50)),
                padding: Edges::all(4),
                border: Edges::all(2),
                ..Style::default()
            },
            ..Block::default()
        },
        Block {
            parent: Some(10),
            style: Style {
                width: Length::Px(30),
                height: Length::Percent(50),
                ..Style::default()
            },
            ..Block::default()
        },
        Block {
            parent: Some(10),
            style: Style {
                width: Length::Px(20),
                height: Length::Px(12),
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
    assert_eq!(
        frame.boxes[10].unwrap().content,
        Rect {
            x: 6,
            y: 344,
            width: 68,
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
    let expected_height = [
        (
            "height-limited",
            10,
            Rect {
                x: 0,
                y: 338,
                width: 80,
                height: 60,
            },
        ),
        (
            "height-limited-first",
            11,
            Rect {
                x: 6,
                y: 344,
                width: 30,
                height: 24,
            },
        ),
        (
            "height-limited-second",
            12,
            Rect {
                x: 6,
                y: 368,
                width: 20,
                height: 12,
            },
        ),
    ];
    for &(id, index, golden) in &expected_height {
        assert_eq!(
            frame.boxes[index]
                .expect("visible min/max-height witness")
                .border,
            golden,
            "independent native constrained-height rectangle for {id}"
        );
    }
    let serialize = |entries: &[(&str, usize, Rect)]| {
        entries
            .iter()
            .map(|&(id, index, _)| {
                let rect = frame.boxes[index].unwrap().border;
                format!(
                    r#""{id}":{{"x":{},"y":{},"width":{},"height":{}}}"#,
                    rect.x, rect.y, rect.width, rect.height
                )
            })
            .collect::<Vec<_>>()
            .join(",")
    };
    (serialize(&expected), serialize(&expected_height))
}

// Source-checked visibility witness independent of the browser's DOM boxes.
// Fixed-height prefixes occupy only their reviewed earlier flow height; the
// hidden group lives in the existing spacer and cannot alter document extents.
fn visibility_states_json() -> String {
    let mut blocks = [
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
            parent: Some(4),
            style: Style {
                width: Length::Px(40),
                height: Length::Px(25),
                visibility: Visibility::Hidden,
                margin: Edges {
                    left: 300,
                    ..Edges::default()
                },
                ..Style::default()
            },
            ..Block::default()
        },
        Block {
            parent: Some(6),
            style: Style {
                width: Length::Px(14),
                height: Length::Px(6),
                visibility: Visibility::Visible,
                ..Style::default()
            },
            ..Block::default()
        },
        Block {
            parent: Some(6),
            style: Style {
                width: Length::Px(12),
                height: Length::Px(5),
                ..Style::default()
            },
            ..Block::default()
        },
        Block {
            parent: Some(4),
            style: Style {
                width: Length::Px(20),
                height: Length::Px(5),
                margin: Edges {
                    left: 300,
                    ..Edges::default()
                },
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
    let hidden = layout(&blocks, 800, 600).expect("hidden visibility layout");
    let expected = [
        (
            "visibility-host",
            6,
            Rect {
                x: 300,
                y: 358,
                width: 40,
                height: 25,
            },
        ),
        (
            "visibility-child",
            7,
            Rect {
                x: 300,
                y: 358,
                width: 14,
                height: 6,
            },
        ),
        (
            "visibility-inherited",
            8,
            Rect {
                x: 300,
                y: 364,
                width: 12,
                height: 5,
            },
        ),
        (
            "visibility-sibling",
            9,
            Rect {
                x: 300,
                y: 383,
                width: 20,
                height: 5,
            },
        ),
    ];
    for &(id, index, golden) in &expected {
        assert_eq!(
            hidden.boxes[index]
                .expect("visible geometry despite style")
                .border,
            golden,
            "independently resolved visibility geometry for {id}"
        );
    }
    assert_eq!(hidden.visibility[6..10], [false, true, false, true]);
    assert_eq!((hidden.scroll_width, hidden.scroll_height), (800, 958));
    assert_eq!(
        hidden.boxes[10].unwrap().border,
        Rect {
            x: 0,
            y: 928,
            width: 80,
            height: 30,
        }
    );

    // A targetable flag is caller input policy, independent of computed
    // visibility. Only this selected group participates in the hit probe.
    let hit_nodes: Vec<_> = blocks
        .iter()
        .enumerate()
        .map(|(index, block)| HitNode {
            parent: block.parent,
            targetable: (6..=9).contains(&index),
        })
        .collect();
    let probes = [
        ("child", 301, 359),
        ("host", 318, 360),
        ("inherited", 301, 366),
        ("sibling", 301, 385),
    ];
    let expected_hidden = [Some(7), None, None, Some(9)];
    blocks[6].style.visibility = Visibility::Visible;
    let visible = layout(&blocks, 800, 600).expect("visible override layout");
    assert_eq!(hidden.boxes, visible.boxes);
    assert_eq!(visible.visibility[6..10], [true, true, true, true]);
    blocks[6].style.visibility = Visibility::Hidden;
    let restored = layout(&blocks, 800, 600).expect("restored inheritance");
    assert_eq!(hidden, restored);

    for (frame, targets) in [
        (&hidden, expected_hidden),
        (&visible, [Some(7), Some(6), Some(8), Some(9)]),
        (&restored, expected_hidden),
    ] {
        for ((probe, x, y), expected_hit) in probes.iter().copied().zip(targets) {
            assert_eq!(
                hit_test(frame, &hit_nodes, x, y),
                Ok(expected_hit),
                "native visibility hit probe {probe}"
            );
        }
    }

    let capture = |frame: &Frame| {
        let boxes = expected
            .iter()
            .map(|&(id, index, _)| {
                let rect = frame.boxes[index].unwrap().border;
                format!(
                    r#""{id}":{{"x":{},"y":{},"width":{},"height":{}}}"#,
                    rect.x, rect.y, rect.width, rect.height
                )
            })
            .collect::<Vec<_>>()
            .join(",");
        let computed = [
            ("visibility-host", 6),
            ("visibility-child", 7),
            ("visibility-inherited", 8),
            ("visibility-sibling", 9),
        ]
        .into_iter()
        .map(|(id, index)| format!(r#""{id}":{}"#, frame.visibility[index]))
        .collect::<Vec<_>>()
        .join(",");
        let hits = probes
            .iter()
            .map(|&(name, x, y)| {
                let target = hit_test(frame, &hit_nodes, x, y).unwrap();
                let id = match target {
                    Some(6) => r#""visibility-host""#,
                    Some(7) => r#""visibility-child""#,
                    Some(8) => r#""visibility-inherited""#,
                    Some(9) => r#""visibility-sibling""#,
                    None => "null",
                    _ => panic!("unexpected native visibility target"),
                };
                format!(r#""{name}":{id}"#)
            })
            .collect::<Vec<_>>()
            .join(",");
        format!(r#"{{"boxes":{{{boxes}}},"computed":{{{computed}}},"hits":{{{hits}}}}}"#)
    };
    let first = capture(&hidden);
    let shown = capture(&visible);
    let final_state = capture(&restored);
    format!(r#"{{"hidden":{first},"visible":{shown},"restored":{final_state}}}"#)
}

// Separate source-checked descendant-clipping witness. Prefix groups occupy
// only their previously asserted flow heights, preserving the original 958px
// document and 350px global-scroll evidence. No HTML/CSS parser is involved.
fn descendant_clip_states_json() -> String {
    let mut blocks = [
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
            parent: Some(4),
            style: Style {
                width: Length::Px(40),
                height: Length::Px(25),
                margin: Edges {
                    left: 300,
                    ..Edges::default()
                },
                ..Style::default()
            },
            ..Block::default()
        },
        Block {
            parent: Some(4),
            style: Style {
                width: Length::Px(20),
                height: Length::Px(5),
                margin: Edges {
                    left: 300,
                    ..Edges::default()
                },
                ..Style::default()
            },
            ..Block::default()
        },
        Block {
            parent: Some(4),
            style: Style {
                width: Length::Px(20),
                height: Length::Px(12),
                margin: Edges {
                    left: 100,
                    ..Edges::default()
                },
                padding: Edges::all(3),
                border: Edges::all(2),
                descendant_clip: DescendantClip::PaddingBox,
                ..Style::default()
            },
            ..Block::default()
        },
        Block {
            parent: Some(8),
            style: Style {
                width: Length::Px(40),
                height: Length::Px(18),
                ..Style::default()
            },
            ..Block::default()
        },
        Block {
            parent: Some(4),
            style: Style {
                width: Length::Px(10),
                height: Length::Px(5),
                margin: Edges {
                    left: 100,
                    ..Edges::default()
                },
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
    let clipped = layout(&blocks, 800, 600).expect("bounded descendant clip");
    let goldens = [
        (
            "clip-parent",
            8,
            Rect {
                x: 100,
                y: 388,
                width: 30,
                height: 22,
            },
        ),
        (
            "clip-child",
            9,
            Rect {
                x: 105,
                y: 393,
                width: 40,
                height: 18,
            },
        ),
        (
            "clip-sibling",
            10,
            Rect {
                x: 100,
                y: 410,
                width: 10,
                height: 5,
            },
        ),
    ];
    for &(id, index, rect) in &goldens {
        assert_eq!(
            clipped.boxes[index]
                .expect("clip witness box exists")
                .border,
            rect,
            "native independent clip geometry for {id}"
        );
    }
    assert_eq!(
        clipped.boxes[8].unwrap().padding,
        Rect {
            x: 102,
            y: 390,
            width: 26,
            height: 18,
        }
    );
    assert!(clipped.clip_descendants[8]);
    assert_eq!((clipped.scroll_width, clipped.scroll_height), (800, 958));
    assert_eq!(
        clipped.boxes[11].unwrap().border,
        Rect {
            x: 0,
            y: 928,
            width: 80,
            height: 30,
        }
    );

    // No compiler or DOM hit-test is consulted to choose the native targets.
    let targets: Vec<_> = blocks
        .iter()
        .enumerate()
        .map(|(index, node)| HitNode {
            parent: node.parent,
            targetable: (8..=10).contains(&index),
        })
        .collect();
    let probes = [
        ("inside", 106, 395),
        ("padding-edge", 127, 395),
        ("border-edge", 128, 395),
        ("outside-parent", 135, 395),
        ("bottom-border", 106, 409),
        ("sibling", 105, 412),
        ("parent-border", 101, 389),
    ];
    let expected_clipped = [Some(9), Some(9), Some(8), None, Some(8), Some(10), Some(8)];
    blocks[8].style.descendant_clip = DescendantClip::None;
    let visible = layout(&blocks, 800, 600).expect("unclipped descendant layout");
    assert_eq!(
        clipped.boxes, visible.boxes,
        "clip must not change normal flow"
    );
    assert_eq!((visible.scroll_width, visible.scroll_height), (800, 958));
    blocks[8].style.descendant_clip = DescendantClip::PaddingBox;
    let restored = layout(&blocks, 800, 600).expect("restored descendant clip");
    assert_eq!(clipped, restored);

    for (frame, expected) in [
        (&clipped, expected_clipped),
        (
            &visible,
            [
                Some(9),
                Some(9),
                Some(9),
                Some(9),
                Some(9),
                Some(10),
                Some(8),
            ],
        ),
        (&restored, expected_clipped),
    ] {
        for ((name, x, y), target) in probes.iter().copied().zip(expected) {
            assert_eq!(
                hit_test(frame, &targets, x, y),
                Ok(target),
                "independent native clipping probe {name}"
            );
        }
    }
    let receipt = |frame: &Frame, mode: &str| {
        let boxes = goldens
            .iter()
            .map(|&(id, index, _)| {
                let rect = frame.boxes[index].unwrap().border;
                format!(
                    r#""{id}":{{"x":{},"y":{},"width":{},"height":{}}}"#,
                    rect.x, rect.y, rect.width, rect.height
                )
            })
            .collect::<Vec<_>>()
            .join(",");
        let hits = probes
            .iter()
            .map(|&(name, x, y)| {
                let id = match hit_test(frame, &targets, x, y).unwrap() {
                    Some(8) => r#""clip-parent""#,
                    Some(9) => r#""clip-child""#,
                    Some(10) => r#""clip-sibling""#,
                    None => "null",
                    _ => panic!("unexpected source-independent clip hit"),
                };
                format!(r#""{name}":{id}"#)
            })
            .collect::<Vec<_>>()
            .join(",");
        format!(r#"{{"mode":"{mode}","boxes":{{{boxes}}},"hits":{{{hits}}}}}"#)
    };
    let initial = receipt(&clipped, "clip");
    let expanded = receipt(&visible, "visible");
    let final_state = receipt(&restored, "clip");
    format!(r#"{{"clipped":{initial},"visible":{expanded},"restored":{final_state}}}"#)
}

// Independent block-flow witness for CSS display:contents. Preceding nodes
// reproduce only their reviewed fixed occupied heights, not DOM measurements.
// The wrapper may have no principal box while its descendants still hit/paint.
fn boxless_states_json() -> String {
    let mut blocks = [
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
            parent: Some(4),
            style: Style {
                height: Length::Px(25),
                margin: Edges {
                    left: 300,
                    ..Edges::default()
                },
                ..Style::default()
            },
            ..Block::default()
        },
        Block {
            parent: Some(4),
            style: Style {
                height: Length::Px(5),
                margin: Edges {
                    left: 300,
                    ..Edges::default()
                },
                ..Style::default()
            },
            ..Block::default()
        },
        Block {
            parent: Some(4),
            style: Style {
                height: Length::Px(22),
                margin: Edges {
                    left: 100,
                    ..Edges::default()
                },
                ..Style::default()
            },
            ..Block::default()
        },
        Block {
            parent: Some(4),
            style: Style {
                height: Length::Px(5),
                margin: Edges {
                    left: 100,
                    ..Edges::default()
                },
                ..Style::default()
            },
            ..Block::default()
        },
        Block {
            parent: Some(4),
            style: Style {
                display: Display::Contents,
                width: Length::Px(40),
                height: Length::Px(24),
                margin: Edges {
                    top: 2,
                    left: 200,
                    ..Edges::default()
                },
                padding: Edges::all(3),
                border: Edges::all(2),
                ..Style::default()
            },
            ..Block::default()
        },
        Block {
            parent: Some(10),
            style: Style {
                width: Length::Px(24),
                height: Length::Px(10),
                ..Style::default()
            },
            ..Block::default()
        },
        Block {
            parent: Some(10),
            style: Style {
                width: Length::Px(12),
                height: Length::Px(6),
                ..Style::default()
            },
            ..Block::default()
        },
        Block {
            parent: Some(4),
            style: Style {
                width: Length::Px(20),
                height: Length::Px(5),
                margin: Edges {
                    left: 200,
                    ..Edges::default()
                },
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
    let contents = layout(&blocks, 800, 600).expect("flattened boxless flow");
    assert!(contents.contents[10]);
    assert_eq!(contents.boxes[10], None);
    assert_eq!((contents.scroll_width, contents.scroll_height), (800, 958));
    assert_eq!(
        contents.boxes[14].unwrap().border,
        Rect {
            x: 0,
            y: 928,
            width: 80,
            height: 30,
        }
    );
    let box_ids = [
        ("contents-first", 11),
        ("contents-second", 12),
        ("contents-sibling", 13),
    ];
    let contents_goldens = [
        Rect {
            x: 0,
            y: 415,
            width: 24,
            height: 10,
        },
        Rect {
            x: 0,
            y: 425,
            width: 12,
            height: 6,
        },
        Rect {
            x: 200,
            y: 431,
            width: 20,
            height: 5,
        },
    ];
    for ((id, index), golden) in box_ids.into_iter().zip(contents_goldens) {
        assert_eq!(
            contents.boxes[index].unwrap().border,
            golden,
            "flattened native box for {id}"
        );
    }
    let hit_nodes: Vec<_> = blocks
        .iter()
        .enumerate()
        .map(|(index, block)| HitNode {
            parent: block.parent,
            targetable: (10..=13).contains(&index),
        })
        .collect();
    let probes = [
        ("first-contents", 1, 416),
        ("second-contents", 1, 427),
        ("wrapper-border", 201, 418),
        ("first-block", 206, 424),
        ("sibling-contents", 202, 432),
        ("sibling-block", 202, 452),
        ("outside", 251, 431),
    ];
    let expected_contents = [Some(11), Some(12), None, None, Some(13), None, None];

    blocks[10].style.display = Display::Block;
    let block = layout(&blocks, 800, 600).expect("normal box wrapper layout");
    assert!(!block.contents[10]);
    assert_eq!(
        block.boxes[10].unwrap().border,
        Rect {
            x: 200,
            y: 417,
            width: 50,
            height: 34,
        }
    );
    let block_goldens = [
        Rect {
            x: 205,
            y: 422,
            width: 24,
            height: 10,
        },
        Rect {
            x: 205,
            y: 432,
            width: 12,
            height: 6,
        },
        Rect {
            x: 200,
            y: 451,
            width: 20,
            height: 5,
        },
    ];
    for ((id, index), golden) in box_ids.into_iter().zip(block_goldens) {
        assert_eq!(
            block.boxes[index].unwrap().border,
            golden,
            "box-bearing native block for {id}"
        );
    }
    assert_eq!((block.scroll_width, block.scroll_height), (800, 958));
    assert_eq!(block.boxes[14], contents.boxes[14]);

    blocks[10].style.display = Display::Contents;
    let restored = layout(&blocks, 800, 600).expect("restored boxless flow");
    assert_eq!(restored, contents);
    let expected_block = [None, None, Some(10), Some(11), Some(10), Some(13), None];
    for (frame, expected) in [
        (&contents, expected_contents),
        (&block, expected_block),
        (&restored, expected_contents),
    ] {
        for ((name, x, y), target) in probes.iter().copied().zip(expected) {
            assert_eq!(
                hit_test(frame, &hit_nodes, x, y),
                Ok(target),
                "independent native boxless probe {name}"
            );
        }
    }

    let receipt = |frame: &Frame, display: &str| {
        let boxes = box_ids
            .iter()
            .map(|&(id, index)| {
                let rect = frame.boxes[index].unwrap().border;
                format!(
                    r#""{id}":{{"x":{},"y":{},"width":{},"height":{}}}"#,
                    rect.x, rect.y, rect.width, rect.height
                )
            })
            .collect::<Vec<_>>()
            .join(",");
        let hits = probes
            .iter()
            .map(|&(name, x, y)| {
                let target = hit_test(frame, &hit_nodes, x, y).unwrap();
                let id = match target {
                    Some(10) => r#""contents-wrapper""#,
                    Some(11) => r#""contents-first""#,
                    Some(12) => r#""contents-second""#,
                    Some(13) => r#""contents-sibling""#,
                    None => "null",
                    _ => panic!("unexpected native boxless hit"),
                };
                format!(r#""{name}":{id}"#)
            })
            .collect::<Vec<_>>()
            .join(",");
        let has_box = frame.boxes[10].is_some();
        format!(
            r#"{{"display":"{display}","hasBox":{has_box},"boxes":{{{boxes}}},"hits":{{{hits}}}}}"#
        )
    };
    let first = receipt(&contents, "contents");
    let shown = receipt(&block, "block");
    let final_state = receipt(&restored, "contents");
    format!(r#"{{"contents":{first},"block":{shown},"restored":{final_state}}}"#)
}

// A source-independent native witness for inherited pointer-events none/auto.
// Earlier fixed blocks occupy their reviewed normal-flow heights; the pointer
// group begins at y436 after the contents, clip and visibility groups.
fn pointer_states_json() -> String {
    let mut blocks = [
        Block::default(),
        Block {
            parent: Some(0),
            style: Style {
                height: Length::Px(86),
                ..Style::default()
            },
            ..Block::default()
        },
        Block {
            parent: Some(0),
            style: Style {
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
                height: Length::Px(80),
                ..Style::default()
            },
            ..Block::default()
        },
        Block {
            parent: Some(4),
            style: Style {
                height: Length::Px(25),
                ..Style::default()
            },
            ..Block::default()
        },
        Block {
            parent: Some(4),
            style: Style {
                height: Length::Px(5),
                ..Style::default()
            },
            ..Block::default()
        },
        Block {
            parent: Some(4),
            style: Style {
                height: Length::Px(22),
                ..Style::default()
            },
            ..Block::default()
        },
        Block {
            parent: Some(4),
            style: Style {
                height: Length::Px(5),
                ..Style::default()
            },
            ..Block::default()
        },
        Block {
            parent: Some(4),
            style: Style {
                display: Display::Contents,
                ..Style::default()
            },
            ..Block::default()
        },
        Block {
            parent: Some(10),
            style: Style {
                height: Length::Px(10),
                ..Style::default()
            },
            ..Block::default()
        },
        Block {
            parent: Some(10),
            style: Style {
                height: Length::Px(6),
                ..Style::default()
            },
            ..Block::default()
        },
        Block {
            parent: Some(4),
            style: Style {
                height: Length::Px(5),
                ..Style::default()
            },
            ..Block::default()
        },
        Block {
            parent: Some(4),
            style: Style {
                pointer_events: PointerEvents::None,
                width: Length::Px(30),
                height: Length::Px(26),
                margin: Edges {
                    left: 350,
                    ..Edges::default()
                },
                ..Style::default()
            },
            ..Block::default()
        },
        Block {
            parent: Some(14),
            style: Style {
                width: Length::Px(10),
                height: Length::Px(6),
                ..Style::default()
            },
            ..Block::default()
        },
        Block {
            parent: Some(14),
            style: Style {
                pointer_events: PointerEvents::Auto,
                width: Length::Px(14),
                height: Length::Px(7),
                ..Style::default()
            },
            ..Block::default()
        },
        Block {
            parent: Some(4),
            style: Style {
                width: Length::Px(20),
                height: Length::Px(5),
                margin: Edges {
                    left: 350,
                    ..Edges::default()
                },
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
    let blocked = layout(&blocks, 800, 600).expect("pointer none inheritance layout");
    let expected = [
        (
            "pointer-host",
            14,
            Rect {
                x: 350,
                y: 436,
                width: 30,
                height: 26,
            },
        ),
        (
            "pointer-inherited",
            15,
            Rect {
                x: 350,
                y: 436,
                width: 10,
                height: 6,
            },
        ),
        (
            "pointer-restored",
            16,
            Rect {
                x: 350,
                y: 442,
                width: 14,
                height: 7,
            },
        ),
        (
            "pointer-sibling",
            17,
            Rect {
                x: 350,
                y: 462,
                width: 20,
                height: 5,
            },
        ),
    ];
    for &(id, index, golden) in &expected {
        assert_eq!(
            blocked.boxes[index]
                .expect("pointer geometry remains")
                .border,
            golden,
            "independent native pointer rectangle for {id}"
        );
    }
    assert_eq!(blocked.pointer_events[14..18], [false, false, true, true]);
    assert_eq!((blocked.scroll_width, blocked.scroll_height), (800, 958));
    assert_eq!(
        blocked.boxes[18].unwrap().border,
        Rect {
            x: 0,
            y: 928,
            width: 80,
            height: 30,
        }
    );
    let targets: Vec<_> = blocks
        .iter()
        .enumerate()
        .map(|(index, node)| HitNode {
            parent: node.parent,
            targetable: (14..=17).contains(&index),
        })
        .collect();
    let probes = [
        ("inherited", 351, 438),
        ("host", 375, 439),
        ("restored", 351, 444),
        ("host-lower", 375, 452),
        ("sibling", 351, 464),
        ("outside", 385, 440),
    ];
    let expected_none = [None, None, Some(16), None, Some(17), None];

    blocks[14].style.pointer_events = PointerEvents::Auto;
    let auto = layout(&blocks, 800, 600).expect("pointer auto inheritance layout");
    assert_eq!(blocked.boxes, auto.boxes);
    assert_eq!(auto.pointer_events[14..18], [true, true, true, true]);
    blocks[14].style.pointer_events = PointerEvents::None;
    let restored = layout(&blocks, 800, 600).expect("restored pointer none layout");
    assert_eq!(restored, blocked);

    for (frame, targets_expected) in [
        (&blocked, expected_none),
        (
            &auto,
            [Some(15), Some(14), Some(16), Some(14), Some(17), None],
        ),
        (&restored, expected_none),
    ] {
        for ((name, x, y), expected_target) in probes.iter().copied().zip(targets_expected) {
            assert_eq!(
                hit_test(frame, &targets, x, y),
                Ok(expected_target),
                "independent native pointer hit {name}"
            );
        }
    }

    let receipt = |frame: &Frame| {
        let boxes = expected
            .iter()
            .map(|&(id, index, _)| {
                let rect = frame.boxes[index].unwrap().border;
                format!(
                    r#""{id}":{{"x":{},"y":{},"width":{},"height":{}}}"#,
                    rect.x, rect.y, rect.width, rect.height
                )
            })
            .collect::<Vec<_>>()
            .join(",");
        let computed = expected
            .iter()
            .map(|&(id, index, _)| format!(r#""{id}":{}"#, frame.pointer_events[index]))
            .collect::<Vec<_>>()
            .join(",");
        let hits = probes
            .iter()
            .map(|&(name, x, y)| {
                let id = match hit_test(frame, &targets, x, y).unwrap() {
                    Some(14) => r#""pointer-host""#,
                    Some(15) => r#""pointer-inherited""#,
                    Some(16) => r#""pointer-restored""#,
                    Some(17) => r#""pointer-sibling""#,
                    None => "null",
                    _ => panic!("unexpected independent pointer target"),
                };
                format!(r#""{name}":{id}"#)
            })
            .collect::<Vec<_>>()
            .join(",");
        format!(r#"{{"computed":{{{computed}}},"boxes":{{{boxes}}},"hits":{{{hits}}}}}"#)
    };
    let none = receipt(&blocked);
    let selected = receipt(&auto);
    let restored = receipt(&restored);
    format!(r#"{{"none":{none},"auto":{selected},"restored":{restored}}}"#)
}

// A separate source-independent projection of CSS horizontal auto margins.
// The preceding fixed normal-flow groups end at y467. This isolated native
// tree retains that exact vertical offset without parsing the HTML fixture.
fn auto_margin_states_json() -> String {
    let mut blocks = [
        Block::default(),
        Block::child(0, 467),
        Block {
            parent: Some(0),
            style: Style {
                width: Length::Px(200),
                height: Length::Px(64),
                ..Style::default()
            },
            ..Block::default()
        },
        Block {
            parent: Some(2),
            style: Style {
                width: Length::Px(40),
                height: Length::Px(8),
                horizontal_auto_margins: HorizontalAutoMargins::Both,
                ..Style::default()
            },
            ..Block::default()
        },
        Block {
            parent: Some(2),
            style: Style {
                width: Length::Px(50),
                height: Length::Px(7),
                margin: Edges {
                    right: 10,
                    ..Edges::default()
                },
                horizontal_auto_margins: HorizontalAutoMargins::Left,
                ..Style::default()
            },
            ..Block::default()
        },
        Block {
            parent: Some(2),
            style: Style {
                width: Length::Px(30),
                height: Length::Px(6),
                margin: Edges {
                    left: 12,
                    ..Edges::default()
                },
                horizontal_auto_margins: HorizontalAutoMargins::Right,
                ..Style::default()
            },
            ..Block::default()
        },
        Block {
            parent: Some(2),
            style: Style {
                max_width: Some(WidthBound::Px(80)),
                height: Length::Px(6),
                horizontal_auto_margins: HorizontalAutoMargins::Both,
                ..Style::default()
            },
            ..Block::default()
        },
        Block {
            parent: Some(0),
            style: Style {
                width: Length::Px(20),
                height: Length::Px(5),
                ..Style::default()
            },
            ..Block::default()
        },
    ];
    let original = layout(&blocks, 800, 600).expect("native auto-margin original");
    blocks[3].style.horizontal_auto_margins = HorizontalAutoMargins::Right;
    blocks[3].style.margin.left = 20;
    let shifted = layout(&blocks, 800, 600).expect("native auto-margin left fixed");
    blocks[3].style.margin.left = 0;
    blocks[3].style.horizontal_auto_margins = HorizontalAutoMargins::Both;
    let restored = layout(&blocks, 800, 600).expect("native auto-margin restored");
    assert_eq!(
        restored, original,
        "auto-margin style restoration must be exact"
    );

    let expected = [
        (
            "auto-host",
            2,
            Rect {
                x: 0,
                y: 467,
                width: 200,
                height: 64,
            },
        ),
        (
            "auto-centered",
            3,
            Rect {
                x: 80,
                y: 467,
                width: 40,
                height: 8,
            },
        ),
        (
            "auto-left",
            4,
            Rect {
                x: 140,
                y: 475,
                width: 50,
                height: 7,
            },
        ),
        (
            "auto-right",
            5,
            Rect {
                x: 12,
                y: 482,
                width: 30,
                height: 6,
            },
        ),
        (
            "auto-limited",
            6,
            Rect {
                x: 60,
                y: 488,
                width: 80,
                height: 6,
            },
        ),
        (
            "auto-sibling",
            7,
            Rect {
                x: 0,
                y: 531,
                width: 20,
                height: 5,
            },
        ),
    ];
    for &(id, index, golden) in &expected {
        assert_eq!(
            original.boxes[index]
                .expect("native auto-margin box")
                .border,
            golden,
            "independent native auto-margin rectangle for {id}"
        );
    }
    assert_eq!(
        shifted.boxes[3].expect("shifted center box").border,
        Rect {
            x: 20,
            y: 467,
            width: 40,
            height: 8
        }
    );
    for &(_, index, golden) in expected.iter().filter(|(_, index, _)| *index != 3) {
        assert_eq!(
            shifted.boxes[index]
                .expect("unaffected auto-margin box")
                .border,
            golden
        );
    }
    let probes = [
        ("center-original", 81, 468),
        ("center-shifted", 21, 468),
        ("left-auto", 141, 476),
        ("right-auto", 13, 483),
        ("limited", 61, 489),
        ("host", 195, 525),
        ("sibling", 1, 533),
        ("outside", 221, 468),
    ];
    let hit_nodes = [
        HitNode::new(None),
        HitNode::new(Some(0)),
        HitNode::new(Some(0)),
        HitNode::new(Some(2)),
        HitNode::new(Some(2)),
        HitNode::new(Some(2)),
        HitNode::new(Some(2)),
        HitNode::new(Some(0)),
    ];
    let hit_names = |frame: &Frame| -> Vec<Option<&'static str>> {
        probes
            .iter()
            .map(|&(_, x, y)| {
                match hit_test(frame, &hit_nodes, x, y).expect("native auto-margin hit") {
                    Some(2) => Some("auto-host"),
                    Some(3) => Some("auto-centered"),
                    Some(4) => Some("auto-left"),
                    Some(5) => Some("auto-right"),
                    Some(6) => Some("auto-limited"),
                    Some(7) => Some("auto-sibling"),
                    None | Some(0) | Some(1) => None,
                    Some(other) => panic!("unexpected native auto-margin target {other}"),
                }
            })
            .collect()
    };
    let before_hits = hit_names(&original);
    let shifted_hits = hit_names(&shifted);
    assert_eq!(
        before_hits,
        vec![
            Some("auto-centered"),
            Some("auto-host"),
            Some("auto-left"),
            Some("auto-right"),
            Some("auto-limited"),
            Some("auto-host"),
            Some("auto-sibling"),
            None,
        ]
    );
    assert_eq!(
        shifted_hits,
        vec![
            Some("auto-host"),
            Some("auto-centered"),
            Some("auto-left"),
            Some("auto-right"),
            Some("auto-limited"),
            Some("auto-host"),
            Some("auto-sibling"),
            None,
        ]
    );
    assert_eq!(hit_names(&restored), before_hits);

    let receipt = |frame: &Frame, hits: &[Option<&str>]| {
        let boxes = expected
            .iter()
            .map(|&(id, index, _)| {
                let r = frame.boxes[index].unwrap().border;
                format!(
                    r#""{id}":{{"x":{},"y":{},"width":{},"height":{}}}"#,
                    r.x, r.y, r.width, r.height
                )
            })
            .collect::<Vec<_>>()
            .join(",");
        let computed = expected
            .iter()
            .filter(|(_, index, _)| (3..=6).contains(index))
            .map(|&(id, index, _)| {
                let box_geometry = frame.boxes[index].unwrap();
                let left = box_geometry.border.x - box_geometry.margin.x;
                let right = box_geometry.margin.width - left - box_geometry.border.width;
                format!(r#""{id}":{{"left":"{left}px","right":"{right}px"}}"#)
            })
            .collect::<Vec<_>>()
            .join(",");
        let hits = probes
            .iter()
            .zip(hits)
            .map(|(&(name, _, _), target)| match target {
                Some(id) => format!(r#""{name}":"{id}""#),
                None => format!(r#""{name}":null"#),
            })
            .collect::<Vec<_>>()
            .join(",");
        format!(r#"{{"computed":{{{computed}}},"boxes":{{{boxes}}},"hits":{{{hits}}}}}"#)
    };
    let centered = receipt(&original, &before_hits);
    let left_fixed = receipt(&shifted, &shifted_hits);
    let restored = receipt(&restored, &before_hits);
    format!(r#"{{"centered":{centered},"leftFixed":{left_fixed},"restored":{restored}}}"#)
}

// A caller-resolved normal-flow witness, independent of the CSS fixture.
// All earlier fixed-height groups occupy 536px; relative positioning shifts
// only the visual parent/descendants, not the following sibling's flow y.
fn relative_states_json() -> String {
    let mut blocks = [
        Block::default(),
        Block::child(0, 536),
        Block {
            parent: Some(0),
            style: Style {
                width: Length::Px(60),
                height: Length::Px(32),
                margin: Edges {
                    left: 240,
                    ..Edges::default()
                },
                relative_offset: RelativeOffset { x: 12, y: 9 },
                ..Style::default()
            },
            ..Block::default()
        },
        Block {
            parent: Some(2),
            style: Style {
                width: Length::Px(18),
                height: Length::Px(8),
                relative_offset: RelativeOffset { x: 4, y: 5 },
                ..Style::default()
            },
            ..Block::default()
        },
        Block {
            parent: Some(0),
            style: Style {
                width: Length::Px(24),
                height: Length::Px(6),
                margin: Edges {
                    left: 350,
                    ..Edges::default()
                },
                ..Style::default()
            },
            ..Block::default()
        },
    ];
    let baseline = layout(&blocks, 800, 600).expect("native relative baseline");
    blocks[2].style.relative_offset.x = 20;
    let moved = layout(&blocks, 800, 600).expect("native relative parent moved");
    blocks[2].style.relative_offset.x = 12;
    let restored = layout(&blocks, 800, 600).expect("native relative restored");
    assert_eq!(restored, baseline, "relative position must restore exactly");
    assert_eq!((baseline.scroll_width, baseline.scroll_height), (800, 600));

    let expected = [
        (
            "relative-host",
            2,
            Rect {
                x: 252,
                y: 545,
                width: 60,
                height: 32,
            },
        ),
        (
            "relative-child",
            3,
            Rect {
                x: 256,
                y: 550,
                width: 18,
                height: 8,
            },
        ),
        (
            "relative-sibling",
            4,
            Rect {
                x: 350,
                y: 568,
                width: 24,
                height: 6,
            },
        ),
    ];
    for &(id, index, golden) in &expected {
        assert_eq!(
            baseline.boxes[index].expect("visible relative box").border,
            golden,
            "independent relative CSS geometry for {id}"
        );
    }
    assert_eq!(
        moved.boxes[2].unwrap().border,
        Rect {
            x: 260,
            y: 545,
            width: 60,
            height: 32
        }
    );
    assert_eq!(
        moved.boxes[3].unwrap().border,
        Rect {
            x: 264,
            y: 550,
            width: 18,
            height: 8
        }
    );
    assert_eq!(moved.boxes[4], baseline.boxes[4], "sibling does not reflow");
    assert_eq!((moved.scroll_width, moved.scroll_height), (800, 600));

    let probes = [
        ("original-parent", 253, 547),
        ("moved-parent", 314, 547),
        ("original-child", 257, 552),
        ("moved-child", 280, 552),
        ("host-interior", 294, 565),
        ("sibling", 351, 570),
        ("outside", 325, 548),
    ];
    let nodes = [
        HitNode::new(None),
        HitNode::new(Some(0)),
        HitNode::new(Some(0)),
        HitNode::new(Some(2)),
        HitNode::new(Some(0)),
    ];
    let targets = |frame: &Frame| -> Vec<Option<&'static str>> {
        probes
            .iter()
            .map(
                |&(_, x, y)| match hit_test(frame, &nodes, x, y).expect("native relative hit") {
                    Some(2) => Some("relative-host"),
                    Some(3) => Some("relative-child"),
                    Some(4) => Some("relative-sibling"),
                    None | Some(0) | Some(1) => None,
                    Some(other) => panic!("unexpected native relative hit {other}"),
                },
            )
            .collect()
    };
    let initial_hits = targets(&baseline);
    let moved_hits = targets(&moved);
    assert_eq!(
        initial_hits,
        vec![
            Some("relative-host"),
            None,
            Some("relative-child"),
            Some("relative-host"),
            Some("relative-host"),
            Some("relative-sibling"),
            None,
        ]
    );
    assert_eq!(
        moved_hits,
        vec![
            None,
            Some("relative-host"),
            None,
            Some("relative-child"),
            Some("relative-host"),
            Some("relative-sibling"),
            None,
        ]
    );
    assert_eq!(targets(&restored), initial_hits);

    let receipt = |frame: &Frame, hits: &[Option<&str>], parent_left: u32| {
        let boxes = expected
            .iter()
            .map(|&(id, index, _)| {
                let rect = frame.boxes[index].unwrap().border;
                format!(
                    r#""{id}":{{"x":{},"y":{},"width":{},"height":{}}}"#,
                    rect.x, rect.y, rect.width, rect.height
                )
            })
            .collect::<Vec<_>>()
            .join(",");
        let computed = format!(
            r#""relative-host":{{"position":"relative","left":"{parent_left}px","top":"9px"}},"relative-child":{{"position":"relative","left":"4px","top":"5px"}}"#
        );
        let hits = probes
            .iter()
            .zip(hits)
            .map(|(&(name, _, _), target)| match target {
                Some(id) => format!(r#""{name}":"{id}""#),
                None => format!(r#""{name}":null"#),
            })
            .collect::<Vec<_>>()
            .join(",");
        format!(r#"{{"computed":{{{computed}}},"boxes":{{{boxes}}},"hits":{{{hits}}}}}"#)
    };
    let first = receipt(&baseline, &initial_hits, 12);
    let second = receipt(&moved, &moved_hits, 20);
    let last = receipt(&restored, &initial_hits, 12);
    format!(r#"{{"baseline":{first},"parentMoved":{second},"restored":{last}}}"#)
}

// The original fixed-height spacer's preceding groups occupy 574px.
// These independent source-resolved nodes include a 200px containing width;
// percentages on ALL four margins, including vertical margins, use that width.
// Hit probes use a real +200px document-space scroll translation.
fn percent_states_json() -> String {
    let rect = |x, y, width, height| Rect {
        x,
        y,
        width,
        height,
    };
    let mut blocks = [
        Block {
            style: Style {
                height: Length::Px(958),
                ..Style::default()
            },
            ..Block::default()
        },
        Block::child(0, 574),
        Block {
            parent: Some(0),
            style: Style {
                width: Length::Px(200),
                height: Length::Px(90),
                border: Edges::all(2),
                padding: Edges::all(4),
                ..Style::default()
            },
            ..Block::default()
        },
        Block {
            parent: Some(2),
            style: Style {
                width: Length::Px(150),
                height: Length::Px(8),
                percentage_margins: PercentageMargins {
                    top: Some(10),
                    right: Some(5),
                    bottom: Some(15),
                    left: Some(20),
                },
                ..Style::default()
            },
            ..Block::default()
        },
        Block {
            parent: Some(2),
            style: Style {
                width: Length::Px(24),
                height: Length::Px(6),
                ..Style::default()
            },
            ..Block::default()
        },
        Block {
            parent: Some(0),
            style: Style {
                width: Length::Px(20),
                height: Length::Px(5),
                margin: Edges {
                    left: 250,
                    ..Edges::default()
                },
                ..Style::default()
            },
            ..Block::default()
        },
    ];
    let baseline = layout(&blocks, 800, 600).expect("native percentage margin baseline");
    blocks[3].style.percentage_margins.top = Some(5);
    let changed = layout(&blocks, 800, 600).expect("native percentage margin change");
    blocks[3].style.percentage_margins.top = Some(10);
    let restored = layout(&blocks, 800, 600).expect("native percentage margin restore");
    assert_eq!(restored, baseline, "percentage margin must restore exactly");

    let expected = [
        ("percent-host", 2, rect(0, 574, 212, 102)),
        ("percent-first", 3, rect(46, 600, 150, 8)),
        ("percent-next", 4, rect(6, 638, 24, 6)),
        ("percent-after", 5, rect(250, 676, 20, 5)),
    ];
    for &(id, index, golden) in &expected {
        assert_eq!(
            baseline.boxes[index].unwrap().border,
            golden,
            "{id} baseline"
        );
    }
    assert_eq!(baseline.boxes[3].unwrap().margin, rect(6, 580, 200, 58));
    assert_eq!(baseline.boxes[3].unwrap().content, rect(46, 600, 150, 8));
    assert_eq!(changed.boxes[3].unwrap().border, rect(46, 590, 150, 8));
    assert_eq!(changed.boxes[3].unwrap().margin, rect(6, 580, 200, 48));
    assert_eq!(changed.boxes[4].unwrap().border, rect(6, 628, 24, 6));
    assert_eq!(
        changed.boxes[5], baseline.boxes[5],
        "outer normal flow persists"
    );
    assert_eq!((baseline.scroll_width, baseline.scroll_height), (800, 958));
    assert_eq!((changed.scroll_width, changed.scroll_height), (800, 958));

    let probes = [
        ("baseline-first", 75, 401),
        ("shifted-first", 75, 392),
        ("top-margin", 75, 385),
        ("baseline-next", 7, 440),
        ("shifted-next", 7, 430),
        ("host-bottom", 180, 450),
        ("following-sibling", 251, 478),
        ("outside", 350, 450),
    ];
    let nodes = [
        HitNode::new(None),
        HitNode::new(Some(0)),
        HitNode::new(Some(0)),
        HitNode::new(Some(2)),
        HitNode::new(Some(2)),
        HitNode::new(Some(0)),
    ];
    let hits_for = |frame: &Frame| -> Vec<Option<&'static str>> {
        probes
            .iter()
            .map(|&(_, x, y)| {
                match hit_test_scrolled(frame, &nodes, x, y, 0, 200)
                    .expect("native scrolled percentage margin hit")
                {
                    Some(2) => Some("percent-host"),
                    Some(3) => Some("percent-first"),
                    Some(4) => Some("percent-next"),
                    Some(5) => Some("percent-after"),
                    None | Some(0) | Some(1) => None,
                    Some(other) => panic!("unexpected percentage-margin hit {other}"),
                }
            })
            .collect()
    };
    let before_hits = hits_for(&baseline);
    let changed_hits = hits_for(&changed);
    assert_eq!(
        before_hits,
        vec![
            Some("percent-first"),
            Some("percent-host"),
            Some("percent-host"),
            Some("percent-next"),
            Some("percent-host"),
            Some("percent-host"),
            Some("percent-after"),
            None,
        ]
    );
    assert_eq!(
        changed_hits,
        vec![
            Some("percent-host"),
            Some("percent-first"),
            Some("percent-host"),
            Some("percent-host"),
            Some("percent-next"),
            Some("percent-host"),
            Some("percent-after"),
            None,
        ]
    );
    assert_eq!(hits_for(&restored), before_hits);

    let receipt = |frame: &Frame, hits: &[Option<&str>]| {
        let boxes = expected
            .iter()
            .map(|&(id, index, _)| {
                let r = frame.boxes[index].unwrap().border;
                format!(
                    r#""{id}":{{"x":{},"y":{},"width":{},"height":{}}}"#,
                    r.x, r.y, r.width, r.height
                )
            })
            .collect::<Vec<_>>()
            .join(",");
        let geometry = frame.boxes[3].unwrap();
        let top = geometry.border.y - geometry.margin.y;
        let left = geometry.border.x - geometry.margin.x;
        let right = geometry.margin.width - geometry.border.width - left;
        let bottom = geometry.margin.height - geometry.border.height - top;
        let computed = format!(
            r#""top":"{top}px","right":"{right}px","bottom":"{bottom}px","left":"{left}px""#
        );
        let hits = probes
            .iter()
            .zip(hits)
            .map(|(&(name, _, _), target)| match target {
                Some(id) => format!(r#""{name}":"{id}""#),
                None => format!(r#""{name}":null"#),
            })
            .collect::<Vec<_>>()
            .join(",");
        format!(
            r#"{{"scrollY":200,"computed":{{{computed}}},"boxes":{{{boxes}}},"hits":{{{hits}}}}}"#
        )
    };
    let before = receipt(&baseline, &before_hits);
    let during = receipt(&changed, &changed_hits);
    let after = receipt(&restored, &before_hits);
    format!(r#"{{"original":{before},"changed":{during},"restored":{after}}}"#)
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
    let (width_bounds, height_bounds) = width_and_height_bounds_json();
    output.push_str(&width_bounds);
    output.push_str(r#"},"heightBounds":{"#);
    output.push_str(&height_bounds);
    output.push_str(r#"},"visibilityStates":"#);
    output.push_str(&visibility_states_json());
    output.push_str(r#","descendantClipStates":"#);
    output.push_str(&descendant_clip_states_json());
    output.push_str(r#","boxlessStates":"#);
    output.push_str(&boxless_states_json());
    output.push_str(r#","pointerStates":"#);
    output.push_str(&pointer_states_json());
    output.push_str(r#","autoMarginStates":"#);
    output.push_str(&auto_margin_states_json());
    output.push_str(r#","relativeStates":"#);
    output.push_str(&relative_states_json());
    output.push_str(r#","percentStates":"#);
    output.push_str(&percent_states_json());
    output.push('}');
    println!("{output}");
}
