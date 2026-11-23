"""Single source of truth for the Life Manager design system.

The FigCraft master, the exported CSS and the running app all read their
values from this module, so changing a token here updates the design master
and the app together.

Colour tokens live in a FigCraft variable collection with a ``Light`` and a
``Dark`` mode.  Every painted layer in the master binds its fill or stroke to
one of these variables, so ``variable.set`` repaints the whole document.
"""

# --------------------------------------------------------------------------
# Colour tokens
# --------------------------------------------------------------------------
# name -> (light, dark, description)
COLORS = {
    # Surfaces
    "canvas": ("#F6F7F5", "#112124", "Page background"),
    "surface": ("#FFFFFF", "#1C3132", "Card and panel surface"),
    "surface-raised": ("#F5F8F5", "#203638", "Recessed surface inside a card"),
    "surface-sunken": ("#EFF2F0", "#182B2C", "Column and track background"),
    "surface-inverse": ("#162929", "#0B191C", "Navigation surface"),
    "surface-inverse-raised": ("#243B3A", "#122A2C", "Raised navigation surface"),
    "surface-selected": ("#335A4B", "#274F45", "Selected navigation item"),
    # Lines
    "border": ("#E4E9E6", "#334747", "Default hairline border"),
    "border-strong": ("#D5DDD8", "#415756", "Emphasised border"),
    # Text
    "text": ("#19292A", "#EDF6F0", "Primary text"),
    "text-secondary": ("#314245", "#D5E5DC", "Secondary text"),
    "text-muted": ("#647779", "#A4B9B0", "Muted text and labels"),
    "text-faint": ("#8E9E9D", "#849D92", "Faintest text"),
    "text-inverse": ("#FFFFFF", "#F1F8F3", "Text on dark surfaces"),
    "text-on-accent": ("#FFFFFF", "#0E1F1B", "Text on the accent colour"),
    # Accents
    "accent": ("#266C56", "#419677", "Primary accent"),
    "accent-hover": ("#1E5B48", "#57A88A", "Primary accent hover"),
    "accent-muted": ("#DDEEE5", "#2B5448", "Accent surface"),
    "accent-quiet": ("#B7D7C8", "#3D7364", "Quiet accent surface"),
    "accent-text": ("#266C56", "#75B49C", "Accent coloured text"),
    "accent-soft-text": ("#3B9571", "#8FC4AE", "Secondary accent text"),
    # Status
    "warning": ("#C16A4C", "#ECAA89", "Needs attention"),
    "warning-muted": ("#F5E8DD", "#4C3830", "Needs attention surface"),
    "warning-text": ("#C16A4C", "#ECAA89", "Needs attention text"),
    "danger": ("#A64F47", "#E08C82", "Destructive action"),
    "danger-muted": ("#F9F1EF", "#4A3230", "Destructive surface"),
    "info": ("#DFEBF2", "#294653", "Informational surface"),
    "info-text": ("#314245", "#A9CBD8", "Informational text"),
    "violet-muted": ("#EAE6F4", "#3B3552", "Secondary accent surface"),
    "yellow-muted": ("#F3E8C8", "#4A4128", "Reserved time surface"),
    # Icons and navigation
    "icon": ("#19292A", "#EDF6F0", "Default icon colour"),
    "icon-muted": ("#8E9E9D", "#849D92", "Muted icon colour"),
    "icon-inverse": ("#7CA69A", "#8FB3A5", "Navigation icon colour"),
    "icon-inverse-active": ("#ECF7F1", "#E7F6ED", "Active navigation icon colour"),
    "icon-on-accent": ("#FFFFFF", "#0E1F1B", "Icon on the accent colour"),
    "icon-on-inverse": ("#A8BFB7", "#9FBBB0", "Secondary text on dark surfaces"),
}

