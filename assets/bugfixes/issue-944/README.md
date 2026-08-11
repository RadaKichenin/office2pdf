# Issue #944 visual audit

The fixture is `tests/fixtures/docx/wasm_registered_cjk.docx` (SHA-256
`319b39fe1ddc570115d9794408c4c69e2015d66cd0c47f95c70af04f7a9f6d69`).
It contains one 18 pt regular run, `Hello 中文测试文档`, declares SimSun for its
Latin and East Asian slots, and deliberately embeds no font. The registered
font is the same OFL-licensed Noto Sans SC Regular subset already attributed by
the #943 fixture; its deobfuscated SHA-256 is
`11aa3b851dbc6ff32184983acd5eb2d229feb16789eea9d02a602748f2b644a2`.

`before.jpg` is the browser-WASM conversion from merge commit
`84f2fa4f6adddde842bd5c9fadc24fb8f25647a1`, using the only API that existed
there, `convertDocxToPdf`. `after.jpg` uses `Office2PdfConverter.registerFont`
and `setLastResortFontFamily("Noto Sans SC")`. `gt.jpg` is a native
embedded-only compilation of the same parsed document, in-memory face, font
context, and generated Typst source. The after and reference PDFs have the
same SHA-256,
`bd931636b76ee7df2150e5d6d88c0e1df103c5ae6419fe31ae8d07b9be7ce9dc`,
and their lossless 150 DPI renders are pixel-identical.

All three PDFs were rendered before inspection with:

```sh
pdftoppm -r 150 -png -singlefile input.pdf output
magick input.png -crop 520x170+110+100 +repage crop.png
magick compare -metric AE -fuzz 5% gt.png output.png diff.png
```

## Before-fix checklist

- Page count and order: one Letter page in both outputs; order matches.
- Element presence: the single run is present, but the six CJK characters are
  `.notdef` boxes in WASM.
- Position: the reference text bounds are `320x35+153+169`; before is
  `204x28+150+147`, 3 px left and 22 px high at 150 DPI.
- Size: the before text block is 116 px narrower and 7 px shorter because it
  uses the wrong glyph advances and metrics.
- Rotation and flip: none in either output.
- Fill: black text on a white page in both outputs.
- Stroke, border, and dash style: none.
- Text content: the reference visibly renders and extracts
  `Hello 中文测试文档`; before displays six boxes and `pdftotext` drops `中`.
- Font family, weight, and style: the reference embeds
  `NotoSansSC-Regular`; before embeds `LibertinusSerif-Regular`. The source run
  is regular with no emphasis.
- Text color: black in both outputs.
- Alignment: both are left aligned, but the fallback changes the starting
  position, baseline, and total advance.
- Line and paragraph spacing: one line and one paragraph; there is no
  inter-line or inter-paragraph spacing to compare.
- Clipping and overflow: none.

## Before-fix fine-detail pass

- The matched `520x170+110+100` crops were inspected side by side at full
  scale.
- On the lossless 150 DPI PNGs, whole-page comparison reports 6,930 differing
  pixels without fuzz and 6,474 with `-fuzz 5%`; every highlighted cluster is
  confined to the one text run. RMSE is `2972.28 (0.0453541)`.
- Hairline inventory: the fixture contains no rules, underlines, borders,
  tick marks, or other elements at or below 1 pt.
- Weight and emphasis inventory: the only run is regular; there are no bold,
  italic, or underlined runs.

## After-fix checklist

- Page count and order: one Letter page in both outputs; order matches.
- Element presence: the complete Latin and CJK run is present.
- Position: both text bounds are exactly `320x35+153+169` at 150 DPI.
- Size: width and height match exactly.
- Rotation and flip: none in either output.
- Fill: black text on a white page in both outputs.
- Stroke, border, and dash style: none.
- Text content: both visibly render and extract `Hello 中文测试文档`.
- Font family, weight, and style: both embed the same subsetted
  `NotoSansSC-Regular` face; the run remains regular.
- Text color: black in both outputs.
- Alignment: left edge, baseline, and glyph advances match exactly.
- Line and paragraph spacing: one line and one paragraph; spacing matches.
- Clipping and overflow: none.

## After-fix fine-detail pass

- The matched crop was re-inspected at full scale.
- Lossless whole-page AE and RMSE are both exactly zero. The normalized
  progressive JPEGs stored here are also byte-identical for `gt.jpg` and
  `after.jpg` (SHA-256
  `cdffceb550af2a9ff7d59ab2a6dc2f587959f79fcc3196ecb68fb4458c5048b4`).
- Hairline inventory: no thin elements exist in the fixture.
- Weight and emphasis inventory: the one regular run has no bold, italic, or
  underline to preserve.
- No visible deviation remains, so this comparison requires no follow-up
  issue reference.

The result-bearing JavaScript API also reports the structured warning
`fallback-used`, `from="SimSun"`, `to="Noto Sans SC"`. Running the same fixture
without registration reports `to=".notdef"`, so a caller can detect the
degraded output without rasterising the PDF.
