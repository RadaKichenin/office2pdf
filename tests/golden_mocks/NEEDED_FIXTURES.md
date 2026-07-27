# Ground-truth fixtures still needed

Some issues can only move once a native Microsoft Office export isolates
one variable — reasoning about our own metrics has repeatedly produced
fixes that measured worse. This file lists the exports still missing.
Producing them needs a machine where Office automation works;
`scripts/macos/export_excel_pdfs.applescript` timed out and then failed
`-50` on the machine where the early entries were investigated. Windows
Office COM automation (Excel/PowerPoint/Word) is the working alternative.

For each fixture: author it in the named app, save the source under
`tests/golden_mocks/business/sources/<type>/`, and export a PDF to
`tests/golden_mocks/business/expected/<type>/` with the same stem.

## 1. A shadowed shape at a known blur radius — #390

**PowerPoint.** One slide, three rectangles with a solid fill and an
`outerShdw`, each at a different `blurRad` — say 6pt, 12pt and 24pt — with
`dist` and `dir` held constant. Leave a wide margin of empty slide around
each so the shadow is not clipped or overlapped.

**What it settles.** Our shadow is a stack of concentric translucent rings.
Replacing the uniform alphas with a graduated ramp looks obviously softer but
scores *worse* on RMSE and `AE -fuzz 1%`, because PowerPoint's shadow is
**tighter** than ours: the gradient was not the only thing wrong, the extent
is too. Varying `blurRad` alone gives the mapping from that value to the
visible spread, which is the term currently being guessed.

## Settled entries

Three earlier entries in this file were fulfilled and their issues resolved:

- **Vertically centred text at several sizes and scripts** (#485, closed):
  a Windows-PowerPoint-authored centring fixture showed the filed 6.4/10.8pt
  title offsets were the pre-#506 descriptor-box measurement bias; the
  remaining body-line divergence is #513.
- **Hangul before terminal punctuation at a line end** (#438): the fixture
  now exists at `tests/fixtures/pptx/hangul_kinsoku_terminal_punct.pptx`
  (Windows-PowerPoint-authored; covers `? ! . , ) ” … :`, the fullwidth
  forms, and the `%` counter-case) and the kinsoku rule it settled is
  implemented in the PPTX parser.
- **A workbook whose Normal font is not Calibri** (#473, closed): native
  Windows Excel exports measured from PDF vectors refuted the ~4%-narrow
  claim; the decoded print model and the remaining platform divergence are
  recorded in #511.

## Measuring against them

Once a pair exists, run the three-axis comparison rather than eyeballing a
pixel count:

```sh
python3 scripts/compare_render.py <expected>.pdf <output>.pdf [--page N]
```

See the "Three-axis comparison" section of `CLAUDE.md` for why a pixel
difference alone is not evidence — on this corpus it has scored two visibly
different renders identically, risen on a correct fix, and stayed flat
through a twentyfold accuracy improvement.
