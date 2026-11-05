//! Independent literal pixels and topmost targets for bounded flex paint scopes.
use grunkalunka_layout::hit_test::{HitNode, hit_test, hit_test_scrolled};
use grunkalunka_layout::paint::{PaintNode, Rgb, paint, paint_scrolled};
use grunkalunka_layout::{
    Block, DescendantClip, Display, Edges, FlexAlign, FlexJustify, Frame, Length, PaintRole,
    PointerEvents, Rect, RelativeOffset, Style, Visibility, layout,
};

const GREY: Rgb = Rgb::new(180, 180, 180);
const RED: Rgb = Rgb::new(230, 30, 30);
const GREEN: Rgb = Rgb::new(30, 180, 40);
const BLUE: Rgb = Rgb::new(30, 40, 230);
const CYAN: Rgb = Rgb::new(20, 190, 190);
const PURPLE: Rgb = Rgb::new(180, 30, 180);
const ORANGE: Rgb = Rgb::new(240, 130, 20);

fn rect(x: u32, y: u32, width: u32, height: u32) -> Rect {
    Rect {
        x,
        y,
        width,
        height,
    }
}

fn block(parent: Option<usize>, width: u32, height: u32) -> Block {
    Block {
        parent,
        style: Style {
            width: Length::Px(width),
            height: Length::Px(height),
            ..Style::default()
        },
        ..Block::default()
    }
}

fn row(parent: Option<usize>, width: u32, height: u32) -> Block {
    let mut node = block(parent, width, height);
    node.style.display = Display::FlexRow;
    node
}

fn policies(blocks: &[Block], colors: &[Rgb]) -> (Vec<HitNode>, Vec<PaintNode>) {
    assert_eq!(blocks.len(), colors.len());
    (
        blocks
            .iter()
            .map(|node| HitNode::new(node.parent))
            .collect(),
        blocks
            .iter()
            .zip(colors)
            .map(|(node, color)| PaintNode {
                parent: node.parent,
                background: Some(*color),
                border: Some(Rgb::BLACK),
            })
            .collect(),
    )
}

#[test]
fn original_overflowing_item_covers_the_following_ordinary_block() {
    // The original #533 overlap: B and the following block both cover (118,62).
    let mut blocks = [
        Block {
            intrinsic_height: 1200,
            ..Block::default()
        },
        row(Some(0), 150, 20),
        block(Some(1), 60, 20),
        Block::child(1, 0),
        block(Some(3), 50, 30),
        block(Some(4), 25, 8),
        block(Some(3), 40, 16),
        block(Some(0), 180, 12),
    ];
    blocks[1].style.column_gap = 10;
    blocks[1].style.margin = Edges {
        top: 20,
        left: 20,
        ..Edges::default()
    };
    blocks[1].style.border = Edges::all(2);
    blocks[1].style.padding = Edges::all(4);
    blocks[1].style.justify_content = FlexJustify::SafeEnd;
    blocks[1].style.align_items = FlexAlign::SafeEnd;
    blocks[2].style.border = Edges::all(1);
    blocks[2].style.padding = Edges::all(1);
    blocks[3].style.display = Display::Contents;
    blocks[4].style.margin = Edges {
        top: 2,
        right: 3,
        bottom: 4,
        left: 5,
    };
    blocks[4].style.border = Edges::all(1);
    blocks[4].style.padding = Edges::all(2);
    blocks[6].style.border = Edges::all(1);
    blocks[7].style.margin.top = 6;
    let frame = layout(&blocks, 260, 180).unwrap();
    assert_eq!(frame.boxes[4].unwrap().border, rect(105, 28, 56, 36));
    assert_eq!(frame.boxes[7].unwrap().border, rect(0, 58, 180, 12));
    let (hits, paints) = policies(
        &blocks,
        &[GREY, ORANGE, BLUE, GREY, GREEN, CYAN, PURPLE, RED],
    );
    let canvas = paint(&frame, &paints, Rgb::WHITE).unwrap();
    assert_eq!(
        (canvas.pixel(118, 62), hit_test(&frame, &hits, 118, 62)),
        (Some(GREEN), Ok(Some(4)))
    );
    assert_eq!(
        (canvas.pixel(118, 63), hit_test(&frame, &hits, 118, 63)),
        (Some(Rgb::BLACK), Ok(Some(4)))
    );
    assert_eq!(
        (canvas.pixel(10, 62), hit_test(&frame, &hits, 10, 62)),
        (Some(RED), Ok(Some(7)))
    );
}

const ROW: usize = 1;
const AFTER: usize = 2;

