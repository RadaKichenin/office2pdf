//! Which of a face's two kern sources reaches the shaper.
//!
//! A face may state its pair adjustments twice: once in the legacy `kern`
//! table and once in the GPOS `kern` feature. OpenType tells a shaper that
//! finds the GPOS feature to ignore the table, and rustybuzz — the shaper
//! Typst positions glyphs with — does exactly that. CoreText, which is what
//! Office on macOS lays text out through, keeps reading the `kern` table of a
//! TrueType face. Where a face's two sources disagree, the choice is visible:
//! `Posterama Bold` states `T`/`A` as -102 units in `kern` against -50 in
//! GPOS, so every kerned pair of a 38pt PowerPoint title landed about half-way
//! and the line came out 2.53pt wide (issue #1116).
//!
//! A face carrying both is therefore handed to the shaper with its GPOS `kern`
//! feature renamed out of reach. That is the one lever that reaches the
//! shaper's own decision — rustybuzz applies the legacy table exactly when it
//! finds no GPOS `kern` feature — and it leaves every other feature of the
//! face, `mark` and `mkmk` included, doing its job. A face that states only
//! one source is left alone: taking GPOS kerning from a face that has nothing
//! else would lose the pairs altogether.

use typst::foundations::Bytes;
use typst::text::Font;

/// The tag a GPOS `kern` feature is renamed to.
///
/// `kerm` is not a registered OpenType feature, so no shaper maps it, and it
/// sorts immediately before `kern` with no registered tag in between. The
/// `FeatureList` therefore stays in the alphabetical order the spec states,
/// which a shaper is entitled to binary-search for another feature.
const UNREACHABLE_KERN_FEATURE_TAG: &[u8; 4] = b"kerm";

const KERN_FEATURE_TAG: &[u8; 4] = b"kern";
const TABLE_RECORD_LEN: usize = 16;

/// The face to hand the shaper in place of `font`, or `None` when `font`
/// itself is already it.
pub(crate) fn face_preferring_legacy_kern(font: &Font) -> Option<Font> {
    if !kerns_horizontally_from_its_kern_table(font) {
        return None;
    }

    let patched: Vec<u8> = bytes_preferring_legacy_kern(font.data(), font.index())?;
    Font::new(Bytes::new(patched), font.index())
}

/// Whether the shaper would find horizontal pairs in the face's `kern` table.
///
/// This asks the same parser the shaper asks, so a `kern` table it cannot read
/// — or one holding only vertical or variable subtables — does not count as a
/// second source. Renaming the GPOS feature for such a face would leave it
/// with no pairs at all.
fn kerns_horizontally_from_its_kern_table(font: &Font) -> bool {
    font.ttf().tables().kern.is_some_and(|kern| {
        kern.subtables
            .into_iter()
            .any(|subtable| subtable.horizontal && !subtable.variable)
    })
}

/// Rewrites `data` so the face at `face_index` offers no GPOS `kern` feature,
/// or returns `None` when the face does not carry both kern sources.
///
/// The rewrite is length-preserving — it edits tags and a lookup count in
/// place — so every table offset in the file survives it.
fn bytes_preferring_legacy_kern(data: &[u8], face_index: u32) -> Option<Vec<u8>> {
    let directory: usize = table_directory_offset(data, face_index)?;
    // An empty `kern` record is no source at all.
    let kern: (usize, usize) = table_range(data, directory, b"kern")?;
    if kern.1 == 0 {
        return None;
    }

    let (gpos_start, gpos_len): (usize, usize) = table_range(data, directory, b"GPOS")?;
    let records: Vec<FeatureRecord> = gpos_feature_records(data, gpos_start, gpos_len)?;
    let kern_records: Vec<&FeatureRecord> = records
        .iter()
        .filter(|record| record.tag == *KERN_FEATURE_TAG)
        .collect();
    if kern_records.is_empty() {
        return None;
    }

    let mut patched: Vec<u8> = data.to_vec();
    for record in &kern_records {
        patched[record.tag_offset..record.tag_offset + 4]
            .copy_from_slice(UNREACHABLE_KERN_FEATURE_TAG);

        // A `LangSys` may name a feature as *required*, in which case a shaper
        // applies it by index without ever reading its tag. Emptying the
        // feature's lookup list keeps such a face from taking both sources at
        // once. A `Feature` table another tag also points at is left alone —
        // its lookups belong to that feature too.
        let shared: bool = records.iter().any(|other| {
            other.tag != *KERN_FEATURE_TAG && other.feature_offset == record.feature_offset
        });
        if !shared
            && let Some(lookup_count) = record.feature_offset.checked_add(2)
            && lookup_count + 2 <= patched.len()
        {
            patched[lookup_count..lookup_count + 2].copy_from_slice(&0u16.to_be_bytes());
        }
    }

    Some(patched)
}

