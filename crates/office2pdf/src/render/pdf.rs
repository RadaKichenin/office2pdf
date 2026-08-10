use std::collections::HashMap;
#[cfg(not(target_arch = "wasm32"))]
use std::path::PathBuf;
#[cfg(not(target_arch = "wasm32"))]
use std::sync::Mutex;
use std::sync::{Arc, OnceLock};
// `SystemTime::now()` panics on wasm32-unknown-unknown; web-time shims it there
// and re-exports std elsewhere. Mirrors the `Instant` handling in lib_pipeline.
#[cfg(not(target_arch = "wasm32"))]
use std::time::{SystemTime, UNIX_EPOCH};
#[cfg(target_arch = "wasm32")]
use web_time::{SystemTime, UNIX_EPOCH};

use typst::diag::FileResult;
use typst::foundations::{Bytes, Datetime};
use typst::syntax::{FileId, Source, VirtualPath};
use typst::text::Font;
use typst::utils::LazyHash;
use typst::{Library, LibraryExt, World};
use typst_kit::fonts::FontSearcher;

use crate::config::PdfStandard;
use crate::error::ConvertError;

use super::typst_gen::ImageAsset;

/// Cached font data (book + font slots). Font discovery is expensive because
/// it scans the filesystem; the result doesn't change during the process
/// lifetime, so we cache it in a global `OnceLock`.
struct CachedFontData {
    book: LazyHash<typst::text::FontBook>,
    fonts: Vec<typst_kit::fonts::FontSlot>,
}

/// Cached system fonts (with system font search). Used when no custom
/// font paths are provided, which is the common case.
#[cfg(not(target_arch = "wasm32"))]
static SYSTEM_FONTS: OnceLock<CachedFontData> = OnceLock::new();

/// Cached font data for resolved extra font path sets.
#[cfg(not(target_arch = "wasm32"))]
static EXTRA_FONT_PATHS_CACHE: OnceLock<Mutex<HashMap<Vec<PathBuf>, Arc<CachedFontData>>>> =
    OnceLock::new();

/// Cached embedded-only fonts (no system font search). Used on WASM
/// or when system fonts are not needed.
static EMBEDDED_FONTS: OnceLock<CachedFontData> = OnceLock::new();

/// Get or initialize cached system fonts (with system font discovery).
#[cfg(not(target_arch = "wasm32"))]
fn get_system_fonts() -> &'static CachedFontData {
    SYSTEM_FONTS.get_or_init(|| {
        let mut searcher = FontSearcher::new();
        searcher.include_system_fonts(true);
        let font_data = searcher.search();
        CachedFontData {
            book: LazyHash::new(font_data.book),
            fonts: font_data.fonts,
        }
    })
}

/// Get or initialize cached fonts for a resolved extra font path set.
#[cfg(not(target_arch = "wasm32"))]
fn get_fonts_for_extra_paths(font_paths: &[PathBuf]) -> Arc<CachedFontData> {
    let cache = EXTRA_FONT_PATHS_CACHE.get_or_init(|| Mutex::new(HashMap::new()));
    {
        let cache_guard = cache
            .lock()
            .expect("font cache mutex should not be poisoned");
        if let Some(cached) = cache_guard.get(font_paths) {
            return Arc::clone(cached);
        }
    }

    let mut searcher = FontSearcher::new();
    searcher.include_system_fonts(true);
    let font_data = searcher.search_with(font_paths.iter().map(|path| path.as_path()));
    let cached = Arc::new(CachedFontData {
        book: LazyHash::new(font_data.book),
        fonts: font_data.fonts,
    });

    let mut cache_guard = cache
        .lock()
        .expect("font cache mutex should not be poisoned");
    let entry = cache_guard
        .entry(font_paths.to_vec())
        .or_insert_with(|| Arc::clone(&cached));
    Arc::clone(entry)
}

/// Get or initialize cached embedded-only fonts.
fn get_embedded_fonts() -> &'static CachedFontData {
    EMBEDDED_FONTS.get_or_init(|| {
        let mut searcher = FontSearcher::new();
        searcher.include_system_fonts(false);
        let font_data = searcher.search();
        CachedFontData {
            book: LazyHash::new(font_data.book),
            fonts: font_data.fonts,
        }
    })
}