struct NestedScene {
    blocks: Vec<Block>,
    colors: Vec<Rgb>,
    wrapper: Option<usize>,
    a: usize,
    b: usize,
    x: usize,
    inner: usize,
}

fn nested_scene(boxless: bool) -> NestedScene {
    // Source indices are parent-before-child, deliberately not DFS preorder:
    // the following root block precedes all of the row's descendants.
    let mut blocks = vec![
        Block::default(),
        row(Some(0), 40, 10),
        block(Some(0), 40, 30),
    ];
    let mut colors = vec![GREY, BLUE, RED];
    let wrapper = if boxless {
        let index = blocks.len();
        let mut node = Block::child(ROW, 0);
        node.style.display = Display::Contents;
        blocks.push(node);
        colors.push(GREY);
        Some(index)
    } else {
        None
    };
    let a = blocks.len();
    blocks.push(block(Some(wrapper.unwrap_or(ROW)), 20, 10));
    let b = blocks.len();
    blocks.push(block(Some(ROW), 20, 10));
    let x = blocks.len();
    blocks.push(row(Some(a), 20, 10));
    let inner = blocks.len();
    let mut item = block(Some(x), 30, 30);
    item.style.margin.left = 5;
    blocks.push(item);
    colors.extend([ORANGE, GREEN, CYAN, PURPLE]);
    NestedScene {
        blocks,
        colors,
        wrapper,
        a,
        b,
        x,
        inner,
    }
}

fn assert_pixels_and_hits(
    frame: &Frame,
    scene: &NestedScene,
    probes: &[(u32, u32, Rgb, Option<usize>)],
) {
    let (hits, paints) = policies(&scene.blocks, &scene.colors);
    let canvas = paint(frame, &paints, Rgb::WHITE).unwrap();
    for &(x, y, color, target) in probes {
        assert_eq!(
            (canvas.pixel(x, y), hit_test(frame, &hits, x, y)),
            (Some(color), Ok(target)),
            "({x},{y})"
        );
    }
}

#[test]
fn nested_scopes_are_atomic_beyond_the_row_and_preserve_ordinary_descendants() {
    for boxless in [false, true] {
        let scene = nested_scene(boxless);
        let frame = layout(&scene.blocks, 40, 40).unwrap();
        assert_eq!(frame.boxes[AFTER].unwrap().content, rect(0, 10, 40, 30));
        assert_eq!(
            frame.boxes[scene.inner].unwrap().content,
            rect(5, 0, 30, 30)
        );
        assert_eq!(frame.paint_roles[ROW], PaintRole::Ordinary);
        assert_eq!(frame.paint_roles[scene.a], PaintRole::FlexItem);
        assert_eq!(frame.paint_roles[scene.b], PaintRole::FlexItem);
        assert_eq!(frame.paint_roles[scene.x], PaintRole::Ordinary);
        assert_eq!(frame.paint_roles[scene.inner], PaintRole::FlexItem);
        if let Some(wrapper) = scene.wrapper {
            assert_eq!(frame.paint_roles[wrapper], PaintRole::Ordinary);
            assert!(frame.contents[wrapper] && frame.boxes[wrapper].is_none());
        }
        // These distinguish scoped order from per-container DFS, a global
        // item FIFO, and a global ordinary-before-items partition respectively.
        assert_pixels_and_hits(
            &frame,
            &scene,
            &[
                (8, 15, PURPLE, Some(scene.inner)),
                (25, 5, GREEN, Some(scene.b)),
                (2, 5, CYAN, Some(scene.x)),
                (38, 15, RED, Some(AFTER)),
            ],
        );
    }
}

#[test]
fn ordinary_block_trees_keep_their_existing_depth_first_precedence() {
    let mut scene = nested_scene(false);
    scene.blocks[ROW].style.display = Display::Block;
    scene.blocks[scene.x].style.display = Display::Block;
    let frame = layout(&scene.blocks, 40, 40).unwrap();
    assert!(
        frame
            .paint_roles
            .iter()
            .all(|role| *role == PaintRole::Ordinary)
    );
    assert_pixels_and_hits(
        &frame,
        &scene,
        &[
            (8, 15, RED, Some(AFTER)),
            (25, 5, PURPLE, Some(scene.inner)),
            (2, 5, CYAN, Some(scene.x)),
        ],
    );
}

