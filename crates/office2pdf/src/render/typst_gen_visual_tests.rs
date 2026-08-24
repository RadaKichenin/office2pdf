use super::*;

// ── Floating image codegen tests ──

#[test]
fn test_floating_image_square_wrap_codegen() {
    let doc = Document {
        metadata: Metadata::default(),
        pages: vec![Page::Flow(FlowPage {
            first_header: None,
            first_footer: None,
            size: PageSize::default(),
            margins: Margins::default(),
            content: vec![Block::FloatingImage(FloatingImage {
                image: ImageData {
                    rotation_deg: None,
                    flip_h: false,
                    flip_v: false,
                    data: vec![0x89, 0x50, 0x4E, 0x47],
                    format: ImageFormat::Png,
                    width: Some(200.0),
                    height: Some(100.0),
                    crop: None,
                    stroke: None,
                    alignment: None,
                    clip_shape: None,
                    shadow: None,
                    paragraph_spacing: None,
                },
                wrap_mode: WrapMode::Square,
                offset_x: 72.0,
                offset_y: 36.0,
            })],
            header: None,
            footer: None,
            columns: None,
            line_grid_pitch: None,
            line_grid_snaps_lines: false,
            page_numbering: None,
        })],
        styles: StyleSheet::default(),
    };

    let output = generate_typst(&doc).unwrap();
    assert!(
        output.source.contains("#place("),
        "Expected #place() for floating image, got:\n{}",
        output.source
    );
    assert!(
        output.source.contains("float: true"),
        "Expected float: true for square wrap, got:\n{}",
        output.source
    );
    assert!(
        output.source.contains("dx: 72pt"),
        "Expected dx: 72pt, got:\n{}",
        output.source
    );
}

#[test]
fn test_floating_image_top_and_bottom_codegen() {
    let doc = Document {
        metadata: Metadata::default(),
        pages: vec![Page::Flow(FlowPage {
            first_header: None,
            first_footer: None,
            size: PageSize::default(),
            margins: Margins::default(),
            content: vec![Block::FloatingImage(FloatingImage {
                image: ImageData {
                    rotation_deg: None,
                    flip_h: false,
                    flip_v: false,
                    data: vec![0x89, 0x50, 0x4E, 0x47],
                    format: ImageFormat::Png,
                    width: Some(150.0),
                    height: Some(75.0),
                    crop: None,
                    stroke: None,
                    alignment: None,
                    clip_shape: None,
                    shadow: None,
                    paragraph_spacing: None,
                },
                wrap_mode: WrapMode::TopAndBottom,
                offset_x: 10.0,
                offset_y: 0.0,
            })],
            header: None,
            footer: None,
            columns: None,
            line_grid_pitch: None,
            line_grid_snaps_lines: false,
            page_numbering: None,
        })],
        styles: StyleSheet::default(),
    };

    let output = generate_typst(&doc).unwrap();
    assert!(
        output.source.contains("#block("),
        "Expected #block() for topAndBottom wrap, got:\n{}",
        output.source
    );
    assert!(
        output.source.contains("#v(75pt)"),
        "Expected vertical space for image height, got:\n{}",
        output.source
    );
}

#[test]
fn test_floating_image_behind_codegen() {
    let doc = Document {
        metadata: Metadata::default(),
        pages: vec![Page::Flow(FlowPage {
            first_header: None,
            first_footer: None,
            size: PageSize::default(),
            margins: Margins::default(),
            content: vec![Block::FloatingImage(FloatingImage {
                image: ImageData {
                    rotation_deg: None,
                    flip_h: false,
                    flip_v: false,
                    data: vec![0x89, 0x50, 0x4E, 0x47],
                    format: ImageFormat::Png,
                    width: Some(100.0),
                    height: Some(50.0),
                    crop: None,
                    stroke: None,
                    alignment: None,
                    clip_shape: None,
                    shadow: None,
                    paragraph_spacing: None,
                },
                wrap_mode: WrapMode::Behind,
                offset_x: 0.0,
                offset_y: 0.0,
            })],
            header: None,
            footer: None,
            columns: None,
            line_grid_pitch: None,
            line_grid_snaps_lines: false,
            page_numbering: None,
        })],
        styles: StyleSheet::default(),
    };

    let output = generate_typst(&doc).unwrap();
    assert!(
        output.source.contains("#place("),
        "Expected #place() for behind wrap, got:\n{}",
        output.source
    );
    assert!(
        !output.source.contains("float: true"),
        "Behind wrap should NOT use float, got:\n{}",
        output.source
    );
}

#[test]
fn test_floating_text_box_square_wrap_codegen() {
    let doc = make_doc(vec![make_flow_page(vec![Block::FloatingTextBox(
        FloatingTextBox {
            content: vec![make_paragraph("Anchored box")],
            wrap_mode: WrapMode::Square,
            width: 200.0,
            height: 100.0,
            padding: Insets::default(),
            vertical_align: TextBoxVerticalAlign::Top,
            offset_x: 72.0,
            offset_y: 36.0,
        },
    )])]);

    let output = generate_typst(&doc).unwrap();
    assert!(
        output.source.contains("#place("),
        "Expected #place() for floating text box, got:\n{}",
        output.source
    );
    assert!(
        output.source.contains("float: true"),
        "Expected float: true for square-wrapped text box, got:\n{}",
        output.source
    );
    assert!(
        output.source.contains("dx: 72pt"),
        "Expected dx: 72pt, got:\n{}",
        output.source
    );
    assert!(
        output.source.contains("width: 200pt"),
        "Expected width: 200pt, got:\n{}",
        output.source
    );
    assert!(
        output.source.contains("height: 100pt"),
        "Expected height: 100pt, got:\n{}",
        output.source
    );
    assert!(
        output.source.contains("Anchored box"),
        "Expected text box content, got:\n{}",
        output.source
    );
}

