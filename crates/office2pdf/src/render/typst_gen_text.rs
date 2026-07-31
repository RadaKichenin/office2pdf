use std::fmt::Write;

use unicode_normalization::UnicodeNormalization;

use crate::render::font_subst;

use super::*;

/// Word's default tab stop interval (0.5 inch = 36pt).
pub(super) const DEFAULT_TAB_WIDTH_PT: f64 = 36.0;
/// East Asian Word's default tab stop (800 twips) when settings.xml omits
/// `w:defaultTabStop`.
pub(super) const EAST_ASIAN_DEFAULT_TAB_WIDTH_PT: f64 = 40.0;
const PPTX_SOFT_LINE_BREAK_CHAR: char = '\u{000B}';
/// In-text marker the PPTX parser places between a Hangul syllable and
/// following terminal punctuation (issue #438); never emitted literally.
const HANGUL_KINSOKU_BREAK_CHAR: char = '\u{200B}';
/// In-text marker the DOCX parser places at an East Asian/Latin boundary that
/// carries no literal space (issue #521); never emitted literally.
const EAST_ASIAN_AUTO_SPACE_CHAR: char = '\u{E001}';
/// Word's automatic space at such a boundary, as a fraction of the run's font
/// size. Measured as exactly a quarter em on a native export at two sizes.
const EAST_ASIAN_AUTO_SPACE_EM: f64 = 0.25;

/// The auto space sized against the *run*, not the paragraph. It is emitted
/// between the run's `#text(size:)` calls rather than inside one, so an `em`
/// there would resolve against the paragraph's default size instead — 11pt
/// where the run is 10.5pt, which is 0.12pt too wide at every boundary.
fn east_asian_auto_space(run: &Run) -> String {
    match run.style.font_size {
        Some(size) => format!("#h({}pt)", format_f64(size * EAST_ASIAN_AUTO_SPACE_EM)),
        None => format!("#h({EAST_ASIAN_AUTO_SPACE_EM}em)"),
    }
}

pub(super) fn generate_paragraph(
    out: &mut String,
    para: &Paragraph,
    line_grid_pitch: Option<f64>,
    default_tab_width_pt: f64,
    breaks_hangul_at_eojeol: bool,
    available_measure_pt: Option<f64>,
) -> Result<(), ConvertError> {
    let style = &para.style;

    if let Some(level) = style.heading_level {
        // A heading is still a paragraph: Word paints its `w:pBdr` and `w:shd`
        // around it exactly as it does around body copy, and a chapter-rule
        // heading style is the commonest place a `w:pBdr` appears at all.
        // Returning here before any decoration was emitted dropped every one
        // of them — 22 chapter rules in the technical-brief fixture, while the
        // header rule on the same page, declared directly rather than through
        // a style, survived (issue #581).
        //
        // The wrapper opens only when there is decoration to carry, so an
        // undecorated heading keeps Typst's own block spacing rather than
        // inheriting a `#block`'s.
        let decorated = style.background.is_some() || style.border.is_some();
        if decorated {
            out.push_str("#block(width: 100%");
            write_block_spacing_params(out, style);
            write_block_decoration_params(out, style);
            out.push_str(")[\n");
            write_paragraph_double_border_overlays(
                out,
                &style.border,
                style.border_space.as_deref().copied().unwrap_or_default(),
            );
        }
        // A contents entry is laid out as body text, not as a copy of the
        // heading, so it cannot be built from the heading's rendered content —
        // the size and weight are inline markup inside it and no enclosing set
        // rule beats them. Drop the heading's plain text under a label instead
        // and let the list style it (issue #610), the same shape the caption
        // lists already use.
        let plain: String = paragraph_plain_text(&para.runs);
        let _ = writeln!(
            out,
            "#metadata((level: {level}, text: \"{}\", font: {}))<{}>",
            escape_typst_string(&plain),
            crate::render::font_subst::font_with_fallbacks_for_text(
                first_run_family(&para.runs).unwrap_or("Calibri"),
                &plain,
            ),
            TOC_ENTRY_LABEL
        );
        let _ = write!(out, "#heading(level: {level})[");
        generate_runs_with_tabs(
            out,
            &para.runs,
            style.tab_stops.as_deref(),
            default_tab_width_pt,
            // A heading emits no fixed text edges of its own, so a frame needs
            // no correction to sit on the surrounding baseline.
            paragraph_eojeol_wrap(breaks_hangul_at_eojeol, style, None, available_measure_pt),
        );
        out.push_str("]\n");
        if decorated {
            out.push_str("]\n");
        }
        return Ok(());
    }

    let line_height_settings: Option<String> =
        word_line_height_settings(&para.runs, style, line_grid_pitch);
    let has_para_style = needs_block_wrapper(style) || line_height_settings.is_some();

    // Word's `w:ind` offsets the paragraph's whole column, and paints
    // `w:shd` and `w:pBdr` from the indent rather than the margin, so the
    // indent goes on an outer block as an inset and the fill and border stay
    // on an inner block that spans only the inset content area (issue #464).
    let indent = paragraph_indent_pt(style);
    if indent.is_some() {
        out.push_str("#block(width: 100%");
        write_block_spacing_params(out, style);
        write_paragraph_indent_inset(out, indent);
        out.push_str(")[\n");
    }

    if has_para_style {
        // The wrapper must span the full line width: Typst blocks shrink to
        // their content by default, which would defeat the inner #align.
        // Word measures `w:spacing w:before/w:after` from the edges of the
        // full line box, which `word_line_height_settings` spans directly,
        // so those gaps reach the block unmodified (issues #394, #452).
        out.push_str("#block(width: 100%");
        if indent.is_none() {
            write_block_spacing_params(out, style);
        }
        write_block_decoration_params(out, style);
        out.push_str(")[\n");
        write_paragraph_double_border_overlays(
            out,
            &style.border,
            style.border_space.as_deref().copied().unwrap_or_default(),
        );
        write_line_box_settings(out, style.line_box);
        write_par_settings(out, style);
        if let Some(ref settings) = line_height_settings {
            out.push_str(settings);
        }
    }

    if para.runs.is_empty() {
        out.push_str("#v(12pt)");
        if has_para_style {
            out.push_str("\n]");
        }
        if indent.is_some() {
            out.push_str("\n]");
        }
        out.push('\n');
        return Ok(());
    }

    let alignment = style.alignment;
    let use_align = matches!(
        alignment,
        Some(Alignment::Center) | Some(Alignment::Right) | Some(Alignment::Left)
    );

    if use_align {
        let align_str = match alignment {
            Some(Alignment::Left) => "left",
            Some(Alignment::Center) => "center",
            Some(Alignment::Right) => "right",
            _ => "left",
        };
        let _ = write!(out, "#align({align_str})[");
    }

    // Whichever fixed line box the wrapper above put in force — the computed
    // Word line, or the paragraph's own `LineBox` — is what a framed eojeol
    // has to restore inside itself. The two are mutually exclusive:
    // `word_line_leading_pt` bails on a paragraph that declares a `LineBox`.
    let line_box_em: Option<(f64, f64)> = has_para_style
        .then(|| {
            word_line_box_em(&para.runs, style, line_grid_pitch).or_else(|| {
                style
                    .line_box
                    .map(|line_box| (line_box.ascent_em, line_box.descent_em))
            })
        })
        .flatten();

    generate_runs_with_tabs(
        out,
        &para.runs,
        style.tab_stops.as_deref(),
        default_tab_width_pt,
        paragraph_eojeol_wrap(
            breaks_hangul_at_eojeol,
            style,
            line_box_em,
            available_measure_pt,
        ),
    );

    if use_align {
        out.push(']');
    }

    if has_para_style {
        out.push_str("\n]");
    }
    if indent.is_some() {
        out.push_str("\n]");
    }

    out.push('\n');
    Ok(())
}

/// The paragraph's `(left, right)` indent in points, or `None` when it has
/// neither. Negative indents — Word lets a paragraph hang into the margin —
/// are clamped to zero, because a Typst inset cannot be negative.
fn paragraph_indent_pt(style: &ParagraphStyle) -> Option<(f64, f64)> {
    let left: f64 = style.indent_left.unwrap_or(0.0).max(0.0);
    let right: f64 = style.indent_right.unwrap_or(0.0).max(0.0);
    (left > 0.0 || right > 0.0).then_some((left, right))
}

fn write_paragraph_indent_inset(out: &mut String, indent: Option<(f64, f64)>) {
    if let Some((left, right)) = indent {
        let _ = write!(
            out,
            ", inset: (left: {}pt, right: {}pt)",
            format_f64(left),
            format_f64(right)
        );
    }
}

/// Whether the paragraph needs its own block to carry style. The indent is
/// deliberately absent: `generate_paragraph` wraps indented paragraphs in
/// their own block, and the fixed-text paths that share this predicate emit
/// no inset, so counting the indent here would open a bare wrapper that only
/// leaks Typst's default block spacing.
pub(super) fn needs_block_wrapper(style: &ParagraphStyle) -> bool {
    style.space_before.is_some()
        || style.space_after.is_some()
        || style.background.is_some()
        || style.border.is_some()
        || style.line_spacing.is_some()
        || style.line_box.is_some()
        || matches!(style.alignment, Some(Alignment::Justify))
        || matches!(style.direction, Some(TextDirection::Rtl))
}

