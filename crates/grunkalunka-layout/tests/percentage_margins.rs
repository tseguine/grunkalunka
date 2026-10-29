//! Exact containing-content-width percentage margins in normal-flow boxes.
//! These are manually selected integer CSS-pixel goldens, not browser captures.
use grunkalunka_layout::hit_test::{HitNode, hit_test};
use grunkalunka_layout::paint::{PaintNode, Rgb, paint};
use grunkalunka_layout::{
    Block, Display, Edges, HorizontalAutoMargins, LayoutError, Length, MAX_VIEWPORT,
    PercentageMargins, Rect, RelativeOffset, Style, WidthBound, layout,
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
fn all_four_percent_margins_use_parent_width_even_for_vertical_edges() {
    assert_eq!(
        Style::default().percentage_margins,
        PercentageMargins::default()
    );
    let blocks = [
        Block {
            style: Style {
                width: Length::Px(200),
                height: Length::Px(100),
                border: Edges::all(2),
                padding: Edges::all(3),
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
            parent: Some(1),
            style: Style {
                width: Length::Px(10),
                height: Length::Px(3),
                percentage_margins: PercentageMargins {
                    top: Some(25),
                    ..PercentageMargins::default()
                },
                ..Style::default()
            },
            ..Block::default()
        },
        Block {
            parent: Some(0),
            style: Style {
                width: Length::Px(20),
                height: Length::Px(4),
                ..Style::default()
            },
            ..Block::default()
        },
    ];
    let frame = layout(&blocks, 260, 120).unwrap();
    assert_eq!(frame.boxes[0].unwrap().border, rect(0, 0, 210, 110));
    assert_eq!(frame.boxes[0].unwrap().content, rect(5, 5, 200, 100));
    assert_eq!(frame.boxes[1].unwrap().margin, rect(5, 5, 96, 66));
    assert_eq!(frame.boxes[1].unwrap().border, rect(45, 25, 46, 16));
    assert_eq!(frame.boxes[1].unwrap().content, rect(48, 28, 40, 10));
    // 25% of child's 40px content WIDTH, not its 10px content height.
    assert_eq!(frame.boxes[2].unwrap().border, rect(48, 38, 10, 3));
    assert_eq!(frame.boxes[3].unwrap().border, rect(5, 71, 20, 4));
    assert_eq!((frame.scroll_width, frame.scroll_height), (260, 120));
    assert_eq!(layout(&blocks, 260, 120), Ok(frame.clone()));

    let hits = [
        HitNode::new(None),
        HitNode::new(Some(0)),
        HitNode::new(Some(1)),
        HitNode::new(Some(0)),
    ];
    let paints = [
        PaintNode {
            parent: None,
            background: Some(Rgb::new(190, 190, 190)),
            border: None,
        },
        PaintNode {
            parent: Some(0),
            background: Some(Rgb::new(235, 30, 30)),
            border: None,
        },
        PaintNode {
            parent: Some(1),
            background: Some(Rgb::new(30, 50, 230)),
            border: None,
        },
        PaintNode {
            parent: Some(0),
            background: Some(Rgb::new(30, 170, 40)),
            border: None,
        },
    ];
    let canvas = paint(&frame, &paints, Rgb::WHITE).unwrap();
    assert_eq!(hit_test(&frame, &hits, 20, 26), Ok(Some(0)));
    assert_eq!(hit_test(&frame, &hits, 47, 26), Ok(Some(1)));
    assert_eq!(hit_test(&frame, &hits, 50, 39), Ok(Some(2)));
    assert_eq!(hit_test(&frame, &hits, 6, 72), Ok(Some(3)));
    assert_eq!(canvas.pixel(20, 26), Some(Rgb::new(190, 190, 190)));
    assert_eq!(canvas.pixel(50, 39), Some(Rgb::new(30, 50, 230)));
    assert_eq!(canvas.pixel(6, 72), Some(Rgb::new(30, 170, 40)));
}

#[test]
fn auto_width_uses_resolved_margin_percentages_before_max_width() {
    let blocks = [
        Block {
            style: Style {
                width: Length::Px(120),
                height: Length::Px(30),
                ..Style::default()
            },
            ..Block::default()
        },
        Block {
            parent: Some(0),
            style: Style {
                max_width: Some(WidthBound::Px(70)),
                height: Length::Px(6),
                percentage_margins: PercentageMargins {
                    left: Some(10),
                    right: Some(15),
                    ..PercentageMargins::default()
                },
                relative_offset: RelativeOffset { x: 5, y: 3 },
                ..Style::default()
            },
            ..Block::default()
        },
        Block::child(0, 4),
    ];
    let frame = layout(&blocks, 180, 40).unwrap();
    assert_eq!(frame.boxes[1].unwrap().margin, rect(5, 3, 100, 6));
    assert_eq!(frame.boxes[1].unwrap().border, rect(17, 3, 70, 6));
    assert_eq!(frame.boxes[1].unwrap().content, rect(17, 3, 70, 6));
    assert_eq!(frame.boxes[2].unwrap().border, rect(0, 6, 120, 4));
    assert_eq!((frame.scroll_width, frame.scroll_height), (180, 40));
}

#[test]
fn horizontal_auto_overrides_percent_and_fixed_margins_without_evaluating_them() {
    let blocks = [
        Block {
            style: Style {
                width: Length::Px(200),
                height: Length::Px(40),
                ..Style::default()
            },
            ..Block::default()
        },
        Block {
            parent: Some(0),
            style: Style {
                width: Length::Px(80),
                height: Length::Px(5),
                margin: Edges {
                    left: u32::MAX,
                    right: 7,
                    top: 3,
                    ..Edges::default()
                },
                percentage_margins: PercentageMargins {
                    left: Some(u32::MAX),
                    right: Some(10),
                    top: Some(5),
                    bottom: Some(10),
                },
                horizontal_auto_margins: HorizontalAutoMargins::Left,
                ..Style::default()
            },
            ..Block::default()
        },
        Block::child(0, 4),
    ];
    let frame = layout(&blocks, 240, 50).unwrap();
    assert_eq!(frame.boxes[1].unwrap().margin, rect(0, 0, 200, 35));
    assert_eq!(frame.boxes[1].unwrap().border, rect(100, 10, 80, 5));
    assert_eq!(frame.boxes[2].unwrap().border, rect(0, 35, 200, 4));
    assert_eq!((frame.scroll_width, frame.scroll_height), (240, 50));
}

#[test]
fn boxless_and_hidden_percentages_are_ignored_but_child_uses_containing_width() {
    let blocks = [
        Block {
            style: Style {
                width: Length::Px(100),
                height: Length::Px(30),
                ..Style::default()
            },
            ..Block::default()
        },
        Block {
            parent: Some(0),
            style: Style {
                display: Display::Contents,
                percentage_margins: PercentageMargins {
                    top: Some(u32::MAX),
                    right: Some(u32::MAX),
                    bottom: Some(u32::MAX),
                    left: Some(u32::MAX),
                },
                ..Style::default()
            },
            ..Block::default()
        },
        Block {
            parent: Some(1),
            style: Style {
                width: Length::Px(30),
                height: Length::Px(5),
                percentage_margins: PercentageMargins {
                    top: Some(10),
                    left: Some(10),
                    ..PercentageMargins::default()
                },
                ..Style::default()
            },
            ..Block::default()
        },
        Block {
            parent: Some(0),
            style: Style {
                display: Display::None,
                percentage_margins: PercentageMargins {
                    top: Some(u32::MAX),
                    ..PercentageMargins::default()
                },
                ..Style::default()
            },
            ..Block::default()
        },
        Block::child(3, 900),
        Block {
            parent: Some(0),
            style: Style {
                width: Length::Px(12),
                height: Length::Px(3),
                ..Style::default()
            },
            ..Block::default()
        },
    ];
    let frame = layout(&blocks, 200, 50).unwrap();
    assert_eq!(frame.boxes[1], None);
    assert!(frame.contents[1]);
    assert_eq!(frame.boxes[2].unwrap().border, rect(10, 10, 30, 5));
    assert_eq!(frame.boxes[3], None);
    assert_eq!(frame.boxes[4], None);
    assert_eq!(frame.boxes[5].unwrap().border, rect(0, 15, 12, 3));
}

#[test]
fn fractional_and_oversized_percentage_margins_fail_without_wraparound() {
    let mut blocks = [Block {
        style: Style {
            percentage_margins: PercentageMargins {
                top: Some(50),
                ..PercentageMargins::default()
            },
            ..Style::default()
        },
        ..Block::default()
    }];
    assert_eq!(
        layout(&blocks, 101, 30),
        Err(LayoutError::FractionalPercentageMargin { node: 0 })
    );
    blocks[0].style.percentage_margins.top = None;
    blocks[0].style.percentage_margins.left = Some(50);
    assert_eq!(
        layout(&blocks, 101, 30),
        Err(LayoutError::FractionalPercentageMargin { node: 0 })
    );

    // Exact but too large for u32 geometry: reject overflow, not wrap.
    blocks[0].style.percentage_margins.left = Some(100_000_000);
    assert_eq!(
        layout(&blocks, MAX_VIEWPORT, 30),
        Err(LayoutError::Overflow)
    );
    // A different oversized declaration is non-integral, which is also
    // rejected before the engine can truncate the percentage result.
    blocks[0].style.percentage_margins.left = Some(u32::MAX);
    assert_eq!(
        layout(&blocks, MAX_VIEWPORT, 30),
        Err(LayoutError::FractionalPercentageMargin { node: 0 })
    );

    blocks[0].style.percentage_margins.left = Some(200);
    blocks[0].style.width = Length::Px(10);
    blocks[0].style.height = Length::Px(4);
    let overflowing = layout(&blocks, 100, 30).unwrap();
    assert_eq!(overflowing.boxes[0].unwrap().border, rect(200, 0, 10, 4));
    assert_eq!(overflowing.scroll_width, 210);
}
