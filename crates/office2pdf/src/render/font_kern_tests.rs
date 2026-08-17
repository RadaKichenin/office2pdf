//! Tests for the kern source handed to the shaper.

use super::*;
use crate::test_support::{
    make_face_carrying_both_kern_sources, make_legacy_kern_table, states_a_gpos_kern_feature,
};

/// A synthetic sfnt, with the offsets a test needs to assert on.
struct SyntheticFace {
    data: Vec<u8>,
    /// Where each GPOS `FeatureRecord`'s tag sits, in the order given.
    feature_tag_offsets: Vec<usize>,
    /// Where each `FeatureRecord`'s `lookupIndexCount` sits.
    feature_lookup_count_offsets: Vec<usize>,
    /// Where the legacy `kern` table starts, when the face carries one.
    legacy_kern_start: Option<usize>,
}

/// Builds an sfnt whose table directory and GPOS `FeatureList` are exact.
///
/// Nothing here has to render: the rewrite reads the table directory and the
/// feature list, so those are the parts a fixture has to state faithfully.
/// Every `Feature` table names one lookup, so a test can see the lookup count
/// being emptied.
fn synthetic_face(feature_tags: &[&[u8; 4]], with_legacy_kern: bool) -> SyntheticFace {
    // GPOS 1.0 header: major, minor, scriptListOffset, featureListOffset,
    // lookupListOffset. The script and lookup lists are left empty.
    const FEATURE_LIST_OFFSET: usize = 10;
    let mut gpos: Vec<u8> = vec![0, 1, 0, 0, 0, 10, 0, 10, 0, 10];
    gpos.extend_from_slice(&(feature_tags.len() as u16).to_be_bytes());

    let mut tag_offsets: Vec<usize> = Vec::new();
    let record_list_len: usize = 6 * feature_tags.len();
    for (index, tag) in feature_tags.iter().enumerate() {
        tag_offsets.push(gpos.len());
        gpos.extend_from_slice(*tag);
        // Each record points at its own `Feature` table past the record list.
        let feature_offset: usize = 2 + record_list_len + 6 * index;
        gpos.extend_from_slice(&(feature_offset as u16).to_be_bytes());
    }
    let mut lookup_count_offsets: Vec<usize> = Vec::new();
    for index in 0..feature_tags.len() {
        assert_eq!(
            gpos.len(),
            FEATURE_LIST_OFFSET + 2 + record_list_len + 6 * index
        );
        gpos.extend_from_slice(&0u16.to_be_bytes()); // featureParamsOffset
        lookup_count_offsets.push(gpos.len());
        gpos.extend_from_slice(&1u16.to_be_bytes()); // lookupIndexCount
        gpos.extend_from_slice(&0u16.to_be_bytes()); // lookupListIndices[0]
    }

    let mut tables: Vec<(&[u8; 4], Vec<u8>)> = vec![(b"GPOS", gpos)];
    if with_legacy_kern {
        tables.push((b"kern", make_legacy_kern_table()));
    }

    let mut data: Vec<u8> = Vec::new();
    data.extend_from_slice(&0x0001_0000u32.to_be_bytes()); // sfntVersion
    data.extend_from_slice(&(tables.len() as u16).to_be_bytes());
    data.extend_from_slice(&[0; 6]); // searchRange, entrySelector, rangeShift

    let mut body_offset: usize = 12 + 16 * tables.len();
    let mut gpos_start: usize = 0;
    let mut legacy_kern_start: Option<usize> = None;
    for (tag, bytes) in &tables {
        data.extend_from_slice(*tag);
        data.extend_from_slice(&0u32.to_be_bytes()); // checkSum
        data.extend_from_slice(&(body_offset as u32).to_be_bytes());
        data.extend_from_slice(&(bytes.len() as u32).to_be_bytes());
        match *tag {
            b"GPOS" => gpos_start = body_offset,
            b"kern" => legacy_kern_start = Some(body_offset),
            _ => {}
        }
        body_offset += bytes.len();
    }
    for (_, bytes) in &tables {
        data.extend_from_slice(bytes);
    }

    SyntheticFace {
        feature_tag_offsets: tag_offsets
            .iter()
            .map(|offset| gpos_start + offset)
            .collect(),
        feature_lookup_count_offsets: lookup_count_offsets
            .iter()
            .map(|offset| gpos_start + offset)
            .collect(),
        legacy_kern_start,
        data,
    }
}

