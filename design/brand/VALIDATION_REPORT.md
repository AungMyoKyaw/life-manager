# Life Manager — Brand Identity Validation

**Date:** 2026-11-14  
**Selection:** Concept B — Offset Channels  
**Status:** **PASS** for the validated design and export criteria.  
**Scope:** Editable FigCraft design and exported art; application runtime integrations and legal trademark clearance excluded.

## Concept evaluation

| Concept | Strengths | Decision |
|---|---|---|
| A · Convergent Bands | Strong flow and directional energy | Not selected: tends to resemble a directional fork at small sizes |
| B · Offset Channels | Simple, balanced counterflow; memorable open center; no obvious productivity cliché; preserves detail at 16 px | **Selected** |
| C · Joined Modules | Architectural and modular | Not selected: top/bottom modular blocks read as separated pictograms |

All three were built as independent, editable vector paths in the same native master and rendered using FigCraft CLI. The selected shape was evaluated against light and dark lockups, macOS icon, pixel-size samples and monochrome scenarios.

## Verified checks

| Check | Result |
|---|---|
| Rebuild through `design/scripts/rebuild_master.py` | PASS |
| Canonical `.figcraft` file opens | PASS |
| Pages and frames | 17 pages; 224 original application frames + 11 brand frames |
| Native vector design | PASS; editable geometric contour layers |
| Existing design system | 49 variables in 3 collections |
| Missing fonts | 0 |
| Original application alignment audit | 0 problems |
| Native static design QA | PASS; 224 expected previews, 224 distinct image hashes |
| Existing application layout regression | PASS; sampled spacious shell and dark appearance rendered identically from pre-logo and post-logo masters |
| SVG and PDF signatures/openability | PASS; SVG parses, PDFs carry valid header/footer |
| Full app icon transparent corners | PASS (alpha=0 at (0,0)) |
| Seven approved square PNG sizes | PASS; 1024, 512, 256, 128, 64, 32, 16 |
| Extra QA size | 24 px visually inspected; not required as separate shipped export |
| macOS ICNS generation and round-trip decoding | PASS; 10 standard iconset images decoded |
| Contrast of `#266C56` on `#F6F7F5` | 5.81:1 |
| Contrast of `#B7D7C8` on `#162929` | 9.81:1 |
| Light, dark, grayscale and several surface specimens | Rendered and inspected |
| Repeatable asset generation | PASS; source generator and exporter retained |
| App/sidebar replacement | DONE following separate authorization on 2026-11-14; see `docs/BRAND_INTEGRATION.md` |
| Git commit/push/deployment | NONE |

## Evidence

- `design/brand/previews/concept-comparison.jpg` — side-by-side concept review
- `design/brand/previews/concept-final.png` — preferred visual identity board
- `design/brand/previews/size-and-surface-test.jpg` — literal pixel-size and surface specimens
- `design/brand/asset-manifest.json` — verified files, sizes and SHA-256 checksums
- `design/qa-results.json` — existing static FigCraft QA result
- `design/figcraft-info.json` — canonical master structure

## Boundaries and remaining work

The sidebar, favicon and Tauri bundle icons have now been integrated into the application code and verified with 60 passing browser tests (including onboarding), successful Svelte checks/build and a signed local macOS `.app` bundle. No menu-bar tray feature currently exists. System-wide installation, notarization and release publication remain out of scope. Distinctiveness review is visual only and does not constitute trademark clearance. Native UI runtime interactions were not exhaustively exercised; browser Playwright coverage and bundle signing were verified. See `docs/BRAND_INTEGRATION.md`.
