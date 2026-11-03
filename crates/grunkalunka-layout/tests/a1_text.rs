//! Exact glyph, wrap, geometry and native-reflow checks for A1's private font.

#[path = "../examples/a1_note_scene/text.rs"]
mod text;

use grunkalunka_layout::paint::{Canvas, PaintNode, Rgb, paint};
use grunkalunka_layout::semantic::{Element, ElementKind, layout_semantic};
use grunkalunka_layout::{Block, MAX_VIEWPORT, Rect, layout};
use text::{TextError, TextPlan};

fn blank(width: u32, height: u32) -> Canvas {
    let frame = layout(&[Block::default()], width, height).unwrap();
    paint(&frame, &[PaintNode::default()], Rgb::WHITE).unwrap()
}

fn draw(source: &str, width: u32) -> Canvas {
    let plan = TextPlan::new(source, width).unwrap();
    let mut canvas = blank(width, plan.height().max(1));
    plan.paint(
        &mut canvas,
        Rect {
            width,
            height: plan.height(),
            ..Rect::default()
        },
        Rgb::BLACK,
    )
    .unwrap();
    canvas
}

#[test]
fn actual_characters_choose_distinct_case_sensitive_ink_with_transparent_spacing() {
    let canvas = draw("Hh", 15);
    // H and h have distinct top rows; both retain the same six-pixel advance.
    for ((x, y), color) in [
        ((2, 2), Rgb::BLACK),
        ((4, 2), Rgb::WHITE),
        ((6, 2), Rgb::BLACK),
        ((7, 2), Rgb::WHITE),
        ((8, 2), Rgb::BLACK),
        ((12, 2), Rgb::WHITE),
        ((12, 5), Rgb::BLACK),
        ((0, 0), Rgb::WHITE),
        ((14, 10), Rgb::WHITE),
    ] {
        assert_eq!(canvas.pixel(x, y), Some(color), "{x},{y}");
    }
    assert_ne!(draw("H", 9).ppm(), draw("I", 9).ppm());
    assert_ne!(draw("H", 9).ppm(), draw("h", 9).ppm());
}

#[test]
fn every_printable_ascii_character_has_a_cell_and_only_space_has_no_ink() {
    let source: String = (32u8..=126).map(char::from).collect();
    let canvas = draw(&source, 574);
    assert_eq!(canvas.height(), 11);
    for (index, scalar) in source.chars().enumerate() {
        let x = 2 + index as u32 * 6;
        let has_ink =
            (2..9).any(|y| (x..x + 5).any(|column| canvas.pixel(column, y) == Some(Rgb::BLACK)));
        assert_eq!(has_ink, scalar != ' ', "U+{:04X}", scalar as u32);
    }
}

#[test]
fn unsupported_scalars_get_visible_boxes_without_losing_cells() {
    let canvas = draw("é子😀\0", 27);
    assert_eq!(canvas.height(), 11);
    for x in [2, 8, 14, 20] {
        for column in x..x + 5 {
            assert_eq!(canvas.pixel(column, 2), Some(Rgb::BLACK));
            assert_eq!(canvas.pixel(column, 8), Some(Rgb::BLACK));
        }
        assert_eq!(canvas.pixel(x, 3), Some(Rgb::BLACK));
        assert_eq!(canvas.pixel(x + 2, 3), Some(Rgb::WHITE));
        assert_eq!(canvas.pixel(x + 2, 4), Some(Rgb::BLACK));
        assert_eq!(canvas.pixel(x + 4, 3), Some(Rgb::BLACK));
    }
    assert_ne!(draw("é", 9).ppm(), draw("?", 9).ppm());
}

#[test]
fn wrapping_preserves_character_order_and_uses_the_measured_line_height() {
    let plan = TextPlan::new("ABCD", 15).unwrap();
    assert_eq!(plan.height(), 20);
    let canvas = draw("ABCD", 15);
    // Two cells fit; C and D begin at y=11, after a nine-pixel line advance.
    for ((x, y), color) in [
        ((2, 2), Rgb::WHITE),
        ((3, 2), Rgb::BLACK),
        ((8, 2), Rgb::BLACK),
        ((12, 2), Rgb::WHITE),
        ((2, 11), Rgb::WHITE),
        ((3, 11), Rgb::BLACK),
        ((8, 11), Rgb::BLACK),
        ((12, 11), Rgb::WHITE),
    ] {
        assert_eq!(canvas.pixel(x, y), Some(color), "{x},{y}");
    }
    for x in 0..15 {
        for y in [0, 1, 9, 10, 18, 19] {
            assert_eq!(canvas.pixel(x, y), Some(Rgb::WHITE), "{x},{y}");
        }
    }
    // Spaces consume cells too, rather than stripping data at a wrap boundary.
    let spaced = draw("A C", 15);
    assert_eq!(spaced.pixel(8, 2), Some(Rgb::WHITE));
    assert_eq!(spaced.pixel(3, 11), Some(Rgb::BLACK));
}

