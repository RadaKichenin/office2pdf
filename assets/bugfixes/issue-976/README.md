# Issue #976 visual audit

The source is the public `002.CONTOSO.pptx` attachment from issue #841
(SHA-256 `0f225bb743f382926a4758091ff49e1759520dc396bbdbddf10061921afd056a`).
Slide 11 contains a `p:pic` named `Kamera 9`, described as `Cameo-objekt`,
with Office 2021 `alf:liveFeedProps`. Its frame is `6382512 x 6858000`
EMU (`502.56 x 540 pt`), while its selected SVG declares `width="321"`,
`height="181"`, and `viewBox="0 0 321 181"`.

`compare.jpg` records the defect as filed: native PowerPoint is left and the
then-current office2pdf output is right. `gt.jpg` comes from a local Microsoft
PowerPoint 16.111.3 PDF export. Its producer is macOS Quartz, it contains all
18 source slides, and `scripts/check_gt_integrity.py` found no structural
corruption. `before.jpg` is a fresh merged-`main` baseline after #956;
`after.jpg` is the issue #976 fix. The Cameo defect is unchanged between the
filing-time output and that fresh baseline. All fix evidence selects slide 11,
was rasterized by `pdftoppm` at 150 DPI to `2000x1125`, and was stored as
progressive JPEG quality 86 with metadata stripped and 150-DPI density restored.

## Before-fix checklist

- Page count and order: both complete exports contain 18 slides; slide 11 is
  compared with slide 11.
- Element presence: every heading, bullet item, footer, label, decorative wave,
  and Cameo picture is visible.
- Position: the Cameo frame starts at the expected right-side origin and fills
  the expected slide height.
- Size: the outer frame is `502.56 x 540 pt`, but the 321:181 SVG viewport is
  stretched into that 0.9307:1 frame. The head and body are visibly too narrow.
- Rotation and flip: none is declared or observed on the Cameo picture.
- Fill: the gray background and white person fill are present.
- Stroke, border, and dash style: the solid dark person outline is present; no
  issue-specific border or dash-style loss is observed.
- Text content: all slide text is visible; this image defect does not alter it.
- Font family, weight, and style: installed-font substitutions between the
  native and office2pdf PDFs are unrelated to the image deformation.
- Text color: no issue-specific text-color difference is observed.
- Alignment: the SVG is centered, but its non-uniform scale is wrong.
- Line and paragraph spacing: the inherited gap fixed by #956 is present. The
  remaining sub-point PowerPoint quantization difference is tracked in #665.
- Clipping and overflow: the custom outer clip is present, but office2pdf
  stretches the SVG before clipping instead of cover-cropping its artwork.

## After-fix checklist

- Page count and order: unchanged and matches the native export.
- Element presence: unchanged; no picture or text content is added or lost.
- Position: the frame origin and extent remain fixed.
- Size: fixed. The Cameo SVG is centrally cropped to
  `viewBox="76.275 0 168.451 181"`, matching the frame aspect ratio before
  scaling. The head and body now match PowerPoint's natural proportions.
- Rotation and flip: unchanged; none on the Cameo picture.
- Fill: matches GT apart from renderer antialiasing at vector edges.
- Stroke, border, and dash style: the solid person outline and custom outer
  boundary match GT; the 5%-fuzz diff leaves only edge antialiasing clusters.
- Text content: unchanged and visible. PowerPoint flattens the sensitivity
  label in its PDF text layer, so office2pdf has one extra selectable line but
  no visual text gain.
- Font family, weight, and style: unchanged apart from accepted installed-font
  substitutions; the image fix does not touch text.
- Text color: unchanged.
- Alignment: fixed. The preserved-aspect artwork remains centered in the frame.
- Line and paragraph spacing: unchanged; the existing #665 residual remains.
- Clipping and overflow: fixed. The SVG is cover-cropped before the custom
  picture geometry is mapped, so the geometry spans the full frame instead of
  being cropped together with the artwork.

## Fine-detail pass

- The full 150-DPI pages and matched `1000x1125+1000+0` Cameo-region crops
  were inspected side by side at full scale.
- On crops decoded from the stored JPEG evidence, the 5%-fuzz difference falls
  from 213,137 pixels before the fix to 3,771 after it. RMSE falls from
  7,100.27 to 660.503.
- On the stored full-page JPEG evidence, the 5%-fuzz difference falls from
  261,805 pixels before the fix to 52,421 after it. RMSE falls from 6,205.29 to
  3,676.48. The remaining highlighted clusters were inspected and are outside
  the Cameo artwork: installed-font/vector rasterization and antialiasing.
- Hairline inventory: no rule, border, underline, tick mark, or other element
  at or below 1 pt was identified. The thicker wave and silhouette strokes are
  present with their solid style.
- Weight and emphasis inventory: the title and `Kostnad`, `Logistikk`, and
  `Vekst` labels are bold; bullet items are regular. No italic or underlined run
  is visible, and the after image preserves the inventory.