#[test]
fn test_floating_text_box_top_and_bottom_codegen() {
    let doc = make_doc(vec![make_flow_page(vec![Block::FloatingTextBox(
        FloatingTextBox {
            content: vec![make_paragraph("Top box")],
            wrap_mode: WrapMode::TopAndBottom,
            width: 150.0,
            height: 60.0,
            padding: Insets::default(),
            vertical_align: TextBoxVerticalAlign::Top,
            offset_x: 10.0,
            offset_y: 0.0,
        },
    )])]);

    let output = generate_typst(&doc).unwrap();
    assert!(
        output.source.contains("#block(width: 100%)"),
        "Expected block wrapper for top-and-bottom text box, got:\n{}",
        output.source
    );
    assert!(
        output.source.contains("#v(60pt)"),
        "Expected reserved vertical space for text box height, got:\n{}",
        output.source
    );
    assert!(
        output.source.contains("Top box"),
        "Expected text box content, got:\n{}",
        output.source
    );
}

// ── Math equation codegen tests ──

#[test]
fn test_floating_text_box_content_is_top_left_aligned_inside_bounds() {
    let doc = make_doc(vec![make_flow_page(vec![Block::FloatingTextBox(
        FloatingTextBox {
            content: vec![make_paragraph("Top aligned")],
            wrap_mode: WrapMode::None,
            width: 120.0,
            height: 40.0,
            padding: Insets::default(),
            vertical_align: TextBoxVerticalAlign::Top,
            offset_x: 10.0,
            offset_y: 5.0,
        },
    )])]);

    let output = generate_typst(&doc).unwrap();

    assert!(
        output
            .source
            .contains("#box(width: 120pt, height: 40pt, inset: 0pt)["),
        "Expected floating text box bounds to use a zero-inset box, got:\n{}",
        output.source
    );
    assert!(
        output
            .source
            .contains("#place(top + left, dy: -6pt)[\n#block(width: 120pt)["),
        "Expected floating text box content to be placed at the top-left of its bounds, got:\n{}",
        output.source
    );
}

#[test]
fn test_floating_text_box_applies_padding_and_center_alignment() {
    let doc = make_doc(vec![make_flow_page(vec![Block::FloatingTextBox(
        FloatingTextBox {
            content: vec![make_paragraph("Centered")],
            wrap_mode: WrapMode::None,
            width: 120.0,
            height: 60.0,
            padding: Insets {
                top: 3.0,
                right: 6.0,
                bottom: 3.0,
                left: 6.0,
            },
            vertical_align: TextBoxVerticalAlign::Center,
            offset_x: 10.0,
            offset_y: 5.0,
        },
    )])]);

    let output = generate_typst(&doc).unwrap();

    assert!(output.source.contains(
        "#box(width: 120pt, height: 60pt, inset: (top: 3pt, right: 6pt, bottom: 3pt, left: 6pt))["
    ));
    assert!(output.source.contains("block(width: 108pt)["));
    assert!(
        output
            .source
            .contains("calc.max(54pt - measure(floating_text_box_content_0).height, 0pt)")
    );
    assert!(output.source.contains("#v(floating_text_box_slack_0 / 2)"));
}

#[test]
fn test_consecutive_floating_shapes_share_one_anchor_line() {
    let shape = Shape {
        kind: ShapeKind::Rectangle,
        fill: Some(Color::new(114, 159, 207)),
        gradient_fill: None,
        pattern_fill: None,
        stroke: None,
        rotation_deg: None,
        opacity: None,
        shadow: None,
    };
    let doc = make_doc(vec![make_flow_page(vec![
        Block::FloatingShape(FloatingShape {
            shape: shape.clone(),
            width: 100.0,
            height: 40.0,
            offset_x: 20.0,
            offset_y: 10.0,
            wrap_mode: WrapMode::None,
        }),
        Block::FloatingShape(FloatingShape {
            shape,
            width: 100.0,
            height: 40.0,
            offset_x: 160.0,
            offset_y: 10.0,
            wrap_mode: WrapMode::None,
        }),
    ])]);

    let output = generate_typst(&doc).unwrap();
    let anchor_count = output
        .source
        .matches("#box(width: 0pt, height: 0pt)")
        .count();

    assert_eq!(
        anchor_count, 1,
        "Consecutive floating shapes from one DOCX paragraph should share one anchor line. Got:\n{}",
        output.source
    );
    assert!(
        output.source.contains("dx: 20pt, dy: 10pt")
            && output.source.contains("dx: 160pt, dy: 10pt"),
        "Expected both floating shapes to remain in the shared anchor group. Got:\n{}",
        output.source
    );
}

#[test]
fn test_codegen_display_math() {
    let doc = make_doc(vec![make_flow_page(vec![Block::MathEquation(
        MathEquation {
            content: "frac(a, b)".to_string(),
            display: true,
        },
    )])]);

    let output = generate_typst(&doc).unwrap();
    assert!(
        output.source.contains("$ frac(a, b) $"),
        "Expected display math '$ frac(a, b) $', got:\n{}",
        output.source
    );
}

#[test]
fn test_codegen_inline_math() {
    let doc = make_doc(vec![make_flow_page(vec![Block::MathEquation(
        MathEquation {
            content: "x^2".to_string(),
            display: false,
        },
    )])]);

    let output = generate_typst(&doc).unwrap();
    assert!(
        output.source.contains("$x^2$"),
        "Expected inline math '$x^2$', got:\n{}",
        output.source
    );
}

#[test]
fn test_codegen_complex_math() {
    let doc = make_doc(vec![make_flow_page(vec![Block::MathEquation(
        MathEquation {
            content: "sum_(i=1)^n i".to_string(),
            display: true,
        },
    )])]);

    let output = generate_typst(&doc).unwrap();
    assert!(
        output.source.contains("$ sum_(i=1)^n i $"),
        "Expected display math with sum, got:\n{}",
        output.source
    );
}

// ── Gradient codegen tests (US-050) ─────────────────────────────────

