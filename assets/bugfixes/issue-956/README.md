# Issue #956 visual audit

The source is the public `002.CONTOSO.pptx` attachment from issue #841
(SHA-256 `0f225bb743f382926a4758091ff49e1759520dc396bbdbddf10061921afd056a`).
This audit isolates slide 11, whose content placeholders contain bare
`<a:pPr rtl="0"/>` paragraphs. The matching layout placeholder declares
`<a:spcBef><a:spcPts val="400"/></a:spcBef>` and
`<a:spcAft><a:spcPts val="0"/></a:spcAft>` at its first list level. The
master underneath it declares 10 pt before and 12 pt after, so the layout's
4 pt and 0 pt are the effective values.

`gt.jpg` comes from a local Microsoft PowerPoint 16.111.3 PDF export. Its PDF
producer is macOS Quartz, it contains all 18 source slides, and
`scripts/check_gt_integrity.py` found no structural corruption. `before.jpg`
and `after.jpg` come from office2pdf before and after the parser change. All
three select slide 11 and were rasterized with `pdftoppm` at 150 DPI to the
same `2000x1125` pixels, then stored as progressive JPEG quality 86 with
metadata stripped and 150-DPI density restored.

## Before-fix checklist

- Page count and order: both complete exports contain 18 slides; slide 11 is
  compared with slide 11.
- Element presence: all headings, seven bullet items, the slide number, footer,
  decorative wave, sensitivity label, and Cameo picture are visible.
- Position: the issue-specific error is vertical. The first `Kostnad` item
  starts at 168.00027 pt, followed at 184.80030 and 201.60028 pt: exactly
  16.8 pt apart instead of PowerPoint's 21.12 and 20.88 pt advances.
- Size: the affected runs remain at their inherited 14 pt size; no
  issue-specific size loss is observed.
- Rotation and flip: none on the affected text.
- Fill: affected text and bullet fills are present.
- Stroke, border, and dash style: the bullets are text glyphs, not stroked
  shapes; the affected list regions contain no border or dashed rule.
- Text content: all affected strings are visible and extract intact.
- Font family, weight, and style: the list runs resolve consistently within
  each export. PowerPoint embeds Posterama, Avenir Next LT Pro, and Courier New
  on this page while office2pdf uses locally available substitutes; those
  accepted availability differences do not cause the missing paragraph gap.
- Text color: all affected items are white as declared by their inherited
  style.
- Alignment: bullet and text origins are consistent across the three groups;
  no issue-specific horizontal displacement is observed.
- Line and paragraph spacing: office2pdf advances every item by 16.8 pt and
  omits the layout's additional 4 pt `spcBef`.
- Clipping and overflow: none in the affected list regions.

## After-fix checklist

- Page count and order: unchanged and matches the native export.
- Element presence: unchanged; no content was added or lost.
- Position: the first `Kostnad` item stays at 168.00027 pt. The next two move
  to 188.80030 and 209.60028 pt, giving the intended 20.8 pt item pitch.
  `Logistikk` likewise advances by 20.8 pt, and a wrapped `Vekst` item keeps
  its 16.8 pt within-item line pitch before the next item receives the gap.
- Size: unchanged; the inherited 14 pt run size is preserved.
- Rotation and flip: unchanged; none on the affected text.
- Fill: unchanged and present.
- Stroke, border, and dash style: unchanged; no affected hairlines or borders.
- Text content: unchanged and extract intact. PowerPoint flattens the visible
  sensitivity label in its PDF text layer, so that selector-level difference
  is an accepted native-export encoding difference rather than content loss.
- Font family, weight, and style: unchanged apart from the accepted
  cross-renderer font substitutions described above.
- Text color: unchanged and matches the declared white list style.
- Alignment: horizontal bullet and text origins remain stable.
- Line and paragraph spacing: fixed. The missing 4 pt inherited gap is now
  applied. PowerPoint's alternating 21.12/20.88 pt quantization versus the
  output's stable 20.8 pt leaves a 0.32-0.40 pt cumulative residual, remaining
  and tracked in #665.
- Clipping and overflow: no affected-list clipping or overflow. The independent
  Cameo SVG aspect-ratio deformation visible at right remains and is tracked in
  #976.

## Fine-detail pass

- The 150-DPI full pages and matched `700x650+250+280` list-region crops were
  inspected side by side at full scale.
- At 5% fuzz, matched crops cut from the stored JPEG evidence improve from
  35,089 differing pixels before the fix to 30,293 after it; RMSE improves
  from 6,832.77 to 6,343.72. Whole-page differences remain dominated by
  accepted font substitution and the separate Cameo defect in #976.
- Text-layout matching improves from 2.53 pt median absolute vertical error
  before the fix to 1.10 pt after it; the worst matched pitch error improves
  from 8.40 pt to 0.60 pt.
- Hairline inventory: no rule, underline, border, tick mark, or other element
  at or below 1 pt occurs in the affected list regions.
- Weight and emphasis inventory: `Kostnad`, `Logistikk`, and `Vekst` are bold;
  the affected bullet items are regular. There are no italic or underlined
  runs in those regions, and the after image preserves this inventory.
