# PowerPoint kerning across spaces

Latin PowerPoint runs now retain the active font's pair adjustments between
spaces and their neighboring characters. The adjustment shares the existing
weak 1/8pt advance-grid correction, so it disappears at line boundaries.
Words keep their internal shaping and spaces remain searchable text.

## Native measurements

Source: `tests/golden_mocks/business/sources/pptx/03_product_launch_en.pptx`,
slide 2. Native PowerPoint 16.112.3 and the converter use the same Arial
faces from PowerPoint's font directory. The second bullet is Arial 15pt.

| Letter after `on ` | Before origin drift | After origin drift |
| --- | ---: | ---: |
| Original `Aug` | +0.834pt | +0.006pt |
| `Hug`, `Vug`, `Wug` controls | +0.005pt | +0.005pt |
| `Yug` control | +0.275pt | +0.004pt |

Drift is the output glyph origin minus the native origin. The original A
moves from 227.625pt to 226.79737pt; native is 226.790998pt. Arial's legacy
space/A adjustment is -113/2048em; space/Y is -37/2048em. Native `A Aug`
probes also expose the pair before the space, requiring both adjustments.

At 88.5pt box width, native keeps `on Aug` together while the old output
split it. The same defect splits native `A Aug` at 81, 81.5, and 82pt.
All twenty-nine candidate probe exports now match native line contents;
checked word origins differ by at most 0.011pt horizontally and 0.12pt
vertically. The 80pt controls wrap before Aug and keep its native left origin.
Every native export passes the integrity gate; every re-zip control is
layout-identical.

Two portable regressions attach space-pair entries to the bundled Noto face,
covering two sizes, kerning disabled, either/both sides of a space, repeated
spaces, a no-pair control, wrapping and new-line origins. Eight width cases
and the wrap regression fail before the fix; both tests pass after it.

## Evidence collection

The JPEGs stack sixteen full 2000 × 1125 pages at 150 DPI. Before uses the
implementation merged at `39111d5b82861b3f9fd3d2d46d4bd49a83628a82`; after
uses this fix. All images use pdftoppm and progressive JPEG quality 86,
with metadata stripped before restoring 150 DPI. The original issue-report
`compare.jpg` is retained.

| Collection pages | Native source pages |
| --- | --- |
| 1–3 | Original product deck, slides 1–3 |
| 4–7 | H, V, W, Y letter controls, slide 2 |
| 8–9 | `on Aug 20`, widths 120 and 80pt, slide 2 |
| 10–11 | `A Aug 20`, widths 120 and 80pt, slide 2 |
| 12–13 | `on Aug 20`, widths 88 and 88.5pt, slide 2 |
| 14–16 | `A Aug 20`, widths 81, 81.5 and 82pt, slide 2 |

The 0.5pt layout audit reports no missing/extra or rewrapped text,
painted-visibility, fill-occlusion, rectangle-geometry or position failures.
Searchable text matches: 2,337 normalized codepoints, 246 spaces and sixteen
control characters on each side. Full-scale crops confirm corrected suffix
spacing and wraps while preserving glyph shape, weight, bullets and indents.

Pages 1, 3, 4, 5, 6, 9, 11 and 12 have identical decoded GT/output/diff pixels
to the before audit. The other pages preserve the same upper heading and
timeline pixels. The chart's fine axes, ticks, blue/gold bars and legend
outlines are unchanged. One material gold legend-edge resampling cluster
remains on page 3, with its exact ID dispositioned as `photo-resampling`.
Sparse body glyph-edge fragments are bounded observations. No remaining
converter deviation was found in this comparison.

## Reproduction

Run this snippet from the repository root to generate five probe specs:

```python
import json, zipfile
import xml.etree.ElementTree as ET
from pathlib import Path

source = Path('tests/golden_mocks/business/sources/pptx/03_product_launch_en.pptx').resolve()
output = Path('target/issue-1582-reproduction').resolve()
output.mkdir(parents=True, exist_ok=True)
part = 'ppt/slides/slide2.xml'
ns = {'p': 'http://schemas.openxmlformats.org/presentationml/2006/main',
      'a': 'http://schemas.openxmlformats.org/drawingml/2006/main'}
for prefix, uri in ns.items():
    ET.register_namespace(prefix, uri)
letters = {'name': 'issue-1582-letters-reproduction', 'base': str(source),
           'part': part, 'page': 2, 'factor': 'initial letter after the space',
           'variants': [{'value': letter, 'find': 'Aug 20',
                         'replace': f'{letter}ug 20', 'count': 1}
                        for letter in ['H', 'V', 'W', 'Y']]}
(output / 'letters.json').write_text(json.dumps(letters, indent=2) + '\n')
for kind, text in [('right', 'on Aug 20'), ('both', 'A Aug 20')]:
    with zipfile.ZipFile(source) as archive:
        tree = ET.fromstring(archive.read(part))
        shape = next(s for s in tree.findall('.//p:sp', ns)
                     if any('All hands' in (t.text or '') for t in s.findall('.//a:t', ns)))
        body = shape.find('p:txBody', ns)
        chosen = next(p for p in body.findall('a:p', ns)
                      if any('All hands' in (t.text or '') for t in p.findall('.//a:t', ns)))
        for paragraph in body.findall('a:p', ns):
            if paragraph is not chosen:
                body.remove(paragraph)
        chosen.find('a:r/a:t', ns).text = text
        shape.find('p:spPr/a:xfrm/a:ext', ns).set('cx', str(120 * 12700))
        base = output / f'wrap-{kind}.pptx'
        with zipfile.ZipFile(base, 'w') as target:
            for entry in archive.infolist():
                payload = (ET.tostring(tree, encoding='utf-8', xml_declaration=True)
                           if entry.filename == part else archive.read(entry.filename))
                target.writestr(entry, payload)
    for series, widths in [('wrap', [80, 89, 89.5, 90, 100]),
                           ('edge', [88, 88.5] if kind == 'right' else [81, 81.5, 82])]:
        spec = {'name': f'issue-1582-{series}-{kind}-reproduction', 'base': str(base),
                'part': part, 'page': 2, 'factor': 'textbox width in points',
                'variants': [{'value': str(width), 'find': f'cx="{120 * 12700}"',
                              'replace': f'cx="{int(width * 12700)}"', 'count': 1}
                             for width in widths]}
        (output / f'{series}-{kind}.json').write_text(json.dumps(spec, indent=2) + '\n')
```

