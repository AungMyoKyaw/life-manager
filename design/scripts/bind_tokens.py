"""Bind the design master to its design tokens.

Every painted layer in the FigCraft master is bound to a colour variable, so
changing one token repaints the whole document.  This pass:

1. creates the token variable collections (colour with a Light and a Dark
   mode, plus radius and spacing);
2. binds each layer's fill, stroke and corner radius to its token;
3. re-resolves every bound layer so the stored literals match the tokens.

The colour of a layer is read from the layer's own paint, so the token it
binds to is decided by the palette the view code asked for.
"""

import json
import pathlib
import sys

HERE = pathlib.Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
import tokens

ROOT = HERE.parents[1]
MASTER = ROOT / "design" / "life-manager-master.json"

# The palette key -> token name map, mirroring build_figcraft.py.
TOKEN_FOR = {
    "paper": "surface", "ink": "text", "ink2": "text-secondary",
    "sub": "text-muted", "faint": "text-faint", "nav": "surface-inverse",
    "nav2": "surface-inverse-raised", "mint": "accent-muted",
    "green": "accent", "green2": "accent-soft-text", "rust": "warning",
    "peach": "warning-muted", "yellow": "yellow-muted", "blue": "info",
    "violet": "violet-muted", "red": "danger", "bg": "canvas",
    "line": "border", "paper2": "surface-raised",
}

# Hex values that are not design tokens: they are surface variants and
# text colours on dark surfaces.  They are bound to the closest token so a
# token change still reaches them.
NEUTRAL_TOKEN = {
    "#FFFFFF": "text-inverse",
    "#F8FCF7": "text-inverse",
    "#EDF5F0": "text-inverse",
    "#F5FAF5": "text-inverse",
    "#F0F9F4": "text-inverse",
    "#EFF8F3": "text-inverse",
    "#F5FAF6": "text-inverse",
    "#F2F9F6": "text-inverse",
    "#F3F9F6": "text-inverse",
    "#E9F5EF": "text-inverse",
    "#E3F1E8": "text-inverse",
    "#ECF7F1": "icon-inverse-active",
    "#E6F1EA": "icon-inverse-active",
    "#D7E9E2": "text-inverse",
    "#D8F1E1": "accent-soft-text",
    "#B0CEC2": "icon-on-inverse",
    "#A8BFB7": "icon-on-inverse",
    "#BFD2CA": "icon-on-inverse",
    "#B9D2C7": "icon-on-inverse",
    "#91AFA4": "icon-on-inverse",
    "#91ACA3": "icon-on-inverse",
    "#A9C0B7": "icon-on-inverse",
    "#C3D6CF": "text-inverse",
    "#7CA69A": "icon-inverse",
    "#72958A": "icon-muted",
    "#335A4B": "surface-selected",
    "#243B3A": "surface-inverse-raised",
    "#436057": "icon-inverse",
    "#92BFB0": "icon-on-inverse",
    "#9FC9BA": "icon-on-inverse",
    "#9DB9AE": "icon-on-inverse",
    "#75B49C": "accent-text",
    "#8FC4AE": "accent-soft-text",
    "#75B49C": "accent-text",
    "#DDEEE5": "accent-muted",
    "#B7D7C8": "accent-quiet",
    "#3D7364": "accent-quiet",
    "#E9F4ED": "accent-muted",
    "#F5F8F5": "surface-raised",
    "#F2F6F3": "surface-raised",
    "#F6F8F6": "surface-raised",
    "#F6F8F5": "surface-raised",
    "#F4F8F5": "surface-raised",
    "#F3F7F4": "surface-raised",
    "#F3F8F4": "surface-raised",
    "#F8FAF8": "surface-raised",
    "#F0F5F1": "surface-raised",
    "#EFF2F0": "surface-sunken",
    "#EEF3EF": "surface-sunken",
    "#EEF2EF": "surface-sunken",
    "#F0F4F1": "surface-raised",
    "#FCFDFC": "surface",
    "#F6F8F7": "surface-sunken",
    "#E4EAE5": "border",
    "#DEE6DF": "border",
    "#DFE8E2": "border",
    "#DDE5E0": "border",
    "#F9F1EF": "danger-muted",
    "#F8F3ED": "warning-muted",
    "#B9D8CB": "accent-quiet",
    "#F2F8F3": "accent-muted",
    "#DDECE3": "accent-muted",
    "#E9F3EC": "accent-muted",
    "#F3F8F4": "surface-raised",
    "#D4E9DF": "icon-inverse-active",
    "#1B3537": "surface-inverse-raised",
    "#102124": "surface-inverse",
    "#0B191C": "surface-inverse",
    "#58A88C": "accent-soft-text",
    "#274F45": "surface-selected",
    "#3B9571": "accent-soft-text",
    "#8E9E9D": "icon-muted",
    "#A64F47": "danger",
    "#647779": "text-muted",
    "#19292A": "text",
    "#314245": "text-secondary",
    "#8DA09A": "text-faint",
    "#E4E9E6": "border",
    "#F5E8DD": "warning-muted",
    "#F3E8C8": "yellow-muted",
    "#DFEBF2": "info",
    "#EAE6F4": "violet-muted",
    "#C16A4C": "warning",
    "#F3F7F4": "surface-raised",
    "#AACBBC": "accent-quiet",
    "#3F6E58": "accent",
    "#1E5B48": "accent-hover",
}

