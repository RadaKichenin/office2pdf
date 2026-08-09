use super::*;

/// Drive the parser the way the slide parser does: positioned just after the
/// `<a:custGeom>` start tag.
fn parse(xml: &str) -> Option<Vec<(f64, f64)>> {
    let mut reader = Reader::from_str(xml);
    loop {
        match reader.read_event() {
            Ok(Event::Start(ref element)) if element.local_name().as_ref() == b"custGeom" => {
                return parse_custom_geometry(&mut reader);
            }
            Ok(Event::Eof) => panic!("custGeom not found"),
            Err(error) => panic!("{error}"),
            _ => {}
        }
    }
}

fn close_to(actual: (f64, f64), expected: (f64, f64)) -> bool {
    (actual.0 - expected.0).abs() < 1e-9 && (actual.1 - expected.1).abs() < 1e-9
}

#[test]
fn a_straight_sided_path_becomes_its_normalized_vertices() {
    let vertices = parse(
        r#"<a:custGeom><a:pathLst><a:path w="1000" h="500">
            <a:moveTo><a:pt x="0" y="0"/></a:moveTo>
            <a:lnTo><a:pt x="1000" y="0"/></a:lnTo>
            <a:lnTo><a:pt x="1000" y="500"/></a:lnTo>
            <a:close/>
        </a:path></a:pathLst></a:custGeom>"#,
    )
    .expect("a triangle is a usable polygon");

    assert_eq!(vertices.len(), 3);
    assert!(close_to(vertices[0], (0.0, 0.0)));
    assert!(close_to(vertices[1], (1.0, 0.0)));
    assert!(close_to(vertices[2], (1.0, 1.0)));
}

/// The coordinate space is the path's own `w`/`h`, not the shape's extent, so
/// a different declared space must move the vertices.
#[test]
fn vertices_normalize_against_the_paths_own_coordinate_space() {
    let vertices = parse(
        r#"<a:custGeom><a:pathLst><a:path w="200" h="200">
            <a:moveTo><a:pt x="0" y="0"/></a:moveTo>
            <a:lnTo><a:pt x="100" y="0"/></a:lnTo>
            <a:lnTo><a:pt x="100" y="50"/></a:lnTo>
            <a:close/>
        </a:path></a:pathLst></a:custGeom>"#,
    )
    .expect("usable polygon");

    assert!(close_to(vertices[1], (0.5, 0.0)), "got {:?}", vertices[1]);
    assert!(close_to(vertices[2], (0.5, 0.25)), "got {:?}", vertices[2]);
}

/// A cubic is sampled, so the result must follow the curve rather than cut the
/// chord. A quarter circle from (0,1) to (1,0) bulges out past the chord's
/// midpoint (0.5, 0.5).
#[test]
fn a_cubic_segment_is_sampled_along_the_curve() {
    let vertices = parse(
        r#"<a:custGeom><a:pathLst><a:path w="1000" h="1000">
            <a:moveTo><a:pt x="0" y="1000"/></a:moveTo>
            <a:cubicBezTo><a:pt x="0" y="448"/><a:pt x="448" y="0"/><a:pt x="1000" y="0"/></a:cubicBezTo>
            <a:lnTo><a:pt x="1000" y="1000"/></a:lnTo>
            <a:close/>
        </a:path></a:pathLst></a:custGeom>"#,
    )
    .expect("usable polygon");

    assert!(
        vertices.len() > 10,
        "the curve must contribute many vertices, got {}",
        vertices.len()
    );
    let midpoint = vertices[vertices.len() / 3];
    assert!(
        midpoint.0 + midpoint.1 < 1.0,
        "a sampled arc bows inside the chord, got {midpoint:?}"
    );
    assert!(
        midpoint.0 > 0.05 && midpoint.1 < 0.95,
        "the curve must leave its start, got {midpoint:?}"
    );
}

