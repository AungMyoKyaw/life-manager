"""Convert SVG path data into FigCraft vector networks.

FigCraft stores a vector as a network of vertices plus the segments between
them.  A straight segment has zero tangents; a curve stores the outgoing
tangent of its start vertex (``ta``) and the incoming tangent of its end
vertex (``tb``), both relative to those vertices.

The networks produced here are written straight into the design master's
JSON, so they carry the true Lucide geometry: arcs become exact circular
bezier segments and curves keep their control handles.

Vertex coordinates are relative to the path's own bounding-box origin, so a
layer's transform places the shape exactly where the caller asked for it.
"""

import math


class Point:
    """A network vertex with its incoming and outgoing tangent handles."""

    __slots__ = ("x", "y", "tin", "tout")

    def __init__(self, x, y, tin=(0.0, 0.0), tout=(0.0, 0.0)):
        self.x = x
        self.y = y
        self.tin = tin
        self.tout = tout


class PathParser:
    def __init__(self, data):
        self.data = data
        self.i = 0
        self.subpaths = []
        self.subpaths_closed = []
        self.controls = []
        self.arc_points = []
        self.current = None
        self.pos = (0.0, 0.0)
        self.start = (0.0, 0.0)
        self.last_cubic_ctrl = None
        self.last_quad_ctrl = None
        self.closed = False

    # -- lexer ------------------------------------------------------------
    def skip(self):
        while self.i < len(self.data) and self.data[self.i] in " ,\t\r\n":
            self.i += 1

    def peek_command(self):
        self.skip()
        if self.i >= len(self.data):
            return None
        c = self.data[self.i]
        return c if c.isalpha() else None

    def has_number(self):
        """True when the next token is a number, not a command."""
        self.skip()
        if self.i >= len(self.data):
            return False
        c = self.data[self.i]
        if c in "+-":
            return True
        return c.isdigit() or c == "."

    def read_number(self):
        self.skip()
        m = self.data
        start = self.i
        if self.i < len(m) and m[self.i] in "+-":
            self.i += 1
        digits = 0
        while self.i < len(m) and m[self.i].isdigit():
            self.i += 1
            digits += 1
        if self.i < len(m) and m[self.i] == ".":
            self.i += 1
            while self.i < len(m) and m[self.i].isdigit():
                self.i += 1
                digits += 1
        if digits == 0:
            raise ValueError("expected a number at %d in %r" % (self.i, self.data))
        if self.i < len(m) and m[self.i] in "eE":
            save = self.i
            self.i += 1
            if self.i < len(m) and m[self.i] in "+-":
                self.i += 1
            exp_digits = 0
            while self.i < len(m) and m[self.i].isdigit():
                self.i += 1
                exp_digits += 1
            if exp_digits == 0:
                self.i = save
        return float(m[start:self.i])

    def read_flag(self):
        self.skip()
        c = self.data[self.i]
        self.i += 1
        return c == "1"

    # -- geometry ---------------------------------------------------------
    def move_to(self, x, y):
        self.flush()
        self.pos = (x, y)
        self.start = (x, y)
        self.current = [Point(x, y)]
        self.closed = False
        self.subpaths.append(self.current)
        self.subpaths_closed.append(False)

    def flush(self):
        self.current = None
        self.last_cubic_ctrl = None
        self.last_quad_ctrl = None

    def line_to(self, x, y):
        self.ensure()
        self.current.append(Point(x, y))
        self.pos = (x, y)

    def cubic_to(self, c1, c2, x, y):
        self.ensure()
        p0 = self.pos
        prev = self.current[-1]
        prev.tout = (c1[0] - p0[0], c1[1] - p0[1])
        self.current.append(Point(x, y, tin=(c2[0] - x, c2[1] - y)))
        self.controls.extend((c1, c2))
        self.pos = (x, y)
        self.last_cubic_ctrl = c2

    def quad_to(self, q, x, y):
        self.ensure()
        p0 = self.pos
        c1 = (p0[0] + 2.0 / 3.0 * (q[0] - p0[0]), p0[1] + 2.0 / 3.0 * (q[1] - p0[1]))
        c2 = (x + 2.0 / 3.0 * (q[0] - x), y + 2.0 / 3.0 * (q[1] - y))
        self.cubic_to(c1, c2, x, y)

    def ensure(self):
        if self.current is None:
            self.current = [Point(self.pos[0], self.pos[1])]
            self.closed = False
            self.subpaths.append(self.current)
            self.subpaths_closed.append(False)

    def close(self):
        if self.current is not None:
            self.closed = True
            for i, sub in enumerate(self.subpaths):
                if sub is self.current:
                    self.subpaths_closed[i] = True
                    break
        self.pos = self.start
        self.flush()

    # -- arcs -------------------------------------------------------------
    def arc_to(self, rx, ry, xrot, large, sweep, x, y):
        self.ensure()
        p0 = self.pos
        if rx == 0 or ry == 0:
            self.line_to(x, y)
            return
        rx = abs(rx)
        ry = abs(ry)
        phi = math.radians(xrot)
        cos_p, sin_p = math.cos(phi), math.sin(phi)

        dx2 = (p0[0] - x) / 2.0
        dy2 = (p0[1] - y) / 2.0
        x1 = cos_p * dx2 + sin_p * dy2
        y1 = -sin_p * dx2 + cos_p * dy2

        lam = x1 * x1 / (rx * rx) + y1 * y1 / (ry * ry)
        if lam > 1:
            scale = math.sqrt(lam)
            rx *= scale
            ry *= scale

        num = rx * rx * ry * ry - rx * rx * y1 * y1 - ry * ry * x1 * x1
        den = rx * rx * y1 * y1 + ry * ry * x1 * x1
        coef = math.sqrt(max(0.0, num / den)) if den else 0.0
        if large == sweep:
            coef = -coef
        cx1 = coef * rx * y1 / ry
        cy1 = -coef * ry * x1 / rx

        cx = cos_p * cx1 - sin_p * cy1 + (p0[0] + x) / 2.0
        cy = sin_p * cx1 + cos_p * cy1 + (p0[1] + y) / 2.0

        def angle(ux, uy, vx, vy):
            dot = ux * vx + uy * vy
            length = math.hypot(ux, uy) * math.hypot(vx, vy)
            if length == 0:
                return 0.0
            c = max(-1.0, min(1.0, dot / length))
            a = math.acos(c)
            if ux * vy - uy * vx < 0:
                a = -a
            return a

        theta1 = angle(1.0, 0.0, (x1 - cx1) / rx, (y1 - cy1) / ry)
        dtheta = angle((x1 - cx1) / rx, (y1 - cy1) / ry, (-x1 - cx1) / rx, (-y1 - cy1) / ry)
        if not sweep and dtheta > 0:
            dtheta -= 2 * math.pi
        elif sweep and dtheta < 0:
            dtheta += 2 * math.pi

        count = max(1, int(math.ceil(abs(dtheta) / (math.pi / 2))))
        delta = dtheta / count
        t = 4.0 / 3.0 * math.tan(delta / 4.0)

        # The true extremes of the ellipse inside this arc's angular range.
        # They are not the bezier control points, so they are collected here.
        for cand in (0.0, math.pi / 2, math.pi, 3 * math.pi / 2):
            px_ = cx + rx * math.cos(cand) * cos_p - ry * math.sin(cand) * sin_p
            py_ = cy + rx * math.cos(cand) * sin_p + ry * math.sin(cand) * cos_p
            if self._angle_in_range(cand, theta1, dtheta):
                self.arc_points.append((px_, py_))

        theta = theta1
        px, py = p0
        for _ in range(count):
            cos1, sin1 = math.cos(theta), math.sin(theta)
            theta2 = theta + delta
            cos2, sin2 = math.cos(theta2), math.sin(theta2)

            ex1 = cx + rx * cos_p * cos1 - ry * sin_p * sin1
            ey1 = cy + rx * sin_p * cos1 + ry * cos_p * sin1
            ex2 = cx + rx * cos_p * cos2 - ry * sin_p * sin2
            ey2 = cy + rx * sin_p * cos2 + ry * cos_p * sin2

            d1x = -rx * cos_p * sin1 - ry * sin_p * cos1
            d1y = -rx * sin_p * sin1 + ry * cos_p * cos1
            d2x = -rx * cos_p * sin2 - ry * sin_p * cos2
            d2y = -rx * sin_p * sin2 + ry * cos_p * cos2

            c1 = (ex1 + t * d1x, ey1 + t * d1y)
            c2 = (ex2 - t * d2x, ey2 - t * d2y)

            self.current[-1].tout = (c1[0] - px, c1[1] - py)
            self.current.append(Point(ex2, ey2, tin=(c2[0] - ex2, c2[1] - ey2)))
            px, py = ex2, ey2
            theta = theta2

        self.pos = (x, y)
        self.last_cubic_ctrl = None

    @staticmethod
    def _angle_in_range(angle, theta1, dtheta):
        """True when an ellipse parameter lies inside the arc's sweep."""
        lo = theta1
        hi = theta1 + dtheta
        if dtheta < 0:
            lo, hi = hi, lo
        a = angle
        while a < lo - 1e-9:
            a += 2 * math.pi
        while a > hi + 1e-9:
            a -= 2 * math.pi
        return lo - 1e-9 <= a <= hi + 1e-9

    # -- driver -----------------------------------------------------------
    def parse(self):
        command = None
        while True:
            self.skip()
            if self.i >= len(self.data):
                break
            cmd = self.peek_command()
            if cmd is None:
                if command is None:
                    break
                if command in "Zz":
                    break
                # An implicit command repeats the previous one.  A moveto
                # repeats as an absolute lineto; every other command repeats
                # as itself, keeping its own relative or absolute form.
                cmd = "L" if command == "M" else command
            else:
                self.i += 1
            relative = cmd.islower()
            upper = cmd.upper()

            if upper == "M":
                x = self.read_number()
                y = self.read_number()
                if relative:
                    x += self.pos[0]
                    y += self.pos[1]
                self.move_to(x, y)
                # Further coordinate pairs after a moveto are linetos.  They
                # are absolute when the moveto was absolute, and relative
                # when the moveto was relative.
                command = "l" if relative else "L"
            elif upper == "L":
                x = self.read_number()
                y = self.read_number()
                if relative:
                    x += self.pos[0]
                    y += self.pos[1]
                self.line_to(x, y)
                command = cmd
            elif upper == "H":
                x = self.read_number()
                if relative:
                    x += self.pos[0]
                self.line_to(x, self.pos[1])
                command = cmd
            elif upper == "V":
                y = self.read_number()
                if relative:
                    y += self.pos[1]
                self.line_to(self.pos[0], y)
                command = cmd
            elif upper == "C":
                c1x = self.read_number()
                c1y = self.read_number()
                c2x = self.read_number()
                c2y = self.read_number()
                x = self.read_number()
                y = self.read_number()
                if relative:
                    c1x += self.pos[0]
                    c1y += self.pos[1]
                    c2x += self.pos[0]
                    c2y += self.pos[1]
                    x += self.pos[0]
                    y += self.pos[1]
                self.cubic_to((c1x, c1y), (c2x, c2y), x, y)
                command = cmd
            elif upper == "S":
                c2x = self.read_number()
                c2y = self.read_number()
                x = self.read_number()
                y = self.read_number()
                if relative:
                    c2x += self.pos[0]
                    c2y += self.pos[1]
                    x += self.pos[0]
                    y += self.pos[1]
                p0 = self.pos
                if self.last_cubic_ctrl:
                    c1x = 2 * p0[0] - self.last_cubic_ctrl[0]
                    c1y = 2 * p0[1] - self.last_cubic_ctrl[1]
                else:
                    c1x, c1y = p0
                self.cubic_to((c1x, c1y), (c2x, c2y), x, y)
                command = cmd
            elif upper == "Q":
                qx = self.read_number()
                qy = self.read_number()
                x = self.read_number()
                y = self.read_number()
                if relative:
                    qx += self.pos[0]
                    qy += self.pos[1]
                    x += self.pos[0]
                    y += self.pos[1]
                self.quad_to((qx, qy), x, y)
                command = cmd
            elif upper == "T":
                x = self.read_number()
                y = self.read_number()
                if relative:
                    x += self.pos[0]
                    y += self.pos[1]
                p0 = self.pos
                if self.last_quad_ctrl:
                    qx = 2 * p0[0] - self.last_quad_ctrl[0]
                    qy = 2 * p0[1] - self.last_quad_ctrl[1]
                else:
                    qx, qy = p0
                self.quad_to((qx, qy), x, y)
                command = cmd
            elif upper == "A":
                rx = self.read_number()
                ry = self.read_number()
                xrot = self.read_number()
                large = self.read_flag()
                sweep = self.read_flag()
                x = self.read_number()
                y = self.read_number()
                if relative:
                    x += self.pos[0]
                    y += self.pos[1]
                self.arc_to(rx, ry, xrot, large, sweep, x, y)
                command = cmd
            elif upper == "Z":
                self.close()
                command = cmd
            else:
                raise ValueError("unsupported path command %r" % cmd)
        self.flush()
        return self.subpaths