/// Line-box settings for a body paragraph: a fixed box spanning Word's full
/// line advance — the font's hhea line, 1.3 times it when the line carries
/// East Asian text, or a snapping document grid's pitch — with zero leading.
/// Typst's glyph-tight default renders such documents 20-30% shorter and
/// shifts every page break (issue #354).
///
/// The baseline sits at a constant `hhea ascender + lineGap` below the box
/// top, never at the font's ascender/descender proportion of it: whatever
/// height the line gains over the font's own — the East Asian bonus's lower
/// half, or a grid slot's slack — accrues below the baseline, not around it
/// (issues #508, #518).
///
/// Carrying the advance inside the box, rather than recovering the
/// remainder as `par(leading:)`, is what makes a paragraph's height match
/// Word's. Typst inserts `leading` only *between* the lines of one
/// paragraph, so an n-line paragraph came out one whole leading short
/// however many lines it had, and consecutive 9pt Courier New paragraphs
/// advanced 28% tighter than Word (issue #452). It also lets `w:spacing
/// w:before/w:after` reach the block unchanged, because the block edges now
/// sit exactly where Word measures those gaps from (issue #394).
pub(super) fn word_line_height_settings(
    runs: &[Run],
    style: &ParagraphStyle,
    line_grid_pitch: Option<f64>,
) -> Option<String> {
    let (top_em, bottom_em) = word_line_box_em(runs, style, line_grid_pitch)?;
    // Pin the line box to the nominal font's own metric edges as fixed em
    // values rather than the "ascender"/"descender" keywords. The keywords
    // let Typst resolve the box against the tallest font on each line, so a
    // bullet marker or em dash pulled from a taller fallback font inflated
    // that one line's advance past the grid/single-spacing (issue #398).
    Some(format!(
        "#set text(top-edge: {}em, bottom-edge: -{}em)\n#set par(leading: 0pt)\n",
        format_f64(top_em),
        format_f64(bottom_em)
    ))
}

/// The `(top-edge, bottom-edge)` in em behind [`word_line_height_settings`],
/// exposed so a framed eojeol can restore the same edges inside itself
/// (issue #626).
pub(super) fn word_line_box_em(
    runs: &[Run],
    style: &ParagraphStyle,
    line_grid_pitch: Option<f64>,
) -> Option<(f64, f64)> {
    let (ascender_em, descender_em, leading_em) =
        word_line_box_and_leading(runs, style, line_grid_pitch)?;
    let metric_em: f64 = ascender_em + descender_em;
    if metric_em <= 0.0 {
        return None;
    }
    let pitch_em: f64 = metric_em + leading_em;
    let top_em: f64 = ascender_em + east_asian_ascent_excess_em(runs, metric_em);
    Some((top_em, pitch_em - top_em))
}

/// Line-box settings for a slide's text: PowerPoint's flat 1.2em line, split
/// at the font's OS/2 `usWinAscent` proportion, with zero leading.
///
/// This is the PPTX counterpart of [`word_line_height_settings`], and the two
/// models genuinely differ. Word's line is the font's own hhea pitch;
/// PowerPoint's ignores the font's metrics for the height and consults them
/// only for where inside it the baseline sits. Slide text used to take the Word
/// treatment, which is up to 4% short per line and accumulates down a bullet
/// list, and it seated the baseline by Typst's normalised ascender, which put
/// a bottom-anchored box's last baseline flat on the inset with no descent gap
/// at all (issue #513).
///
/// `<a:lnSpc><a:spcPct>` scales that line rather than replacing it: the advance
/// is `percent x 1.2em`, and the baseline keeps the font's share of the taller
/// box. Carrying the percentage as `par(leading)` instead moved nothing between
/// single-line paragraphs — a slide's code block is one `<a:p>` per line — so
/// the lines overlapped (issue #541).
///
/// `None` when the paragraph carries its own line box, when its spacing is an
/// absolute `a:spcPts` advance, or when the font's metrics are unknown.
pub(super) fn powerpoint_line_height_settings(
    runs: &[Run],
    style: &ParagraphStyle,
) -> Option<String> {
    if style.line_box.is_some() {
        return None;
    }
    let percent: f64 = match style.line_spacing {
        None => 1.0,
        Some(LineSpacing::Proportional(factor)) if factor > 0.0 => factor,
        Some(_) => return None,
    };
    let family: &str = runs
        .iter()
        .find_map(|run| run.style.font_family.as_deref())?;
    let (ascent_em, descent_em) = crate::render::pdf::powerpoint_line_box_em(family)?;
    Some(format!(
        "#set text(top-edge: {}em, bottom-edge: -{}em)\n#set par(leading: 0pt)\n",
        format_f64(ascent_em * percent),
        format_f64(descent_em * percent)
    ))
}

/// The nominal font's `(above baseline, below baseline)` split plus the
/// leading, in em, that tops the line box up to Word's line advance. `None`
/// when the metric-edge treatment does not apply.
///
/// Since #508 the pair already sums to the font's single-spacing pitch, so the
/// leading is zero for a Latin line with no grid; it carries the East Asian
/// bonus and any grid slack (issue #518).
fn word_line_box_and_leading(
    runs: &[Run],
    style: &ParagraphStyle,
    line_grid_pitch: Option<f64>,
) -> Option<(f64, f64, f64)> {
    let leading_pt: f64 = word_line_leading_pt(runs, style, line_grid_pitch)?;
    let family: &str = east_asian_aware_metric_family(runs)?;
    let (ascender_em, descender_em, _word_pitch_em) =
        crate::render::pdf::font_line_metrics_em(family)?;
    let font_size: f64 = paragraph_font_size_pt(runs);
    Some((ascender_em, descender_em, leading_pt / font_size))
}

/// Word gives a line carrying East Asian text 130% of the font's own hhea
/// line, and centres the bonus on the baseline: half above, half below.
///
/// Both halves are measured, not assumed. Against native Word exports an Arial
/// first baseline sits at `hhea ascender + lineGap` = 0.937988em below the text
/// top while a Malgun Gothic one at the same settings sits at 1.28786em, and
/// the difference is exactly `0.15 x` Malgun's 1.330078em hhea pitch — the term
/// #508 could not attribute to any font table. The matching lower half shows up
/// as the advance: every Korean fixture in the business corpus paces its
/// wrapped lines at `1.3 x` the hhea pitch (10.5pt Malgun measures 18.00-18.24
/// against 18.156 predicted), and 06_official_letter_ko's 9.5pt paragraphs
/// advance 16.43pt where the font's bare hhea line is 12.64pt (issue #518).
const EAST_ASIAN_LINE_HEIGHT_FACTOR: f64 = 1.3;

/// The half of that bonus which lands above the baseline.
const EAST_ASIAN_ASCENT_EXCESS: f64 = (EAST_ASIAN_LINE_HEIGHT_FACTOR - 1.0) / 2.0;

/// Whether Word treats this paragraph's lines as East Asian.
///
/// Word only gives the bonus to lines that actually carry East Asian
/// characters: in a native Korean export the Arial runs of the same document
/// keep their plain hhea line, and inflating them too made every Western
/// document 30-50% taller (issue #354).
/// The family whose metrics pace these runs' lines.
///
/// A line carrying East Asian text is paced by the East Asian face, not by the
/// Latin one the same runs also name: the 1.3 factor above was measured
/// against Malgun Gothic's hhea pitch. Reading the Latin family was harmless
/// only while `w:eastAsia` was being dropped and the Latin family was the one
/// actually shaping the Hangul (issue #575).
fn east_asian_aware_metric_family(runs: &[Run]) -> Option<&str> {
    let latin = || runs.iter().find_map(|run| run.style.font_family.as_deref());
    if has_east_asian_text(runs) {
        runs.iter()
            .find_map(|run| run.style.east_asian_font_family.as_deref())
            .or_else(latin)
    } else {
        latin()
    }
}

fn has_east_asian_text(runs: &[Run]) -> bool {
    runs.iter().any(|run| run.text.chars().any(is_cjk_like))
}

/// The extra ascent, in em, that Word gives a line carrying East Asian text.
///
/// `pitch_em` is the font's own hhea pitch, never the line's advance: under a
/// document grid the slot's extra height accrues entirely below the baseline,
/// so this term must not scale with the slot (issue #518).
fn east_asian_ascent_excess_em(runs: &[Run], pitch_em: f64) -> f64 {
    if has_east_asian_text(runs) {
        EAST_ASIAN_ASCENT_EXCESS * pitch_em
    } else {
        0.0
    }
}

/// The line advance Word gives this paragraph before any grid is consulted:
/// the font's hhea line, or 1.3 times it when the line carries East Asian
/// text (issue #518).
fn word_natural_line_em(runs: &[Run], word_pitch_em: f64) -> f64 {
    if has_east_asian_text(runs) {
        EAST_ASIAN_LINE_HEIGHT_FACTOR * word_pitch_em
    } else {
        word_pitch_em
    }
}

/// The font size Word resolves a paragraph's line box against: the largest
/// size among its runs, falling back to the Word default when unset.
fn paragraph_font_size_pt(runs: &[Run]) -> f64 {
    largest_font_size_pt(runs.iter().filter_map(|run| run.style.font_size))
}

/// The largest declared size, or Word's default when nothing declares one —
/// which is also what an `em` resolves against, since the generator emits no
/// document-wide `#set text(size:)`.
fn largest_font_size_pt(sizes: impl Iterator<Item = f64>) -> f64 {
    let largest: f64 = sizes.fold(f64::NAN, f64::max);
    if largest.is_nan() { 11.0 } else { largest }
}

/// Whether a cell's grid-snapped line box already contains the paragraph's
/// `w:spacing w:after`, so the caller must not emit it a second time.
///
/// Mirrors the guard inside [`word_cell_line_box_settings`] exactly, including
/// its early return for paragraphs that carry their own line spacing or box —
/// gating on `row_has_east_asian_text` alone would strip the gap from those.
pub(super) fn cell_grid_absorbs_space_after(
    style: &ParagraphStyle,
    line_grid_pitch: Option<f64>,
    row_has_east_asian_text: bool,
) -> bool {
    row_has_east_asian_text
        && style.line_spacing.is_none()
        && style.line_box.is_none()
        && line_grid_pitch.is_some_and(|pitch| pitch > 0.0)
}

/// A table-cell paragraph's fixed line box, resolved at the paragraph's own
/// font size. `top_em`/`bottom_em` are the metric edges the cell emits;
/// `leading_pt` is the gap between line boxes.
pub(super) struct CellLineBox {
    pub top_em: f64,
    pub bottom_em: f64,
    /// Zero except when the box is re-seated on the descender: the surplus
    /// removed below the baseline moves here, so multi-line
    /// baseline-to-baseline advance is unchanged (issue #618).
    pub leading_pt: f64,
    pub font_size_pt: f64,
}

