//! Independent resolved input for the fixed-row/alignment browser witness. No HTML/CSS
//! parser or browser receipt is used to construct these native boxes or hits.

use grunkalunka_layout::hit_test::{HitNode, hit_test_scrolled};
use grunkalunka_layout::paint::{PaintNode, Rgb, paint_scrolled};
use grunkalunka_layout::{
    Block, DescendantClip, Display, Edges, FlexAlign, FlexJustify, FlexWrap, Frame, Length, Rect,
    RelativeOffset, Style, layout,
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
const PHASES: [&str; 30] = [
    "nowrap",
    "wrap",
    "wide",
    "columnGap",
    "rowGap",
    "hidden",
    "relative",
    "clipped",
    "scrolled",
    "justifyEnd",
    "justifyCenter",
    "spaceBetween",
    "alignEnd",
    "alignCenter",
    "wrapAlign",
    "wrapSingle",
    "safeOverflow",
    "overlapClipped",
    "nestedOverlap",
    "nestedOverflow",
    "relativeAlign",
    "clippedAlign",
    "scrolledAlign",
    "selfAlignMixed",
    "selfAlignAuto",
    "selfAlignWrap",
    "selfAlignOverflow",
    "selfAlignClipped",
    "selfAlignScrolled",
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
    if matches!(
        phase,
        "wrap"
            | "wide"
            | "columnGap"
            | "rowGap"
            | "hidden"
            | "relative"
            | "clipped"
            | "scrolled"
            | "wrapAlign"
            | "wrapSingle"
            | "selfAlignWrap"
    ) {
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
        "justifyEnd" | "justifyCenter" => {
            blocks[1].style.width = Length::Px(220);
            blocks[1].style.justify_content = if phase == "justifyEnd" {
                FlexJustify::SafeEnd
            } else {
                FlexJustify::SafeCenter
            };
        }
        "spaceBetween" => {
            blocks[1].style.width = Length::Px(230);
            blocks[1].style.justify_content = FlexJustify::SpaceBetween;
        }
        "alignEnd" | "alignCenter" | "wrapSingle" | "scrolledAlign" => {
            blocks[1].style.width = Length::Px(220);
            blocks[1].style.height = Length::Px(60);
            blocks[1].style.align_items = if phase == "alignEnd" || phase == "wrapSingle" {
                FlexAlign::SafeEnd
            } else {
                FlexAlign::SafeCenter
            };
        }
        "wrapAlign" => {
            blocks[1].style.justify_content = FlexJustify::SpaceBetween;
            blocks[1].style.align_items = FlexAlign::SafeCenter;
        }
        "safeOverflow" | "overlapClipped" | "nestedOverflow" => {
            blocks[1].style.height = Length::Px(20);
            blocks[1].style.justify_content = FlexJustify::SafeEnd;
            blocks[1].style.align_items = FlexAlign::SafeEnd;
            if phase == "overlapClipped" {
                blocks[1].style.descendant_clip = DescendantClip::PaddingBox;
            } else if phase == "nestedOverflow" {
                blocks[5].style.height = Length::Px(40);
            }
        }
        "nestedOverlap" => blocks[4].style.margin.top = 0,
        "relativeAlign" | "clippedAlign" => {
            blocks[1].style.width = Length::Px(220);
            blocks[1].style.justify_content = FlexJustify::SafeCenter;
            if phase == "relativeAlign" {
                blocks[1].style.height = Length::Px(60);
                blocks[1].style.align_items = FlexAlign::SafeCenter;
                blocks[4].style.relative_offset = RelativeOffset { x: 8, y: 9 };
            } else {
                blocks[1].style.width = Length::Px(216);
                blocks[1].style.height = Length::Px(28);
                blocks[1].style.align_items = FlexAlign::SafeEnd;
                blocks[1].style.descendant_clip = DescendantClip::PaddingBox;
            }
        }
        "selfAlignMixed" | "selfAlignAuto" => {
            blocks[1].style.width = Length::Px(220);
            blocks[1].style.height = Length::Px(60);
            blocks[1].style.align_items = FlexAlign::SafeCenter;
            blocks[2].style.align_self = Some(FlexAlign::Start);
            // The boxless wrapper and ordinary child do not apply this field.
            blocks[3].style.align_self = Some(FlexAlign::SafeEnd);
            blocks[3].style.align_items = FlexAlign::Start;
            blocks[5].style.align_self = Some(FlexAlign::SafeEnd);
            blocks[4].style.align_self = Some(FlexAlign::SafeEnd);
            if phase == "selfAlignAuto" {
                blocks[4].style.align_self = None;
            }
        }
        "selfAlignWrap" => {
            blocks[1].style.width = Length::Px(220);
            blocks[1].style.height = Length::Px(60);
            blocks[1].style.align_items = FlexAlign::SafeCenter;
            blocks[2].style.align_self = Some(FlexAlign::SafeEnd);
            blocks[3].style.align_self = Some(FlexAlign::SafeEnd);
            blocks[6].style.align_self = Some(FlexAlign::Start);
        }
        "selfAlignOverflow" => {
            blocks[1].style.width = Length::Px(220);
            blocks[1].style.height = Length::Px(20);
            blocks[2].style.align_self = Some(FlexAlign::SafeCenter);
            blocks[4].style.align_self = Some(FlexAlign::SafeEnd);
            blocks[6].style.align_self = Some(FlexAlign::SafeCenter);
        }
        "selfAlignClipped" | "selfAlignScrolled" => {
            blocks[1].style.width = Length::Px(220);
            blocks[1].style.height = Length::Px(36);
            blocks[1].style.descendant_clip = DescendantClip::PaddingBox;
            blocks[2].style.align_self = Some(FlexAlign::SafeEnd);
            blocks[4].style.align_self = Some(FlexAlign::SafeCenter);
            blocks[4].style.relative_offset.y = 15;
            blocks[6].style.align_self = Some(FlexAlign::SafeCenter);
        }
        "nowrap" | "wrap" | "scrolled" | "restored" => {}
        _ => panic!("unknown native flex phase"),
    }
    if matches!(phase, "nestedOverlap" | "nestedOverflow") {
        blocks[4].style.display = Display::FlexRow;
        blocks[5].style.width = Length::Px(100);
    }
    (
        blocks,
        if matches!(phase, "scrolled" | "scrolledAlign" | "selfAlignScrolled") {
            10
        } else {
            0
        },
    )
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
// At width 220, 30 pixels remain after the existing gaps. A nowrap content
// height of 60 leaves cross-axis spaces of 36, 18 and 42. Wrapped lines keep
// their intrinsic 42/18 heights even when the host has an explicit height.
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
        "justifyEnd" => {
            expected[0] = rect(20, 20, 232, 54);
            expected[1] = rect(56, 26, 64, 24);
            expected[2] = rect(135, 28, 56, 36);
            expected[3] = rect(138, 31, 25, 8);
            expected[4] = rect(204, 26, 42, 18);
        }
        "justifyCenter" => {
            expected[0] = rect(20, 20, 232, 54);
            expected[1] = rect(41, 26, 64, 24);
            expected[2] = rect(120, 28, 56, 36);
            expected[3] = rect(123, 31, 25, 8);
            expected[4] = rect(189, 26, 42, 18);
        }
        "spaceBetween" => {
            expected[0] = rect(20, 20, 242, 54);
            expected[2] = rect(125, 28, 56, 36);
            expected[3] = rect(128, 31, 25, 8);
            expected[4] = rect(214, 26, 42, 18);
        }
        "alignEnd" => {
            expected[0] = rect(20, 20, 232, 72);
            expected[1] = rect(26, 62, 64, 24);
            expected[2] = rect(105, 46, 56, 36);
            expected[3] = rect(108, 49, 25, 8);
            expected[4] = rect(174, 68, 42, 18);
            expected[5] = rect(0, 98, 180, 12);
        }
        "alignCenter" | "scrolledAlign" => {
            expected[0] = rect(20, 20, 232, 72);
            expected[1] = rect(26, 44, 64, 24);
            expected[2] = rect(105, 37, 56, 36);
            expected[3] = rect(108, 40, 25, 8);
            expected[4] = rect(174, 47, 42, 18);
            expected[5] = rect(0, 98, 180, 12);
        }
        "wrapAlign" => {
            expected[0] = rect(20, 20, 162, 80);
            expected[1] = rect(26, 35, 64, 24);
            expected[2] = rect(117, 28, 56, 36);
            expected[3] = rect(120, 31, 25, 8);
            expected[4] = rect(26, 76, 42, 18);
            expected[5] = rect(0, 106, 180, 12);
        }
        "wrapSingle" => {
            expected[0] = rect(20, 20, 232, 72);
            expected[1] = rect(26, 44, 64, 24);
            expected[4] = rect(174, 50, 42, 18);
            expected[5] = rect(0, 98, 180, 12);
        }
        "safeOverflow" | "overlapClipped" | "nestedOverflow" => {
            expected[0] = rect(20, 20, 162, 32);
            expected[4] = rect(174, 28, 42, 18);
            expected[5] = rect(0, 58, 180, 12);
            if phase == "nestedOverflow" {
                expected[3] = rect(108, 31, 100, 40);
            }
        }
        "nestedOverlap" => {
            expected[0] = rect(20, 20, 162, 52);
            expected[2] = rect(105, 26, 56, 36);
            expected[3] = rect(108, 29, 100, 8);
            expected[5] = rect(0, 78, 180, 12);
        }
        "relativeAlign" => {
            expected[0] = rect(20, 20, 232, 72);
            expected[1] = rect(41, 44, 64, 24);
            expected[2] = rect(128, 46, 56, 36);
            expected[3] = rect(131, 49, 25, 8);
            expected[4] = rect(189, 47, 42, 18);
            expected[5] = rect(0, 98, 180, 12);
        }
        "clippedAlign" => {
            expected[0] = rect(20, 20, 228, 40);
            expected[1] = rect(39, 30, 64, 24);
            expected[2] = rect(118, 28, 56, 36);
            expected[3] = rect(121, 31, 25, 8);
            expected[4] = rect(187, 36, 42, 18);
            expected[5] = rect(0, 66, 180, 12);
        }
        "selfAlignMixed" | "selfAlignAuto" => {
            expected[0] = rect(20, 20, 232, 72);
            expected[2] = rect(105, 46, 56, 36);
            expected[3] = rect(108, 49, 25, 8);
            expected[4] = rect(174, 47, 42, 18);
            expected[5] = rect(0, 98, 180, 12);
            if phase == "selfAlignAuto" {
                expected[2] = rect(105, 37, 56, 36);
                expected[3] = rect(108, 40, 25, 8);
            }
        }
        "selfAlignWrap" => {
            expected[0] = rect(20, 20, 232, 72);
            expected[1] = rect(26, 44, 64, 24);
            expected[5] = rect(0, 98, 180, 12);
        }
        "selfAlignOverflow" => {
            expected[0] = rect(20, 20, 232, 32);
            expected[4] = rect(174, 27, 42, 18);
            expected[5] = rect(0, 58, 180, 12);
        }
        "selfAlignClipped" | "selfAlignScrolled" => {
            expected[0] = rect(20, 20, 232, 48);
            expected[1] = rect(26, 38, 64, 24);
            expected[2] = rect(105, 43, 56, 36);
            expected[3] = rect(108, 46, 25, 8);
            expected[4] = rect(174, 35, 42, 18);
            expected[5] = rect(0, 74, 180, 12);
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
        "justifyEnd" => {
            expected = [
                Some("row-host"),
                Some("row-a"),
                Some("row-b"),
                Some("row-after"),
                Some("row-host"),
                None,
                Some("row-host"),
                Some("row-a"),
                Some("row-host"),
            ]
        }
        "justifyCenter" => {
            expected = [
                Some("row-host"),
                Some("row-host"),
                Some("row-host"),
                Some("row-after"),
                Some("row-host"),
                None,
                Some("row-host"),
                Some("row-a"),
                Some("row-host"),
            ]
        }
        "spaceBetween" => {
            expected = [
                Some("row-a"),
                Some("row-host"),
                Some("row-b"),
                Some("row-after"),
                Some("row-host"),
                None,
                Some("row-host"),
                Some("row-host"),
                Some("row-host"),
            ]
        }
        "alignEnd" => {
            expected = [
                Some("row-host"),
                Some("row-host"),
                Some("row-host"),
                Some("row-a"),
                Some("row-b"),
                None,
                Some("row-host"),
                Some("row-host"),
                Some("row-host"),
            ]
        }
        "alignCenter" => {
            expected = [
                Some("row-host"),
                Some("row-host"),
                Some("row-host"),
                Some("row-host"),
                Some("row-b"),
                None,
                Some("row-host"),
                Some("row-host"),
                Some("row-host"),
            ]
        }
        "wrapAlign" => {
            expected = [
                Some("row-host"),
                Some("row-host"),
                Some("row-host"),
                Some("row-c"),
                Some("row-b"),
                Some("row-after"),
                Some("row-host"),
                Some("row-host"),
                None,
            ]
        }
        "wrapSingle" | "scrolledAlign" => {
            expected = [
                Some("row-host"),
                Some("row-inner"),
                Some("row-host"),
                Some("row-host"),
                Some("row-b"),
                None,
                Some("row-host"),
                Some("row-host"),
                Some("row-host"),
            ]
        }
        "safeOverflow" | "overlapClipped" | "nestedOverflow" => {
            expected = [
                Some("row-a"),
                Some("row-inner"),
                Some("row-c"),
                None,
                Some("row-b"),
                None,
                Some("row-host"),
                Some("row-host"),
                None,
            ];
            if phase == "overlapClipped" {
                expected[2] = Some("row-host");
                expected[4] = Some("row-after");
            } else if phase == "nestedOverflow" {
                expected[4] = Some("row-inner");
            }
        }
        "nestedOverlap" => expected[4] = Some("row-host"),
        "relativeAlign" => {
            expected = [
                Some("row-host"),
                Some("row-host"),
                Some("row-host"),
                Some("row-host"),
                Some("row-host"),
                None,
                Some("row-host"),
                Some("row-host"),
                Some("row-host"),
            ]
        }
        "clippedAlign" => {
            expected = [
                Some("row-host"),
                Some("row-host"),
                Some("row-host"),
                None,
                None,
                None,
                Some("row-host"),
                Some("row-a"),
                None,
            ]
        }
        "selfAlignMixed" | "selfAlignAuto" => {
            expected = [
                Some("row-a"),
                Some("row-host"),
                Some("row-host"),
                Some("row-host"),
                Some("row-b"),
                None,
                Some("row-host"),
                Some("row-host"),
                Some("row-host"),
            ]
        }
        "selfAlignWrap" => {
            expected[0] = Some("row-host");
            expected[3] = Some("row-host");
            expected[8] = Some("row-host");
        }
        "selfAlignOverflow" => {
            expected[3] = None;
            expected[8] = Some("row-host");
        }
        "selfAlignClipped" => {
            expected[0] = Some("row-host");
            expected[1] = Some("row-host");
            expected[2] = Some("row-host");
            expected[8] = Some("row-host");
        }
        "selfAlignScrolled" => {
            expected[1] = Some("row-host");
            expected[3] = None;
            expected[4] = None;
            expected[8] = Some("row-host");
        }
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

fn assert_native_pixels(frame: &Frame, blocks: &[Block; 8], phase: &str, scroll_y: u32) {
    const BLUE: Rgb = Rgb::new(0, 0, 255);
    const GREEN: Rgb = Rgb::new(0, 255, 0);
    const PURPLE: Rgb = Rgb::new(128, 0, 128);
    const ORANGE: Rgb = Rgb::new(255, 128, 0);
    let (x, y, expected) = match phase {
        "safeOverflow" => (118, 62, BLUE),
        "overlapClipped" => (118, 62, ORANGE),
        "nestedOverlap" => (180, 30, PURPLE),
        "nestedOverflow" => (118, 62, GREEN),
        "selfAlignMixed" => (110, 52, GREEN),
        "selfAlignAuto" => (110, 52, BLUE),
        "selfAlignWrap" => (180, 30, PURPLE),
        "selfAlignOverflow" | "selfAlignClipped" => (118, 62, BLUE),
        "selfAlignScrolled" => (118, 62, Rgb::WHITE),
        _ => return,
    };
    let paints: Vec<_> = blocks
        .iter()
        .enumerate()
        .map(|(index, block)| PaintNode {
            parent: block.parent,
            background: match index {
                1 => Some(Rgb::WHITE),
                2 => Some(Rgb::new(255, 0, 0)),
                4 => Some(BLUE),
                5 => Some(GREEN),
                6 => Some(PURPLE),
                7 => Some(ORANGE),
                _ => None,
            },
            border: Some(Rgb::BLACK),
        })
        .collect();
    let canvas =
        paint_scrolled(frame, &paints, Rgb::WHITE, 0, scroll_y).expect("native flex-order paint");
    assert_eq!(canvas.pixel(x, y), Some(expected), "{phase}: native pixel");
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
    assert_native_pixels(&frame, &blocks, phase, scroll_y);
    (
        frame,
        format!(r#""{phase}":{{"scrollY":{scroll_y},"boxes":{{{boxes}}},"hits":{{{hits}}}}}"#),
    )
}

fn main() {
    let mut baseline = None;
    let mut aligned = None;
    let mut self_clipped = None;
    let states = PHASES
        .into_iter()
        .map(|phase| {
            let (frame, state) = receipt(phase);
            if phase == "nowrap" {
                baseline = Some(frame);
            } else if phase == "restored" {
                assert_eq!(baseline.as_ref(), Some(&frame), "full frame restoration");
            } else if phase == "alignCenter" {
                aligned = Some(frame);
            } else if phase == "scrolledAlign" {
                assert_eq!(
                    aligned.as_ref(),
                    Some(&frame),
                    "aligned scroll preserves full frame"
                );
            } else if phase == "selfAlignClipped" {
                self_clipped = Some(frame);
            } else if phase == "selfAlignScrolled" {
                assert_eq!(
                    self_clipped.as_ref(),
                    Some(&frame),
                    "item-aligned scroll preserves full frame"
                );
            }
            state
        })
        .collect::<Vec<_>>()
        .join(",");
    println!(
        r#"{{"schemaVersion":1,"kind":"grunkalunka-native-flex-geometry","fixtureId":"native-flex-bridge-v1","viewport":{{"width":800,"height":600,"devicePixelRatio":1}},"states":{{{states}}}}}"#
    );
}