# --------------------------------------------------------------------------
# Radius tokens
# --------------------------------------------------------------------------
# name -> (value, description)
RADII = {
    "sm": (6.0, "Small control"),
    "md": (9.0, "Button and chip"),
    "lg": (13.0, "Card"),
    "xl": (17.0, "Dialog"),
    "pill": (999.0, "Fully rounded"),
}

# --------------------------------------------------------------------------
# Type scale
# --------------------------------------------------------------------------
# name -> (size, weight, line height, description)
TYPE_SCALE = {
    "display": (32.0, 700, 38.0, "Screen title"),
    "title": (24.0, 700, 30.0, "Dialog and section title"),
    "heading": (19.0, 700, 25.0, "Card title"),
    "body": (13.0, 400, 19.0, "Default body"),
    "body-strong": (13.0, 700, 19.0, "Emphasised body"),
    "label": (11.0, 700, 15.0, "Form and table label"),
    "caption": (10.0, 400, 14.0, "Supporting caption"),
    "eyebrow": (10.0, 800, 14.0, "Uppercase section eyebrow"),
    "mono": (12.0, 400, 19.0, "Terminal output"),
}

# --------------------------------------------------------------------------
# Spacing scale
# --------------------------------------------------------------------------
# name -> (value, description)
SPACING = {
    "1": (4.0, "Tight gap"),
    "2": (8.0, "Icon to label"),
    "3": (12.0, "Inner padding"),
    "4": (16.0, "Card padding"),
    "5": (20.0, "Card padding wide"),
    "6": (24.0, "Section gap"),
    "7": (32.0, "Section gap wide"),
    "8": (40.0, "Page gutter"),
}

# --------------------------------------------------------------------------
# Layout grid
# --------------------------------------------------------------------------
# Every board uses the same grid so that layers in different boards line up
# with each other.
CANVAS = {"width": 1440.0, "height": 900.0}
SIDEBAR_WIDTH = 252.0
CONTENT_GUTTER = 38.0
CONTENT_LEFT = SIDEBAR_WIDTH + CONTENT_GUTTER          # 290
CONTENT_RIGHT = CANVAS["width"] - CONTENT_GUTTER       # 1402
CONTENT_WIDTH = CONTENT_RIGHT - CONTENT_LEFT           # 1112
CARD_GAP = 16.0

# The metric row: three cards that together fill the content width.
METRIC_GAP = 15.0
METRIC_WIDTHS = (337.0, 337.0, 438.0)

# The body row: a wide column and a narrow column.
BODY_GAP = 16.0
BODY_WIDTHS = (688.0, 408.0)


def content_columns(widths, gap=CARD_GAP, left=CONTENT_LEFT, total=CONTENT_WIDTH):
    """Place columns that together fill the content width exactly.

    The widths are scaled so that the columns and the gaps between them add
    up to the content width, which keeps the right edge on the grid.
    """
    gaps = gap * (len(widths) - 1)
    scale = (total - gaps) / sum(widths)
    widths = [w * scale for w in widths]
    out = []
    x = left
    for w in widths[:-1]:
        out.append((x, w))
        x += w + gap
    out.append((x, widths[-1]))
    return out


def _hex_to_rgba(value):
    value = value.lstrip("#")
    return {
        "r": int(value[0:2], 16) / 255.0,
        "g": int(value[2:4], 16) / 255.0,
        "b": int(value[4:6], 16) / 255.0,
        "a": 1.0,
    }


def hex_to_rgba(value):
    return _hex_to_rgba(value)


def rgba(value):
    """Legacy helper kept for the build scripts: a hex string to an rgba map."""
    return _hex_to_rgba(value)


def paint(color, opacity=1.0):
    """A solid FigCraft paint for a hex colour."""
    return [
        {
            "type": "solid",
            "color": _hex_to_rgba(color),
            "opacity": opacity,
            "blend": "normal",
            "visible": True,
        }
    ]


