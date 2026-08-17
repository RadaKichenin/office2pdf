#![cfg(not(target_arch = "wasm32"))] // native-only unit tests (filesystem, system fonts)
use super::*;

#[test]
fn typst_cache_state_evicts_after_the_bounded_document_interval() {
    let mut state = TypstCacheState {
        active_compilations: 0,
        completed_since_eviction: 0,
    };

    for _ in 0..TYPST_CACHE_EVICTION_INTERVAL {
        assert!(!state.begin_compilation());
        state.finish_compilation();
    }

    assert!(state.begin_compilation());
    assert_eq!(state.completed_since_eviction, 0);
    state.finish_compilation();
}

#[test]
fn typst_cache_state_defers_eviction_while_compilations_overlap() {
    let mut state = TypstCacheState {
        active_compilations: 1,
        completed_since_eviction: TYPST_CACHE_EVICTION_INTERVAL,
    };

    assert!(!state.begin_compilation());
    state.finish_compilation();
    state.finish_compilation();
    assert!(state.begin_compilation());
    state.finish_compilation();
}
use crate::test_support::make_test_svg;

#[test]
fn test_compile_simple_text() {
    let result = compile_to_pdf("Hello, World!", &[], None, &[], false, false).unwrap();
    assert!(!result.is_empty(), "PDF bytes should not be empty");
    assert!(
        result.starts_with(b"%PDF"),
        "PDF should start with %PDF magic bytes"
    );
}

#[test]
fn test_compile_with_page_setup() {
    let source = r#"#set page(width: 612pt, height: 792pt)
Hello from a US Letter page."#;
    let result = compile_to_pdf(source, &[], None, &[], false, false).unwrap();
    assert!(!result.is_empty());
    assert!(result.starts_with(b"%PDF"));
}

#[test]
fn test_compile_styled_text() {
    let source = r#"#text(weight: "bold", size: 16pt)[Bold Title]

#text(style: "italic")[Italic body text]

#underline[Underlined text]"#;
    let result = compile_to_pdf(source, &[], None, &[], false, false).unwrap();
    assert!(!result.is_empty());
    assert!(result.starts_with(b"%PDF"));
}

#[test]
fn test_compile_colored_text() {
    let source = r#"#text(fill: rgb(255, 0, 0))[Red text]
#text(fill: rgb(0, 128, 255))[Blue text]"#;
    let result = compile_to_pdf(source, &[], None, &[], false, false).unwrap();
    assert!(!result.is_empty());
    assert!(result.starts_with(b"%PDF"));
}

#[test]
fn test_compile_alignment() {
    let source = r#"#align(center)[Centered text]

#align(right)[Right-aligned text]"#;
    let result = compile_to_pdf(source, &[], None, &[], false, false).unwrap();
    assert!(!result.is_empty());
    assert!(result.starts_with(b"%PDF"));
}

#[test]
fn test_compile_invalid_source_returns_error() {
    // Invalid Typst source should produce a compilation error
    let result = compile_to_pdf(
        "#invalid-func-that-does-not-exist()",
        &[],
        None,
        &[],
        false,
        false,
    );
    assert!(result.is_err(), "Invalid source should produce an error");
}

#[test]
fn test_compile_empty_source() {
    // Empty source should still produce valid PDF (empty page)
    let result = compile_to_pdf("", &[], None, &[], false, false).unwrap();
    assert!(!result.is_empty());
    assert!(result.starts_with(b"%PDF"));
}

#[test]
fn test_compile_multiple_paragraphs() {
    let source = "First paragraph.\n\nSecond paragraph.\n\nThird paragraph.";
    let result = compile_to_pdf(source, &[], None, &[], false, false).unwrap();
    assert!(!result.is_empty());
    assert!(result.starts_with(b"%PDF"));
}

/// Compute CRC32 over PNG chunk type + data.
fn png_crc32(chunk_type: &[u8], data: &[u8]) -> u32 {
    let mut crc: u32 = 0xFFFF_FFFF;
    for &byte in chunk_type.iter().chain(data.iter()) {
        crc ^= byte as u32;
        for _ in 0..8 {
            if crc & 1 != 0 {
                crc = (crc >> 1) ^ 0xEDB8_8320;
            } else {
                crc >>= 1;
            }
        }
    }
    crc ^ 0xFFFF_FFFF
}

/// Build a minimal valid 1x1 red PNG with correct CRC checksums.
fn make_test_png() -> Vec<u8> {
    let mut png = Vec::new();
    // PNG signature
    png.extend_from_slice(&[0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A]);

    // IHDR: 1x1, 8-bit RGB
    let ihdr_data: [u8; 13] = [
        0x00, 0x00, 0x00, 0x01, // width=1
        0x00, 0x00, 0x00, 0x01, // height=1
        0x08, // bit depth=8
        0x02, // color type=RGB
        0x00, 0x00, 0x00, // compression, filter, interlace
    ];
    let ihdr_type = b"IHDR";
    png.extend_from_slice(&(ihdr_data.len() as u32).to_be_bytes());
    png.extend_from_slice(ihdr_type);
    png.extend_from_slice(&ihdr_data);
    png.extend_from_slice(&png_crc32(ihdr_type, &ihdr_data).to_be_bytes());

    // IDAT: zlib-compressed row [filter=0, R=255, G=0, B=0]
    let idat_data: [u8; 15] = [
        0x78, 0x01, // zlib header
        0x01, // BFINAL=1, BTYPE=00 (stored)
        0x04, 0x00, 0xFB, 0xFF, // LEN=4, NLEN
        0x00, 0xFF, 0x00, 0x00, // filter + RGB
        0x03, 0x01, 0x01, 0x00, // adler32
    ];
    let idat_type = b"IDAT";
    png.extend_from_slice(&(idat_data.len() as u32).to_be_bytes());
    png.extend_from_slice(idat_type);
    png.extend_from_slice(&idat_data);
    png.extend_from_slice(&png_crc32(idat_type, &idat_data).to_be_bytes());

    // IEND
    let iend_type = b"IEND";
    png.extend_from_slice(&0u32.to_be_bytes());
    png.extend_from_slice(iend_type);
    png.extend_from_slice(&png_crc32(iend_type, &[]).to_be_bytes());

    png
}

