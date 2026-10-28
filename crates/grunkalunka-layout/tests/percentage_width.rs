//! Independent exact geometry for selected integer percentage widths.
//! Browser-level CSS parsing, box-sizing and subpixel rounding remain external.
use grunkalunka_layout::{
    Block, Display, Edges, LayoutError, Length, MAX_VIEWPORT, Rect, Style, layout,
};

fn rect(x: u32, y: u32, width: u32, height: u32) -> Rect {
    Rect { x, y, width, height }
}

#[test]
fn nested_percent_width_uses_containing_content_width_not_border_width() {
    let blocks = [
        Block {
            style: Style {
                width: Length::Px(210),
                padding: Edges::all(12),
                border: Edges::all(3),
                ..Style::default()
            },
            ..Block::default()
        },
        Block {
            parent: Some(0),
            style: Style {
                width: Length::Percent(50),
                height: Length::Px(20),
                margin: Edges::all(4),
                padding: Edges::all(5),
                border: Edges::all(2),
                ..Style::default()
            },
            ..Block::default()
        },
        Block {
            parent: Some(1),
            style: Style {
                width: Length::Percent(40),
                height: Length::Px(5),
                ..Style::default()
            },
            ..Block::default()
        },
        Block {
            parent: Some(0),
            style: Style {
                height: Length::Px(10),
                ..Style::default()
            },
            ..Block::default()
        },
    ];
    let frame = layout(&blocks, 800, 600).unwrap();
    let root = frame.boxes[0].unwrap();
    let first = frame.boxes[1].unwrap();
    let nested = frame.boxes[2].unwrap();
    let sibling = frame.boxes[3].unwrap();

    assert_eq!(root.content, rect(15, 15, 210, 52));
    assert_eq!(root.border, rect(0, 0, 240, 82));
    assert_eq!(first.border, rect(19, 19, 119, 34));
    assert_eq!(first.content, rect(26, 26, 105, 20));
    assert_eq!(nested.border, rect(26, 26, 42, 5));
    assert_eq!(sibling.content, rect(15, 57, 210, 10));
    assert_eq!((frame.scroll_width, frame.scroll_height), (800, 600));
}

#[test]
fn percentages_can_overflow_containing_width_without_clamping() {
    let blocks = [
        Block {
            style: Style {
                width: Length::Percent(200),
                height: Length::Px(1),
                ..Style::default()
            },
            ..Block::default()
        },
    ];
    let frame = layout(&blocks, 100, 20).unwrap();
    assert_eq!(frame.boxes[0].unwrap().content.width, 200);
    assert_eq!(frame.scroll_width, 200);

    let mut zero = blocks;
    zero[0].style.width = Length::Percent(0);
    let frame = layout(&zero, 100, 20).unwrap();
    assert_eq!(frame.boxes[0].unwrap().content.width, 0);
}

#[test]
fn fractional_widths_and_unsupported_percentage_heights_fail_closed() {
    let fractional = [Block {
        style: Style {
            width: Length::Percent(50),
            ..Style::default()
        },
        ..Block::default()
    }];
    assert_eq!(
        layout(&fractional, 7, 20),
        Err(LayoutError::FractionalPercentageWidth { node: 0 })
    );

    let unsupported = [Block {
        style: Style {
            height: Length::Percent(50),
            ..Style::default()
        },
        ..Block::default()
    }];
    assert_eq!(
        layout(&unsupported, 100, 20),
        Err(LayoutError::UnsupportedPercentageHeight { node: 0 })
    );
}

#[test]
fn oversized_percentage_geometry_overflows_instead_of_wrapping() {
    let blocks = [Block {
        style: Style {
            width: Length::Percent(u32::MAX),
            ..Style::default()
        },
        ..Block::default()
    }];
    assert_eq!(layout(&blocks, MAX_VIEWPORT, 20), Err(LayoutError::Overflow));
}

#[test]
fn display_none_skips_unsupported_descendants_and_keeps_sibling_flow() {
    let blocks = [
        Block::default(),
        Block {
            parent: Some(0),
            style: Style {
                display: Display::None,
                height: Length::Percent(50),
                width: Length::Percent(u32::MAX),
                ..Style::default()
            },
            ..Block::default()
        },
        Block::child(0, 11),
    ];
    let frame = layout(&blocks, 100, 40).unwrap();
    assert_eq!(frame.boxes[1], None);
    assert_eq!(frame.boxes[2].unwrap().border, rect(0, 0, 100, 11));
}
