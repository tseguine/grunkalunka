//! Role resolution and public-consumer boundaries for scoped paint ordering.

use grunkalunka_layout::hit_test::{HitError, HitNode, hit_test, hit_test_scrolled};
use grunkalunka_layout::paint::{PaintError, PaintNode, Rgb, paint, paint_scrolled};
use grunkalunka_layout::{
    Block, Display, Frame, LayoutError, Length, MAX_BLOCKS, MAX_DEPTH, PaintRole, PointerEvents,
    Style, Visibility, layout,
};

fn node(parent: Option<usize>, display: Display, width: u32, height: u32) -> Block {
    Block {
        parent,
        style: Style {
            display,
            width: Length::Px(width),
            height: Length::Px(height),
            ..Style::default()
        },
        ..Block::default()
    }
}

fn policies(blocks: &[Block]) -> (Vec<HitNode>, Vec<PaintNode>) {
    let hits = blocks
        .iter()
        .map(|block| HitNode::new(block.parent))
        .collect();
    let paints = blocks
        .iter()
        .enumerate()
        .map(|(index, block)| PaintNode {
            parent: block.parent,
            background: Some(if index == blocks.len() - 1 {
                Rgb::BLACK
            } else {
                Rgb::WHITE
            }),
            border: None,
        })
        .collect();
    (hits, paints)
}

#[test]
fn generated_parent_roles_follow_contents_and_recompute_after_display_changes() {
    use PaintRole::{FlexItem, Ordinary};
    // Parent-before-child input deliberately interleaves the following block
    // before the row's descendants. Root and nested contents are boxless.
    let mut blocks = vec![
        node(None, Display::Contents, 64, 64),
        node(Some(0), Display::FlexRow, 30, 10),
        node(Some(0), Display::Block, 20, 10),
        node(Some(1), Display::Contents, 0, 0),
        node(Some(3), Display::Block, 10, 10),
        node(Some(3), Display::FlexRow, 10, 10),
        node(Some(4), Display::Block, 5, 5),
        node(Some(5), Display::Contents, 0, 0),
        node(Some(7), Display::Block, 5, 5),
        node(Some(1), Display::None, 0, 0),
        node(Some(9), Display::FlexRow, 1, 1),
        node(Some(10), Display::Block, 1, 1),
    ];
    let initial = layout(&blocks, 64, 64).unwrap();
    assert_eq!(
        initial.paint_roles,
        [
            Ordinary, Ordinary, Ordinary, Ordinary, FlexItem, FlexItem, Ordinary, Ordinary,
            FlexItem, Ordinary, Ordinary, Ordinary
        ]
    );
    assert!(initial.boxes[9..].iter().all(Option::is_none));
    blocks[4].style.visibility = Visibility::Hidden;
    blocks[4].style.pointer_events = PointerEvents::None;
    blocks[6].style.visibility = Visibility::Visible;
    blocks[6].style.pointer_events = PointerEvents::Auto;
    assert_eq!(
        layout(&blocks, 64, 64).unwrap().paint_roles,
        initial.paint_roles
    );

    blocks[1].style.display = Display::Block;
    let ordinary_parent = layout(&blocks, 64, 64).unwrap();
    assert_eq!(ordinary_parent.paint_roles[4], Ordinary);
    assert_eq!(ordinary_parent.paint_roles[5], Ordinary);
    assert_eq!(ordinary_parent.paint_roles[8], FlexItem);
    blocks[1].style.display = Display::None;
    let suppressed = layout(&blocks, 64, 64).unwrap();
    assert!(suppressed.paint_roles.iter().all(|role| *role == Ordinary));
    blocks[1].style.display = Display::FlexRow;
    assert_eq!(
        layout(&blocks, 64, 64).unwrap().paint_roles,
        initial.paint_roles
    );
}