#[test]
fn test_embedded_fonts_are_available() {
    // MinimalWorld should always have embedded fallback fonts available
    // (Libertinus Serif, New Computer Modern, DejaVu Sans Mono)
    let world = MinimalWorld::new("", &[], &[]);
    assert!(
        world.font_source.len() > 0,
        "MinimalWorld should have at least the embedded fallback fonts"
    );
}

#[test]
fn test_system_fonts_enabled() {
    // With system font discovery enabled, on typical systems we should have
    // more fonts than just the embedded set. On minimal systems, we at least
    // have the embedded fonts.
    let world = MinimalWorld::new("", &[], &[]);
    let embedded_only_count = {
        let mut s = FontSearcher::new();
        s.include_system_fonts(false);
        s.search().fonts.len()
    };
    // At minimum, we should have the embedded fonts
    assert!(
        world.font_source.len() >= embedded_only_count,
        "System font discovery should not reduce available fonts: total {} vs embedded-only {}",
        world.font_source.len(),
        embedded_only_count
    );
}

#[test]
fn test_compile_with_system_font_name() {
    // A document specifying a common system font should compile successfully.
    // Typst falls back to embedded fonts if the named font isn't available,
    // so this test always succeeds — but with system fonts enabled, the
    // named font will be used if present on the system.
    let source = r#"#set text(font: "Arial")
Hello with a system font."#;
    let result = compile_to_pdf(source, &[], None, &[], false, false).unwrap();
    assert!(!result.is_empty());
    assert!(result.starts_with(b"%PDF"));
}

#[test]
fn test_embedded_fonts_still_available_as_fallback() {
    // Embedded fonts (Libertinus Serif) must still be available even with
    // system font discovery enabled.
    let source = r#"#set text(font: "Libertinus Serif")
Text in Libertinus Serif."#;
    let result = compile_to_pdf(source, &[], None, &[], false, false).unwrap();
    assert!(!result.is_empty());
    assert!(result.starts_with(b"%PDF"));
}

#[test]
fn test_compile_pdfa2b_produces_valid_pdf() {
    let result = compile_to_pdf(
        "Hello PDF/A!",
        &[],
        Some(crate::config::PdfStandard::PdfA2b),
        &[],
        false,
        false,
    )
    .unwrap();
    assert!(!result.is_empty());
    assert!(result.starts_with(b"%PDF"));
}

#[test]
fn test_compile_pdfa2b_contains_xmp_metadata() {
    let result = compile_to_pdf(
        "PDF/A metadata test",
        &[],
        Some(crate::config::PdfStandard::PdfA2b),
        &[],
        false,
        false,
    )
    .unwrap();
    // PDF/A-2b requires XMP metadata with pdfaid namespace
    let pdf_str = String::from_utf8_lossy(&result);
    assert!(
        pdf_str.contains("pdfaid") || pdf_str.contains("PDF/A"),
        "PDF/A output should contain PDF/A identification metadata"
    );
}

#[test]
fn test_compile_default_no_pdfa_metadata() {
    let result = compile_to_pdf("Regular PDF", &[], None, &[], false, false).unwrap();
    let pdf_str = String::from_utf8_lossy(&result);
    // A regular PDF should not have pdfaid conformance metadata
    assert!(
        !pdf_str.contains("pdfaid:conformance"),
        "Regular PDF should not contain PDF/A conformance metadata"
    );
}

#[test]
fn test_compile_with_font_paths_empty() {
    // Empty font paths should work the same as without
    let result = compile_to_pdf("Hello!", &[], None, &[], false, false).unwrap();
    assert!(!result.is_empty());
    assert!(result.starts_with(b"%PDF"));
}

#[test]
fn test_compile_with_caller_provided_in_memory_font() {
    let carrier = include_bytes!("../../../../tests/fixtures/docx/wasm_embedded_cjk.docx");
    let embedded = crate::parser::embedded_fonts::extract_embedded_font_data(
        carrier,
        crate::config::Format::Docx,
    )
    .expect("the #943 fixture should carry an embedded font");
    let fonts = load_fonts_from_bytes(embedded.font_bytes());
    assert!(!fonts.is_empty(), "the fixture font should parse");

    let pdf = compile_to_pdf_with_fonts(
        r#"#text(font: ("No Such Font", "Noto Sans SC"))[Hello 中文测试文档]"#,
        &[],
        None,
        &[],
        &fonts,
        false,
        false,
    )
    .expect("the in-memory face should compile on native targets too");

    assert!(
        pdf.windows(b"NotoSansSC".len())
            .any(|window| window == b"NotoSansSC"),
        "the output PDF should embed the caller-provided face"
    );
}