/// Compile Typst markup to PDF bytes.
///
/// When `pdf_standard` is `Some`, the output PDF will conform to the
/// specified standard (e.g., PDF/A-2b for archival).
/// When `font_paths` is non-empty, those directories are searched for
/// additional fonts (highest priority).
///
/// On native targets, system fonts are discovered automatically. On WASM,
/// only embedded fonts are used and `font_paths` is ignored.
///
/// # PDF output size optimization
///
/// typst-pdf (via krilla) applies the following optimizations by default:
///
/// - **Content stream compression**: All content streams use FLATE (deflate)
///   compression (`compress_content_streams: true`). Typical reduction: 60-80%.
/// - **Font subsetting**: Only glyphs actually used in the document are embedded
///   (via the `subsetter` crate). Typical reduction: 70-90% of font data.
/// - **Image pass-through**: Embedded images (PNG, JPEG) are included as-is
///   without re-encoding, preserving their original compression.
///
/// Expected output sizes:
/// - Empty page: ~10-30 KB (font data + PDF structure overhead)
/// - 10-page text-only document: ~30-60 KB
/// - Document with images: baseline + proportional to image data size
#[cfg(not(target_arch = "wasm32"))]
pub fn compile_to_pdf(
    typst_source: &str,
    images: &[ImageAsset],
    pdf_standard: Option<PdfStandard>,
    font_paths: &[PathBuf],
    tagged: bool,
    pdf_ua: bool,
) -> Result<Vec<u8>, ConvertError> {
    let world = MinimalWorld::new(typst_source, images, font_paths);
    compile_to_pdf_inner(&world, pdf_standard, tagged, pdf_ua)
}

/// Compile Typst markup to PDF bytes (WASM target).
///
/// Uses embedded fonts only. System font paths are not supported on WASM.
#[cfg(target_arch = "wasm32")]
pub fn compile_to_pdf(
    typst_source: &str,
    images: &[ImageAsset],
    pdf_standard: Option<PdfStandard>,
    _font_paths: &[std::path::PathBuf],
    tagged: bool,
    pdf_ua: bool,
) -> Result<Vec<u8>, ConvertError> {
    let world = MinimalWorld::new_embedded_only(typst_source, images);
    compile_to_pdf_inner(&world, pdf_standard, tagged, pdf_ua)
}

fn compile_to_pdf_inner(
    world: &MinimalWorld,
    pdf_standard: Option<PdfStandard>,
    tagged: bool,
    pdf_ua: bool,
) -> Result<Vec<u8>, ConvertError> {
    let warned = typst::compile::<typst::layout::PagedDocument>(world);
    let document = warned.output.map_err(|errors| {
        let messages: Vec<String> = errors.iter().map(|e| e.message.to_string()).collect();
        ConvertError::Render(format!("Typst compilation failed: {}", messages.join("; ")))
    })?;

    // Build PDF standards list
    let mut pdf_standards = Vec::new();
    if let Some(PdfStandard::PdfA2b) = pdf_standard {
        pdf_standards.push(typst_pdf::PdfStandard::A_2b);
    }
    if pdf_ua {
        pdf_standards.push(typst_pdf::PdfStandard::Ua_1);
    }
    let standards = if pdf_standards.is_empty() {
        typst_pdf::PdfStandards::default()
    } else {
        typst_pdf::PdfStandards::new(&pdf_standards)
            .map_err(|e| ConvertError::Render(format!("PDF standard configuration error: {e}")))?
    };

    // PDF/A and PDF/UA require a document creation timestamp
    let needs_timestamp = pdf_standard.is_some() || pdf_ua;
    let timestamp = if needs_timestamp {
        Some(typst_pdf::Timestamp::new_utc(current_utc_datetime()))
    } else {
        None
    };

    // Enable tagging when explicitly requested or when PDF/UA requires it
    let enable_tagged = tagged || pdf_ua;

    let options = typst_pdf::PdfOptions {
        standards,
        timestamp,
        tagged: enable_tagged,
        ..Default::default()
    };
    typst_pdf::pdf(&document, &options).map_err(|errors| {
        let messages: Vec<String> = errors.iter().map(|e| e.message.to_string()).collect();
        ConvertError::Render(format!("PDF export failed: {}", messages.join("; ")))
    })
}

/// One shaped text run as the layout engine actually placed it.
#[cfg(all(test, not(target_arch = "wasm32")))]
#[derive(Debug, Clone)]
pub(crate) struct PlacedTextRun {
    /// Distance from the page's top edge down to the run's baseline, in points.
    pub baseline_pt: f64,
    /// Distance from the page's left edge to the run's origin, in points.
    pub left_pt: f64,
    /// The family the run was actually shaped with, which is not always the one
    /// the source asked for.
    pub family: String,
    pub text: String,
}

