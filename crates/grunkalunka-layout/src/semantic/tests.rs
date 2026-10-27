use super::*;
use crate::Length;

fn rect(x: u32, y: u32, width: u32, height: u32) -> Rect {
    Rect {
        x,
        y,
        width,
        height,
    }
}

fn outline<'a>() -> [Element<'a>; 7] {
    [
        Element::new(None, ElementKind::Body, "", 0),
        Element::new(Some(0), ElementKind::Main, "", 0),
        Element::new(Some(1), ElementKind::Heading(1), "Heading", 24),
        Element::new(Some(1), ElementKind::OrderedList, "", 0),
        Element::new(Some(3), ElementKind::ListItem, "First", 18),
        Element::new(Some(3), ElementKind::ListItem, "Second", 18),
        Element::new(Some(1), ElementKind::Paragraph, "Closing", 16),
    ]
}

#[test]
fn semantic_outline_has_exact_independent_box_coordinates() {
    let nodes = outline();
    let view = layout_semantic(&nodes, 320, 100).unwrap();
    let boxes = &view.frame.boxes;
    assert_eq!(boxes[0].unwrap().border, rect(0, 0, 320, 116));
    assert_eq!(boxes[1].unwrap().content, rect(8, 8, 304, 100));
    assert_eq!(boxes[2].unwrap().border, rect(8, 8, 304, 24));
    assert_eq!(boxes[3].unwrap().border, rect(8, 40, 304, 40));
    assert_eq!(boxes[3].unwrap().content, rect(24, 40, 288, 40));
    assert_eq!(boxes[4].unwrap().border, rect(24, 40, 288, 18));
    assert_eq!(boxes[5].unwrap().border, rect(24, 60, 288, 18));
    assert_eq!(boxes[6].unwrap().border, rect(8, 86, 304, 16));
    assert_eq!(
        (view.frame.scroll_width, view.frame.scroll_height),
        (320, 116)
    );
    assert_eq!(
        view.text_boxes,
        vec![
            TextBox {
                node: 2,
                text: "Heading",
                content: rect(8, 8, 304, 24)
            },
            TextBox {
                node: 4,
                text: "First",
                content: rect(24, 40, 288, 18)
            },
            TextBox {
                node: 5,
                text: "Second",
                content: rect(24, 60, 288, 18)
            },
            TextBox {
                node: 6,
                text: "Closing",
                content: rect(8, 86, 304, 16)
            },
        ]
    );
    assert_eq!(layout_semantic(&nodes, 320, 100).unwrap(), view);
}

#[test]
fn hiding_a_list_removes_descendants_and_reflows_the_paragraph() {
    let mut nodes = outline();
    nodes[3].hidden = true;
    let collapsed = layout_semantic(&nodes, 320, 100).unwrap();
    for node in [3, 4, 5] {
        assert!(collapsed.frame.boxes[node].is_none());
    }
    assert_eq!(
        collapsed.frame.boxes[6].unwrap().border,
        rect(8, 40, 304, 16)
    );
    assert_eq!(collapsed.frame.boxes[0].unwrap().border.height, 70);
    assert_eq!(collapsed.frame.scroll_height, 100);
    assert_eq!(
        collapsed
            .text_boxes
            .iter()
            .map(|x| x.node)
            .collect::<Vec<_>>(),
        vec![2, 6]
    );
    nodes[3].hidden = false;
    assert_eq!(
        layout_semantic(&nodes, 320, 100).unwrap().frame.boxes[6]
            .unwrap()
            .border
            .y,
        86
    );
}