#[test]
fn test_in_memory_last_resort_bypasses_process_metric_cache() {
    let carrier = include_bytes!("../../../../tests/fixtures/docx/wasm_embedded_cjk.docx");
    let embedded = crate::parser::embedded_fonts::extract_embedded_font_data(
        carrier,
        crate::config::Format::Docx,
    )
    .expect("the #943 fixture should carry an embedded font");
    let fonts = load_fonts_from_bytes(embedded.font_bytes());
    let font = fonts.first().expect("the fixture font should parse");
    let ttf = font.ttf();
    let upem = f64::from(ttf.units_per_em()).max(1.0);
    let expected_pitch =
        (f64::from(ttf.ascender()) - f64::from(ttf.descender()) + f64::from(ttf.line_gap())) / upem;

    // Populate the process cache from the machine's ordinary font set first.
    let _ = font_line_metrics_em("SimSun");

    let context = crate::render::font_context::resolve_font_search_context_from_fonts(&fonts)
        .with_last_resort_family(Some("Noto Sans SC"));
    let actual = crate::render::font_subst::with_font_search_context(Some(&context), || {
        font_line_metrics_em("SimSun")
    })
    .expect("the active in-memory last resort should provide metrics");

    assert!(
        (actual.2 - expected_pitch).abs() < 1e-12,
        "active metrics must come from Noto Sans SC even after SimSun was cached: {actual:?}"
    );
}

#[test]
fn test_path_font_last_resort_bypasses_process_metric_caches() {
    let carrier = include_bytes!("../../../../tests/fixtures/docx/wasm_embedded_cjk.docx");
    let embedded_data = crate::parser::embedded_fonts::extract_embedded_font_data(
        carrier,
        crate::config::Format::Docx,
    )
    .expect("the #969 fixture should carry an embedded font");
    let fonts = load_fonts_from_bytes(embedded_data.font_bytes());
    let font = fonts.first().expect("the fixture font should parse");
    let ttf = font.ttf();
    let upem = f64::from(ttf.units_per_em()).max(1.0);
    let expected = (
        (f64::from(ttf.ascender()) + f64::from(ttf.line_gap())) / upem,
        -f64::from(ttf.descender()) / upem,
        (f64::from(ttf.ascender()) - f64::from(ttf.descender()) + f64::from(ttf.line_gap())) / upem,
    );
    let expected_hhea_ascender = f64::from(ttf.tables().hhea.ascender) / upem;
    let expected_cap_height = font.metrics().cap_height.get();
    let ascent = f64::from(ttf.ascender()).abs() / upem;
    let descent = f64::from(ttf.descender()).abs() / upem;
    let line_gap = f64::from(ttf.line_gap()).abs() / upem;
    // What this test pins is which *face* answers, not which split rule: the
    // rule itself is covered by
    // `test_powerpoint_line_box_shares_the_gap_inclusive_line` and
    // `an_overflowing_face_shares_the_line_box_in_its_own_proportion`.
    let expected_powerpoint = powerpoint_line_box_split_em(ascent, descent, line_gap)
        .expect("the fixture face declares an ascent");

    // Native document fonts are materialized into a conversion-local path.
    // Prime the family-only process cache with a miss before that path becomes
    // active: the conversion context must still be allowed to resolve its own
    // final fallback face.
    let missing_family = "office2pdf issue 969 cache miss";
    assert!(font_line_metrics_em(missing_family).is_none());
    assert!(font_hhea_ascender_em(missing_family).is_none());
    assert!(font_cap_height_em(missing_family).is_none());
    let cached_powerpoint = powerpoint_line_box_em(missing_family)
        .expect("the ordinary PowerPoint metric falls back to Typst's default face");

    let embedded_dir =
        crate::parser::embedded_fonts::extract_embedded_fonts(carrier, crate::config::Format::Docx)
            .expect("the #969 fixture font should materialize");
    let context = crate::render::font_context::resolve_font_search_context(&[embedded_dir
        .path()
        .to_path_buf()])
    .with_last_resort_family(Some("Noto Sans SC"));
    let (actual, hhea_ascender, cap_height, powerpoint) =
        crate::render::font_subst::with_font_search_context(Some(&context), || {
            (
                font_line_metrics_em(missing_family),
                font_hhea_ascender_em(missing_family),
                font_cap_height_em(missing_family),
                powerpoint_line_box_em(missing_family),
            )
        });
    let actual = actual.expect("the active path font should provide line metrics");
    let hhea_ascender = hhea_ascender.expect("the active path font should provide an ascender");
    let cap_height = cap_height.expect("the active path font should provide a cap height");
    let powerpoint = powerpoint.expect("the active path font should provide a PowerPoint split");

    assert!(
        (actual.0 - expected.0).abs() < 1e-12
            && (actual.1 - expected.1).abs() < 1e-12
            && (actual.2 - expected.2).abs() < 1e-12,
        "active metrics must come from the materialized Noto Sans SC face: {actual:?}"
    );
    assert!((hhea_ascender - expected_hhea_ascender).abs() < 1e-12);
    assert!((cap_height - expected_cap_height).abs() < 1e-12);
    assert!(
        (powerpoint.0 - expected_powerpoint.0).abs() < 1e-12
            && (powerpoint.1 - expected_powerpoint.1).abs() < 1e-12,
        "active PowerPoint metrics must come from Noto Sans SC: {powerpoint:?}"
    );
    assert_ne!(
        powerpoint, cached_powerpoint,
        "the materialized face should replace the cached default-font split"
    );
}

#[test]
fn test_compile_with_nonexistent_font_path() {
    // Non-existent font path should not crash — FontSearcher skips invalid dirs
    let paths = vec![PathBuf::from("/nonexistent/font/path")];
    let result = compile_to_pdf("Hello!", &[], None, &paths, false, false).unwrap();
    assert!(!result.is_empty());
    assert!(result.starts_with(b"%PDF"));
}

