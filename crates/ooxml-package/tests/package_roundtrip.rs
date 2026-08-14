//! Lossless round-trip tests for the OPC package model (issue #746).
//!
//! Every assertion here reads the saved container back with the `zip` crate
//! directly, so the crate under test never verifies itself.

use std::io::{Cursor, Read, Write};
use std::path::PathBuf;

use ooxml_package::{OpcPackage, PackageError, TargetMode, resolve_relationship_target};

/// OPC is format-agnostic: the round-trip guarantees must hold for all three
/// OOXML flavors, not just the PPTX milestone target.
const ROUND_TRIP_FIXTURES: [&str; 4] = [
    "pptx/minimal.pptx",
    "pptx/WithMaster.pptx",
    "docx/unit_test_headers.docx",
    "xlsx/temperature.xlsx",
];

fn fixture_path(relative: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/fixtures")
        .join(relative)
}

fn fixture_bytes(relative: &str) -> Vec<u8> {
    std::fs::read(fixture_path(relative)).expect("fixture should be readable")
}

/// `(name, decompressed payload)` for every non-directory entry, in order.
fn zip_entries(bytes: &[u8]) -> Vec<(String, Vec<u8>)> {
    let mut archive = zip::ZipArchive::new(Cursor::new(bytes)).expect("container should open");
    let mut entries: Vec<(String, Vec<u8>)> = Vec::new();
    for index in 0..archive.len() {
        let mut file = archive.by_index(index).expect("entry should open");
        if file.is_dir() {
            continue;
        }
        let mut payload: Vec<u8> = Vec::new();
        file.read_to_end(&mut payload)
            .expect("entry should decompress");
        entries.push((file.name().to_string(), payload));
    }
    entries
}

/// `(name, raw compressed payload)` for every non-directory entry, in order.
fn raw_zip_entries(bytes: &[u8]) -> Vec<(String, Vec<u8>)> {
    let mut archive = zip::ZipArchive::new(Cursor::new(bytes)).expect("container should open");
    let mut entries: Vec<(String, Vec<u8>)> = Vec::new();
    for index in 0..archive.len() {
        let mut file = archive.by_index_raw(index).expect("raw entry should open");
        if file.is_dir() {
            continue;
        }
        let mut payload: Vec<u8> = Vec::new();
        file.read_to_end(&mut payload)
            .expect("raw entry should read");
        entries.push((file.name().to_string(), payload));
    }
    entries
}

/// Rebuild `original` with extra entries appended, using the `zip` crate only.
fn with_extra_entries(original: &[u8], extras: &[(&str, &[u8])]) -> Vec<u8> {
    let mut archive = zip::ZipArchive::new(Cursor::new(original)).expect("container should open");
    let mut writer = zip::ZipWriter::new(Cursor::new(Vec::new()));
    for index in 0..archive.len() {
        let file = archive.by_index_raw(index).expect("raw entry should open");
        writer.raw_copy_file(file).expect("raw copy should succeed");
    }
    for (name, payload) in extras {
        writer
            .start_file(*name, zip::write::FileOptions::default())
            .expect("extra entry should start");
        writer.write_all(payload).expect("extra entry should write");
    }
    writer
        .finish()
        .expect("container should finish")
        .into_inner()
}

#[test]
fn no_op_round_trip_preserves_every_part_payload_and_order() {
    for fixture in ROUND_TRIP_FIXTURES {
        let original: Vec<u8> = fixture_bytes(fixture);
        let package = OpcPackage::from_bytes(original.clone())
            .unwrap_or_else(|error| panic!("{fixture} should load: {error}"));
        let saved: Vec<u8> = package
            .save_to_bytes()
            .unwrap_or_else(|error| panic!("{fixture} should save: {error}"));
        assert_eq!(
            zip_entries(&original),
            zip_entries(&saved),
            "{fixture}: part names, order, or payloads changed across a no-op round trip"
        );
    }
}

#[test]
fn untouched_part_payloads_are_copied_byte_for_byte_compressed() {
    for fixture in ROUND_TRIP_FIXTURES {
        let original: Vec<u8> = fixture_bytes(fixture);
        let package = OpcPackage::from_bytes(original.clone()).expect("fixture should load");
        let saved: Vec<u8> = package.save_to_bytes().expect("package should save");
        assert_eq!(
            raw_zip_entries(&original),
            raw_zip_entries(&saved),
            "{fixture}: untouched entries should keep their exact compressed payload"
        );
    }
}