/// One entry of a GPOS `FeatureList`, with the byte offsets the rewrite edits.
struct FeatureRecord {
    tag: [u8; 4],
    /// Where the record's tag sits in the file.
    tag_offset: usize,
    /// Where the `Feature` table the record points at sits in the file.
    feature_offset: usize,
}

/// Where the table directory of the face at `face_index` starts.
///
/// A TrueType collection holds one directory per face at an offset its header
/// states; a standalone face has the single directory at the start of the file.
fn table_directory_offset(data: &[u8], face_index: u32) -> Option<usize> {
    if data.get(..4)? != b"ttcf" {
        return (face_index == 0).then_some(0);
    }

    let count: u32 = read_u32(data, 8)?;
    if face_index >= count {
        return None;
    }
    let offset: usize = 12 + 4 * usize::try_from(face_index).ok()?;
    usize::try_from(read_u32(data, offset)?).ok()
}

/// The start and length of `tag`'s table, both bounded by the file.
fn table_range(data: &[u8], directory: usize, tag: &[u8; 4]) -> Option<(usize, usize)> {
    let count: u16 = read_u16(data, directory.checked_add(4)?)?;
    for index in 0..usize::from(count) {
        let record: usize = directory.checked_add(12 + TABLE_RECORD_LEN * index)?;
        if data.get(record..record + 4)? != tag {
            continue;
        }
        let start: usize = usize::try_from(read_u32(data, record + 8)?).ok()?;
        let length: usize = usize::try_from(read_u32(data, record + 12)?).ok()?;
        if start.checked_add(length)? > data.len() {
            return None;
        }
        return Some((start, length));
    }
    None
}

/// Every `FeatureRecord` of the GPOS table starting at `gpos_start`.
fn gpos_feature_records(
    data: &[u8],
    gpos_start: usize,
    gpos_len: usize,
) -> Option<Vec<FeatureRecord>> {
    let feature_list: usize =
        gpos_start.checked_add(usize::from(read_u16(data, gpos_start.checked_add(6)?)?))?;
    if feature_list.checked_add(2)? > gpos_start.checked_add(gpos_len)? {
        return None;
    }

    let count: u16 = read_u16(data, feature_list)?;
    let mut records: Vec<FeatureRecord> = Vec::with_capacity(usize::from(count));
    for index in 0..usize::from(count) {
        let record: usize = feature_list.checked_add(2 + 6 * index)?;
        let tag: [u8; 4] = data.get(record..record + 4)?.try_into().ok()?;
        let feature_offset: usize =
            feature_list.checked_add(usize::from(read_u16(data, record + 4)?))?;
        records.push(FeatureRecord {
            tag,
            tag_offset: record,
            feature_offset,
        });
    }
    Some(records)
}

fn read_u16(data: &[u8], offset: usize) -> Option<u16> {
    let bytes: [u8; 2] = data.get(offset..offset.checked_add(2)?)?.try_into().ok()?;
    Some(u16::from_be_bytes(bytes))
}

fn read_u32(data: &[u8], offset: usize) -> Option<u32> {
    let bytes: [u8; 4] = data.get(offset..offset.checked_add(4)?)?.try_into().ok()?;
    Some(u32::from_be_bytes(bytes))
}

#[cfg(test)]
#[path = "font_kern_tests.rs"]
mod tests;