def css_variables():
    """The token table as CSS custom properties for the light theme."""
    out = {}
    for name, (light, _dark, _desc) in COLORS.items():
        out["--" + name] = light
    for name, (value, _desc) in RADII.items():
        out["--radius-" + name] = f"{value:g}px"
    for name, (size, _weight, _line, _desc) in TYPE_SCALE.items():
        out["--text-" + name] = f"{size:g}px"
    for name, (value, _desc) in SPACING.items():
        out["--space-" + name] = f"{value:g}px"
    return out


def css_dark_variables():
    """The token table as CSS custom properties for the dark theme."""
    out = {}
    for name, (_light, dark, _desc) in COLORS.items():
        out["--" + name] = dark
    return out


def css_block(selector, values, indent=""):
    body = ";".join(f"{k}:{v}" for k, v in values.items())
    return f"{indent}{selector}{{{body}}}"


def _rust_ident(name):
    return name.replace("-", "_")


def _hex_u32(value):
    return "0x" + value.lstrip("#").upper()


def rust_tokens_source():
    """Generate the Rust design-token module for `crates/life-gpui`."""
    lines = [
        "//! Design tokens generated from `design/scripts/tokens.py`.",
        "//!",
        "//! Run `python3 design/scripts/sync_tokens.py` after updating token values.",
        "#![allow(dead_code)]",
        "",
        "use gpui_kit::{Rgba, rgb};",
        "",
        "#[derive(Clone, Copy, Debug, PartialEq, Eq)]",
        "pub enum ThemeMode {",
        "    Light,",
        "    Dark,",
        "}",
        "",
        "#[derive(Clone, Copy, Debug)]",
        "pub struct Palette {",
    ]
    for name, (_light, _dark, desc) in COLORS.items():
        lines.append(f"    /// {desc}")
        lines.append(f"    pub {_rust_ident(name)}: u32,")
    lines += [
        "}",
        "",
        "impl Palette {",
        "    pub const fn for_mode(mode: ThemeMode) -> Self {",
        "        match mode {",
        "            ThemeMode::Light => LIGHT,",
        "            ThemeMode::Dark => DARK,",
        "        }",
        "    }",
    ]
    for name in COLORS:
        ident = _rust_ident(name)
        lines.append(f"    pub fn {ident}(&self) -> Rgba {{")
        lines.append(f"        rgb(self.{ident})")
        lines.append("    }")
    lines += [
        "}",
        "",
        "pub const LIGHT: Palette = Palette {",
    ]
    for name, (light, _dark, _desc) in COLORS.items():
        lines.append(f"    {_rust_ident(name)}: {_hex_u32(light)},")
    lines += [
        "};",
        "",
        "pub const DARK: Palette = Palette {",
    ]
    for name, (_light, dark, _desc) in COLORS.items():
        lines.append(f"    {_rust_ident(name)}: {_hex_u32(dark)},")
    lines += [
        "};",
        "",
    ]
    for name, (value, desc) in RADII.items():
        lines.append(f"/// {desc}")
        lines.append(f"pub const RADIUS_{_rust_ident(name).upper()}: f32 = {value:.1f};")
    lines.append("")
    for name, (size, _weight, _line, desc) in TYPE_SCALE.items():
        lines.append(f"/// {desc}")
        lines.append(f"pub const TEXT_{_rust_ident(name).upper()}: f32 = {size:.1f};")
    lines.append("")
    for name, (value, desc) in SPACING.items():
        lines.append(f"/// {desc}")
        lines.append(f"pub const SPACE_{_rust_ident(name).upper()}: f32 = {value:.1f};")
    lines += [
        "",
        f"pub const CANVAS_WIDTH: f32 = {CANVAS['width']:.1f};",
        f"pub const CANVAS_HEIGHT: f32 = {CANVAS['height']:.1f};",
        f"pub const SIDEBAR_WIDTH: f32 = {SIDEBAR_WIDTH:.1f};",
        f"pub const CONTENT_GUTTER: f32 = {CONTENT_GUTTER:.1f};",
        f"pub const CARD_GAP: f32 = {CARD_GAP:.1f};",
        "",
    ]
    return "\n".join(lines)
