# Life Manager — Brand Usage Guide

**Identity:** Life Manager · **Selected symbol:** Offset Channels (Concept B)  
**Source of truth:** `design/life-manager-master.figcraft` → page **17 Brand Identity**  
**Generator:** `design/scripts/build_brand_identity.py` (part of `rebuild_master.py`)  
**Export recipe:** `python3 design/scripts/export_brand_identity.py`

## Rationale

The custom geometric mark contains two rounded, offset counterflow channels around an open center. It represents independent domains of life becoming coherent without merging into a single forced agenda. It is intentionally not a calendar, generic checkmark, clock, robot or infinity icon.

The logo uses editable native FigCraft vector contours. The app-icon background is itself a closed vector squircle, with transparent outside corners. The icon is not a flattened graphic added to FigCraft.

## Official colors

| Application | Color | Source token |
|---|---|---|
| Primary symbol on a light background | `#266C56` | `accent` |
| Mint symbol on dark background | `#B7D7C8` | `accent-quiet` |
| Dark app-icon background | `#162929` | `surface-inverse` |
| Light canvas | `#F6F7F5` | `canvas` |
| Text on light canvas | `#19292A` | `text` |
| White monochrome mark | `#FFFFFF` | `text-inverse` |

Measured pairwise contrasts: 5.81:1 for primary green on light canvas, 9.81:1 for mint symbol on the dark icon. These ratios are for the stated pairs, not blanket accessibility claims about every use.

## Variants and placement

- **Application/Dock:** `exports/LifeManager.icns` or size-specific `exports/LifeManager-{size}.png`. The dark, self-contained squircle should not be framed in another large colored square. The mask must preserve transparent outside corners.
- **Horizontal wordmark:** `exports/life-manager-wordmark.svg` (or PNG/PDF) for README banners, website or release materials. Keep the name as **Life Manager** with the approved spacing.
- **Standalone symbol:** `exports/life-manager-symbol.svg` for light backgrounds; do not place the green symbol on low-contrast dark green.
- **Dark surfaces:** `exports/life-manager-dark.png` or the white mark on dark backgrounds.
- **Monochrome:** `exports/life-manager-black.svg` and `exports/life-manager-white.svg` for embossing, grayscale and single-color use.
- **Menu bar:** `exports/life-manager-menubar.png` for visual reference. Actual template-image behavior/tinting must be tested during separate app integration.
- **Favicons:** Derive 32/16-pixel transparently from the standalone symbol; use alpha-preserving export and inspect the actual pixel grid.

## Clear space and minimum size

- Preserve at least **12% of symbol width** of clear space around the standalone mark, except when an existing squircle composition already supplies safe space.
- Keep the horizontal wordmark legible; do not crop its ascenders/descenders or stretch it nonuniformly.
- At 16 and 24 pixels, use the simplified/small icon shape and view at actual pixels before shipping, rather than judging only a zoomed vector.

## What not to do

Do not add gradients, shadows, glossy/glass effects, distorted proportions, arbitrary recolors, simulated clocks, AI sparkles or checkmarks. Do not flatten master geometry into a screenshot. Do not replace existing application UI/sidebar assets as an indirect side effect of brand generation.

## Authoring workflow

From the repository root:

```bash
python3 design/scripts/rebuild_master.py
python3 design/scripts/export_brand_identity.py
python3 design/scripts/validate_design.py
```

The rebuild includes the brand generator after the standard token binding pass. The exporter writes logo assets, previews and `design/brand/asset-manifest.json`. The third command verifies existing screen-design integrity as well as the expanded FigCraft master.

## Legal and font considerations

The geometry was authored as custom vector paths for this project. A visual comparison with common app-icon patterns is not a trademark search or legal clearance. The wordmark uses the existing macOS Helvetica Neue/system-compatible design vocabulary; do not redistribute proprietary font files. The SVG export represents its visible shapes as vector paths, avoiding the need to bundle the font in that export.

## Integration boundary

**Brand asset sync is wired via `scripts/sync-brand-assets.py`**, which populates `assets/brand/` and `assets/icons/` (including `assets/icons/icon.icns` used by `scripts/release.sh` for `Life Manager.app`). The GPUI desktop shell (`crates/life-gpui/src/views.rs`) currently renders `IconName::Sparkles` in the sidebar header rather than the Offset Channels SVG mark. The application has no menu-bar tray feature; none was introduced. Release publication and app notarization are not included.
