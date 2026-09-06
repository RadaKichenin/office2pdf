//! Excel positive-axis backgrounds in the original table fill layer.
//!
//! Expanding completed cell rectangles keeps automatic row sizes, merges, and
//! page fragments intact. Backgrounds remain below borders and cell content.

use std::collections::HashMap;
use typst::introspection::Tag;
use typst::layout::{Abs, Frame, FrameItem, PagedDocument, Point};
use typst::model::TableElem;
use typst::syntax::Span;
use typst::visualize::Geometry;

pub(super) fn adjust_cell_fills(document: &mut PagedDocument) {
    let mut tables = HashMap::new();
    for page in &document.pages {
        collect_tables(&page.frame, &mut tables);
    }
    if tables.is_empty() {
        return;
    }
    tracing::debug!(
        tables = tables.len(),
        "applying Excel cell background extents"
    );
    for page in &mut document.pages {
        adjust_frame(&mut page.frame, &tables);
    }
}

fn collect_tables(frame: &Frame, tables: &mut HashMap<Span, f64>) {
    for (_, item) in frame.items() {
        match item {
            FrameItem::Group(group) => collect_tables(&group.frame, tables),
            FrameItem::Tag(Tag::Start(content, _))
                if content.to_packed::<TableElem>().is_some() =>
            {
                let Some(label) = content.label() else {
                    continue;
                };
                let text = label.resolve();
                if !text.starts_with("o2p-excel-fill-") {
                    continue;
                }
                let Some(scale) = text.rsplit('-').next().and_then(|s| s.parse::<f64>().ok())
                else {
                    continue;
                };
                if scale.is_finite() && scale > 0.0 && scale <= 1.0 {
                    tables.insert(content.span(), scale);
                }
            }
            _ => {}
        }
    }
}

fn adjust_frame(frame: &mut Frame, tables: &HashMap<Span, f64>) {
    let mut items: Vec<(Point, FrameItem)> = frame.items().cloned().collect();
    let mut table_slots: HashMap<Span, Vec<usize>> = HashMap::new();
    for (index, (_, item)) in items.iter_mut().enumerate() {
        match item {
            FrameItem::Group(group) => adjust_frame(&mut group.frame, tables),
            FrameItem::Shape(shape, span) if shape.fill.is_some() && shape.stroke.is_none() => {
                let Some(scale) = tables.get(span) else {
                    continue;
                };
                let Geometry::Rect(size) = &mut shape.geometry else {
                    continue;
                };
                size.x += Abs::pt(*scale);
                size.y += Abs::pt(*scale);
                table_slots.entry(*span).or_default().push(index);
            }
            _ => {}
        }
    }
    // Typst groups fills by column. Excel paints row-first, so a later row
    // owns the shared lower-left corner even beside a horizontal merge.
    for slots in table_slots.values() {
        let mut fills: Vec<_> = slots.iter().map(|i| items[*i].clone()).collect();
        fills.sort_by(|a, b| {
            a.0.y
                .to_pt()
                .total_cmp(&b.0.y.to_pt())
                .then_with(|| a.0.x.to_pt().total_cmp(&b.0.x.to_pt()))
        });
        for (index, fill) in slots.iter().zip(fills) {
            items[*index] = fill;
        }
    }
    frame.clear();
    frame.push_multiple(items);
}