# Colours that are part of the Light theme's own palette: the token that
# holds exactly this value is preferred over the neutral map.
VALUE_TO_TOKEN = {}
for name, (light, _dark, _desc) in tokens.COLORS.items():
    VALUE_TO_TOKEN.setdefault(light.upper(), name)
    VALUE_TO_TOKEN.setdefault(_dark.upper(), name)


def hex_of(color):
    return "#%02X%02X%02X" % (
        round(color["r"] * 255), round(color["g"] * 255), round(color["b"] * 255)
    )


def token_for_color(color):
    """The design token that owns a paint colour."""
    value = hex_of(color).upper()
    if value in VALUE_TO_TOKEN:
        return VALUE_TO_TOKEN[value]
    if value in NEUTRAL_TOKEN:
        return NEUTRAL_TOKEN[value]
    return None


def radius_token(value):
    """The radius token that matches a corner radius, if any."""
    for name, (size, _desc) in tokens.RADII.items():
        if abs(size - value) < 0.01:
            return name
    return None


def build_variables():
    """The FigCraft variable collections for the token table."""
    collections = []
    variables = []

    collections.append({
        "id": "VariableCollectionId:1",
        "name": "Color",
        "modes": [
            {"id": "VariableCollectionId:1/mode1", "name": "Light"},
            {"id": "VariableCollectionId:1/mode2", "name": "Dark"},
        ],
    })
    for i, (name, (light, dark, desc)) in enumerate(tokens.COLORS.items(), start=1):
        variables.append({
            "id": f"VariableID:{i}",
            "collection": "VariableCollectionId:1",
            "name": name,
            "resolvedType": "color",
            "description": desc,
            "values": {
                "VariableCollectionId:1/mode1": {
                    "type": "color",
                    "value": tokens.hex_to_rgba(light),
                },
                "VariableCollectionId:1/mode2": {
                    "type": "color",
                    "value": tokens.hex_to_rgba(dark),
                },
            },
        })

    radius_base = len(tokens.COLORS)
    collections.append({
        "id": "VariableCollectionId:2",
        "name": "Radius",
        "modes": [{"id": "VariableCollectionId:2/mode1", "name": "Default"}],
    })
    for i, (name, (value, desc)) in enumerate(tokens.RADII.items(), start=radius_base + 1):
        variables.append({
            "id": f"VariableID:{i}",
            "collection": "VariableCollectionId:2",
            "name": name,
            "resolvedType": "number",
            "description": desc,
            "values": {"VariableCollectionId:2/mode1": {"type": "number", "value": value}},
        })

    space_base = radius_base + len(tokens.RADII)
    collections.append({
        "id": "VariableCollectionId:3",
        "name": "Space",
        "modes": [{"id": "VariableCollectionId:3/mode1", "name": "Default"}],
    })
    for i, (name, (value, desc)) in enumerate(tokens.SPACING.items(), start=space_base + 1):
        variables.append({
            "id": f"VariableID:{i}",
            "collection": "VariableCollectionId:3",
            "name": name,
            "resolvedType": "number",
            "description": desc,
            "values": {"VariableCollectionId:3/mode1": {"type": "number", "value": value}},
        })

    return {"collections": collections, "variables": variables}


def variable_id_for(token_name):
    if token_name in tokens.COLORS:
        return f"VariableID:{list(tokens.COLORS).index(token_name) + 1}"
    if token_name in tokens.RADII:
        base = len(tokens.COLORS)
        return f"VariableID:{base + list(tokens.RADII).index(token_name) + 1}"
    return None


def bind_master(doc):
    """Bind every painted layer to its design token."""
    nodes = doc["nodes"]
    stats = {"fills": 0, "strokes": 0, "radius": 0, "unbound": 0, "text": 0}
    for key, node in nodes.items():
        if node.get("type") == "page":
            continue
        bound = node.get("bound", {})

        for field, index in (("fills", 0), ("strokes", 0)):
            paints = node.get(field)
            if not paints or not isinstance(paints, list):
                continue
            paint = paints[index]
            color = paint.get("color")
            if not color:
                continue
            token = token_for_color(color)
            if token:
                var = variable_id_for(token)
                if var:
                    bound[f"{field}/0/color"] = var
                    stats["fills" if field == "fills" else "strokes"] += 1
            elif field == "fills":
                stats["unbound"] += 1

        radius = node.get("corner_radius")
        if radius is None and isinstance(node.get("corners"), dict):
            radii = node["corners"].get("radii") or []
            if radii:
                radius = radii[0]
        if radius:
            token = radius_token(radius)
            if token:
                var = variable_id_for(token)
                if var:
                    bound["cornerRadius"] = var
                    stats["radius"] += 1

        if bound:
            node["bound"] = bound
        if node.get("type") == "text":
            stats["text"] += 1
    return stats


def main():
    doc = json.loads(MASTER.read_text())
    doc["variables"] = build_variables()
    stats = bind_master(doc)
    MASTER.write_text(json.dumps(doc, ensure_ascii=False, separators=(",", ":")))
    print("tokens bound:", stats)
    print("variables:", len(doc["variables"]["variables"]),
          "collections:", len(doc["variables"]["collections"]))


if __name__ == "__main__":
    main()
