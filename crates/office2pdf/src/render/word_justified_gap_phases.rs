//! Word's three-phase spread of a justified line's stretch demand, applied
//! after Typst has chosen and stretched the lines (issue #1280).
//!
//! Word spends a justified line's demand in three phases (measured over
//! demands from 6.2pt to 300.2pt, issue #1053): the word spaces fill to half
//! an em, then the East Asian/Latin auto spaces take the remainder, then
//! every expandable gap sits at one common width. Typst spends it in one
//! pass — every gap grows by its own stretchability times a single ratio
//! shared by the whole line, and once that is spent, by an equal extra — so
//! with both kinds of gap stating the same half-em ceiling (issue #1193) the
//! third phase comes out exact and the first two do not: an ordinary
//! justified line, whose demand never reaches the ceiling, moved its auto
//! spaces from the first point of demand, 0.27pt per gap on the fixture line
//! this issue measured.
//!
//! Typst's line breaks, shaping and line boxes are kept. Only the widths of
//! the space glyphs on such a line change, and everything after each one
//! moves by the difference; the line's width is conserved, so nothing else
//! on the page sees the change.

use std::ops::Range;

use typst::foundations::Value;
use typst::introspection::{MetadataElem, Tag};
use typst::layout::{Abs, Em, Frame, FrameItem, PagedDocument, Point};
use typst::visualize::Geometry;

use super::typst_gen::{
    EAST_ASIAN_AUTO_SPACE_EM, EAST_ASIAN_AUTO_SPACE_GLYPH, EAST_ASIAN_JUSTIFIED_GAP_CEILING_EM,
    EAST_ASIAN_JUSTIFIED_GAP_RATIO_TERM_PERCENT,
};

/// The metadata value codegen places immediately before every East
/// Asian/Latin auto space, so the completed frame can tell the auto space
/// from a word space (both are space glyphs to Typst).
pub(crate) const AUTO_SPACE_MARKER: &str = "office2pdf-docx-auto-space";

/// Positions and widths closer than this are the same edge.
const EDGE_EPSILON_PT: f64 = 1e-4;

/// Which of Word's two gap kinds an expandable gap is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum GapKind {
    /// A literal space between words.
    WordSpace,
    /// The automatic space at an East Asian/Latin boundary.
    AutoSpace,
}

/// One expandable gap of a justified line, in points.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct GapBudget {
    pub kind: GapKind,
    /// The width the gap has on an unstretched line.
    pub natural: f64,
    /// The width Word lets the gap reach before it levels every gap on the
    /// line to one common width.
    pub ceiling: f64,
}

/// The widths Word gives `gaps` when the line needs `demand` points more
/// than its natural width.
///
/// Each phase fills its gaps by equal amounts until one reaches its ceiling,
/// then goes on with the rest — at one size that is Word's equal split, and
/// it keeps a smaller face's gap from overshooting its own half em on a line
/// that mixes sizes. What is left past every ceiling is added equally to
/// every gap, which is also what Typst does past the ceiling, so a line in
/// phase 3 comes out exactly as Typst already laid it.
pub(super) fn spread_demand_as_word_does(gaps: &[GapBudget], demand: f64) -> Vec<f64> {
    let mut widths: Vec<f64> = gaps.iter().map(|gap| gap.natural).collect();
    let mut remaining: f64 = demand.max(0.0);
    for phase in [GapKind::WordSpace, GapKind::AutoSpace] {
        let members: Vec<usize> = (0..gaps.len())
            .filter(|&index| gaps[index].kind == phase)
            .collect();
        remaining = fill_equally(gaps, &mut widths, &members, remaining);
    }
    if remaining > 0.0 && !gaps.is_empty() {
        let extra: f64 = remaining / gaps.len() as f64;
        for width in &mut widths {
            *width += extra;
        }
    }
    widths
}