#[test]
fn measured_content_contains_all_ink_and_canvas_clips_offscreen_cells() {
    let plan = TextPlan::new("H", 9).unwrap();
    let mut canvas = blank(20, 20);
    plan.paint(
        &mut canvas,
        Rect {
            x: 17,
            y: 17,
            width: 9,
            height: 11,
        },
        Rgb::BLACK,
    )
    .unwrap();
    for y in 0..20 {
        for x in 0..20 {
            let expected = if (x, y) == (19, 19) {
                Rgb::BLACK
            } else {
                Rgb::WHITE
            };
            assert_eq!(canvas.pixel(x, y), Some(expected), "{x},{y}");
        }
    }
}

#[test]
fn geometry_failures_are_reported_before_any_pixels_change() {
    let plan = TextPlan::new("AB", 15).unwrap();
    let mut canvas = blank(20, 20);
    let unchanged = canvas.ppm();
    for content in [
        Rect {
            width: 14,
            height: 11,
            ..Rect::default()
        },
        Rect {
            width: 15,
            height: 10,
            ..Rect::default()
        },
        Rect {
            x: u32::MAX - 5,
            width: 15,
            height: 11,
            ..Rect::default()
        },
        Rect {
            y: u32::MAX - 5,
            width: 15,
            height: 11,
            ..Rect::default()
        },
    ] {
        assert_eq!(
            plan.paint(&mut canvas, content, Rgb::BLACK),
            Err(TextError::Geometry)
        );
        assert_eq!(canvas.ppm(), unchanged);
    }
}

#[test]
fn text_work_is_bounded_by_utf8_bytes_and_width_is_checked() {
    assert_eq!(TextPlan::new("", 9).unwrap().height(), 0);
    assert_eq!(TextPlan::new(" ", 9).unwrap().height(), 11);
    assert_eq!(TextPlan::new(&"A".repeat(4096), 9).unwrap().height(), 36866);
    assert_eq!(
        TextPlan::new(&"A".repeat(4097), 9).unwrap_err(),
        TextError::TextTooLarge
    );
    assert_eq!(
        TextPlan::new(&"é".repeat(2049), 9).unwrap_err(),
        TextError::TextTooLarge
    );
    for width in [0, 8, MAX_VIEWPORT + 1] {
        assert_eq!(
            TextPlan::new("A", width).unwrap_err(),
            TextError::InvalidWidth
        );
    }
}

#[test]
fn measured_wrapping_moves_following_native_rows_and_their_glyphs() {
    let label = "A".repeat(15);
    let mut elements = [
        Element::new(None, ElementKind::Body, "", 0),
        Element::new(Some(0), ElementKind::Main, "", 0),
        Element::new(Some(1), ElementKind::Heading(1), "Note", 24),
        Element::new(Some(1), ElementKind::OrderedList, "", 0),
        Element::new(Some(3), ElementKind::ListItem, &label, 18),
        Element::new(Some(3), ElementKind::ListItem, "End", 18),
    ];
    let initial = layout_semantic(&elements, 80, 160).unwrap();
    assert_eq!(initial.frame.boxes[4].unwrap().content.width, 48);
    assert_eq!(initial.frame.boxes[5].unwrap().content.y, 60);
    let mut plans = Vec::new();
    for anchor in &initial.text_boxes {
        let plan = TextPlan::new(anchor.text, anchor.content.width).unwrap();
        let element = &mut elements[anchor.node];
        element.measured_text_height = element.measured_text_height.max(plan.height());
        plans.push((anchor.node, plan));
    }
    let reflowed = layout_semantic(&elements, 80, 160).unwrap();
    assert_eq!(reflowed.text_boxes[1].text, label);
    assert_eq!(reflowed.frame.boxes[4].unwrap().content.y, 40);
    assert_eq!(reflowed.frame.boxes[4].unwrap().content.height, 29);
    assert_eq!(reflowed.frame.boxes[5].unwrap().content.y, 71);
    let paints: Vec<_> = elements
        .iter()
        .map(|element| PaintNode {
            parent: element.parent,
            ..PaintNode::default()
        })
        .collect();
    let mut canvas = paint(&reflowed.frame, &paints, Rgb::WHITE).unwrap();
    for (node, plan) in plans {
        plan.paint(
            &mut canvas,
            reflowed.frame.boxes[node].unwrap().content,
            Rgb::BLACK,
        )
        .unwrap();
    }
    assert_eq!(canvas.pixel(27, 60), Some(Rgb::BLACK), "wrapped A");
    assert_eq!(canvas.pixel(26, 73), Some(Rgb::BLACK), "reflowed E");
    assert_eq!(canvas.pixel(24, 40), Some(Rgb::WHITE), "inset preserved");
}