For each spec run `python3 scripts/probe_harness.py SPEC.json --backend office`.
The harness exports native base, re-zip control and variants inside PowerPoint's
sandbox, rejecting a changed control. Check every native PDF with
`python3 scripts/check_gt_integrity.py NATIVE.pdf --source SOURCE.pptx --json`.

Build each implementation with `cargo build --locked -p office2pdf-cli` and
preserve its binary separately. Convert with
`office2pdf SOURCE.pptx -o OUTPUT.pdf --font-path FONT_DIRECTORY`.
The measured font directory is PowerPoint.app's `Contents/Resources/DFonts`.
Use the full original deck, and `--slides 2` for probes; extract native slide
2 for each probe. Assemble the collection in the table's order with `mutool merge`.

Run `python3 scripts/compare_layout.py GT.pdf AFTER.pdf --json --audit --fine-shift 0.5`
and `python3 scripts/compare_text_layer.py GT.pdf AFTER.pdf --json`.
For every page run `python3 scripts/compare_render.py GT.pdf AFTER.pdf --page N --dpi 150 --fine-shift 0.5 --artifacts-dir DIRECTORY --cluster-report PATH`.
Inspect the full pages, matched crops and 5% pixel sweep, then rerun with
`--cluster-dispositions PATH --strict-clusters`. Only page 3 needs a material
cluster disposition; its gold legend boundary retains its full outline and
fill, with resampled edge pixels differing.

## SHA-256 provenance

- gt.pdf: `b8492b78fe4cb9b98e2be8c2c67d4081ccce6bcd4647c10b1455b764b5c8d4a8`
- before.pdf: `0e01114f8ad76c4af7ebde9da3ba6675d8997cbe7c52b0f53992cff33c22a4a3`
- after.pdf: `80ca91db3607564326321fc839aeea6bb121f4164a86be9a4e0dd84835fcf66e`
- after-cli: `204e5a0a907ccc54674d4f1cd507242c7d79fc3a3033efce40f0236fb28a692d`
- Original product PPTX: `2470f5ca6abd44b1b15113aa3028e625738b73f619da3fb16892f5d561b4d7aa`

| Collection page | Source package SHA-256 |
| --- | --- |
| 4 | `95d2ddaeb2fc2f98bfc5fb73937a7a197db43194a52f5cc1b3e54a4877926f0c` |
| 5 | `95e57de16bdebbe965bd1c2016a9c3c6744975108cff554d2f4ab658ecc0976c` |
| 6 | `7826f14fa35781cba85c7c430eac9ec1be0d55ac4d9f357e697d7eedd789e122` |
| 7 | `46c59fd983d3e81cb71264fa6ea0569c4462eca227b781512a26aa0022c32f29` |
| 8 | `d16650c7a8ba64dcc76967d7c49292417572203a29315057458fa9d905c34155` |
| 9 | `743da222ce018f4290bba6c212c35d6fc5778f36d68b86693766eb7fd8fdd7f8` |
| 10 | `95b21abd134980ee55514f358032ca420baf124119577657af8b6530e9625001` |
| 11 | `814828a33828ba6f2026330409539fa704f4d022335559bdbd8f9e173f4cf62c` |
| 12 | `21a17f157b76ba51d4f07b369bc01fa738c94f428b1729f0062eb74933172a54` |
| 13 | `713f6e4cc315813db7b29aed73ca871220e58d4797d9b54669ee10483a5b0087` |
| 14 | `d9f99891978456609dbcc6850deff46168bec0cb1eb1fc4a0959c38bb1d4496f` |
| 15 | `25ca678d916645efff8998a5d60cac97691e3f94e055760377422cd7e9eba91d` |
| 16 | `be0e24cfa83d45d083a2c076da83dfcfcbb7880df3fc31e44a7f73efe221b4c8` |
