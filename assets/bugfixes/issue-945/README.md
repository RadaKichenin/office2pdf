# Issue #945 visual audit

The fixture is `tests/fixtures/docx/wasm_registered_cjk.docx` (SHA-256
`319b39fe1ddc570115d9794408c4c69e2015d66cd0c47f95c70af04f7a9f6d69`).
It contains one 18 pt Regular run, `Hello 中文测试文档`, declares SimSun, and
embeds no font.

`before.jpg` is a fresh default-feature Node WASM conversion from merge commit
`e220d548b64fb949308c115c24508c1269c1c197`. `after.jpg` is the same conversion
with only `wasm-cjk-font` enabled; no caller font or last-resort family was
provided. `gt.jpg` is the native embedded-only reference already established
for the same fixture in #944. It uses the related Noto Sans SC Regular face;
the feature output uses the documented Noto Sans CJK SC Regular GB2312 subset.

Both release modules were optimized by the same `wasm-pack`/`wasm-opt` run.
The default module is 44,739,647 bytes and the feature module is 48,215,568
bytes, an increase of 3,475,921 bytes. The bundled OTF itself is 3,511,684
bytes.

All pages were rendered before inspection with:

```sh
pdftoppm -r 150 -png -singlefile input.pdf output
magick output.png -crop 520x170+110+100 +repage crop.png
magick output.png -strip -interlace Plane -quality 86 output.jpg
magick compare -metric AE -fuzz 5% gt.jpg output.jpg diff.png
```

## Before-feature checklist

- Page count and order: one Letter page in both outputs; order matches.
- Element presence: the one text run is present, but its six Chinese
  characters are `.notdef` boxes.
- Position: the GT text bounds are `320x35+153+169`; before is
  `204x28+150+147`, 3 px left and 22 px high at 150 DPI.
- Size: the before text block is 116 px narrower and 7 px shorter because the
  fallback face has different metrics and no Chinese glyphs.
- Rotation and flip: none in either output.
- Fill: black text on a white page in both outputs.
- Stroke, border, and dash style: none.
- Text content: GT visibly renders and extracts `Hello 中文测试文档`; before
  displays six boxes and `pdftotext` drops `中`.
- Font family, weight, and style: GT embeds `NotoSansSC-Regular`; before embeds
  `LibertinusSerif-Regular`. The source run is Regular with no emphasis.
- Text color: black in both outputs.
- Alignment: both are left aligned, but the fallback changes the origin,
  baseline, and total advance.
- Line and paragraph spacing: one line and one paragraph; there is no
  inter-line or inter-paragraph spacing to compare.
- Clipping and overflow: none.

## Before-feature fine-detail pass

- The matched `520x170+110+100` crops were inspected side by side at full
  scale.
- Whole-page comparison against GT reports 6,470 differing pixels with 5%
  fuzz and RMSE `2961.15 (0.0451842)`. Every highlighted cluster belongs to
  the one text run.
- Hairline inventory: there are no rules, underlines, borders, tick marks, or
  other elements at or below 1 pt.
- Weight and emphasis inventory: the only run is Regular; there are no bold,
  italic, or underlined runs.

## After-feature checklist

- Page count and order: one Letter page in both outputs; order matches.
- Element presence: the complete Latin and Chinese run is present.
- Position: both text bounds are exactly `320x35+153+169` at 150 DPI.
- Size: width and height match exactly.
- Rotation and flip: none in either output.
- Fill: black text on a white page in both outputs.
- Stroke, border, and dash style: none.
- Text content: both visibly render and extract `Hello 中文测试文档`.
- Font family, weight, and style: GT uses Noto Sans SC Regular and the feature
  output embeds its documented Noto Sans CJK SC Regular subset. Both are
  Regular Noto Simplified Chinese faces; no emphasis changes.
- Text color: black in both outputs.
- Alignment: left edge, baseline, and total advance match exactly.
- Line and paragraph spacing: one line and one paragraph; spacing matches.
- Clipping and overflow: none.

## After-feature fine-detail pass

- The matched crop was re-inspected at full scale.
- Whole-page comparison reports 66 differing pixels with 5% fuzz. Without
  fuzz it reports 5,216 edge pixels, but RMSE is only
  `68.2793 (0.00104188)`; the clusters are the expected antialiasing-level
  differences between the related Noto SC and Noto CJK SC faces. Geometry and
  visible glyph shapes match.
- Hairline inventory: no thin elements exist in the fixture.
- Weight and emphasis inventory: the one Regular run has no bold, italic, or
  underline to preserve.
- No visible deviation remains, so this comparison requires no follow-up
  issue reference.

The result-bearing API reports `fallback-used`, `from="SimSun"`,
`to="Noto Sans CJK SC"`. The default build still reports `to=".notdef"`, which
confirms that the font and its size cost remain opt-in.
