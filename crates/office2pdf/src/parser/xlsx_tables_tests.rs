use super::*;

/// Office's default palette, as the audited workbooks' themes declare it —
/// six accents over a white `lt1` and a black `dk1`.
fn palette() -> StylePalette {
    StylePalette {
        accents: vec![
            Color::new(0x4f, 0x81, 0xbd), // accent1
            Color::new(0xc0, 0x50, 0x4d), // accent2
            Color::new(0x9b, 0xbb, 0x59), // accent3
            Color::new(0x80, 0x64, 0xa2), // accent4
            Color::new(0x4b, 0xac, 0xc6), // accent5
            Color::new(0xf7, 0x96, 0x46), // accent6
        ],
        light: Color::white(),
        dark: Color::black(),
    }
}

/// The audited workbook's table part, trimmed to the elements that matter.
fn module_inventory_table(style_info: &str) -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
        <table xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main" id="1"
               name="ModuleInventory" displayName="ModuleInventory" ref="A4:H175"
               headerRowCount="1" totalsRowCount="0">
          <autoFilter ref="A4:H175"/>
          {style_info}
        </table>"#
    )
}

/// The colours a native Excel-for-Mac export prints for a theme colour tinted
/// by each amount the built-in table styles use.
///
/// Three accents from two Office themes plus the neutral shade, measured off
/// `mutool draw -F trace` of the same table restyled through Excel 16
/// (issue #1125). Together they pin the whole tint model: nothing but the
/// 240-step HLS range with a truncated luminance reproduces all seven.
fn measured_tints() -> [(Color, f64, Color); 7] {
    [
        // accent 1 of the Office 2007 theme, banded and ruled
        (
            Color::new(0x4f, 0x81, 0xbd),
            0.8,
            Color::new(0xdc, 0xe6, 0xf1),
        ),
        (
            Color::new(0x4f, 0x81, 0xbd),
            0.4,
            Color::new(0x95, 0xb3, 0xd7),
        ),
        // accent 2 of the same theme, from the TableStyleMedium3 variant
        (
            Color::new(0xc0, 0x50, 0x4d),
            0.8,
            Color::new(0xf2, 0xdc, 0xdb),
        ),
        (
            Color::new(0xc0, 0x50, 0x4d),
            0.4,
            Color::new(0xda, 0x96, 0x94),
        ),
        // accent 1 of the current Office theme, from `SH001-Table.xlsx`
        (
            Color::new(0x5b, 0x9b, 0xd5),
            0.8,
            Color::new(0xdd, 0xeb, 0xf7),
        ),
        (
            Color::new(0x5b, 0x9b, 0xd5),
            0.4,
            Color::new(0x9b, 0xc2, 0xe6),
        ),
        // `lt1` shaded, which is the band an accent-less style prints
        (Color::white(), -0.15, Color::new(0xd9, 0xd9, 0xd9)),
    ]
}

#[test]
fn tinting_reproduces_every_measured_native_export_colour() {
    for (source, amount, expected) in measured_tints() {
        assert_eq!(
            tint(source, amount),
            expected,
            "{source:?} tinted {amount} should print {expected:?}"
        );
    }
}

#[test]
fn a_medium2_table_stripes_in_accent1_at_a_twenty_percent_tint() {
    // Excel paints DCE6F1 for TableStyleMedium2 against a 4F81BD accent 1,
    // which is the accent moved 80% of the way to white on Excel's HLS range.
    let xml = module_inventory_table(
        r#"<tableStyleInfo name="TableStyleMedium2" showRowStripes="1" showColumnStripes="0"/>"#,
    );

    let stripes = parse_table_part(&xml, &palette()).expect("stripes are declared");

    assert_eq!(stripes.fill_at(1, 5), Some(Color::new(0xdc, 0xe6, 0xf1)));
}