#[test]
fn test_gradient_background_codegen() {
    let page = Page::Fixed(FixedPage {
        size: PageSize {
            width: 720.0,
            height: 540.0,
        },
        elements: vec![],
        background_color: Some(Color::new(255, 0, 0)),
        background_gradient: Some(GradientFill {
            stops: vec![
                GradientStop {
                    position: 0.0,
                    color: Color::new(255, 0, 0),
                },
                GradientStop {
                    position: 1.0,
                    color: Color::new(0, 0, 255),
                },
            ],
            angle: 90.0,
        }),
    });
    let doc = make_doc(vec![page]);
    let output = generate_typst(&doc).unwrap();
    assert!(
        output.source.contains("gradient.linear("),
        "Should contain gradient.linear. Got: {}",
        output.source,
    );
    assert!(
        output.source.contains("(rgb(255, 0, 0), 0%)"),
        "Should contain first stop. Got: {}",
        output.source,
    );
    assert!(
        output.source.contains("(rgb(0, 0, 255), 100%)"),
        "Should contain second stop. Got: {}",
        output.source,
    );
    assert!(
        output.source.contains("angle: 90deg"),
        "Should contain angle. Got: {}",
        output.source,
    );
}

#[test]
fn test_gradient_background_no_angle_codegen() {
    let page = Page::Fixed(FixedPage {
        size: PageSize {
            width: 720.0,
            height: 540.0,
        },
        elements: vec![],
        background_color: None,
        background_gradient: Some(GradientFill {
            stops: vec![
                GradientStop {
                    position: 0.0,
                    color: Color::new(255, 255, 255),
                },
                GradientStop {
                    position: 1.0,
                    color: Color::new(0, 0, 0),
                },
            ],
            angle: 0.0,
        }),
    });
    let doc = make_doc(vec![page]);
    let output = generate_typst(&doc).unwrap();
    assert!(
        output.source.contains("gradient.linear("),
        "Should contain gradient.linear. Got: {}",
        output.source,
    );
    assert!(
        !output.source.contains("angle:"),
        "Should not contain angle for 0 degrees. Got: {}",
        output.source,
    );
}

#[test]
fn test_gradient_shape_fill_codegen() {
    let elem = FixedElement {
        x: 10.0,
        y: 20.0,
        width: 200.0,
        height: 150.0,
        kind: FixedElementKind::Shape(Shape {
            kind: ShapeKind::Rectangle,
            fill: Some(Color::new(255, 0, 0)),
            gradient_fill: Some(GradientFill {
                stops: vec![
                    GradientStop {
                        position: 0.0,
                        color: Color::new(0, 128, 0),
                    },
                    GradientStop {
                        position: 1.0,
                        color: Color::new(0, 0, 128),
                    },
                ],
                angle: 45.0,
            }),
            pattern_fill: None,
            stroke: None,
            rotation_deg: None,
            opacity: None,
            shadow: None,
        }),
    };
    let doc = make_doc(vec![make_fixed_page(720.0, 540.0, vec![elem])]);
    let output = generate_typst(&doc).unwrap();
    assert!(
        output.source.contains("gradient.linear("),
        "Should contain gradient.linear for shape. Got: {}",
        output.source,
    );
    assert!(
        output.source.contains("(rgb(0, 128, 0), 0%)"),
        "Should contain first stop. Got: {}",
        output.source,
    );
    assert!(
        !output.source.contains("fill: rgb(255, 0, 0)"),
        "Should not contain fallback solid fill. Got: {}",
        output.source,
    );
}

#[test]
fn test_light_upward_diagonal_pattern_fill_codegen() {
    let elem = FixedElement {
        x: 10.0,
        y: 20.0,
        width: 200.0,
        height: 150.0,
        kind: FixedElementKind::Shape(Shape {
            kind: ShapeKind::Rectangle,
            fill: None,
            gradient_fill: None,
            pattern_fill: Some(PatternFill {
                preset: PatternPreset::LightUpwardDiagonal,
                foreground: Color::new(0, 0, 255),
                background: Color::new(255, 255, 255),
            }),
            stroke: None,
            rotation_deg: None,
            opacity: None,
            shadow: None,
        }),
    };
    let doc = make_doc(vec![make_fixed_page(720.0, 540.0, vec![elem])]);
    let output = generate_typst(&doc).unwrap();

    assert!(
        output.source.contains("tiling(size: (2.72pt, 2.72pt))"),
        "Should emit a repeating DrawingML pattern. Got: {}",
        output.source,
    );
    assert!(
        output
            .source
            .contains("fill: rgb(255, 255, 255), stroke: none"),
        "Should paint the pattern background without an outline. Got: {}",
        output.source,
    );
    assert!(
        output.source.contains("stroke: 0.24pt + rgb(0, 0, 255)"),
        "Should paint the light upward hatch in the foreground color. Got: {}",
        output.source,
    );
}

#[test]
#[cfg(not(target_arch = "wasm32"))]
fn test_all_pattern_presets_compile_to_pdf() {
    let elements: Vec<FixedElement> = PatternPreset::ALL
        .iter()
        .copied()
        .enumerate()
        .map(|(index, preset)| FixedElement {
            x: (index % 9) as f64 * 48.0,
            y: (index / 9) as f64 * 48.0,
            width: 40.0,
            height: 40.0,
            kind: FixedElementKind::Shape(Shape {
                kind: ShapeKind::Rectangle,
                fill: None,
                gradient_fill: None,
                pattern_fill: Some(PatternFill {
                    preset,
                    foreground: Color::new(0, 0, 255),
                    background: Color::new(255, 255, 255),
                }),
                stroke: None,
                rotation_deg: None,
                opacity: None,
                shadow: None,
            }),
        })
        .collect();
    let doc = make_doc(vec![make_fixed_page(440.0, 300.0, elements)]);
    let output = generate_typst(&doc).unwrap();
    let pdf =
        crate::render::pdf::compile_to_pdf(&output.source, &output.images, None, &[], false, false)
            .expect("Every DrawingML preset pattern should compile as Typst");

    assert!(pdf.starts_with(b"%PDF"));
}

// ── Shadow codegen tests ──────────────────────────────────────────

