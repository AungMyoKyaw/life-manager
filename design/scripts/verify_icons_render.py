"""Render Lucide icons with the local parser and with figcraft-cli, then
compare the two images pixel by pixel.

This is the end-to-end check that the icons in the design master are the
real Lucide geometry.
"""
import json
import pathlib
import subprocess
import sys
import tempfile

HERE = pathlib.Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
from svgpath import path_network

INK = {"r": 0.098, "g": 0.161, "b": 0.165, "a": 1}
STROKE = {"weight": 1.8, "align": "center", "sides": None, "cap_start": "round",
          "cap_end": "round", "join": "round", "miter_limit": 4.0, "dashes": []}


def my_icon_node(name, size=48.0):
    """The icon as FigCraft stores it: vertices relative to their origin.

    FigCraft scales a path to the width and height it is given, so the
    geometry here is scaled the same way for the comparison to be exact.
    """
    import lucide
    data = lucide.icon_path(name)
    net, bbox = path_network(data, normalize=False)
    # FigCraft stretches the path's own box onto the requested width and
    # height, so the geometry is scaled the same way here.
    scale_x = size / bbox[2] if bbox[2] else 1.0
    scale_y = size / bbox[3] if bbox[3] else 1.0
    for v in net["vertices"]:
        v["x"] *= scale_x
        v["y"] *= scale_y
    for s in net["segments"]:
        s["ta"] = [s["ta"][0] * scale_x, s["ta"][1] * scale_y]
        s["tb"] = [s["tb"][0] * scale_x, s["tb"][1] * scale_y]
    min_x = min(v["x"] for v in net["vertices"])
    min_y = min(v["y"] for v in net["vertices"])
    max_x = max(v["x"] for v in net["vertices"])
    max_y = max(v["y"] for v in net["vertices"])
    for v in net["vertices"]:
        v["x"] -= min_x
        v["y"] -= min_y
    return net, max_x - min_x, max_y - min_y, min_x, min_y


def build_doc(items):
    doc = json.loads((HERE / "figcraft-template.json").read_text())
    doc["name"] = "icons"
    doc["nodes"] = {
        "1": {"id": 1, "type": "page", "name": "Page 1", "children": [2],
              "width": 0.0, "height": 0.0,
              "background": {"r": 1, "g": 1, "b": 1, "a": 1}},
        "2": {"id": 2, "type": "frame", "name": "Sheet", "parent": 1, "children": [],
              "width": 1100, "height": 400, "clip": True,
              "fills": [{"type": "solid", "color": {"r": 1, "g": 1, "b": 1, "a": 1},
                         "opacity": 1, "blend": "normal", "visible": True}]},
    }
    doc["pages"] = [1]
    doc["current_page"] = 1
    nid = 3
    for i, (name, net, w, h, ox, oy) in enumerate(items):
        x = 20 + (i % 10) * 108
        y = 20 + (i // 10) * 108
        doc["nodes"][str(nid)] = {
            "id": nid, "type": "vector", "name": name, "parent": 2,
            "network": net, "width": w, "height": h,
            "transform": [1.0, 0.0, 0.0, 1.0, x + ox, y + oy],
            "fills": [],
            "strokes": [{"type": "solid", "color": INK, "opacity": 1,
                         "blend": "normal", "visible": True}],
            "stroke": STROKE,
        }
        doc["nodes"]["2"]["children"].append(nid)
        nid += 1
    doc["next_id"] = nid
    return doc


def figcraft_items(names, work, size=48.0):
    """The same icons built by figcraft-cli itself, at the same size."""
    script = []
    for i, n in enumerate(names):
        script.append({
            "command": "node.create",
            "params": {
                "type": "vector", "parent": 2, "x": 0, "y": 0, "name": n,
                "path": __import__("lucide").icon_path(n),
                "width": size, "height": size,
                "props": {"fills": [], "strokes": ["#19292A"], "strokeWeight": 1.8,
                          "strokeCap": "round", "strokeJoin": "round"},
            },
        })
    (work / "s.json").write_text(json.dumps(script))
    subprocess.run(
        ["figcraft-cli", "run", str(work / "s.json"), "--in", str(work / "base.figcraft"),
         "--out", str(work / "o.figcraft"), "--automation-read-root", str(work),
         "--automation-write-root", str(work)], capture_output=True, text=True, check=True)
    subprocess.run(["figcraft-cli", "convert", str(work / "o.figcraft"), str(work / "o.json")],
                   capture_output=True, text=True, check=True)
    d = json.loads((work / "o.json").read_text())
    out = []
    by_name = {}
    for v in d["nodes"].values():
        if v.get("type") == "vector":
            by_name[v["name"]] = v
    for i, n in enumerate(names):
        v = by_name[n]
        out.append((n, v["network"], v["width"], v["height"],
                    v["transform"][4], v["transform"][5]))
    return out


def main():
    import lucide
    if len(sys.argv) > 1 and sys.argv[1] == "--all":
        import re
        names = []
        for f in (HERE.parents[1] / "app" / "src").rglob("*.svelte"):
            for b in re.findall(r"import\s*\{([^}]*)\}\s*from\s*'lucide-svelte'",
                                f.read_text()):
                for n in b.split(","):
                    n = n.strip()
                    if n and n not in names:
                        names.append(n)
    else:
        names = sys.argv[1:] or ["check", "circle", "search", "calendar", "inbox", "sun"]
    work = pathlib.Path(tempfile.mkdtemp())
    subprocess.run(["figcraft-cli", "convert", str(HERE / "figcraft-template.json"),
                    str(work / "base.figcraft")], capture_output=True, text=True, check=True)

    mine = []
    for n in names:
        net, w, h, ox, oy = my_icon_node(n)
        mine.append((n, net, w, h, ox, oy))
    ref = figcraft_items(names, work)

    for tag, items in (("mine", mine), ("fig", ref)):
        doc = build_doc(items)
        (work / f"d-{tag}.json").write_text(json.dumps(doc))
        subprocess.run(["figcraft-cli", "convert", str(work / f"d-{tag}.json"),
                        str(work / f"d-{tag}.figcraft")], capture_output=True, text=True, check=True)
        subprocess.run(["figcraft-cli", "export", str(work / f"d-{tag}.figcraft"), "--node", "2",
                        "--out", str(work / f"{tag}.png"), "--scale", "2"],
                       capture_output=True, text=True, check=True)

    from PIL import Image, ImageChops
    a = Image.open(work / "mine.png").convert("L")
    b = Image.open(work / "fig.png").convert("L")
    if a.size != b.size:
        print("SIZE MISMATCH", a.size, b.size)
        return ["<size>"]
    diff = ImageChops.difference(a, b)
    bbox = diff.getbbox()
    changed = sum(1 for v in diff.getdata() if v > 60)
    ratio = changed / (a.size[0] * a.size[1])
    print(f"icons={len(names)} changed_px={changed} ratio={ratio:.5f} bbox={bbox}")
    print("RESULT:", "IDENTICAL" if ratio < 0.001 else "DIFFERENT")
    return [] if ratio < 0.001 else ["<diff>"]


if __name__ == "__main__":
    main()
