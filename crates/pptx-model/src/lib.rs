//! Lossless, editable PPTX document model (issue #746).
//!
//! [`PptxDocument`] wraps [`ooxml_package::OpcPackage`] so every part of the
//! original file — including parts and XML this crate does not model —
//! survives load, edit, and save. Edits are surgical: replacing a text run
//! splices only that run's character data inside its slide part and leaves
//! every other byte of the package untouched.
//!
//! The rendering pipeline in `office2pdf` stays a derived projection: saving
//! this model yields a standard `.pptx` container that the existing
//! converter (and PowerPoint) can open, so PDF preview is always available
//! without this model ever becoming lossy.

use std::borrow::Cow;

use ooxml_package::{OpcPackage, resolve_relationship_target};
use quick_xml::NsReader;
use quick_xml::events::{BytesRef, BytesStart, Event};
use quick_xml::name::{Namespace, QName, ResolveResult};

const DRAWINGML_NAMESPACE: &[u8] = b"http://schemas.openxmlformats.org/drawingml/2006/main";
const PRESENTATIONML_NAMESPACE: &[u8] =
    b"http://schemas.openxmlformats.org/presentationml/2006/main";
const RELATIONSHIPS_NAMESPACE: &[u8] =
    b"http://schemas.openxmlformats.org/officeDocument/2006/relationships";
/// Prefix shared by every PresentationML main-part content type
/// (presentation, slideshow, template, and their macro-enabled variants).
const PRESENTATION_CONTENT_TYPE_PREFIX: &str =
    "application/vnd.openxmlformats-officedocument.presentationml";

/// Errors raised while loading or editing a PPTX document.
#[derive(Debug, thiserror::Error)]
pub enum PptxError {
    /// The underlying OPC package failed to load, read, or save.
    #[error(transparent)]
    Package(#[from] ooxml_package::PackageError),
    /// The package is a valid OPC container but not a PowerPoint presentation.
    #[error("package is not a PowerPoint presentation: {0}")]
    NotAPresentation(String),
    /// A slide part holds XML this crate could not parse.
    #[error("malformed XML in part {part:?}: {message}")]
    MalformedXml {
        /// Part name whose XML failed to parse.
        part: String,
        /// Parser diagnostic.
        message: String,
    },
    /// A slide index is out of range.
    #[error("slide index {index} out of range: the presentation has {slide_count} slides")]
    SlideIndexOutOfRange {
        /// The requested zero-based slide index.
        index: usize,
        /// Number of slides in the presentation.
        slide_count: usize,
    },
    /// A text-run index is out of range for the addressed slide.
    #[error("text run index {index} out of range: slide part {part:?} has {run_count} runs")]
    TextRunIndexOutOfRange {
        /// Slide part name.
        part: String,
        /// The requested zero-based run index.
        index: usize,
        /// Number of text runs on that slide.
        run_count: usize,
    },
}

/// A lossless, editable PowerPoint presentation.
#[derive(Debug)]
pub struct PptxDocument {
    package: OpcPackage,
    /// Slide part names in presentation (slide show) order, resolved from
    /// `p:sldIdLst` through the presentation part's relationships.
    slide_part_names: Vec<String>,
}

impl PptxDocument {
    /// Load a presentation from `.pptx` container bytes.
    pub fn from_bytes(bytes: Vec<u8>) -> Result<Self, PptxError> {
        let package: OpcPackage = OpcPackage::from_bytes(bytes)?;

        let root_relationships: Vec<ooxml_package::Relationship> = package.relationships(None)?;
        let office_document: &ooxml_package::Relationship = root_relationships
            .iter()
            .find(|relationship| {
                relationship.target_mode == ooxml_package::TargetMode::Internal
                    && relationship.relationship_type.ends_with("/officeDocument")
            })
            .ok_or_else(|| {
                PptxError::NotAPresentation(
                    "the package declares no officeDocument relationship".to_string(),
                )
            })?;
        let presentation_part: String = resolve_relationship_target(None, &office_document.target);
        if !package.has_part(&presentation_part) {
            return Err(PptxError::NotAPresentation(format!(
                "main part {presentation_part:?} is missing"
            )));
        }
        let content_type: Option<String> = package.content_type_of(&presentation_part)?;
        if !content_type
            .as_deref()
            .is_some_and(|declared| declared.starts_with(PRESENTATION_CONTENT_TYPE_PREFIX))
        {
            return Err(PptxError::NotAPresentation(format!(
                "main part content type {content_type:?} is not PresentationML"
            )));
        }

        let presentation_xml: Cow<'_, [u8]> = package.part_bytes(&presentation_part)?;
        let slide_relationship_ids: Vec<String> =
            parse_slide_relationship_ids(&presentation_xml, &presentation_part)?;
        drop(presentation_xml);

        let presentation_relationships: Vec<ooxml_package::Relationship> =
            package.relationships(Some(&presentation_part))?;
        let mut slide_part_names: Vec<String> = Vec::with_capacity(slide_relationship_ids.len());
        for relationship_id in &slide_relationship_ids {
            let relationship = presentation_relationships
                .iter()
                .find(|relationship| relationship.id == *relationship_id)
                .ok_or_else(|| {
                    PptxError::NotAPresentation(format!(
                        "sldIdLst references undeclared relationship {relationship_id:?}"
                    ))
                })?;
            let slide_part: String =
                resolve_relationship_target(Some(&presentation_part), &relationship.target);
            if !package.has_part(&slide_part) {
                return Err(PptxError::NotAPresentation(format!(
                    "slide part {slide_part:?} is missing"
                )));
            }
            slide_part_names.push(slide_part);
        }
        tracing::debug!(
            slide_count = slide_part_names.len(),
            presentation_part,
            "loaded PPTX document"
        );

        Ok(Self {
            package,
            slide_part_names,
        })
    }

