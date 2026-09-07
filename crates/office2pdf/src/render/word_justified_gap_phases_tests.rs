use super::*;

/// Malgun Gothic at 10.5pt, the face and size of
/// `tests/fixtures/docx/korean_alignment_autospace.docx`: a 3.695pt word
/// space, a 2.625pt quarter-em auto space, and a 5.25pt half-em ceiling.
fn malgun_line(word_spaces: usize, auto_spaces: usize) -> Vec<GapBudget> {
    let word = GapBudget {
        kind: GapKind::WordSpace,
        natural: 3.695,
        ceiling: 5.25,
    };
    let auto = GapBudget {
        kind: GapKind::AutoSpace,
        natural: 2.625,
        ceiling: 5.25,
    };
    // Interleave them the way the fixture's lines do, so the order of the
    // answer is the order of the gaps and not of their kinds.
    let mut gaps: Vec<GapBudget> = Vec::new();
    for index in 0..word_spaces.max(auto_spaces) {
        if index < word_spaces {
            gaps.push(word);
        }
        if index < auto_spaces {
            gaps.push(auto);
        }
    }
    gaps
}

fn assert_close(actual: f64, expected: f64, what: &str) {
    assert!(
        (actual - expected).abs() < 1e-9,
        "{what}: expected {expected}, got {actual}"
    );
}

/// Phase 1, the fixture's paragraph 4 line 2: 3.05pt over 12 word spaces and
/// 4 auto spaces. Word's export puts the word spaces at 3.9493pt and leaves
/// the auto spaces at 2.6240pt — the demand goes to the word spaces alone.
#[test]
fn a_demand_under_the_word_space_room_goes_to_the_word_spaces_alone() {
    let gaps = malgun_line(12, 4);
    let widths = spread_demand_as_word_does(&gaps, 3.05);

    for (gap, width) in gaps.iter().zip(&widths) {
        match gap.kind {
            GapKind::WordSpace => assert_close(*width, 3.695 + 3.05 / 12.0, "word space"),
            GapKind::AutoSpace => assert_close(*width, 2.625, "auto space"),
        }
    }
}

/// Phase 2: the word spaces sit pinned at half an em and the auto spaces
/// take what is left, equally.
#[test]
fn a_demand_past_the_word_space_room_spills_into_the_auto_spaces() {
    let gaps = malgun_line(12, 4);
    let word_room: f64 = 12.0 * (5.25 - 3.695);
    let widths = spread_demand_as_word_does(&gaps, word_room + 2.0);

    for (gap, width) in gaps.iter().zip(&widths) {
        match gap.kind {
            GapKind::WordSpace => assert_close(*width, 5.25, "word space"),
            GapKind::AutoSpace => assert_close(*width, 2.625 + 0.5, "auto space"),
        }
    }
}

/// Phase 3: past every ceiling, every gap takes the same extra, which at one
/// size is Word's one common width (issue #1053).
#[test]
fn a_demand_past_every_ceiling_levels_every_gap() {
    let gaps = malgun_line(12, 4);
    let room: f64 = 12.0 * (5.25 - 3.695) + 4.0 * (5.25 - 2.625);
    let widths = spread_demand_as_word_does(&gaps, room + 1.6);

    for width in &widths {
        assert_close(*width, 5.35, "every gap");
    }
}

/// Whatever the phase, the line stays as wide as Typst made it.
#[test]
fn the_spread_conserves_the_line_width() {
    let gaps = malgun_line(5, 3);
    let natural: f64 = gaps.iter().map(|gap| gap.natural).sum();
    for demand in [0.0, 0.4, 7.7, 14.0, 22.0, 40.0] {
        let total: f64 = spread_demand_as_word_does(&gaps, demand).iter().sum();
        assert_close(total, natural + demand, &format!("demand {demand}"));
    }
}

/// Word spaces of two sizes have different room: the 9pt one reaches its
/// half em first and the 12pt one takes the rest, rather than both taking
/// an equal share that overshoots the smaller ceiling.
#[test]
fn a_word_space_at_its_ceiling_hands_the_rest_to_the_others() {
    let gaps = [
        GapBudget {
            kind: GapKind::WordSpace,
            natural: 9.0 * 0.352,
            ceiling: 4.5,
        },
        GapBudget {
            kind: GapKind::WordSpace,
            natural: 12.0 * 0.352,
            ceiling: 6.0,
        },
        GapBudget {
            kind: GapKind::AutoSpace,
            natural: 2.25,
            ceiling: 4.5,
        },
    ];
    let widths = spread_demand_as_word_does(&gaps, 3.0);

    assert_close(widths[0], 4.5, "the 9pt space stops at its ceiling");
    assert_close(
        widths[1],
        12.0 * 0.352 + 3.0 - (4.5 - 9.0 * 0.352),
        "the 12pt space",
    );
    assert_close(widths[2], 2.25, "the auto space waits for phase 2");
}

/// A line of nothing but auto spaces has no phase 1: the demand goes to
/// them directly.
#[test]
fn a_line_without_word_spaces_stretches_its_auto_spaces_from_the_start() {
    let gaps = malgun_line(0, 3);
    let widths = spread_demand_as_word_does(&gaps, 1.2);

    for width in &widths {
        assert_close(*width, 2.625 + 0.4, "auto space");
    }
}
