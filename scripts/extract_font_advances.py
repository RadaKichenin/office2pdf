#!/usr/bin/env python3
"""Print reproducible OpenType hmtx advances for a Unicode range."""

from __future__ import annotations

import argparse
import hashlib
from pathlib import Path

from fontTools.ttLib import TTFont


def name_value(font: TTFont, name_id: int) -> str:
    """Return a stable Unicode value for one OpenType name-table field."""
    values = sorted(
        {
            record.toUnicode()
            for record in font["name"].names
            if record.nameID == name_id
        }
    )
    return " | ".join(values)


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("font", type=Path)
    parser.add_argument("--start", type=lambda value: int(value, 0), default=0x20)
    parser.add_argument("--end", type=lambda value: int(value, 0), default=0x7E)
    args = parser.parse_args()

    source = args.font.read_bytes()
    font = TTFont(args.font)
    cmap = font.getBestCmap()
    metrics = font["hmtx"].metrics
    missing = [codepoint for codepoint in range(args.start, args.end + 1) if codepoint not in cmap]
    if missing:
        formatted = ", ".join(f"U+{codepoint:04X}" for codepoint in missing)
        raise RuntimeError(f"font has no glyph for: {formatted}")

    advances = [metrics[cmap[codepoint]][0] for codepoint in range(args.start, args.end + 1)]
    print(f"family={name_value(font, 1)}")
    print(f"subfamily={name_value(font, 2)}")
    print(f"version={name_value(font, 5)}")
    print(f"postscript_name={name_value(font, 6)}")
    print(f"sha256={hashlib.sha256(source).hexdigest()}")
    print(f"units_per_em={font['head'].unitsPerEm}")
    print(f"range=U+{args.start:04X}..=U+{args.end:04X}")
    print("advances=[")
    for offset in range(0, len(advances), 16):
        print("    " + ", ".join(str(value) for value in advances[offset : offset + 16]) + ",")
    print("]")


if __name__ == "__main__":
    main()
