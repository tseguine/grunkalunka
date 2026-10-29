//! Explicit optional width constraints in an integer-only headless block model.
//! All goldens are independent literal rectangles, not browser captures.
//! Percentage limits reference the containing CONTENT width before box sizing.
use grunkalunka_layout::{
    Block, BoxSizing, Display, Edges, LayoutError, Length, MAX_VIEWPORT, Rect, Style, WidthBound,
    layout,
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
fn default_width_bounds_preserve_auto_layout_and_nested_percentages() {
    assert_eq!(Style::default().min_width, None);
    assert_eq!(Style::default().max_width, None);
    let blocks = [
        Block {
            style: Style {
                margin: Edges {
                    left: 3,
                    right: 3,
                    ..Edges::default()
                },
                border: Edges::all(2),
                padding: Edges::all(5),
                height: Length::Px(12),
                ..Style::default()
            },
            ..Block::default()
        },
        Block {
            parent: Some(0),
            style: Style {
                width: Length::Percent(50),
                height: Length::Px(3),
                ..Style::default()
            },
            ..Block::default()
        },
    ];
    let before = layout(&blocks, 100, 50).unwrap();
    assert_eq!(before.boxes[0].unwrap().content, rect(10, 7, 80, 12));
    assert_eq!(before.boxes[1].unwrap().border, rect(10, 7, 40, 3));
}

#[test]
fn minimum_width_expands_auto_box_and_descendant_containing_width() {
    let blocks = [
        Block {
            style: Style {
                min_width: Some(WidthBound::Px(110)),
                margin: Edges {
                    left: 3,
                    right: 4,
                    ..Edges::default()
                },
                border: Edges::all(2),
                padding: Edges::all(5),
                height: Length::Px(12),
                ..Style::default()
            },
            ..Block::default()
        },
        Block {
            parent: Some(0),
            style: Style {
                width: Length::Percent(50),
                height: Length::Px(3),
                ..Style::default()
            },
            ..Block::default()
        },
    ];
    let frame = layout(&blocks, 100, 50).unwrap();
    assert_eq!(frame.boxes[0].unwrap().margin, rect(0, 0, 131, 26));
    assert_eq!(frame.boxes[0].unwrap().border, rect(3, 0, 124, 26));
    assert_eq!(frame.boxes[0].unwrap().content, rect(10, 7, 110, 12));
    assert_eq!(frame.boxes[1].unwrap().border, rect(10, 7, 55, 3));
    assert_eq!((frame.scroll_width, frame.scroll_height), (131, 50));
}

#[test]
fn maximum_percentage_constrains_fixed_box_before_descendant_layout() {
    let blocks = [
        Block {
            style: Style {
                width: Length::Px(200),
                border: Edges::all(2),
                padding: Edges::all(3),
                ..Style::default()
            },
            ..Block::default()
        },
        Block {
            parent: Some(0),
            style: Style {
                width: Length::Px(185),
                max_width: Some(WidthBound::Percent(60)),
                height: Length::Px(10),
                border: Edges::all(1),
                padding: Edges::all(2),
                ..Style::default()
            },
            ..Block::default()
        },
        Block {
            parent: Some(1),
            style: Style {
                width: Length::Percent(50),
                height: Length::Px(5),
                ..Style::default()
            },
            ..Block::default()
        },
    ];
    let frame = layout(&blocks, 250, 60).unwrap();
    assert_eq!(frame.boxes[0].unwrap().content.width, 200);
    assert_eq!(frame.boxes[1].unwrap().border, rect(5, 5, 126, 16));
    assert_eq!(frame.boxes[1].unwrap().content, rect(8, 8, 120, 10));
    assert_eq!(frame.boxes[2].unwrap().content, rect(8, 8, 60, 5));
}

#[test]
fn contradictory_maximum_never_overrides_explicit_minimum() {
    let blocks = [Block {
        style: Style {
            width: Length::Px(90),
            min_width: Some(WidthBound::Px(140)),
            max_width: Some(WidthBound::Px(100)),
            height: Length::Px(5),
            ..Style::default()
        },
        ..Block::default()
    }];
    let frame = layout(&blocks, 120, 20).unwrap();
    assert_eq!(frame.boxes[0].unwrap().content, rect(0, 0, 140, 5));
    assert_eq!(frame.scroll_width, 140);
}

#[test]
fn border_box_limits_compare_outer_lengths_excluding_margins() {
    let blocks = [
        Block {
            style: Style {
                box_sizing: BoxSizing::BorderBox,
                width: Length::Px(120),
                min_width: Some(WidthBound::Px(110)),
                max_width: Some(WidthBound::Px(90)),
                height: Length::Px(20),
                border: Edges::all(2),
                padding: Edges::all(4),
                ..Style::default()
            },
            ..Block::default()
        },
        Block {
            parent: Some(0),
            style: Style {
                width: Length::Percent(50),
                height: Length::Px(4),
                ..Style::default()
            },
            ..Block::default()
        },
    ];
    let frame = layout(&blocks, 200, 60).unwrap();
    assert_eq!(frame.boxes[0].unwrap().border, rect(0, 0, 110, 20));
    assert_eq!(frame.boxes[0].unwrap().content, rect(6, 6, 98, 8));
    assert_eq!(frame.boxes[1].unwrap().content, rect(6, 6, 49, 4));

    let small = [Block {
        style: Style {
            box_sizing: BoxSizing::BorderBox,
            width: Length::Px(200),
            max_width: Some(WidthBound::Px(4)),
            border: Edges::all(3),
            padding: Edges::all(4),
            ..Style::default()
        },
        ..Block::default()
    }];
    let frame = layout(&small, 100, 40).unwrap();
    assert_eq!(frame.boxes[0].unwrap().content.width, 0);
    assert_eq!(frame.boxes[0].unwrap().border.width, 14);
}

#[test]
fn percentage_minimum_wins_smaller_percentage_maximum() {
    let blocks = [
        Block {
            style: Style {
                width: Length::Px(200),
                ..Style::default()
            },
            ..Block::default()
        },
        Block {
            parent: Some(0),
            style: Style {
                width: Length::Px(170),
                min_width: Some(WidthBound::Percent(75)),
                max_width: Some(WidthBound::Percent(60)),
                height: Length::Px(1),
                ..Style::default()
            },
            ..Block::default()
        },
    ];
    let frame = layout(&blocks, 250, 50).unwrap();
    assert_eq!(frame.boxes[1].unwrap().content.width, 150);
}

#[test]
fn fractional_and_oversized_percentage_bounds_fail_closed_even_if_not_selected() {
    let fractional = [Block {
        style: Style {
            width: Length::Px(2),
            max_width: Some(WidthBound::Percent(50)),
            ..Style::default()
        },
        ..Block::default()
    }];
    assert_eq!(
        layout(&fractional, 7, 20),
        Err(LayoutError::FractionalPercentageWidth { node: 0 })
    );

    let fractional_min = [Block {
        style: Style {
            width: Length::Px(2),
            min_width: Some(WidthBound::Percent(50)),
            ..Style::default()
        },
        ..Block::default()
    }];
    assert_eq!(
        layout(&fractional_min, 7, 20),
        Err(LayoutError::FractionalPercentageWidth { node: 0 })
    );

    let oversized = [Block {
        style: Style {
            max_width: Some(WidthBound::Percent(100_000_000)),
            ..Style::default()
        },
        ..Block::default()
    }];
    assert_eq!(
        layout(&oversized, MAX_VIEWPORT, 30),
        Err(LayoutError::Overflow)
    );
}

#[test]
fn display_none_skips_invalid_bounds_in_hidden_subtree_and_reflows_sibling() {
    let blocks = [
        Block::default(),
        Block {
            parent: Some(0),
            style: Style {
                display: Display::None,
                min_width: Some(WidthBound::Percent(50)),
                max_width: Some(WidthBound::Percent(u32::MAX)),
                ..Style::default()
            },
            ..Block::default()
        },
        Block {
            parent: Some(1),
            style: Style {
                min_width: Some(WidthBound::Percent(50)),
                ..Style::default()
            },
            ..Block::default()
        },
        Block {
            parent: Some(0),
            style: Style {
                width: Length::Px(3),
                height: Length::Px(7),
                ..Style::default()
            },
            ..Block::default()
        },
    ];
    let frame = layout(&blocks, 7, 20).unwrap();
    assert_eq!(frame.boxes[1], None);
    assert_eq!(frame.boxes[2], None);
    assert_eq!(frame.boxes[3].unwrap().border, rect(0, 0, 3, 7));
}
