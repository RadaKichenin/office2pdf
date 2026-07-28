use super::*;

/// Office's default accent palette, as the audited workbook's theme declares it.
fn accents() -> Vec<Color> {
    vec![
        Color::new(0x4f, 0x81, 0xbd), // accent1
        Color::new(0xc0, 0x50, 0x4d), // accent2
        Color::new(0x9b, 0xbb, 0x59), // accent3
        Color::new(0x80, 0x64, 0xa2), // accent4
        Color::new(0x4b, 0xac, 0xc6), // accent5
        Color::new(0xf7, 0x96, 0x46), // accent6
    ]
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

    let stripes = parse_table_part(&xml, &accents()).expect("stripes are declared");

    assert_eq!(stripes.fill_at(1, 5), Some(Color::new(0xdc, 0xe6, 0xf2)));
}

#[test]
fn stripes_start_at_the_first_body_row_and_alternate() {
    // The header occupies row 4, so row 5 is the first body row — and Excel
    // shades it, then every second row after it.
    let xml =
        module_inventory_table(r#"<tableStyleInfo name="TableStyleMedium2" showRowStripes="1"/>"#);

    let stripes = parse_table_part(&xml, &accents()).unwrap();

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

    let stripes = parse_table_part(&xml, &accents()).unwrap();

    assert!(stripes.fill_at(8, 5).is_some(), "column H is the last one");
    assert!(stripes.fill_at(9, 5).is_none(), "column I is outside");
}

#[test]
fn a_table_that_asks_for_no_row_stripes_gets_none() {
    let xml = module_inventory_table(
        r#"<tableStyleInfo name="TableStyleMedium2" showRowStripes="0" showColumnStripes="1"/>"#,
    );

    assert_eq!(parse_table_part(&xml, &accents()), None);
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
        let expected = tint(accents()[accent_index], 0.8);

        assert_eq!(
            stripe_fill_for_style(style, &accents()),
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
            stripe_fill_for_style(dark_style, &accents()),
            None,
            "{dark_style} opens its band on the dark style, which carries no accent"
        );
    }
}

#[test]
fn an_unresolved_style_family_leaves_the_table_unstriped() {
    // Light and Dark tints are not pinned down by any fixture here, so they
    // must fall back to today's behaviour rather than to a guess.
    for style in ["TableStyleLight2", "TableStyleDark3", "MyCustomStyle"] {
        assert_eq!(stripe_fill_for_style(style, &accents()), None, "{style}");
    }
}

#[test]
fn a_table_without_a_header_row_stripes_from_its_first_row() {
    let xml = r#"<?xml version="1.0"?>
        <table xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main"
               ref="B2:D10" headerRowCount="0">
          <tableStyleInfo name="TableStyleMedium2" showRowStripes="1"/>
        </table>"#;

    let stripes = parse_table_part(xml, &accents()).unwrap();

    assert!(
        stripes.fill_at(2, 2).is_some(),
        "row 2 is the first body row"
    );
    assert!(stripes.fill_at(2, 3).is_none());
}
