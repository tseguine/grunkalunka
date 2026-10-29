//! Exact nonnegative horizontal auto-margin geometry in the headless block model.
//! Goldens are independently chosen CSS-pixel rectangles, not browser receipts.

use grunkalunka_layout::hit_test::{HitNode, hit_test};
use grunkalunka_layout::{
    Block, BoxSizing, Display, Edges, HorizontalAutoMargins, LayoutError, Length, Rect, Style,
    WidthBound, layout,
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
fn centered_root_and_nested_block_preserve_containing_width_and_hit_targets() {
    assert_eq!(
        Style::default().horizontal_auto_margins,
        HorizontalAutoMargins::None
    );
    let blocks = [
        Block {
            style: Style {
                width: Length::Px(200),
                height: Length::Px(40),
                border: Edges::all(2),
                padding: Edges::all(3),
                horizontal_auto_margins: HorizontalAutoMargins::Both,
                ..Style::default()
            },
            ..Block::default()
        },
        Block {
            parent: Some(0),
            style: Style {
                width: Length::Px(40),
                height: Length::Px(10),
                border: Edges::all(1),
                padding: Edges::all(2),
                horizontal_auto_margins: HorizontalAutoMargins::Both,
                ..Style::default()
            },
            ..Block::default()
        },
        Block {
            parent: Some(1),
            style: Style {
                width: Length::Percent(50),
                height: Length::Px(3),
                ..Style::default()
            },
            ..Block::default()
        },
    ];
    let frame = layout(&blocks, 260, 60).unwrap();
    assert_eq!(frame.boxes[0].unwrap().margin, rect(0, 0, 260, 50));
    assert_eq!(frame.boxes[0].unwrap().border, rect(25, 0, 210, 50));
    assert_eq!(frame.boxes[0].unwrap().content, rect(30, 5, 200, 40));
    assert_eq!(frame.boxes[1].unwrap().margin, rect(30, 5, 200, 16));
    assert_eq!(frame.boxes[1].unwrap().border, rect(107, 5, 46, 16));
    assert_eq!(frame.boxes[1].unwrap().content, rect(110, 8, 40, 10));
    assert_eq!(frame.boxes[2].unwrap().border, rect(110, 8, 20, 3));
    assert_eq!((frame.scroll_width, frame.scroll_height), (260, 60));
    assert_eq!(layout(&blocks, 260, 60), Ok(frame.clone()));

    let hits = [
        HitNode::new(None),
        HitNode::new(Some(0)),
        HitNode::new(Some(1)),
    ];
    assert_eq!(hit_test(&frame, &hits, 20, 6), Ok(None));
    assert_eq!(hit_test(&frame, &hits, 80, 6), Ok(Some(0)));
    assert_eq!(hit_test(&frame, &hits, 107, 5), Ok(Some(1)));
    assert_eq!(hit_test(&frame, &hits, 110, 8), Ok(Some(2)));
}

#[test]
fn left_and_right_auto_override_numeric_edges_and_auto_width_fills_then_clamps() {
    let blocks = [
        Block {
            style: Style {
                width: Length::Px(160),
                height: Length::Px(30),
                ..Style::default()
            },
            ..Block::default()
        },
        Block {
            parent: Some(0),
            style: Style {
                width: Length::Px(40),
                height: Length::Px(4),
                horizontal_auto_margins: HorizontalAutoMargins::Left,
                margin: Edges {
                    left: u32::MAX, // ignored in favor of auto
                    right: 10,
                    top: 2,
                    ..Edges::default()
                },
                ..Style::default()
            },
            ..Block::default()
        },
        Block {
            parent: Some(0),
            style: Style {
                width: Length::Px(40),
                height: Length::Px(4),
                horizontal_auto_margins: HorizontalAutoMargins::Right,
                margin: Edges {
                    left: 12,
                    right: u32::MAX, // ignored in favor of auto
                    ..Edges::default()
                },
                ..Style::default()
            },
            ..Block::default()
        },
        Block {
            parent: Some(0),
            style: Style {
                height: Length::Px(4),
                horizontal_auto_margins: HorizontalAutoMargins::Both,
                ..Style::default()
            },
            ..Block::default()
        },
        Block {
            parent: Some(0),
            style: Style {
                max_width: Some(WidthBound::Px(100)),
                height: Length::Px(4),
                horizontal_auto_margins: HorizontalAutoMargins::Both,
                ..Style::default()
            },
            ..Block::default()
        },
    ];
    let frame = layout(&blocks, 170, 40).unwrap();
    assert_eq!(frame.boxes[1].unwrap().border, rect(110, 2, 40, 4));
    assert_eq!(frame.boxes[2].unwrap().border, rect(12, 6, 40, 4));
    assert_eq!(frame.boxes[3].unwrap().border, rect(0, 10, 160, 4));
    assert_eq!(frame.boxes[4].unwrap().border, rect(30, 14, 100, 4));
    assert_eq!(frame.boxes[1].unwrap().margin.width, 160);
    assert_eq!(frame.boxes[2].unwrap().margin.width, 160);
    assert_eq!(frame.boxes[3].unwrap().margin.width, 160);
    assert_eq!(frame.boxes[4].unwrap().margin.width, 160);
    assert_eq!((frame.scroll_width, frame.scroll_height), (170, 40));
}

#[test]
fn border_box_percentages_and_conflicting_bounds_are_resolved_before_centering() {
    let mut blocks = [
        Block {
            style: Style {
                width: Length::Px(200),
                height: Length::Px(30),
                border: Edges::all(1),
                padding: Edges::all(2),
                ..Style::default()
            },
            ..Block::default()
        },
        Block {
            parent: Some(0),
            style: Style {
                box_sizing: BoxSizing::BorderBox,
                width: Length::Percent(80),
                min_width: Some(WidthBound::Px(110)),
                max_width: Some(WidthBound::Percent(60)),
                height: Length::Px(20),
                border: Edges::all(1),
                padding: Edges::all(2),
                horizontal_auto_margins: HorizontalAutoMargins::Both,
                ..Style::default()
            },
            ..Block::default()
        },
        Block {
            parent: Some(1),
            style: Style {
                width: Length::Percent(50),
                height: Length::Px(2),
                ..Style::default()
            },
            ..Block::default()
        },
    ];
    let frame = layout(&blocks, 220, 50).unwrap();
    assert_eq!(frame.boxes[1].unwrap().border, rect(43, 3, 120, 20));
    assert_eq!(frame.boxes[1].unwrap().content, rect(46, 6, 114, 14));
    assert_eq!(frame.boxes[2].unwrap().border, rect(46, 6, 57, 2));

    // The explicit minimum wins a smaller maximum, changing the two
    // auto margins only after the final used border-box width is known.
    blocks[1].style.min_width = Some(WidthBound::Percent(70));
    let raised = layout(&blocks, 220, 50).unwrap();
    assert_eq!(raised.boxes[1].unwrap().border, rect(33, 3, 140, 20));
    assert_eq!(raised.boxes[1].unwrap().content, rect(36, 6, 134, 14));
    assert_eq!(raised.boxes[2].unwrap().border, rect(36, 6, 67, 2));
}

#[test]
fn overconstrained_auto_margins_become_zero_and_overflow_remains_visible() {
    let mut blocks = [Block {
        style: Style {
            width: Length::Px(130),
            height: Length::Px(3),
            horizontal_auto_margins: HorizontalAutoMargins::Both,
            ..Style::default()
        },
        ..Block::default()
    }];
    let frame = layout(&blocks, 100, 20).unwrap();
    assert_eq!(frame.boxes[0].unwrap().margin, rect(0, 0, 130, 3));
    assert_eq!(frame.boxes[0].unwrap().border, rect(0, 0, 130, 3));
    assert_eq!(frame.scroll_width, 130);

    blocks[0].style.horizontal_auto_margins = HorizontalAutoMargins::Left;
    blocks[0].style.margin.right = 20;
    let with_fixed_right = layout(&blocks, 100, 20).unwrap();
    assert_eq!(with_fixed_right.boxes[0].unwrap().border.x, 0);
    assert_eq!(with_fixed_right.boxes[0].unwrap().margin.width, 150);
    assert_eq!(with_fixed_right.scroll_width, 150);

    blocks[0].style.horizontal_auto_margins = HorizontalAutoMargins::Both;
    blocks[0].style.margin.right = 0;
    blocks[0].style.width = Length::Auto;
    blocks[0].style.border = Edges::all(60);
    let edges_overflow = layout(&blocks, 100, 20).unwrap();
    assert_eq!(edges_overflow.boxes[0].unwrap().content.width, 0);
    assert_eq!(
        edges_overflow.boxes[0].unwrap().border,
        rect(0, 0, 120, 123)
    );
    assert_eq!(edges_overflow.scroll_width, 120);
}

#[test]
fn fractional_centering_fails_closed_and_hidden_or_boxless_wrappers_ignore_auto() {
    let odd = [Block {
        style: Style {
            width: Length::Px(30),
            horizontal_auto_margins: HorizontalAutoMargins::Both,
            ..Style::default()
        },
        ..Block::default()
    }];
    assert_eq!(
        layout(&odd, 101, 30),
        Err(LayoutError::FractionalAutoMargin { node: 0 })
    );

    let blocks = [
        Block {
            style: Style {
                width: Length::Px(120),
                height: Length::Px(25),
                padding: Edges::all(2),
                ..Style::default()
            },
            ..Block::default()
        },
        Block {
            parent: Some(0),
            style: Style {
                display: Display::Contents,
                horizontal_auto_margins: HorizontalAutoMargins::Both,
                width: Length::Px(u32::MAX),
                margin: Edges::all(u32::MAX),
                ..Style::default()
            },
            ..Block::default()
        },
        Block {
            parent: Some(1),
            style: Style {
                width: Length::Px(40),
                height: Length::Px(6),
                horizontal_auto_margins: HorizontalAutoMargins::Both,
                ..Style::default()
            },
            ..Block::default()
        },
        Block {
            parent: Some(0),
            style: Style {
                display: Display::None,
                horizontal_auto_margins: HorizontalAutoMargins::Both,
                width: Length::Percent(50),
                ..Style::default()
            },
            ..Block::default()
        },
        Block {
            parent: Some(0),
            style: Style {
                width: Length::Px(20),
                height: Length::Px(4),
                margin: Edges {
                    left: 5,
                    ..Edges::default()
                },
                ..Style::default()
            },
            ..Block::default()
        },
    ];
    let frame = layout(&blocks, 150, 40).unwrap();
    assert_eq!(frame.boxes[1], None);
    assert!(frame.contents[1]);
    assert_eq!(frame.boxes[2].unwrap().border, rect(42, 2, 40, 6));
    assert_eq!(frame.boxes[3], None);
    assert_eq!(frame.boxes[4].unwrap().border, rect(7, 8, 20, 4));
    let hits = [
        HitNode::new(None),
        HitNode::new(Some(0)),
        HitNode::new(Some(1)),
        HitNode::new(Some(0)),
        HitNode::new(Some(0)),
    ];
    assert_eq!(hit_test(&frame, &hits, 42, 2), Ok(Some(2)));
}