#[test]
fn malformed_role_metadata_is_rejected_before_any_hit_selection() {
    let blocks = [
        node(None, Display::Block, 30, 30),
        node(Some(0), Display::FlexRow, 20, 10),
        node(Some(1), Display::Block, 10, 10),
        node(Some(1), Display::Contents, 0, 0),
        node(Some(3), Display::Block, 5, 5),
        node(Some(1), Display::None, 1, 1),
    ];
    let frame = layout(&blocks, 30, 30).unwrap();
    let (hits, paints) = policies(&blocks);
    let mut missing = frame.clone();
    missing.paint_roles.pop();
    assert_eq!(
        paint(&missing, &paints, Rgb::WHITE),
        Err(PaintError::NodeCountMismatch)
    );
    assert_eq!(
        hit_test(&missing, &hits, 0, 0),
        Err(HitError::NodeCountMismatch)
    );
    // A valid candidate exists at (0,0); malformed later metadata still wins.
    // Outside-viewport queries must perform the same metadata validation.
    for index in [0, 3, 5] {
        let mut malformed = frame.clone();
        malformed.paint_roles[index] = PaintRole::FlexItem;
        assert_eq!(
            paint(&malformed, &paints, Rgb::WHITE),
            Err(PaintError::InvalidBox { node: index })
        );
        assert_eq!(
            hit_test(&malformed, &hits, 0, 0),
            Err(HitError::InvalidBox { node: index })
        );
        assert_eq!(
            hit_test(&malformed, &hits, 100, 100),
            Err(HitError::InvalidBox { node: index })
        );
    }
    // Even an ineligible nested item is validated before an earlier target
    // can be accepted, or an outside-viewport query can return no target.
    let mut malformed = frame;
    malformed.visibility[4] = false;
    malformed.pointer_events[4] = false;
    malformed.boxes[4].as_mut().unwrap().content.x = u32::MAX;
    let mut ineligible_hits = hits;
    ineligible_hits[4].targetable = false;
    assert_eq!(
        paint(&malformed, &paints, Rgb::WHITE),
        Err(PaintError::Overflow)
    );
    assert_eq!(
        hit_test(&malformed, &ineligible_hits, 0, 0),
        Err(HitError::Overflow)
    );
    assert_eq!(
        hit_test(&malformed, &ineligible_hits, 100, 100),
        Err(HitError::Overflow)
    );
}

fn append_frame_item(frame: &mut Frame) {
    frame.boxes.push(*frame.boxes.last().unwrap());
    frame.contents.push(false);
    frame.paint_roles.push(PaintRole::FlexItem);
    frame.visibility.push(true);
    frame.pointer_events.push(true);
    frame.clip_descendants.push(false);
}

#[test]
fn nested_and_wide_paint_scopes_keep_native_stack_and_input_bounds() {
    std::thread::Builder::new()
        .name("flex-paint-boundaries".into())
        .stack_size(2 * 1024 * 1024)
        .spawn(|| {
            let mut nested = vec![node(None, Display::FlexRow, 1, 1)];
            for index in 1..=MAX_DEPTH {
                nested.push(node(Some(index - 1), Display::FlexRow, 1, 1));
            }
            let mut frame = layout(&nested, 1, 1).unwrap();
            let (hits, paints) = policies(&nested);
            assert_eq!(
                paint(&frame, &paints, Rgb::WHITE).unwrap().pixel(0, 0),
                Some(Rgb::BLACK)
            );
            assert_eq!(hit_test(&frame, &hits, 0, 0), Ok(Some(MAX_DEPTH)));
            nested.push(node(Some(MAX_DEPTH), Display::FlexRow, 1, 1));
            assert_eq!(
                layout(&nested, 1, 1),
                Err(LayoutError::TooDeep {
                    node: MAX_DEPTH + 1
                })
            );
            append_frame_item(&mut frame);
            let (hits, paints) = policies(&nested);
            assert_eq!(
                paint(&frame, &paints, Rgb::WHITE),
                Err(PaintError::TooDeep {
                    node: MAX_DEPTH + 1
                })
            );
            assert_eq!(
                hit_test(&frame, &hits, 0, 0),
                Err(HitError::TooDeep {
                    node: MAX_DEPTH + 1
                })
            );

            let mut wide = vec![node(None, Display::FlexRow, 1, 1)];
            wide.extend((1..MAX_BLOCKS).map(|_| node(Some(0), Display::Block, 1, 1)));
            let mut frame = layout(&wide, 1, 1).unwrap();
            let (hits, paints) = policies(&wide);
            let scroll_x = (MAX_BLOCKS - 2) as u32;
            assert_eq!(
                paint_scrolled(&frame, &paints, Rgb::WHITE, scroll_x, 0)
                    .unwrap()
                    .pixel(0, 0),
                Some(Rgb::BLACK)
            );
            assert_eq!(
                hit_test_scrolled(&frame, &hits, 0, 0, scroll_x, 0),
                Ok(Some(MAX_BLOCKS - 1))
            );
            wide.push(node(Some(0), Display::Block, 1, 1));
            assert_eq!(layout(&wide, 1, 1), Err(LayoutError::TooManyBlocks));
            append_frame_item(&mut frame);
            let (hits, paints) = policies(&wide);
            assert_eq!(
                paint(&frame, &paints, Rgb::WHITE),
                Err(PaintError::NodeCountMismatch)
            );
            assert_eq!(hit_test(&frame, &hits, 0, 0), Err(HitError::TooManyNodes));
        })
        .unwrap()
        .join()
        .unwrap();
}