#[test]
fn test_shape_shadow_codegen() {
    use crate::ir::Shadow;

    let elem = FixedElement {
        x: 10.0,
        y: 20.0,
        width: 200.0,
        height: 150.0,
        kind: FixedElementKind::Shape(Shape {
            kind: ShapeKind::Rectangle,
            fill: Some(Color::new(255, 0, 0)),
            gradient_fill: None,
            pattern_fill: None,
            stroke: None,
            rotation_deg: None,
            opacity: None,
            shadow: Some(Shadow {
                blur_radius: 4.0,
                distance: 3.0,
                direction: 45.0,
                color: Color::new(0, 0, 0),
                opacity: 0.5,
            }),
        }),
    };
    let doc = make_doc(vec![make_fixed_page(720.0, 540.0, vec![elem])]);
    let output = generate_typst(&doc).unwrap();
    // A blurred shadow stacks CDF-solved rings, each a black rect with its
    // own alpha. Matched on the shape rather than on a particular alpha,
    // which moves whenever the ramp is retuned (#662); what this test is
    // really about is that the stack precedes the shape it sits under.
    assert!(
        output.source.contains("rgb(0, 0, 0, "),
        "Shadow layers should use rgb with per-ring alpha. Got: {}",
        output.source,
    );
    let shadow_pos = output.source.find("rgb(0, 0, 0, ");
    let main_pos = output.source.find("rgb(255, 0, 0)");
    assert!(
        shadow_pos < main_pos,
        "Shadow should appear before main shape in output",
    );
}

#[test]
fn test_shape_no_shadow_no_extra_output() {
    let elem = FixedElement {
        x: 10.0,
        y: 20.0,
        width: 200.0,
        height: 150.0,
        kind: FixedElementKind::Shape(Shape {
            kind: ShapeKind::Rectangle,
            fill: Some(Color::new(255, 0, 0)),
            gradient_fill: None,
            pattern_fill: None,
            stroke: None,
            rotation_deg: None,
            opacity: None,
            shadow: None,
        }),
    };
    let doc = make_doc(vec![make_fixed_page(720.0, 540.0, vec![elem])]);
    let output = generate_typst(&doc).unwrap();
    assert!(
        !output.source.contains("rgb(0, 0, 0,"),
        "No shadow should produce no rgb shadow. Got: {}",
        output.source,
    );
}

#[test]
fn test_gradient_prefers_over_solid_fill() {
    let page = Page::Fixed(FixedPage {
        size: PageSize {
            width: 720.0,
            height: 540.0,
        },
        elements: vec![],
        background_color: Some(Color::new(128, 128, 128)),
        background_gradient: Some(GradientFill {
            stops: vec![
                GradientStop {
                    position: 0.0,
                    color: Color::new(255, 0, 0),
                },
                GradientStop {
                    position: 1.0,
                    color: Color::new(0, 0, 255),
                },
            ],
            angle: 180.0,
        }),
    });
    let doc = make_doc(vec![page]);
    let output = generate_typst(&doc).unwrap();
    assert!(
        output.source.contains("gradient.linear("),
        "Gradient should be preferred. Got: {}",
        output.source,
    );
    assert!(
        !output.source.contains("fill: rgb(128, 128, 128)"),
        "Solid fallback should not appear. Got: {}",
        output.source,
    );
}

#[test]
fn test_gradient_unsorted_stops_rendered_in_sorted_order() {
    let page = Page::Fixed(FixedPage {
        size: PageSize {
            width: 720.0,
            height: 540.0,
        },
        elements: vec![],
        background_color: None,
        background_gradient: Some(GradientFill {
            stops: vec![
                GradientStop {
                    position: 1.0,
                    color: Color::new(0, 0, 255),
                },
                GradientStop {
                    position: 0.5,
                    color: Color::new(0, 255, 0),
                },
                GradientStop {
                    position: 0.0,
                    color: Color::new(255, 0, 0),
                },
            ],
            angle: 90.0,
        }),
    });
    let doc = make_doc(vec![page]);
    let output = generate_typst(&doc).unwrap();
    let src = &output.source;
    let pos_red = src.find("(rgb(255, 0, 0), 0%)").expect("red stop missing");
    let pos_green = src
        .find("(rgb(0, 255, 0), 50%)")
        .expect("green stop missing");
    let pos_blue = src
        .find("(rgb(0, 0, 255), 100%)")
        .expect("blue stop missing");
    assert!(
        pos_red < pos_green && pos_green < pos_blue,
        "Stops should be in sorted order (0% < 50% < 100%). Got: {}",
        src,
    );
}

#[test]
fn test_shape_shadow_blur_renders_layered_rings() {
    // PowerPoint blurs outer shadows over `blurRad`; a single crisp offset
    // duplicate reads as a second shape. The approximation stacks
    // `SHADOW_RING_COUNT` concentric rings across the measured Gaussian
    // (sigma = blurRad/3, rings out to `SHADOW_RING_EXTENT_SIGMA` each
    // way) whose compounded alphas step down the CDF from the full opacity
    // inside to under 1% of it at the rim (issues #390, #662, #784).
    use crate::ir::Shadow;

    let elem = FixedElement {
        x: 10.0,
        y: 20.0,
        width: 200.0,
        height: 150.0,
        kind: FixedElementKind::Shape(Shape {
            kind: ShapeKind::Rectangle,
            fill: Some(Color::new(255, 0, 0)),
            gradient_fill: None,
            pattern_fill: None,
            stroke: None,
            rotation_deg: None,
            opacity: None,
            shadow: Some(Shadow {
                blur_radius: 8.0,
                distance: 3.0,
                direction: 45.0,
                color: Color::new(0, 0, 0),
                opacity: 0.5,
            }),
        }),
    };
    let doc = make_doc(vec![make_fixed_page(720.0, 540.0, vec![elem])]);
    let source = generate_typst(&doc).unwrap().source;

    // One translucent rect per ring. The count is what decides whether the
    // falloff reads as a ramp or as plateaus, so it is asserted rather than
    // the individual alphas — those move whenever the ramp is retuned, and
    // the Gaussian shape they encode is checked in
    // `test_blur_ring_coverage_follows_gaussian_cdf` (#662).
    assert_eq!(
        source.matches("rgb(0, 0, 0, ").count(),
        SHADOW_RING_COUNT,
        "expected one rect per ring in: {source}"
    );
    // sigma = 8pt / 3, and the rings reach the declared extent each
    // way, so the outermost outsets the 200x150 shape by that and the
    // innermost insets it by the same.
    let reach = SHADOW_RING_EXTENT_SIGMA * (8.0 / 3.0);
    let outermost = format!(
        "width: {}pt, height: {}pt",
        format_f64(200.0 + 2.0 * reach),
        format_f64(150.0 + 2.0 * reach)
    );
    assert!(
        source.contains(&outermost),
        "outermost ring must outset the shape by {reach}pt: {source}"
    );
    let innermost = format!(
        "width: {}pt, height: {}pt",
        format_f64(200.0 - 2.0 * reach),
        format_f64(150.0 - 2.0 * reach)
    );
    assert!(
        source.contains(&innermost),
        "innermost ring must inset the shape by {reach}pt: {source}"
    );
}

