# Feature-gated WASM Chinese font

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
