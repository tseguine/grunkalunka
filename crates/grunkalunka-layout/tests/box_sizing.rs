//! Exact content-box versus border-box geometry from caller-resolved inputs.
//! No CSS parsing, fractional pixel rounding, or general formatting context.
use grunkalunka_layout::{
    Block, BoxSizing, Display, Edges, LayoutError, Length, Rect, Style, layout,
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
fn fixed_border_box_sizing_excludes_padding_border_but_not_margins() {
    let blocks = [Block {
        style: Style {
            box_sizing: BoxSizing::BorderBox,
            width: Length::Px(120),
            height: Length::Px(80),
            margin: Edges {
                left: 4,
                right: 6,
                top: 5,
                bottom: 9,
            },
            border: Edges::all(3),
            padding: Edges::all(7),
            ..Style::default()
        },
        ..Block::default()
    }];
    let frame = layout(&blocks, 200, 150).unwrap();
    let geometry = frame.boxes[0].unwrap();
    assert_eq!(geometry.margin, rect(0, 0, 130, 94));
    assert_eq!(geometry.border, rect(4, 5, 120, 80));
    assert_eq!(geometry.padding, rect(7, 8, 114, 74));
    assert_eq!(geometry.content, rect(14, 15, 100, 60));
    assert_eq!((frame.scroll_width, frame.scroll_height), (200, 150));
}

#[test]
fn nested_percent_sizing_uses_each_parent_definite_content_box() {
    let blocks = [
        Block {
            style: Style {
                box_sizing: BoxSizing::BorderBox,
                width: Length::Px(200),
                height: Length::Px(100),
                border: Edges::all(5),
                padding: Edges::all(10),
                ..Style::default()
            },
            ..Block::default()
        },
        Block {
            parent: Some(0),
            style: Style {
                box_sizing: BoxSizing::BorderBox,
                width: Length::Percent(50),
                height: Length::Percent(50),
                border: Edges::all(1),
                padding: Edges::all(4),
                ..Style::default()
            },
            ..Block::default()
        },
        Block {
            parent: Some(1),
            style: Style {
                width: Length::Percent(40),
                height: Length::Percent(20),
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
    let frame = layout(&blocks, 250, 150).unwrap();
    assert_eq!(frame.boxes[0].unwrap().border, rect(0, 0, 200, 100));
    assert_eq!(frame.boxes[0].unwrap().content, rect(15, 15, 170, 70));
    assert_eq!(frame.boxes[1].unwrap().border, rect(15, 15, 85, 35));
    assert_eq!(frame.boxes[1].unwrap().content, rect(20, 20, 75, 25));
    assert_eq!(frame.boxes[2].unwrap().content, rect(20, 20, 30, 5));
    assert_eq!(frame.boxes[3].unwrap().content, rect(15, 50, 170, 10));
}

#[test]
fn too_small_declared_border_box_cannot_make_negative_content_dimensions() {
    let blocks = [
        Block {
            style: Style {
                box_sizing: BoxSizing::BorderBox,
                width: Length::Px(5),
                height: Length::Px(5),
                border: Edges::all(3),
                padding: Edges::all(2),
                ..Style::default()
            },
            ..Block::default()
        },
        Block {
            parent: Some(0),
            style: Style {
                width: Length::Px(20),
                height: Length::Px(20),
                ..Style::default()
            },
            ..Block::default()
        },
    ];
    let frame = layout(&blocks, 20, 20).unwrap();
    let root = frame.boxes[0].unwrap();
    assert_eq!(root.content, rect(5, 5, 0, 0));
    assert_eq!(root.border, rect(0, 0, 10, 10));
    assert_eq!(frame.boxes[1].unwrap().border, rect(5, 5, 20, 20));
    assert_eq!((frame.scroll_width, frame.scroll_height), (25, 25));
}

#[test]
fn root_border_box_percentages_use_viewport_without_rounding() {
    let blocks = [Block {
        style: Style {
            box_sizing: BoxSizing::BorderBox,
            width: Length::Percent(100),
            height: Length::Percent(100),
            border: Edges::all(1),
            padding: Edges::all(2),
            ..Style::default()
        },
        ..Block::default()
    }];
    let frame = layout(&blocks, 100, 80).unwrap();
    assert_eq!(frame.boxes[0].unwrap().border, rect(0, 0, 100, 80));
    assert_eq!(frame.boxes[0].unwrap().content, rect(3, 3, 94, 74));

    let mut fractional = blocks;
    fractional[0].style.width = Length::Percent(50);
    assert_eq!(
        layout(&fractional, 7, 80),
        Err(LayoutError::FractionalPercentageWidth { node: 0 })
    );
    fractional[0].style.width = Length::Auto;
    fractional[0].style.height = Length::Percent(50);
    assert_eq!(
        layout(&fractional, 100, 7),
        Err(LayoutError::FractionalPercentageHeight { node: 0 })
    );
}

#[test]
fn auto_dimensions_and_hidden_subtrees_preserve_existing_flow_rules() {
    assert_eq!(Style::default().box_sizing, BoxSizing::ContentBox);
    let mut blocks = [
        Block {
            style: Style {
                margin: Edges::all(2),
                border: Edges::all(1),
                padding: Edges::all(3),
                ..Style::default()
            },
            ..Block::default()
        },
        Block::child(0, 11),
    ];
    let content = layout(&blocks, 50, 40).unwrap();
    blocks[0].style.box_sizing = BoxSizing::BorderBox;
    let border = layout(&blocks, 50, 40).unwrap();
    assert_eq!(
        content, border,
        "auto sizes have no explicit border-box length"
    );

    blocks[1].style.display = Display::None;
    blocks[1].style.height = Length::Percent(50);
    let hidden = layout(&blocks, 50, 40).unwrap();
    assert_eq!(hidden.boxes[1], None);
}
