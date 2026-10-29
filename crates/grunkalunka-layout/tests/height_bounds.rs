//! Caller-resolved optional minimum and maximum heights in bounded block flow.
//! Native integer geometry only: no CSS fallback, parser, or browser oracle.
use grunkalunka_layout::{
    Block, BoxSizing, Display, Edges, HeightBound, LayoutError, Length, MAX_VIEWPORT, Rect, Style,
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
fn default_height_bounds_preserve_existing_auto_and_fixed_layout() {
    assert_eq!(Style::default().min_height, None);
    assert_eq!(Style::default().max_height, None);
    let blocks = [
        Block::default(),
        Block {
            parent: Some(0),
            style: Style {
                height: Length::Px(8),
                ..Style::default()
            },
            ..Block::default()
        },
        Block::child(0, 6),
    ];
    let frame = layout(&blocks, 60, 40).unwrap();
    assert_eq!(frame.boxes[0].unwrap().content, rect(0, 0, 60, 14));
    assert_eq!(frame.boxes[2].unwrap().content, rect(0, 8, 60, 6));
}

#[test]
fn minimum_expands_fixed_height_before_descendant_percent_resolves() {
    let blocks = [
        Block {
            style: Style {
                height: Length::Px(20),
                min_height: Some(HeightBound::Px(80)),
                border: Edges::all(2),
                padding: Edges::all(3),
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
    let frame = layout(&blocks, 100, 100).unwrap();
    assert_eq!(frame.boxes[0].unwrap().border, rect(0, 0, 100, 90));
    assert_eq!(frame.boxes[0].unwrap().content, rect(5, 5, 90, 80));
    assert_eq!(frame.boxes[1].unwrap().content, rect(5, 5, 90, 40));
}

#[test]
fn maximum_limits_fixed_height_but_overflowing_children_remain_visible() {
    let blocks = [
        Block {
            style: Style {
                height: Length::Px(100),
                max_height: Some(HeightBound::Px(30)),
                ..Style::default()
            },
            ..Block::default()
        },
        Block {
            parent: Some(0),
            style: Style {
                height: Length::Px(60),
                ..Style::default()
            },
            ..Block::default()
        },
    ];
    let frame = layout(&blocks, 100, 40).unwrap();
    assert_eq!(frame.boxes[0].unwrap().border, rect(0, 0, 100, 30));
    assert_eq!(frame.boxes[1].unwrap().border, rect(0, 0, 100, 60));
    assert_eq!((frame.scroll_width, frame.scroll_height), (100, 60));
}

#[test]
fn auto_height_bounds_apply_after_children_and_minimum_wins() {
    let blocks = [
        Block {
            style: Style {
                min_height: Some(HeightBound::Px(60)),
                max_height: Some(HeightBound::Px(40)),
                ..Style::default()
            },
            ..Block::default()
        },
        Block {
            parent: Some(0),
            style: Style {
                height: Length::Px(30),
                ..Style::default()
            },
            ..Block::default()
        },
        Block {
            parent: Some(0),
            style: Style {
                height: Length::Px(15),
                ..Style::default()
            },
            ..Block::default()
        },
    ];
    let frame = layout(&blocks, 100, 70).unwrap();
    assert_eq!(frame.boxes[0].unwrap().content, rect(0, 0, 100, 60));
    assert_eq!(frame.boxes[2].unwrap().content, rect(0, 30, 100, 15));

    let mut capped = blocks;
    capped[0].style.min_height = None;
    capped[0].style.max_height = Some(HeightBound::Px(20));
    let frame = layout(&capped, 100, 30).unwrap();
    assert_eq!(frame.boxes[0].unwrap().content.height, 20);
    assert_eq!(frame.boxes[2].unwrap().border.y, 30);
    assert_eq!(frame.scroll_height, 45, "visible children still contribute scroll extents");
}

#[test]
fn percentage_limits_use_definite_parent_content_height() {
    let blocks = [
        Block {
            style: Style {
                height: Length::Px(200),
                border: Edges::all(2),
                padding: Edges::all(5),
                ..Style::default()
            },
            ..Block::default()
        },
        Block {
            parent: Some(0),
            style: Style {
                height: Length::Px(180),
                min_height: Some(HeightBound::Percent(75)),
                max_height: Some(HeightBound::Percent(60)),
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
                height: Length::Px(10),
                ..Style::default()
            },
            ..Block::default()
        },
    ];
    let frame = layout(&blocks, 100, 250).unwrap();
    assert_eq!(frame.boxes[0].unwrap().content, rect(7, 7, 86, 200));
    assert_eq!(frame.boxes[1].unwrap().content, rect(7, 7, 86, 150));
    assert_eq!(frame.boxes[2].unwrap().content, rect(7, 7, 86, 75));
    assert_eq!(frame.boxes[3].unwrap().content, rect(7, 157, 86, 10));
}

#[test]
fn border_box_height_limits_include_border_and_padding_edges() {
    let blocks = [
        Block {
            style: Style {
                box_sizing: BoxSizing::BorderBox,
                height: Length::Px(120),
                min_height: Some(HeightBound::Px(110)),
                max_height: Some(HeightBound::Px(90)),
                border: Edges::all(2),
                padding: Edges::all(4),
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
    let frame = layout(&blocks, 100, 150).unwrap();
    assert_eq!(frame.boxes[0].unwrap().border, rect(0, 0, 100, 110));
    assert_eq!(frame.boxes[0].unwrap().content, rect(6, 6, 88, 98));
    assert_eq!(frame.boxes[1].unwrap().content, rect(6, 6, 88, 49));

    let mut tiny = blocks;
    tiny[0].style.min_height = None;
    tiny[0].style.max_height = Some(HeightBound::Px(4));
    tiny[1].style.height = Length::Px(1);
    let frame = layout(&tiny, 100, 150).unwrap();
    assert_eq!(frame.boxes[0].unwrap().content.height, 0);
    assert_eq!(frame.boxes[0].unwrap().border.height, 12);
}

#[test]
fn fractional_and_oversized_height_bounds_fail_even_when_not_selected() {
    for use_min in [true, false] {
        let bound = Some(HeightBound::Percent(50));
        let blocks = [Block {
            style: Style {
                height: Length::Px(2),
                min_height: if use_min { bound } else { None },
                max_height: if use_min { None } else { bound },
                ..Style::default()
            },
            ..Block::default()
        }];
        assert_eq!(
            layout(&blocks, 100, 7),
            Err(LayoutError::FractionalPercentageHeight { node: 0 })
        );
    }
    let huge = [Block {
        style: Style {
            max_height: Some(HeightBound::Percent(100_000_000)),
            ..Style::default()
        },
        ..Block::default()
    }];
    assert_eq!(
        layout(&huge, 100, MAX_VIEWPORT),
        Err(LayoutError::Overflow)
    );
}

#[test]
fn indefinite_auto_parent_rejects_percentage_bounds_on_children() {
    let blocks = [
        Block::default(),
        Block {
            parent: Some(0),
            style: Style {
                min_height: Some(HeightBound::Percent(50)),
                ..Style::default()
            },
            ..Block::default()
        },
    ];
    assert_eq!(
        layout(&blocks, 100, 40),
        Err(LayoutError::UnsupportedPercentageHeight { node: 1 })
    );

    let mut maximum = blocks;
    maximum[1].style.min_height = None;
    maximum[1].style.max_height = Some(HeightBound::Percent(20));
    assert_eq!(
        layout(&maximum, 100, 40),
        Err(LayoutError::UnsupportedPercentageHeight { node: 1 })
    );
}

#[test]
fn hidden_subtrees_skip_unrepresentable_height_bounds() {
    let blocks = [
        Block::default(),
        Block {
            parent: Some(0),
            style: Style {
                display: Display::None,
                min_height: Some(HeightBound::Percent(50)),
                max_height: Some(HeightBound::Percent(u32::MAX)),
                ..Style::default()
            },
            ..Block::default()
        },
        Block {
            parent: Some(1),
            style: Style {
                min_height: Some(HeightBound::Percent(50)),
                ..Style::default()
            },
            ..Block::default()
        },
        Block {
            parent: Some(0),
            style: Style {
                height: Length::Px(7),
                ..Style::default()
            },
            ..Block::default()
        },
    ];
    let frame = layout(&blocks, 100, 21).unwrap();
    assert_eq!(frame.boxes[1], None);
    assert_eq!(frame.boxes[2], None);
    assert_eq!(frame.boxes[3].unwrap().content, rect(0, 0, 100, 7));
}
