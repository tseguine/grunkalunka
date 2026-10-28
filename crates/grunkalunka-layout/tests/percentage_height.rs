//! Bounded integral percentage-height layout with definite containing boxes.
//! CSS auto-height fallback, subpixel rounding and general block formatting
//! remain outside this caller-resolved native geometry experiment.
use grunkalunka_layout::{
    Block, Display, Edges, LayoutError, Length, MAX_VIEWPORT, Rect, Style, layout,
};

fn rect(x: u32, y: u32, width: u32, height: u32) -> Rect {
    Rect {
        x,
        y,
        width,
        height,
    }
}

#[test]
fn child_percentages_resolve_against_parent_content_height_not_border_box() {
    let blocks = [
        Block {
            style: Style {
                width: Length::Px(100),
                height: Length::Px(80),
                border: Edges::all(2),
                padding: Edges::all(4),
                ..Style::default()
            },
            ..Block::default()
        },
        Block {
            parent: Some(0),
            style: Style {
                width: Length::Px(30),
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
            parent: Some(0),
            style: Style {
                width: Length::Px(20),
                height: Length::Percent(25),
                ..Style::default()
            },
            ..Block::default()
        },
    ];
    let frame = layout(&blocks, 200, 150).unwrap();
    assert_eq!(frame.boxes[0].unwrap().content, rect(6, 6, 100, 80));
    assert_eq!(frame.boxes[0].unwrap().border, rect(0, 0, 112, 92));
    assert_eq!(frame.boxes[1].unwrap().content, rect(9, 12, 30, 40));
    assert_eq!(frame.boxes[1].unwrap().border, rect(6, 9, 36, 46));
    assert_eq!(frame.boxes[2].unwrap().border, rect(6, 60, 20, 20));
    assert_eq!((frame.scroll_width, frame.scroll_height), (200, 150));
}

#[test]
fn root_uses_viewport_height_and_nested_definite_percentages_chain() {
    let blocks = [
        Block {
            style: Style {
                height: Length::Percent(50),
                padding: Edges::all(6),
                ..Style::default()
            },
            ..Block::default()
        },
        Block {
            parent: Some(0),
            style: Style {
                height: Length::Percent(50),
                ..Style::default()
            },
            ..Block::default()
        },
        Block {
            parent: Some(1),
            style: Style {
                height: Length::Percent(40),
                ..Style::default()
            },
            ..Block::default()
        },
    ];
    let frame = layout(&blocks, 100, 100).unwrap();
    assert_eq!(frame.boxes[0].unwrap().content, rect(6, 6, 88, 50));
    assert_eq!(frame.boxes[0].unwrap().border, rect(0, 0, 100, 62));
    assert_eq!(frame.boxes[1].unwrap().content, rect(6, 6, 88, 25));
    assert_eq!(frame.boxes[2].unwrap().content, rect(6, 6, 88, 10));
}

#[test]
fn overfull_percentage_height_overflows_fixed_parent_and_extends_scroll() {
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
                height: Length::Percent(200),
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
    let frame = layout(&blocks, 50, 30).unwrap();
    assert_eq!(frame.boxes[0].unwrap().border, rect(0, 0, 50, 20));
    assert_eq!(frame.boxes[1].unwrap().border, rect(0, 0, 50, 40));
    assert_eq!(frame.boxes[2].unwrap().border, rect(0, 40, 50, 10));
    assert_eq!((frame.scroll_width, frame.scroll_height), (50, 50));
}

#[test]
fn indefinite_parent_and_fractional_css_pixel_heights_fail_closed() {
    let unavailable = [
        Block::default(),
        Block {
            parent: Some(0),
            style: Style {
                height: Length::Percent(50),
                ..Style::default()
            },
            ..Block::default()
        },
    ];
    assert_eq!(
        layout(&unavailable, 100, 40),
        Err(LayoutError::UnsupportedPercentageHeight { node: 1 })
    );

    let fractional = [
        Block {
            style: Style {
                height: Length::Px(7),
                ..Style::default()
            },
            ..Block::default()
        },
        Block {
            parent: Some(0),
            style: Style {
                height: Length::Percent(50),
                ..Style::default()
            },
            ..Block::default()
        },
    ];
    assert_eq!(
        layout(&fractional, 100, 40),
        Err(LayoutError::FractionalPercentageHeight { node: 1 })
    );
}

#[test]
fn extreme_percentage_height_fails_without_wrapping_and_hidden_nodes_skip_it() {
    let extreme = [Block {
        style: Style {
            height: Length::Percent(100_000_000),
            ..Style::default()
        },
        ..Block::default()
    }];
    assert_eq!(
        layout(&extreme, 100, MAX_VIEWPORT),
        Err(LayoutError::Overflow)
    );

    let blocks = [
        Block::default(),
        Block {
            parent: Some(0),
            style: Style {
                display: Display::None,
                height: Length::Percent(u32::MAX),
                ..Style::default()
            },
            ..Block::default()
        },
        Block {
            parent: Some(1),
            style: Style {
                height: Length::Percent(50),
                ..Style::default()
            },
            ..Block::default()
        },
        Block {
            parent: Some(0),
            style: Style {
                height: Length::Px(11),
                ..Style::default()
            },
            ..Block::default()
        },
    ];
    let frame = layout(&blocks, 100, 40).unwrap();
    assert_eq!(frame.boxes[1], None);
    assert_eq!(frame.boxes[2], None);
    assert_eq!(frame.boxes[3].unwrap().content, rect(0, 0, 100, 11));
}
