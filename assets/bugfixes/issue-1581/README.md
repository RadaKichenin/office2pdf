# PowerPoint Latin mathematical-symbol advances

A `×`, `÷`, or `±` inside an otherwise Latin run no longer disables
PowerPoint's 1/8pt nominal advance rounding for the entire run. Words retain
the existing whole-word shaping and kerning treatment, and spaces and ASCII
hyphens retain their break opportunities. Explicit tracking and other scripts
retain their existing handling.

## Measurements

The original source is
`tests/golden_mocks/business/sources/pptx/08_marketing_report_en.pptx`, slide 3.
The first bullet is one Arial 17pt run. Native PowerPoint 16.112.3 and the
converter use the same Arial faces from PowerPoint's font directory.

| First-bullet sign | Before end drift | After end drift |
| --- | ---: | ---: |
| Multiplication `×` | -1.073pt | +0.018pt |
| Division `÷` | -1.168pt | +0.018pt |
| Plus-minus `±` | -1.168pt | +0.018pt |
| ASCII `x` control | +0.023pt | +0.023pt |

Drift is the output's final painted glyph advance endpoint minus the native
endpoint. The original final period's endpoint moves from 526.2429pt to
527.3338pt; native is 527.3157pt. All fifteen width/control probes preserve
native line contents, including numeric expressions and hyphen breaks;
measured probe baselines agree within 0.12pt. Native re-zip controls are
layout-identical and every probe export passes the GT integrity check.

Two portable tests use embedded fonts: one measures the following run's
position for all three signs and an ASCII control in two font families;
the other checks numeric-expression, space, and hyphen wrapping. The width
test fails for all six mathematical-sign cases before this fix.

## Evidence collection

The JPEGs stack seventeen full 2000 × 1125 pages vertically at 150 DPI.
Before is the verified main implementation at
`391c1d1d08940a7d2038d3529d6afe8033d608eb`; after uses this fix.
All three images use pdftoppm, progressive JPEG quality 86, metadata stripped
before restoring 150 DPI. The original issue-report `compare.jpg` is retained.

| Collection pages | Native source pages |
| --- | --- |
| 1–3 | Original marketing deck, slides 1–3 |
| 4 | Original first bullet with `×` replaced by `÷`, slide 3 |
| 5 | Original first bullet with `×` replaced by `±`, slide 3 |
| 6–9 | Multiplication wrap probe, widths 230, 120, 150, 190pt, slide 3 |
| 10–13 | Division wrap probe, same width order, slide 3 |
| 14–17 | Plus-minus wrap probe, same width order, slide 3 |

The 0.5pt layout audit reports no missing/extra text, changed wrapping,
visibility, fill-occlusion, matched rectangle geometry, or position failures.
Searchable text and the codepoint-class census match: 1,593 normalized
codepoints. The original slide's seven material width-drift clusters are gone.
The high-DPI content crops preserve bold red headings, regular dark body text,
bullet indents, mathematical glyph shapes, and native line breaks. Remaining
sparse glyph-edge rasterization fragments are recorded as bounded observations.
The first two slides have identical decoded GT/output/diff pixels to the
previously inspected #1584 comparison, including the chart's fine axes,
ticks, bar edges and legend outlines. No new converter deviation was found.

## Reproduction

Run this Python snippet from the repository root to generate four probe specs:

```python
import json, zipfile
import xml.etree.ElementTree as ET
from pathlib import Path

source = Path('tests/golden_mocks/business/sources/pptx/08_marketing_report_en.pptx')
output = Path('target/issue-1581-reproduction').resolve()
output.mkdir(parents=True, exist_ok=True)
part = 'ppt/slides/slide3.xml'
ns = {'p': 'http://schemas.openxmlformats.org/presentationml/2006/main',
      'a': 'http://schemas.openxmlformats.org/drawingml/2006/main'}
for prefix, uri in ns.items():
    ET.register_namespace(prefix, uri)

# Change only the mathematical sign in the original first bullet.
symbol_spec = {
    'name': 'issue-1581-symbols-reproduction', 'base': str(source.resolve()),
    'part': part, 'page': 3, 'factor': 'first-bullet mathematical sign',
    'variants': [{'value': name, 'find': '2.1×', 'replace': f'2.1{symbol}', 'count': 1}
                 for name, symbol in [('ascii-x', 'x'), ('division', '÷'), ('plus-minus', '±')]]}
(output / 'symbols.json').write_text(json.dumps(symbol_spec, indent=2) + '\n')

# Each width series keeps the same short text and changes only a:xfrm/a:ext@cx.
for name, symbol in [('multiply', '×'), ('divide', '÷'), ('plusminus', '±')]:
    with zipfile.ZipFile(source) as archive:
        tree = ET.fromstring(archive.read(part))
        shape = next(s for s in tree.findall('.//p:sp', ns)
                     if any('Webinar' in (t.text or '') for t in s.findall('.//a:t', ns)))
        body = shape.find('p:txBody', ns)
        paragraphs = body.findall('a:p', ns)
        for paragraph in paragraphs[1:]:
            body.remove(paragraph)
        paragraphs[0].find('a:r/a:t', ns).text = f'Rates 2.1{symbol}3.0 lower-cost units'
        shape.find('p:spPr/a:xfrm/a:ext', ns).set('cx', str(230 * 12700))
        base = output / f'wrap-{name}.pptx'
        with zipfile.ZipFile(base, 'w', zipfile.ZIP_DEFLATED) as target:
            for entry in archive.infolist():
                payload = (ET.tostring(tree, encoding='utf-8', xml_declaration=True)
                           if entry.filename == part else archive.read(entry.filename))
                target.writestr(entry, payload)
    spec = {'name': f'issue-1581-wrap-{name}-reproduction', 'base': str(base),
            'part': part, 'page': 3, 'factor': 'textbox width in points',
            'variants': [{'value': str(width), 'find': f'cx="{230 * 12700}"',
                          'replace': f'cx="{width * 12700}"', 'count': 1}
                         for width in [190, 150, 120]]}
    (output / f'wrap-{name}.json').write_text(json.dumps(spec, indent=2) + '\n')
```