/// A four-cubic circle comes back as a closed ring whose points all sit one
/// radius from the centre — the avatar frames on the deck in issue #841 are
/// exactly this shape.
#[test]
fn a_four_segment_bezier_circle_stays_circular() {
    // 0.5523 is the standard circle-approximation constant, in a 1000 space.
    let vertices = parse(
        r#"<a:custGeom><a:pathLst><a:path w="1000" h="1000">
            <a:moveTo><a:pt x="500" y="0"/></a:moveTo>
            <a:cubicBezTo><a:pt x="776" y="0"/><a:pt x="1000" y="224"/><a:pt x="1000" y="500"/></a:cubicBezTo>
            <a:cubicBezTo><a:pt x="1000" y="776"/><a:pt x="776" y="1000"/><a:pt x="500" y="1000"/></a:cubicBezTo>
            <a:cubicBezTo><a:pt x="224" y="1000"/><a:pt x="0" y="776"/><a:pt x="0" y="500"/></a:cubicBezTo>
            <a:cubicBezTo><a:pt x="0" y="224"/><a:pt x="224" y="0"/><a:pt x="500" y="0"/></a:cubicBezTo>
            <a:close/>
        </a:path></a:pathLst></a:custGeom>"#,
    )
    .expect("usable polygon");

    for (x, y) in &vertices {
        let radius = ((x - 0.5).powi(2) + (y - 0.5).powi(2)).sqrt();
        assert!(
            (radius - 0.5).abs() < 0.01,
            "({x}, {y}) is {radius} from the centre, not 0.5"
        );
    }
}

/// A polygon cannot carve a hole, so the path that carries the silhouette —
/// the largest — is the one kept.
#[test]
fn the_largest_path_wins_when_a_geometry_declares_several() {
    let vertices = parse(
        r#"<a:custGeom><a:pathLst>
            <a:path w="1000" h="1000">
                <a:moveTo><a:pt x="400" y="400"/></a:moveTo>
                <a:lnTo><a:pt x="600" y="400"/></a:lnTo>
                <a:lnTo><a:pt x="600" y="600"/></a:lnTo>
                <a:close/>
            </a:path>
            <a:path w="1000" h="1000">
                <a:moveTo><a:pt x="0" y="0"/></a:moveTo>
                <a:lnTo><a:pt x="1000" y="0"/></a:lnTo>
                <a:lnTo><a:pt x="1000" y="1000"/></a:lnTo>
                <a:lnTo><a:pt x="0" y="1000"/></a:lnTo>
                <a:close/>
            </a:path>
        </a:pathLst></a:custGeom>"#,
    )
    .expect("usable polygon");

    assert_eq!(vertices.len(), 4, "the outer square is the silhouette");
    assert!(close_to(vertices[2], (1.0, 1.0)));
}

#[test]
fn a_geometry_with_no_usable_path_returns_none() {
    assert_eq!(parse(r#"<a:custGeom><a:pathLst/></a:custGeom>"#), None);
    // A self-closing `<a:custGeom/>` is not covered here: it arrives as an
    // empty element, which the caller handles without opening a subtree, so
    // this function never sees one.
    // Two points enclose no area.
    assert_eq!(
        parse(
            r#"<a:custGeom><a:pathLst><a:path w="100" h="100">
                <a:moveTo><a:pt x="0" y="0"/></a:moveTo>
                <a:lnTo><a:pt x="100" y="100"/></a:lnTo>
            </a:path></a:pathLst></a:custGeom>"#
        ),
        None
    );
    // A path that declares no coordinate space cannot be normalized.
    assert_eq!(
        parse(
            r#"<a:custGeom><a:pathLst><a:path>
                <a:moveTo><a:pt x="0" y="0"/></a:moveTo>
                <a:lnTo><a:pt x="100" y="0"/></a:lnTo>
                <a:lnTo><a:pt x="100" y="100"/></a:lnTo>
            </a:path></a:pathLst></a:custGeom>"#
        ),
        None
    );
}

/// The reader must be left on `</a:custGeom>` so the caller's parse stays in
/// step, including when the geometry is unusable.
#[test]
fn the_reader_stops_at_the_end_of_the_geometry() {
    let xml = r#"<a:spPr><a:custGeom><a:pathLst><a:path w="10" h="10">
            <a:moveTo><a:pt x="0" y="0"/></a:moveTo>
        </a:path></a:pathLst></a:custGeom><a:solidFill/></a:spPr>"#;
    let mut reader = Reader::from_str(xml);
    loop {
        match reader.read_event() {
            Ok(Event::Start(ref element)) if element.local_name().as_ref() == b"custGeom" => {
                let _ = parse_custom_geometry(&mut reader);
                break;
            }
            Ok(Event::Eof) => panic!("custGeom not found"),
            _ => {}
        }
    }
    // The very next element is the sibling that followed the geometry.
    match reader.read_event() {
        Ok(Event::Empty(ref element)) | Ok(Event::Start(ref element)) => {
            assert_eq!(element.local_name().as_ref(), b"solidFill");
        }
        other => panic!("expected solidFill, got {other:?}"),
    }
}