#[test]
fn test_compile_with_embedded_image() {
    let png_data = make_test_png();
    let images = vec![ImageAsset {
        path: "img-0.png".to_string(),
        data: png_data,
    }];
    let source = r#"#image("img-0.png", width: 100pt)"#;
    let result = compile_to_pdf(source, &images, None, &[], false, false).unwrap();
    assert!(!result.is_empty());
    assert!(result.starts_with(b"%PDF"));
}

#[test]
fn test_compile_with_embedded_svg_image() {
    let svg_data = make_test_svg();
    let images = vec![ImageAsset {
        path: "img-0.svg".to_string(),
        data: svg_data,
    }];
    let source = r#"#image("img-0.svg", width: 100pt)"#;
    let result = compile_to_pdf(source, &images, None, &[], false, false).unwrap();
    assert!(!result.is_empty());
    assert!(result.starts_with(b"%PDF"));
}

#[test]
fn test_embedded_only_world_produces_valid_pdf() {
    // Simulates the WASM code path: embedded fonts only, no system fonts.
    // This verifies that the embedded-only MinimalWorld can produce valid PDFs.
    let world = MinimalWorld::new_embedded_only("Hello from embedded-only world!", &[]);
    assert!(
        world.font_source.len() > 0,
        "Embedded-only world should have fonts"
    );

    let warned = typst::compile::<typst::layout::PagedDocument>(&world);
    let document = warned.output.expect("Compilation should succeed");
    let pdf = typst_pdf::pdf(&document, &typst_pdf::PdfOptions::default())
        .expect("PDF export should succeed");
    assert!(pdf.starts_with(b"%PDF"));
}

#[test]
fn test_embedded_only_world_has_fonts() {
    // The embedded-only constructor (used on WASM) must have at least
    // the embedded fallback fonts (Libertinus, New Computer Modern, DejaVu).
    let world = MinimalWorld::new_embedded_only("", &[]);
    let embedded_count = {
        let mut s = FontSearcher::new();
        s.include_system_fonts(false);
        s.search().fonts.len()
    };
    assert_eq!(
        world.font_source.len(),
        embedded_count,
        "Embedded-only world should have exactly the embedded fonts"
    );
}

#[test]
fn test_pdfa_timestamp_is_not_hardcoded() {
    // PDF/A output should contain the actual conversion timestamp,
    // not the previously hardcoded 2024-01-01.
    let result = compile_to_pdf(
        "Timestamp test",
        &[],
        Some(crate::config::PdfStandard::PdfA2b),
        &[],
        false,
        false,
    )
    .unwrap();
    let pdf_str = String::from_utf8_lossy(&result);
    // The old hardcoded date was 2024-01-01T00:00:00 — it should no longer appear
    assert!(
        !pdf_str.contains("2024-01-01T00:00:00"),
        "PDF/A timestamp should not be the hardcoded 2024-01-01T00:00:00"
    );
}

#[test]
fn test_current_utc_datetime_is_valid() {
    // The helper should produce a valid Datetime that can create a Timestamp.
    let dt = current_utc_datetime();
    let _ts = typst_pdf::Timestamp::new_utc(dt);
}

#[test]
fn test_pdfa_timestamp_has_recent_date() {
    // The PDF/A XMP metadata should contain a date from the current
    // decade, not a hardcoded past date.
    let result = compile_to_pdf(
        "Year test",
        &[],
        Some(crate::config::PdfStandard::PdfA2b),
        &[],
        false,
        false,
    )
    .unwrap();
    let pdf_str = String::from_utf8_lossy(&result);
    // The XMP metadata should contain a CreateDate field
    assert!(
        pdf_str.contains("xmp:CreateDate") || pdf_str.contains("CreateDate"),
        "PDF/A should contain creation date metadata"
    );
    // The date should NOT be the hardcoded 2024-01-01
    assert!(
        !pdf_str.contains("2024-01-01"),
        "PDF/A timestamp should not contain hardcoded 2024-01-01"
    );
}

// --- PDF output size optimization tests (US-089) ---

#[test]
fn test_pdf_uses_flate_compression() {
    // typst-pdf (via krilla) compresses content streams with FLATE by default.
    // Verify that the output PDF contains FlateDecode filter references.
    let source = "Hello, compressed world! ".repeat(100);
    let result = compile_to_pdf(&source, &[], None, &[], false, false).unwrap();
    let pdf_str = String::from_utf8_lossy(&result);
    assert!(
        pdf_str.contains("FlateDecode"),
        "PDF content streams should use FlateDecode compression"
    );
}

#[test]
fn test_font_subsetting_reduces_size() {
    // A PDF using only a few glyphs should be significantly smaller than
    // one using many distinct glyphs, demonstrating font subsetting is active.
    // "Few glyphs" document: only ASCII letters a-z
    let few_glyphs = compile_to_pdf("abcdefghij", &[], None, &[], false, false).unwrap();

    // "Many glyphs" document: diverse characters force more glyph data.
    // Avoid Typst special characters (#, $, *, _, etc.) to keep it valid markup.
    let many_glyphs_source = "abcdefghijklmnopqrstuvwxyz \
        ABCDEFGHIJKLMNOPQRSTUVWXYZ 0123456789 \
        The quick brown fox jumps over the lazy dog. \
        SPHINX OF BLACK QUARTZ, JUDGE MY VOW. \
        Pack my box with five dozen liquor jugs. \
        How vexingly quick daft zebras jump.";
    let many_glyphs = compile_to_pdf(many_glyphs_source, &[], None, &[], false, false).unwrap();

    // With font subsetting, the "few glyphs" PDF should be noticeably smaller.
    // Without subsetting, both would embed the full font and be similar in size.
    assert!(
        few_glyphs.len() < many_glyphs.len(),
        "PDF with fewer glyphs ({} bytes) should be smaller than PDF with many glyphs ({} bytes), \
         indicating font subsetting is active",
        few_glyphs.len(),
        many_glyphs.len()
    );
}

