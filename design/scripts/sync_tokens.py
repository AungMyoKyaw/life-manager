#!/usr/bin/env python3
"""Write the GPUI application's design tokens from the design system's token table.

The FigCraft master and the running GPUI app read their colours, radii, type
scale and spacing from ``design/scripts/tokens.py``.
"""
import pathlib
import sys

HERE = pathlib.Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
import tokens

ROOT = HERE.parents[1]
RUST_TOKENS = ROOT / "crates" / "life-gpui" / "src" / "tokens.rs"


def main():
    RUST_TOKENS.parent.mkdir(parents=True, exist_ok=True)
    RUST_TOKENS.write_text(tokens.rust_tokens_source())
    print(
        f"wrote {len(tokens.COLORS)} light, {len(tokens.DARK)} dark, "
        f"{len(tokens.RADIUS)} radius, {len(tokens.TYPE_SCALE)} type and "
        f"{len(tokens.SPACE)} spacing tokens to {RUST_TOKENS}"
    )


if __name__ == "__main__":
    main()