/// One concentric duplicate of the shadow's silhouette, in points.
struct ShadowRing {
    dx: f64,
    dy: f64,
    width: f64,
    height: f64,
    /// The corner arc's radius. A ring that writes no `radius:` turns a
    /// square corner, which is the same thing as an arc of radius zero.
    radius: f64,
    /// The ring's own fill alpha, 0-255. The stack compounds, so this is not
    /// the coverage an observer sees at the ring.
    alpha: u8,
}

impl ShadowRing {
    /// Whether the ring covers a point, given in points relative to the
    /// shape's own top-left corner.
    fn covers(&self, x: f64, y: f64) -> bool {
        let (left, top) = (self.dx, self.dy);
        let (right, bottom) = (self.dx + self.width, self.dy + self.height);
        if x < left || x > right || y < top || y > bottom {
            return false;
        }
        let radius: f64 = self.radius;
        // Outside a corner's own quadrant the box's edge is the boundary; the
        // arc only decides the four corner squares.
        let centre_x: f64 = if x < left + radius {
            left + radius
        } else if x > right - radius {
            right - radius
        } else {
            return true;
        };
        let centre_y: f64 = if y < top + radius {
            top + radius
        } else if y > bottom - radius {
            bottom - radius
        } else {
            return true;
        };
        (x - centre_x).powi(2) + (y - centre_y).powi(2) <= radius * radius
    }
}

/// The alpha the whole ring stack compounds to at a point, as a fraction of
/// paper: what a reader measures there, against the shadow's own opacity.
fn shadow_coverage_at(rings: &[ShadowRing], x: f64, y: f64) -> f64 {
    let mut transmitted: f64 = 1.0;
    for ring in rings.iter().filter(|ring| ring.covers(x, y)) {
        transmitted *= 1.0 - f64::from(ring.alpha) / 255.0;
    }
    1.0 - transmitted
}

/// The length `key` introduces in `fragment`, in points, or `None` when
/// `fragment` does not carry that key.
///
/// Read by key rather than by position: a direction that is a multiple of 90
/// degrees leaves a cosine residue of about 1e-17 in the offset, which no
/// exact literal can spell, and a ring writes a variable number of lengths.
fn optional_pt(fragment: &str, key: &str) -> Option<f64> {
    let start: usize = fragment.find(key)? + key.len();
    let rest: &str = &fragment[start..];
    let end: usize = rest.find("pt")?;
    rest[..end].trim().parse::<f64>().ok()
}

fn read_pt(fragment: &str, key: &str) -> f64 {
    optional_pt(fragment, key)
        .unwrap_or_else(|| panic!("no `{key}` length in shadow ring: {fragment}"))
}

/// The alpha byte a ring's black fill carries.
fn read_ring_alpha(fragment: &str) -> u8 {
    const KEY: &str = "rgb(0, 0, 0, ";
    let start: usize = fragment
        .find(KEY)
        .unwrap_or_else(|| panic!("no shadow fill in ring: {fragment}"))
        + KEY.len();
    let rest: &str = &fragment[start..];
    let end: usize = rest
        .find(')')
        .unwrap_or_else(|| panic!("unterminated shadow fill in ring: {fragment}"));
    rest[..end]
        .trim()
        .parse::<u8>()
        .unwrap_or_else(|error| panic!("bad shadow alpha in ring: {fragment} ({error})"))
}

/// Every shadow ring in `source`, in emission order — the stack is painted
/// from the innermost outwards, so the last ring is the widest.
///
/// Each ring is emitted on its own line and the shadow colour carries an
/// alpha, so `rgb(0, 0, 0, ` picks the rings out of a source whose shape fill
/// and outline are any other colour.
fn shadow_rings(source: &str) -> Vec<ShadowRing> {
    let rings: Vec<ShadowRing> = source
        .lines()
        .filter(|line| line.contains("rgb(0, 0, 0, "))
        .map(|line| ShadowRing {
            dx: read_pt(line, "dx: "),
            dy: read_pt(line, "dy: "),
            width: read_pt(line, "width: "),
            height: read_pt(line, "height: "),
            radius: optional_pt(line, "radius: ").unwrap_or(0.0),
            alpha: read_ring_alpha(line),
        })
        .collect();
    assert!(!rings.is_empty(), "no shadow ring in output: {source}");
    rings
}

/// The `(dx, dy, width, height)` of the first shadow ring in `source`, in
/// points.
fn first_shadow_layer_geometry(source: &str) -> (f64, f64, f64, f64) {
    let ring: &ShadowRing = &shadow_rings(source)[0];
    (ring.dx, ring.dy, ring.width, ring.height)
}

