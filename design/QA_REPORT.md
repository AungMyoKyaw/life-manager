# FigCraft design QA report

Result: **PASS** (static design verification only).

| Check | Observed |
|---|---|
| Native pages | 17 |
| Named top-level frames | 235 |
| Native nodes | 20384 |
| Exported frame PNGs | 224 |
| Distinct rendered PNGs | 224 |
| Contact sheets | 16 |
| Requirement ID mappings | 200 |
| Layers exceeding 1440 × 900 frame | 0 |
| Missing fonts | [] |
| Design token collections | 3 |
| Design token variables | 49 |
| Layers bound to a token | 20351 |
| Lucide icon layers | 2916 |
| Visual-group alignment problems | 0 |

## Explicit limits

This QA validates the static design file, frame inventories, exported PNG sizes, screenshot uniqueness, layer containment, traceability references, font availability, the design-token bindings and the alignment of each visual group. All 16 contact sheets have been visually reviewed and the design revised where major semantic mismatches were found. It does not verify runtime accessibility, executable buttons, drag/drop, calendar recurrence, desktop or CLI behavior.

## Repeat

1. Run the design generator.
2. Convert JSON to native FigCraft and refresh figcraft-info.json.
3. Export all screenshots and contact sheets.
4. Run python3 design/scripts/make_coverage.py.
5. Run python3 design/scripts/validate_design.py.

The actual current outputs are also in design/qa-results.json.