/// Every text run the compiled document places on `page_index`, in layout order.
///
/// The emitted source cannot show where a line ends up: `top-edge`, `place` and
/// `measure` are all resolved by the layout engine, and issue #629's first
/// attempt passed a source-string assertion while moving every wrapped line of
/// the paragraph it touched. Tests for placement therefore compile the source
/// and read the frames.
///
/// Group transforms are followed by their translation only; nothing in a header
/// or footer story rotates or scales, and a run inside such a group would be
/// reported at the wrong place rather than silently skipped.
#[cfg(all(test, not(target_arch = "wasm32")))]
pub(crate) fn compiled_text_runs(
    typst_source: &str,
    page_index: usize,
) -> Result<Vec<PlacedTextRun>, ConvertError> {
    use typst::layout::{Frame, FrameItem, Point};

    fn collect(frame: &Frame, origin: Point, out: &mut Vec<PlacedTextRun>) {
        for (position, item) in frame.items() {
            let at: Point = origin + *position;
            match item {
                FrameItem::Group(group) => {
                    let shift = Point::new(group.transform.tx, group.transform.ty);
                    collect(&group.frame, at + shift, out);
                }
                FrameItem::Text(text) => out.push(PlacedTextRun {
                    baseline_pt: at.y.to_pt(),
                    left_pt: at.x.to_pt(),
                    family: text.font.info().family.clone(),
                    text: text.text.to_string(),
                }),
                _ => {}
            }
        }
    }

    // The same font set the conversion pipeline compiles with, so a probe sees
    // the faces `font_hhea_ascender_em` measured rather than a substitute.
    // Resolving it rescans the font directories, which dominates a probe's
    // runtime, so the paths are resolved once for the whole test process.
    static PROBE_FONT_PATHS: OnceLock<Vec<PathBuf>> = OnceLock::new();
    let font_paths: &Vec<PathBuf> = PROBE_FONT_PATHS.get_or_init(|| {
        super::font_context::resolve_font_search_context(&[])
            .search_paths()
            .to_vec()
    });
    let world = MinimalWorld::new(typst_source, &[], font_paths);
    let warned = typst::compile::<typst::layout::PagedDocument>(&world);
    let document = warned.output.map_err(|errors| {
        let messages: Vec<String> = errors.iter().map(|e| e.message.to_string()).collect();
        ConvertError::Render(format!("Typst compilation failed: {}", messages.join("; ")))
    })?;
    let page = document.pages.get(page_index).ok_or_else(|| {
        ConvertError::Render(format!(
            "page {page_index} is past the document's {} pages",
            document.pages.len()
        ))
    })?;
    let mut runs: Vec<PlacedTextRun> = Vec::new();
    collect(&page.frame, Point::zero(), &mut runs);
    Ok(runs)
}

/// Convert the current system time to a Typst `Datetime` in UTC.
///
/// Uses `std::time::SystemTime` to avoid an external chrono dependency.
/// The civil date is computed from the Unix timestamp using Howard Hinnant's
/// algorithm (<http://howardhinnant.github.io/date_algorithms.html>).
fn current_utc_datetime() -> Datetime {
    let duration = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    let secs = duration.as_secs() as i64;

    // Split into days since epoch and time-of-day
    let days = secs.div_euclid(86400);
    let rem = secs.rem_euclid(86400);
    let hours = (rem / 3600) as u8;
    let minutes = ((rem % 3600) / 60) as u8;
    let seconds = (rem % 60) as u8;

    // Civil date from day count since Unix epoch (1970-01-01)
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097) as u32; // day of era [0, 146096]
    let yoe = (doe - doe / 1461 + doe / 36524 - doe / 146096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100); // day of year [0, 365]
    let mp = (5 * doy + 2) / 153; // [0, 11]
    let d = (doy - (153 * mp + 2) / 5 + 1) as u8;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u8;
    let y = if m <= 2 { y + 1 } else { y } as i32;

    Datetime::from_ymd_hms(y, m, d, hours, minutes, seconds)
        .expect("valid date derived from SystemTime")
}

/// Font data source: either a static reference to cached fonts or owned
/// data for custom font path searches.
enum FontSource {
    /// Reference to globally cached font data (common case).
    Cached(&'static CachedFontData),
    /// Shared cached font data for resolved extra font paths.
    /// Only constructed on native (extra font paths need filesystem access).
    #[cfg_attr(target_arch = "wasm32", allow(dead_code))]
    Shared(Arc<CachedFontData>),
}

impl FontSource {
    fn book(&self) -> &LazyHash<typst::text::FontBook> {
        match self {
            Self::Cached(d) => &d.book,
            Self::Shared(d) => &d.book,
        }
    }

