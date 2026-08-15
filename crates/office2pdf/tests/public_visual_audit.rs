#![cfg(not(target_arch = "wasm32"))]

mod common;

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::process::Command;

use office2pdf::config::{ConvertOptions, Format};

#[derive(Debug, serde::Deserialize)]
struct VisualAuditManifest {
    format: String,
    cases: Vec<VisualAuditCase>,
}

#[derive(Debug, serde::Deserialize)]
struct VisualAuditCase {
    id: String,
    fixture: String,
    focus: String,
}

#[derive(serde::Serialize)]
struct VisualAuditReport<'a> {
    format: &'a str,
    dpi: u32,
    cases: Vec<VisualAuditResult>,
}

#[derive(serde::Serialize)]
struct VisualAuditResult {
    id: String,
    fixture: String,
    focus: String,
    status: String,
    ground_truth_pages: usize,
    output_pages: usize,
    ground_truth_text_length: usize,
    output_text_length: usize,
    ground_truth_images: Vec<String>,
    output_images: Vec<String>,
}

fn project_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn load_visual_audit_manifest(path: &Path) -> VisualAuditManifest {
    let data = std::fs::read_to_string(path)
        .unwrap_or_else(|error| panic!("read visual audit manifest {}: {error}", path.display()));
    serde_json::from_str(&data)
        .unwrap_or_else(|error| panic!("parse visual audit manifest {}: {error}", path.display()))
}

fn render_pdf_to_jpegs(pdf_path: &Path, output_dir: &Path, prefix: &str, dpi: u32) -> Vec<PathBuf> {
    std::fs::create_dir_all(output_dir).expect("create visual audit output directory");
    let output_prefix = output_dir.join(prefix);
    let status = Command::new("pdftoppm")
        .args([
            "-jpeg",
            "-jpegopt",
            "quality=86,progressive=y",
            "-r",
            &dpi.to_string(),
        ])
        .arg(pdf_path)
        .arg(&output_prefix)
        .status()
        .expect("run pdftoppm");
    assert!(
        status.success(),
        "pdftoppm failed for {}",
        pdf_path.display()
    );

    let mut images: Vec<PathBuf> = std::fs::read_dir(output_dir)
        .expect("read visual audit output directory")
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| {
            path.extension().is_some_and(|extension| extension == "jpg")
                && path
                    .file_name()
                    .is_some_and(|name| name.to_string_lossy().starts_with(prefix))
        })
        .collect();
    images.sort();
    images
}

fn relative_image_paths(images: &[PathBuf], report_dir: &Path) -> Vec<String> {
    images
        .iter()
        .map(|path| {
            path.strip_prefix(report_dir)
                .unwrap_or(path)
                .to_string_lossy()
                .replace('\\', "/")
        })
        .collect()
}

/// How a format's ground truth is produced: which AppleScript exports it, and
/// which sandboxed Microsoft app that script drives.
struct NativeExporter {
    /// File name under `scripts/macos/`.
    script: &'static str,
    /// Bundle identifier, i.e. the app's container directory name.
    bundle_id: &'static str,
    /// How the app is named in a failure message.
    app_name: &'static str,
}

fn native_exporter(format: Format) -> NativeExporter {
    match format {
        Format::Docx => NativeExporter {
            script: "export_word_pdfs.applescript",
            bundle_id: "com.microsoft.Word",
            app_name: "Word",
        },
        Format::Pptx => NativeExporter {
            script: "export_powerpoint_pdfs.applescript",
            bundle_id: "com.microsoft.Powerpoint",
            app_name: "PowerPoint",
        },
        Format::Xlsx => NativeExporter {
            script: "export_excel_pdfs.applescript",
            bundle_id: "com.microsoft.Excel",
            app_name: "Excel",
        },
    }
}

/// Directory a GT export stages the fixtures and PDFs it hands the native app.
///
/// The Microsoft apps are sandboxed: a fixture opened from, or a PDF saved to,
/// anywhere outside the app's own container costs a per-file "Grant Access"
/// dialog, and an unattended export stalls on the first one (#1051, #1082).
/// Reaching into a *different* app's container prompts just the same, so each
/// format stages in the container of the app that opens it.
fn office_stage_dir(containers_root: &Path, format: Format) -> PathBuf {
    containers_root
        .join(native_exporter(format).bundle_id)
        .join("Data")
        .join("visual-audit")
}

