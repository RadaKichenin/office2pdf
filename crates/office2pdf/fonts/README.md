# Bundled fallback fonts

## Noto Sans 2.015

`NotoSans-Regular.ttf` is the unhinted static Regular face from the official
Noto Sans 2.015 release. It is loaded only when a document requests Aptos.
LibreOffice 25.2 resolves the unembedded Aptos footer in the poster fixture
from issue #1463 to Noto Sans; shipping the same pinned face keeps its glyph
widths independent of whether the converting host has Microsoft Office fonts.
An Aptos face embedded in the package or supplied by the caller still wins.

- Release tag: `NotoSans-v2.015`
- Source commit: `c4a321e123e4d4ff315f57f4e0adf294fe3a95be`
- Release URL: <https://github.com/notofonts/latin-greek-cyrillic/releases/tag/NotoSans-v2.015>
- Release archive SHA-256: `0c34df072a3fa7efbb7cbf34950e1f971a4447cffe365d3a359e2d4089b958f5`
- `NotoSans-Regular.ttf` SHA-256: `f3961a9cde016d41a4879aecda1474d3a36d6bf54fa0e4643de029cc2248b0e8`
- License: SIL Open Font License 1.1; see `OFL-1.1.txt`

Reproduce the committed asset from the pinned release archive while keeping
the archive's nested path out of the repository:

```sh
unzip -p NotoSans-v2.015.zip \
  NotoSans/unhinted/ttf/NotoSans-Regular.ttf \
  > crates/office2pdf/fonts/NotoSans-Regular.ttf
printf '%s  %s\n' \
  f3961a9cde016d41a4879aecda1474d3a36d6bf54fa0e4643de029cc2248b0e8 \
  crates/office2pdf/fonts/NotoSans-Regular.ttf \
  | shasum -a 256 -c -
```

The final command must report `OK`.

## Selawik 1.01

`Selawik-Regular.ttf` and `Selawik-Bold.ttf` are the `selawk.ttf` and
`selawkb.ttf` faces from Microsoft's official Selawik 1.01 binary release.
Microsoft describes Selawik as its open-source replacement for Segoe UI, and
the two families carry the same glyph advances used by the issue #1472 Gift
Budget workbook. They are loaded only when a document requests Segoe UI, so an
unavailable proprietary face keeps its native spreadsheet indents and wrap
boundaries without changing the fallback book for unrelated conversions.

- Release tag: `1.01`
- Source repository: <https://github.com/microsoft/Selawik>
- Release URL: <https://github.com/microsoft/Selawik/releases/tag/1.01>
- Release archive SHA-256: `3f62c51e05e3b5a1e6241cf92a371f0be2ea1183aa87b30718bbd40832a8d423`
- `Selawik-Regular.ttf` SHA-256: `e9d98518d8ac2817782a9a382430463a2e0793ea68350b695bb727d9a830ee1c`
- `Selawik-Bold.ttf` SHA-256: `f0db5e174a90e0956ad7d2844bdca1d5e6da92ec65b2c04e57ba9b180668c904`
- License: SIL Open Font License 1.1 with Reserved Font Name Selawik; see
  `Selawik-LICENSE.txt`
- `Selawik-LICENSE.txt` SHA-256: `77b7c2506d4efb22e09c8ccf10159f4956eab3ef7c007fef95de136bcf45300c`

Reproduce the committed assets from the pinned release archive:

```sh
gh release download 1.01 \
  --repo microsoft/Selawik \
  --pattern Selawik_Release.zip
printf '%s  %s\n' \
  3f62c51e05e3b5a1e6241cf92a371f0be2ea1183aa87b30718bbd40832a8d423 \
  Selawik_Release.zip \
  | shasum -a 256 -c -
unzip -p Selawik_Release.zip selawk.ttf \
  > crates/office2pdf/fonts/Selawik-Regular.ttf
unzip -p Selawik_Release.zip selawkb.ttf \
  > crates/office2pdf/fonts/Selawik-Bold.ttf
curl -fL \
  https://raw.githubusercontent.com/microsoft/Selawik/1.01/LICENSE.txt \
  -o crates/office2pdf/fonts/Selawik-LICENSE.txt
printf '%s  %s\n' \
  e9d98518d8ac2817782a9a382430463a2e0793ea68350b695bb727d9a830ee1c \
  crates/office2pdf/fonts/Selawik-Regular.ttf \
  f0db5e174a90e0956ad7d2844bdca1d5e6da92ec65b2c04e57ba9b180668c904 \
  crates/office2pdf/fonts/Selawik-Bold.ttf \
  77b7c2506d4efb22e09c8ccf10159f4956eab3ef7c007fef95de136bcf45300c \
  crates/office2pdf/fonts/Selawik-LICENSE.txt \
  | shasum -a 256 -c -
```

