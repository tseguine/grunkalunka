//! Exact percentage-padding geometry resolved against containing content width.
//! Independent integer-pixel goldens, not parsed CSS or browser-derived output.
use grunkalunka_layout::hit_test::{HitNode, hit_test, hit_test_scrolled};
use grunkalunka_layout::paint::{PaintNode, Rgb, paint, paint_scrolled};
use grunkalunka_layout::{
    Block, BoxSizing, DescendantClip, Display, Edges, LayoutError, Length, MAX_VIEWPORT,
    PercentagePadding, Rect, RelativeOffset, Style, WidthBound, layout,
};

const GREY: Rgb = Rgb::new(180, 180, 180);
const RED: Rgb = Rgb::new(235, 30, 30);
const BLUE: Rgb = Rgb::new(30, 60, 225);
const GREEN: Rgb = Rgb::new(25, 180, 50);

fn rect(x: u32, y: u32, width: u32, height: u32) -> Rect {
    Rect {
        x,
        y,
        width,
        height,
    }
}

#[test]
fn all_four_padding_percentages_use_containing_content_width() {
    assert_eq!(
        Style::default().percentage_padding,
        PercentagePadding::default()
    );
    let blocks = [
        Block {
            style: Style {
                width: Length::Px(200),
                height: Length::Px(90),
                border: Edges::all(2),
                padding: Edges::all(3),
                ..Style::default()
            },
            ..Block::default()
        },
        Block {
            parent: Some(0),
            style: Style {
                width: Length::Px(60),
                height: Length::Px(12),
                margin: Edges {
                    left: 6,
                    right: 2,
                    top: 3,
                    bottom: 4,
                },
                border: Edges::all(1),
                padding: Edges {
                    top: 2,
                    right: 5,
                    bottom: 7,
                    left: 4,
                },
                percentage_padding: PercentagePadding {
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
                width: Length::Percent(50),
                height: Length::Px(4),
                percentage_padding: PercentagePadding {
                    top: Some(10),
                    left: Some(20),
                    ..PercentagePadding::default()
                },
                ..Style::default()
            },
            ..Block::default()
        },
        Block::child(0, 8),
    ];
    let frame = layout(&blocks, 280, 180).unwrap();
    assert_eq!(frame.boxes[0].unwrap().border, rect(0, 0, 210, 100));
    assert_eq!(frame.boxes[0].unwrap().content, rect(5, 5, 200, 90));
    let child = frame.boxes[1].unwrap();
    assert_eq!(child.margin, rect(5, 5, 120, 71));
    assert_eq!(child.border, rect(11, 8, 112, 64));
    assert_eq!(child.padding, rect(12, 9, 110, 62));
    assert_eq!(child.content, rect(52, 29, 60, 12));
    // The grandchild's top padding uses 10% of 60px CONTENT WIDTH,
    // not its parent's 12px height or the original root 200px width.
    assert_eq!(frame.boxes[2].unwrap().border, rect(52, 29, 42, 10));
    assert_eq!(frame.boxes[2].unwrap().content, rect(64, 35, 30, 4));
    assert_eq!(frame.boxes[3].unwrap().border, rect(5, 76, 200, 8));
    assert_eq!((frame.scroll_width, frame.scroll_height), (280, 180));
    assert_eq!(layout(&blocks, 280, 180), Ok(frame.clone()));

    let hits = [
        HitNode::new(None),
        HitNode::new(Some(0)),
        HitNode::new(Some(1)),
        HitNode::new(Some(0)),
    ];
    let paints = [
        PaintNode {
            parent: None,
            background: Some(GREY),
            border: None,
        },
        PaintNode {
            parent: Some(0),
            background: Some(RED),
            border: None,
        },
        PaintNode {
            parent: Some(1),
            background: Some(BLUE),
            border: None,
        },
        PaintNode {
            parent: Some(0),
            background: Some(GREEN),
            border: None,
        },
    ];
    let painted = paint(&frame, &paints, Rgb::WHITE).unwrap();
    for ((x, y), color, target) in [
        ((10, 15), GREY, Some(0)),
        ((20, 15), RED, Some(1)),
        ((52, 35), BLUE, Some(2)),
        ((5, 76), GREEN, Some(3)),
    ] {
        assert_eq!(painted.pixel(x, y), Some(color));
        assert_eq!(hit_test(&frame, &hits, x, y), Ok(target));
    }
}

#[test]
fn percent_padding_participates_in_auto_width_and_border_box_limits() {
    let mut blocks = [
        Block {
            style: Style {
                width: Length::Px(120),
                height: Length::Px(40),
                ..Style::default()
            },
            ..Block::default()
        },
        Block {
            parent: Some(0),
            style: Style {
                height: Length::Px(6),
                max_width: Some(WidthBound::Px(70)),
                margin: Edges {
                    left: 4,
                    right: 6,
                    ..Edges::default()
                },
                border: Edges::all(1),
                padding: Edges::all(1),
                percentage_padding: PercentagePadding {
                    top: Some(5),
                    right: Some(15),
                    bottom: Some(10),
                    left: Some(10),
                },
                relative_offset: RelativeOffset { x: 5, y: 3 },
                ..Style::default()
            },
            ..Block::default()
        },
        Block::child(0, 4),
    ];
    let auto = layout(&blocks, 180, 60).unwrap();
    assert_eq!(auto.boxes[1].unwrap().margin, rect(5, 3, 112, 26));
    assert_eq!(auto.boxes[1].unwrap().border, rect(9, 3, 102, 26));
    assert_eq!(auto.boxes[1].unwrap().content, rect(22, 10, 70, 6));
    assert_eq!(auto.boxes[2].unwrap().border, rect(0, 26, 120, 4));

    blocks[1].style.box_sizing = BoxSizing::BorderBox;
    blocks[1].style.width = Length::Px(70);
    blocks[1].style.height = Length::Px(36);
    blocks[1].style.max_width = None;
    let border = layout(&blocks, 180, 60).unwrap();
    assert_eq!(border.boxes[1].unwrap().border, rect(9, 3, 70, 36));
    assert_eq!(border.boxes[1].unwrap().content, rect(22, 10, 38, 16));
    assert_eq!(border.boxes[2].unwrap().border, rect(0, 36, 120, 4));
}

#[test]
fn hidden_and_boxless_wrappers_ignore_their_percentage_padding() {
    let blocks = [
        Block {
            style: Style {
                width: Length::Px(100),
                height: Length::Px(40),
                ..Style::default()
            },
            ..Block::default()
        },
        Block {
            parent: Some(0),
            style: Style {
                display: Display::Contents,
                padding: Edges::all(u32::MAX),
                percentage_padding: PercentagePadding {
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
                width: Length::Px(20),
                height: Length::Px(5),
                percentage_padding: PercentagePadding {
                    top: Some(10),
                    right: Some(10),
                    bottom: Some(10),
                    left: Some(10),
                },
                ..Style::default()
            },
            ..Block::default()
        },
        Block {
            parent: Some(0),
            style: Style {
                display: Display::None,
                percentage_padding: PercentagePadding {
                    left: Some(u32::MAX),
                    ..PercentagePadding::default()
                },
                ..Style::default()
            },
            ..Block::default()
        },
        Block::child(3, 999),
        Block::child(0, 4),
    ];
    let frame = layout(&blocks, 200, 50).unwrap();
    assert_eq!(frame.boxes[1], None);
    assert!(frame.contents[1]);
    assert_eq!(frame.boxes[2].unwrap().border, rect(0, 0, 40, 25));
    assert_eq!(frame.boxes[2].unwrap().content, rect(10, 10, 20, 5));
    assert_eq!(frame.boxes[3], None);
    assert_eq!(frame.boxes[4], None);
    assert_eq!(frame.boxes[5].unwrap().border, rect(0, 25, 100, 4));
}

#[test]
fn fractional_and_oversized_padding_percentages_fail_closed() {
    let mut blocks = [Block {
        style: Style {
            percentage_padding: PercentagePadding {
                top: Some(50),
                ..PercentagePadding::default()
            },
            ..Style::default()
        },
        ..Block::default()
    }];
    assert_eq!(
        layout(&blocks, 101, 30),
        Err(LayoutError::FractionalPercentagePadding { node: 0 })
    );
    blocks[0].style.percentage_padding.top = None;
    blocks[0].style.percentage_padding.left = Some(50);
    assert_eq!(
        layout(&blocks, 101, 30),
        Err(LayoutError::FractionalPercentagePadding { node: 0 })
    );

    // Exactly integral but too large for u32, distinct from fractional.
    blocks[0].style.percentage_padding.left = Some(100_000_000);
    assert_eq!(
        layout(&blocks, MAX_VIEWPORT, 30),
        Err(LayoutError::Overflow)
    );
    blocks[0].style.percentage_padding.left = Some(u32::MAX);
    assert_eq!(
        layout(&blocks, MAX_VIEWPORT, 30),
        Err(LayoutError::FractionalPercentagePadding { node: 0 })
    );

    blocks[0].style.percentage_padding.left = Some(200);
    blocks[0].style.width = Length::Px(10);
    blocks[0].style.height = Length::Px(4);
    let overflow = layout(&blocks, 100, 30).unwrap();
    assert_eq!(overflow.boxes[0].unwrap().border, rect(0, 0, 210, 4));
    assert_eq!(overflow.boxes[0].unwrap().content, rect(200, 0, 10, 4));
    assert_eq!(overflow.scroll_width, 210);
}

#[test]
fn percentage_padding_clip_uses_new_padding_box_and_preserves_scroll_translation() {
    let blocks = [
        Block {
            style: Style {
                width: Length::Px(50),
                height: Length::Px(20),
                border: Edges::all(2),
                descendant_clip: DescendantClip::PaddingBox,
                percentage_padding: PercentagePadding {
                    top: Some(10),
                    right: Some(10),
                    bottom: Some(10),
                    left: Some(10),
                },
                ..Style::default()
            },
            ..Block::default()
        },
        Block {
            parent: Some(0),
            style: Style {
                width: Length::Px(80),
                height: Length::Px(14),
                relative_offset: RelativeOffset { x: 10, y: 0 },
                ..Style::default()
            },
            ..Block::default()
        },
    ];
    let frame = layout(&blocks, 100, 60).unwrap();
    assert_eq!(frame.boxes[0].unwrap().border, rect(0, 0, 74, 44));
    assert_eq!(frame.boxes[0].unwrap().padding, rect(2, 2, 70, 40));
    assert_eq!(frame.boxes[0].unwrap().content, rect(12, 12, 50, 20));
    assert_eq!(frame.boxes[1].unwrap().border, rect(22, 12, 80, 14));
    assert_eq!((frame.scroll_width, frame.scroll_height), (102, 60));
    let hits = [HitNode::new(None), HitNode::new(Some(0))];
    let paints = [
        PaintNode {
            parent: None,
            background: Some(GREY),
            border: Some(Rgb::BLACK),
        },
        PaintNode {
            parent: Some(0),
            background: Some(BLUE),
            border: None,
        },
    ];
    let clipped = paint(&frame, &paints, Rgb::WHITE).unwrap();
    assert_eq!(clipped.pixel(71, 14), Some(BLUE));
    assert_eq!(clipped.pixel(72, 14), Some(Rgb::BLACK));
    assert_eq!(hit_test(&frame, &hits, 71, 14), Ok(Some(1)));
    assert_eq!(hit_test(&frame, &hits, 72, 14), Ok(Some(0)));

    let scrolled = paint_scrolled(&frame, &paints, Rgb::WHITE, 2, 0).unwrap();
    assert_eq!(scrolled.pixel(69, 14), Some(BLUE));
    assert_eq!(scrolled.pixel(70, 14), Some(Rgb::BLACK));
    assert_eq!(hit_test_scrolled(&frame, &hits, 69, 14, 2, 0), Ok(Some(1)));
    assert_eq!(hit_test_scrolled(&frame, &hits, 70, 14, 2, 0), Ok(Some(0)));
}