    fn fonts(&self) -> &[typst_kit::fonts::FontSlot] {
        match self {
            Self::Cached(d) => &d.fonts,
            Self::Shared(d) => &d.fonts,
        }
    }
}

/// Minimal World implementation providing Typst compiler with source, fonts, and images.
struct MinimalWorld {
    library: LazyHash<Library>,
    font_source: FontSource,
    source: Source,
    images: HashMap<String, Bytes>,
}

impl MinimalWorld {
    /// Create a new `MinimalWorld` with system fonts and optional custom font paths.
    ///
    /// When `font_paths` is empty (the common case), system fonts are loaded from
    /// a process-wide cache, avoiding expensive filesystem scanning on repeated calls.
    /// Resolved extra font path sets are also cached by path list.
    #[cfg(not(target_arch = "wasm32"))]
    fn new(source_text: &str, images: &[ImageAsset], font_paths: &[PathBuf]) -> Self {
        let font_source = if font_paths.is_empty() {
            FontSource::Cached(get_system_fonts())
        } else {
            FontSource::Shared(get_fonts_for_extra_paths(font_paths))
        };

        let main_id = FileId::new(None, VirtualPath::new("main.typ"));
        let source = Source::new(main_id, source_text.to_string());

        let image_map: HashMap<String, Bytes> = images
            .iter()
            .map(|a| (a.path.clone(), Bytes::new(a.data.clone())))
            .collect();

        Self {
            library: LazyHash::new(Library::default()),
            font_source,
            source,
            images: image_map,
        }
    }

    /// Create a new `MinimalWorld` with embedded fonts only (no system font search).
    ///
    /// Uses a process-wide cache for embedded font data. This is the constructor
    /// used on WASM targets where system font discovery is not available.
    #[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
    fn new_embedded_only(source_text: &str, images: &[ImageAsset]) -> Self {
        let main_id = FileId::new(None, VirtualPath::new("main.typ"));
        let source = Source::new(main_id, source_text.to_string());

        let image_map: HashMap<String, Bytes> = images
            .iter()
            .map(|a| (a.path.clone(), Bytes::new(a.data.clone())))
            .collect();

        Self {
            library: LazyHash::new(Library::default()),
            font_source: FontSource::Cached(get_embedded_fonts()),
            source,
            images: image_map,
        }
    }
}

impl World for MinimalWorld {
    fn library(&self) -> &LazyHash<Library> {
        &self.library
    }

    fn book(&self) -> &LazyHash<typst::text::FontBook> {
        self.font_source.book()
    }

    fn main(&self) -> FileId {
        self.source.id()
    }

    fn source(&self, id: FileId) -> FileResult<Source> {
        if id == self.source.id() {
            Ok(self.source.clone())
        } else {
            Err(typst::diag::FileError::NotFound(
                id.vpath().as_rootless_path().into(),
            ))
        }
    }

    fn file(&self, id: FileId) -> FileResult<Bytes> {
        if id == self.source.id() {
            Ok(Bytes::new(self.source.text().as_bytes().to_vec()))
        } else {
            // Check if it's an embedded image file
            let path = id.vpath().as_rootless_path().to_string_lossy();
            if let Some(data) = self.images.get(path.as_ref()) {
                Ok(data.clone()) // Bytes::clone is cheap (reference-counted)
            } else {
                Err(typst::diag::FileError::NotFound(
                    id.vpath().as_rootless_path().into(),
                ))
            }
        }
    }

    fn font(&self, index: usize) -> Option<Font> {
        self.font_source
            .fonts()
            .get(index)
            .and_then(|slot| slot.get())
    }