fn home_containers_root() -> PathBuf {
    PathBuf::from(std::env::var_os("HOME").expect("HOME is set"))
        .join("Library")
        .join("Containers")
}

/// Copy every case's fixture into `stage`; return the exporter's `(id, path)`
/// arguments.
///
/// Each copy is named after the case id, which `assert_manifest_cases` pins as
/// unique — two cases pointing at one fixture would otherwise share a staged
/// file and race each other.
fn stage_fixtures(
    stage: &Path,
    fixtures_dir: &Path,
    cases: &[VisualAuditCase],
) -> Vec<(String, PathBuf)> {
    let input_dir = stage.join("input");
    std::fs::create_dir_all(&input_dir).expect("create staged fixture directory");
    cases
        .iter()
        .map(|case| {
            let source = fixtures_dir.join(&case.fixture);
            let staged = input_dir.join(match source.extension() {
                Some(extension) => format!("{}.{}", case.id, extension.to_string_lossy()),
                None => case.id.clone(),
            });
            std::fs::copy(&source, &staged).unwrap_or_else(|error| {
                panic!("stage visual audit fixture {}: {error}", source.display())
            });
            (case.id.clone(), staged)
        })
        .collect()
}

/// Move every PDF the sandboxed app wrote into `destination`; return the count.
///
/// A plain rename would fail when `VISUAL_AUDIT_DIR` points at another volume,
/// so the PDFs are copied and then dropped from the container.
fn collect_exported_pdfs(staged_pdf_dir: &Path, destination: &Path) -> usize {
    std::fs::create_dir_all(destination).expect("create ground truth directory");
    let mut exported: usize = 0;
    for entry in std::fs::read_dir(staged_pdf_dir).expect("read staged PDF directory") {
        let path = entry.expect("read staged PDF entry").path();
        if !path
            .extension()
            .is_some_and(|extension| extension.eq_ignore_ascii_case("pdf"))
        {
            continue;
        }
        let target = destination.join(path.file_name().expect("staged PDF file name"));
        std::fs::copy(&path, &target)
            .unwrap_or_else(|error| panic!("copy GT PDF out of the container: {error}"));
        std::fs::remove_file(&path)
            .unwrap_or_else(|error| panic!("drop the container's GT PDF copy: {error}"));
        exported += 1;
    }
    exported
}

/// Export every manifest case with the native app that owns `format`.
///
/// The app only ever sees paths inside its own container; the PDFs are moved
/// into `ground_truth_dir` once it has quit.
fn export_ground_truth(
    manifest: &VisualAuditManifest,
    format: Format,
    fixtures_dir: &Path,
    ground_truth_dir: &Path,
) {
    let exporter = native_exporter(format);
    assert_eq!(
        std::env::consts::OS,
        "macos",
        "Microsoft {} GT export is only available on macOS",
        exporter.app_name
    );
    std::fs::create_dir_all(ground_truth_dir).expect("create GT directory");

    // A stage left behind by an aborted run would hand the audit its stale
    // PDFs, so start from nothing.
    let stage = office_stage_dir(&home_containers_root(), format);
    if stage.exists() {
        std::fs::remove_dir_all(&stage).expect("clean stale sandbox stage");
    }
    let staged_pdf_dir = stage.join("pdf");
    std::fs::create_dir_all(&staged_pdf_dir).expect("create staged PDF directory");

    let mut command = Command::new("osascript");
    command
        .arg(project_root().join("scripts/macos").join(exporter.script))
        .arg(&staged_pdf_dir);
    for (id, staged_fixture) in stage_fixtures(&stage, fixtures_dir, &manifest.cases) {
        command.arg(id).arg(staged_fixture);
    }
    let output = command.output().expect("run native GT exporter");

    // Salvage whatever did export before reporting a partial batch's failure.
    let exported = collect_exported_pdfs(&staged_pdf_dir, ground_truth_dir);
    std::fs::remove_dir_all(&stage).expect("remove the sandbox stage");
    assert!(
        output.status.success(),
        "{} GT export failed:\nstdout: {}\nstderr: {}",
        exporter.app_name,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        exported > 0,
        "{} exported no PDFs at all",
        exporter.app_name
    );
}

fn generate_powerpoint_ground_truth(
    manifest: &VisualAuditManifest,
    fixtures_dir: &Path,
    ground_truth_dir: &Path,
) {
    export_ground_truth(manifest, Format::Pptx, fixtures_dir, ground_truth_dir);
}