#[test]
fn test_shadow_silhouette_outsets_by_half_the_outline_width() {
    // PowerPoint casts `a:outerShdw` from the *stroked* silhouette — the fill
    // path grown by half the line width — and only then offsets it by `dist`.
    // Measured on `customGeo.pptx` page 46 against a native macOS PowerPoint
    // export: the banner's fill path is x [60, 672], y [36, 113.75] under a
    // 3pt outline, and the export's flattened shadow bitmap puts its
    // half-alpha silhouette at x [58.44, 673.56], y [36.00, 116.88] — the fill
    // path outset by 1.5pt and then moved 1.57pt down, agreeing on every edge
    // to within 0.09pt (issue #1057).
    use crate::ir::Shadow;

    let elem = FixedElement {
        x: 60.0,
        y: 36.0,
        width: 612.0,
        height: 77.75,
        kind: FixedElementKind::Shape(Shape {
            kind: ShapeKind::Rectangle,
            fill: Some(Color::new(79, 129, 189)),
            gradient_fill: None,
            pattern_fill: None,
            stroke: Some(BorderSide {
                width: 3.0,
                color: Color::new(255, 255, 255),
                style: BorderLineStyle::Solid,
                join: LineJoin::Round,
            }),
            rotation_deg: None,
            opacity: None,
            // Blur left off so the silhouette itself is what the geometry
            // below measures; the blur's own reach is covered by
            // `test_shape_shadow_blur_renders_layered_rings`.
            shadow: Some(Shadow {
                blur_radius: 0.0,
                distance: 1.57,
                direction: 90.0,
                color: Color::new(0, 0, 0),
                opacity: 0.38,
            }),
        }),
    };
    let doc = make_doc(vec![make_fixed_page(720.0, 540.0, vec![elem])]);
    let source = generate_typst(&doc).unwrap().source;

    let outset: f64 = 1.5;
    let (dx, dy, width, height) = first_shadow_layer_geometry(&source);
    assert!(
        (width - (612.0 + 2.0 * outset)).abs() < 0.01
            && (height - (77.75 + 2.0 * outset)).abs() < 0.01,
        "a 3pt outline must grow the shadow silhouette by {outset}pt a side, \
         got {width}x{height}pt: {source}"
    );
    // The silhouette grows about its own centre, so the outset shifts the
    // placement back by the same amount before `dist` moves it. Against the
    // shape's own x=60, y=36 that puts the silhouette at x [58.5, 673.5],
    // y [36.07, 116.82] — the export's, to within 0.09pt.
    assert!(
        (dx - -outset).abs() < 0.01 && (dy - (1.57 - outset)).abs() < 0.01,
        "the grown silhouette must stay centred on the shape, got dx {dx}pt \
         dy {dy}pt: {source}"
    );
}

#[test]
fn test_shadow_silhouette_ignores_outline_when_shape_has_none() {
    // The control for the outset above: a shape with no `a:ln` casts from its
    // fill path alone, so the crisp duplicate keeps the shape's own size and
    // sits exactly `dist` away (issue #1057).
    use crate::ir::Shadow;

    let elem = FixedElement {
        x: 60.0,
        y: 36.0,
        width: 612.0,
        height: 77.75,
        kind: FixedElementKind::Shape(Shape {
            kind: ShapeKind::Rectangle,
            fill: Some(Color::new(79, 129, 189)),
            gradient_fill: None,
            pattern_fill: None,
            stroke: None,
            rotation_deg: None,
            opacity: None,
            shadow: Some(Shadow {
                blur_radius: 0.0,
                distance: 1.57,
                direction: 90.0,
                color: Color::new(0, 0, 0),
                opacity: 0.38,
            }),
        }),
    };
    let doc = make_doc(vec![make_fixed_page(720.0, 540.0, vec![elem])]);
    let source = generate_typst(&doc).unwrap().source;

    let (dx, dy, width, height) = first_shadow_layer_geometry(&source);
    assert!(
        (width - 612.0).abs() < 0.01 && (height - 77.75).abs() < 0.01,
        "an outline-less shape casts its fill path unchanged, got \
         {width}x{height}pt: {source}"
    );
    assert!(
        dx.abs() < 0.01 && (dy - 1.57).abs() < 0.01,
        "an outline-less shadow sits exactly `dist` away, got dx {dx}pt \
         dy {dy}pt: {source}"
    );
}

/// `blurRad="40000"` EMU, the blur the banner's theme effect declares.
const BANNER_BLUR_RADIUS_PT: f64 = 40000.0 / 12700.0;
const BANNER_WIDTH_PT: f64 = 612.0;
const BANNER_HEIGHT_PT: f64 = 77.75;

/// How far the outermost ring reaches past the silhouette, in points: the
/// declared extent in sigma units, at the measured sigma of `blurRad / 3`.
fn banner_blur_reach() -> f64 {
    SHADOW_RING_EXTENT_SIGMA * BANNER_BLUR_RADIUS_PT / 3.0
}

/// The `customGeo.pptx` page 46 title banner under `stroke`, carrying the
/// theme's own `<a:outerShdw blurRad="40000" dist="20000" dir="5400000">` —
/// 3.15pt of blur, 1.57pt down.
fn banner_shadow_source(stroke: Option<BorderSide>) -> String {
    banner_shadow_source_with(stroke, BANNER_BLUR_RADIUS_PT, 0.38)
}

/// The same banner under an arbitrary blur and shadow opacity, so a rule the
/// blur's sigma scales can be triangulated instead of read off one deck.
fn banner_shadow_source_with(stroke: Option<BorderSide>, blur_radius: f64, opacity: f64) -> String {
    use crate::ir::Shadow;

    let elem = FixedElement {
        x: 60.0,
        y: 36.0,
        width: BANNER_WIDTH_PT,
        height: BANNER_HEIGHT_PT,
        kind: FixedElementKind::Shape(Shape {
            kind: ShapeKind::Rectangle,
            fill: Some(Color::new(79, 129, 189)),
            gradient_fill: None,
            pattern_fill: None,
            stroke,
            rotation_deg: None,
            opacity: None,
            shadow: Some(Shadow {
                blur_radius,
                distance: 1.57,
                direction: 90.0,
                color: Color::new(0, 0, 0),
                opacity,
            }),
        }),
    };
    let doc = make_doc(vec![make_fixed_page(720.0, 540.0, vec![elem])]);
    generate_typst(&doc).unwrap().source
}

fn banner_outline(join: LineJoin) -> Option<BorderSide> {
    Some(BorderSide {
        width: 3.0,
        color: Color::new(255, 255, 255),
        style: BorderLineStyle::Solid,
        join,
    })
}

/// The standard normal CDF at the sample points below, for reading the
/// blurred corner's falloff without borrowing the implementation's own
/// approximation. Indexed by distance in sigma: 0, 0.5, 1, 1.5, 2.
const NORMAL_TAIL_BY_SIGMA: [(f64, f64); 5] = [
    (0.0, 0.500_000),
    (0.5, 0.308_538),
    (1.0, 0.158_655),
    (1.5, 0.066_807),
    (2.0, 0.022_750),
];

