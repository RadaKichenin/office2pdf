# Agent Rules

- Always read and follow the rules defined in `CLAUDE.md` before starting any task.
- For visual bug fixes, store reproducible before/after images under `assets/bugfixes/issue-<number>/`.
- If a dependency limitation or bug breaks PDF conversion, clone that library, fix and test it upstream, and open a PR. Follow its repository conventions and match the tone and scope of its recently merged PRs.
- Before every commit, delegate a read-only freshness audit and wait for `PASS:`. Codex must use `documentation_freshness_reviewer`; Claude Code must use `documentation-freshness-reviewer`. Compare existing documentation, examples, and code comments with the current code and configuration; update or remove stale versions, commands, APIs, behavior, defaults, paths, architecture, limitations, and unverified claims in the same commit.

## Release Rules

- Follow `RELEASING.md` end to end in one turn; a version bump or GitHub Release alone is not completion.
- Perform every GitHub operation for this repository as `developer0hye`. Ignore stale shell tokens with `GH_TOKEN=''` and verify `gh api user --jq .login` before the first write.
- After the version PR merges, start releases only by dispatching `release.yml` with the tag. Monitor that exact run and verify the tag, both crates.io packages, and all six assets before reporting success.

## Visual Check Discipline (applies to every agent; canonical copy in CLAUDE.md)

- **Enumerate before fixing.** For every compared page, walk this checklist and record each deviation before touching code: page count/order; element presence; position; size; rotation/flip; fill; stroke/border (incl. dash style); text content; font family/weight/style; text color; alignment; line/paragraph spacing; clipping/overflow.
- **One issue per root cause.** When one image reveals multiple independent defects, file a separate issue for each — never bundle them into one issue or one PR. Fix them sequentially.
- **Closing condition.** An issue may be closed only when a fresh GT comparison shows its specific defect gone. Every remaining visible deviation on that comparison must already have its own open issue — file the missing ones before closing.
- **After images are re-audited.** When posting an after image, re-run the checklist on it; each still-visible deviation gets an issue reference in the PR body ("remaining, tracked in #N").

### Fine-detail analysis (thin and small elements)

Whole-page thumbnails at 80 DPI hide hairlines, dash patterns, font weight, and sub-pixel offsets. For every compared page:

1. **High-DPI pass.** Render both sides at ≥150 DPI (`pdftoppm -r 150`) before judging any checklist item involving stroke width, dash style, font weight/italic, or small glyphs. Never mark those items "OK" from an 80 DPI image.
2. **Region crops.** For each region containing text, lines, or decorations, cut matched crops from GT and output (`magick input.png -crop WxH+X+Y crop.png`) and view them side by side at full scale.
3. **Pixel-difference sweep.** Run `magick compare -metric AE -fuzz 5% gt.png out.png diff.png` on size-normalized pages; view `diff.png` and inspect every highlighted cluster. A checklist pass is complete only when each cluster is either explained by an accepted rendering difference (fonts/antialiasing) or captured as an issue.
4. **Hairline inventory.** Explicitly enumerate elements ≤1pt (rules, underlines, dashed/dotted lines, borders, tick marks) found in GT and confirm each exists in the output at matching position, width, and dash pattern.
5. **Weight/emphasis inventory.** Enumerate bold/italic/underlined runs visible in GT (including CJK) and confirm the same emphasis in the output — weight differences must be checked on the high-DPI crops, not thumbnails.
