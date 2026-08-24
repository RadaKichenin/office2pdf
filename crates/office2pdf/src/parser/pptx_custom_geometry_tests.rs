use super::*;

/// A square shape box, so a test that declares its own `<a:path w= h=>`
/// coordinate space is unaffected by the extent behind it.
const UNIT_EXTENT: ShapeExtent = ShapeExtent {
    width: 1000.0,
    height: 1000.0,
};

/// Drive the parser the way the slide parser does: positioned just after the
/// `<a:custGeom>` start tag.
fn parse_subpaths_in(xml: &str, extent: ShapeExtent) -> Vec<Subpath> {
    let mut reader = Reader::from_str(xml);
    loop {
        match reader.read_event() {
            Ok(Event::Start(ref element)) if element.local_name().as_ref() == b"custGeom" => {
                return parse_custom_geometry(&mut reader, extent);
            }
            Ok(Event::Eof) => panic!("custGeom not found"),
            Err(error) => panic!("{error}"),
            _ => {}
        }
    }
}

fn parse_subpaths(xml: &str) -> Vec<Subpath> {
    parse_subpaths_in(xml, UNIT_EXTENT)
}

/// The vertices alone, for the tests whose subject is the geometry rather
/// than whether each outline closes.
fn parse(xml: &str) -> Vec<Vec<(f64, f64)>> {
    parse_subpaths(xml)
        .into_iter()
        .map(|subpath| subpath.vertices)
        .collect()
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
    .pop()
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
    .pop()
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
    .pop().expect("usable polygon");

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
    .pop().expect("usable polygon");

    for (x, y) in &vertices {
        let radius = ((x - 0.5).powi(2) + (y - 0.5).powi(2)).sqrt();
        assert!(
            (radius - 0.5).abs() < 0.01,
            "({x}, {y}) is {radius} from the centre, not 0.5"
        );
    }
}

/// Every subpath is returned, in document order — a geometry's outline and
/// the shapes inside it are separate polygons, and dropping any of them threw
/// away the deck's wave line-art (issue #866).
#[test]
fn every_path_of_a_multi_path_geometry_is_returned() {
    let paths = parse(
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
    );

    assert_eq!(paths.len(), 2, "got {paths:?}");
    assert_eq!(paths[0].len(), 3, "the small triangle comes first");
    assert_eq!(paths[1].len(), 4, "the square follows it");
}

