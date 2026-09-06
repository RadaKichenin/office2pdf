//! Story-grid and paragraph-mark seating after Typst chooses physical lines.
//!
//! Typst supplies the shaping, wrapping, and line boxes. Moving its completed
//! line paint keeps those decisions intact, including decorations and links.

use std::collections::VecDeque;
use typst::foundations::Value;
use typst::introspection::{MetadataElem, Tag};
use typst::layout::{Abs, Frame, FrameItem, PagedDocument, Point, Transform};

const START: &str = "office2pdf-pptx-paragraph-mark";
const END: &str = "office2pdf-pptx-paragraph-end";
const MARKER: &str = "office2pdf-pptx-list-marker";
const LIST_START: &str = "office2pdf-pptx-list-start";
const LIST_END: &str = "office2pdf-pptx-list-end";
const LINE_EPSILON_PT: f64 = 0.001;

pub(super) fn adjust_paragraph_marks(document: &mut PagedDocument) {
    for page in &mut document.pages {
        adjust_frame(&mut page.frame);
    }
}

fn metadata(item: &FrameItem) -> Option<Value> {
    let FrameItem::Tag(Tag::Start(content, _)) = item else {
        return None;
    };
    content
        .to_packed::<MetadataElem>()
        .map(|element| element.value.clone())
}

#[derive(Clone, Copy)]
struct LineGrid {
    raw_first: f64,
    painted_first: f64,
    raw_nonfinal: f64,
}

struct SizeSeats {
    marked_em: f64,
    nonfinal_em: f64,
    advance_em: f64,
    reference_layout: f64,
    reference_marked: f64,
    reference_nonfinal: f64,
}

impl SizeSeats {
    fn layout_seat(&self, font_size: f64) -> f64 {
        (self.marked_em * font_size)
            .round()
            .clamp(0.0, self.advance_em * font_size)
    }
}

struct PhysicalLine {
    baseline: f64,
    font_size: f64,
}

struct Seating {
    nonfinal_delta: f64,
    marker: bool,
    grid: Option<LineGrid>,
    size_seats: Option<SizeSeats>,
}

impl Seating {
    fn line_deltas(&self, lines: &[PhysicalLine]) -> Vec<f64> {
        let Some(first) = lines.first() else {
            return Vec::new();
        };
        // The paragraph helper already seats an unwrapped line. In particular,
        // centered/bottom single-line layout must retain its existing seat.
        if lines.len() == 1 {
            return vec![0.0];
        }
        lines
            .iter()
            .enumerate()
            .map(|(index, line)| {
                let final_line = index + 1 == lines.len();
                if let Some(grid) = self.grid {
                    let offset = line.baseline - first.baseline;
                    let mut raw = if final_line {
                        grid.raw_first
                    } else {
                        grid.raw_nonfinal
                    };
                    let mut first_layout_shift = 0.0;
                    if let Some(seats) = &self.size_seats {
                        let (seat_em, reference_raw) = if final_line {
                            (seats.marked_em, seats.reference_marked)
                        } else {
                            (seats.nonfinal_em, seats.reference_nonfinal)
                        };
                        let layout_seat = seats.layout_seat(line.font_size);
                        first_layout_shift =
                            seats.layout_seat(first.font_size) - seats.reference_layout;
                        // Mixed-size runs supply their own integer layout
                        // seats. Restore this line's residual, rather than
                        // carrying the paragraph's largest-font residual.
                        raw += seat_em * line.font_size
                            - layout_seat
                            - (reference_raw - seats.reference_layout)
                            + first_layout_shift;
                    }
                    // Preserve the unsnapped phase: rounding a line offset
                    // after rounding its first baseline loses the carry.
                    (raw + offset).round() - grid.painted_first - offset - first_layout_shift
                } else if final_line {
                    0.0
                } else {
                    self.nonfinal_delta
                }
            })
            .collect()
    }
}

fn finite_number(value: &Value) -> Option<f64> {
    let number = match value {
        Value::Float(number) => *number,
        Value::Int(number) => *number as f64,
        _ => return None,
    };
    number.is_finite().then_some(number)
}

fn finite_array<const N: usize>(value: &Value) -> Option<[f64; N]> {
    let Value::Array(values) = value else {
        return None;
    };
    if values.len() != N {
        return None;
    }
    let mut numbers = [0.0; N];
    for (number, value) in numbers.iter_mut().zip(values.iter()) {
        *number = finite_number(value)?;
    }
    Some(numbers)
}

