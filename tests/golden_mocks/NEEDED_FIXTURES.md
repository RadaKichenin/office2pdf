# Ground-truth fixtures still needed

Five open issues are blocked on the same thing: a native Microsoft Office
export that isolates one variable. Each has had at least one fix written,
measured, and rejected, because the corpus cannot distinguish between the
competing explanations. Reasoning further about our own metrics has not
worked — three separate attempts on #485 all moved Latin and Korean text in
opposite directions.

Producing these needs a machine where Office automation works.
`scripts/macos/export_excel_pdfs.applescript` timed out and then failed
`-50` on the machine where this was investigated, so the XLSX ground truth
in `assets/bugfixes/issue-430/gt.jpg` is a lossy JPEG — which is how the
~4% figure in #473 came to be overstated.

For each fixture: author it in the named app, save the source under
`tests/golden_mocks/business/sources/<type>/`, and export a PDF to
`tests/golden_mocks/business/expected/<type>/` with the same stem.

## 1. Vertically centred text at several sizes and scripts — #485, #402

**PowerPoint.** One slide. Six text boxes, all identical except for the run
inside them: same position, same size, `anchor="ctr"`, no autofit.

| Box | Font | Size |
| --- | --- | --- |
| 1 | Arial (or Calibri) | 12pt |
| 2 | Arial | 24pt |
| 3 | Arial | 32pt |
| 4 | Malgun Gothic | 12pt |
| 5 | Malgun Gothic | 24pt |
| 6 | Malgun Gothic | 32pt |

Put the same short string in each — one line, no wrapping. `제목 Title` works
for both scripts.

**What it settles.** Centred titles currently sit 6.4pt low for Latin and
10.8pt low for Korean, consistently across all ten decks. Three attempts to
fix this by resizing the line box each improved Latin by exactly 1pt and
made Korean *worse*. Varying size and script independently separates the
constant term from the script-dependent one, which no amount of reasoning
about our own line metrics has managed. The same measurement bears on #402,
where a 17pt Korean title under an 18pt `w:docGrid` is short by a similar
amount and the 16pt-versus-17pt threshold brackets the answer tightly.

## 2. Hangul before terminal punctuation at a line end — #438

**PowerPoint.** One slide, one text box, Malgun Gothic. Several paragraphs,
each ending in a Hangul syllable followed by one of `?`, `!`, `.`, `,`, `)`
and their fullwidth forms `？`, `！`, `。`, `、`, `）`. Size the box so each
of those pairs lands exactly at a line-end boundary — that is the whole
point, so it is worth checking the wrap position in PowerPoint before
saving.

**What it settles.** PowerPoint breaks between a Hangul syllable and a
following `?` when the pair does not fit; UAX #14 forbids that break, so we
move both down. The entire corpus contains exactly **one** instance of this,
with no coverage of the other marks. One positive sample is not enough to
encode a kinsoku rule that would apply to every Korean deck, and the failure
mode — punctuation opening a line where PowerPoint would not break — is
visible on every affected slide.

## 3. A workbook whose Normal font is not Calibri — #473

**Excel.** Any sheet with visible content. In the workbook's `Normal` cell
style set the font to **Malgun Gothic 12pt** (not Calibri), and give the
columns explicit widths — several different values, including a
non-integer one such as `6.7109375`.

**What it settles.** All ten XLSX golden mocks declare Calibri 11, and
`column_width_to_pt` is accurate to under 1pt across five of them
(0.37–0.94pt). So the formula is not globally wrong. The ~4% error reported
in #473 was sampled from a lossy JPEG at an antialiased fill boundary, where
one pixel is 0.48pt, so the real gap may be under 1pt. A PDF export settles
whether there is a defect at all before anyone risks touching a function that
five verified sheets depend on.

## 4. A shadowed shape at a known blur radius — #390

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