/// Raise `members` by equal amounts, each stopping at its ceiling, until
/// `amount` is spent; return what could not be placed.
fn fill_equally(gaps: &[GapBudget], widths: &mut [f64], members: &[usize], amount: f64) -> f64 {
    let mut remaining: f64 = amount;
    let mut open: Vec<usize> = members
        .iter()
        .copied()
        .filter(|&index| gaps[index].ceiling > widths[index])
        .collect();
    while remaining > 0.0 && !open.is_empty() {
        let share: f64 = remaining / open.len() as f64;
        // The gap with the least room decides how far this round can go
        // before somebody's ceiling is crossed.
        let step: f64 = open
            .iter()
            .map(|&index| gaps[index].ceiling - widths[index])
            .fold(share, f64::min);
        for &index in &open {
            widths[index] += step;
        }
        remaining -= step * open.len() as f64;
        open.retain(|&index| gaps[index].ceiling - widths[index] > EDGE_EPSILON_PT);
        if step <= 0.0 {
            break;
        }
    }
    remaining.max(0.0)
}

/// Re-spread every stretched justified line that carries the auto space.
pub(super) fn spread_justified_gaps_as_word_does(document: &mut PagedDocument) {
    for page in &mut document.pages {
        adjust_frame(&mut page.frame);
    }
}

fn is_auto_space_marker(item: &FrameItem) -> bool {
    let FrameItem::Tag(Tag::Start(content, _)) = item else {
        return false;
    };
    content
        .to_packed::<MetadataElem>()
        .is_some_and(|element| matches!(&element.value, Value::Str(value) if value.as_str() == AUTO_SPACE_MARKER))
}

fn contains_auto_space_marker(frame: &Frame) -> bool {
    frame.items().any(|(_, item)| match item {
        FrameItem::Group(group) => contains_auto_space_marker(&group.frame),
        _ => is_auto_space_marker(item),
    })
}

fn adjust_frame(frame: &mut Frame) {
    if !contains_auto_space_marker(frame) {
        return;
    }
    let mut items: Vec<(Point, FrameItem)> = frame.items().cloned().collect();
    for (_, item) in &mut items {
        if let FrameItem::Group(group) = item {
            adjust_frame(&mut group.frame);
        }
    }
    for line in line_ranges(&items) {
        re_spread_line(&mut items[line]);
    }
    frame.clear();
    frame.push_multiple(items);
}

/// Where an inline item's baseline sits, for the kinds Typst's line builder
/// seats on the line's baseline. Decorations and links sit over the text they
/// belong to and say nothing about the line themselves.
fn inline_baseline(position: Point, item: &FrameItem) -> Option<Abs> {
    match item {
        FrameItem::Text(_) | FrameItem::Tag(_) => Some(position.y),
        FrameItem::Group(group) => Some(position.y + group.frame.baseline()),
        FrameItem::Image(_, size, _) => Some(position.y + size.y),
        FrameItem::Shape(..) | FrameItem::Link(..) => None,
    }
}

fn item_width(item: &FrameItem) -> Abs {
    match item {
        FrameItem::Text(text) => text.width(),
        FrameItem::Group(group) => group.frame.width(),
        FrameItem::Image(_, size, _) | FrameItem::Link(_, size) => size.x,
        FrameItem::Shape(shape, _) => match &shape.geometry {
            Geometry::Line(to) => to.x.max(Abs::zero()),
            Geometry::Rect(size) => size.x,
            Geometry::Curve(curve) => curve.bbox_size().x,
        },
        FrameItem::Tag(_) => Abs::zero(),
    }
}

/// The contiguous item ranges that make up one line each.
///
/// Typst pushes a line's items in logical order, each seated after the one
/// before it, so a line reads left to right without overlap and the next
/// line starts again at the left on another baseline. Either sign — an item
/// starting well before the previous one ends, or a baseline well away from
/// the previous one — opens a new line; a negative spacer or a shifted
/// superscript moves by less than half an em and stays on its line.
fn line_ranges(items: &[(Point, FrameItem)]) -> Vec<Range<usize>> {
    let mut ranges: Vec<Range<usize>> = Vec::new();
    let mut start: usize = 0;
    let mut previous: Option<(Abs, Abs, Abs)> = None;
    for (index, (position, item)) in items.iter().enumerate() {
        let Some(baseline) = inline_baseline(*position, item) else {
            continue;
        };
        let size: Abs = match item {
            FrameItem::Text(text) => text.size,
            _ => previous.map_or(Abs::pt(10.0), |(_, _, size)| size),
        };
        if let Some((right, previous_baseline, previous_size)) = previous {
            let tolerance: Abs = previous_size * 0.6;
            let starts_over: bool = position.x < right - tolerance;
            let leaves_the_baseline: bool = (baseline - previous_baseline).abs() > tolerance;
            if starts_over || leaves_the_baseline {
                ranges.push(start..index);
                start = index;
            }
        }
        previous = Some((position.x + item_width(item), baseline, size));
    }
    ranges.push(start..items.len());
    ranges
}