def _subpath_network(points, closed):
    """A network for one subpath.

    A closed subpath gets a wrap segment from its last vertex back to its
    first; an open one does not, so no stray line is drawn across it.
    """
    # A path that returns to its start leaves a duplicate last vertex; merge
    # it into the first one instead of emitting a zero-length segment.
    if len(points) > 1:
        first, last = points[0], points[-1]
        if abs(first.x - last.x) < 1e-9 and abs(first.y - last.y) < 1e-9:
            if closed:
                first.tin = last.tin
                points = points[:-1]
            else:
                # An open path whose last point returns to its first is a
                # closed shape drawn without the z command.
                first.tin = last.tin
                points = points[:-1]
                closed = True
    vertices = []
    segments = []
    for p in points:
        vertices.append({"radius": 0.0, "x": p.x, "y": p.y})
    n = len(points)
    last_segment = n if closed else n - 1
    for i in range(last_segment):
        a = i
        b = (i + 1) % n
        segments.append({
            "a": a,
            "b": b,
            "ta": list(points[i].tout),
            "tb": list(points[b].tin),
        })
    return vertices, segments


def path_network(data, normalize=True, scale=1.0, dx=0.0, dy=0.0):
    """SVG path data -> (network, bbox).

    With ``normalize`` the vertices are relative to the path's bounding-box
    origin and the caller must offset the transform by that origin, which is
    what FigCraft itself does.  With ``normalize=False`` the vertices keep
    their own coordinate space, so a 24x24 icon can be scaled in place.
    """
    parser = PathParser(data)
    subpaths = parser.parse()
    closed_flags = parser.subpaths_closed
    vertices = []
    segments = []
    xs = []
    ys = []
    # The box of a path includes its curve control points, so a curve that
    # bows past its endpoints still fits inside the layer.
    for c in parser.controls:
        xs.append(c[0])
        ys.append(c[1])
    for c in parser.arc_points:
        xs.append(c[0])
        ys.append(c[1])
    for index, sub in enumerate(subpaths):
        if len(sub) < 2:
            continue
        closed = closed_flags[index] if index < len(closed_flags) else False
        vs, ss = _subpath_network(sub, closed)
        base = len(vertices)
        vertices.extend(vs)
        for s in ss:
            segments.append({"a": s["a"] + base, "b": s["b"] + base,
                             "ta": s["ta"], "tb": s["tb"]})
        for p in sub:
            xs.append(p.x)
            ys.append(p.y)
    if not xs:
        return {"vertices": [], "segments": [], "regions": []}, (0.0, 0.0, 0.0, 0.0)
    min_x, min_y = min(xs), min(ys)
    max_x, max_y = max(xs), max(ys)
    # The box is measured after scaling, so it describes the geometry that
    # is actually written out.
    min_x, min_y = min_x * scale + dx, min_y * scale + dy
    max_x, max_y = max_x * scale + dx, max_y * scale + dy
    bbox = (min_x, min_y, max_x - min_x, max_y - min_y)
    for v in vertices:
        v["x"] = v["x"] * scale + dx
        v["y"] = v["y"] * scale + dy
    # The tangent handles are offsets, so they scale with the geometry.
    for s in segments:
        s["ta"] = [s["ta"][0] * scale, s["ta"][1] * scale]
        s["tb"] = [s["tb"][0] * scale, s["tb"][1] * scale]
    network = {"vertices": vertices, "segments": segments, "regions": []}
    return network, bbox