    fn today(&self, _offset: Option<i64>) -> Option<Datetime> {
        None
    }
}

#[cfg(test)]
#[path = "pdf_tests.rs"]
mod tests;

/// The face the compiler will shape `family` with.
///
/// The declared name may be one the font book does not register — a localized
/// East Asian family, or a face the machine does not have — so the same alias
/// and substitute chain rendering resolves through is walked here (issue #575).
/// Resolving through the same font set the compiler uses also primes the
/// compile-time cache.
#[cfg(not(target_arch = "wasm32"))]
fn best_face(family: &str) -> Option<typst::text::Font> {
    let search_context = super::font_context::resolve_font_search_context(&[]);
    let data = get_fonts_for_extra_paths(search_context.search_paths());
    super::font_subst::family_candidates(family)
        .iter()
        .find_map(|candidate| {
            data.book.select(
                &candidate.to_lowercase(),
                typst::text::FontVariant::default(),
            )
        })
        .and_then(|index| data.fonts.get(index))
        .and_then(|slot| slot.get())
}

/// Look a per-family `f64` metric up through a process-wide cache.
///
/// Font resolution walks the substitute chain and opens the face, so every
/// caller that asks the same question twice would pay for it twice.
#[cfg(not(target_arch = "wasm32"))]
fn cached_family_metric(
    cache: &OnceLock<Mutex<HashMap<String, Option<f64>>>>,
    family: &str,
    compute: impl FnOnce(&typst::text::Font) -> Option<f64>,
) -> Option<f64> {
    let cache = cache.get_or_init(|| Mutex::new(HashMap::new()));
    let key: String = family.to_lowercase();
    if let Some(cached) = cache
        .lock()
        .expect("metrics cache mutex should not be poisoned")
        .get(&key)
    {
        return *cached;
    }

    let value: Option<f64> = best_face(family).and_then(|font| compute(&font));
    cache
        .lock()
        .expect("metrics cache mutex should not be poisoned")
        .insert(key, value);
    value
}

/// The `hhea` ascender of the best face for `family`, in em units.
///
/// This is the ascent Word measures a header story's first baseline by, and it
/// is deliberately *not* [`font_line_metrics_em`]'s first element: that one
/// folds in the `hhea` line gap, which Word keeps above the header origin
/// rather than below it. The 0.0327em difference on Arial is why an 8pt header
/// baseline lands at 42.64pt below `w:pgMar/@w:header` = 35.40pt instead of
/// 42.90pt — the native export measures 42.72pt on its 0.24pt grid (issues
/// #508, #629).
///
/// Read out of the `hhea` table directly rather than through
/// `ttf_parser::Face::ascender`, whose name promises `hhea` but which returns
/// OS/2 `sTypoAscender` whenever `fsSelection` sets `USE_TYPO_METRICS`: 84 of
/// the 1109 faces installed on the calibration machine set that bit, and on 8
/// of them — Cambria Math among them, at 0.9502em against 0.7778em — the two
/// tables disagree, so the alias silently answers a different question.
///
/// Which table Word measures by is *not* settled by the corpus. Both calibrated
/// faces, Arial (0.9053em) and Malgun Gothic (1.0884em), carry `usWinAscent`
/// equal to their `hhea` ascender, and no header in the corpus uses a face where
/// they differ — though 407 of the 1109 local faces do, by up to 0.2275em
/// (Candara: 0.7246 against 0.9521). `hhea` is chosen because the ground truth
/// is macOS Word, whose text stack reports a face's ascent from `hhea`, while
/// `usWinAscent` is the GDI quantity; and because [`font_line_metrics_em`]
/// builds Word's single-line pitch from `hhea`'s ascender, descender and line
/// gap, calibrated to 0.0005em on Arial (issue #508) — taking the two halves of
/// one line box from two different tables would be the odd choice.
///
/// TODO(the corpus cannot separate `hhea` from `usWinAscent`): a native macOS
/// Word export of a Candara header would settle it outright. The attempt in this
/// session could not — every `save as … format PDF` died with AppleEvent -1712
/// and wrote a 0-byte file, across seven attempts and a clean relaunch. Note also that `font_line_metrics_em`
/// still reads its ascender through the alias, so the two disagree on those 8
/// faces until it is given the same explicit read.
#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn font_hhea_ascender_em(family: &str) -> Option<f64> {
    static ASCENDER_CACHE: OnceLock<Mutex<HashMap<String, Option<f64>>>> = OnceLock::new();
    cached_family_metric(&ASCENDER_CACHE, family, |font| {
        let ttf = font.ttf();
        Some(f64::from(ttf.tables().hhea.ascender) / f64::from(ttf.units_per_em()).max(1.0))
    })
}

#[cfg(target_arch = "wasm32")]
pub(crate) fn font_hhea_ascender_em(_family: &str) -> Option<f64> {
    None
}

/// The cap height of the best face for `family`, in em units.
///
/// This is the ascent the compiler itself gives a text line: `top-edge` defaults
/// to `"cap-height"`, and the value is taken from the very `Font` the compile
/// will shape with, so it tracks Typst's own fallbacks (`OS/2 sCapHeight`, else
/// the typographic ascender, else `hhea`) instead of restating them. Reading it
/// is what lets the header band be shifted by the *difference* between Word's
/// seat and the compiler's, leaving every line box — and therefore the story's
/// baseline-to-baseline advance — exactly as the compiler would lay it out
/// (issue #629). The advance is set separately, as a story-level
/// `par(leading:)` that tops this cap-height edge up to Word's pitch (issue
/// #735) — reading the cap height here is what keeps the two independent, since
/// the leading is computed as the remainder after it.
#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn font_cap_height_em(family: &str) -> Option<f64> {
    static CAP_HEIGHT_CACHE: OnceLock<Mutex<HashMap<String, Option<f64>>>> = OnceLock::new();
    cached_family_metric(&CAP_HEIGHT_CACHE, family, |font| {
        Some(font.metrics().cap_height.get())
    })
}