#[test]
fn test_shadow_ring_corner_falls_like_the_product_of_two_edge_gaussians() {
    // A convex corner is where two blurred edges meet, and an isotropic
    // Gaussian's coverage there is the *product* of the two edges' own
    // tails — a quarter at the corner point itself, against the half a
    // single edge keeps. Dilating the silhouette by a disc instead
    // reproduces the edge's value at the same signed distance, which leaves
    // the corner twice as dense as PowerPoint draws it (issue #1204).
    //
    // Measured on `customGeo.pptx` page 46 against a native macOS
    // PowerPoint export at 1200 DPI, sampling grey outward along the
    // 45-degree diagonal from the silhouette's bottom-left corner: the
    // export reads 238.3 there against the disc stack's 229.1, while both
    // agree to a level along the bottom edge (206.7 against 203.0).
    //
    // Triangulated over three blur/opacity pairs so the rule has to come
    // from the Gaussian's own scale rather than one deck's numbers.
    //
    // The stack is 24 flat-alpha steps, so a sample between two ring
    // boundaries carries the band's value rather than the curve's: measured
    // across the offsets and blurs below that stair is worth up to 0.023 of
    // the shadow's opacity on the edge and 0.012 at the corner. Dilating by
    // a disc misses the corner by 0.24 of it, eight times the tolerance.
    const RAMP_QUANTISATION: f64 = 0.03;

    for (blur_radius, opacity) in [(BANNER_BLUR_RADIUS_PT, 0.38), (9.0, 0.40), (24.0, 0.60)] {
        // No outline, so the silhouette is the fill path's own square corner
        // and the truth is exactly the product of the two edge tails.
        let source: String = banner_shadow_source_with(None, blur_radius, opacity);
        let rings: Vec<ShadowRing> = shadow_rings(&source);
        let sigma: f64 = blur_radius / 3.0;
        // `dist` 1.57pt straight down puts the silhouette's bottom-left
        // corner here, in the shape's own coordinates.
        let (corner_x, corner_y) = (0.0, BANNER_HEIGHT_PT + 1.57);

        for (offset_sigma, tail) in NORMAL_TAIL_BY_SIGMA {
            let offset: f64 = offset_sigma * sigma;
            let corner: f64 = shadow_coverage_at(&rings, corner_x - offset, corner_y + offset);
            let expected_corner: f64 = opacity * tail * tail;
            assert!(
                (corner - expected_corner).abs() < RAMP_QUANTISATION * opacity,
                "at {offset_sigma} sigma along the diagonal the corner must \
                 compound to {expected_corner}, got {corner} (blur \
                 {blur_radius}pt, opacity {opacity}): {source}"
            );
            // The control the fix must not disturb: the same distance below
            // the bottom edge, far from either corner, still carries the
            // single edge's own tail.
            let edge: f64 = shadow_coverage_at(&rings, BANNER_WIDTH_PT / 2.0, corner_y + offset);
            assert!(
                (edge - opacity * tail).abs() < RAMP_QUANTISATION * opacity,
                "at {offset_sigma} sigma below the edge the stack must \
                 compound to {}, got {edge} (blur {blur_radius}pt, opacity \
                 {opacity}): {source}",
                opacity * tail
            );
        }
    }
}

#[test]
fn test_shadow_ring_corner_matches_the_native_export_under_a_round_join() {
    // The real-world pin for the rule above, on the silhouette PowerPoint
    // actually casts: `customGeo.pptx` page 46's banner carries a 3pt
    // outline, whose round join arcs the silhouette by 1.5pt before the blur
    // ever reaches it (#1057, #1090, #1138). Grey sampled outward along the
    // 45-degree diagonal from the silhouette's bottom-left corner
    // (58.5, 116.82) on a `pdftoppm -r 1200` render of a native macOS
    // PowerPoint export (issue #1204).
    const EXPORT_GREY_BY_DIAGONAL_PT: [(f64, f64); 3] = [(0.0, 238.3), (0.6, 247.1), (1.2, 253.0)];

    let source: String = banner_shadow_source(banner_outline(LineJoin::Round));
    let rings: Vec<ShadowRing> = shadow_rings(&source);
    // The silhouette's own bounding-box corner, in the shape's coordinates:
    // the fill path grown 1.5pt by the outline, then moved 1.57pt down.
    let (corner_x, corner_y) = (-1.5, BANNER_HEIGHT_PT + 1.57 + 1.5);

    for (diagonal, grey) in EXPORT_GREY_BY_DIAGONAL_PT {
        let step: f64 = diagonal / std::f64::consts::SQRT_2;
        let measured: f64 = shadow_coverage_at(&rings, corner_x - step, corner_y + step);
        let exported: f64 = (255.0 - grey) / 255.0;
        assert!(
            (measured - exported).abs() < 0.008,
            "{diagonal}pt out along the corner diagonal the export covers \
             {exported} of the paper, got {measured}: {source}"
        );
    }
}

#[test]
fn test_shadow_ring_corner_arcs_like_a_round_outline_join() {
    // PowerPoint casts the shadow from the *stroked* shape (#1057), so the
    // silhouette turns each corner the way the outline does — an arc of
    // radius half the line width under DrawingML's round default (#1090).
    // A plain `#rect` made each ring a mitre reaching `expansion * sqrt(2)`
    // instead (issue #1138), and dilating the arc by the ring's own offset
    // is the floor every ring's radius has to clear: the Gaussian rounds a
    // corner further still, never less (issue #1204).
    let rounded: Vec<ShadowRing> =
        shadow_rings(&banner_shadow_source(banner_outline(LineJoin::Round)));
    let mitred: Vec<ShadowRing> =
        shadow_rings(&banner_shadow_source(banner_outline(LineJoin::Miter)));
    assert_eq!(
        rounded.len(),
        mitred.len(),
        "the join must not change the ring count"
    );

    for (ring, mitred_ring) in rounded.iter().zip(&mitred) {
        // Each ring is the silhouette offset by its own expansion, which the
        // emitted box records as half its growth over the fill path.
        let expansion: f64 = (ring.width - BANNER_WIDTH_PT) / 2.0;
        assert!(
            ring.radius >= expansion.max(0.0) - 1e-9,
            "a ring expanded {expansion}pt must arc by at least that much, \
             got {}pt",
            ring.radius,
        );
        // The join is what separates the two: a round one adds its own arc
        // to the silhouette before the blur ever sees the corner.
        assert!(
            ring.radius > mitred_ring.radius + 1e-9,
            "a round join must arc a ring further than a mitre, got {}pt \
             against {}pt",
            ring.radius,
            mitred_ring.radius,
        );
        assert!(
            (ring.width - mitred_ring.width).abs() < 1e-9,
            "the join must not change how far the silhouette is outset"
        );
    }
}