    /// Load a presentation from a file on disk.
    pub fn from_path(path: impl AsRef<std::path::Path>) -> Result<Self, PptxError> {
        Self::from_bytes(std::fs::read(path).map_err(ooxml_package::PackageError::from)?)
    }

    /// Slide part names in presentation (slide show) order.
    pub fn slide_part_names(&self) -> &[String] {
        &self.slide_part_names
    }

    /// Number of slides in the presentation.
    pub fn slide_count(&self) -> usize {
        self.slide_part_names.len()
    }

    /// The text of every `a:t` run on a slide, in document order.
    pub fn slide_text_runs(&self, slide_index: usize) -> Result<Vec<String>, PptxError> {
        let slide_part: &str = self.slide_part_name_at(slide_index)?;
        let xml: Cow<'_, [u8]> = self.package.part_bytes(slide_part)?;
        let spans: Vec<TextRunSpan> = scan_text_runs(&xml, slide_part)?;
        Ok(spans.into_iter().map(|span| span.text).collect())
    }

    /// Replace the text of one run on one slide.
    ///
    /// Only the addressed slide part is rewritten; every other package part
    /// keeps its exact original bytes.
    pub fn set_slide_text_run(
        &mut self,
        slide_index: usize,
        run_index: usize,
        new_text: &str,
    ) -> Result<(), PptxError> {
        let slide_part: String = self.slide_part_name_at(slide_index)?.to_string();
        let xml: Vec<u8> = self.package.part_bytes(&slide_part)?.into_owned();
        let spans: Vec<TextRunSpan> = scan_text_runs(&xml, &slide_part)?;
        let span: &TextRunSpan =
            spans
                .get(run_index)
                .ok_or_else(|| PptxError::TextRunIndexOutOfRange {
                    part: slide_part.clone(),
                    index: run_index,
                    run_count: spans.len(),
                })?;
        let spliced: Vec<u8> = splice_text_run(&xml, span, new_text);
        tracing::debug!(
            slide_part,
            run_index,
            new_text_len = new_text.len(),
            "replacing slide text run"
        );
        self.package.set_part_bytes(&slide_part, spliced)?;
        Ok(())
    }

    /// The underlying lossless package (for example to inspect dirty parts).
    pub fn package(&self) -> &OpcPackage {
        &self.package
    }

    /// Serialize the presentation back into `.pptx` container bytes.
    pub fn save_to_bytes(&self) -> Result<Vec<u8>, PptxError> {
        Ok(self.package.save_to_bytes()?)
    }