#[cfg(target_arch = "wasm32")]
pub(crate) fn font_cap_height_em(_family: &str) -> Option<f64> {
    None
}

/// Line metrics of the best face for `family`, in em units:
/// `(above baseline, below baseline, Word single-line pitch)`.
///
/// The first two split the third at the point Word puts the baseline —
/// `hhea` ascender plus line gap — so they always sum to the pitch. Typst's
/// own `metrics.ascender`/`descender` pair cannot express that split: it is
/// normalised, summing to exactly 1.0 for faces like Malgun Gothic, which put
/// the baseline 0.2em off where Word does (issue #508).
///
/// The pitch itself is the `hhea` ascender + descender + line gap sum that
/// Word uses for "single" line spacing (issue #354).
#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn font_line_metrics_em(family: &str) -> Option<(f64, f64, f64)> {
    use std::collections::HashMap;
    use std::sync::Mutex;
    type LineMetricsEm = Option<(f64, f64, f64)>;
    static METRICS_CACHE: OnceLock<Mutex<HashMap<String, LineMetricsEm>>> = OnceLock::new();

    let cache = METRICS_CACHE.get_or_init(|| Mutex::new(HashMap::new()));
    let key: String = family.to_lowercase();
    if let Some(cached) = cache
        .lock()
        .expect("metrics cache mutex should not be poisoned")
        .get(&key)
    {
        return *cached;
    }

    let metrics: Option<(f64, f64, f64)> = best_face(family).map(|font| {
        let ttf = font.ttf();
        let upem = f64::from(ttf.units_per_em()).max(1.0);
        let hhea_pitch_em = (f64::from(ttf.ascender()) - f64::from(ttf.descender())
            + f64::from(ttf.line_gap()))
            / upem;
        // Word seats the baseline `hhea ascender + lineGap` below the top
        // of the line, not at the font's ascender/descender proportion of
        // the box — measured to 0.0005em on Arial (issue #508). Typst's
        // `metrics` pair is normalised (Malgun Gothic's sums to exactly
        // 1.0) and cannot express that, so the split comes from `hhea`.
        //
        // Safe to change only since #512: the document-grid arms used to
        // derive their line count from this pair, so altering it silently
        // repaginated. They now choose between the grid pitch and the
        // natural line without consulting it.
        let top_em: f64 = (f64::from(ttf.ascender()) + f64::from(ttf.line_gap())) / upem;
        (top_em, hhea_pitch_em - top_em, hhea_pitch_em)
    });
    cache
        .lock()
        .expect("metrics cache mutex should not be poisoned")
        .insert(key, metrics);
    metrics
}

#[cfg(target_arch = "wasm32")]
pub(crate) fn font_line_metrics_em(_family: &str) -> Option<(f64, f64, f64)> {
    None
}

/// Maximum horizontal advance over the digits U+0030..=U+0039 of the best
/// face for `family`, in em units.
///
/// Excel derives every column print metric from this value of the face it
/// resolves for the workbook Normal font: 17 one-factor native Excel-for-Mac
/// probes show the column character-unit is `round_half_up(advance × size)`
/// integer points, matching each face's real `hmtx` maximum (issue #621).
/// This resolves families outside the parser's reference table — the same
/// alias and substitute chain rendering uses, so the metric tracks the face
/// the glyphs will actually come from.
#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn max_digit_advance_em(family: &str) -> Option<f64> {
    use std::collections::HashMap;
    use std::sync::Mutex;
    static ADVANCE_CACHE: OnceLock<Mutex<HashMap<String, Option<f64>>>> = OnceLock::new();

    let cache = ADVANCE_CACHE.get_or_init(|| Mutex::new(HashMap::new()));
    let key: String = family.to_lowercase();
    if let Some(cached) = cache
        .lock()
        .expect("digit advance cache mutex should not be poisoned")
        .get(&key)
    {
        return *cached;
    }

    // Use the same font set the compiler will use (system + discovered
    // Office font dirs); this also primes the compile-time cache.
    let search_context = super::font_context::resolve_font_search_context(&[]);
    let data = get_fonts_for_extra_paths(search_context.search_paths());
    let advance: Option<f64> = super::font_subst::family_candidates(family)
        .iter()
        .find_map(|candidate| {
            data.book.select(
                &candidate.to_lowercase(),
                typst::text::FontVariant::default(),
            )
        })
        .and_then(|index| data.fonts.get(index))
        .and_then(|slot| slot.get())
        .and_then(|font| {
            let ttf = font.ttf();
            let upem: f64 = f64::from(ttf.units_per_em()).max(1.0);
            ('0'..='9')
                .filter_map(|digit| {
                    ttf.glyph_index(digit)
                        .and_then(|glyph| ttf.glyph_hor_advance(glyph))
                })
                .map(|glyph_advance| f64::from(glyph_advance) / upem)
                .fold(None, |widest: Option<f64>, advance_em: f64| {
                    Some(widest.map_or(advance_em, |w| w.max(advance_em)))
                })
        });
    cache
        .lock()
        .expect("digit advance cache mutex should not be poisoned")
        .insert(key, advance);
    advance
}