#[test]
fn test_multipage_text_pdf_size_reasonable() {
    // A 10-page text-only document should produce a PDF well under 500KB.
    // This verifies that compression and font subsetting keep output compact.
    //
    // typst-pdf behavior (verified):
    // - Content streams use FLATE compression (compress_content_streams: true)
    // - Fonts are automatically subset to include only used glyphs
    // - No unnecessary re-encoding of embedded data
    let mut source = String::new();
    for i in 1..=10 {
        if i > 1 {
            source.push_str("#pagebreak()\n");
        }
        source.push_str(&format!(
            "= Page {i}\n\n\
             This is page {i} of a multi-page document used to verify \
             that PDF output size remains reasonable with compression \
             and font subsetting enabled.\n\n\
             Lorem ipsum dolor sit amet, consectetur adipiscing elit. \
             Sed do eiusmod tempor incididunt ut labore et dolore magna \
             aliqua. Ut enim ad minim veniam, quis nostrud exercitation \
             ullamco laboris nisi ut aliquip ex ea commodo consequat.\n\n"
        ));
    }
    let result = compile_to_pdf(&source, &[], None, &[], false, false).unwrap();

    // 500KB = 512_000 bytes — generous upper bound for 10 pages of text
    assert!(
        result.len() < 512_000,
        "10-page text-only PDF should be under 500KB, actual size: {} bytes ({:.1} KB)",
        result.len(),
        result.len() as f64 / 1024.0
    );
}

#[test]
fn test_pdf_with_image_size_proportional() {
    // A PDF with an embedded image should not inflate the image size
    // significantly. The output PDF should be proportional to the input
    // image data size (not orders of magnitude larger from re-encoding).
    let png_data = make_test_png();
    let png_size = png_data.len();
    let images = vec![ImageAsset {
        path: "img-0.png".to_string(),
        data: png_data,
    }];
    let source = r#"#image("img-0.png", width: 100pt)"#;
    let result = compile_to_pdf(source, &images, None, &[], false, false).unwrap();

    // The PDF has overhead (fonts, structure, metadata) beyond the image.
    // But the total should not be unreasonably large for a tiny 1x1 image.
    // A 1x1 PNG is ~70 bytes; the PDF overhead is typically 10-30KB (fonts).
    // We assert the total is under 100KB to catch re-encoding issues.
    assert!(
        result.len() < 100_000,
        "PDF with tiny 1x1 image should be under 100KB, actual: {} bytes ({:.1} KB). \
         Image was {} bytes. Possible image re-encoding issue.",
        result.len(),
        result.len() as f64 / 1024.0,
        png_size
    );
}

#[test]
fn test_empty_page_pdf_baseline_size() {
    // An empty page PDF establishes the baseline overhead (fonts, structure).
    // This helps verify that additional content adds proportional size, not
    // excessive bloat from uncompressed data.
    let result = compile_to_pdf("", &[], None, &[], false, false).unwrap();

    // Empty page PDF should be compact — mostly font data and PDF structure.
    // Typically 10-30KB depending on embedded font data.
    assert!(
        result.len() < 100_000,
        "Empty page PDF should be under 100KB (baseline), actual: {} bytes ({:.1} KB)",
        result.len(),
        result.len() as f64 / 1024.0
    );
}

#[test]
fn test_compression_effective_for_repetitive_content() {
    // FLATE compression is especially effective on repetitive content.
    // A document with highly repetitive text should compress well,
    // producing a PDF not much larger than a document with less text.
    let short_source = "Hello world.\n\n";
    let short_pdf = compile_to_pdf(short_source, &[], None, &[], false, false).unwrap();

    // 100x the text content, but should compress to much less than 100x the size
    let long_source = "Hello world.\n\n".repeat(100);
    let long_pdf = compile_to_pdf(&long_source, &[], None, &[], false, false).unwrap();

    // With compression, 100x content should produce far less than 10x the PDF size.
    // The ratio demonstrates that content streams are being compressed.
    let size_ratio = long_pdf.len() as f64 / short_pdf.len() as f64;
    assert!(
        size_ratio < 10.0,
        "100x content should produce less than 10x PDF size with compression. \
         Short: {} bytes, Long: {} bytes, Ratio: {:.1}x",
        short_pdf.len(),
        long_pdf.len(),
        size_ratio
    );
}

// --- Tagged PDF and PDF/UA tests (US-096) ---

#[test]
fn test_tagged_pdf_contains_structure_tags() {
    // A tagged PDF with headings should contain StructTreeRoot and heading tags
    let source = "= My Heading\n\nSome paragraph text.\n\n== Sub Heading\n\nMore text.";
    let result = compile_to_pdf(source, &[], None, &[], true, false).unwrap();
    assert!(result.starts_with(b"%PDF"));
    let pdf_str = String::from_utf8_lossy(&result);
    // Tagged PDFs must contain a StructTreeRoot
    assert!(
        pdf_str.contains("StructTreeRoot") || pdf_str.contains("MarkInfo"),
        "Tagged PDF should contain structure tree or mark info"
    );
}