/// Line-box settings for a table cell: a fixed box spanning the font's full
/// single-spacing (hhea) line — 1.3 times it for an East Asian row — seated at
/// the same constant ascent the body path uses. In the default symmetric
/// emission the box carries the whole line advance below the ascent with zero
/// leading, so a single-line cell occupies the full line height Word gives it
/// rather than only the tighter metric box (which left auto-height rows too
/// short, issue #396). When `seats_text_on_descender` is set (bottom-aligned
/// spreadsheet cells in fixed-height rows), the box instead ends at the
/// font's descender and the removed sub-baseline surplus moves into leading,
/// so the last line's descent rests on the row's bottom inset edge while
/// multi-line advance is unchanged (issue #618). `None` when the font's
/// metrics are unknown or the paragraph carries its own line spacing/box.
///
/// The box also carries the paragraph's `w:spacing w:after` when a snapping
/// grid is in force, because Word snaps the line and that gap together (issues
/// #500, #503).
pub(super) fn word_cell_line_box_settings(
    runs: &[Run],
    style: &ParagraphStyle,
    line_grid_pitch: Option<f64>,
    row_has_east_asian_text: bool,
    seats_text_on_descender: bool,
) -> Option<String> {
    let line_box: CellLineBox = word_cell_line_box(
        runs,
        style,
        line_grid_pitch,
        row_has_east_asian_text,
        seats_text_on_descender,
    )?;
    Some(format!(
        "#set text(top-edge: {}em, bottom-edge: -{}em)\n#set par(leading: {}pt)\n",
        format_f64(line_box.top_em),
        format_f64(line_box.bottom_em),
        format_f64(line_box.leading_pt)
    ))
}

/// The line box behind [`word_cell_line_box_settings`], exposed so the spill
/// wrapper can size its clip box and strut from the same numbers the block
/// emits (issue #618).
pub(super) fn word_cell_line_box(
    runs: &[Run],
    style: &ParagraphStyle,
    line_grid_pitch: Option<f64>,
    row_has_east_asian_text: bool,
    seats_text_on_descender: bool,
) -> Option<CellLineBox> {
    if style.line_spacing.is_some() || style.line_box.is_some() {
        return None;
    }
    let family: &str = east_asian_aware_metric_family(runs)?;
    let (ascender_em, descender_em, word_pitch_em) =
        crate::render::pdf::font_line_metrics_em(family)?;
    let metric_em: f64 = ascender_em + descender_em;
    if metric_em <= 0.0 || word_pitch_em <= 0.0 {
        return None;
    }
    let font_size: f64 = paragraph_font_size_pt(runs);
    // The row decides whether its lines are East Asian, not this cell: reading
    // each cell's own text put a Korean label and its numeric neighbours on
    // line boxes of different heights, splitting one row across two baselines
    // 4.29pt apart (issue #498). So both the 1.3 line-height bonus and the
    // ascent excess it implies key on the row's answer.
    let natural_em: f64 = if row_has_east_asian_text {
        EAST_ASIAN_LINE_HEIGHT_FACTOR * word_pitch_em
    } else {
        word_pitch_em
    };
    // A grid-snapped row snaps the line *plus* the paragraph's own `w:spacing
    // w:after`, not the line alone. Snapping the bare line and then adding the
    // gap outside it made every grid-scoped row 1.06pt too tall, because
    // 12.64pt of Malgun and a 1.5pt gap both fit inside one 18pt line where
    // 18 + 1.5 does not (issues #500, #503). `cell_grid_absorbs_space_after`
    // gates the caller's matching suppression of the trailing gap; the two must
    // agree.
    let advance_em: f64 = match line_grid_pitch.filter(|pitch| *pitch > 0.0) {
        Some(pitch) if row_has_east_asian_text => {
            // Same two-way choice as the body path (issue #508), with the
            // paragraph's `w:after` inside the quantity being compared.
            let natural_pt: f64 = natural_em * font_size + style.space_after.unwrap_or(0.0);
            let advance_pt: f64 = if natural_pt <= pitch {
                pitch
            } else {
                natural_pt
            };
            advance_pt / font_size
        }
        _ => natural_em,
    };
    let excess_em: f64 = if row_has_east_asian_text {
        EAST_ASIAN_ASCENT_EXCESS * word_pitch_em
    } else {
        0.0
    };
    let top_em: f64 = ascender_em + excess_em;
    // Excel rests a bottom-aligned cell's last line on its descender: the
    // descent bottom sits on the row's bottom inset edge with all slack above.
    // The symmetric box carries the East Asian 0.15-line surplus below the
    // baseline, which floated bottom-aligned Korean cells above where Excel
    // prints them (issue #618). Ending the box at the descender and moving the
    // surplus into leading keeps multi-line advance identical. Word and Excel
    // both measure a line's bottom down to the descender line — the same rule
    // the header/footer path already applies with `bottom-edge: "descender"`.
    // TODO(#618 follow-up: leading is one per-paragraph pt value derived from
    // the max run size, so mixed-font-size wrapped lines gain
    // 0.15*pitch*(max-line) advance error; needs per-line seating if a real
    // sheet exhibits it).
    let (bottom_em, leading_pt): (f64, f64) = if seats_text_on_descender {
        (
            descender_em,
            ((advance_em - top_em - descender_em) * font_size).max(0.0),
        )
    } else {
        (advance_em - top_em, 0.0)
    };
    Some(CellLineBox {
        top_em,
        bottom_em,
        leading_pt,
        font_size_pt: font_size,
    })
}

/// The top-up that raises the font's typographic metric box to Word's line
/// advance — its hhea single-space line, or the document grid pitch for East
/// Asian text under a `w:docGrid`. `word_line_height_settings` folds this
/// into the fixed line-box height rather than emitting it as `par(leading:)`
/// whitespace between boxes, because Typst inserts that only *between* the
/// lines of one paragraph and every paragraph then came up one top-up short
/// (issues #354, #452). A proportional `w:lineRule="auto"` scales the result
/// rather than replacing it, because that is what Word's own rule means.
/// `None` when the paragraph states an exact advance, carries its own line
/// box, or the font's metrics are unknown — the treatment does not apply
/// then.
pub(super) fn word_line_leading_pt(
    runs: &[Run],
    style: &ParagraphStyle,
    line_grid_pitch: Option<f64>,
) -> Option<f64> {
    if style.line_box.is_some() {
        return None;
    }
    // `w:lineRule="auto"` scales Word's own line rather than replacing it:
    // `w:line="278"` means 1.158 of the line this function computes. Bailing
    // out on any `w:spacing w:line` left those paragraphs to Typst's default
    // leading, which knows nothing of the East Asian line — 15.4pt against
    // Word's 19.9pt on the technical brief (issue #575). An exact rule states
    // the advance outright and is still handled as a plain `par(leading:)`.
    let proportion: f64 = match style.line_spacing {
        None => 1.0,
        Some(LineSpacing::Proportional(factor)) if factor > 0.0 => factor,
        Some(_) => return None,
    };
    let family: &str = east_asian_aware_metric_family(runs)?;
    let (ascender_em, descender_em, word_pitch_em) =
        crate::render::pdf::font_line_metrics_em(family)?;
    let font_size: f64 = runs
        .iter()
        .filter_map(|run| run.style.font_size)
        .fold(f64::NAN, f64::max);
    let font_size: f64 = if font_size.is_nan() { 11.0 } else { font_size };
    let line_box_pt: f64 = (ascender_em + descender_em) * font_size;
    if line_box_pt <= 0.0 {
        return None;
    }

    // Word's single spacing is the font's full hhea line, which the metric pair
    // sums to directly (issue #508) - so for Latin text this top-up is zero and
    // the subtraction below is just a guard for a face whose reported pitch
    // exceeds its own ascent-plus-descent. East Asian lines get 30% more
    // (issue #518).
    let natural_line_pt: f64 = word_natural_line_em(runs, word_pitch_em) * font_size;

    // Word only snaps East Asian text to the document grid: Latin-only
    // paragraphs keep their hhea line height even under one (native Word GT:
    // Arial 10.5 lines stay 12pt in a Korean document). Snapping Latin
    // paragraphs inflated every Western document by 30-50% (issue #354).
    let advance_pt: f64 = match line_grid_pitch {
        Some(pitch) if pitch > 0.0 && has_east_asian_text(runs) => {
            // A grid line never compresses text below the height its font
            // needs, and Word chooses between exactly two advances, never a
            // multiple: the grid pitch when the natural line fits inside one
            // grid line, otherwise the natural line untouched (issues #402,
            // #508).
            if natural_line_pt <= pitch {
                pitch
            } else {
                natural_line_pt
            }
        }
        _ => natural_line_pt,
    };
    Some((advance_pt * proportion - line_box_pt).max(0.0))
}

pub(super) fn write_block_params(out: &mut String, style: &ParagraphStyle) {
    let mut first = true;

    if let Some(above) = style.space_before {
        write_param(out, &mut first, &format!("above: {}pt", format_f64(above)));
    }
    if let Some(below) = style.space_after {
        write_param(out, &mut first, &format!("below: {}pt", format_f64(below)));
    }
}

/// The paragraph's `w:spacing` gaps, for a parameter list that already has a
/// first entry (every parameter is prefixed with a comma). They belong to
/// the outermost block, so an indent wrapper does not separate them from the
/// neighbouring paragraphs they collapse against.
fn write_block_spacing_params(out: &mut String, style: &ParagraphStyle) {
    if let Some(above) = style.space_before {
        let _ = write!(out, ", above: {}pt", format_f64(above));
    }
    if let Some(below) = style.space_after {
        let _ = write!(out, ", below: {}pt", format_f64(below));
    }
}