#[cfg(target_arch = "wasm32")]
pub(crate) fn max_digit_advance_em(_family: &str) -> Option<f64> {
    None
}

/// Total horizontal advance of `text`, in em units, on the face `family`
/// resolves to at the requested weight.
///
/// Word's auto table layout never compresses a column below its min-content
/// width — the advance of its widest unbreakable token — so the DOCX parser
/// needs advances from the same faces the renderer will draw with
/// (issue #624). The face is resolved once per `(family, weight)` through the
/// same alias and substitute chain rendering uses and cached; bold runs must
/// measure against the bold face because its advances differ (Libertinus
/// Serif's "Total" is 2.392em bold against 2.138em regular).
///
/// Kerning and ligatures are deliberately ignored: the per-glyph `hmtx` sum
/// reproduced Word's invoice column widths within 0.10pt, and callers assert
/// with tolerances, so the ≲1-2% shaping error is acceptable. Returns `None`
/// when no face resolves or any character lacks a glyph, so the caller can
/// degrade to a measurement-free path.
///
/// Accepted limitation (shared behavior with `max_digit_advance_em`, whose
/// resolution chain is identical): resolution sees only the system fonts
/// plus the discovered Office font dirs —
/// `ConvertOptions::font_paths` and fonts embedded in the document itself are
/// not consulted, because the parser has no per-conversion font context to
/// thread through. A family only such fonts provide simply fails to resolve
/// here and the caller degrades to its measurement-free path, so the miss is
/// conservative rather than wrong.
/// TODO(issue #624): thread the per-conversion font set (options.font_paths +
/// document-embedded faces) into these measurement helpers once one exists.
#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn text_advance_em(family: &str, bold: bool, text: &str) -> Option<f64> {
    use std::collections::HashMap;
    use std::sync::Mutex;
    type ResolvedFaceCache = HashMap<(String, bool), Option<typst::text::Font>>;
    static FACE_CACHE: OnceLock<Mutex<ResolvedFaceCache>> = OnceLock::new();

    let cache = FACE_CACHE.get_or_init(|| Mutex::new(HashMap::new()));
    let key: (String, bool) = (family.to_lowercase(), bold);
    let cached_font: Option<Option<typst::text::Font>> = cache
        .lock()
        .expect("resolved face cache mutex should not be poisoned")
        .get(&key)
        .cloned();
    let font: Option<typst::text::Font> = match cached_font {
        Some(font) => font,
        None => {
            // Use the same font set the compiler will use (system + discovered
            // Office font dirs); this also primes the compile-time cache.
            let search_context = super::font_context::resolve_font_search_context(&[]);
            let data = get_fonts_for_extra_paths(search_context.search_paths());
            let variant = typst::text::FontVariant {
                weight: if bold {
                    typst::text::FontWeight::BOLD
                } else {
                    typst::text::FontWeight::REGULAR
                },
                ..typst::text::FontVariant::default()
            };
            let resolved: Option<typst::text::Font> = super::font_subst::family_candidates(family)
                .iter()
                .find_map(|candidate| data.book.select(&candidate.to_lowercase(), variant))
                .and_then(|index| data.fonts.get(index))
                .and_then(|slot| slot.get());
            cache
                .lock()
                .expect("resolved face cache mutex should not be poisoned")
                .insert(key, resolved.clone());
            resolved
        }
    };

    let font: typst::text::Font = font?;
    let ttf = font.ttf();
    let upem: f64 = f64::from(ttf.units_per_em()).max(1.0);
    let mut total_em: f64 = 0.0;
    for character in text.chars() {
        let glyph_advance: u16 = ttf
            .glyph_index(character)
            .and_then(|glyph| ttf.glyph_hor_advance(glyph))?;
        total_em += f64::from(glyph_advance) / upem;
    }
    Some(total_em)
}

