//! Series stroke defaults from the chart's related Office chart-style part.

use std::io::{Read, Seek};

use quick_xml::Reader;
use quick_xml::events::Event;
use zip::ZipArchive;

use super::{chart, drawingml::SchemeColors, xml_util};
use crate::ir::Chart;

const CHART_STYLE_REL: &str = "http://schemas.microsoft.com/office/2011/relationships/chartStyle";

/// Parse a package chart with its related stroke defaults. The XML parser
/// applies defaults before distinct OOXML families are normalized into IR.
pub(crate) fn parse_chart_with_style<R: Read + Seek>(
    archive: &mut ZipArchive<R>,
    chart_path: &str,
    xml: &str,
    scheme: &SchemeColors<'_>,
) -> Option<Chart> {
    let Some(style_xml) = load_style(archive, chart_path) else {
        return chart::parse_chart_xml(xml, scheme);
    };
    let defaults = chart::parse_series_stroke_defaults(&style_xml);
    chart::parse_chart_xml_with_stroke_defaults(xml, scheme, defaults)
}

fn load_style<R: Read + Seek>(archive: &mut ZipArchive<R>, chart_path: &str) -> Option<String> {
    let (directory, file) = chart_path.rsplit_once('/').unwrap_or(("", chart_path));
    let rels_path = if directory.is_empty() {
        format!("_rels/{file}.rels")
    } else {
        format!("{directory}/_rels/{file}.rels")
    };
    let rels_xml = read_part(archive, &rels_path)?;
    let target = chart_style_target(&rels_xml)?;
    let style_path = resolve_part(directory, &target);
    read_part(archive, &style_path).or_else(|| {
        tracing::debug!(
            chart_path,
            style_path,
            "Related chart style part is unavailable"
        );
        None
    })
}

fn read_part<R: Read + Seek>(archive: &mut ZipArchive<R>, path: &str) -> Option<String> {
    let mut part = archive.by_name(path).ok()?;
    let mut text = String::new();
    part.read_to_string(&mut text).ok()?;
    Some(text)
}

fn chart_style_target(xml: &str) -> Option<String> {
    let mut reader = Reader::from_str(xml);
    loop {
        match reader.read_event() {
            Ok(Event::Start(ref element)) | Ok(Event::Empty(ref element))
                if element.local_name().as_ref() == b"Relationship"
                    && xml_util::get_attr_str(element, b"Type").as_deref()
                        == Some(CHART_STYLE_REL)
                    && xml_util::get_attr_str(element, b"TargetMode").as_deref()
                        != Some("External") =>
            {
                if let Some(target) = xml_util::get_attr_str(element, b"Target") {
                    return Some(target);
                }
            }
            Ok(Event::Eof) | Err(_) => return None,
            _ => {}
        }
    }
}

fn resolve_part(directory: &str, target: &str) -> String {
    if let Some(absolute) = target.strip_prefix('/') {
        return absolute.to_owned();
    }
    let mut parts: Vec<&str> = directory
        .split('/')
        .filter(|part| !part.is_empty())
        .collect();
    for part in target.split('/') {
        match part {
            "" | "." => {}
            ".." => {
                parts.pop();
            }
            _ => parts.push(part),
        }
    }
    parts.join("/")
}

#[cfg(test)]
#[path = "chart_style_tests.rs"]
mod tests;
