#!/usr/bin/env python3
"""Machine-derived evidence: map every FigCraft board to an app surface and a domain action.

Classification is textual and reproducible, not a judgement call:
  UI      -> at least one significant board token appears in app/src (view/label/aria)
  DOMAIN  -> a board token appears in crates/life-core or crates/life-cli
  NONE    -> no token found in either tree
A board can be both UI and DOMAIN. This is a LOWER BOUND on coverage: a token hit
proves a word exists in the source, not that the board's behavior is implemented.
NONE means no word overlap at all, so those boards definitely have no surface.
"""
import json, pathlib, re
ROOT = pathlib.Path(__file__).resolve().parents[1]
D = ROOT / "design"
STOP = set("""and the of a an to in for with your my me at on by from is are be it
this that you all new open view detail panel screen state page list create edit
""".split())

def norm(s):
    s = s.lower().replace("·", " ")
    s = re.sub(r"[^a-z0-9 ]+", " ", s)
    return [w for w in s.split() if len(w) > 3 and w not in STOP]

def tree_text(paths, suffixes):
    text = {}
    for p in paths:
        if not p.exists():
            continue
        for f in p.rglob("*"):
            if f.suffix not in suffixes:
                continue
            if "node_modules" in str(f) or "/target/" in str(f) or "/dist/" in str(f):
                continue
            try:
                text[str(f)] = f.read_text(errors="ignore").lower()
            except Exception:
                pass
    return text

app = tree_text([ROOT / "crates" / "life-gpui" / "src"], {".rs"})
core = tree_text([ROOT / "crates" / "life-core" / "src", ROOT / "crates" / "life-cli" / "src"], {".rs"})
app_blob = "\n".join(app.values())
core_blob = "\n".join(core.values())

inv = json.loads((D / "screen-inventory.json").read_text())
rows = []
for b in inv:
    toks = norm(b["title"])
    ui_hits = sorted({t for t in toks if t in app_blob})
    core_hits = sorted({t for t in toks if t in core_blob})
    rows.append({
        "board_id": b["board_id"], "page": b["page"], "title": b["title"],
        "ui": bool(ui_hits), "domain": bool(core_hits),
        "ui_tokens": ui_hits, "domain_tokens": core_hits,
        "status": "UI+DOMAIN" if ui_hits and core_hits else ("UI" if ui_hits else ("DOMAIN" if core_hits else "NONE")),
    })
none = [r for r in rows if r["status"] == "NONE"]
ui_only = [r for r in rows if r["status"] == "UI"]
dom_only = [r for r in rows if r["status"] == "DOMAIN"]
both = [r for r in rows if r["status"] == "UI+DOMAIN"]
out = {
    "boards": len(rows),
    "ui_and_domain": len(both),
    "ui_only": len(ui_only),
    "domain_only": len(dom_only),
    "no_surface_found": len(none),
    "method": "token overlap between board titles and app/src + crates source text",
    "limitation": "Textual lower bound. A token hit proves a word exists in the source, not that the board's behavior is implemented. NONE means no word overlap at all.",
    "no_surface_found_boards": [{"board_id": r["board_id"], "page": r["page"], "title": r["title"]} for r in none],
    "rows": rows,
}
dest = ROOT / "qa" / "board-coverage.json"
dest.write_text(json.dumps(out, indent=2))
print(json.dumps({k: v for k, v in out.items() if k not in ("rows", "no_surface_found_boards")}, indent=2))
print()
print("NO SURFACE FOUND (%d):" % len(none))
for r in none:
    print("  %03d  %-30s %s" % (r["board_id"], r["page"], r["title"]))
