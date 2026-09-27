# An inline DrawingML text box is a box on the anchor paragraph's line

Word draws a `wp:inline` text box as one item on the line of the paragraph that
anchors it: the line grows to the box's height, the box's bottom edge rests on
the baseline, and the box's own paragraphs flow inside its `a:ln` outline.

office2pdf emitted the box's paragraphs as ordinary flow blocks under the anchor
paragraph and drew no outline. `Box text` got a line of its own at the left
margin, and every line below the box landed at the wrong height.

## Measured

Native Word for Mac export as GT, both read with `mutool draw -F stext`
(baselines) and `-F trace` (paths). Same package, same page.

| Line | Word for Mac | office2pdf before | office2pdf after |
| --- | --- | --- | --- |
| `Box text` | 86.16 | 102.97, its own line, no box | 85.92 |
| `Anchor paragraph with a text box` | 108.00 | 82.32 | 108.00 |
| `Plain paragraph` | 128.64 | 115.62 | 128.65 |
| `Shaded paragraph` | 149.28 | 136.26 | 149.30 |
| `Plain paragraph after` | 169.92 | 156.91 | 169.95 |

`before` is `main` at `17d4a1b8`, measured rather than quoted: the issue's table
was taken at `8e3e3ffd`, before #1689 landed, and its `Plain paragraph` figure no
longer holds. `before.pdf` contains no `stroke_path` at all.

Word's anchor baseline is 108.00 because the 36pt box, not the 11pt Arial text,
sets the line's ascent: the box occupies 72.00–108.00 and the text rests on its
bottom edge.

The outline matches to the digits the traces carry. Word strokes it in an
EMU-scaled space — `linewidth="9525"` under a `.00007874016` (1/12700) CTM, so
0.75pt — over a path of 1828800 × 457200 EMU from origin (233.4282, 72). Ours is
`linewidth=".75"` at unit scale over 144 × 36pt from (233.42823, 72). The layout
audit matches 3/3 rectangles with a 0.005pt mean centre delta.

`Box text` remains 0.24pt high, and 0.33pt to the left. That is Word's
first-baseline seat inside a text box, which this change does not model; both are
below the materiality bar for placement and below the 0.5pt fine-shift threshold
the audit ran at, so they are recorded against the standing sub-material tracker
#1874.

## Fix

Typst aligns an inline `box` on the first baseline of its in-flow content, which
leaves the box hanging below the line instead of standing on it — measured at
85.92pt for the anchor baseline against Word's 108.00pt. Placing the content out
of flow makes the box's own bottom edge its baseline, which is where Word rests
it. That is the same `#place(top + left)` the anchored text box path already
uses.

`Run::inline_box` carries the box through the IR, so it stays in the paragraph's
inline sequence rather than becoming a sibling block. It rides the run itself
rather than a side table keyed by run index, so the box reaches the page from
every paragraph path — a table cell's and a tabbed paragraph's included — and not
only the body's.

`DrawingTextBoxInfo` gains the shape frame — `a:ln`, `a:solidFill`, and the
`wps:bodyPr` insets — read through the same `ShapeBuilder` the geometry-only
shape scan uses, so the two scanners cannot drift apart on what an `a:ln` means.
That scan is confined to the outer `w:drawing`, so a picture nested in one of the
box's own paragraphs cannot overwrite the box's extent or frame with its own.

A box holding a table or a picture still flattens into the body flow, because the
run emitters carry no generator context. That remainder is #1889.

## Package

The evidence package is the one in `assets/bugfixes/issue-1689/README.md`,
byte for byte: this defect and #1689's were the two findings on that page. Run
that README's script to regenerate it. The PDF the GT came from was exported by
Microsoft Word for Mac through `scripts/macos/export_word_pdfs.applescript`.
