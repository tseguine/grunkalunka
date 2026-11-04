//! A1-specific host adapter for the independently reusable native semantic
//! layout, bounded bitmap text service and RGB painter. This is not HTML or DOM.
//! A trusted host passes only validated visible WIT heading records over stdin;
//! no note source, path, network or guest capability enters this process.

use grunkalunka_layout::Rect;
use grunkalunka_layout::hit_test::{HitNode, hit_test_scrolled};
use grunkalunka_layout::paint::{PaintNode, Rgb, paint_scrolled};
use grunkalunka_layout::semantic::{Element, ElementKind, default_style, layout_semantic};
use std::fmt::Write as _;
use std::io::{self, Read, Write};

#[path = "a1_note_scene/text.rs"]
mod text;

const VIEW_WIDTH: u32 = 320;
const VIEW_HEIGHT: u32 = 160;
const MAX_WIRE_BYTES: usize = 24 * 1024;
const MAX_SOURCE_BYTES: u32 = 4096;
const MAX_LABEL_BYTES: usize = 4096;
const MAX_HEADINGS: usize = 64;
const USAGE: &str = "usage: a1_note_scene --report|--ppm|--view|--hit X Y [--scroll-y U32]";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Profile {
    Granted,
    Denied,
}

impl Profile {
    fn label(self) -> &'static str {
        match self {
            Self::Granted => "granted",
            Self::Denied => "denied",
        }
    }
}

struct Heading {
    level: u8,
    start: u32,
    end: u32,
    _section_end: u32,
    _parent: Option<usize>,
    text: String,
}

struct Note {
    profile: Profile,
    headings: Vec<Heading>,
    control: Option<OutlineControl>,
}

#[derive(Clone, Copy)]
struct OutlineControl {
    total_count: usize,
    expanded: bool,
}

fn decimal<T: std::str::FromStr>(input: &str, label: &str) -> Result<T, String> {
    input
        .parse()
        .map_err(|_| format!("invalid decimal {label}"))
}

fn nibble(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        _ => None,
    }
}

fn decode_hex(input: &str) -> Result<String, String> {
    if input.len() > MAX_LABEL_BYTES * 2 || !input.len().is_multiple_of(2) {
        return Err("invalid heading hex length".into());
    }
    let mut bytes = Vec::with_capacity(input.len() / 2);
    for pair in input.as_bytes().as_chunks::<2>().0 {
        let high = nibble(pair[0]).ok_or("invalid heading hex byte")?;
        let low = nibble(pair[1]).ok_or("invalid heading hex byte")?;
        bytes.push(high << 4 | low);
    }
    String::from_utf8(bytes).map_err(|_| "invalid heading UTF-8".into())
}

fn encode_hex(text: &str) -> String {
    let mut encoded = String::with_capacity(text.len() * 2);
    for byte in text.bytes() {
        write!(&mut encoded, "{byte:02x}").expect("write to String");
    }
    encoded
}