#[test]
fn hidden_item_scopes_keep_visible_descendants_and_independent_input_filters() {
    let mut scene = nested_scene(true);
    scene.blocks[scene.a].style.visibility = Visibility::Hidden;
    scene.blocks[scene.a].style.pointer_events = PointerEvents::None;
    scene.blocks[scene.inner].style.visibility = Visibility::Visible;
    scene.blocks[scene.inner].style.pointer_events = PointerEvents::Auto;
    scene.blocks[scene.b].style.visibility = Visibility::Hidden;
    scene.blocks[scene.b].style.pointer_events = PointerEvents::None;
    let restored = scene.blocks.len();
    let mut child = block(Some(scene.b), 5, 5);
    child.style.visibility = Visibility::Visible;
    child.style.pointer_events = PointerEvents::Auto;
    scene.blocks.push(child);
    scene.colors.push(ORANGE);
    let frame = layout(&scene.blocks, 40, 40).unwrap();
    assert_eq!(frame.paint_roles[scene.a], PaintRole::FlexItem);
    assert_eq!(frame.paint_roles[scene.b], PaintRole::FlexItem);
    assert_eq!(frame.paint_roles[restored], PaintRole::Ordinary);
    assert_pixels_and_hits(
        &frame,
        &scene,
        &[
            (8, 15, PURPLE, Some(scene.inner)),
            (2, 5, BLUE, Some(ROW)),
            (22, 2, ORANGE, Some(restored)),
            (27, 2, PURPLE, Some(scene.inner)),
        ],
    );
    let (mut hits, mut paints) = policies(&scene.blocks, &scene.colors);
    hits[restored].targetable = false;
    let canvas = paint(&frame, &paints, Rgb::WHITE).unwrap();
    assert_eq!(canvas.pixel(22, 2), Some(ORANGE));
    assert_eq!(hit_test(&frame, &hits, 22, 2), Ok(Some(scene.inner)));
    hits[restored].targetable = true;
    paints[restored].background = None;
    let transparent = paint(&frame, &paints, Rgb::WHITE).unwrap();
    assert_eq!(transparent.pixel(22, 2), Some(PURPLE));
    assert_eq!(hit_test(&frame, &hits, 22, 2), Ok(Some(restored)));
}

#[test]
fn display_none_prunes_roles_and_subtrees_without_visibility_revival() {
    let mut scene = nested_scene(true);
    let wrapper = scene.wrapper.unwrap();
    scene.blocks[scene.inner].style.visibility = Visibility::Visible;
    for hidden in [wrapper, scene.a] {
        scene.blocks[wrapper].style.display = Display::Contents;
        scene.blocks[scene.a].style.display = Display::Block;
        scene.blocks[hidden].style.display = Display::None;
        let frame = layout(&scene.blocks, 40, 40).unwrap();
        for node in [scene.a, scene.x, scene.inner] {
            assert!(frame.boxes[node].is_none());
            assert_eq!(frame.paint_roles[node], PaintRole::Ordinary);
        }
        assert_eq!(frame.paint_roles[scene.b], PaintRole::FlexItem);
        assert_pixels_and_hits(
            &frame,
            &scene,
            &[(8, 5, GREEN, Some(scene.b)), (8, 15, RED, Some(AFTER))],
        );
    }
    scene.blocks[0].style.display = Display::None;
    let absent = layout(&scene.blocks, 40, 40).unwrap();
    assert!(absent.boxes.iter().all(Option::is_none));
    assert!(
        absent
            .paint_roles
            .iter()
            .all(|role| *role == PaintRole::Ordinary)
    );
    assert_pixels_and_hits(&absent, &scene, &[(8, 15, Rgb::WHITE, None)]);
}

#[test]
fn deferred_scopes_keep_original_ancestry_and_empty_clip_intersections() {
    let mut scene = nested_scene(true);
    scene.blocks[ROW].style.descendant_clip = DescendantClip::PaddingBox;
    scene.blocks[scene.wrapper.unwrap()].style.descendant_clip = DescendantClip::PaddingBox;
    let clipped = layout(&scene.blocks, 40, 40).unwrap();
    assert_pixels_and_hits(
        &clipped,
        &scene,
        &[
            (8, 5, PURPLE, Some(scene.inner)),
            (25, 5, GREEN, Some(scene.b)),
            (8, 15, RED, Some(AFTER)),
        ],
    );
    scene.blocks[scene.x].style.relative_offset.y = 20;
    scene.blocks[scene.x].style.descendant_clip = DescendantClip::PaddingBox;
    let disjoint = layout(&scene.blocks, 40, 40).unwrap();
    assert_eq!(
        disjoint.boxes[scene.x].unwrap().padding,
        rect(0, 20, 20, 10)
    );
    assert_pixels_and_hits(
        &disjoint,
        &scene,
        &[(8, 25, RED, Some(AFTER)), (8, 5, ORANGE, Some(scene.a))],
    );
    scene.blocks[ROW].style.descendant_clip = DescendantClip::None;
    let inner_clip = layout(&scene.blocks, 40, 40).unwrap();
    assert_pixels_and_hits(
        &inner_clip,
        &scene,
        &[
            (8, 25, PURPLE, Some(scene.inner)),
            (8, 35, RED, Some(AFTER)),
            (25, 25, RED, Some(AFTER)),
        ],
    );
}