fn start_seating(item: &FrameItem) -> Option<Seating> {
    let Value::Array(values) = metadata(item)? else {
        return None;
    };
    let [Value::Str(name), delta, options @ ..] = values.as_slice() else {
        return None;
    };
    if ![START, MARKER].contains(&name.as_str()) || options.len() > 2 {
        return None;
    }
    let nonfinal_delta = finite_number(delta)?;
    let grid = match options.first() {
        None | Some(Value::None) => None,
        Some(value) => {
            let [raw_first, painted_first, raw_nonfinal] = finite_array(value)?;
            Some(LineGrid {
                raw_first,
                painted_first,
                raw_nonfinal,
            })
        }
    };
    let size_seats = match options.get(1) {
        None | Some(Value::None) => None,
        Some(value) => {
            let [
                marked_em,
                nonfinal_em,
                advance_em,
                reference_layout,
                reference_marked,
                reference_nonfinal,
            ] = finite_array(value)?;
            if marked_em < 0.0 || nonfinal_em < 0.0 || advance_em < 0.0 {
                return None;
            }
            Some(SizeSeats {
                marked_em,
                nonfinal_em,
                advance_em,
                reference_layout,
                reference_marked,
                reference_nonfinal,
            })
        }
    };
    Some(Seating {
        nonfinal_delta,
        marker: name.as_str() == MARKER,
        grid,
        size_seats,
    })
}

fn is_end(item: &FrameItem) -> bool {
    named_tag(item, END)
}

fn named_tag(item: &FrameItem, expected: &str) -> bool {
    matches!(metadata(item), Some(Value::Str(name)) if name.as_str() == expected)
}

fn contains_mark_scope(frame: &Frame) -> bool {
    frame.items().any(|(_, item)| match item {
        FrameItem::Group(group) => contains_mark_scope(&group.frame),
        _ => start_seating(item).is_some(),
    })
}

fn adjust_frame(frame: &mut Frame) {
    adjust_frame_with_markers(frame, &mut VecDeque::new());
}

fn adjust_frame_with_markers(frame: &mut Frame, markers: &mut VecDeque<f64>) {
    if !contains_mark_scope(frame) {
        return;
    }
    let mut items: Vec<(Point, FrameItem)> = frame.items().cloned().collect();
    adjust_items(&mut items, markers);
    frame.clear();
    frame.push_multiple(items);
}

fn body_deltas(items: &[(Point, FrameItem)], deltas: &mut VecDeque<f64>) {
    let mut index = 0;
    while index < items.len() {
        if let Some(seating) = start_seating(&items[index].1)
            && let Some(end) = (index + 1..items.len()).find(|&i| is_end(&items[i].1))
        {
            if !seating.marker {
                let lines = scope_lines(&items[index + 1..end]);
                deltas.push_back(seating.line_deltas(&lines).first().copied().unwrap_or(0.0));
            }
            index = end + 1;
            continue;
        }
        if let FrameItem::Group(group) = &items[index].1 {
            let children: Vec<_> = group.frame.items().cloned().collect();
            body_deltas(&children, deltas);
        }
        index += 1;
    }
}

fn scope_lines(items: &[(Point, FrameItem)]) -> Vec<PhysicalLine> {
    fn collect(item: &FrameItem, transform: Transform, lines: &mut Vec<PhysicalLine>) {
        match item {
            FrameItem::Text(text) => lines.push(PhysicalLine {
                baseline: transform.ty.to_pt(),
                font_size: text.size.to_pt() * transform.sy.get().abs(),
            }),
            FrameItem::Group(group) => {
                let transform = transform.pre_concat(group.transform);
                for (position, child) in group.frame.items() {
                    collect(
                        child,
                        transform.pre_concat(Transform::translate(position.x, position.y)),
                        lines,
                    );
                }
            }
            _ => {}
        }
    }

    let mut fragments = Vec::new();
    for (position, item) in items {
        collect(
            item,
            Transform::translate(position.x, position.y),
            &mut fragments,
        );
    }
    fragments.sort_by(|a, b| a.baseline.total_cmp(&b.baseline));
    let mut lines: Vec<PhysicalLine> = Vec::new();
    for fragment in fragments {
        if let Some(line) = lines.last_mut()
            && (line.baseline - fragment.baseline).abs() < LINE_EPSILON_PT
        {
            line.font_size = line.font_size.max(fragment.font_size);
        } else {
            lines.push(fragment);
        }
    }
    lines
}