/// Wraps faces into a TrueType collection, whose header offsets each face's
/// own table directory. Returns the file and where each face's directory
/// starts, since a collection's table records hold file-absolute offsets and
/// so have to be rebased onto the face's place in it.
fn synthetic_collection(faces: &[&SyntheticFace]) -> (Vec<u8>, Vec<usize>) {
    let mut starts: Vec<usize> = Vec::with_capacity(faces.len());
    let mut offset: usize = 12 + 4 * faces.len();
    for face in faces {
        starts.push(offset);
        offset += face.data.len();
    }

    let mut data: Vec<u8> = Vec::new();
    data.extend_from_slice(b"ttcf");
    data.extend_from_slice(&0x0001_0000u32.to_be_bytes());
    data.extend_from_slice(&(faces.len() as u32).to_be_bytes());
    for start in &starts {
        data.extend_from_slice(&(*start as u32).to_be_bytes());
    }

    for (face, start) in faces.iter().zip(&starts) {
        let mut bytes: Vec<u8> = face.data.clone();
        for index in 0..usize::from(u16_at(&bytes, 4)) {
            let record: usize = 12 + 16 * index;
            let rebased: u32 = u32::from_be_bytes(
                bytes[record + 8..record + 12]
                    .try_into()
                    .expect("four bytes"),
            ) + *start as u32;
            bytes[record + 8..record + 12].copy_from_slice(&rebased.to_be_bytes());
        }
        data.extend_from_slice(&bytes);
    }
    (data, starts)
}

fn tag_at(data: &[u8], offset: usize) -> &[u8] {
    &data[offset..offset + 4]
}

fn u16_at(data: &[u8], offset: usize) -> u16 {
    u16::from_be_bytes(data[offset..offset + 2].try_into().expect("two bytes"))
}

#[test]
fn a_face_carrying_both_kern_sources_loses_its_gpos_kern_feature() {
    // The whole point of the rewrite: with the GPOS `kern` feature out of the
    // shaper's reach, rustybuzz falls back to the legacy `kern` table, which
    // is the source PowerPoint positions the same glyphs from (issue #1116).
    let face = synthetic_face(&[b"kern", b"mark"], true);

    let patched: Vec<u8> = bytes_preferring_legacy_kern(&face.data, 0)
        .expect("a face carrying both kern sources must be rewritten");

    assert_ne!(
        tag_at(&patched, face.feature_tag_offsets[0]),
        b"kern",
        "the GPOS kern feature must no longer answer to its own tag"
    );
    assert_eq!(
        tag_at(&patched, face.feature_tag_offsets[1]),
        b"mark",
        "no other GPOS feature may be touched"
    );
    assert_eq!(
        patched.len(),
        face.data.len(),
        "the rewrite must preserve length so every table offset survives it"
    );
}

#[test]
fn the_renamed_feature_keeps_the_feature_list_alphabetical() {
    // OpenType states `FeatureRecord`s in alphabetical order, and a shaper may
    // binary-search them for a feature it did not reach through the language
    // system. A replacement tag that sorted elsewhere would break that search
    // for a neighbouring feature, so it has to land in the same slot.
    let face = synthetic_face(&[b"cpsp", b"kern", b"mark"], true);

    let patched: Vec<u8> =
        bytes_preferring_legacy_kern(&face.data, 0).expect("both sources are present");

    let tags: Vec<&[u8]> = face
        .feature_tag_offsets
        .iter()
        .map(|offset| tag_at(&patched, *offset))
        .collect();
    assert!(
        tags.windows(2).all(|pair| pair[0] <= pair[1]),
        "feature tags must stay ordered: {tags:?}"
    );
}

#[test]
fn the_renamed_feature_keeps_no_lookups() {
    // A `LangSys` may name a feature as *required*, in which case a shaper
    // applies it by index and never reads its tag. Emptying the lookup list
    // keeps such a face from kerning from both sources at once.
    let face = synthetic_face(&[b"kern", b"mark"], true);

    let patched: Vec<u8> =
        bytes_preferring_legacy_kern(&face.data, 0).expect("both sources are present");

    assert_eq!(
        u16_at(&patched, face.feature_lookup_count_offsets[0]),
        0,
        "the renamed feature must apply no lookups"
    );
    assert_eq!(
        u16_at(&patched, face.feature_lookup_count_offsets[1]),
        1,
        "another feature's lookups must survive"
    );
}

#[test]
fn the_legacy_kern_table_survives_the_rewrite() {
    // Renaming the GPOS feature is only half the answer: the values the shaper
    // falls back to have to still be there.
    let face = synthetic_face(&[b"kern"], true);
    let kern_start: usize = face.legacy_kern_start.expect("the fixture states one");

    let patched: Vec<u8> =
        bytes_preferring_legacy_kern(&face.data, 0).expect("both sources are present");

    assert_eq!(
        &patched[kern_start..],
        &face.data[kern_start..],
        "the legacy kern table's bytes must be untouched"
    );
}