/// The paragraph's shading and borders, which Word paints across the
/// paragraph's own column — from the left indent to the right indent — so
/// they belong to the innermost block (issue #464).
fn write_block_decoration_params(out: &mut String, style: &ParagraphStyle) {
    if let Some(background) = style.background {
        let _ = write!(out, ", fill: {}", rgb(&background));
    }
    if let Some(border) = &style.border {
        write_paragraph_border_params(
            out,
            border,
            style.border_space.as_deref().copied().unwrap_or_default(),
        );
    }
}

fn stroke_literal(side: &BorderSide) -> String {
    // Callers skip Double sides (drawn as overlays), so for every reachable
    // style this matches the table flavor of the shared stroke formatter.
    stroke_value(side, true)
}

/// Emit `stroke:`/`inset:` block parameters for the paragraph's borders.
/// Double rules are drawn as overlays (Typst strokes have no double style),
/// so those sides only reserve inset space here.
///
/// Each side reserves its own `w:space` plus the rule's own thickness. A fixed
/// 4pt stood in for `w:space` until #520: a letterhead declaring 8pt then
/// pulled every line below it up by the difference, and the error is a step,
/// not a drift, so it survives to the bottom of the page.
fn write_paragraph_border_params(out: &mut String, border: &CellBorder, space: Insets) {
    let mut strokes: Vec<String> = Vec::new();
    let mut insets: Vec<String> = Vec::new();

    let mut push_side = |name: &str, side: &Option<BorderSide>, gap: f64| {
        let Some(side) = side else {
            return;
        };
        let reserved = if side.style == BorderLineStyle::Double {
            gap + double_rule_thickness(side.width)
        } else {
            strokes.push(format!("{name}: {}", stroke_literal(side)));
            gap + side.width
        };
        insets.push(format!("{name}: {}pt", format_f64(reserved)));
    };
    push_side("top", &border.top, space.top);
    push_side("bottom", &border.bottom, space.bottom);
    push_side("left", &border.left, space.left);
    push_side("right", &border.right, space.right);

    if !strokes.is_empty() {
        let _ = write!(out, ", stroke: ({})", strokes.join(", "));
    }
    if !insets.is_empty() {
        let _ = write!(out, ", inset: ({})", insets.join(", "));
    }
}

/// A Word double rule draws two lines of the declared width separated by a gap
/// of the same width, so it stands three widths tall in total. Measured on
/// 06_official_letter_ko's `w:sz="8"` letterhead rule: 3pt, against the GT's
/// 2.93pt gap between the paragraph below it and the rule's far edge.
fn double_rule_thickness(width: f64) -> f64 {
    width * 3.0
}

/// Draw double-rule paragraph borders as two placed hairlines; Typst strokes
/// cannot render Word's double style. Only horizontal doubles occur in
/// practice (letterhead rules); vertical doubles fall back to a single
/// stroke drawn by `write_paragraph_border_params`.
fn write_paragraph_double_border_overlays(
    out: &mut String,
    border: &Option<Box<CellBorder>>,
    space: Insets,
) {
    let Some(border) = border else {
        return;
    };
    for (name, side, gap) in [
        ("top", &border.top, space.top),
        ("bottom", &border.bottom, space.bottom),
    ] {
        let Some(side) = side else {
            continue;
        };
        if side.style != BorderLineStyle::Double {
            continue;
        }
        let w = side.width;
        let near_dy = gap + w;
        let far_dy = gap + double_rule_thickness(w);
        let (align, sign) = if name == "top" {
            ("top", -1.0)
        } else {
            ("bottom", 1.0)
        };
        for dy in [near_dy, far_dy] {
            let _ = write!(
                out,
                "#place({align}, dy: {}pt, line(length: 100%, stroke: {}pt + {}))",
                format_f64(sign * dy),
                format_f64(w),
                rgb(&side.color),
            );
        }
    }
}

pub(super) fn write_par_settings(out: &mut String, style: &ParagraphStyle) {
    if let Some(ref spacing) = style.line_spacing {
        match spacing {
            LineSpacing::Proportional(factor) => {
                let leading = factor * 0.65;
                let _ = writeln!(out, "  #set par(leading: {}em)", format_f64(leading));
            }
            LineSpacing::Exact(pts) => {
                let _ = writeln!(out, "  #set par(leading: {}pt)", format_f64(*pts));
            }
        }
    }
    if matches!(style.alignment, Some(Alignment::Justify)) {
        out.push_str("  #set par(justify: true)\n");
    }
    if matches!(style.direction, Some(TextDirection::Rtl)) {
        out.push_str("  #set text(dir: rtl)\n");
    }
}

pub(super) fn write_line_box_settings(out: &mut String, line_box: Option<LineBox>) {
    let Some(line_box) = line_box else {
        return;
    };
    let _ = writeln!(
        out,
        "#set text(top-edge: {}em, bottom-edge: -{}em)",
        format_f64(line_box.ascent_em),
        format_f64(line_box.descent_em),
    );
    out.push_str("#set par(leading: 0pt)\n");
}

pub(super) fn generate_runs_with_tabs(
    out: &mut String,
    runs: &[Run],
    tab_stops: Option<&[TabStop]>,
    default_tab_width_pt: f64,
    eojeol_wrap: EojeolWrap,
) {
    if !paragraph_contains_tabs(runs) {
        generate_runs(out, runs, eojeol_wrap);
        return;
    }

    let segments: Vec<Vec<Run>> = split_runs_on_tabs(runs);
    out.push_str("#context {\n");

    for (index, segment) in segments.iter().enumerate() {
        let _ = write!(out, "  let tab_segment_{index} = [");
        generate_runs(out, segment, eojeol_wrap);
        out.push_str("]\n");

        if index == 0 {
            out.push_str("  let tab_prefix_0 = tab_segment_0\n");
            continue;
        }

        write_tab_segment_bindings(out, index, segment, tab_stops, default_tab_width_pt);
    }

    let _ = writeln!(out, "  tab_prefix_{}", segments.len() - 1);
    out.push('}');
}

pub(super) fn generate_runs_with_tabs_no_wrap(
    out: &mut String,
    runs: &[Run],
    tab_stops: Option<&[TabStop]>,
    default_tab_width_pt: f64,
) {
    let preserve_cjk_no_wrap: bool = runs
        .iter()
        .filter(|run| run.footnote.is_none())
        .any(|run| run.text.chars().any(is_cjk_like));
    let mut no_wrap_state: NoWrapState = NoWrapState::default();
    let transformed_runs: Vec<Run> = runs
        .iter()
        .map(|run| {
            let mut transformed_run: Run = run.clone();
            if transformed_run.footnote.is_none() {
                transformed_run.text = no_wrap_text(
                    &transformed_run.text,
                    preserve_cjk_no_wrap,
                    &mut no_wrap_state,
                );
            } else {
                no_wrap_state = NoWrapState::default();
            }
            transformed_run
        })
        .collect();

    // Slide text keeps PowerPoint's own breaking, which splits Korean
    // mid-word; this path additionally forbids every break outright.
    generate_runs_with_tabs(
        out,
        &transformed_runs,
        tab_stops,
        default_tab_width_pt,
        EojeolWrap::Syllable,
    );
}

#[derive(Clone, Copy, Default)]
struct NoWrapState {
    previous_visible_char: Option<char>,
    previous_non_breaking_space: bool,
}

/// Emits Typst variable bindings for a non-first tab segment: measurement,
/// decimal anchor (if applicable), default remainder, advance, fill, and
/// the accumulated prefix content variable.
fn write_tab_segment_bindings(
    out: &mut String,
    index: usize,
    segment: &[Run],
    tab_stops: Option<&[TabStop]>,
    default_tab_width_pt: f64,
) {
    let _ = writeln!(
        out,
        "  let tab_prefix_width_{index} = measure(tab_prefix_{}).width",
        index - 1
    );
    let _ = writeln!(
        out,
        "  let tab_segment_width_{index} = measure(tab_segment_{index}).width"
    );

    if let Some(anchor_runs) = extract_decimal_anchor_runs(segment) {
        let _ = write!(out, "  let tab_decimal_anchor_{index} = [");
        // Measured for its width only, which a frame does not change, so the
        // anchor stays the plain emission.
        generate_runs(out, &anchor_runs, EojeolWrap::Syllable);
        out.push_str("]\n");
        let _ = writeln!(
            out,
            "  let tab_decimal_width_{index} = measure(tab_decimal_anchor_{index}).width"
        );
    }

    let _ = writeln!(
        out,
        "  let tab_default_remainder_{index} = calc.rem-euclid(tab_prefix_width_{index}.abs.pt(), {})",
        format_f64(default_tab_width_pt)
    );
    let _ = writeln!(
        out,
        "  let tab_advance_{index} = {}",
        build_tab_advance_expr(index, segment, tab_stops, default_tab_width_pt)
    );
    let _ = writeln!(
        out,
        "  let tab_fill_{index} = {}",
        build_tab_fill_expr(index, tab_stops)
    );
    let _ = writeln!(
        out,
        "  let tab_prefix_{index} = [#tab_prefix_{}#tab_fill_{index}#tab_segment_{index}]",
        index - 1
    );
}

fn paragraph_contains_tabs(runs: &[Run]) -> bool {
    runs.iter().any(|run| run.text.contains('\t'))
}

/// Whether a run list keeps each Hangul eojeol — a space-delimited Korean
/// word — whole when a line has to break (issue #626).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(super) enum EojeolWrap {
    /// Typst's UAX #14 default, which permits a break between any two Hangul
    /// syllable blocks. PowerPoint and a *justified* Word line both break
    /// Korean mid-word, and our output already matches them there, so slides,
    /// sheets and justified paragraphs stay on this.
    #[default]
    Syllable,
    /// Emit each eojeol inside an inline `#box`. A frame is a single object to
    /// UAX #14, so no break opportunity survives inside it. Typst 0.14 offers
    /// no other lever: `text(lang: "ko")`, `par(linebreaks:)` and
    /// `text(costs:)` were each measured to leave the breakpoints untouched,
    /// because typst-layout builds its ICU4X segmenters with default options
    /// and never consults `Lang::KOREAN`. The repo already uses the same
    /// mechanism in the opposite direction, where `#box[]` *creates* a
    /// contingent break for PowerPoint kinsoku (issue #438).
    ///
    /// A no-break marker between the syllables — U+2060 WORD JOINER, the
    /// obvious alternative and the shape the auto-space and kinsoku markers
    /// in this file already use — does suppress the breaks, but it lands in
    /// the PDF text layer and makes the text unsearchable. That is issue
    /// #664, reproduced from first principles on a probe before this
    /// mechanism was chosen; a frame leaves the text layer untouched.
    ///
    /// `line_box_em` is the paragraph's fixed `(top-edge, bottom-edge)` when
    /// it declares them, so the frame can restore them; see
    /// [`write_eojeol_frame_open`]. `measure_pt` is the width one line of the
    /// paragraph has, which bounds how wide a token may be and still be
    /// framed; see [`is_framed_eojeol`].
    Eojeol {
        line_box_em: Option<(f64, f64)>,
        measure_pt: Option<f64>,
    },
}