fn adjust_items(items: &mut [(Point, FrameItem)], markers: &mut VecDeque<f64>) {
    let mut index = 0;
    while index < items.len() {
        if named_tag(&items[index].1, LIST_START)
            && let Some(end) = (index + 1..items.len()).find(|&i| named_tag(&items[i].1, LIST_END))
        {
            let slice = &mut items[index + 1..end];
            let mut list_markers = VecDeque::new();
            body_deltas(slice, &mut list_markers);
            adjust_items(slice, &mut list_markers);
            index = end + 1;
            continue;
        }
        if let Some(seating) = start_seating(&items[index].1)
            && let Some(end) = (index + 1..items.len()).find(|&i| is_end(&items[i].1))
        {
            let slice = &mut items[index + 1..end];
            let lines = scope_lines(slice);
            if seating.marker {
                let delta = markers.pop_front().unwrap_or(0.0);
                // Flat list marker and body scopes retain Typst's item order.
                // A one-line marker receives its body's first-line movement,
                // including zero for an unwrapped body.
                for (position, _) in slice {
                    position.y += Abs::pt(delta);
                }
            } else if lines.len() > 1 {
                let deltas = seating.line_deltas(&lines);
                let baselines: Vec<f64> = lines.iter().map(|line| line.baseline).collect();
                for (position, item) in slice {
                    move_line_paint(position, item, Transform::identity(), &baselines, &deltas);
                }
            }
            index = end + 1;
            continue;
        }
        if let FrameItem::Group(group) = &mut items[index].1 {
            adjust_frame_with_markers(&mut group.frame, markers);
        }
        index += 1;
    }
}

fn collect_baselines(item: &FrameItem, transform: Transform, baselines: &mut Vec<f64>) {
    match item {
        FrameItem::Text(_) => baselines.push(transform.ty.to_pt()),
        FrameItem::Group(group) => {
            let transform = transform.pre_concat(group.transform);
            for (position, child) in group.frame.items() {
                collect_baselines(
                    child,
                    transform.pre_concat(Transform::translate(position.x, position.y)),
                    baselines,
                );
            }
        }
        _ => {}
    }
}

fn move_line_paint(
    position: &mut Point,
    item: &mut FrameItem,
    parent: Transform,
    baselines: &[f64],
    deltas: &[f64],
) {
    let at = parent.pre_concat(Transform::translate(position.x, position.y));
    let mut own_baselines = Vec::new();
    collect_baselines(item, at, &mut own_baselines);
    let spans_lines = own_baselines.first().is_some_and(|first| {
        own_baselines
            .iter()
            .any(|y| (y - first).abs() >= LINE_EPSILON_PT)
    });
    if spans_lines {
        if let FrameItem::Group(group) = item {
            let transform = at.pre_concat(group.transform);
            let mut children: Vec<(Point, FrameItem)> = group.frame.items().cloned().collect();
            for (position, child) in &mut children {
                move_line_paint(position, child, transform, baselines, deltas);
            }
            group.frame.clear();
            group.frame.push_multiple(children);
        }
        return;
    }
    if matches!(item, FrameItem::Tag(_)) {
        return;
    }
    let anchor = if let Some(&baseline) = own_baselines.first() {
        baseline
    } else {
        // An ungrouped underline or link rectangle belongs to the nearest
        // physical line; a link starts at the glyphs' top, so its lower edge
        // identifies that line instead of the preceding line's baseline.
        match item {
            FrameItem::Link(_, size) => at.ty.to_pt() + size.y.to_pt() * at.sy.get(),
            _ => at.ty.to_pt(),
        }
    };
    let delta = baselines
        .iter()
        .zip(deltas)
        .min_by(|(left, _), (right, _)| (*left - anchor).abs().total_cmp(&(*right - anchor).abs()))
        .map_or(0.0, |(_, &delta)| delta);
    if delta.abs() > LINE_EPSILON_PT
        && let Some(inverse) = parent.invert()
    {
        // Apply the displacement in paragraph coordinates even inside a
        // scaled or rotated text run, preserving the original transform.
        position.x += Abs::pt(delta) * inverse.kx.get();
        position.y += Abs::pt(delta) * inverse.sy.get();
    }
}