#[test]
fn a_face_with_only_a_gpos_kern_feature_is_left_alone() {
    // Most modern faces state their pairs in GPOS alone. Taking that feature
    // away would leave them with no kerning at all, which is a worse answer
    // than the one this rule replaces.
    let face = synthetic_face(&[b"kern", b"mark"], false);

    assert!(
        bytes_preferring_legacy_kern(&face.data, 0).is_none(),
        "a face with one kern source has nothing to choose between"
    );
}

#[test]
fn a_face_with_only_a_legacy_kern_table_is_left_alone() {
    // The shaper already reads the legacy table when GPOS states no `kern`.
    let face = synthetic_face(&[b"mark"], true);

    assert!(bytes_preferring_legacy_kern(&face.data, 0).is_none());
}

#[test]
fn a_face_without_gpos_is_left_alone() {
    let mut face = synthetic_face(&[b"kern"], true);
    // Rename the GPOS record in the table directory, whose first record starts
    // at byte 12, so the face has no positioning table at all.
    face.data[12..16].copy_from_slice(b"JSTF");

    assert!(bytes_preferring_legacy_kern(&face.data, 0).is_none());
}

#[test]
fn a_collection_rewrites_the_face_the_index_names() {
    // A `.ttc` holds one table directory per face at an offset its header
    // states, and Typst asks for a face by that index. Rewriting the wrong
    // directory would leave the face actually in use unchanged.
    let one_source = synthetic_face(&[b"kern"], false);
    let two_sources = synthetic_face(&[b"kern"], true);
    let (collection, starts): (Vec<u8>, Vec<usize>) =
        synthetic_collection(&[&one_source, &two_sources]);
    let second_face_start: usize = starts[1];

    assert!(
        bytes_preferring_legacy_kern(&collection, 0).is_none(),
        "face 0 carries one kern source"
    );
    let patched: Vec<u8> =
        bytes_preferring_legacy_kern(&collection, 1).expect("face 1 carries both");
    assert_eq!(
        &patched[..second_face_start],
        &collection[..second_face_start],
        "the other face's bytes must be untouched"
    );
    assert_ne!(
        &patched[second_face_start..],
        &collection[second_face_start..],
        "the named face must be the one rewritten"
    );
}

#[test]
fn a_face_index_past_the_collection_is_left_alone() {
    let face = synthetic_face(&[b"kern"], true);
    let (collection, _): (Vec<u8>, Vec<usize>) = synthetic_collection(&[&face]);

    assert!(bytes_preferring_legacy_kern(&collection, 1).is_none());
}

#[test]
fn a_truncated_face_is_left_alone() {
    // Font data reaches this from the filesystem and from inside the document
    // being converted, so a malformed face has to fall back to the shaper's
    // own answer rather than panic.
    let face = synthetic_face(&[b"kern"], true);

    for length in 0..face.data.len() {
        assert!(
            bytes_preferring_legacy_kern(&face.data[..length], 0).is_none(),
            "a face truncated to {length} bytes must not be rewritten"
        );
    }
}

// ----- Real faces -----

#[test]
fn a_real_face_with_one_kern_source_keeps_its_gpos_pairs() {
    // The bundled CJK face states its pairs in GPOS alone, like most modern
    // faces, so the rule has to leave a real font of that shape untouched.
    let data: &[u8] = include_bytes!("../../fonts/NotoSansCJKsc-GB2312.otf");
    let font: Font = Font::new(Bytes::new(data.to_vec()), 0).expect("the bundled face parses");

    assert!(states_a_gpos_kern_feature(&font));
    assert!(face_preferring_legacy_kern(&font).is_none());
}

#[test]
fn a_real_face_carrying_both_sources_is_handed_over_kerning_from_the_legacy_table() {
    // The end of the chain, on a face a shaper can actually read: the face the
    // renderer hands Typst offers no GPOS `kern`, still carries the legacy
    // table, and is otherwise the same font.
    let base: &[u8] = include_bytes!("../../fonts/NotoSansCJKsc-GB2312.otf");
    let data: Vec<u8> = make_face_carrying_both_kern_sources(base);
    let font: Font = Font::new(Bytes::new(data), 0).expect("the rebuilt face parses");
    assert!(states_a_gpos_kern_feature(&font));
    assert!(font.ttf().tables().kern.is_some());

    let handed_over: Font =
        face_preferring_legacy_kern(&font).expect("the face carries both sources");

    assert!(
        !states_a_gpos_kern_feature(&handed_over),
        "the shaper must find no GPOS kern feature"
    );
    assert!(
        handed_over.ttf().tables().kern.is_some(),
        "the legacy table must be what is left to kern from"
    );
    assert_eq!(
        handed_over.ttf().number_of_glyphs(),
        font.ttf().number_of_glyphs(),
        "the rest of the face must be unchanged"
    );
    assert_eq!(handed_over.info().family, font.info().family);
}