/// The longest token still framed when its width cannot be measured, in
/// characters.
///
/// A token wider than the line cannot break inside its frame, so it starts a
/// line of its own and then overflows it — one line more than Word spends,
/// plus ink outside the column. [`is_framed_eojeol`] therefore compares the
/// token's measured advance against the paragraph's own measure. This cap is
/// only the fallback for when one of the two is unknown: on `wasm32`, where
/// [`text_advance_em`](crate::render::pdf::text_advance_em) always returns
/// `None`, for a run that names no family or size, and for a container whose
/// measure did not reach codegen. An eojeol is a stem plus its particles and
/// rarely reaches ten syllables, so twenty is a generous ceiling.
const MAX_UNMEASURED_EOJEOL_CHARS: usize = 20;

/// How a paragraph breaks its Hangul lines.
///
/// Word falls back to syllable breaking to keep a *justified* line from going
/// too loose — measured on the contract fixture, where both Word and this
/// generator split `보관|한다.` in the two `w:jc="both"` paragraphs — so a
/// justified paragraph keeps the engine default.
///
/// `container_measure_pt` is the width the enclosing container gives a line —
/// the page's text width, a table column, a text box — before this
/// paragraph's own indents are taken off it.
pub(super) fn paragraph_eojeol_wrap(
    breaks_hangul_at_eojeol: bool,
    style: &ParagraphStyle,
    line_box_em: Option<(f64, f64)>,
    container_measure_pt: Option<f64>,
) -> EojeolWrap {
    if !breaks_hangul_at_eojeol || matches!(style.alignment, Some(Alignment::Justify)) {
        return EojeolWrap::Syllable;
    }
    // A hanging first line (`indent_first_line < 0`) is wider than the rest,
    // so the continuation lines — the ones a frame can be pushed onto — are
    // the binding measure and the negative first-line indent is ignored.
    let measure_pt: Option<f64> = container_measure_pt
        .map(|measure| {
            measure - style.indent_left.unwrap_or(0.0) - style.indent_right.unwrap_or(0.0)
        })
        .filter(|measure| *measure > 0.0);
    EojeolWrap::Eojeol {
        line_box_em,
        measure_pt,
    }
}

pub(super) fn generate_runs(out: &mut String, runs: &[Run], eojeol_wrap: EojeolWrap) {
    let EojeolWrap::Eojeol {
        line_box_em,
        measure_pt,
    } = eojeol_wrap
    else {
        for run in runs {
            generate_run(out, run);
        }
        return;
    };

    // Everything between two frames is coalesced and spliced back into whole
    // runs before it is emitted, so a paragraph in which no eojeol is framed —
    // every Latin one, and every Korean one whose words are all single
    // syllables — keeps byte-identical markup.
    let mut units: Vec<(bool, Vec<EojeolPiece>)> = Vec::new();
    for token in split_runs_into_eojeol_tokens(runs) {
        match (is_framed_eojeol(&token, measure_pt), units.last_mut()) {
            (false, Some((false, unframed))) => unframed.extend(token),
            (framed, _) => units.push((framed, token)),
        }
    }

    for (framed, pieces) in &units {
        if *framed {
            write_eojeol_frame_open(out, pieces, line_box_em);
        }
        write_eojeol_pieces(out, pieces);
        if *framed {
            write_eojeol_frame_close(out, line_box_em);
        }
    }
}

/// A slice of one run, tagged with the run it was cut from.
///
/// The tag is what lets [`write_eojeol_pieces`] splice neighbouring slices
/// back together: `escape_typst` reads its whole input — a run of spaces
/// becomes a code-mode string, a leading `<digits>.` an escaped enum marker —
/// so re-joining pieces of *different* runs could change the markup where
/// concatenating pieces of the same run never can.
struct EojeolPiece {
    run_index: usize,
    run: Run,
}

/// Emits pieces, re-joining every neighbouring pair cut from the same run.
fn write_eojeol_pieces(out: &mut String, pieces: &[EojeolPiece]) {
    let mut pending: Option<(usize, Run)> = None;
    for piece in pieces {
        match pending {
            Some((run_index, ref mut previous)) if run_index == piece.run_index => {
                previous.text.push_str(&piece.run.text);
            }
            _ => {
                if let Some((_, previous)) = pending.take() {
                    generate_run(out, &previous);
                }
                pending = Some((piece.run_index, piece.run.clone()));
            }
        }
    }
    if let Some((_, previous)) = pending {
        generate_run(out, &previous);
    }
}

/// The characters a Word line may end at, which therefore close an eojeol.
///
/// A tab is among them, so a frame can never straddle a
/// [`split_runs_on_tabs`] segment. A no-break space cannot host a break at
/// all, but it still separates words, and treating it as a boundary keeps a
/// whole run of them out of one token.
fn is_eojeol_delimiter(ch: char) -> bool {
    matches!(
        ch,
        ' ' | '\u{00A0}' | '\t' | '\n' | PPTX_SOFT_LINE_BREAK_CHAR
    )
}

/// Hangul: a precomposed syllable block, a conjoining jamo, or a
/// compatibility jamo. Han and kana are deliberately absent — Chinese and
/// Japanese really do break between characters, and framing them would
/// destroy correct output.
fn is_hangul(ch: char) -> bool {
    matches!(ch as u32, 0x1100..=0x11FF | 0x3130..=0x318F | 0xAC00..=0xD7A3)
}

/// Splits a run list into the tokens Word may break between: maximal stretches
/// of delimiter-free text, each possibly spanning several runs, alternating
/// with the delimiters themselves.
///
/// Grouping happens here, structurally, rather than through a marker pair in
/// the text: [`extract_decimal_anchor_runs`] slices a run sub-list for a
/// decimal tab stop and would sever an open/close pair, emitting unbalanced
/// markup. Spanning runs matters because a bold or coloured fragment inside a
/// word would otherwise leave a frame boundary — itself a break opportunity —
/// in the middle of the word.
///
/// A footnote run is a token of its own: the reference is an anchor, not part
/// of any word.
fn split_runs_into_eojeol_tokens(runs: &[Run]) -> Vec<Vec<EojeolPiece>> {
    let mut tokens: Vec<Vec<EojeolPiece>> = Vec::new();
    let mut token: Vec<EojeolPiece> = Vec::new();

    for (run_index, run) in runs.iter().enumerate() {
        if run.footnote.is_some() {
            if !token.is_empty() {
                tokens.push(std::mem::take(&mut token));
            }
            tokens.push(vec![EojeolPiece {
                run_index,
                run: run.clone(),
            }]);
            continue;
        }
        // An empty run still emits its wrappers, so it must survive the split.
        if run.text.is_empty() {
            token.push(EojeolPiece {
                run_index,
                run: run.clone(),
            });
            continue;
        }

        let mut piece_start: usize = 0;
        let mut piece_is_delimiter: bool = is_eojeol_delimiter(
            run.text
                .chars()
                .next()
                .expect("a non-empty run has a first char"),
        );
        for (offset, ch) in run.text.char_indices() {
            let ch_is_delimiter: bool = is_eojeol_delimiter(ch);
            if ch_is_delimiter == piece_is_delimiter {
                continue;
            }
            push_eojeol_piece(
                &mut tokens,
                &mut token,
                run_index,
                run,
                &run.text[piece_start..offset],
                piece_is_delimiter,
            );
            piece_start = offset;
            piece_is_delimiter = ch_is_delimiter;
        }
        push_eojeol_piece(
            &mut tokens,
            &mut token,
            run_index,
            run,
            &run.text[piece_start..],
            piece_is_delimiter,
        );
    }

    if !token.is_empty() {
        tokens.push(token);
    }
    tokens
}

fn push_eojeol_piece(
    tokens: &mut Vec<Vec<EojeolPiece>>,
    token: &mut Vec<EojeolPiece>,
    run_index: usize,
    run: &Run,
    text: &str,
    is_delimiter: bool,
) {
    let piece: EojeolPiece = EojeolPiece {
        run_index,
        run: Run {
            text: text.to_string(),
            style: run.style.clone(),
            href: run.href.clone(),
            footnote: None,
        },
    };
    if !is_delimiter {
        token.push(piece);
        return;
    }
    if !token.is_empty() {
        tokens.push(std::mem::take(token));
    }
    tokens.push(vec![piece]);
}