#[test]
fn test_untagged_pdf_no_structure_tree() {
    // Without tagging, there should be no StructTreeRoot
    let source = "= My Heading\n\nSome text.";
    let result = compile_to_pdf(source, &[], None, &[], false, false).unwrap();
    assert!(result.starts_with(b"%PDF"));
    let pdf_str = String::from_utf8_lossy(&result);
    assert!(
        !pdf_str.contains("StructTreeRoot"),
        "Untagged PDF should not contain StructTreeRoot"
    );
}

#[test]
fn test_pdf_ua_produces_valid_pdf() {
    // PDF/UA mode should produce a valid PDF with tagging enabled.
    // PDF/UA-1 requires a document title.
    let source = "#set document(title: \"Accessible Document\")\n= Accessible Document\n\nThis document is PDF/UA compliant.";
    let result = compile_to_pdf(source, &[], None, &[], false, true).unwrap();
    assert!(result.starts_with(b"%PDF"));
    let pdf_str = String::from_utf8_lossy(&result);
    // PDF/UA output should contain pdfuaid metadata
    assert!(
        pdf_str.contains("pdfuaid"),
        "PDF/UA output should contain pdfuaid metadata"
    );
}

#[test]
fn test_pdf_ua_implies_tagged() {
    // PDF/UA should produce a tagged PDF even if tagged=false.
    // PDF/UA-1 requires a document title.
    let source = "#set document(title: \"Test\")\n= Heading\n\nParagraph.";
    let result = compile_to_pdf(source, &[], None, &[], false, true).unwrap();
    let pdf_str = String::from_utf8_lossy(&result);
    assert!(
        pdf_str.contains("StructTreeRoot") || pdf_str.contains("MarkInfo"),
        "PDF/UA should produce tagged PDF"
    );
}

#[test]
fn test_tagged_pdf_with_table() {
    let source = "#table(columns: 2, [A], [B], [C], [D])";
    let result = compile_to_pdf(source, &[], None, &[], true, false).unwrap();
    assert!(result.starts_with(b"%PDF"));
    // Should be a valid PDF (compilation doesn't fail with tagging)
}

#[test]
fn test_tagged_pdf_with_pdfa_combined() {
    // Tagged + PDF/A should work together
    let source = "= Archival Accessible\n\nBoth standards combined.";
    let result = compile_to_pdf(
        source,
        &[],
        Some(crate::config::PdfStandard::PdfA2b),
        &[],
        true,
        false,
    )
    .unwrap();
    assert!(result.starts_with(b"%PDF"));
    let pdf_str = String::from_utf8_lossy(&result);
    assert!(pdf_str.contains("pdfaid"), "Should contain PDF/A metadata");
    assert!(
        pdf_str.contains("StructTreeRoot") || pdf_str.contains("MarkInfo"),
        "Should contain structure tags"
    );
}

/// The embedded Libertinus Serif faces make the token measurement
/// deterministic on every target (like the digit-advance pin for #621).
/// Ground truth from fontTools `hmtx` sums on the typst-assets faces:
/// "Total" is 2.138em regular and 2.392em bold at 1000 upem — the bold face
/// must be selected for bold runs, not the regular one (issue #624).
#[test]
fn test_text_advance_em_reads_regular_and_bold_faces() {
    let regular: f64 = text_advance_em("Libertinus Serif", false, "Total")
        .expect("the embedded Libertinus Serif regular face must resolve");
    assert!(
        (regular - 2.138).abs() < 1e-6,
        "regular 'Total' should be 2.138em, got {regular}"
    );

    let bold: f64 = text_advance_em("Libertinus Serif", true, "Total")
        .expect("the embedded Libertinus Serif bold face must resolve");
    assert!(
        (bold - 2.392).abs() < 1e-6,
        "bold 'Total' should be 2.392em, got {bold}"
    );
}

/// A character without a glyph (U+E000 private use) yields `None` so the
/// caller can degrade to a measurement-free path; an empty string is a valid
/// zero-width measurement.
#[test]
fn test_text_advance_em_is_none_for_missing_glyphs() {
    assert_eq!(text_advance_em("Libertinus Serif", false, "\u{E000}"), None);
    assert_eq!(text_advance_em("Libertinus Serif", false, ""), Some(0.0));
}

/// A caller that quantizes advances one at a time needs them one at a time.
///
/// Ground truth from the typst-assets Libertinus Serif regular `hmtx`, at its
/// 1000-unit em: `T` 597, `o` 504, `t` 316, `a` 457, `l` 264. Their sum is the
/// 2.138em [`text_advance_em`] reports, but Excel rounds each one to a whole
/// point before adding it, and at 10pt those two orders differ by 0.62pt
/// (issue #1088).
#[test]
fn test_glyph_advances_em_reports_each_glyph_separately() {
    let advances: Vec<f64> = glyph_advances_em("Libertinus Serif", false, "Total")
        .expect("the embedded Libertinus Serif regular face must resolve");
    let expected: [f64; 5] = [0.597, 0.504, 0.316, 0.457, 0.264];
    assert_eq!(advances.len(), expected.len(), "one advance per glyph");
    for (glyph, (advance, want)) in advances.iter().zip(expected).enumerate() {
        assert!(
            (advance - want).abs() < 1e-9,
            "glyph {glyph} of 'Total' should advance {want}em, got {advance}"
        );
    }
    let sum: f64 = advances.iter().sum();
    let total: f64 = text_advance_em("Libertinus Serif", false, "Total")
        .expect("the embedded Libertinus Serif regular face must resolve");
    assert!(
        (sum - total).abs() < 1e-9,
        "the parts must sum to the whole: {sum} against {total}"
    );
}

