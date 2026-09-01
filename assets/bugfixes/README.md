# Bug-fix evidence

Visual evidence lives in an issue-numbered directory and is checked by the Visual
PR Contract job (`scripts/check_visual_pr.py`). There are two modes; a pull
request declares which one it uses in `Visual audit > Evidence mode`.

## Evidence mode: `fix`

A pull request that fixes a visual defect keeps all three images and a layout
report:

```text
assets/bugfixes/issue-<number>/gt.jpg
assets/bugfixes/issue-<number>/before.jpg
assets/bugfixes/issue-<number>/after.jpg
assets/bugfixes/issue-<number>/layout-audit.json
```

Touching any image makes the gate require and validate the full trio. Every fix
pull request with a raster change changes `after.jpg` and `layout-audit.json`; `gt.jpg` and
`before.jpg` may remain from the defect evidence when they are still current.
Generate all evidence from the same input document, page, resolution, and
renderer.

For a text-layer-only fix, the honest before and after page renders can be
byte- and pixel-identical. Regenerate and verify the current `after.jpg` without
annotations; it need not appear in the diff when the render is unchanged. Set
`Text-layer-only: Yes` and `Pixel delta: 0` in the pull request's `Visual audit`,
change `layout-audit.json`, and use that report to demonstrate that the missing
or extra searchable text is resolved. The contract requires both `before.jpg`
and `after.jpg` and independently runs ImageMagick's exact decoded-pixel
comparison; the declaration cannot bypass a raster-changing fix.

Generate the machine-readable layout report from those same GT and after PDFs:

```sh
python3 scripts/compare_layout.py --json --audit --fine-shift 0.5 gt.pdf after.pdf \
  > assets/bugfixes/issue-<number>/layout-audit.json
```

The command exits nonzero when material findings remain but still writes the
report. Change both `after.jpg` and `layout-audit.json` in a raster-changing fix
pull request; use the text-layer-only exception above when the current render is
byte-identical. Record the same `Fine-detail threshold` in the PR's `Visual
audit`, then mark each layout-audit category as `Pass` or list the open issues
that classify it. Those references must also appear in `Remaining:` deviation
rows. The gate rejects a claimed pass when the report contains a page-count
difference, missing/extra/reflowed text, changed wraps, a painted-text visibility
mismatch, a visible-fill occlusion, or a text shift above the configured fine or
large threshold.

## Evidence mode: `defect`

A pull request that files a visual defect commits a single side-by-side image —
ground truth on the left, office2pdf output at filing time on the right:

```text
assets/bugfixes/issue-<number>/compare.jpg
```

Both panels come from the same page and resolution, so `compare.jpg` must be an
even number of pixels wide for the halves to split evenly.

## JPEG rules the gate enforces

Every committed image must be:

- **progressive** — baseline JPEG is rejected;
- **at least 150 DPI** in the JFIF density header, and rendered at that DPI
  rather than upscaled afterwards;
- **free of metadata** — APP1 (Exif/XMP), APP2 (ICC), APP13 (Photoshop/IPTC), and
  COM markers must all be absent.

Use quality 86 and preserve the source pixel dimensions. Set the density *after*
`-strip`, because `-strip` also drops the JFIF header that carries it:

```sh
magick page.png -strip -density 150 -units PixelsPerInch \
  -interlace Plane -quality 86 assets/bugfixes/issue-<number>/after.jpg
```

Validate locally before pushing:

```sh
python3 scripts/check_visual_pr.py --event event.json --base main --head HEAD \
  --repository developer0hye/office2pdf --root .
```

## Non-image files

Markdown and text files under `assets/bugfixes/` — this README included — are
bookkeeping, not evidence. The gate skips them, so a pull request that changes
only such a file may check `No rendered PDF change`. A fix-mode
`layout-audit.json` is validated separately and must be updated with
`after.jpg` in its issue directory.

Anything else in the directory is treated as evidence and must match one of the
layouts above. The nested `issue-<number>/audit/<case>/` layout used by the early
tracking issues (#213, #214) predates the gate and is not accepted for new
evidence.