fn generate_excel_ground_truth(
    manifest: &VisualAuditManifest,
    fixtures_dir: &Path,
    ground_truth_dir: &Path,
) {
    export_ground_truth(manifest, Format::Xlsx, fixtures_dir, ground_truth_dir);

    for case in &manifest.cases {
        let prefix = format!("{}-sheet-", case.id);
        let mut sheet_pdfs: Vec<PathBuf> = std::fs::read_dir(ground_truth_dir)
            .expect("read Excel GT directory")
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .filter(|path| {
                path.file_name().is_some_and(|name| {
                    let name = name.to_string_lossy();
                    name.starts_with(&prefix) && name.ends_with(".pdf")
                })
            })
            .collect();
        sheet_pdfs.sort();
        assert!(
            !sheet_pdfs.is_empty(),
            "Excel exported no visible worksheets for {}",
            case.id
        );

        let combined_pdf = ground_truth_dir.join(format!("{}.pdf", case.id));
        if sheet_pdfs.len() == 1 {
            std::fs::copy(&sheet_pdfs[0], &combined_pdf).expect("copy single-sheet Excel GT");
        } else {
            let status = Command::new("pdfunite")
                .args(&sheet_pdfs)
                .arg(&combined_pdf)
                .status()
                .expect("run pdfunite for Excel GT");
            assert!(
                status.success(),
                "pdfunite failed while combining Excel GT for {}",
                case.id
            );
        }
        for sheet_pdf in sheet_pdfs {
            std::fs::remove_file(sheet_pdf).expect("remove intermediate Excel sheet PDF");
        }
    }
}

fn assert_manifest_cases(manifest: &VisualAuditManifest, extension: &str) {
    let fixtures_dir = project_root().join("tests/fixtures");
    let mut ids: BTreeSet<&str> = BTreeSet::new();
    for case in &manifest.cases {
        assert!(
            ids.insert(&case.id),
            "duplicate {extension} visual audit id: {}",
            case.id
        );
        assert!(
            case.id
                .chars()
                .all(|character| character.is_ascii_lowercase()
                    || character.is_ascii_digit()
                    || character == '-'),
            "visual audit id must be lowercase ASCII kebab-case: {}",
            case.id
        );
        assert!(
            fixtures_dir.join(&case.fixture).is_file(),
            "missing {extension} visual audit fixture: {}",
            case.fixture
        );
    }
}

#[test]
fn pptx_visual_audit_manifest_covers_priority_areas() {
    let manifest_path = project_root().join("tests/visual_audits/pptx.json");
    let manifest = load_visual_audit_manifest(&manifest_path);

    assert_eq!(manifest.format, "pptx");
    assert!(manifest.cases.len() >= 8);
    assert_manifest_cases(&manifest, "PPTX");
    for focus in [
        "group transforms",
        "image crop",
        "master and layout",
        "theme table",
        "image transparency",
        "SmartArt",
        "chart",
        "text rotation",
    ] {
        assert!(
            manifest.cases.iter().any(|case| case.focus == focus),
            "missing PPTX visual audit focus: {focus}"
        );
    }
}

#[test]
fn xlsx_visual_audit_manifest_covers_priority_areas() {
    let manifest_path = project_root().join("tests/visual_audits/xlsx.json");
    let manifest = load_visual_audit_manifest(&manifest_path);

    assert_eq!(manifest.format, "xlsx");
    assert!(manifest.cases.len() >= 10);
    assert_manifest_cases(&manifest, "XLSX");
    for focus in [
        "page setup",
        "headers and footers",
        "repeating titles",
        "row and column sizing",
        "right-to-left",
        "number formats",
        "conditional formatting",
        "drawings",
        "charts",
        "text boxes",
    ] {
        assert!(
            manifest.cases.iter().any(|case| case.focus == focus),
            "missing XLSX visual audit focus: {focus}"
        );
    }
}