#[test]
fn stripes_start_at_the_first_body_row_and_alternate() {
    // The header occupies row 4, so row 5 is the first body row — and Excel
    // shades it, then every second row after it.
    let xml =
        module_inventory_table(r#"<tableStyleInfo name="TableStyleMedium2" showRowStripes="1"/>"#);

    let stripes = parse_table_part(&xml, &palette()).unwrap();

    assert_eq!(
        stripes.fill_at(1, 4),
        Some(palette().accents[0]),
        "the header row takes the style's own fill, not the band"
    );
    assert!(stripes.fill_at(1, 5).is_some());
    assert!(stripes.fill_at(1, 6).is_none());
    assert!(stripes.fill_at(1, 7).is_some());
    assert!(
        stripes.fill_at(1, 175).is_some(),
        "the last body row is 175"
    );
    assert!(stripes.fill_at(1, 176).is_none(), "past the table range");
}

#[test]
fn stripes_stay_inside_the_tables_columns() {
    let xml =
        module_inventory_table(r#"<tableStyleInfo name="TableStyleMedium2" showRowStripes="1"/>"#);

    let stripes = parse_table_part(&xml, &palette()).unwrap();

    assert!(stripes.fill_at(8, 5).is_some(), "column H is the last one");
    assert!(stripes.fill_at(9, 5).is_none(), "column I is outside");
}

#[test]
fn a_table_that_asks_for_no_row_stripes_keeps_the_rest_of_its_style() {
    // `showRowStripes` scopes the banding alone, so a Medium2 table with it
    // off still fills and rules the header row the native export fills.
    let xml = module_inventory_table(
        r#"<tableStyleInfo name="TableStyleMedium2" showRowStripes="0" showColumnStripes="1"/>"#,
    );

    let style = parse_table_part(&xml, &palette()).expect("the header paint survives");

    assert_eq!(style.fill_at(1, 5), None, "no band without row stripes");
    assert_eq!(style.fill_at(1, 4), Some(palette().accents[0]));
    assert!(style.border_at(1, 4).is_some());
}

#[test]
fn a_later_medium_band_bands_its_rows_and_paints_nothing_else() {
    // The same probe shows Medium8..28 painting something else entirely —
    // Medium9 fills every row in two tints, Medium15 boxes the table in 2pt
    // rules, Medium22 bands in greys — so only the banding issue #532
    // resolved for them stands, and the band-one header treatment must not
    // leak onto them.
    for style_name in [
        "TableStyleMedium9",
        "TableStyleMedium16",
        "TableStyleMedium23",
    ] {
        let paint = built_in_table_style(style_name, &palette())
            .unwrap_or_else(|| panic!("{style_name} keeps its banding"));

        assert_eq!(
            paint.stripe,
            Some(tint(palette().accents[0], 0.8)),
            "{style_name}"
        );
        assert_eq!(paint.rule, None, "{style_name} rules nothing we measured");
        assert_eq!(
            paint.header, None,
            "{style_name} paints no header we measured"
        );
    }
}

#[test]
fn medium_styles_walk_the_accents_in_bands_of_seven() {
    // Medium2..7 take accent 1..6; Medium8 restarts the band on the dark
    // style, and Medium9 is accent 1 again.
    for (style, accent_index) in [
        ("TableStyleMedium2", 0),
        ("TableStyleMedium7", 5),
        ("TableStyleMedium9", 0),
        ("TableStyleMedium14", 5),
    ] {
        let expected = tint(palette().accents[accent_index], 0.8);

        assert_eq!(
            built_in_table_style(style, &palette()).and_then(|paint| paint.stripe),
            Some(expected),
            "{style} should stripe in accent {}",
            accent_index + 1
        );
    }

    for dark_style in ["TableStyleMedium8", "TableStyleMedium15"] {
        assert_eq!(
            built_in_table_style(dark_style, &palette()),
            None,
            "{dark_style} opens an unmeasured band on the dark style"
        );
    }
}

#[test]
fn medium_band_one_opens_on_the_dark_style() {
    // Medium1 is the band's accent-less member: the native export fills its
    // header row and rules it in `dk1`, and bands the body out of `lt1` the
    // same 15% Light1 does.
    assert_eq!(
        built_in_table_style("TableStyleMedium1", &palette()),
        Some(TableStylePaint {
            stripe: Some(Color::new(0xd9, 0xd9, 0xd9)),
            rule: Some(TableRule {
                color: Color::black(),
                extent: RuleExtent::EveryRowAndOuterEdges,
            }),
            header: Some(HeaderPaint {
                fill: Color::black(),
                text: Color::white(),
            }),
        })
    );
}

#[test]
fn a_medium2_table_fills_its_header_row_in_accent1_and_prints_it_white() {
    // The native export paints G1's header band solid #4F81BD and sets its
    // run in white bold — the whole treatment issue #1125 reports missing.
    let xml =
        module_inventory_table(r#"<tableStyleInfo name="TableStyleMedium2" showRowStripes="1"/>"#);

    let style = parse_table_part(&xml, &palette()).unwrap();

    assert_eq!(
        style.fill_at(1, 4),
        Some(Color::new(0x4f, 0x81, 0xbd)),
        "the header row is filled in the accent itself"
    );
    assert_eq!(style.header_text_color_at(1, 4), Some(Color::white()));
    assert!(style.bolds_header_at(1, 4));

    assert_eq!(
        style.header_text_color_at(1, 5),
        None,
        "a body row keeps its own ink"
    );
    assert_eq!(
        style.header_text_color_at(9, 4),
        None,
        "column I is outside the table"
    );
}

#[test]
fn a_light1_header_row_is_bolded_but_left_in_its_own_ink() {
    // Light1's header carries no fill, so there is nothing for white text to
    // sit on and the export prints it black.
    let xml =
        module_inventory_table(r#"<tableStyleInfo name="TableStyleLight1" showRowStripes="1"/>"#);

    let style = parse_table_part(&xml, &palette()).unwrap();

    assert!(style.bolds_header_at(1, 4));
    assert_eq!(style.header_text_color_at(1, 4), None);
    assert_eq!(style.fill_at(1, 4), None, "the header row is unfilled");
}

#[test]
fn a_medium2_table_rules_every_row_boundary_and_both_outer_edges() {
    // Against Light1's three rules, the native Medium2 export lays a #95B3D7
    // 1pt band on every row boundary and runs one down the table's left and
    // right edges (issue #1125).
    let xml =
        module_inventory_table(r#"<tableStyleInfo name="TableStyleMedium2" showRowStripes="1"/>"#);

    let style = parse_table_part(&xml, &palette()).unwrap();
    let rule_color = Color::new(0x95, 0xb3, 0xd7);

    let header = style.border_at(1, 4).expect("the header row is ruled");
    let top = header.top.as_ref().expect("a rule sits above the header");
    assert_eq!(top.color, rule_color, "Medium2 rules in the accent at 40%");
    assert_eq!(top.width, 1.0, "a thin Excel border is a 1pt band");
    assert_eq!(top.style, BorderLineStyle::Solid);
    assert!(header.bottom.is_some(), "the header is closed by a rule");

    let interior = style
        .border_at(4, 100)
        .expect("an interior body row is ruled too");
    assert!(
        interior.bottom.is_some(),
        "every row boundary carries a rule"
    );
    assert!(interior.top.is_none(), "the rule hangs below each row");
    assert!(
        interior.left.is_none() && interior.right.is_none(),
        "column D is neither outer edge"
    );

    let first_column = style.border_at(1, 100).expect("column A is the left edge");
    assert_eq!(
        first_column.left.as_ref().map(|side| side.color),
        Some(rule_color)
    );
    assert!(first_column.right.is_none());

    let last_column = style.border_at(8, 100).expect("column H is the right edge");
    assert_eq!(
        last_column.right.as_ref().map(|side| side.color),
        Some(rule_color)
    );
    assert!(last_column.left.is_none());

    assert!(
        style.border_at(9, 100).is_none(),
        "column I is outside the table"
    );
    assert!(style.border_at(1, 176).is_none(), "past the last row");
}

#[test]
fn light_band_one_walks_the_accents_like_medium_does() {
    // Light1 opens the band on the neutral style: `lt1` shaded 15% over `dk1`
    // rules, which is the #D9D9D9 on black the native export prints.
    assert_eq!(
        built_in_table_style("TableStyleLight1", &palette()),
        Some(TableStylePaint {
            stripe: Some(Color::new(0xd9, 0xd9, 0xd9)),
            rule: Some(TableRule {
                color: Color::black(),
                extent: RuleExtent::HeaderAndFoot,
            }),
            header: None,
        })
    );

    // Light2..7 take accent 1..6, banding at the same 20% tint as Medium and
    // ruling in the accent itself.
    for (style, accent_index) in [("TableStyleLight2", 0), ("TableStyleLight7", 5)] {
        let accent = palette().accents[accent_index];

        assert_eq!(
            built_in_table_style(style, &palette()),
            Some(TableStylePaint {
                stripe: Some(tint(accent, 0.8)),
                rule: Some(TableRule {
                    color: accent,
                    extent: RuleExtent::HeaderAndFoot,
                }),
                header: None,
            }),
            "{style} should paint in accent {}",
            accent_index + 1
        );
    }
}

#[test]
fn an_unresolved_style_family_leaves_the_table_unpainted() {
    // The probe behind issue #1080 shows the Light family's later bands
    // painting something else entirely — Light8 fills its header row solid
    // and rules every row, Light15 boxes the whole table — and the Dark
    // family is not measured at all. Each must fall back to today's
    // behaviour rather than to a guess.
    for style in [
        "TableStyleLight8",
        "TableStyleLight15",
        "TableStyleDark3",
        "MyCustomStyle",
    ] {
        assert_eq!(built_in_table_style(style, &palette()), None, "{style}");
    }
}

#[test]
fn a_light1_table_rules_its_header_and_its_foot_and_nothing_between() {
    let xml =
        module_inventory_table(r#"<tableStyleInfo name="TableStyleLight1" showRowStripes="1"/>"#);

    let style = parse_table_part(&xml, &palette()).expect("Light1 paints its rules");

    let header = style
        .border_at(1, 4)
        .expect("the header row is ruled twice");
    let top: &BorderSide = header.top.as_ref().expect("a rule sits above the header");
    assert_eq!(top.color, Color::black(), "Light1 rules in `dk1`");
    assert_eq!(top.width, 1.0, "a thin Excel border is a 1pt band");
    assert_eq!(top.style, BorderLineStyle::Solid);
    assert!(header.bottom.is_some(), "the header is closed by a rule");
    assert!(
        header.left.is_none() && header.right.is_none(),
        "Light1 draws no verticals"
    );

    assert!(
        style.border_at(1, 5).is_none(),
        "the first body row carries no rule"
    );
    assert!(
        style.border_at(1, 174).is_none(),
        "nor does the second-to-last"
    );

    let foot = style.border_at(1, 175).expect("the table's foot is ruled");
    assert!(foot.bottom.is_some());
    assert!(foot.top.is_none(), "the foot rule hangs below the last row");

    assert!(
        style.border_at(9, 4).is_none(),
        "column I is outside the table"
    );
    assert!(
        style.border_at(1, 176).is_none(),
        "past the table's last row"
    );
}

#[test]
fn a_light1_table_prints_its_header_row_bold() {
    let xml =
        module_inventory_table(r#"<tableStyleInfo name="TableStyleLight1" showRowStripes="1"/>"#);

    let style = parse_table_part(&xml, &palette()).unwrap();

    assert!(style.bolds_header_at(1, 4), "row 4 is the header row");
    assert!(!style.bolds_header_at(1, 5), "row 5 is the first body row");
    assert!(
        !style.bolds_header_at(9, 4),
        "column I is outside the table"
    );
}

#[test]
fn a_light1_table_without_row_stripes_still_rules_and_bolds_its_header() {
    // Excel scopes `showRowStripes` to the banding alone: the same table
    // exported with it off keeps all three rules and the bold header.
    let xml = module_inventory_table(
        r#"<tableStyleInfo name="TableStyleLight1" showRowStripes="0" showColumnStripes="0"/>"#,
    );

    let style = parse_table_part(&xml, &palette()).expect("the rules survive");

    assert_eq!(style.fill_at(1, 5), None, "no band without row stripes");
    assert!(style.border_at(1, 4).is_some());
    assert!(style.bolds_header_at(1, 4));
}

#[test]
fn a_headerless_light1_table_rules_only_its_outer_edges() {
    // With no header row there is nothing to close, so the middle rule has no
    // row to hang on and only the table's own top and bottom print.
    let xml = r#"<?xml version="1.0"?>
        <table xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main"
               ref="B2:D10" headerRowCount="0">
          <tableStyleInfo name="TableStyleLight1" showRowStripes="1"/>
        </table>"#;

    let style = parse_table_part(xml, &palette()).unwrap();

    let first = style.border_at(2, 2).expect("the table's top is ruled");
    assert!(first.top.is_some());
    assert!(first.bottom.is_none(), "row 2 closes no header");
    assert!(style.border_at(2, 3).is_none());
    assert!(!style.bolds_header_at(2, 2), "the table has no header row");
}

#[test]
fn a_table_without_a_header_row_stripes_from_its_first_row() {
    let xml = r#"<?xml version="1.0"?>
        <table xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main"
               ref="B2:D10" headerRowCount="0">
          <tableStyleInfo name="TableStyleMedium2" showRowStripes="1"/>
        </table>"#;

    let stripes = parse_table_part(xml, &palette()).unwrap();

    assert!(
        stripes.fill_at(2, 2).is_some(),
        "row 2 is the first body row"
    );
    assert!(stripes.fill_at(2, 3).is_none());
}