fn is_space_char(character: char) -> bool {
    // Typst's own `is_space`: the three characters its justifier stretches.
    matches!(character, ' ' | '\u{00A0}' | '\u{3000}')
}

/// One space glyph of the line as Typst left it.
struct Gap {
    item: usize,
    glyph: usize,
    kind: GapKind,
    size: Abs,
    left: Abs,
    width: Abs,
}

impl Gap {
    fn right(&self) -> Abs {
        self.left + self.width
    }
}

/// Every stretched space glyph on the line, left to right. `None` for a
/// line this pass does not read: vertical text.
fn line_gaps(items: &[(Point, FrameItem)]) -> Option<Vec<Gap>> {
    let mut gaps: Vec<Gap> = Vec::new();
    let mut marked: bool = false;
    for (index, (position, item)) in items.iter().enumerate() {
        let FrameItem::Text(text) = item else {
            if matches!(item, FrameItem::Tag(_)) {
                marked |= is_auto_space_marker(item);
            } else {
                marked = false;
            }
            continue;
        };
        let is_auto_space: bool =
            marked && text.text.as_str() == EAST_ASIAN_AUTO_SPACE_GLYPH && text.glyphs.len() == 1;
        marked = false;
        let mut cursor: Abs = position.x;
        for (glyph_index, glyph) in text.glyphs.iter().enumerate() {
            if glyph.y_advance != Em::zero() {
                return None;
            }
            let width: Abs = glyph.x_advance.at(text.size);
            let characters: &str = &text.text[glyph.range()];
            if !characters.is_empty()
                && characters.chars().all(is_space_char)
                && width > Abs::zero()
            {
                gaps.push(Gap {
                    item: index,
                    glyph: glyph_index,
                    kind: if is_auto_space {
                        GapKind::AutoSpace
                    } else {
                        GapKind::WordSpace
                    },
                    size: text.size,
                    left: cursor,
                    width,
                });
            }
            cursor += width;
        }
    }
    Some(gaps)
}

/// The ratio term of the ceiling codegen states, as a fraction.
fn ratio_term() -> f64 {
    EAST_ASIAN_JUSTIFIED_GAP_RATIO_TERM_PERCENT / 100.0
}

/// How far Typst may stretch a gap of `natural` width under that ceiling:
/// `(max - 100%)` relative to the gap's own width, with
/// `max = ratio_term + ceiling_em * em`.
fn typst_room_pt(natural_pt: f64, size_pt: f64) -> f64 {
    (EAST_ASIAN_JUSTIFIED_GAP_CEILING_EM * size_pt + natural_pt * (ratio_term() - 1.0)).max(0.0)
}

/// The share of its room every gap on the line took, read off the auto
/// spaces, whose natural width codegen fixed at a quarter em of their own
/// size. `None` when the line is not one this pass changes: unstretched or
/// compressed, past every ceiling (phase 3, which Typst already has exactly),
/// or with auto spaces that disagree, which means a limit other than the
/// codegen ceiling was in force.
fn typst_stretch_ratio(gaps: &[Gap]) -> Option<f64> {
    let mut ratio: Option<f64> = None;
    for gap in gaps.iter().filter(|gap| gap.kind == GapKind::AutoSpace) {
        let size_pt: f64 = gap.size.to_pt();
        let natural_pt: f64 = EAST_ASIAN_AUTO_SPACE_EM * size_pt;
        let room_pt: f64 = typst_room_pt(natural_pt, size_pt);
        if room_pt <= 0.0 {
            return None;
        }
        let this: f64 = (gap.width.to_pt() - natural_pt) / room_pt;
        match ratio {
            Some(known) if (known - this).abs() > 1e-6 => return None,
            Some(_) => {}
            None => ratio = Some(this),
        }
    }
    let ratio: f64 = ratio?;
    (ratio > 1e-9 && ratio < 1.0 - 1e-9).then_some(ratio)
}