#[test]
fn part_bytes_returns_the_decompressed_payload() {
    let original: Vec<u8> = fixture_bytes("pptx/WithMaster.pptx");
    let package = OpcPackage::from_bytes(original.clone()).expect("fixture should load");
    let expected: Vec<(String, Vec<u8>)> = zip_entries(&original);
    for (name, payload) in &expected {
        let actual = package.part_bytes(name).expect("part should be readable");
        assert_eq!(
            actual.as_ref(),
            payload.as_slice(),
            "part {name} should decompress to the original payload"
        );
    }
    assert_eq!(
        package.part_names(),
        expected
            .iter()
            .map(|(name, _)| name.as_str())
            .collect::<Vec<&str>>(),
        "part_names should list every part in container order"
    );
}

#[test]
fn unknown_parts_survive_a_round_trip() {
    let opaque_payload: Vec<u8> = (0u8..=255).cycle().take(1024).collect();
    let extras: [(&str, &[u8]); 2] = [
        (
            "customXml/unknown-tool-data.xml",
            b"<unknownTool><keep everything=\"intact\"/></unknownTool>".as_slice(),
        ),
        ("ppt/media/opaque-blob.bin", opaque_payload.as_slice()),
    ];
    let with_unknown: Vec<u8> = with_extra_entries(&fixture_bytes("pptx/WithMaster.pptx"), &extras);

    let package = OpcPackage::from_bytes(with_unknown.clone()).expect("package should load");
    for (name, payload) in &extras {
        assert!(
            package.has_part(name),
            "unknown part {name} should be visible"
        );
        assert_eq!(
            package
                .part_bytes(name)
                .expect("unknown part should read")
                .as_ref(),
            *payload,
            "unknown part {name} should keep its payload in memory"
        );
    }

    let saved: Vec<u8> = package.save_to_bytes().expect("package should save");
    assert_eq!(
        zip_entries(&with_unknown),
        zip_entries(&saved),
        "unknown parts must survive a no-op round trip untouched"
    );
}

#[test]
fn modifying_one_part_rewrites_only_that_entry() {
    let original: Vec<u8> = fixture_bytes("pptx/WithMaster.pptx");
    let target_part: &str = "ppt/slides/slide2.xml";

    let mut package = OpcPackage::from_bytes(original.clone()).expect("fixture should load");
    let original_slide: Vec<u8> = package
        .part_bytes(target_part)
        .expect("slide should be readable")
        .into_owned();
    let modified_slide: Vec<u8> = String::from_utf8(original_slide.clone())
        .expect("slide XML should be UTF-8")
        .replace("subtitle", "caption")
        .into_bytes();
    assert_ne!(
        original_slide, modified_slide,
        "the edit should change the slide"
    );

    package
        .set_part_bytes(target_part, modified_slide.clone())
        .expect("existing part should accept new bytes");
    assert_eq!(
        package.dirty_part_names(),
        vec![target_part],
        "exactly the edited part should be dirty"
    );

    let saved: Vec<u8> = package.save_to_bytes().expect("package should save");
    let original_raw: Vec<(String, Vec<u8>)> = raw_zip_entries(&original);
    let saved_raw: Vec<(String, Vec<u8>)> = raw_zip_entries(&saved);
    assert_eq!(
        original_raw
            .iter()
            .map(|(name, _)| name.as_str())
            .collect::<Vec<&str>>(),
        saved_raw
            .iter()
            .map(|(name, _)| name.as_str())
            .collect::<Vec<&str>>(),
        "entry names and order should be preserved"
    );
    for ((name, original_payload), (_, saved_payload)) in original_raw.iter().zip(saved_raw.iter())
    {
        if name == target_part {
            continue;
        }
        assert_eq!(
            original_payload, saved_payload,
            "untouched entry {name} should keep its exact compressed payload"
        );
    }
    assert_eq!(
        zip_entries(&saved)
            .into_iter()
            .find(|(name, _)| name == target_part)
            .expect("edited part should exist")
            .1,
        modified_slide,
        "the edited part should carry the replacement payload"
    );
}