/// Whether a token is an eojeol Word would keep whole: it carries Hangul, it
/// is long enough for a break to fall inside it, and it is narrow enough that
/// a line can hold it.
///
/// `measure_pt` is the width one line of the paragraph has. A frame is opaque
/// to line breaking, so a token wider than that would be pushed onto a line of
/// its own and still overflow it — one line more than Word spends, with ink
/// outside the column. Word itself breaks such a token at character level, so
/// this returns `false` and the token keeps the engine's syllable breaking.
fn is_framed_eojeol(token: &[EojeolPiece], measure_pt: Option<f64>) -> bool {
    if token.iter().any(|piece| piece.run.footnote.is_some()) {
        return false;
    }
    // Letter spacing survives a frame boundary by a rule this generator cannot
    // predict. Measured on typst 0.14 at `tracking: 0.7pt`: framing the four
    // Korean words of a 13pt centred heading made it 1.4pt *narrower*, while
    // framing the four words of a 9pt one made it 3.0pt *wider* — the shaper
    // does not simply trim one step per item. A tracked run is decorative
    // display text, short enough that it does not wrap and so has nothing to
    // gain here, so it keeps today's emission until the rule is measured.
    if token
        .iter()
        .any(|piece| piece.run.style.letter_spacing.is_some_and(|s| s != 0.0))
    {
        return false;
    }
    let mut visible_chars: usize = 0;
    let mut has_hangul: bool = false;
    for ch in token.iter().flat_map(|piece| piece.run.text.chars()) {
        // The in-text markers stand for spacing and break opportunities, not
        // glyphs, so they cannot make a one-syllable token breakable.
        if matches!(ch, EAST_ASIAN_AUTO_SPACE_CHAR | HANGUL_KINSOKU_BREAK_CHAR) {
            continue;
        }
        has_hangul |= is_hangul(ch);
        visible_chars += 1;
    }
    if !has_hangul || visible_chars < 2 {
        return false;
    }
    match (measure_pt, eojeol_advance_pt(token)) {
        (Some(measure), Some(advance)) => advance <= measure,
        // Either the container's measure or the token's advance is unknown;
        // fall back to the character ceiling, which at least keeps a
        // pathologically long token out of a frame.
        _ => visible_chars <= MAX_UNMEASURED_EOJEOL_CHARS,
    }
}

/// The advance a token takes on a line, in points, measured with the same
/// machinery the auto-layout column widths use (issue #624): each piece's
/// resolved family — the `w:eastAsia` face for East Asian codepoints — its
/// weight, and its own size.
///
/// `None` when a piece names no family or no size, or when a glyph is missing
/// from the resolved face; on `wasm32`
/// [`text_advance_em`](crate::render::pdf::text_advance_em) is always `None`,
/// so the whole guard degrades to its character ceiling there.
///
/// The in-text markers are skipped: they stand for a `#h()` the shaper never
/// sees as a glyph. That under-counts an auto-space boundary by a quarter em,
/// which only matters for a token already within a quarter em of the measure.
fn eojeol_advance_pt(token: &[EojeolPiece]) -> Option<f64> {
    let mut advance_pt: f64 = 0.0;
    for piece in token {
        let latin_family: &str = piece.run.style.font_family.as_deref()?;
        let east_asian_family: &str = piece
            .run
            .style
            .east_asian_font_family
            .as_deref()
            .unwrap_or(latin_family);
        let font_size_pt: f64 = piece.run.style.font_size?;
        let is_bold: bool = effective_font_weight(&piece.run.style)
            .is_some_and(|weight| weight != "regular" && weight != "light");
        // One `text_advance_em` call per maximal same-face segment: the call
        // takes a global face-cache lock, so a per-character loop would be
        // needlessly hot on long Korean paragraphs.
        let mut segment: String = String::new();
        let mut segment_is_east_asian: Option<bool> = None;
        for character in piece.run.text.chars() {
            if matches!(
                character,
                EAST_ASIAN_AUTO_SPACE_CHAR | HANGUL_KINSOKU_BREAK_CHAR
            ) {
                continue;
            }
            let is_east_asian: bool = is_cjk_like(character);
            if segment_is_east_asian != Some(is_east_asian) && !segment.is_empty() {
                let family: &str = if segment_is_east_asian == Some(true) {
                    east_asian_family
                } else {
                    latin_family
                };
                advance_pt +=
                    crate::render::pdf::text_advance_em(family, is_bold, &segment)? * font_size_pt;
                segment.clear();
            }
            segment_is_east_asian = Some(is_east_asian);
            segment.push(character);
        }
        if !segment.is_empty() {
            let family: &str = if segment_is_east_asian == Some(true) {
                east_asian_family
            } else {
                latin_family
            };
            advance_pt +=
                crate::render::pdf::text_advance_em(family, is_bold, &segment)? * font_size_pt;
        }
    }
    Some(advance_pt)
}

/// Opens an eojeol's frame.
///
/// Under Word's fixed line box (issues #354, #508) a bare `#box` seats its
/// baseline on its own *bottom* edge, which would drop the framed text by the
/// descent while the spaces around it stayed put. The frame therefore restores
/// the paragraph's edges inside itself and shifts its baseline back up by the
/// descent.
///
/// Those edges are re-emitted in points rather than the `em` the paragraph
/// declares: an `em` resolves against each run's own size, so a size change
/// inside one eojeol would leave the frame's height and its baseline shift
/// disagreeing. Resolving them at the token's own largest size reproduces
/// exactly what the same text contributes to the line unframed.
fn write_eojeol_frame_open(
    out: &mut String,
    token: &[EojeolPiece],
    line_box_em: Option<(f64, f64)>,
) {
    let Some((top_em, bottom_em)) = line_box_em else {
        out.push_str("#box[");
        return;
    };
    let font_size_pt: f64 =
        largest_font_size_pt(token.iter().filter_map(|piece| piece.run.style.font_size));
    let top_pt: f64 = top_em * font_size_pt;
    let bottom_pt: f64 = bottom_em * font_size_pt;
    let _ = write!(
        out,
        "#box(baseline: {}pt)[#text(top-edge: {}pt, bottom-edge: -{}pt)[",
        format_f64(bottom_pt),
        format_f64(top_pt),
        format_f64(bottom_pt)
    );
}

fn write_eojeol_frame_close(out: &mut String, line_box_em: Option<(f64, f64)>) {
    out.push_str(if line_box_em.is_some() { "]]" } else { "]" });
}

fn no_wrap_text(text: &str, preserve_cjk_no_wrap: bool, state: &mut NoWrapState) -> String {
    if !preserve_cjk_no_wrap {
        return text.to_string();
    }

    let mut out: String = String::new();

    for ch in text.chars() {
        if matches!(ch, '\t' | PPTX_SOFT_LINE_BREAK_CHAR) {
            out.push(ch);
            *state = NoWrapState::default();
            continue;
        }

        // A no-wrap box never takes the kinsoku break, so drop its marker
        // instead of letting the zero-width space reach the text layer.
        if ch == HANGUL_KINSOKU_BREAK_CHAR {
            continue;
        }

        if ch == ' ' {
            out.push('\u{00A0}');
            state.previous_visible_char = None;
            state.previous_non_breaking_space = true;
            continue;
        }

        if state.previous_non_breaking_space
            || state
                .previous_visible_char
                .is_some_and(|prev| needs_no_wrap_joiner(prev, ch))
        {
            out.push('\u{2060}');
        }
        out.push(ch);
        state.previous_visible_char = Some(ch);
        state.previous_non_breaking_space = false;
    }

    out
}

fn needs_no_wrap_joiner(previous: char, current: char) -> bool {
    !previous.is_whitespace() && !current.is_whitespace()
}

pub(crate) fn is_cjk_like(ch: char) -> bool {
    matches!(
        ch as u32,
        0x1100..=0x11FF
            | 0x2E80..=0x2FFF
            | 0x3000..=0x303F
            | 0x3040..=0x30FF
            | 0x3130..=0x318F
            | 0x31F0..=0x31FF
            | 0x3400..=0x4DBF
            | 0x4E00..=0x9FFF
            | 0xAC00..=0xD7AF
            | 0xF900..=0xFAFF
            | 0xFF00..=0xFFEF
    )
}

fn split_runs_on_tabs(runs: &[Run]) -> Vec<Vec<Run>> {
    let mut segments: Vec<Vec<Run>> = vec![Vec::new()];

    for run in runs {
        if run.footnote.is_some() || !run.text.contains('\t') {
            if run.footnote.is_some() || !run.text.is_empty() {
                segments
                    .last_mut()
                    .expect("split_runs_on_tabs should always have a segment")
                    .push(run.clone());
            }
            continue;
        }

        for (index, part) in run.text.split('\t').enumerate() {
            if index > 0 {
                segments.push(Vec::new());
            }

            if !part.is_empty() {
                segments
                    .last_mut()
                    .expect("split_runs_on_tabs should always have a segment")
                    .push(Run {
                        text: part.to_string(),
                        style: run.style.clone(),
                        href: run.href.clone(),
                        footnote: None,
                    });
            }
        }
    }

    segments
}

fn extract_decimal_anchor_runs(runs: &[Run]) -> Option<Vec<Run>> {
    let visible_text: String = runs
        .iter()
        .filter(|run| run.footnote.is_none())
        .map(|run| run.text.as_str())
        .collect();
    let separator_offset: usize = find_decimal_separator_offset(&visible_text)?;

    let mut anchor_runs: Vec<Run> = Vec::new();
    let mut visible_offset: usize = 0;

    for run in runs {
        if run.footnote.is_some() {
            anchor_runs.push(run.clone());
            continue;
        }

        let run_end: usize = visible_offset + run.text.len();

        // Entire run falls before the separator — include it whole.
        if run_end <= separator_offset {
            if !run.text.is_empty() {
                anchor_runs.push(run.clone());
            }
            visible_offset = run_end;
            continue;
        }

        // This run spans the separator — include only the portion before it.
        let chars_before_separator: usize = separator_offset.saturating_sub(visible_offset);
        if chars_before_separator > 0 {
            anchor_runs.push(Run {
                text: run.text[..chars_before_separator].to_string(),
                style: run.style.clone(),
                href: run.href.clone(),
                footnote: None,
            });
        }

        return Some(anchor_runs);
    }

    None
}

fn find_decimal_separator_offset(text: &str) -> Option<usize> {
    let separator = text.char_indices().rev().find(|(offset, ch)| {
        matches!(ch, '.' | ',')
            && has_ascii_digit_before(text, *offset)
            && has_ascii_digit_after(text, *offset + ch.len_utf8())
    })?;

    if is_grouped_integer(
        &text
            .chars()
            .filter(|ch| ch.is_ascii_digit() || matches!(ch, '.' | ','))
            .collect::<String>(),
        separator.1,
    ) {
        return None;
    }

    Some(separator.0)
}