/// The width a word space had before Typst stretched it by `ratio` of its
/// room under the ceiling — the inverse of the stretch, so a tracked or
/// kerned space reads back as itself. `None` for a width no space under
/// this ceiling could have reached.
fn word_space_natural_pt(width_pt: f64, size_pt: f64, ratio: f64) -> Option<f64> {
    let ceiling_pt: f64 = EAST_ASIAN_JUSTIFIED_GAP_CEILING_EM * size_pt;
    let natural_pt: f64 = (width_pt - ratio * ceiling_pt) / (1.0 - ratio * (1.0 - ratio_term()));
    (natural_pt > 0.0 && natural_pt < ceiling_pt && natural_pt <= width_pt + EDGE_EPSILON_PT)
        .then_some(natural_pt)
}

fn re_spread_line(items: &mut [(Point, FrameItem)]) {
    let Some(gaps) = line_gaps(items) else {
        return;
    };
    let Some(ratio) = typst_stretch_ratio(&gaps) else {
        return;
    };

    let mut budgets: Vec<GapBudget> = Vec::with_capacity(gaps.len());
    for gap in &gaps {
        let size_pt: f64 = gap.size.to_pt();
        let ceiling: f64 = EAST_ASIAN_JUSTIFIED_GAP_CEILING_EM * size_pt;
        let natural: f64 = match gap.kind {
            GapKind::AutoSpace => EAST_ASIAN_AUTO_SPACE_EM * size_pt,
            GapKind::WordSpace => {
                let Some(natural) = word_space_natural_pt(gap.width.to_pt(), size_pt, ratio) else {
                    return;
                };
                natural
            }
        };
        budgets.push(GapBudget {
            kind: gap.kind,
            natural,
            ceiling,
        });
    }
    let demand: f64 = gaps
        .iter()
        .zip(&budgets)
        .map(|(gap, budget)| gap.width.to_pt() - budget.natural)
        .sum();
    let targets: Vec<f64> = spread_demand_as_word_does(&budgets, demand);

    // Everything is decided against the positions Typst left before any of
    // them moves: an item after a gap moves by the gap's change, and a
    // decoration or link drawn across a gap widens by it.
    let mut shifts: Vec<Abs> = vec![Abs::zero(); items.len()];
    let mut widenings: Vec<Abs> = vec![Abs::zero(); items.len()];
    for (gap, target) in gaps.iter().zip(&targets) {
        let delta: Abs = Abs::pt(*target) - gap.width;
        if delta.abs() <= Abs::pt(EDGE_EPSILON_PT) {
            continue;
        }
        for (index, (position, item)) in items.iter().enumerate() {
            if index == gap.item {
                continue;
            }
            let left: Abs = position.x;
            let right: Abs = left + item_width(item);
            if left >= gap.right() - Abs::pt(EDGE_EPSILON_PT) {
                shifts[index] += delta;
            } else if right > gap.left + Abs::pt(EDGE_EPSILON_PT) {
                match item {
                    FrameItem::Shape(..) | FrameItem::Link(..) => widenings[index] += delta,
                    // Another advancing item overlapping the gap is not a line
                    // this pass understands; leave Typst's spread alone.
                    _ => return,
                }
            }
        }
    }

    for (gap, target) in gaps.iter().zip(&targets) {
        let FrameItem::Text(text) = &mut items[gap.item].1 else {
            unreachable!("a gap is always read from a text item");
        };
        text.glyphs[gap.glyph].x_advance = Em::from_abs(Abs::pt(*target), text.size);
    }
    for (index, (position, item)) in items.iter_mut().enumerate() {
        position.x += shifts[index];
        let widening: Abs = widenings[index];
        if widening.abs() <= Abs::pt(EDGE_EPSILON_PT) {
            continue;
        }
        match item {
            FrameItem::Link(_, size) => size.x += widening,
            FrameItem::Shape(shape, _) => match &mut shape.geometry {
                Geometry::Line(to) => to.x += widening,
                Geometry::Rect(size) => size.x += widening,
                // A curved decoration keeps its outline; the gap under it is
                // sub-point and it is not the text.
                Geometry::Curve(_) => {}
            },
            _ => {}
        }
    }
}

#[cfg(test)]
#[path = "word_justified_gap_phases_tests.rs"]
mod tests;