#[cfg(target_arch = "wasm32")]
pub(crate) fn text_advance_em(_family: &str, _bold: bool, _text: &str) -> Option<f64> {
    None
}

/// PowerPoint's line height factor: it gives every line 1.2 times the font
/// size, whatever the font's own metrics say.
///
/// Measured on native exports from both platforms. On macOS PowerPoint, one
/// wrapped Arial paragraph advances `(last baseline - first) / 14 = 20.3657pt`
/// at 17pt (1.1980em) and `/ 18 = 28.8400pt` at 24pt (1.2017em) — the division
/// cancels the export's 0.24pt position grid, which is what made small samples
/// look font-dependent. Windows PowerPoint measures the same 1.2000em flat for
/// Arial, Calibri, and Malgun Gothic (issues #485, #513).
pub(crate) const POWERPOINT_LINE_HEIGHT_FACTOR: f64 = 1.2;

/// PowerPoint's `(above baseline, below baseline)` split of that line, in em.
///
/// PowerPoint splits the line's **extra leading evenly** above and below the
/// glyphs: the glyphs take `ascent + descent` and the remainder of the 1.2em
/// line is halved, seating the baseline at `(1.2 + ascent - descent) / 2`.
///
/// This replaced a proportional split, `1.2 x winAscent / (winAscent +
/// winDescent)`. Both reproduce the line's *height* — they must, since both
/// span 1.2em — so only the first baseline separates them, and the native
/// exports side with the even split. On `08_marketing_report_en` p3, a 17pt
/// top-anchored frame, GT's first baseline is 142.08pt; the even split predicts
/// 142.09 and the proportional one put us at 142.53, 0.45pt low. Arial's two
/// predictions are 0.9467em and 0.9724em (issue #660).
///
/// The metrics are hhea's, not OS/2's `usWin*` pair: the even split needs the
/// ascent and descent as em fractions rather than as a ratio, and the two
/// tables disagree about the denominator.
///
/// The below-baseline half is the descent gap a bottom-anchored box keeps under
/// its last baseline, which we used to drop entirely (issue #513).
#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn powerpoint_line_box_em(family: &str) -> Option<(f64, f64)> {
    use std::collections::HashMap;
    use std::sync::Mutex;
    type LineBoxEm = Option<(f64, f64)>;
    static CACHE: OnceLock<Mutex<HashMap<String, LineBoxEm>>> = OnceLock::new();

    let cache = CACHE.get_or_init(|| Mutex::new(HashMap::new()));
    let key: String = family.to_lowercase();
    if let Some(cached) = cache
        .lock()
        .expect("metrics cache mutex should not be poisoned")
        .get(&key)
    {
        return *cached;
    }

    // Resolve through the same substitute chain Typst sees in the emitted font
    // list. If none exists, Typst uses its embedded default, so the metrics
    // lookup must end there too.
    // Otherwise a missing Office face can collapse consecutive paragraphs to
    // the fallback font's glyph height (issue #705).
    let split: Option<(f64, f64)> = best_face(family)
        .or_else(|| best_face(crate::defaults::TYPST_DEFAULT_FONT_FAMILY))
        .and_then(|font| {
            let ttf = font.ttf();
            let upem = f64::from(ttf.units_per_em()).max(1.0);
            let ascent = f64::from(ttf.ascender()).abs() / upem;
            let descent = f64::from(ttf.descender()).abs() / upem;
            (ascent > 0.0).then(|| {
                // The glyphs occupy `ascent + descent`; whatever the 1.2em line
                // has left over is the extra leading, and PowerPoint puts half
                // of it above the glyphs and half below. So the baseline sits
                // `ascent + (1.2 - ascent - descent) / 2` below the top, which
                // is the expression below.
                let above = (POWERPOINT_LINE_HEIGHT_FACTOR + ascent - descent) / 2.0;
                let above = above.clamp(0.0, POWERPOINT_LINE_HEIGHT_FACTOR);
                (above, POWERPOINT_LINE_HEIGHT_FACTOR - above)
            })
        });
    cache
        .lock()
        .expect("metrics cache mutex should not be poisoned")
        .insert(key, split);
    split
}

#[cfg(target_arch = "wasm32")]
pub(crate) fn powerpoint_line_box_em(_family: &str) -> Option<(f64, f64)> {
    None
}
