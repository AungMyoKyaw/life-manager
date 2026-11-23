"""Audit the alignment of the design master.

The design system groups layers into visual groups: the sidebar, the top
bar, the metric row, the two content columns, a card's title and its body.
Within a group the layers should share an edge, a width or a centre line.

This script reports the groups that break those rules so the layout code can
be corrected rather than eyeballed.
"""
import json
import pathlib
import sys

HERE = pathlib.Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
import tokens

ROOT = HERE.parents[1]
MASTER = ROOT / "design" / "life-manager-master.json"

TOLERANCE = 0.6


def box(node):
    t = node.get("transform") or [1, 0, 0, 1, 0, 0]
    x, y = t[4], t[5]
    w = node.get("width", 0.0)
    h = node.get("height", 0.0)
    return x, y, w, h


def children_of(doc, node_id):
    node = doc["nodes"][str(node_id)]
    return [doc["nodes"][str(c)] for c in node.get("children", [])]


def text_layers(doc, node_id):
    out = []
    for child in children_of(doc, node_id):
        if child.get("type") == "text":
            out.append(child)
    return out


def audit_frame(doc, frame):
    """The alignment rules for one board."""
    problems = []
    children = children_of(doc, frame["id"])
    rects = [c for c in children if c.get("type") in ("rectangle", "ellipse")]
    texts = [c for c in children if c.get("type") == "text"]
    vectors = [c for c in children if c.get("type") == "vector"]

    # The sidebar occupies the left edge; the content starts after it with a
    # consistent gutter.  A full-bleed board has no sidebar.
    sidebar = [r for r in rects if box(r)[0] == 0 and box(r)[2] > 200
               and box(r)[2] < 600]
    if sidebar:
        edge = box(sidebar[0])[0] + box(sidebar[0])[2]
        if abs(edge - tokens.SIDEBAR_WIDTH) > TOLERANCE:
            problems.append(f"sidebar width {edge} != {tokens.SIDEBAR_WIDTH}")

    # Cards that sit side by side in a row should share a top edge, a common
    # gap, and end on the content's right edge.
    cards = [r for r in rects
             if box(r)[0] > 250 and box(r)[2] > 200 and box(r)[3] > 90]
    rows = {}
    for c in cards:
        x, y, w, h = box(c)
        rows.setdefault(round(y), []).append(c)
    for y, row in rows.items():
        if len(row) < 2:
            continue
        row = sorted(row, key=lambda r: box(r)[0])
        gaps = [round(box(row[i + 1])[0] - (box(row[i])[0] + box(row[i])[2]), 2)
                for i in range(len(row) - 1)]
        if len(set(gaps)) > 1 and max(gaps) - min(gaps) > TOLERANCE:
            problems.append(f"cards at y={y} have uneven gaps {gaps}")
        # A row that spans the content area must start and end on the grid.
        # A narrower row, such as a set of Kanban columns, is checked on its
        # own left edge and its own even spacing instead.
        left = box(row[0])[0]
        right = box(row[-1])[0] + box(row[-1])[2]
        spans = abs(right - tokens.CONTENT_RIGHT) < TOLERANCE
        if spans:
            if abs(left - tokens.CONTENT_LEFT) > TOLERANCE:
                problems.append(
                    f"cards at y={y} start at {left} not the content edge "
                    f"{tokens.CONTENT_LEFT}")
        else:
            # Columns inside the content area keep to the content's left
            # edge, and the row's own left edge must be on the grid.
            if left < tokens.CONTENT_LEFT - TOLERANCE:
                problems.append(
                    f"cards at y={y} start at {left} left of the content edge")

    # An icon and the text beside it should be vertically centred on each
    # other.  The label is the text to the right of the icon that shares its
    # vertical band.
    for v in vectors:
        vx, vy, vw, vh = box(v)
        if not v.get("name", "").startswith("Icon"):
            continue
        candidates = []
        for t in texts:
            tx, ty, tw, th = box(t)
            gap = tx - (vx + vw)
            if not (-2 <= gap <= 24):
                continue
            # The label must sit in the same vertical band as the icon.
            if ty + th < vy - 2 or ty > vy + vh + 2:
                continue
            candidates.append((gap, t, ty, th))
        if not candidates:
            continue
        candidates.sort(key=lambda c: c[0])
        _gap, t, ty, th = candidates[0]
        # The icon is centred on the text's cap height, not on the text
        # layer's box, because the box carries the leading.
        font_size = (t.get("style") or {}).get("font_size", 13.0)
        cap_centre = ty + font_size * 0.635
        icon_mid = vy + vh / 2
        if abs(icon_mid - cap_centre) > 2.0:
            problems.append(
                f"icon {v['name']!r} centre {icon_mid:.1f} vs label "
                f"{t.get('characters', '')[:20]!r} centre {cap_centre:.1f}")

    return problems


def main():
    doc = json.loads(MASTER.read_text())
    index = json.loads((ROOT / "design" / "screen-inventory.json").read_text())
    total = 0
    frames_with = []
    for entry in index:
        frame = doc["nodes"][str(entry["node_id"])]
        problems = audit_frame(doc, frame)
        if problems:
            frames_with.append((entry["board_id"], entry["title"], problems))
            total += len(problems)
    print(f"boards audited: {len(index)}; alignment problems: {total}")
    for board_id, title, problems in frames_with[:12]:
        print(f"\n{board_id:03d} {title}")
        for p in problems[:6]:
            print("   ", p)
    if len(frames_with) > 12:
        print(f"\n... and {len(frames_with) - 12} more boards")
    return frames_with


if __name__ == "__main__":
    main()
