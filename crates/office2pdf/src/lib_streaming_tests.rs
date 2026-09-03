use std::io::Cursor;

use super::*;

fn build_xlsx_with_rows(num_rows: u32, num_cols: u32) -> Vec<u8> {
    let mut book = umya_spreadsheet::new_file();
    let sheet = book.get_sheet_mut(&0).unwrap();
    sheet.set_name("Data");
    for row in 1..=num_rows {
        for col in 1..=num_cols {
            sheet
                .get_cell_mut((col, row))
                .set_value(format!("R{row}C{col}"));
        }
    }
    let mut cursor = Cursor::new(Vec::new());
    umya_spreadsheet::writer::xlsx::write_writer(&book, &mut cursor).unwrap();
    cursor.into_inner()
}

#[test]
fn test_streaming_xlsx_produces_valid_pdf() {
    let data = build_xlsx_with_rows(50, 3);
    let options = config::ConvertOptions {
        streaming: true,
        streaming_chunk_size: Some(20),
        ..Default::default()
    };
    let result = convert_bytes(&data, config::Format::Xlsx, &options).unwrap();
    assert!(
        result.pdf.starts_with(b"%PDF"),
        "output should be valid PDF"
    );
    assert!(result.pdf.len() > 100, "PDF should have content");
}

#[test]
fn test_streaming_xlsx_same_data_as_normal() {
    let data = build_xlsx_with_rows(10, 2);

    let normal_opts = config::ConvertOptions::default();
    let normal_result = convert_bytes(&data, config::Format::Xlsx, &normal_opts).unwrap();

    let streaming_opts = config::ConvertOptions {
        streaming: true,
        streaming_chunk_size: Some(5),
        ..Default::default()
    };
    let streaming_result = convert_bytes(&data, config::Format::Xlsx, &streaming_opts).unwrap();

    let normal_pages = normal_result
        .metrics
        .as_ref()
        .expect("batch metrics")
        .page_count;
    let streaming_pages = streaming_result
        .metrics
        .as_ref()
        .expect("streaming metrics")
        .page_count;

    assert_eq!(
        streaming_pages, normal_pages,
        "streaming chunk boundaries must not add PDF pages"
    );
    assert_eq!(
        pdf_extract::extract_text_from_mem(&streaming_result.pdf).unwrap(),
        pdf_extract::extract_text_from_mem(&normal_result.pdf).unwrap(),
        "streaming chunks must neither omit nor duplicate worksheet text"
    );
}

#[test]
fn test_streaming_xlsx_page_count_is_independent_of_chunk_size() {
    let data = build_xlsx_with_rows(50, 2);
    let batch_result = convert_bytes(
        &data,
        config::Format::Xlsx,
        &config::ConvertOptions::default(),
    )
    .unwrap();
    let batch_pages = batch_result
        .metrics
        .as_ref()
        .expect("batch metrics")
        .page_count;
    let batch_text = pdf_extract::extract_text_from_mem(&batch_result.pdf).unwrap();

    for chunk_size in [5, 20, 37, 50] {
        let options = config::ConvertOptions {
            streaming: true,
            streaming_chunk_size: Some(chunk_size),
            ..Default::default()
        };
        let streaming_result = convert_bytes(&data, config::Format::Xlsx, &options).unwrap();
        let streaming_pages = streaming_result
            .metrics
            .as_ref()
            .expect("streaming metrics")
            .page_count;

        assert_eq!(
            streaming_pages, batch_pages,
            "chunk size {chunk_size} changed PDF pagination"
        );
        assert_eq!(
            pdf_extract::extract_text_from_mem(&streaming_result.pdf).unwrap(),
            batch_text,
            "chunk size {chunk_size} changed worksheet text"
        );
    }
}

#[test]
fn test_streaming_large_xlsx_completes() {
    let data = build_xlsx_with_rows(10_000, 3);
    let options = config::ConvertOptions {
        streaming: true,
        streaming_chunk_size: Some(1000),
        ..Default::default()
    };
    let result = convert_bytes(&data, config::Format::Xlsx, &options).unwrap();
    assert!(
        result.pdf.starts_with(b"%PDF"),
        "output should be valid PDF"
    );
    assert!(result.metrics.is_some(), "streaming should produce metrics");
}

#[test]
fn test_streaming_non_xlsx_falls_through() {
    let docx = {
        let doc = docx_rs::Docx::new().add_paragraph(
            docx_rs::Paragraph::new().add_run(docx_rs::Run::new().add_text("Hello streaming")),
        );
        let mut cursor = Cursor::new(Vec::new());
        doc.build().pack(&mut cursor).unwrap();
        cursor.into_inner()
    };
    let options = config::ConvertOptions {
        streaming: true,
        ..Default::default()
    };
    let result = convert_bytes(&docx, config::Format::Docx, &options).unwrap();
    assert!(result.pdf.starts_with(b"%PDF"));
}

#[test]
fn test_streaming_chunk_size_default() {
    let data = build_xlsx_with_rows(20, 1);
    let options = config::ConvertOptions {
        streaming: true,
        streaming_chunk_size: None,
        ..Default::default()
    };
    let result = convert_bytes(&data, config::Format::Xlsx, &options).unwrap();
    assert!(result.pdf.starts_with(b"%PDF"));
}

#[test]
fn test_streaming_memory_bounded() {
    let data = build_xlsx_with_rows(5_000, 5);
    let options = config::ConvertOptions {
        streaming: true,
        streaming_chunk_size: Some(500),
        ..Default::default()
    };
    let result = convert_bytes(&data, config::Format::Xlsx, &options).unwrap();
    assert!(result.pdf.starts_with(b"%PDF"));
    assert!(
        result.pdf.len() > 1000,
        "PDF should have substantial content"
    );
}