fn has_ascii_digit_before(text: &str, offset: usize) -> bool {
    text[..offset].chars().rev().any(|ch| ch.is_ascii_digit())
}

fn has_ascii_digit_after(text: &str, offset: usize) -> bool {
    text[offset..].chars().any(|ch| ch.is_ascii_digit())
}

fn is_grouped_integer(text: &str, separator: char) -> bool {
    if text
        .chars()
        .any(|ch| matches!(ch, '.' | ',') && ch != separator)
    {
        return false;
    }

    let parts: Vec<&str> = text.split(separator).collect();
    parts.len() > 1
        && parts
            .iter()
            .all(|part| !part.is_empty() && part.chars().all(|ch| ch.is_ascii_digit()))
        && parts[1..].iter().all(|part| part.len() == 3)
}

fn build_tab_advance_expr(
    index: usize,
    segment: &[Run],
    tab_stops: Option<&[TabStop]>,
    default_tab_width_pt: f64,
) -> String {
    let prefix_width_var = format!("tab_prefix_width_{index}");
    let segment_width_var = format!("tab_segment_width_{index}");
    let decimal_width_var =
        extract_decimal_anchor_runs(segment).map(|_| format!("tab_decimal_width_{index}"));
    let default_expr = build_default_tab_advance_expr(index, default_tab_width_pt);

    let Some(tab_stops) = tab_stops else {
        return default_expr;
    };

    if tab_stops.is_empty() {
        return default_expr;
    }

    let mut expr = String::new();
    for (stop_index, stop) in tab_stops.iter().enumerate() {
        let branch = format!(
            "calc.max(0pt, {}pt - {prefix_width_var} - {})",
            format_f64(stop.position),
            tab_alignment_offset_expr(stop, &segment_width_var, decimal_width_var.as_deref())
        );

        if stop_index == 0 {
            let _ = write!(
                expr,
                "if {prefix_width_var} < {}pt {{ {branch} }}",
                format_f64(stop.position)
            );
        } else {
            let _ = write!(
                expr,
                " else if {prefix_width_var} < {}pt {{ {branch} }}",
                format_f64(stop.position)
            );
        }
    }

    let _ = write!(expr, " else {{ {default_expr} }}");
    expr
}

fn build_tab_fill_expr(index: usize, tab_stops: Option<&[TabStop]>) -> String {
    let Some(tab_stops) = tab_stops else {
        return format!("h(tab_advance_{index})");
    };

    if tab_stops.is_empty() {
        return format!("h(tab_advance_{index})");
    }

    let prefix_width_var = format!("tab_prefix_width_{index}");
    let mut expr = String::new();
    for (stop_index, stop) in tab_stops.iter().enumerate() {
        let branch = tab_fill_content_expr(index, stop.leader);

        if stop_index == 0 {
            let _ = write!(
                expr,
                "if {prefix_width_var} < {}pt {{ {branch} }}",
                format_f64(stop.position)
            );
        } else {
            let _ = write!(
                expr,
                " else if {prefix_width_var} < {}pt {{ {branch} }}",
                format_f64(stop.position)
            );
        }
    }

    let _ = write!(expr, " else {{ h(tab_advance_{index}) }}");
    expr
}

fn tab_fill_content_expr(index: usize, leader: TabLeader) -> String {
    let leader_markup = match leader {
        TabLeader::None => return format!("h(tab_advance_{index})"),
        TabLeader::Dot => ".",
        TabLeader::Hyphen => "-",
        TabLeader::Underscore => "\\_",
    };

    format!("box(width: tab_advance_{index}, repeat[{leader_markup}])")
}

fn build_default_tab_advance_expr(index: usize, default_tab_width_pt: f64) -> String {
    format!(
        "if tab_default_remainder_{index} == 0 {{ {}pt }} else {{ ({} - tab_default_remainder_{index}) * 1pt }}",
        format_f64(default_tab_width_pt),
        format_f64(default_tab_width_pt)
    )
}

fn tab_alignment_offset_expr(
    stop: &TabStop,
    segment_width_var: &str,
    decimal_width_var: Option<&str>,
) -> String {
    match stop.alignment {
        TabAlignment::Left => "0pt".to_string(),
        TabAlignment::Center => format!("{segment_width_var} / 2"),
        TabAlignment::Right => segment_width_var.to_string(),
        TabAlignment::Decimal => decimal_width_var.unwrap_or(segment_width_var).to_string(),
    }
}

pub(super) fn generate_run(out: &mut String, run: &Run) {
    if let Some(ref content) = run.footnote {
        // The note's runs carry the style its `w:pStyle` and `w:rPr` resolved
        // to, so they emit through the ordinary run path rather than as a bare
        // string that would take the engine's own footnote styling (#580).
        out.push_str("#footnote[");
        // The note's own line box is the engine's, not the referring
        // paragraph's, so a frame here has no edges to restore.
        generate_runs(out, content, EojeolWrap::Syllable);
        out.push(']');
        return;
    }

    if run.text.contains(PPTX_SOFT_LINE_BREAK_CHAR)
        || run.text.contains(HANGUL_KINSOKU_BREAK_CHAR)
        || run.text.contains(EAST_ASIAN_AUTO_SPACE_CHAR)
    {
        write_run_with_break_markers(out, run);
        return;
    }

    write_run_segment(out, run, &run.text);
}

/// Expands the PPTX in-text markers: a soft line break becomes
/// `#linebreak()`, and a kinsoku break marker (issue #438) becomes an
/// empty `#box[]`. An inline frame is a Contingent Break in UAX #14
/// (U+FFFC), so the line may end between a Hangul syllable and its
/// trailing punctuation — which LB13 otherwise forbids. LB13 still glues
/// the mark to the frame, so the two move to the next line together, and
/// the zero-size frame neither disturbs line metrics nor leaves a
/// zero-width space in the PDF text layer.
fn write_run_with_break_markers(out: &mut String, run: &Run) {
    let mut segment_start: usize = 0;

    for (offset, ch) in run.text.char_indices() {
        let auto_space: String;
        let replacement: &str = match ch {
            PPTX_SOFT_LINE_BREAK_CHAR => "#linebreak()",
            HANGUL_KINSOKU_BREAK_CHAR => "#box[]",
            EAST_ASIAN_AUTO_SPACE_CHAR => {
                auto_space = east_asian_auto_space(run);
                &auto_space
            }
            _ => continue,
        };
        if segment_start < offset {
            write_run_segment(out, run, &run.text[segment_start..offset]);
        }
        out.push_str(replacement);
        segment_start = offset + ch.len_utf8();
    }

    if segment_start < run.text.len() {
        write_run_segment(out, run, &run.text[segment_start..]);
    }
}

fn write_run_segment(out: &mut String, run: &Run, text: &str) {
    let style = &run.style;

    let needs_all_caps: bool = matches!(style.all_caps, Some(true));
    let escaped: String = if needs_all_caps {
        escape_typst(&text.to_uppercase())
    } else {
        escape_typst(text)
    };

    let wrappers: Vec<String> = collect_formatting_wrappers(run);

    for wrapper in &wrappers {
        out.push_str(wrapper);
    }

    write_run_content(out, &escaped, style);

    for _ in &wrappers {
        out.push(']');
    }
}

/// Builds the ordered list of `#command[` openers that wrap a run's content.
/// The order matches the original nesting: link > highlight > strike >
/// underline > super/sub > smallcaps.
fn collect_formatting_wrappers(run: &Run) -> Vec<String> {
    let style: &TextStyle = &run.style;
    let mut wrappers: Vec<String> = Vec::new();

    if let Some(ref href) = run.href {
        wrappers.push(format!("#link(\"{href}\")["));
    }
    if let Some(ref highlight) = style.highlight {
        wrappers.push(format!("#highlight(fill: {})[", rgb(highlight)));
    }
    if matches!(style.strikethrough, Some(true)) {
        wrappers.push("#strike[".to_string());
    }
    if matches!(style.underline, Some(true)) {
        wrappers.push("#underline[".to_string());
    }
    if matches!(style.vertical_align, Some(VerticalTextAlign::Superscript)) {
        wrappers.push("#super[".to_string());
    }
    if matches!(style.vertical_align, Some(VerticalTextAlign::Subscript)) {
        wrappers.push("#sub[".to_string());
    }
    if matches!(style.small_caps, Some(true)) {
        wrappers.push("#smallcaps[".to_string());
    }

    wrappers
}

/// Writes the innermost content of a run: either `#text(params)[escaped]`
/// when text properties are present, or the escaped text directly (with a
/// `#[...]` safety wrapper when needed to prevent Typst syntax ambiguity).
fn write_run_content(out: &mut String, escaped: &str, style: &TextStyle) {
    if has_text_properties(style) {
        out.push_str("#text(");
        write_text_params_for_text(out, style, escaped);
        out.push_str(")[");
        out.push_str(escaped);
        out.push(']');
        return;
    }

    let needs_safety_wrap: bool = !escaped.is_empty()
        && out.ends_with(']')
        && !out.ends_with("\\]")
        && matches!(escaped.as_bytes()[0], b'(' | b'.' | b'[');

    if needs_safety_wrap {
        out.push_str("#[");
        out.push_str(escaped);
        out.push(']');
    } else {
        out.push_str(escaped);
    }
}

pub(super) fn has_text_properties(style: &TextStyle) -> bool {
    matches!(style.bold, Some(true))
        || matches!(style.italic, Some(true))
        || style.font_size.is_some()
        || style.color.is_some()
        || style.font_family.is_some()
        || style.letter_spacing.is_some()
}

fn inferred_font_weight(font_family: &str) -> Option<&'static str> {
    let lower = font_family.trim().to_ascii_lowercase();
    if lower.contains("extrabold") || lower.contains("extra bold") {
        Some("extrabold")
    } else if lower.contains("semibold") || lower.contains("semi bold") {
        Some("semibold")
    } else if lower.contains("medium") {
        Some("medium")
    } else if lower.contains("light") {
        Some("light")
    } else {
        None
    }
}

