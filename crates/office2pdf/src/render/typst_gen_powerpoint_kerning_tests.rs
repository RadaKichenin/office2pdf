use super::*;
use typst::foundations::Bytes;
use typst::text::Font;

// Native Arial probes expose pairs on either side of a space. Attach that
// topology to a bundled face so the regression does not require Office fonts.
const PAIRS: [(char, char, i16); 3] = [(' ', 'A', -100), ('A', ' ', -60), ('Y', ' ', -35)];

fn space_kern_font() -> Font {
    let data = include_bytes!("../../fonts/NotoSansCJKsc-GB2312.otf");
    let base = Font::new(Bytes::new(data.to_vec()), 0).unwrap();
    let mut pairs: Vec<(u16, u16, i16)> = PAIRS
        .iter()
        .map(|&(left, right, value)| {
            (
                base.ttf().glyph_index(left).unwrap().0,
                base.ttf().glyph_index(right).unwrap().0,
                value,
            )
        })
        .collect();
    pairs.sort_by_key(|&(left, right, _)| (left, right));
    let count = pairs.len() as u16;
    let selector = count.ilog2() as u16;
    let search_range = (1u16 << selector) * 6;
    let mut kern = Vec::new();
    for value in [
        0u16,
        1,
        0,
        14 + count * 6,
        1,
        count,
        search_range,
        selector,
        count * 6 - search_range,
    ] {
        kern.extend_from_slice(&value.to_be_bytes());
    }
    for (left, right, value) in pairs {
        kern.extend_from_slice(&left.to_be_bytes());
        kern.extend_from_slice(&right.to_be_bytes());
        kern.extend_from_slice(&value.to_be_bytes());
    }
    Font::new(
        Bytes::new(crate::test_support::make_face_with_legacy_kern_table(
            data, &kern,
        )),
        0,
    )
    .unwrap()
}

fn grid_width(font: &Font, text: &str, size: f64, kerning: bool) -> f64 {
    let face = font.ttf();
    let units = f64::from(face.units_per_em());
    let nominal: f64 = text
        .chars()
        .map(|ch| {
            let glyph = face.glyph_index(ch).unwrap();
            (f64::from(face.glyph_hor_advance(glyph).unwrap()) / units * size * 8.0).round() / 8.0
        })
        .sum();
    let adjustment: f64 = text
        .chars()
        .zip(text.chars().skip(1))
        .flat_map(|(left, right)| {
            PAIRS
                .iter()
                .filter(move |&&(a, b, _)| a == left && b == right)
                .map(|&(_, _, value)| f64::from(value) / units * size)
        })
        .sum();
    nominal + if kerning { adjustment } else { 0.0 }
}

fn place_text(
    font: &Font,
    text: &str,
    size: f64,
    width: f64,
    kerning: bool,
    marker: bool,
) -> Vec<crate::render::pdf::PlacedTextRun> {
    let style = TextStyle {
        font_family: Some(font.info().family.clone()),
        font_size: Some(size),
        pair_kerning: Some(if kerning {
            crate::ir::PairKerning::AtOrAbovePt(1.0)
        } else {
            crate::ir::PairKerning::Never
        }),
        ..TextStyle::default()
    };
    let mut runs = vec![Run {
        text: text.into(),
        style: style.clone(),
        href: None,
        footnote: None,
    }];
    if marker {
        runs.push(Run {
            text: "END".into(),
            style: TextStyle {
                bold: Some(true),
                ..style
            },
            href: None,
            footnote: None,
        });
    }
    let document = make_doc(vec![make_fixed_page(
        960.0,
        540.0,
        vec![make_fixed_text_box(
            72.0,
            72.0,
            width,
            300.0,
            Insets::default(),
            crate::ir::TextBoxVerticalAlign::Top,
            vec![Block::Paragraph(Paragraph {
                style: ParagraphStyle::default(),
                runs,
            })],
        )],
    )]);
    let fonts = [font.clone()];
    let context = crate::render::font_context::resolve_font_search_context_from_fonts(&fonts);
    let output = crate::render::font_subst::with_font_search_context(Some(&context), || {
        generate_typst(&document).unwrap()
    });
    crate::render::pdf::compiled_text_runs_with_fonts(&output.source, 0, &fonts).unwrap()
}

#[test]
fn powerpoint_preserves_pair_kerning_on_both_sides_of_spaces() {
    let font = space_kern_font();
    let mut failures = Vec::new();
    for size in [12.0, 17.0] {
        for kerning in [false, true] {
            for text in ["on Aug ", "A Aug ", "Y Aug ", "A  Aug ", "on Hug "] {
                let runs = place_text(&font, text, size, 800.0, kerning, true);
                let end = runs.iter().find(|run| run.text == "END").unwrap();
                let expected = 72.0 + grid_width(&font, text, size, kerning);
                if (end.left_pt - expected).abs() > 0.01 {
                    failures.push(format!(
                        "{text:?} {size}pt kern={kerning}: end={}, expected={expected}",
                        end.left_pt
                    ));
                }
            }
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn powerpoint_space_pairs_affect_wraps_without_shifting_new_line_starts() {
    let font = space_kern_font();
    let size = 17.0;
    let first_line = "A Aug";
    let kerned = grid_width(&font, first_line, size, true);
    let unkerned = grid_width(&font, first_line, size, false);
    assert!(unkerned - kerned > 1.0);
    for (width, kerning, expected) in [
        ((kerned + unkerned) / 2.0, true, vec!["A Aug", "20"]),
        ((kerned + unkerned) / 2.0, false, vec!["A", "Aug", "20"]),
        (kerned - 1.0, true, vec!["A", "Aug", "20"]),
    ] {
        let runs = place_text(&font, "A Aug 20", size, width, kerning, false);
        let mut lines: Vec<(f64, f64, String)> = Vec::new();
        for run in runs {
            if let Some((_, _, text)) = lines
                .iter_mut()
                .find(|(y, _, _)| (*y - run.baseline_pt).abs() < 0.01)
            {
                text.push_str(&run.text);
            } else {
                lines.push((run.baseline_pt, run.left_pt, run.text));
            }
        }
        lines.sort_by(|left, right| left.0.total_cmp(&right.0));
        assert_eq!(
            lines
                .iter()
                .map(|(_, _, text)| text.trim())
                .collect::<Vec<_>>(),
            expected,
            "width={width}, kern={kerning}: {lines:?}"
        );
        for (_, x, text) in &lines {
            assert!((x - 72.0).abs() < 0.01, "new line {text:?}: x={x}");
        }
    }
}