/// PowerPoint shares its 1.2em line in the proportion of the face's own hhea
/// line — **line gap included** — and the ascent side gets only the bare
/// ascender.
///
/// Arial is the face that shows the gap term, since it is one of the few in the
/// corpus that declares one (hhea ascender 1854, descender -434, **line gap
/// 67** per 2048 upem). Its three candidate shares are:
///
/// - gap-inclusive proportional `1854/2355 x 1.2` = **0.94471em**
/// - even split `(1.2 + 0.9053 - 0.2119) / 2` = 0.94668em
/// - gap-free proportional `1854/2288 x 1.2` = 0.97238em
///
/// The first two differ by 0.002em and only separate at sizes where they round
/// to different points, which is why #660 could fit the even split to a 17pt
/// frame. They separate on the 28pt centred titles of the golden mocks, which
/// [`crate::render::typst_gen_fixed_page_textbox_tests`] pins against the
/// committed native exports (issue #1118).
#[test]
fn test_powerpoint_line_box_shares_the_gap_inclusive_line() {
    let Some((above, below)) = powerpoint_line_box_em("Arial") else {
        return; // no Arial-compatible face on this host
    };

    assert!(
        (above + below - POWERPOINT_LINE_HEIGHT_FACTOR).abs() < 1e-9,
        "the split must still span the 1.2em line, got {above} + {below}"
    );
    assert!(
        (above - 0.94471).abs() < 0.001,
        "Arial's first baseline must sit 0.94471em below the box top — its own \
         share of the box counting the hhea line gap — not {above}em"
    );
}

/// Every Arial seat the committed golden-mock exports carry, at the twelve
/// sizes those decks use.
///
/// The figures come from the native PowerPoint 16.111 exports under
/// `tests/golden_mocks/business/expected/pptx/`, traced with
/// `mutool draw -F trace`. A frame's content top is its `a:off` plus the
/// `a:bodyPr` top inset; a centred single-line frame seats its 1.2em line
/// `(content height - 1.2 x size) / 2` further down. Subtracting that leaves
/// the seat inside the line, which the exports put on a whole point (#1074).
///
/// This is what re-checking #660's `08_marketing_report_en` p3 frame against
/// its native export turned up, as issue #1118 asked: the 28pt cells separate
/// the gap-inclusive share from the even split, and the even split is the one
/// that misses them. Arial's own line gap is what the two disagree about, so
/// the same table also fixes where PowerPoint puts that gap — a gap given to
/// the ascent side, or halved across both, lands on 31pt at 32pt where the
/// exports show 30.
#[test]
fn arial_slide_seats_match_the_golden_mock_exports() {
    // Arial: hhea ascender 1854, descender -434, line gap 67 per 2048 upem.
    let upem: f64 = 2048.0;
    let ascent_em: f64 = 1854.0 / upem;
    let descent_em: f64 = 434.0 / upem;
    let line_gap_em: f64 = 67.0 / upem;

    let (above, below) = powerpoint_line_box_split_em(ascent_em, descent_em, line_gap_em)
        .expect("a positive ascent splits the line box");
    assert!(
        (above + below - POWERPOINT_LINE_HEIGHT_FACTOR).abs() < 1e-9,
        "the split must still span the 1.2em line, got {above} + {below}"
    );

    // (font size pt, the seat the exports put inside the line, in pt).
    const EXPORTED: [(f64, f64); 12] = [
        (12.0, 11.04),
        (12.5, 12.06),
        (13.0, 11.88),
        (14.5, 13.92),
        (15.0, 13.92),
        (17.0, 16.08),
        (18.0, 17.04),
        (19.0, 17.88),
        (28.0, 26.04),
        (30.0, 28.08),
        (32.0, 30.00),
        (40.0, 37.92),
    ];
    // The exports quantise a position to a 0.24pt grid, so a whole-point seat
    // is within half of that of the measured one or it is a different model.
    const HALF_GRID_PT: f64 = 0.12 + 1e-9;

    let natural_em: f64 = ascent_em + descent_em + line_gap_em;
    let rivals: [(&str, f64); 3] = [
        (
            "the even split",
            (POWERPOINT_LINE_HEIGHT_FACTOR + ascent_em - descent_em) / 2.0,
        ),
        (
            "a gap-free proportional share",
            POWERPOINT_LINE_HEIGHT_FACTOR * ascent_em / (ascent_em + descent_em),
        ),
        (
            "the gap given to the ascent side",
            POWERPOINT_LINE_HEIGHT_FACTOR * (ascent_em + line_gap_em) / natural_em,
        ),
    ];
    let mut rival_misses: [usize; 3] = [0; 3];

    for (size_pt, export_pt) in EXPORTED {
        let seat_pt: f64 = (above * size_pt).round();
        assert!(
            (seat_pt - export_pt).abs() <= HALF_GRID_PT,
            "at {size_pt}pt the exports seat the baseline {export_pt}pt into the \
             line; the split predicts {seat_pt}pt"
        );
        for (index, (_, share_em)) in rivals.iter().enumerate() {
            if ((share_em * size_pt).round() - export_pt).abs() > HALF_GRID_PT {
                rival_misses[index] += 1;
            }
        }
    }

    // Triangulation: each rival has to be ruled out by this table, or the table
    // would pass on it too. The even split survives all but the 28pt cells,
    // which is why it stood until #1118.
    for ((name, _), misses) in rivals.iter().zip(rival_misses) {
        assert!(
            misses > 0,
            "{name} must be a model this table rules out, but it misses none of \
             the {} cells",
            EXPORTED.len()
        );
    }
    assert_eq!(
        rival_misses[0], 1,
        "only the 28pt cells separate the even split from the gap-inclusive share"
    );
}