fn font_weight_rank(weight: &str) -> u8 {
    match weight {
        "light" => 1,
        "medium" => 2,
        "semibold" => 3,
        "bold" => 4,
        "extrabold" => 5,
        "black" => 6,
        _ => 0,
    }
}

fn effective_font_weight(style: &TextStyle) -> Option<&'static str> {
    // Only infer weight from font family name when the font (or its alias)
    // is actually available.  When using fallback fonts, uncommonly heavy
    // weights (e.g. "extrabold" = 800) may not exist in the substitute,
    // causing Typst to fall back to its built-in serif font instead.
    let inferred = style.font_family.as_deref().and_then(|family| {
        if font_subst::is_primary_font_available(family) {
            inferred_font_weight(family)
        } else {
            None
        }
    });
    let explicit = matches!(style.bold, Some(true)).then_some("bold");
    match (explicit, inferred) {
        (Some(explicit), Some(inferred)) => {
            if font_weight_rank(explicit) >= font_weight_rank(inferred) {
                Some(explicit)
            } else {
                Some(inferred)
            }
        }
        (Some(explicit), None) => Some(explicit),
        (None, Some(inferred)) => Some(inferred),
        (None, None) => None,
    }
}

pub(super) fn write_text_params(out: &mut String, style: &TextStyle) {
    write_text_params_for_text(out, style, "");
}

/// As [`write_text_params`], but told what the run holds.
///
/// The font list has to answer for the script the text is written in, not only
/// for the family it names: a run can declare a face that has no glyph for its
/// own content (issues #537, #543).
pub(super) fn write_text_params_for_text(out: &mut String, style: &TextStyle, text: &str) {
    let mut first = true;

    if let Some(ref family) = style.font_family {
        let font_value = match style.east_asian_font_family {
            Some(ref east_asian) if !east_asian.eq_ignore_ascii_case(family) => {
                font_subst::font_with_east_asian_fallbacks(family, east_asian, text)
            }
            _ => font_subst::font_with_fallbacks_for_text(family, text),
        };
        write_param(out, &mut first, &format!("font: {font_value}"));
    }
    if let Some(size) = style.font_size {
        write_param(out, &mut first, &format!("size: {}pt", format_f64(size)));
    }
    if let Some(weight) = effective_font_weight(style) {
        write_param(out, &mut first, &format!("weight: \"{weight}\""));
    }
    if matches!(style.italic, Some(true)) {
        write_param(out, &mut first, "style: \"italic\"");
    }
    if let Some(ref color) = style.color {
        write_param(out, &mut first, &format_color(color));
    }
    if let Some(spacing) = style.letter_spacing {
        write_param(
            out,
            &mut first,
            &format!("tracking: {}pt", format_f64(spacing)),
        );
    }
}

pub(super) fn write_param(out: &mut String, first: &mut bool, param: &str) {
    if !*first {
        out.push_str(", ");
    }
    out.push_str(param);
    *first = false;
}

pub(super) fn format_color(color: &Color) -> String {
    format!("fill: {}", rgb(color))
}

/// The char index Typst reads a *line-leading* markup marker at.
///
/// Typst recognises those markers through one leading space, so the scan
/// steps over one. Which text lands at the start of an escaping unit is the
/// generator's choice, not the document's: a run is cut at every tab, at
/// every in-text marker, and since #626 at every eojeol boundary, so a
/// paragraph's ` + ` or ` = ` reaches [`escape_typst`] as a unit of its own.
///
/// Exactly one space, because that is the only leading whitespace
/// [`escape_typst`] emits as markup. A run of two or more — and any run after
/// a hard break — leaves as a code-mode string, which cannot open a marker.
/// Measured on typst: `[ 2026. 7. 17.]`, `[ + x]` and `[ = x]` become an
/// enumeration, a list item and a heading; `[#"  ";+ x]`, `[#"  ";= x]` and a
/// leading U+00A0 do not.
fn line_leading_markup_index(text: &str) -> usize {
    usize::from(text.starts_with(' ') && !text[1..].starts_with(' '))
}

/// Whether `text` opens with a Typst line-leading marker whose first
/// character must be escaped to neutralise it.
///
/// The full set of Typst markup that is only meaningful at a line start:
///
/// | Marker | Handling |
/// | --- | --- |
/// | `- ` bullet list | here, and also escaped everywhere else (`--` ligates) |
/// | `+ ` numbered list | here — `+` is otherwise a literal |
/// | `= ` heading (any run of `=`) | here — `=` is otherwise a literal |
/// | `/ ` term list | already escaped unconditionally below |
/// | `<digits>. ` enumeration | [`enum_marker_dot`], which escapes the dot |
///
/// Every other Typst shorthand (`#`, `*`, `_`, `` ` ``, `$`, `<`, `>`, `@`,
/// `~`, `\`, `[`, `]`, `{`, `}`, `"`, `'`) is markup wherever it appears and
/// is escaped unconditionally, so a line start needs no extra rule for it.
///
/// A marker also needs trailing whitespace to be one: `[ =x]` and `[+]` stay
/// literal. Escaping only the *first* character of a `==`-style run is
/// enough — measured on typst, `[ \== ]` renders ` == ` — because what
/// remains no longer starts the line.
fn opens_line_leading_marker(text: &str) -> bool {
    match text.chars().next() {
        // A one-byte marker char, so the byte slice is safe.
        Some('-' | '+') => text[1..].chars().next().is_some_and(char::is_whitespace),
        Some('=') => text
            .trim_start_matches('=')
            .chars()
            .next()
            .is_some_and(char::is_whitespace),
        _ => false,
    }
}

pub(super) fn escape_typst(text: &str) -> String {
    let normalized_text: String = text.nfc().collect();
    let leading_space: usize = line_leading_markup_index(&normalized_text);
    let after_space: &str = &normalized_text[leading_space..];

    // A leading `-`/`+` bullet or `=` heading run would be re-typeset as that
    // marker, deleting the character from the page: ` + ` between two Korean
    // eojeol became an enumeration item and ` = ` an empty heading (#626).
    let line_leading_marker: Option<usize> =
        opens_line_leading_marker(after_space).then_some(leading_space);

    // A leading "<digits>. " would be re-typeset as a Typst numbered-list
    // marker (e.g. "2026. 07. 17." became "2026. 7. 17."); escape its dot.
    // `"시행일자: 2026. 7. 17."` reached this function as `" 2026. 7. 17."`
    // once #626 cut the run at the eojeol boundary, and Typst put the date on
    // an enumeration line of its own.
    let enum_marker_dot: Option<usize> = {
        let digit_count = after_space
            .chars()
            .take_while(|c| c.is_ascii_digit())
            .count();
        let rest = &after_space[digit_count..];
        if digit_count > 0 && (rest.starts_with(". ") || rest == ".") {
            Some(leading_space + digit_count)
        } else {
            None
        }
    };

    let mut result = String::with_capacity(normalized_text.len());
    let mut chars = normalized_text.chars().peekable();
    let mut char_index: usize = 0;

    let mut after_linebreak = false;
    while let Some(ch) = chars.next() {
        let should_escape_list_prefix: bool = line_leading_marker == Some(char_index);

        match ch {
            // A hard line break (`<w:br/>`, carried through the IR as '\n') must
            // force a new line. A bare newline in Typst markup collapses to a
            // space, which silently merged code lines like `echo` / `printf`
            // (issue #176).
            '\n' => result.push_str("#linebreak()"),
            '\r' => {}
            // Word preserves literal space runs (xml:space="preserve") that
            // documents use for manual alignment and code indentation; Typst
            // markup collapses consecutive and line-leading spaces to one.
            // Emit runs of two or more — and post-break indentation — as a
            // code-mode string, which markup cannot collapse (issue #352).
            // Single run-leading spaces stay literal: they sit between
            // sibling runs in the same markup line and survive as-is.
            ' ' if after_linebreak || chars.peek().is_some_and(|next| *next == ' ') => {
                let mut run_len: usize = 1;
                while chars.peek().is_some_and(|next| *next == ' ') {
                    chars.next();
                    run_len += 1;
                    char_index += 1;
                }
                result.push_str("#\"");
                result.push_str(&" ".repeat(run_len));
                // The semicolon ends the code expression: without it, a
                // following `(` or `[` in the text would chain onto the
                // string as a function call (`#"  "(SIB)`).
                result.push_str("\";");
            }
            // Quotes and hyphens are Typst markup shorthands: smartquote
            // curls straight quotes, `--` ligates to an en dash, and a
            // hyphen before digits becomes a Unicode minus. Word stores the
            // literal characters the author typed, so all of them must
            // render verbatim (issue #353).
            '#' | '*' | '_' | '`' | '<' | '>' | '@' | '\\' | '~' | '/' | '$' | '[' | ']' | '{'
            | '}' | '"' | '\'' | '-'
                if !should_escape_list_prefix =>
            {
                result.push('\\');
                result.push(ch);
            }
            _ if should_escape_list_prefix => {
                result.push('\\');
                result.push(ch);
            }
            '.' if enum_marker_dot == Some(char_index) => {
                result.push('\\');
                result.push('.');
            }
            _ => result.push(ch),
        }

        after_linebreak = ch == '\n';
        char_index += 1;
    }
    result
}

/// The Typst label a heading's contents-entry marker carries.
pub(super) const TOC_ENTRY_LABEL: &str = "o2p-toc";

/// A heading's text with no markup, for the contents entry that points at it.
fn paragraph_plain_text(runs: &[Run]) -> String {
    runs.iter().map(|run| run.text.as_str()).collect()
}

/// Escape a Rust string for use inside a Typst double-quoted string literal.
fn escape_typst_string(text: &str) -> String {
    text.replace('\\', "\\\\").replace('"', "\\\"")
}

/// The family the heading's first run names, for the contents entry's own
/// fallback chain — a Korean entry needs the Korean face even though the entry
/// is laid out at body size (issue #610).
fn first_run_family(runs: &[Run]) -> Option<&str> {
    runs.iter().find_map(|run| run.style.font_family.as_deref())
}