The final two checksum commands must report `OK` for the archive, both faces,
and the license.

## Noto Serif 2.015

`NotoSerif-Regular.ttf` and `NotoSerif-Bold.ttf` are the unhinted static
Regular and Bold faces from the official Noto Serif 2.015 release. They are
loaded only when a document requests `Avenir Next LT Pro`, `Avenir Next W1G
Medium`, or `The Hand Black`. LibreOffice 25.2 resolves those unembedded faces
to Noto Serif in the poster fixture from issue #1458; shipping the same pinned
faces keeps glyph widths and line advance independent of fonts installed on the
host.

- Release tag: `NotoSerif-v2.015`
- Source commit: `c4a321e123e4d4ff315f57f4e0adf294fe3a95be`
- Release URL: <https://github.com/notofonts/latin-greek-cyrillic/releases/tag/NotoSerif-v2.015>
- Release archive SHA-256: `0e9a43c8a4b94ac76f55069ed1d7385bbcaf6b99527a94deb5619e032b7e76c1`
- `NotoSerif-Regular.ttf` SHA-256: `a15cfbbc1539d707115111d672d590a3d70d4f74b4c0a315956da20ae19a14e1`
- `NotoSerif-Bold.ttf` SHA-256: `24ad531e6b05ddad8c3d89572d2c93eb86a6b74e652ce7ee3c3e171de68e84c3`
- License: SIL Open Font License 1.1; see `OFL-1.1.txt`

Reproduce the committed assets from the pinned release archive while keeping
the archive's nested paths out of the repository:

```sh
unzip -p NotoSerif-v2.015.zip \
  NotoSerif/unhinted/ttf/NotoSerif-Regular.ttf \
  > crates/office2pdf/fonts/NotoSerif-Regular.ttf
unzip -p NotoSerif-v2.015.zip \
  NotoSerif/unhinted/ttf/NotoSerif-Bold.ttf \
  > crates/office2pdf/fonts/NotoSerif-Bold.ttf
printf '%s  %s\n' \
  a15cfbbc1539d707115111d672d590a3d70d4f74b4c0a315956da20ae19a14e1 \
  crates/office2pdf/fonts/NotoSerif-Regular.ttf \
  24ad531e6b05ddad8c3d89572d2c93eb86a6b74e652ce7ee3c3e171de68e84c3 \
  crates/office2pdf/fonts/NotoSerif-Bold.ttf \
  | shasum -a 256 -c -
```

The final command must report `OK` for both files.

## Feature-gated WASM Chinese font

`NotoSansCJKsc-GB2312.otf` is included in compiled output only when both the
`wasm32` target and the `wasm-cjk-font` feature are active. It is a Regular
subset of `NotoSansCJKsc-Regular.otf` from the official Noto CJK repository.

- Source commit: `f8d157532fbfaeda587e826d4cd5b21a49186f7c`
- Source URL: <https://github.com/notofonts/noto-cjk/blob/f8d157532fbfaeda587e826d4cd5b21a49186f7c/Sans/OTF/SimplifiedChinese/NotoSansCJKsc-Regular.otf>
- Source SHA-256: `2c76254f6fc379fddfce0a7e84fb5385bb135d3e399294f6eeb6680d0365b74b`
- Subset SHA-256: `55b8b0257bf8ab3bff637b4150840e898d562099c881e68edce52ba0c1c1f43e`
- Subset size: 3,511,684 bytes
- License: SIL Open Font License 1.1; see `OFL-1.1.txt`

The subset contains printable ASCII and every Unicode character decoded from
valid two-byte GB2312 sequences: 7,540 codepoints total, of which 7,445 are in
GB2312 and 6,763 are Han characters. This is Simplified Chinese coverage, not a
claim of full Traditional Chinese, Japanese, or Korean support.

Rebuild with FontTools 4.63.0 after downloading the pinned source face:

```sh
uv run --with fonttools==4.63.0 python scripts/build_wasm_cjk_font.py \
  /path/to/NotoSansCJKsc-Regular.otf \
  crates/office2pdf/fonts/NotoSansCJKsc-GB2312.otf
```
