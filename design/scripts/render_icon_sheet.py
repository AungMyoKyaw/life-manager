"""Render the app's Lucide icons through the production build code path.

This uses ``build_figcraft.Doc.icon`` so the sheet shows exactly what the
design master contains.
"""
import json
import pathlib
import subprocess
import sys
import tempfile

HERE = pathlib.Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
import build_figcraft as bf


def main():
    names = []
    for f in (HERE.parents[1] / "app" / "src").rglob("*.svelte"):
        import re
        for b in re.findall(r"import\s*\{([^}]*)\}\s*from\s*'lucide-svelte'",
                            f.read_text()):
            for n in b.split(","):
                n = n.strip()
                if n and n not in names:
                    names.append(n)

    doc = json.loads((HERE / "figcraft-template.json").read_text())
    doc["name"] = "icon-sheet"
    doc["nodes"] = {
        "1": {"id": 1, "type": "page", "name": "Page 1", "children": [2],
              "width": 0.0, "height": 0.0,
              "background": {"r": 1, "g": 1, "b": 1, "a": 1}},
        "2": {"id": 2, "type": "frame", "name": "Sheet", "parent": 1, "children": [],
              "width": 1200, "height": 900, "clip": True,
              "fills": [{"type": "solid", "color": {"r": 1, "g": 1, "b": 1, "a": 1},
                         "opacity": 1, "blend": "normal", "visible": True}]},
    }
    doc["pages"] = [1]
    doc["current_page"] = 1
    nid = 3
    per = 10
    cell = 116
    for i, name in enumerate(names):
        col = i % per
        row = i // per
        x = 20 + col * cell
        y = 20 + row * 110
        node = {
            "id": nid, "type": "vector", "name": name,
            "network": None, "width": 0.0, "height": 0.0,
            "transform": [1.0, 0.0, 0.0, 1.0, float(x), float(y)],
            "fills": [],
            "strokes": [{"type": "solid", "color": {"r": 0.098, "g": 0.161, "b": 0.165, "a": 1},
                         "opacity": 1, "blend": "normal", "visible": True}],
            "stroke": {"weight": 2.0, "align": "center", "sides": None,
                       "cap_start": "round", "cap_end": "round", "join": "round",
                       "miter_limit": 4.0, "dashes": []},
        }
        doc["nodes"][str(nid)] = node
        doc["nodes"]["2"]["children"].append(nid)
        nid += 1

    # Build the icon geometry with the production helper.
    D = bf.Doc()
    D.j = doc
    D.n = nid
    for i, name in enumerate(names):
        col = i % per
        row = i // per
        D.icon(2, name, 20 + col * cell, 20 + row * 110, 56.0, "ink", 2.0)

    doc["next_id"] = D.n
    work = pathlib.Path(tempfile.mkdtemp())
    (work / "d.json").write_text(json.dumps(doc))
    subprocess.run(["figcraft-cli", "convert", str(work / "d.json"), str(work / "d.figcraft")],
                   check=True, capture_output=True)
    subprocess.run(["figcraft-cli", "export", str(work / "d.figcraft"), "--node", "2",
                    "--out", "/tmp/production-icons.png", "--scale", "1"],
                   check=True, capture_output=True)
    print("rendered", len(names), "icons -> /tmp/production-icons.png")


if __name__ == "__main__":
    main()