#[test]
fn caller_styles_apply_without_css_parsing_or_markup_interpretation() {
    let mut nodes = outline();
    nodes[2].text = "<script>alert('not executed')</script>";
    nodes[2].resolved_style = Some(Style {
        width: Length::Px(400),
        ..Style::default()
    });
    let expanded = layout_semantic(&nodes, 320, 100).unwrap();
    assert_eq!(
        expanded.text_boxes[0].text,
        "<script>alert('not executed')</script>"
    );
    assert_eq!(expanded.frame.boxes[2].unwrap().border, rect(8, 8, 400, 24));
    assert_eq!(
        (expanded.frame.scroll_width, expanded.frame.scroll_height),
        (408, 108)
    );
    nodes[2].hidden = true;
    assert_eq!(
        layout_semantic(&nodes, 320, 100).unwrap().text_boxes[0].text,
        "First"
    );
    assert_eq!(
        layout_semantic(&nodes, 320, 100)
            .unwrap()
            .frame
            .scroll_width,
        320
    );
}

#[test]
fn semantic_validation_rejects_bad_root_heading_mixed_content_and_missing_metrics() {
    let mut nodes = outline();
    nodes[0].kind = ElementKind::Main;
    assert_eq!(
        layout_semantic(&nodes, 320, 100),
        Err(SemanticError::InvalidRootKind)
    );

    let mut nodes = outline();
    nodes[2].kind = ElementKind::Heading(7);
    assert_eq!(
        layout_semantic(&nodes, 320, 100),
        Err(SemanticError::InvalidHeadingLevel { node: 2 })
    );
    nodes[2].kind = ElementKind::Heading(0);
    assert_eq!(
        layout_semantic(&nodes, 320, 100),
        Err(SemanticError::InvalidHeadingLevel { node: 2 })
    );

    let mut nodes = outline();
    nodes[0].text = "root text";
    nodes[0].measured_text_height = 16;
    assert_eq!(
        layout_semantic(&nodes, 320, 100),
        Err(SemanticError::MixedTextAndChildren { node: 0 })
    );

    let mut nodes = outline();
    nodes[4].measured_text_height = 0;
    assert_eq!(
        layout_semantic(&nodes, 320, 100),
        Err(SemanticError::UnmeasuredText { node: 4 })
    );

    let mut nodes = outline();
    nodes[4].kind = ElementKind::Body;
    assert_eq!(
        layout_semantic(&nodes, 320, 100),
        Err(SemanticError::NestedBody { node: 4 })
    );
}

#[test]
fn malformed_parent_viewport_and_oversized_trees_fail_closed() {
    assert_eq!(
        layout_semantic(&[], 320, 100),
        Err(SemanticError::Layout(LayoutError::EmptyDocument))
    );
    let nodes = outline();
    assert_eq!(
        layout_semantic(&nodes, 0, 100),
        Err(SemanticError::Layout(LayoutError::InvalidViewport))
    );
    let mut nodes = outline();
    nodes[4].parent = Some(4);
    assert_eq!(
        layout_semantic(&nodes, 320, 100),
        Err(SemanticError::Layout(LayoutError::InvalidParent {
            node: 4
        }))
    );
    let root = Element::new(None, ElementKind::Body, "", 0);
    let oversized = vec![root; MAX_BLOCKS + 1];
    assert_eq!(
        layout_semantic(&oversized, 320, 100),
        Err(SemanticError::Layout(LayoutError::TooManyBlocks))
    );
}

#[test]
fn utf8_byte_quota_and_numeric_overflow_are_bounded() {
    let enormous = "é".repeat(MAX_TEXT_BYTES / 2 + 1);
    let nodes = [
        Element::new(None, ElementKind::Body, "", 0),
        Element::new(Some(0), ElementKind::Paragraph, &enormous, 16),
    ];
    assert_eq!(
        layout_semantic(&nodes, 320, 100),
        Err(SemanticError::TextQuotaExceeded)
    );
    let nodes = [
        Element::new(None, ElementKind::Body, "", 0),
        Element::new(Some(0), ElementKind::Paragraph, "overflow", u32::MAX),
    ];
    assert_eq!(
        layout_semantic(&nodes, 320, 100),
        Err(SemanticError::Layout(LayoutError::Overflow))
    );
}