#[test]
fn a_geometry_with_no_usable_path_returns_nothing() {
    assert!(parse(r#"<a:custGeom><a:pathLst/></a:custGeom>"#).is_empty());
    // A self-closing `<a:custGeom/>` is not covered here: it arrives as an
    // empty element, which the caller handles without opening a subtree, so
    // this function never sees one.
    // A lone `moveTo` draws nothing: one point is neither an area nor a line.
    assert!(
        parse(
            r#"<a:custGeom><a:pathLst><a:path w="100" h="100">
                <a:moveTo><a:pt x="0" y="0"/></a:moveTo>
            </a:path></a:pathLst></a:custGeom>"#
        )
        .is_empty()
    );
    // Two points enclose no area, so a closed outline of two is dropped.
    assert!(
        parse(
            r#"<a:custGeom><a:pathLst><a:path w="100" h="100">
                <a:moveTo><a:pt x="0" y="0"/></a:moveTo>
                <a:lnTo><a:pt x="100" y="100"/></a:lnTo>
                <a:close/>
            </a:path></a:pathLst></a:custGeom>"#
        )
        .is_empty()
    );
    // A path that declares no coordinate space and sits in a shape with no
    // box either cannot be normalized against anything.
    assert!(
        parse_subpaths_in(
            r#"<a:custGeom><a:pathLst><a:path>
                <a:moveTo><a:pt x="0" y="0"/></a:moveTo>
                <a:lnTo><a:pt x="100" y="0"/></a:lnTo>
                <a:lnTo><a:pt x="100" y="100"/></a:lnTo>
            </a:path></a:pathLst></a:custGeom>"#,
            ShapeExtent::new(0.0, 0.0),
        )
        .is_empty()
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
                let _ = parse_custom_geometry(&mut reader, UNIT_EXTENT);
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

/// One `<a:path>` may hold several subpaths: a `moveTo` after a `close` starts
/// a new outline rather than continuing the last one. Concatenating them into
/// a single ring joined the end of one to the start of the next and painted
/// the wedge between — issue #866, on a layout shape whose path is
/// `moveTo lnTo lnTo close moveTo lnTo …`.
#[test]
fn subpaths_within_one_path_do_not_join() {
    let vertices = parse(
        r#"<a:custGeom><a:pathLst><a:path w="1000" h="1000">
            <a:moveTo><a:pt x="0" y="0"/></a:moveTo>
            <a:lnTo><a:pt x="100" y="0"/></a:lnTo>
            <a:lnTo><a:pt x="100" y="100"/></a:lnTo>
            <a:close/>
            <a:moveTo><a:pt x="0" y="1000"/></a:moveTo>
            <a:lnTo><a:pt x="1000" y="1000"/></a:lnTo>
            <a:lnTo><a:pt x="1000" y="0"/></a:lnTo>
            <a:lnTo><a:pt x="0" y="0"/></a:lnTo>
            <a:close/>
        </a:path></a:pathLst></a:custGeom>"#,
    )
    .pop()
    .expect("usable polygon");

    // The two subpaths stay separate rather than being welded into one ring.
    // This checks the second; the first is the small triangle.
    assert_eq!(vertices.len(), 4, "got {vertices:?}");
    assert!(close_to(vertices[0], (0.0, 1.0)));
    assert!(close_to(vertices[1], (1.0, 1.0)));
    assert!(close_to(vertices[2], (1.0, 0.0)));
    assert!(close_to(vertices[3], (0.0, 0.0)));
}

/// A `moveTo` that is not preceded by a `close` also starts a subpath — Office
/// writes both forms.
#[test]
fn a_move_to_without_a_close_also_starts_a_subpath() {
    let vertices = parse(
        r#"<a:custGeom><a:pathLst><a:path w="1000" h="1000">
            <a:moveTo><a:pt x="0" y="0"/></a:moveTo>
            <a:lnTo><a:pt x="100" y="0"/></a:lnTo>
            <a:lnTo><a:pt x="100" y="100"/></a:lnTo>
            <a:moveTo><a:pt x="0" y="1000"/></a:moveTo>
            <a:lnTo><a:pt x="1000" y="1000"/></a:lnTo>
            <a:lnTo><a:pt x="1000" y="0"/></a:lnTo>
            <a:lnTo><a:pt x="0" y="0"/></a:lnTo>
        </a:path></a:pathLst></a:custGeom>"#,
    )
    .pop()
    .expect("usable polygon");

    assert_eq!(vertices.len(), 4, "got {vertices:?}");
}

/// A single-subpath geometry is unaffected, so the split does not fragment the
/// shapes that already worked.
#[test]
fn a_single_subpath_is_unchanged_by_the_split() {
    let vertices = parse(
        r#"<a:custGeom><a:pathLst><a:path w="1000" h="1000">
            <a:moveTo><a:pt x="0" y="0"/></a:moveTo>
            <a:lnTo><a:pt x="1000" y="0"/></a:lnTo>
            <a:lnTo><a:pt x="1000" y="1000"/></a:lnTo>
            <a:lnTo><a:pt x="0" y="1000"/></a:lnTo>
            <a:close/>
        </a:path></a:pathLst></a:custGeom>"#,
    )
    .pop()
    .expect("usable polygon");

    assert_eq!(vertices.len(), 4);
}

/// `a:close` is what tells the outline to return to its start. A path that
/// states none is a polyline: the elbow connectors of the deck on issue #1205
/// end at their last point, and joining that back to the first draws a
/// diagonal across the slide.
#[test]
fn an_unclosed_path_stays_open() {
    let subpaths = parse_subpaths(
        r#"<a:custGeom><a:pathLst><a:path w="1000" h="1000">
            <a:moveTo><a:pt x="0" y="0"/></a:moveTo>
            <a:lnTo><a:pt x="0" y="1000"/></a:lnTo>
            <a:lnTo><a:pt x="1000" y="1000"/></a:lnTo>
            <a:lnTo><a:pt x="1000" y="0"/></a:lnTo>
        </a:path></a:pathLst></a:custGeom>"#,
    );

    assert_eq!(subpaths.len(), 1);
    assert!(!subpaths[0].closed, "no a:close was stated");
    assert_eq!(subpaths[0].vertices.len(), 4);
}

/// The same path with `a:close` is a ring.
#[test]
fn a_closed_path_is_marked_closed() {
    let subpaths = parse_subpaths(
        r#"<a:custGeom><a:pathLst><a:path w="1000" h="1000">
            <a:moveTo><a:pt x="0" y="0"/></a:moveTo>
            <a:lnTo><a:pt x="0" y="1000"/></a:lnTo>
            <a:lnTo><a:pt x="1000" y="1000"/></a:lnTo>
            <a:close/>
        </a:path></a:pathLst></a:custGeom>"#,
    );

    assert_eq!(subpaths.len(), 1);
    assert!(subpaths[0].closed);
}

/// One `<a:path>` can hold a closed outline and an open one, and each keeps
/// its own answer rather than the last one seen.
#[test]
fn openness_is_tracked_per_subpath() {
    let subpaths = parse_subpaths(
        r#"<a:custGeom><a:pathLst><a:path w="1000" h="1000">
            <a:moveTo><a:pt x="0" y="0"/></a:moveTo>
            <a:lnTo><a:pt x="200" y="0"/></a:lnTo>
            <a:lnTo><a:pt x="200" y="200"/></a:lnTo>
            <a:close/>
            <a:moveTo><a:pt x="400" y="400"/></a:moveTo>
            <a:lnTo><a:pt x="900" y="400"/></a:lnTo>
            <a:lnTo><a:pt x="900" y="900"/></a:lnTo>
        </a:path></a:pathLst></a:custGeom>"#,
    );

    assert_eq!(subpaths.len(), 2, "got {subpaths:?}");
    assert!(subpaths[0].closed, "the first outline stated a:close");
    assert!(!subpaths[1].closed, "the second did not");
}

/// Two points enclose nothing but still draw a line, so an open two-point
/// subpath survives. The deck on issue #1205 draws each connector's vertical
/// leg exactly that way — `moveTo` then one `lnTo`, with `<a:noFill/>`.
#[test]
fn an_open_two_point_subpath_is_kept() {
    let subpaths = parse_subpaths(
        r#"<a:custGeom><a:pathLst><a:path w="1000" h="1000">
            <a:moveTo><a:pt x="500" y="0"/></a:moveTo>
            <a:lnTo><a:pt x="500" y="1000"/></a:lnTo>
        </a:path></a:pathLst></a:custGeom>"#,
    );

    assert_eq!(subpaths.len(), 1, "the line is not dropped: {subpaths:?}");
    assert!(!subpaths[0].closed);
    assert!(close_to(subpaths[0].vertices[0], (0.5, 0.0)));
    assert!(close_to(subpaths[0].vertices[1], (0.5, 1.0)));
}

/// A path that declares no `w`/`h` is in the shape's own coordinate space, so
/// it normalizes against the extent. Every guide-driven geometry takes this
/// form, since its guides already compute EMU in the shape's units
/// (issue #1205).
#[test]
fn a_path_without_a_coordinate_space_normalizes_against_the_extent() {
    let subpaths = parse_subpaths_in(
        r#"<a:custGeom><a:pathLst><a:path>
            <a:moveTo><a:pt x="0" y="0"/></a:moveTo>
            <a:lnTo><a:pt x="200" y="0"/></a:lnTo>
            <a:lnTo><a:pt x="200" y="50"/></a:lnTo>
            <a:close/>
        </a:path></a:pathLst></a:custGeom>"#,
        ShapeExtent::new(400.0, 200.0),
    );

    let vertices: &Vec<(f64, f64)> = &subpaths[0].vertices;
    assert!(close_to(vertices[1], (0.5, 0.0)), "got {:?}", vertices[1]);
    assert!(close_to(vertices[2], (0.5, 0.25)), "got {:?}", vertices[2]);
}

/// A declared `<a:path w= h=>` still wins over the extent, so the geometries
/// that already worked are untouched.
#[test]
fn a_declared_coordinate_space_still_wins_over_the_extent() {
    let subpaths = parse_subpaths_in(
        r#"<a:custGeom><a:pathLst><a:path w="200" h="200">
            <a:moveTo><a:pt x="0" y="0"/></a:moveTo>
            <a:lnTo><a:pt x="100" y="0"/></a:lnTo>
            <a:lnTo><a:pt x="100" y="50"/></a:lnTo>
            <a:close/>
        </a:path></a:pathLst></a:custGeom>"#,
        ShapeExtent::new(4000.0, 8000.0),
    );

    let vertices: &Vec<(f64, f64)> = &subpaths[0].vertices;
    assert!(close_to(vertices[1], (0.5, 0.0)), "got {:?}", vertices[1]);
    assert!(close_to(vertices[2], (0.5, 0.25)), "got {:?}", vertices[2]);
}

/// A coordinate may be a guide name rather than a number. Reading only
/// numbers left the geometry empty and the caller's rectangle fallback
/// standing in for it (issue #1205).
#[test]
fn a_vertex_may_name_a_guide() {
    let subpaths = parse_subpaths_in(
        r#"<a:custGeom>
            <a:avLst><a:gd name="adj" fmla="val 5400"/></a:avLst>
            <a:gdLst>
                <a:gd name="left" fmla="val 0"/>
                <a:gd name="inset" fmla="*/ adj w 21600"/>
                <a:gd name="bottom" fmla="val h"/>
            </a:gdLst>
            <a:pathLst><a:path>
                <a:moveTo><a:pt x="left" y="left"/></a:moveTo>
                <a:lnTo><a:pt x="inset" y="left"/></a:lnTo>
                <a:lnTo><a:pt x="inset" y="bottom"/></a:lnTo>
                <a:close/>
            </a:path></a:pathLst>
        </a:custGeom>"#,
        ShapeExtent::new(400.0, 200.0),
    );

    assert_eq!(subpaths.len(), 1, "got {subpaths:?}");
    let vertices: &Vec<(f64, f64)> = &subpaths[0].vertices;
    // 5400/21600 of the width is a quarter of it.
    assert!(close_to(vertices[0], (0.0, 0.0)), "got {:?}", vertices[0]);
    assert!(close_to(vertices[1], (0.25, 0.0)), "got {:?}", vertices[1]);
    assert!(close_to(vertices[2], (0.25, 1.0)), "got {:?}", vertices[2]);
}

/// The built-in variables work without an `<a:gdLst>` at all — `<a:pt x="r"
/// y="b"/>` is the shape's bottom-right corner.
#[test]
fn a_vertex_may_name_a_built_in_variable() {
    let subpaths = parse_subpaths_in(
        r#"<a:custGeom><a:pathLst><a:path>
            <a:moveTo><a:pt x="l" y="t"/></a:moveTo>
            <a:lnTo><a:pt x="r" y="t"/></a:lnTo>
            <a:lnTo><a:pt x="hc" y="b"/></a:lnTo>
            <a:close/>
        </a:path></a:pathLst></a:custGeom>"#,
        ShapeExtent::new(400.0, 200.0),
    );

    let vertices: &Vec<(f64, f64)> = &subpaths[0].vertices;
    assert!(close_to(vertices[0], (0.0, 0.0)), "got {:?}", vertices[0]);
    assert!(close_to(vertices[1], (1.0, 0.0)), "got {:?}", vertices[1]);
    assert!(close_to(vertices[2], (0.5, 1.0)), "got {:?}", vertices[2]);
}

/// A guide list is read in document order, so a `<a:pt>` may depend on a
/// chain of them — which is how a hundred-entry list turns one adjust value
/// into a corner inset.
#[test]
fn a_chain_of_guides_resolves_before_the_path_reads_it() {
    let subpaths = parse_subpaths_in(
        r#"<a:custGeom>
            <a:avLst><a:gd name="f0" fmla="val 2160"/></a:avLst>
            <a:gdLst>
                <a:gd name="f1" fmla="pin 0 f0 10800"/>
                <a:gd name="f2" fmla="val ss"/>
                <a:gd name="f3" fmla="*/ f2 1 21600"/>
                <a:gd name="f4" fmla="*/ f1 f3 1"/>
                <a:gd name="f5" fmla="val 0"/>
            </a:gdLst>
            <a:pathLst><a:path>
                <a:moveTo><a:pt x="f5" y="f5"/></a:moveTo>
                <a:lnTo><a:pt x="f4" y="f5"/></a:lnTo>
                <a:lnTo><a:pt x="f4" y="f4"/></a:lnTo>
                <a:close/>
            </a:path></a:pathLst>
        </a:custGeom>"#,
        ShapeExtent::new(1000.0, 500.0),
    );

    // 2160 * min(1000, 500) / 21600 is 50.
    let vertices: &Vec<(f64, f64)> = &subpaths[0].vertices;
    assert!(close_to(vertices[1], (0.05, 0.0)), "got {:?}", vertices[1]);
    assert!(close_to(vertices[2], (0.05, 0.1)), "got {:?}", vertices[2]);
}

/// A vertex naming something nothing defines is dropped rather than defaulted
/// to the shape's corner, which would drag the outline through it.
#[test]
fn a_vertex_naming_an_undefined_guide_is_dropped() {
    let subpaths = parse_subpaths_in(
        r#"<a:custGeom><a:pathLst><a:path>
            <a:moveTo><a:pt x="l" y="t"/></a:moveTo>
            <a:lnTo><a:pt x="r" y="t"/></a:lnTo>
            <a:lnTo><a:pt x="nowhere" y="t"/></a:lnTo>
            <a:lnTo><a:pt x="r" y="b"/></a:lnTo>
            <a:close/>
        </a:path></a:pathLst></a:custGeom>"#,
        ShapeExtent::new(400.0, 200.0),
    );

    let vertices: &Vec<(f64, f64)> = &subpaths[0].vertices;
    assert_eq!(vertices.len(), 3, "the unresolved vertex is skipped");
    assert!(close_to(vertices[2], (1.0, 1.0)), "got {:?}", vertices[2]);
}