fn parse(input: &str) -> Result<Note, String> {
    if !input.ends_with('\n') || input.contains('\r') {
        return Err("expected canonical newline-terminated scene protocol".into());
    }
    let mut lines = input.split_terminator('\n');
    let header = lines.next().ok_or("missing A1 scene header")?;
    let fields = header.split('\t').collect::<Vec<_>>();
    let interactive = match (fields[0], fields.len()) {
        ("A1SCENE1", 5) => false,
        ("A1SCENE2", 7) => true,
        _ => return Err("invalid A1 scene header".into()),
    };
    let profile = match fields[1] {
        "granted" => Profile::Granted,
        "denied" => Profile::Denied,
        _ => return Err("invalid A1 scene profile".into()),
    };
    let source_bytes: u32 = decimal(fields[2], "source length")?;
    let count: usize = decimal(fields[3], "visible heading count")?;
    let reads: u32 = decimal(fields[4], "backing reads")?;
    if source_bytes > MAX_SOURCE_BYTES || count > MAX_HEADINGS {
        return Err("scene input exceeds Markdown quotas".into());
    }
    if (profile == Profile::Denied && (source_bytes != 0 || count != 0 || reads != 0))
        || (profile == Profile::Granted && reads != 1)
    {
        return Err("scene authority/read contract violation".into());
    }
    let control = if interactive {
        let total_count: usize = decimal(fields[5], "total heading count")?;
        let expanded = match fields[6] {
            "0" => false,
            "1" => true,
            _ => return Err("invalid outline expansion state".into()),
        };
        if total_count > MAX_HEADINGS
            || count != if expanded { total_count } else { 0 }
            || (total_count == 0 && expanded)
            || (profile == Profile::Denied && total_count != 0)
        {
            return Err("invalid outline control state".into());
        }
        Some(OutlineControl {
            total_count,
            expanded,
        })
    } else {
        None
    };

    let mut headings: Vec<Heading> = Vec::with_capacity(count);
    let mut ancestors: Vec<usize> = Vec::new();
    let mut previous_end = 0;
    let mut previous_start: Option<u32> = None;
    let mut total_labels = 0usize;
    for index in 0..count {
        let line = lines.next().ok_or("scene heading count mismatch")?;
        let fields = line.split('\t').collect::<Vec<_>>();
        if fields.len() != 6 {
            return Err("invalid A1 heading row".into());
        }
        let level: u8 = decimal(fields[0], "heading level")?;
        let start: u32 = decimal(fields[1], "heading start")?;
        let end: u32 = decimal(fields[2], "heading end")?;
        let section_end: u32 = decimal(fields[3], "heading section end")?;
        let parent: Option<usize> = if fields[4] == "-1" {
            None
        } else {
            Some(decimal(fields[4], "heading parent")?)
        };
        if !(1..=6).contains(&level)
            || start < previous_end
            || previous_start.is_some_and(|prev| start <= prev)
            || end < start
            || section_end < end
            || section_end > source_bytes
        {
            return Err("invalid heading source ranges".into());
        }
        while ancestors
            .last()
            .is_some_and(|&previous| headings[previous].level >= level)
        {
            ancestors.pop();
        }
        if parent != ancestors.last().copied() {
            return Err("invalid WIT heading ancestry".into());
        }
        let text = decode_hex(fields[5])?;
        total_labels = total_labels
            .checked_add(text.len())
            .ok_or("heading text length overflow")?;
        if total_labels > MAX_LABEL_BYTES {
            return Err("heading label quota exceeded".into());
        }
        headings.push(Heading {
            level,
            start,
            end,
            _section_end: section_end,
            _parent: parent,
            text,
        });
        ancestors.push(index);
        previous_end = end;
        previous_start = Some(start);
    }
    if lines.next().is_some() {
        return Err("trailing A1 scene input".into());
    }
    Ok(Note {
        profile,
        headings,
        control,
    })
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let output = args.next().ok_or(USAGE)?;
    let hit = if output == "--hit" {
        let x: u32 = decimal(&args.next().ok_or("missing hit X")?, "hit X")?;
        let y: u32 = decimal(&args.next().ok_or("missing hit Y")?, "hit Y")?;
        Some((x, y))
    } else if matches!(output.as_str(), "--report" | "--ppm" | "--view") {
        None
    } else {
        return Err(USAGE.into());
    };
    let scroll_y: u32 = match args.next().as_deref() {
        None => 0,
        Some("--scroll-y") => decimal(&args.next().ok_or("missing scroll Y")?, "scroll Y")?,
        Some(_) => return Err("unexpected A1 scene arguments".into()),
    };
    if args.next().is_some() {
        return Err("unexpected A1 scene arguments".into());
    }

    let mut input = Vec::new();
    io::stdin()
        .lock()
        .take((MAX_WIRE_BYTES + 1) as u64)
        .read_to_end(&mut input)?;
    if input.len() > MAX_WIRE_BYTES {
        return Err("A1 scene wire input too large".into());
    }
    let input = String::from_utf8(input)?;
    let note = parse(&input).map_err(io::Error::other)?;

    let toggle_enabled = note.profile == Profile::Granted
        && note.control.is_some_and(|control| control.total_count > 0);
    let title = if toggle_enabled {
        if note.control.is_some_and(|control| control.expanded) {
            "Hide outline"
        } else {
            "Show outline"
        }
    } else {
        "Note preview"
    };
    let mut elements = vec![
        Element::new(None, ElementKind::Body, "", 0),
        Element::new(Some(0), ElementKind::Main, "", 0),
        Element::new(Some(1), ElementKind::Heading(1), title, 24),
    ];
    let mut colors = vec![Rgb::WHITE, Rgb::new(245, 245, 240), Rgb::new(54, 107, 178)];
    if note.profile == Profile::Denied {
        elements.push(Element::new(
            Some(1),
            ElementKind::Paragraph,
            "Document access denied.",
            18,
        ));
        colors.push(Rgb::new(192, 80, 80));
    } else if note.headings.is_empty() {
        let status = if note.control.is_some_and(|control| control.total_count == 0) {
            "No headings."
        } else {
            "Outline collapsed."
        };
        elements.push(Element::new(Some(1), ElementKind::Paragraph, status, 18));
        colors.push(Rgb::new(160, 160, 160));
    } else {
        elements.push(Element::new(Some(1), ElementKind::OrderedList, "", 0));
        colors.push(Rgb::WHITE);
        for (index, heading) in note.headings.iter().enumerate() {
            let mut item = Element::new(Some(3), ElementKind::ListItem, &heading.text, 18);
            let mut style = default_style(ElementKind::ListItem);
            style.padding.left = u32::from(heading.level - 1) * 8;
            item.resolved_style = Some(style);
            elements.push(item);
            colors.push(if index % 2 == 0 {
                Rgb::new(87, 164, 118)
            } else {
                Rgb::new(237, 183, 84)
            });
        }
    }
    // Widths come from actual native content boxes. Retain the measured glyph
    // plans while updating heights, then reflow before painting or hit testing.
    let initial = layout_semantic(&elements, VIEW_WIDTH, VIEW_HEIGHT)
        .map_err(|error| io::Error::other(format!("native semantic layout: {error:?}")))?;
    let mut text_plans = Vec::with_capacity(initial.text_boxes.len());
    for anchor in &initial.text_boxes {
        let plan = text::TextPlan::new(anchor.text, anchor.content.width)
            .map_err(|error| io::Error::other(format!("A1 text measurement: {error:?}")))?;
        let element = &mut elements[anchor.node];
        element.measured_text_height = element.measured_text_height.max(plan.height());
        text_plans.push((anchor.node, plan));
    }
    let semantic = layout_semantic(&elements, VIEW_WIDTH, VIEW_HEIGHT)
        .map_err(|error| io::Error::other(format!("measured native layout: {error:?}")))?;
    let paints: Vec<_> = elements
        .iter()
        .zip(colors)
        .map(|(node, color)| PaintNode {
            parent: node.parent,
            background: Some(color),
            border: None,
        })
        .collect();
    // The native painter rejects offsets beyond the document extent. Its
    // canvas also translates document-space bitmap ink by this same offset.
    let mut canvas = paint_scrolled(&semantic.frame, &paints, Rgb::WHITE, 0, scroll_y)
        .map_err(|error| io::Error::other(format!("native RGB paint: {error:?}")))?;
    for (node, plan) in text_plans {
        let content = semantic.frame.boxes[node]
            .ok_or("A1 measured text lost its layout box")?
            .content;
        plan.paint(&mut canvas, content, Rgb::BLACK)
            .map_err(|error| io::Error::other(format!("A1 text painting: {error:?}")))?;
    }

    // The opt-in session protocol keeps one stable toggle target even
    // while collapsed. Legacy scenes retain their heading-only targets.
    let targets: Vec<_> = elements
        .iter()
        .enumerate()
        .map(|(index, node)| HitNode {
            parent: node.parent,
            targetable: if note.control.is_some() {
                toggle_enabled && index == 2
            } else {
                index >= 4 && index < 4 + note.headings.len()
            },
        })
        .collect();
    let mut stdout = io::stdout().lock();
    if let Some((x, y)) = hit {
        let selected = hit_test_scrolled(&semantic.frame, &targets, x, y, 0, scroll_y)
            .map_err(|error| io::Error::other(format!("native A1 hit test: {error:?}")))?;
        if note.control.is_some() {
            writeln!(
                stdout,
                "A1HIT2\t{}",
                if selected.is_some() { "toggle" } else { "none" }
            )?;
        } else if let Some(node) = selected {
            let heading_index = node - 4;
            let heading = &note.headings[heading_index];
            writeln!(
                stdout,
                "A1HIT1\t{heading_index}\t{}\t{}\t{}",
                heading.start, heading.end, heading.level
            )?;
        } else {
            writeln!(stdout, "A1HIT1\tnone")?;
        }
    } else if output == "--ppm" {
        stdout.write_all(&canvas.ppm())?;
    } else if output == "--view" {
        let mut control_point = None;
        if toggle_enabled {
            let viewport = Rect {
                x: 0,
                y: scroll_y,
                width: VIEW_WIDTH,
                height: VIEW_HEIGHT,
            };
            let visible = semantic.frame.boxes[2]
                .ok_or("A1 control lost its layout box")?
                .border
                .intersection(viewport)
                .ok_or("A1 control viewport overflow")?;
            if visible.width > 0 && visible.height > 0 {
                let x = visible.x + visible.width / 2;
                let y = visible.y - scroll_y + visible.height / 2;
                let selected = hit_test_scrolled(&semantic.frame, &targets, x, y, 0, scroll_y)
                    .map_err(|error| {
                        io::Error::other(format!("native A1 control hit test: {error:?}"))
                    })?;
                if selected != Some(2) {
                    return Err("visible A1 control point is not targetable".into());
                }
                control_point = Some((x, y));
            }
        }
        let (control_x, control_y) = control_point.map_or_else(
            || ("none".to_owned(), "none".to_owned()),
            |(x, y)| (x.to_string(), y.to_string()),
        );
        writeln!(
            stdout,
            "A1VIEW1\t{VIEW_WIDTH}\t{VIEW_HEIGHT}\t{}\t{scroll_y}\t{control_x}\t{control_y}",
            semantic.frame.scroll_height
        )?;
    } else {
        writeln!(
            stdout,
            "A1FRAME1\t{}\t{}\t{}\t{}\t{}",
            note.profile.label(),
            canvas.width(),
            canvas.height(),
            semantic.frame.scroll_height,
            semantic.text_boxes.len()
        )?;
        for anchor in &semantic.text_boxes {
            let heading = anchor
                .node
                .checked_sub(4)
                .and_then(|i| note.headings.get(i));
            let (start, end, level) = heading.map_or((-1, -1, 0), |value| {
                (
                    i64::from(value.start),
                    i64::from(value.end),
                    i64::from(value.level),
                )
            });
            writeln!(
                stdout,
                "T\t{}\t{}\t{}\t{}\t{}\t{start}\t{end}\t{level}\t{}",
                anchor.node,
                anchor.content.x,
                anchor.content.y,
                anchor.content.width,
                anchor.content.height,
                encode_hex(anchor.text)
            )?;
        }
    }
    Ok(())
}

fn main() {
    if let Err(error) = run() {
        eprintln!("A1 native note scene: {error:#}");
        std::process::exit(1);
    }
}