For each generated spec, run
`python3 scripts/probe_harness.py SPEC.json --backend office`.
The harness creates both the unchanged base and a re-zip control inside the
PowerPoint sandbox. It rejects a changed control. Check each native PDF with
`python3 scripts/check_gt_integrity.py NATIVE.pdf --source SOURCE.pptx --json`.

Build each implementation with `cargo build --locked -p office2pdf-cli` and
preserve its binary separately. Convert the original deck and every probe
using `office2pdf SOURCE.pptx -o OUTPUT.pdf --font-path FONT_DIRECTORY`;
on macOS the measured font directory is PowerPoint.app's
`Contents/Resources/DFonts`. Use `--slides 3` for each probe, and extract native
slide 3. Assemble the pages in the table's order with `mutool merge`.

Run `python3 scripts/compare_layout.py GT.pdf AFTER.pdf --json --audit --fine-shift 0.5`
and `python3 scripts/compare_text_layer.py GT.pdf AFTER.pdf --json`.
For every page, run `python3 scripts/compare_render.py GT.pdf AFTER.pdf --page N --dpi 150 --fine-shift 0.5 --artifacts-dir DIRECTORY --cluster-report PATH`.
Inspect full pages, matched crops and the 5% pixel sweep, then rerun with
`--cluster-dispositions PATH --strict-clusters`. No material cluster remains
on these pages; the disposition groups are empty.

## SHA-256 provenance

These hashes identify the measured collection and source packages.

- gt.pdf: `882bdd899783ea46f26da9233fae6cc914afb5003a2a2d43923fb1ddb2cc5cb4`
- before.pdf: `2e97d69dfaa0ff914ee79b957c433e939efe041561d0d0c416a5d91b59047386`
- after.pdf: `3f8b1ef7410c00c4d6d9c456d4835f0cf21689c99a13240a74f6081e2c1cc72a`
- after-cli: `8aaf0a6ee60738438c70dd4b0c5408abb054d7bb422b09aab68f864587941817`
- Original marketing PPTX: `8fe9aa23f97af98ae077e93b849d29d6559e962c322b959f09295b7802c2acaa`

| Collection page | Source package SHA-256 |
| --- | --- |
| 4 | `863e2859c7d39a810f907fc93f9060bfc091cea8b9c272502b1bbd3f3c555345` |
| 5 | `8e75f2d6e309773701e697e261c81016477e1019640c8830dc67d7f3c245d6de` |
| 6 | `dc7d2df15ba2ab2aa371ca172224307e6a83501c25e892a33b46d9e000fd6b72` |
| 7 | `eb0b1f94b0d55c1583065d24477f2329c80d9ca41b2cf725f16992134c52f414` |
| 8 | `2445a9b7b22d44490ff2ef4de58dd20a250428184dc2cb09734ee2d457a5ce15` |
| 9 | `b1b165655d52d086460322f135c79a27a9d91001833b5c36654d32b27aaf35e7` |
| 10 | `01963f94f38f9608db994d4f88642d4ea194ea061ad21188ce18632d41707153` |
| 11 | `9de650e698e1cfaafdc367efd4a327dabf701a9a3921730bdad3cbffb02ecd60` |
| 12 | `f4b27871cd2235d886e9d38b51d08150bf2bd5c9f6a881686d555bcdb06e71cf` |
| 13 | `cfcf02101ee62601191c45ed4e7473db354cfea10dc5d45ebb90fc10612dedc6` |
| 14 | `258c03582443ffc0e5f755dfda879553c841e24f1ded63ccd38bec6188dba4ea` |
| 15 | `79e3ba7cd8422b44890d1f09e3d8c37c10d016032f36422b60b34ee5c849c7be` |
| 16 | `8837faaf480aa61f19f1a0bf25c36668a64626b25ebc8c571f283902e65551d6` |
| 17 | `73df6b24c0307ef191ffc7378de6036c9f8a0b1c66f0d8215c790d09045aa072` |