#[test]
fn test_shadow_ring_corner_takes_no_arc_from_a_mitre_join() {
    // Triangulation on the join: `a:miter` runs the stroke out to a point
    // that already sits on the corner of the outset box, so it contributes
    // no arc of its own and the ring is left with the blur's — exactly the
    // arc an outline-less shape's ring carries (issues #1138, #1204).
    let mitred: Vec<ShadowRing> =
        shadow_rings(&banner_shadow_source(banner_outline(LineJoin::Miter)));
    let bare: Vec<ShadowRing> = shadow_rings(&banner_shadow_source(None));
    assert_eq!(mitred.len(), bare.len(), "the ring count must not change");

    for (ring, bare_ring) in mitred.iter().zip(&bare) {
        assert!(
            (ring.radius - bare_ring.radius).abs() < 1e-9,
            "a mitred outline must leave the same arc an outline-less shape \
             carries, got {}pt against {}pt",
            ring.radius,
            bare_ring.radius,
        );
    }
    // The silhouette itself still grows by half the line width — the join
    // decides the corner, not the outset (#1057).
    let outermost: &ShadowRing = mitred.last().expect("no shadow ring");
    let reach: f64 = banner_blur_reach();
    assert!(
        (outermost.width - (BANNER_WIDTH_PT + 2.0 * (1.5 + reach))).abs() < 0.01,
        "a mitred outline must still outset the silhouette by 1.5pt, got \
         {}pt wide",
        outermost.width,
    );
}

#[test]
fn test_shadow_ring_corner_stays_drawable_across_the_whole_stack() {
    // Control for the arcs above: Typst rejects a negative radius outright,
    // and an eroded ring — one inside the silhouette — still has a corner
    // the blur rounds, because the coverage a corner loses to the second
    // axis does not stop at the silhouette (issues #1138, #1204).
    let rings: Vec<ShadowRing> = shadow_rings(&banner_shadow_source(None));
    assert!(
        rings.iter().all(|ring| ring.radius >= 0.0),
        "no ring may carry a negative corner radius"
    );
    assert!(
        rings
            .iter()
            .all(|ring| ring.radius <= 0.5 * ring.width.min(ring.height) + 1e-9),
        "no ring may arc further than half its own box"
    );
    assert!(
        rings[0].radius > 0.0,
        "an eroded ring still turns a rounded corner, got {}pt",
        rings[0].radius,
    );
}

#[test]
fn test_shadow_ring_keeps_the_dilated_arc_where_the_silhouette_is_already_blunt() {
    // The limit the corner correction has to respect: once the silhouette's
    // own arc is many sigma across, the blur sees a locally straight
    // boundary there and the iso-coverage contour is the equidistant one
    // again. A rounded rectangle 50pt in the corner under a 3.15pt blur
    // (sigma 1.05) is 47 sigma across its arc, where the correction has
    // decayed to a twentieth of a point (issue #1204).
    use crate::ir::Shadow;

    let side: f64 = 200.0;
    let radius_fraction: f64 = 0.25;
    let elem = FixedElement {
        x: 60.0,
        y: 36.0,
        width: side,
        height: side,
        kind: FixedElementKind::Shape(Shape {
            kind: ShapeKind::RoundedRectangle { radius_fraction },
            fill: Some(Color::new(79, 129, 189)),
            gradient_fill: None,
            pattern_fill: None,
            stroke: None,
            rotation_deg: None,
            opacity: None,
            shadow: Some(Shadow {
                blur_radius: BANNER_BLUR_RADIUS_PT,
                distance: 1.57,
                direction: 90.0,
                color: Color::new(0, 0, 0),
                opacity: 0.38,
            }),
        }),
    };
    let doc = make_doc(vec![make_fixed_page(720.0, 540.0, vec![elem])]);
    let source = generate_typst(&doc).unwrap().source;

    let silhouette_radius: f64 = radius_fraction * side;
    for ring in shadow_rings(&source) {
        let expansion: f64 = (ring.width - side) / 2.0;
        let dilated: f64 = silhouette_radius + expansion;
        assert!(
            (ring.radius - dilated).abs() < 0.05,
            "a blunt corner's ring must keep its dilated {dilated}pt arc, \
             got {}pt: {source}",
            ring.radius,
        );
    }
}

#[test]
fn test_shape_shadow_without_blur_keeps_single_duplicate() {
    use crate::ir::Shadow;

    let elem = FixedElement {
        x: 10.0,
        y: 20.0,
        width: 200.0,
        height: 150.0,
        kind: FixedElementKind::Shape(Shape {
            kind: ShapeKind::Rectangle,
            fill: Some(Color::new(255, 0, 0)),
            gradient_fill: None,
            pattern_fill: None,
            stroke: None,
            rotation_deg: None,
            opacity: None,
            shadow: Some(Shadow {
                blur_radius: 0.0,
                distance: 3.0,
                direction: 45.0,
                color: Color::new(0, 0, 0),
                opacity: 0.5,
            }),
        }),
    };
    let doc = make_doc(vec![make_fixed_page(720.0, 540.0, vec![elem])]);
    let source = generate_typst(&doc).unwrap().source;

    assert_eq!(
        source.matches("rgb(0, 0, 0, 128)").count(),
        1,
        "zero blur keeps the single offset duplicate: {source}"
    );
}
