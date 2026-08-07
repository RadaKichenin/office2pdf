# PPTX Font Resolution and Text Style Inheritance

## Summary

This note documents a PPTX fidelity issue that initially looked like a layout bug
but was actually caused by incomplete font and text-style resolution.

The visible symptom was that text blocks on several slides, especially the
slide 4 `SKILLS` area, did not match PowerPoint:

- narrow labels wrapped differently
- title badges had different sizing and weight
- some text appeared visually shifted even when box coordinates were correct

The root cause was not hardcoded slide geometry. The real issue was that
`office2pdf` did not resolve PPT text styling the way PowerPoint does.

## Problem

PowerPoint text formatting is not determined only by direct run properties.
For a text box, effective styling can come from multiple OOXML layers:

1. direct run properties in `a:rPr`
2. paragraph properties in `a:pPr`
3. text body defaults in `a:lstStyle`
4. default paragraph properties in `a:defPPr`
5. level-specific paragraph properties in `a:lvlNpPr`
6. default run properties in `a:defRPr`
7. theme font references such as `+mj-lt` / `+mn-lt`

Before the fix, the PPTX parser mostly used direct `a:rPr` and direct
`a:latin` values on runs. That was incomplete.

As a result:

- runs with missing direct font settings lost their inherited family, size,
  color, and weight
- text boxes using `lstStyle` defaults rendered with weaker styling than
  PowerPoint
- line wrapping diverged because the effective font metrics were wrong

Fallback selection also depends on where an available substitute came from.
After the requested family, candidates are ranked as Office-managed fonts,
user-provided font paths, other available fonts, and unavailable names. The
static substitution-table order breaks ties within the same source rank.

## Why PowerPoint Looked Correct

The affected deck does not rely only on theme fonts. It contains explicit
typeface information such as `Pretendard`, `Pretendard SemiBold`, and related
variants in slide XML.

PowerPoint applies:

- direct run formatting
- inherited `lstStyle` defaults
- script-aware typeface resolution across `latin`, `ea`, and `cs`
- local fallback behavior when the exact face is unavailable

`office2pdf` was missing part of that chain, so the generated PDF diverged
even though the original PPTX rendered correctly in PowerPoint.

## Fix

### 1. Parse text body style inheritance

Added a text-body default model in the PPTX parser so a text box can inherit
style from `a:lstStyle`.

Implementation:

- introduced `PptxTextBodyStyleDefaults`
- introduced `PptxTextLevelStyle`
- parse `a:lstStyle`, `a:defPPr`, `a:lvlNpPr`, and nested `a:defRPr`
- merge inherited paragraph and run styles before applying direct paragraph/run
  overrides
- apply typeface from `a:latin`, `a:ea`, and `a:cs`, not only direct `a:latin`

This makes text boxes behave much closer to PowerPoint's effective style model.

### 2. Rank available fallback sources

Fallback selection for known families such as `Pretendard` ranks available
substitutes by source first, then preserves the substitution table's order
within each source rank.

That means:

- Office-managed fonts outrank user-provided and ordinary system fonts
- user-provided fonts outrank ordinary system fonts
- preferred substitute order remains stable among candidates from the same
  source

## Files Involved

- `crates/office2pdf/src/parser/pptx.rs`
- `crates/office2pdf/src/render/font_subst.rs`
- `crates/office2pdf/src/render/typst_gen.rs`

## Verification

The fix was verified in three ways:

1. unit tests for `lstStyle` default run inheritance
2. unit tests for fallback ordering under a mixed Office/system font context
3. reconversion of the real PPTX deck and manual visual inspection

Observed result after the text-inheritance fix:

- slide 4 `SKILLS` labels no longer wrap as before
- badge/title styling is closer to PowerPoint
- the resolved fallback now follows the available font source priority rather
  than assuming one machine-specific family

## Key Lesson

When a PPTX rendering mismatch looks like a textbox position problem, do not
assume the coordinates are wrong first. In PowerPoint, incorrect font
resolution often becomes a layout problem:

- different font family
- different weight
- different inherited size
- different line spacing

Those differences change wrapping and visual alignment even if the text box
geometry is correct.
