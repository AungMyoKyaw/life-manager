"""Compare the local path parser with figcraft-cli's own import, structurally."""
import json
import pathlib
import subprocess
import sys
import tempfile

HERE = pathlib.Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
from svgpath import path_network


def figcraft_vector(path_data, work):
    script = [{
        "command": "node.create",
        "params": {
            "type": "vector", "parent": 2, "x": 0, "y": 0, "name": "T",
            "path": path_data,
            "props": {"fills": [], "strokes": ["#19292A"], "strokeWeight": 2,
                      "strokeCap": "round", "strokeJoin": "round"},
        },
    }]
    (work / "s.json").write_text(json.dumps(script))
    r = subprocess.run(
        ["figcraft-cli", "run", str(work / "s.json"), "--in", str(work / "base.figcraft"),
         "--out", str(work / "o.figcraft"), "--automation-read-root", str(work),
         "--automation-write-root", str(work)],
        capture_output=True, text=True,
    )
    if r.returncode != 0:
        raise RuntimeError(r.stdout + r.stderr)
    subprocess.run(["figcraft-cli", "convert", str(work / "o.figcraft"), str(work / "o.json")],
                   capture_output=True, text=True, check=True)
    d = json.loads((work / "o.json").read_text())
    for v in d["nodes"].values():
        if v.get("type") == "vector" and v.get("name") == "T":
            return v["network"], v["width"], v["height"], v["transform"][4], v["transform"][5]
    raise RuntimeError("figcraft produced no vector")


def my_vector(path_data):
    net, bbox = path_network(path_data, normalize=False)
    min_x, min_y, w, h = bbox
    for v in net["vertices"]:
        v["x"] -= min_x
        v["y"] -= min_y
    return net, w, h, min_x, min_y


def compare(mine, ref, tol=0.05):
    """Describe the first structural difference between two networks."""
    mnet, mw, mh, mdx, mdy = mine
    rnet, rw, rh, rdx, rdy = ref
    problems = []
    if abs(mw - rw) > tol:
        problems.append(f"width {mw:.3f} vs {rw:.3f}")
    if abs(mh - rh) > tol:
        problems.append(f"height {mh:.3f} vs {rh:.3f}")
    if abs(mdx - rdx) > tol:
        problems.append(f"origin x {mdx:.3f} vs {rdx:.3f}")
    if abs(mdy - rdy) > tol:
        problems.append(f"origin y {mdy:.3f} vs {rdy:.3f}")
    mv, rv = mnet["vertices"], rnet["vertices"]
    ms, rs = mnet["segments"], rnet["segments"]
    if len(mv) != len(rv):
        problems.append(f"vertices {len(mv)} vs {len(rv)}")
    else:
        for i, (a, b) in enumerate(zip(mv, rv)):
            if abs(a["x"] - b["x"]) > tol or abs(a["y"] - b["y"]) > tol:
                problems.append(f"vertex {i} ({a['x']:.3f},{a['y']:.3f}) vs ({b['x']:.3f},{b['y']:.3f})")
                break
    if len(ms) != len(rs):
        problems.append(f"segments {len(ms)} vs {len(rs)}")
    else:
        for i, (a, b) in enumerate(zip(ms, rs)):
            if a["a"] != b["a"] or a["b"] != b["b"]:
                problems.append(f"segment {i} links ({a['a']},{a['b']}) vs ({b['a']},{b['b']})")
                break
            for k in ("ta", "tb"):
                if abs(a[k][0] - b[k][0]) > tol or abs(a[k][1] - b[k][1]) > tol:
                    problems.append(f"segment {i} {k} {a[k]} vs {b[k]}")
                    break
    return problems


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

    bad = []
    for n in names:
        data = lucide.icon_path(n)
        problems = compare(my_vector(data), figcraft_vector(data, work))
        if problems:
            bad.append(n)
            print(f"DIFF {n:24s} {problems[0]}")
            if len(problems) > 1:
                print(f"      {problems[1]}")
    print(f"compared {len(names)} icons; mismatched: {len(bad)}")
    return bad


if __name__ == "__main__":
    main()