#[test]
fn each_scope_keeps_its_own_border_outside_its_descendant_clip() {
    let mut blocks = [
        Block::default(),
        row(Some(0), 10, 4),
        block(Some(0), 18, 10),
        block(Some(1), 12, 8),
        block(Some(3), 20, 20),
    ];
    blocks[1].style.border = Edges::all(2);
    blocks[1].style.padding = Edges::all(2);
    blocks[1].style.descendant_clip = DescendantClip::PaddingBox;
    blocks[3].style.border = Edges::all(1);
    blocks[3].style.descendant_clip = DescendantClip::PaddingBox;
    let frame = layout(&blocks, 30, 30).unwrap();
    assert_eq!(frame.boxes[1].unwrap().padding, rect(2, 2, 14, 8));
    assert_eq!(frame.boxes[3].unwrap().padding, rect(5, 5, 12, 8));
    let (hits, paints) = policies(&blocks, &[GREY, CYAN, RED, BLUE, GREEN]);
    let canvas = paint(&frame, &paints, Rgb::WHITE).unwrap();
    for (x, y, color, target) in [
        (4, 5, Rgb::BLACK, 3),
        (1, 5, Rgb::BLACK, 1),
        (6, 6, GREEN, 4),
        (6, 13, RED, 2),
    ] {
        assert_eq!(
            (canvas.pixel(x, y), hit_test(&frame, &hits, x, y)),
            (Some(color), Ok(Some(target)))
        );
    }
}

#[test]
fn scoped_order_and_clips_use_the_same_document_coordinates_when_scrolled() {
    let mut scene = nested_scene(true);
    let frame = layout(&scene.blocks, 20, 20).unwrap();
    assert_eq!((frame.scroll_width, frame.scroll_height), (40, 40));
    let (hits, paints) = policies(&scene.blocks, &scene.colors);
    for (scroll_x, scroll_y, x, y, color, target) in [
        (10, 0, 15, 5, GREEN, scene.b),
        (0, 10, 8, 5, PURPLE, scene.inner),
        (0, 10, 2, 5, RED, AFTER),
    ] {
        let canvas = paint_scrolled(&frame, &paints, Rgb::WHITE, scroll_x, scroll_y).unwrap();
        assert_eq!(
            (
                canvas.pixel(x, y),
                hit_test_scrolled(&frame, &hits, x, y, scroll_x, scroll_y)
            ),
            (Some(color), Ok(Some(target)))
        );
    }
    scene.blocks[ROW].style.descendant_clip = DescendantClip::PaddingBox;
    let clipped = layout(&scene.blocks, 20, 20).unwrap();
    let canvas = paint_scrolled(&clipped, &paints, Rgb::WHITE, 0, 10).unwrap();
    assert_eq!(
        (
            canvas.pixel(8, 5),
            hit_test_scrolled(&clipped, &hits, 8, 5, 0, 10)
        ),
        (Some(RED), Ok(Some(AFTER)))
    );
}

#[test]
fn relative_offsets_change_geometry_without_changing_paint_roles() {
    let mut scene = nested_scene(false);
    let original = layout(&scene.blocks, 40, 40).unwrap();
    scene.blocks[scene.a].style.relative_offset = RelativeOffset { x: 1, y: 2 };
    scene.blocks[AFTER].style.relative_offset.x = 4;
    let shifted = layout(&scene.blocks, 40, 40).unwrap();
    assert_eq!(shifted.paint_roles, original.paint_roles);
    assert_eq!(shifted.boxes[scene.a].unwrap().content, rect(1, 2, 20, 10));
    assert_eq!(
        shifted.boxes[scene.inner].unwrap().content,
        rect(6, 2, 30, 30)
    );
    assert_eq!(shifted.boxes[scene.b], original.boxes[scene.b]);
    assert_eq!(shifted.boxes[AFTER].unwrap().content, rect(4, 10, 40, 30));
    assert_pixels_and_hits(
        &shifted,
        &scene,
        &[
            (8, 15, PURPLE, Some(scene.inner)),
            (25, 5, GREEN, Some(scene.b)),
            (2, 5, CYAN, Some(scene.x)),
        ],
    );
}
