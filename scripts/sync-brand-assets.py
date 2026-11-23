#!/usr/bin/env python3
"""Install Life Manager's approved FigCraft brand into application assets.

The source artwork is design/brand-vectorcraft, produced from the native
life-manager-master.figcraft master.
"""
from __future__ import annotations
from pathlib import Path
from PIL import Image
import argparse
import shutil

ROOT = Path(__file__).resolve().parents[1]
SOURCE = ROOT / "design/brand-vectorcraft"
BRAND = ROOT / "assets/brand"
ICONS = ROOT / "assets/icons"


def ensure(name: str) -> Path:
    p = SOURCE / name
    if not p.is_file() or p.stat().st_size == 0:
        raise SystemExit(f"Missing branded source asset: {p}")
    return p


def main() -> None:
    parser = argparse.ArgumentParser(description="Sync VectorCraft brand assets into GPUI app assets")
    parser.add_argument("--check", action="store_true", help="Validate existing files instead of installing")
    args = parser.parse_args()
    icon = ensure("life-manager-app-icon.png")
    symbol = ensure("life-manager-symbol.svg")
    wordmark = ensure("life-manager-wordmark.svg")
    targets = {
        BRAND / "life-manager-app-icon.png": icon,
        BRAND / "life-manager-symbol.svg": symbol,
        BRAND / "life-manager-wordmark.svg": wordmark,
    }
    if args.check:
        for dst, src in targets.items():
            if not dst.is_file() or dst.read_bytes() != src.read_bytes():
                raise SystemExit(f"Brand sync failed: {dst.relative_to(ROOT)} is missing or stale")
        for name in ("32x32.png", "128x128.png", "128x128@2x.png", "icon.icns", "icon.ico"):
            p = ICONS / name
            if not p.is_file() or not p.stat().st_size:
                raise SystemExit(f"Brand sync failed: {p.relative_to(ROOT)}")
        for name, sz in (("32x32.png", 32), ("128x128.png", 128), ("128x128@2x.png", 256)):
            with Image.open(ICONS / name) as im:
                if im.size != (sz, sz):
                    raise SystemExit(f"Brand icon size mismatch: {name}: {im.size}")
        print("PASS: FigCraft logo, wordmark, and native GPUI macOS icon files present and synced")
        return

    for dst, src in targets.items():
        dst.parent.mkdir(parents=True, exist_ok=True)
        shutil.copy2(src, dst)
        print(f"Synced {dst.relative_to(ROOT)}")
    print("PASS: Brand assets synced into assets/brand and assets/icons")


if __name__ == "__main__":
    main()