fn run_visual_audit(
    manifest_name: &str,
    format: Format,
    generate_ground_truth: fn(&VisualAuditManifest, &Path, &Path),
) {
    assert!(
        common::is_pdftoppm_available(),
        "pdftoppm (poppler-utils) is required"
    );
    assert!(
        common::is_pdftotext_available(),
        "pdftotext (poppler-utils) is required"
    );

    let dpi: u32 = std::env::var("VISUAL_DPI")
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(150);
    let manifest = load_visual_audit_manifest(
        &project_root().join(format!("tests/visual_audits/{manifest_name}.json")),
    );
    let fixtures_dir = project_root().join("tests/fixtures");
    let report_dir = std::env::var_os("VISUAL_AUDIT_DIR").map_or_else(
        || {
            project_root()
                .join("target/visual-audit")
                .join(manifest_name)
        },
        PathBuf::from,
    );
    let ground_truth_dir = report_dir.join("ground-truth-pdf");
    std::fs::create_dir_all(&report_dir).expect("create visual audit report directory");

    if std::env::var("GENERATE_MICROSOFT_GT").as_deref() == Ok("1") {
        generate_ground_truth(&manifest, &fixtures_dir, &ground_truth_dir);
    }

    let mut results: Vec<VisualAuditResult> = Vec::new();
    for case in &manifest.cases {
        let fixture_path = fixtures_dir.join(&case.fixture);
        let ground_truth_pdf = ground_truth_dir.join(format!("{}.pdf", case.id));
        assert!(
            ground_truth_pdf.is_file(),
            "missing Microsoft GT PDF for {}: run with GENERATE_MICROSOFT_GT=1",
            case.id
        );

        let case_dir = report_dir.join(&case.id);
        if case_dir.exists() {
            std::fs::remove_dir_all(&case_dir).expect("clean visual audit case directory");
        }
        std::fs::create_dir_all(&case_dir).expect("create visual audit case directory");

        let input = std::fs::read(&fixture_path).expect("read visual audit fixture");
        let conversion = office2pdf::convert_bytes(&input, format, &ConvertOptions::default());
        let Ok(conversion) = conversion else {
            results.push(VisualAuditResult {
                id: case.id.clone(),
                fixture: case.fixture.clone(),
                focus: case.focus.clone(),
                status: format!("conversion_error: {}", conversion.unwrap_err()),
                ground_truth_pages: 0,
                output_pages: 0,
                ground_truth_text_length: 0,
                output_text_length: 0,
                ground_truth_images: Vec::new(),
                output_images: Vec::new(),
            });
            continue;
        };

        let output_pdf = case_dir.join("office2pdf.pdf");
        std::fs::write(&output_pdf, conversion.pdf).expect("write office2pdf audit PDF");
        let ground_truth_images = render_pdf_to_jpegs(&ground_truth_pdf, &case_dir, "gt", dpi);
        let output_images = render_pdf_to_jpegs(&output_pdf, &case_dir, "output", dpi);
        let ground_truth_text = common::extract_text_from_pdf_file(&ground_truth_pdf);
        let output_text = common::extract_text_from_pdf_file(&output_pdf);

        results.push(VisualAuditResult {
            id: case.id.clone(),
            fixture: case.fixture.clone(),
            focus: case.focus.clone(),
            status: "ok".to_string(),
            ground_truth_pages: ground_truth_images.len(),
            output_pages: output_images.len(),
            ground_truth_text_length: ground_truth_text.len(),
            output_text_length: output_text.len(),
            ground_truth_images: relative_image_paths(&ground_truth_images, &report_dir),
            output_images: relative_image_paths(&output_images, &report_dir),
        });
    }

    let report = VisualAuditReport {
        format: &manifest.format,
        dpi,
        cases: results,
    };
    let report_json = serde_json::to_string_pretty(&report).expect("serialize visual audit report");
    std::fs::write(report_dir.join("report.json"), format!("{report_json}\n"))
        .expect("write visual audit report");
    println!(
        "{} visual audit report: {}",
        manifest.format.to_uppercase(),
        report_dir.display()
    );
}

/// A scratch directory for the staging tests below, unique per call.
///
/// Tests run in parallel threads of one process, so a shared name would let
/// two of them delete each other's files mid-assertion.
fn scratch_dir(label: &str) -> PathBuf {
    static COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let path = std::env::temp_dir().join(format!(
        "office2pdf-visual-audit-{label}-{}-{}",
        std::process::id(),
        COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    ));
    if path.exists() {
        std::fs::remove_dir_all(&path).expect("clean scratch directory");
    }
    std::fs::create_dir_all(&path).expect("create scratch directory");
    path
}

