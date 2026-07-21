## Summary

<!-- What changed and why? -->

## Related issue

<!-- Use "Fixes #N" when this PR fully resolves an issue. -->

## Testing

<!-- List the exact commands and manual checks run. -->

## Visual impact

<!-- Select exactly one. A reason is required when selecting no rendered change. -->

- [ ] No rendered PDF change
- [ ] Rendered PDF change or visual evidence added
- Reason: <!-- Required for "No rendered PDF change" -->

## Visual audit

<!-- Required when rendered output changes or assets/bugfixes/ is modified. Delete only when "No rendered PDF change" is selected. -->

- Issue: #<!-- N -->
- Fixture: <!-- repository path -->
- Page(s): <!-- compared page numbers -->
- Renderer and DPI: <!-- e.g. pdftoppm, 150 DPI -->
- Evidence mode: `fix` <!-- `fix` requires gt/before/after; `defect` requires compare -->
- New follow-up issues found in this audit: <!-- #N, #N or None; create issues before completing the audit -->
- GT: `assets/bugfixes/issue-<!-- N -->/gt.jpg`
- Before: `assets/bugfixes/issue-<!-- N -->/before.jpg`
- After: `assets/bugfixes/issue-<!-- N -->/after.jpg`
- Compare: `assets/bugfixes/issue-<!-- N -->/compare.jpg`

### Visual comparison

<!-- Replace each cell comment with rendered Markdown image syntax using a stable commit-pinned raw URL or GitHub attachment URL. -->

| GT | Before | After |
| --- | --- | --- |
| <!-- ![GT](IMAGE_URL) --> | <!-- ![Before](IMAGE_URL) --> | <!-- ![After](IMAGE_URL) --> |

<!-- Defect mode only: replace the cell comment with the rendered Compare image and remove the GT/Before/After table. -->

| Compare |
| --- |
| <!-- ![Compare](IMAGE_URL) --> |

### Required inspection

- [ ] Rendered all evidence at 150 DPI or higher
- [ ] Stored progressive JPEG quality 86 assets with metadata stripped
- [ ] Inspected matched region crops at full resolution
- [ ] Ran the 5% fuzz pixel-difference sweep
- [ ] Inventoried hairlines and border dash styles
- [ ] Inventoried font weight, italic, and underline emphasis

### Deviation audit

<!-- Every result must start with: Matches GT, Fixed, No deviation observed, or Remaining: #N. -->

| Check | Result |
| --- | --- |
| Page count/order | <!-- status --> |
| Element presence | <!-- status --> |
| Position/size | <!-- status --> |
| Rotation/flip | <!-- status --> |
| Fill | <!-- status --> |
| Stroke/border | <!-- status --> |
| Text content | <!-- status --> |
| Font family/weight/style | <!-- status --> |
| Text color | <!-- status --> |
| Alignment | <!-- status --> |
| Line/paragraph spacing | <!-- status --> |
| Clipping/overflow | <!-- status --> |

## Checklist

- [ ] Commits include a `Signed-off-by` line
- [ ] PR scope contains one root cause
- [ ] Remaining visual deviations each reference an open issue
