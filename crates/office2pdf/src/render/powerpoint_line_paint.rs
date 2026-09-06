//! Final paragraph-mark seating after Typst has chosen physical line breaks.
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

fn start_delta(item: &FrameItem) -> Option<(f64, bool)> {
    let Value::Array(values) = metadata(item)? else {
        return None;
    };
    let [Value::Str(name), Value::Float(delta)] = values.as_slice() else {
        return None;
    };
    ([START, MARKER].contains(&name.as_str()) && delta.is_finite())
        .then_some((*delta, name.as_str() == MARKER))
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
        _ => start_delta(item).is_some(),
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
        if let Some((delta, marker)) = start_delta(&items[index].1)
            && let Some(end) = (index + 1..items.len()).find(|&i| is_end(&items[i].1))
        {
            if !marker {
                let baselines = scope_baselines(&items[index + 1..end]);
                deltas.push_back(if baselines.len() > 1 { delta } else { 0.0 });
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

fn scope_baselines(items: &[(Point, FrameItem)]) -> Vec<f64> {
    let mut baselines = Vec::new();
    for (position, item) in items {
        collect_baselines(
            item,
            Transform::translate(position.x, position.y),
            &mut baselines,
        );
    }
    baselines.sort_by(f64::total_cmp);
    baselines.dedup_by(|left, right| (*left - *right).abs() < LINE_EPSILON_PT);
    baselines
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
        if let Some((delta, marker)) = start_delta(&items[index].1)
            && let Some(end) = (index + 1..items.len()).find(|&i| is_end(&items[i].1))
        {
            let slice = &mut items[index + 1..end];
            let delta = if marker {
                markers.pop_front().unwrap_or(0.0)
            } else {
                delta
            };
            let baselines = scope_baselines(slice);
            if marker {
                // Flat list marker and body scopes retain Typst's item order.
                // A one-line marker receives its body's first-line movement,
                // including zero for an unwrapped body.
                for (position, _) in slice {
                    position.y += Abs::pt(delta);
                }
            } else if baselines.len() > 1 && delta.abs() > LINE_EPSILON_PT {
                for (position, item) in slice {
                    move_nonfinal_paint(position, item, Transform::identity(), &baselines, delta);
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

fn move_nonfinal_paint(
    position: &mut Point,
    item: &mut FrameItem,
    parent: Transform,
    baselines: &[f64],
    delta: f64,
) {
    let at = parent.pre_concat(Transform::translate(position.x, position.y));
    let mut own_baselines = Vec::new();
    collect_baselines(item, at, &mut own_baselines);
    let last = *baselines.last().expect("a wrapped paragraph has baselines");
    let has_final = own_baselines
        .iter()
        .any(|y| (y - last).abs() < LINE_EPSILON_PT);
    let has_nonfinal = own_baselines
        .iter()
        .any(|y| (y - last).abs() >= LINE_EPSILON_PT);
    if has_final && has_nonfinal {
        if let FrameItem::Group(group) = item {
            let transform = at.pre_concat(group.transform);
            let mut children: Vec<(Point, FrameItem)> = group.frame.items().cloned().collect();
            for (position, child) in &mut children {
                move_nonfinal_paint(position, child, transform, baselines, delta);
            }
            group.frame.clear();
            group.frame.push_multiple(children);
        }
        return;
    }
    let move_item = if own_baselines.is_empty() {
        // An ungrouped underline or link rectangle belongs to the nearest
        // physical line; a link starts at the glyphs' top, so its lower edge
        // identifies that line instead of the preceding line's baseline.
        let anchor = match item {
            FrameItem::Link(_, size) => at.ty.to_pt() + size.y.to_pt() * at.sy.get(),
            _ => at.ty.to_pt(),
        };
        !matches!(item, FrameItem::Tag(_))
            && baselines
                .iter()
                .min_by(|left, right| (*left - anchor).abs().total_cmp(&(*right - anchor).abs()))
                .is_some_and(|nearest| (nearest - last).abs() >= LINE_EPSILON_PT)
    } else {
        has_nonfinal
    };
    if move_item && let Some(inverse) = parent.invert() {
        // Apply the displacement in paragraph coordinates even inside a
        // scaled or rotated text run, preserving the original transform.
        position.x += Abs::pt(delta) * inverse.kx.get();
        position.y += Abs::pt(delta) * inverse.sy.get();
    }
}
