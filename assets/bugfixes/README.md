# Bug-fix evidence

Visual evidence lives in an issue-numbered directory and is checked by the Visual
PR Contract job (`scripts/check_visual_pr.py`). There are two modes; a pull
request declares which one it uses in `Visual audit > Evidence mode`.

## Evidence mode: `fix`

A pull request that fixes a visual defect commits all three images:

```text
assets/bugfixes/issue-<number>/gt.jpg
assets/bugfixes/issue-<number>/before.jpg
assets/bugfixes/issue-<number>/after.jpg
```

Touching any one of them makes the gate require and validate all three, so the
trio always lands together. Generate them from the same input document, page,
resolution, and renderer.

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
only such a file may check `No rendered PDF change`.

Anything else in the directory is treated as evidence and must match one of the
layouts above. The nested `issue-<number>/audit/<case>/` layout used by the early
tracking issues (#213, #214) predates the gate and is not accepted for new
evidence.
