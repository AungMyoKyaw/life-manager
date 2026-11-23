"""Lucide icon geometry for the FigCraft master.

The design master draws real Lucide icons: the geometry is read from the
``lucide-svelte`` package the app already depends on, so the design and the
running app use exactly the same icon set.

``lucide-svelte`` stores each icon as a list of SVG primitives.  Only
``path`` elements can be handed to FigCraft directly, so the other
primitives are converted to path data here.
"""

import json
import pathlib
import re

_HERE = pathlib.Path(__file__).resolve().parent
ICONS_DIR = _HERE.parents[1] / "app" / "node_modules" / "lucide-svelte" / "dist" / "icons"
ALIASES_FILE = _HERE.parents[1] / "app" / "node_modules" / "lucide-svelte" / "dist" / "aliases" / "aliases.js"

_NODE_RE = re.compile(r"const iconNode = (\[.*?\]);", re.S)
_REEXPORT_RE = re.compile(r"export \{ default \} from \"\./([a-z0-9-]+)\.svelte\";")
_ALIAS_RE = re.compile(r"default as ([A-Za-z0-9_]+)\s*\}\s*from\s*'\.\./icons/([a-z0-9-]+)\.js'")

_cache = {}
_alias_cache = None


def _num(value):
    text = str(value).strip()
    try:
        return float(text)
    except ValueError:
        return 0.0


def _circle_path(cx, cy, r):
    """A circle as two half arcs.

    The sweep flag is 0 so the arc runs counter-clockwise on screen, which
    matches how the arc is drawn in the rest of the icon set.
    """
    return (f"M {cx - r} {cy} A {r} {r} 0 1 0 {cx + r} {cy} "
            f"A {r} {r} 0 1 0 {cx - r} {cy} Z")


def _ellipse_path(cx, cy, rx, ry):
    return (f"M {cx - rx} {cy} A {rx} {ry} 0 1 0 {cx + rx} {cy} "
            f"A {rx} {ry} 0 1 0 {cx - rx} {cy} Z")


def _rect_path(x, y, w, h, rx=0.0):
    if not rx:
        return f"M {x} {y} H {x + w} V {y + h} H {x} Z"
    return (f"M {x + rx} {y} H {x + w - rx} A {rx} {rx} 0 0 1 {x + w} {y + rx} "
            f"V {y + h - rx} A {rx} {rx} 0 0 1 {x + w - rx} {y + h} "
            f"H {x + rx} A {rx} {rx} 0 0 1 {x} {y + h - rx} "
            f"V {y + rx} A {rx} {rx} 0 0 1 {x + rx} {y} Z")


def _points_path(points, close=False):
    raw = str(points).replace(",", " ").split()
    pairs = [(_num(raw[i]), _num(raw[i + 1])) for i in range(0, len(raw) - 1, 2)]
    if not pairs:
        return ""
    path = "M " + " L ".join(f"{x} {y}" for x, y in pairs)
    return path + (" Z" if close else "")


def _node_to_path(tag, attrs):
    if tag == "path":
        return attrs.get("d", "")
    if tag == "circle":
        return _circle_path(_num(attrs["cx"]), _num(attrs["cy"]), _num(attrs["r"]))
    if tag == "ellipse":
        return _ellipse_path(_num(attrs["cx"]), _num(attrs["cy"]),
                             _num(attrs["rx"]), _num(attrs["ry"]))
    if tag == "rect":
        return _rect_path(_num(attrs.get("x", 0)), _num(attrs.get("y", 0)),
                          _num(attrs["width"]), _num(attrs["height"]),
                          _num(attrs.get("rx", 0)))
    if tag == "line":
        return (f"M {_num(attrs['x1'])} {_num(attrs['y1'])} "
                f"L {_num(attrs['x2'])} {_num(attrs['y2'])}")
    if tag in ("polyline", "polygon"):
        return _points_path(attrs.get("points", ""), close=(tag == "polygon"))
    raise ValueError("unsupported Lucide primitive: " + tag)


def _read_icon_source(name, _seen=None):
    """The raw iconNode list for a Lucide icon, by kebab-case name."""
    _seen = _seen or set()
    if name in _seen:
        raise FileNotFoundError("circular Lucide alias: " + name)
    _seen.add(name)

    svelte = ICONS_DIR / (name + ".svelte")
    if svelte.is_file():
        match = _NODE_RE.search(svelte.read_text())
        if match:
            return json.loads(match.group(1))

    # An alias ships only a .js re-export; follow it to the real icon file.
    reexport = ICONS_DIR / (name + ".js")
    if reexport.is_file():
        match = _REEXPORT_RE.search(reexport.read_text())
        if match:
            return _read_icon_source(match.group(1), _seen)

    target = _alias_target(name)
    if target:
        return _read_icon_source(target, _seen)
    raise FileNotFoundError("no Lucide icon named " + name)


def _alias_target(name):
    global _alias_cache
    if _alias_cache is None:
        _alias_cache = {}
        if ALIASES_FILE.is_file():
            for alias, target in _ALIAS_RE.findall(ALIASES_FILE.read_text()):
                _alias_cache[alias] = target
    return _alias_cache.get(name)


def aliases():
    """The Lucide alias table: PascalCase name -> kebab-case icon name."""
    global _alias_cache
    if _alias_cache is None:
        _alias_target("__init__")
    return dict(_alias_cache or {})


def lucide_path(name):
    """The SVG path data of a Lucide icon, in its own 24x24 coordinate space."""
    if name in _cache:
        return _cache[name]
    nodes = _read_icon_source(name)
    parts = [_node_to_path(tag, attrs) for tag, attrs in nodes]
    path = " ".join(p for p in parts if p)
    _cache[name] = path
    return path


def _kebab(name):
    """PascalCase -> kebab-case, matching the Lucide package's file names.

    ``LayoutDashboard`` becomes ``layout-dashboard`` and ``Clock3`` becomes
    ``clock-3``, which is how the icons are named on disk.
    """
    step = re.sub(r"(?<=[a-z0-9])(?=[A-Z])", "-", name)
    step = re.sub(r"(?<=[A-Za-z])(?=[0-9])", "-", step)
    return step.lower()


def resolve(name):
    """Resolve a PascalCase or kebab-case Lucide name to a kebab-case name."""
    if "-" in name:
        return name
    target = _alias_target(name)
    if target:
        return target
    kebab = _kebab(name)
    if (ICONS_DIR / (kebab + ".svelte")).is_file():
        return kebab
    if (ICONS_DIR / (kebab + ".js")).is_file():
        return kebab
    return kebab


def icon_path(name):
    """The SVG path data for a Lucide icon given either naming style."""
    return lucide_path(resolve(name))