    fn slide_part_name_at(&self, slide_index: usize) -> Result<&str, PptxError> {
        self.slide_part_names
            .get(slide_index)
            .map(String::as_str)
            .ok_or(PptxError::SlideIndexOutOfRange {
                index: slide_index,
                slide_count: self.slide_part_names.len(),
            })
    }
}

/// One `a:t` element found in a slide part.
struct TextRunSpan {
    /// Unescaped character data currently in the run.
    text: String,
    /// Where the run's bytes live, for surgical replacement.
    location: TextRunLocation,
}

enum TextRunLocation {
    /// `<a:t>…</a:t>`: replace the bytes between the tags.
    Element {
        content_start: usize,
        content_end: usize,
    },
    /// `<a:t/>`: replace the whole tag with an open/content/close triple.
    EmptyTag {
        tag_start: usize,
        tag_end: usize,
        /// The tag name as written (usually `a:t`), reused when rebuilding.
        qualified_name: Vec<u8>,
    },
}

fn malformed(part: &str, message: impl ToString) -> PptxError {
    PptxError::MalformedXml {
        part: part.to_string(),
        message: message.to_string(),
    }
}

fn is_in_namespace<R>(
    reader: &NsReader<R>,
    name: QName<'_>,
    namespace: &[u8],
    local: &[u8],
) -> bool {
    let (resolve_result, local_name) = reader.resolve_element(name);
    matches!(resolve_result, ResolveResult::Bound(Namespace(bound)) if bound == namespace)
        && local_name.as_ref() == local
}

/// The `r:id` of every `p:sldId` in the presentation part, in `sldIdLst`
/// (slide show) order.
fn parse_slide_relationship_ids(xml: &[u8], part: &str) -> Result<Vec<String>, PptxError> {
    let mut reader: NsReader<&[u8]> = NsReader::from_reader(xml);
    let mut relationship_ids: Vec<String> = Vec::new();
    loop {
        match reader.read_event() {
            Ok(Event::Start(element)) | Ok(Event::Empty(element))
                if is_in_namespace(&reader, element.name(), PRESENTATIONML_NAMESPACE, b"sldId") =>
            {
                relationship_ids.push(slide_relationship_id(&reader, &element, part)?);
            }
            Ok(Event::Eof) => break,
            Ok(_) => {}
            Err(error) => return Err(malformed(part, error)),
        }
    }
    Ok(relationship_ids)
}

fn slide_relationship_id(
    reader: &NsReader<&[u8]>,
    element: &BytesStart<'_>,
    part: &str,
) -> Result<String, PptxError> {
    for attribute in element.attributes() {
        let attribute = attribute.map_err(|error| malformed(part, error))?;
        let (resolve_result, local_name) = reader.resolve_attribute(attribute.key);
        let is_relationship_id: bool = matches!(
            resolve_result,
            ResolveResult::Bound(Namespace(bound)) if bound == RELATIONSHIPS_NAMESPACE
        ) && local_name.as_ref() == b"id";
        if is_relationship_id {
            return Ok(attribute
                .unescape_value()
                .map_err(|error| malformed(part, error))?
                .into_owned());
        }
    }
    Err(malformed(part, "sldId element lacks an r:id attribute"))
}

/// Every `a:t` element in a slide part, in document order, with the byte
/// spans needed to splice replacement text.
fn scan_text_runs(xml: &[u8], part: &str) -> Result<Vec<TextRunSpan>, PptxError> {
    let mut reader: NsReader<&[u8]> = NsReader::from_reader(xml);
    let mut spans: Vec<TextRunSpan> = Vec::new();
    // `(content_start, accumulated text)` of an open `<a:t>` element. The
    // schema forbids nested `a:t`, so one pending slot suffices.
    let mut pending: Option<(usize, String)> = None;
    loop {
        let event_start: usize = reader.buffer_position() as usize;
        let event: Event<'_> = reader
            .read_event()
            .map_err(|error| malformed(part, error))?;
        let event_end: usize = reader.buffer_position() as usize;
        match event {
            Event::Start(element)
                if is_in_namespace(&reader, element.name(), DRAWINGML_NAMESPACE, b"t") =>
            {
                pending = Some((event_end, String::new()));
            }
            Event::Empty(element)
                if is_in_namespace(&reader, element.name(), DRAWINGML_NAMESPACE, b"t") =>
            {
                spans.push(TextRunSpan {
                    text: String::new(),
                    location: TextRunLocation::EmptyTag {
                        tag_start: event_start,
                        tag_end: event_end,
                        qualified_name: element.name().as_ref().to_vec(),
                    },
                });
            }
            Event::End(element)
                if pending.is_some()
                    && is_in_namespace(&reader, element.name(), DRAWINGML_NAMESPACE, b"t") =>
            {
                let (content_start, text) = pending.take().expect("pending run is present");
                spans.push(TextRunSpan {
                    text,
                    location: TextRunLocation::Element {
                        content_start,
                        content_end: event_start,
                    },
                });
            }
            Event::Text(text) => {
                if let Some((_, accumulated)) = pending.as_mut() {
                    let decoded: Cow<'_, str> =
                        text.decode().map_err(|error| malformed(part, error))?;
                    let unescaped: Cow<'_, str> = quick_xml::escape::unescape(decoded.as_ref())
                        .map_err(|error| malformed(part, error))?;
                    accumulated.push_str(&unescaped);
                }
            }
            Event::CData(cdata) => {
                if let Some((_, accumulated)) = pending.as_mut() {
                    accumulated.push_str(&String::from_utf8_lossy(&cdata.into_inner()));
                }
            }
            Event::GeneralRef(reference) => {
                if let Some((_, accumulated)) = pending.as_mut() {
                    accumulated.push_str(&resolve_general_reference(&reference, part)?);
                }
            }
            Event::Eof => {
                if pending.is_some() {
                    return Err(malformed(part, "unterminated a:t element"));
                }
                break;
            }
            _ => {}
        }
    }
    Ok(spans)
}

/// Resolve a general entity reference occurring inside run text.
///
/// OOXML documents carry no DTD, so only character references and the five
/// predefined XML entities can appear; `unescape` rejects anything else.
fn resolve_general_reference(reference: &BytesRef<'_>, part: &str) -> Result<String, PptxError> {
    let decoded: Cow<'_, str> = reference.decode().map_err(|error| malformed(part, error))?;
    let wrapped: String = format!("&{};", decoded.as_ref());
    let unescaped: Cow<'_, str> =
        quick_xml::escape::unescape(&wrapped).map_err(|error| malformed(part, error))?;
    Ok(unescaped.into_owned())
}

/// Rebuild the slide XML with one run's character data replaced.
///
/// Everything outside the addressed span is copied byte-for-byte.
fn splice_text_run(xml: &[u8], span: &TextRunSpan, new_text: &str) -> Vec<u8> {
    let escaped: Cow<'_, str> = quick_xml::escape::partial_escape(new_text);
    match &span.location {
        TextRunLocation::Element {
            content_start,
            content_end,
        } => {
            let mut spliced: Vec<u8> =
                Vec::with_capacity(xml.len() - (content_end - content_start) + escaped.len());
            spliced.extend_from_slice(&xml[..*content_start]);
            spliced.extend_from_slice(escaped.as_bytes());
            spliced.extend_from_slice(&xml[*content_end..]);
            spliced
        }
        TextRunLocation::EmptyTag {
            tag_start,
            tag_end,
            qualified_name,
        } => {
            // `<a:t ... />` becomes `<a:t ...>new text</a:t>`, keeping any
            // attributes the original tag carried.
            let raw_tag: &[u8] = &xml[*tag_start..*tag_end];
            let open_tag: &[u8] = raw_tag
                .strip_suffix(b"/>")
                .expect("an empty-element tag ends with />");
            let mut spliced: Vec<u8> = Vec::with_capacity(xml.len() + escaped.len() + 16);
            spliced.extend_from_slice(&xml[..*tag_start]);
            spliced.extend_from_slice(open_tag);
            spliced.push(b'>');
            spliced.extend_from_slice(escaped.as_bytes());
            spliced.extend_from_slice(b"</");
            spliced.extend_from_slice(qualified_name);
            spliced.push(b'>');
            spliced.extend_from_slice(&xml[*tag_end..]);
            spliced
        }
    }
}