#[test]
fn each_format_is_exported_by_its_own_sandboxed_app() {
    // Reaching into another app's container prompts exactly like reaching
    // outside the sandbox, so the container has to match the exporter.
    let powerpoint = native_exporter(Format::Pptx);
    assert_eq!(powerpoint.bundle_id, "com.microsoft.Powerpoint");
    assert_eq!(powerpoint.script, "export_powerpoint_pdfs.applescript");

    let excel = native_exporter(Format::Xlsx);
    assert_eq!(excel.bundle_id, "com.microsoft.Excel");
    assert_eq!(excel.script, "export_excel_pdfs.applescript");

    assert!(
        project_root()
            .join("scripts/macos")
            .join(powerpoint.script)
            .is_file()
    );
    assert!(
        project_root()
            .join("scripts/macos")
            .join(excel.script)
            .is_file()
    );
}

#[test]
fn the_ground_truth_stage_sits_inside_the_driven_apps_container() {
    let containers = Path::new("/home/tester/Library/Containers");
    assert!(
        office_stage_dir(containers, Format::Pptx)
            .starts_with(containers.join("com.microsoft.Powerpoint").join("Data")),
        "PowerPoint GT must stage inside PowerPoint's container"
    );
    assert_ne!(
        office_stage_dir(containers, Format::Pptx),
        office_stage_dir(containers, Format::Xlsx)
    );
}

#[test]
fn staged_fixtures_are_container_local_copies_named_by_case_id() {
    let stage = scratch_dir("stage");
    let fixtures = scratch_dir("fixtures");
    std::fs::write(fixtures.join("bar-chart.pptx"), b"PK fixture").expect("write fixture");
    let cases = vec![
        VisualAuditCase {
            id: "chart".to_string(),
            fixture: "bar-chart.pptx".to_string(),
            focus: "chart".to_string(),
        },
        VisualAuditCase {
            id: "chart-again".to_string(),
            fixture: "bar-chart.pptx".to_string(),
            focus: "chart".to_string(),
        },
    ];

    let jobs = stage_fixtures(&stage, &fixtures, &cases);

    assert_eq!(jobs.len(), 2);
    for (index, (id, staged)) in jobs.iter().enumerate() {
        assert_eq!(id, &cases[index].id);
        assert!(
            staged.starts_with(&stage),
            "fixture staged outside the container: {}",
            staged.display()
        );
        assert_eq!(
            staged.file_name().and_then(|name| name.to_str()),
            Some(format!("{id}.pptx").as_str()),
            "two cases sharing one fixture must not collide"
        );
        assert_eq!(
            std::fs::read(staged).expect("read staged fixture"),
            b"PK fixture"
        );
    }

    std::fs::remove_dir_all(&stage).ok();
    std::fs::remove_dir_all(&fixtures).ok();
}

#[test]
fn exported_pdfs_are_moved_out_of_the_container() {
    let staged_pdf_dir = scratch_dir("exported");
    let destination = scratch_dir("ground-truth").join("nested");
    std::fs::write(staged_pdf_dir.join("chart.pdf"), b"%PDF chart").expect("write staged PDF");
    std::fs::write(staged_pdf_dir.join("grid-sheet-01.pdf"), b"%PDF sheet")
        .expect("write staged sheet PDF");
    std::fs::write(staged_pdf_dir.join("chart.pptx"), b"PK copy").expect("write staged package");

    let moved = collect_exported_pdfs(&staged_pdf_dir, &destination);

    assert_eq!(moved, 2);
    assert_eq!(
        std::fs::read(destination.join("chart.pdf")).expect("read moved PDF"),
        b"%PDF chart"
    );
    assert!(destination.join("grid-sheet-01.pdf").is_file());
    assert!(
        !staged_pdf_dir.join("chart.pdf").exists(),
        "the container copy must not linger"
    );
    assert!(
        staged_pdf_dir.join("chart.pptx").is_file(),
        "only PDFs are the export's product"
    );

    std::fs::remove_dir_all(&staged_pdf_dir).ok();
    std::fs::remove_dir_all(destination.parent().expect("scratch parent")).ok();
}

#[test]
#[ignore]
fn test_public_pptx_visual_audit() {
    run_visual_audit("pptx", Format::Pptx, generate_powerpoint_ground_truth);
}

#[test]
#[ignore]
fn test_public_xlsx_visual_audit() {
    run_visual_audit("xlsx", Format::Xlsx, generate_excel_ground_truth);
}
