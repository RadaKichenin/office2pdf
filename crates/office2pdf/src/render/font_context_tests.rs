#![cfg(not(target_arch = "wasm32"))] // native-only unit tests (filesystem, system fonts)
use super::*;
use std::fs;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

struct TempDir {
    path: PathBuf,
}

impl TempDir {
    fn new(prefix: &str) -> Self {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time should be valid")
            .as_nanos();
        let path = std::env::temp_dir().join(format!("{prefix}-{unique}"));
        fs::create_dir_all(&path).unwrap();
        Self { path }
    }

    fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

#[test]
fn test_discover_macos_office_font_paths_ignores_mutable_per_user_caches() {
    let temp = TempDir::new("office-font-discovery-order");
    let apps = temp.path().join("Applications");
    let home = temp.path().join("home");

    fs::create_dir_all(apps.join("Microsoft PowerPoint.app/Contents/Resources/DFonts")).unwrap();
    fs::create_dir_all(apps.join("Microsoft Word.app/Contents/Resources/DFonts")).unwrap();
    fs::create_dir_all(
        home.join("Library/Group Containers/UBF8T346G9.Office/FontCache/4/CloudFonts"),
    )
    .unwrap();
    fs::create_dir_all(
        home.join("Library/Group Containers/UBF8T346G9.Office/FontCache/4/PreviewFont"),
    )
    .unwrap();

    let discovered = discover_macos_office_font_paths_from(&[apps]);
    let expected = vec![
        fs::canonicalize(
            temp.path()
                .join("Applications/Microsoft PowerPoint.app/Contents/Resources/DFonts"),
        )
        .unwrap(),
        fs::canonicalize(
            temp.path()
                .join("Applications/Microsoft Word.app/Contents/Resources/DFonts"),
        )
        .unwrap(),
    ];

    assert_eq!(discovered, expected);
}

#[test]
fn test_discover_macos_office_font_paths_does_not_use_a_cache_only_home() {
    let temp = TempDir::new("office-font-discovery-version");
    let apps = temp.path().join("Applications");
    let home = temp.path().join("home");

    fs::create_dir_all(
        home.join("Library/Group Containers/UBF8T346G9.Office/FontCache/4/CloudFonts"),
    )
    .unwrap();
    fs::create_dir_all(
        home.join("Library/Group Containers/UBF8T346G9.Office/FontCache/7/CloudFonts"),
    )
    .unwrap();
    fs::create_dir_all(
        home.join("Library/Group Containers/UBF8T346G9.Office/FontCache/7/PreviewFont"),
    )
    .unwrap();

    let discovered = discover_macos_office_font_paths_from(&[apps]);
    assert!(
        discovered.is_empty(),
        "application-private caches must not make an unembedded font appear installed: \
         {discovered:#?}"
    );
}

#[test]
fn test_merge_prioritized_paths_keeps_first_occurrence() {
    let temp = TempDir::new("office-font-merge");
    let first = temp.path().join("first");
    let second = temp.path().join("second");
    fs::create_dir_all(&first).unwrap();
    fs::create_dir_all(&second).unwrap();

    let merged = merge_prioritized_paths(
        &canonicalize_existing_dirs(vec![first.clone(), second.clone()]),
        &canonicalize_existing_dirs(vec![second, first]),
    );

    assert_eq!(merged.len(), 2);
    assert!(merged[0].ends_with("first"));
    assert!(merged[1].ends_with("second"));
}

#[test]
fn test_canonicalize_existing_dirs_skips_missing_paths() {
    let temp = TempDir::new("office-font-canonicalize");
    let existing = temp.path().join("existing");
    fs::create_dir_all(&existing).unwrap();
    let missing = temp.path().join("missing");

    let canonicalized =
        canonicalize_existing_dirs(vec![existing.clone(), missing, existing.clone()]);

    assert_eq!(canonicalized.len(), 1);
    assert_eq!(canonicalized[0], fs::canonicalize(existing).unwrap());
}

#[test]
fn test_in_memory_document_font_context_indexes_family_and_cjk_coverage() {
    let docx = include_bytes!("../../../../tests/fixtures/docx/wasm_embedded_cjk.docx");
    let embedded = crate::parser::embedded_fonts::extract_embedded_font_data(
        docx,
        crate::config::Format::Docx,
    )
    .expect("fixture should expose its embedded font bytes");
    let fonts: Vec<typst::text::Font> = embedded
        .font_bytes()
        .flat_map(|data| typst::text::Font::iter(typst::foundations::Bytes::new(data.to_vec())))
        .collect();

    let context = resolve_font_search_context_from_fonts(&fonts);

    assert!(context.has_family("Noto Sans SC"));
    assert!(context.covers_script("Noto Sans SC", TextScript::Chinese));
    assert_eq!(context.family_source_rank("Noto Sans SC"), 1);
    assert!(context.search_paths().is_empty());
}
