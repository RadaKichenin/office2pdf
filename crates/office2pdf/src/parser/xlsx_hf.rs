use crate::error::ConvertWarning;
use crate::ir::{
    Alignment, HFInline, HeaderFooter, HeaderFooterParagraph, ParagraphStyle, Run, TextStyle,
};

/// Parse an Excel header/footer format string into IR HeaderFooter.
///
/// Excel format strings use `&L`, `&C`, `&R` to define left/center/right sections,
/// `&P` for current page number, `&N` for total page count, and `&A` for the
/// worksheet name.
///
/// `sheet_name` resolves `&A`. It is a component of Excel's built-in "Sheet
/// name" header, so it turns up in files nobody customised.
///
/// Codes naming data this parser does not hold now warn instead of vanishing
/// (issue #690). `&F` and `&Z` want the workbook's file name and path, which
/// never reach `Parser::parse` — it takes bytes. `&D` and `&T` are Excel's
/// *print* date and time, which a deterministic converter has no defensible
/// value for. `&G` is a picture.
///
/// Returns `None` if the format string is empty.
pub(super) fn parse_hf_format_string(
    format_str: &str,
    sheet_name: &str,
    warnings: &mut Vec<ConvertWarning>,
) -> Option<HeaderFooter> {
    let s = format_str.trim();
    if s.is_empty() {
        return None;
    }

    // Split into left/center/right sections
    let mut left = String::new();
    let mut center = String::new();
    let mut right = String::new();
    let mut current = &mut center; // Default section is center if no &L/&C/&R prefix

    let chars: Vec<char> = s.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        if chars[i] == '&' && i + 1 < chars.len() {
            match chars[i + 1] {
                'L' => {
                    current = &mut left;
                    i += 2;
                }
                'C' => {
                    current = &mut center;
                    i += 2;
                }
                'R' => {
                    current = &mut right;
                    i += 2;
                }
                'P' => {
                    current.push('\x01'); // Sentinel for page number
                    i += 2;
                }
                'N' => {
                    current.push('\x02'); // Sentinel for total pages
                    i += 2;
                }
                '&' => {
                    // Escaped ampersand: && → &
                    current.push('&');
                    i += 2;
                }
                '"' => {
                    // Font name: &"FontName" — skip to closing quote
                    i += 2; // skip &"
                    while i < chars.len() && chars[i] != '"' {
                        i += 1;
                    }
                    if i < chars.len() {
                        i += 1; // skip closing "
                    }
                }
                c if c.is_ascii_digit() => {
                    // Font size: &NN — skip digits
                    i += 1; // skip &
                    while i < chars.len() && chars[i].is_ascii_digit() {
                        i += 1;
                    }
                }
                'K' => {
                    // Font color: &KRRGGBB (or &KTTSNN theme form) — skip the
                    // six code characters; leaving them printed literal hex
                    // prefixes like "000000top center".
                    i += 2;
                    let mut consumed = 0;
                    while i < chars.len() && consumed < 6 && chars[i].is_ascii_alphanumeric() {
                        i += 1;
                        consumed += 1;
                    }
                }
                'A' => {
                    // Worksheet name. Excel's built-in "Sheet name" header is
                    // `&C&A`, so this turns up in files nobody customised; it
                    // used to fall through the catch-all and, being the whole
                    // section, took the paragraph with it (issue #690).
                    current.push_str(sheet_name);
                    i += 2;
                }
                'F' | 'Z' | 'D' | 'T' | 'G' => {
                    // Codes that name text Excel prints but this parser cannot
                    // produce. Report them rather than dropping them silently,
                    // so a missing header is traceable to its cause.
                    let described = match chars[i + 1] {
                        // `Parser::parse` takes bytes; no source path reaches it.
                        'F' => "&F (file name)",
                        'Z' => "&Z (file path)",
                        // Excel's *print* date/time. A converter that must be
                        // deterministic — byte-identical output for identical
                        // input — has no defensible value to substitute.
                        'D' => "&D (print date)",
                        'T' => "&T (print time)",
                        _ => "&G (picture)",
                    };
                    warnings.push(ConvertWarning::UnsupportedElement {
                        format: "XLSX".to_string(),
                        element: format!("header/footer field code {described}"),
                    });
                    i += 2;
                }
                _ => {
                    // The remaining codes are formatting toggles — &B bold,
                    // &I italic, &U underline, &E double underline, &S strike,
                    // &X superscript, &Y subscript, &O outline, &H shadow —
                    // plus section separators this parser does not model. They
                    // carry no text of their own, so skipping one changes how
                    // the header looks, never whether it appears. That is why
                    // they stay silent while the codes above warn.
                    i += 2;
                }
            }
        } else {
            current.push(chars[i]);
            i += 1;
        }
    }

    let mut paragraphs = Vec::new();

    // Build paragraph for each non-empty section
    let sections = [
        (&left, Alignment::Left),
        (&center, Alignment::Center),
        (&right, Alignment::Right),
    ];

    for (text, alignment) in &sections {
        if text.is_empty() {
            continue;
        }
        let elements = build_hf_elements(text);
        if !elements.is_empty() {
            paragraphs.push(HeaderFooterParagraph {
                style: ParagraphStyle {
                    alignment: Some(*alignment),
                    ..ParagraphStyle::default()
                },
                elements,
                border: None,
                border_space: None,
                frame: None,
            });
        }
    }

    if paragraphs.is_empty() {
        None
    } else {
        Some(HeaderFooter {
            paragraphs,
            distance_from_edge: None,
        })
    }
}

/// Build HFInline elements from a section string, replacing sentinel chars.
pub(super) fn build_hf_elements(section: &str) -> Vec<HFInline> {
    let mut elements = Vec::new();
    let mut current_text = String::new();

    for ch in section.chars() {
        match ch {
            '\x01' => {
                // Page number sentinel
                if !current_text.is_empty() {
                    elements.push(HFInline::Run(Run {
                        text: std::mem::take(&mut current_text),
                        style: TextStyle::default(),
                        href: None,
                        footnote: None,
                    }));
                }
                elements.push(HFInline::PageNumber(TextStyle::default()));
            }
            '\x02' => {
                // Total pages sentinel
                if !current_text.is_empty() {
                    elements.push(HFInline::Run(Run {
                        text: std::mem::take(&mut current_text),
                        style: TextStyle::default(),
                        href: None,
                        footnote: None,
                    }));
                }
                elements.push(HFInline::TotalPages(TextStyle::default()));
            }
            _ => {
                current_text.push(ch);
            }
        }
    }

    if !current_text.is_empty() {
        elements.push(HFInline::Run(Run {
            text: current_text,
            style: TextStyle::default(),
            href: None,
            footnote: None,
        }));
    }

    elements
}
