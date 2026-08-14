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

/// The `(dx, dy, width, height)` of the first shadow ring in `source`, in
/// points.
///
/// Read numerically rather than matched as a string: a direction that is a
/// multiple of 90 degrees leaves a cosine residue of about 1e-17 in the
/// offset, which no exact literal can spell.
fn first_shadow_layer_geometry(source: &str) -> (f64, f64, f64, f64) {
    let shadow_start = source
        .find("rgb(0, 0, 0, ")
        .expect("no shadow ring in output");
    let placement = source[..shadow_start]
        .rfind("#place(top + left, dx: ")
        .expect("shadow ring is not placed");
    let numbers: Vec<f64> = source[placement..shadow_start]
        .split(|c: char| !(c.is_ascii_digit() || c == '.' || c == '-'))
        .filter(|token| !token.is_empty())
        .filter_map(|token| token.parse::<f64>().ok())
        .collect();
    assert_eq!(
        numbers.len(),
        4,
        "expected dx, dy, width and height in: {}",
        &source[placement..shadow_start],
    );
    (numbers[0], numbers[1], numbers[2], numbers[3])
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
