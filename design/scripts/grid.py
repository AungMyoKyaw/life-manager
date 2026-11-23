"""The layout grid for the design master.

Every board places its layers through these helpers, so layers in a visual
group share an edge, a width or a centre line, and boards that use the same
grid align with each other.
"""

import tokens

LEFT = tokens.CONTENT_LEFT          # 290
RIGHT = tokens.CONTENT_RIGHT        # 1402
WIDTH = tokens.CONTENT_WIDTH        # 1112
GAP = tokens.CARD_GAP               # 16


def columns(count, gap=GAP, left=LEFT, total=WIDTH):
    """The x position and width of each column in an even split."""
    return tokens.content_columns([1.0] * count, gap, left, total)


def metric_row(left=LEFT, gap=tokens.METRIC_GAP, total=WIDTH):
    """The three metric cards, sharing the content width."""
    return tokens.content_columns(list(tokens.METRIC_WIDTHS), gap, left, total)


def body_columns(left=LEFT, gap=tokens.BODY_GAP, total=WIDTH):
    """The wide body column and the narrow side column."""
    return tokens.content_columns(list(tokens.BODY_WIDTHS), gap, left, total)


def stack(widths, gap=GAP, left=LEFT):
    """Place a sequence of items left to right, returning (x, width) pairs."""
    out = []
    x = left
    for w in widths:
        out.append((x, w))
        x += w + gap
    return out


def centre(outer_left, outer_width, inner_width):
    """The x that centres an inner element inside an outer one."""
    return outer_left + (outer_width - inner_width) / 2.0


def icon_y(text_y, text_height, icon_size, font_size=13.0):
    """The y that vertically centres an icon beside a line of text.

    A text layer's box carries the leading below the baseline, so the icon is
    centred on the text's cap height rather than on the box.  For the UI
    face the cap top sits 0.27 of the font size below the box top and the
    baseline one font size below it, which puts the cap centre at 0.635.
    """
    cap_centre = text_y + font_size * 0.635
    return cap_centre - icon_size / 2.0


def icon_x(text_x, icon_size, gap):
    """The x that places an icon a fixed gap to the left of its label."""
    return text_x - gap - icon_size