#[test]
fn relationships_resolve_to_existing_parts_before_and_after_round_trip() {
    for fixture in ROUND_TRIP_FIXTURES {
        let package = OpcPackage::from_bytes(fixture_bytes(fixture)).expect("fixture should load");
        assert_relationships_resolve(&package, fixture);

        let reloaded =
            OpcPackage::from_bytes(package.save_to_bytes().expect("package should save"))
                .expect("saved container should reload");
        assert_relationships_resolve(&reloaded, fixture);
    }
}

fn assert_relationships_resolve(package: &OpcPackage, fixture: &str) {
    let root_relationships: Vec<ooxml_package::Relationship> = package
        .relationships(None)
        .expect("root relationships should parse");
    let office_document = root_relationships
        .iter()
        .find(|relationship| relationship.relationship_type.ends_with("/officeDocument"))
        .unwrap_or_else(|| panic!("{fixture} should declare an officeDocument relationship"));
    assert!(
        package.has_part(&resolve_relationship_target(None, &office_document.target)),
        "{fixture}: officeDocument target should exist"
    );

    let part_names: Vec<String> = package
        .part_names()
        .into_iter()
        .map(str::to_string)
        .collect();
    for part_name in &part_names {
        let relationships = package
            .relationships(Some(part_name))
            .expect("part relationships should parse");
        for relationship in relationships {
            if relationship.target_mode == TargetMode::External {
                continue;
            }
            let resolved: String =
                resolve_relationship_target(Some(part_name), &relationship.target);
            assert!(
                package.has_part(&resolved),
                "{fixture}: relationship {id} of {part_name} targets missing part {resolved}",
                id = relationship.id,
            );
        }
    }
}

#[test]
fn content_types_cover_every_part() {
    for fixture in ROUND_TRIP_FIXTURES {
        let package = OpcPackage::from_bytes(fixture_bytes(fixture)).expect("fixture should load");
        for part_name in package.part_names() {
            if part_name == "[Content_Types].xml" {
                continue;
            }
            let content_type: Option<String> = package
                .content_type_of(part_name)
                .expect("content types should parse");
            assert!(
                content_type.is_some_and(|declared| !declared.is_empty()),
                "{fixture}: part {part_name} should have a declared content type"
            );
        }
    }
}

#[test]
fn from_bytes_rejects_bytes_that_are_not_a_zip_container() {
    let error: PackageError =
        OpcPackage::from_bytes(b"this is not a zip container".to_vec()).unwrap_err();
    assert!(
        matches!(error, PackageError::Container(_)),
        "expected a container error, got: {error}"
    );
}

#[test]
fn missing_parts_are_reported_by_name() {
    let mut package =
        OpcPackage::from_bytes(fixture_bytes("pptx/minimal.pptx")).expect("fixture should load");
    let read_error: PackageError = package.part_bytes("ppt/absent.xml").unwrap_err();
    assert!(
        matches!(&read_error, PackageError::MissingPart(name) if name == "ppt/absent.xml"),
        "expected MissingPart, got: {read_error}"
    );
    let write_error: PackageError = package
        .set_part_bytes("ppt/absent.xml", vec![1, 2, 3])
        .unwrap_err();
    assert!(
        matches!(&write_error, PackageError::MissingPart(name) if name == "ppt/absent.xml"),
        "expected MissingPart, got: {write_error}"
    );
}

#[test]
fn relationship_targets_resolve_relative_to_their_source_part() {
    assert_eq!(
        resolve_relationship_target(None, "ppt/presentation.xml"),
        "ppt/presentation.xml"
    );
    assert_eq!(
        resolve_relationship_target(Some("ppt/presentation.xml"), "slides/slide1.xml"),
        "ppt/slides/slide1.xml"
    );
    assert_eq!(
        resolve_relationship_target(
            Some("ppt/slideLayouts/slideLayout1.xml"),
            "../slideMasters/slideMaster1.xml"
        ),
        "ppt/slideMasters/slideMaster1.xml"
    );
    assert_eq!(
        resolve_relationship_target(Some("ppt/presentation.xml"), "/docProps/core.xml"),
        "docProps/core.xml"
    );
}