/// A face whose own line *fits* inside the 1.2em box is shared like any other
/// — the extra leading is not halved.
///
/// Measured on a native PowerPoint 16.112 export of a one-factor probe deck:
/// bottom-anchored text boxes with every inset zeroed, traced with
/// `mutool draw -F trace`, at the 14 sizes below. Georgia is the probe's only
/// face that fits the box (hhea 1878/-449 per 2048 upem, no line gap, so a
/// 1.13623em line), which is what lets it tell the two shares apart: its
/// proportional share is 0.968457em and its even share 0.948877em. A
/// bottom-anchored box keeps `1.2 x size - round(share x size)` under its last
/// baseline, the seat being rounded to a whole point (issue #1074).
///
/// The even split is outside the export's 0.12pt half-grid at 9 of the 14
/// sizes and 2.04pt out at 72pt; the proportional share is inside it at all 14.
/// The five sizes where they agree — 8, 18, 24, 28 and 48 — are the ones where
/// they round to the same point, which is how the branch survived #660 (issue
/// #1118).
#[test]
fn a_face_that_fits_the_line_box_is_shared_like_any_other() {
    // Georgia: hhea ascender 1878, descender -449, no line gap, 2048 upem.
    let upem: f64 = 2048.0;
    let ascent_em: f64 = 1878.0 / upem;
    let descent_em: f64 = 449.0 / upem;
    assert!(
        ascent_em + descent_em < POWERPOINT_LINE_HEIGHT_FACTOR,
        "this test needs a face that fits the box, got {}em",
        ascent_em + descent_em
    );

    let (above, below) = powerpoint_line_box_split_em(ascent_em, descent_em, 0.0)
        .expect("a positive ascent splits the line box");
    assert!(
        (above + below - POWERPOINT_LINE_HEIGHT_FACTOR).abs() < 1e-9,
        "the split must still span the 1.2em line, got {above} + {below}"
    );

    // (font size pt, the gap the export keeps below the last baseline in pt).
    const PROBE: [(f64, f64); 14] = [
        (8.0, 1.680),
        (11.0, 2.160),
        (14.0, 2.800),
        (18.0, 4.560),
        (24.0, 5.920),
        (28.0, 6.560),
        (32.0, 7.400),
        (36.0, 8.200),
        (40.0, 8.960),
        (44.0, 9.880),
        (48.0, 11.560),
        (54.0, 12.760),
        (72.0, 16.360),
        (100.0, 23.120),
    ];
    // The exports quantise a position to a 0.24pt grid, so a modelled gap is
    // within half of that of the measured one or it is a different model. Two
    // of the 14 sizes land exactly on that half-grid, hence the float slack.
    const HALF_GRID_PT: f64 = 0.12 + 1e-9;
    let gap_pt = |share_em: f64, size_pt: f64| -> f64 {
        POWERPOINT_LINE_HEIGHT_FACTOR * size_pt - (share_em * size_pt).round()
    };

    let even_em: f64 = (POWERPOINT_LINE_HEIGHT_FACTOR + ascent_em - descent_em) / 2.0;
    let mut even_misses: usize = 0;
    for (size_pt, export_pt) in PROBE {
        let modelled_pt: f64 = gap_pt(above, size_pt);
        assert!(
            (modelled_pt - export_pt).abs() <= HALF_GRID_PT,
            "at {size_pt}pt the export keeps {export_pt}pt under the baseline; \
             the split predicts {modelled_pt}pt"
        );
        if (gap_pt(even_em, size_pt) - export_pt).abs() > HALF_GRID_PT {
            even_misses += 1;
        }
    }

    // Triangulation: without this the even split would pass the loop above at
    // the five sizes where the two shares round to the same point.
    assert_eq!(
        even_misses, 9,
        "the even split must still be the model this probe rules out"
    );
}

#[test]
fn the_world_hands_typst_the_face_that_kerns_from_the_legacy_table() {
    // Every face Typst shapes with comes through `World::font`, so that is
    // where the kern-source choice has to land: a face carrying both sources
    // must reach the shaper without its GPOS `kern` feature (issue #1116).
    let base: &[u8] = include_bytes!("../../fonts/NotoSansCJKsc-GB2312.otf");
    let font = Font::new(
        Bytes::new(crate::test_support::make_face_carrying_both_kern_sources(
            base,
        )),
        0,
    )
    .expect("the rebuilt face parses");
    assert!(crate::test_support::states_a_gpos_kern_feature(&font));

    let world = MinimalWorld::new_embedded_with_fonts("", &[], std::slice::from_ref(&font));
    let handed_over = world.font(0).expect("the in-memory face leads the book");

    assert!(
        !crate::test_support::states_a_gpos_kern_feature(&handed_over),
        "the compiler must be handed the face that kerns from the legacy table"
    );
    assert!(handed_over.ttf().tables().kern.is_some());
}

#[test]
fn the_world_hands_typst_a_one_source_face_unchanged() {
    // A face that states its pairs in GPOS alone has nothing to fall back to,
    // so it must reach the shaper exactly as it was loaded.
    let base: &[u8] = include_bytes!("../../fonts/NotoSansCJKsc-GB2312.otf");
    let font = Font::new(Bytes::new(base.to_vec()), 0).expect("the bundled face parses");

    let world = MinimalWorld::new_embedded_with_fonts("", &[], std::slice::from_ref(&font));
    let handed_over = world.font(0).expect("the in-memory face leads the book");

    assert!(crate::test_support::states_a_gpos_kern_feature(
        &handed_over
    ));
    assert_eq!(handed_over.data().len(), font.data().len());
}
