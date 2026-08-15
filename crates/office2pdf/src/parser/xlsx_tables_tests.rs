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

#[test]
fn a_medium2_table_stripes_in_accent1_at_a_twenty_percent_tint() {
    // Excel paints DCE6F2 for TableStyleMedium2 against a 4F81BD accent 1,
    // which is the accent moved 80% of the way to white.
    let xml = module_inventory_table(
        r#"<tableStyleInfo name="TableStyleMedium2" showRowStripes="1" showColumnStripes="0"/>"#,
    );

    let stripes = parse_table_part(&xml, &palette()).expect("stripes are declared");

    assert_eq!(stripes.fill_at(1, 5), Some(Color::new(0xdc, 0xe6, 0xf2)));
}

#[test]
fn stripes_start_at_the_first_body_row_and_alternate() {
    // The header occupies row 4, so row 5 is the first body row — and Excel
    // shades it, then every second row after it.
    let xml =
        module_inventory_table(r#"<tableStyleInfo name="TableStyleMedium2" showRowStripes="1"/>"#);

    let stripes = parse_table_part(&xml, &palette()).unwrap();

    assert!(
        stripes.fill_at(1, 4).is_none(),
        "the header row is not a stripe"
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
fn a_table_that_asks_for_no_row_stripes_gets_none() {
    let xml = module_inventory_table(
        r#"<tableStyleInfo name="TableStyleMedium2" showRowStripes="0" showColumnStripes="1"/>"#,
    );

    assert_eq!(parse_table_part(&xml, &palette()), None);
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

    for dark_style in [
        "TableStyleMedium1",
        "TableStyleMedium8",
        "TableStyleMedium15",
    ] {
        assert_eq!(
            built_in_table_style(dark_style, &palette()),
            None,
            "{dark_style} opens its band on the dark style, which carries no accent"
        );
    }
}

#[test]
fn the_medium_family_leaves_its_header_row_alone() {
    // The native export bolds and fills a Medium header too, but that whole
    // treatment is unimplemented (issue #1125) — resolving only the banding
    // must not half-paint it.
    assert_eq!(
        built_in_table_style("TableStyleMedium2", &palette())
            .expect("Medium2's banding is resolved")
            .rule,
        None
    );
}

#[test]
fn light_band_one_walks_the_accents_like_medium_does() {
    // Light1 opens the band on the neutral style: `lt1` shaded 15% over `dk1`
    // rules, which is the #D9D9D9 on black the native export prints.
    assert_eq!(
        built_in_table_style("TableStyleLight1", &palette()),
        Some(TableStylePaint {
            stripe: Some(Color::new(0xd9, 0xd9, 0xd9)),
            rule: Some(Color::black()),
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
                rule: Some(accent),
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
