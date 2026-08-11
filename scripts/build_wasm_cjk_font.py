#!/usr/bin/env python3
"""Build the feature-gated Simplified Chinese font shipped in WASM."""

from __future__ import annotations

import argparse
import hashlib
import tempfile
from pathlib import Path

from fontTools import subset


EXPECTED_SOURCE_SHA256 = (
    "2c76254f6fc379fddfce0a7e84fb5385bb135d3e399294f6eeb6680d0365b74b"
)
EXPECTED_GB2312_CHARACTER_COUNT = 7_445
EXPECTED_GB2312_HAN_COUNT = 6_763


def gb2312_codepoints() -> set[int]:
    characters: set[str] = set()
    for lead in range(0xA1, 0xF8):
        for trail in range(0xA1, 0xFF):
            try:
                characters.add(bytes((lead, trail)).decode("gb2312"))
            except UnicodeDecodeError:
                continue

    han_count = sum("\u4e00" <= character <= "\u9fff" for character in characters)
    if len(characters) != EXPECTED_GB2312_CHARACTER_COUNT:
        raise RuntimeError(f"unexpected GB2312 character count: {len(characters)}")
    if han_count != EXPECTED_GB2312_HAN_COUNT:
        raise RuntimeError(f"unexpected GB2312 Han count: {han_count}")

    return {ord(character) for character in characters} | set(range(0x20, 0x7F))


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("source", type=Path)
    parser.add_argument("output", type=Path)
    args = parser.parse_args()

    source_sha256 = hashlib.sha256(args.source.read_bytes()).hexdigest()
    if source_sha256 != EXPECTED_SOURCE_SHA256:
        raise RuntimeError(
            f"source SHA-256 mismatch: expected {EXPECTED_SOURCE_SHA256}, got {source_sha256}"
        )

    codepoints = gb2312_codepoints()
    args.output.parent.mkdir(parents=True, exist_ok=True)
    with tempfile.NamedTemporaryFile(mode="w", encoding="ascii") as manifest:
        manifest.write(",".join(f"U+{codepoint:04X}" for codepoint in sorted(codepoints)))
        manifest.flush()
        subset.main(
            [
                str(args.source),
                f"--output-file={args.output}",
                f"--unicodes-file={manifest.name}",
                "--layout-features=*",
                "--glyph-names",
                "--symbol-cmap",
                "--legacy-cmap",
                "--notdef-glyph",
                "--notdef-outline",
                "--recommended-glyphs",
                "--name-IDs=*",
                "--name-languages=*",
                "--name-legacy",
            ]
        )

    output_sha256 = hashlib.sha256(args.output.read_bytes()).hexdigest()
    print(f"codepoints={len(codepoints)}")
    print(f"bytes={args.output.stat().st_size}")
    print(f"sha256={output_sha256}")


if __name__ == "__main__":
    main()
