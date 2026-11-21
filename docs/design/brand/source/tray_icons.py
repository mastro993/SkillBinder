"""Draws assets/tray: SkillBinder's rune, the one-color 18-unit template glyph `GLYPHS.bindrune` in
docs/design/brand/source/icons.js, stroked 1.4 units wide with round caps and joins. Coverage is
16x16 supersampled. The Windows and Linux glyphs carry a one-pixel halo of the other color, which
disappears on the panel they are picked for and keeps them visible on one they guess wrong, such
as GNOME's top bar, dark under a light theme. Each glyph has a dot variant, shown while something
waits for the user: a solid dot at the top right, in the glyph's color, cut out of the rune by a
clear gap (spec D6).

Usage: python3 docs/design/brand/source/tray_icons.py assets/tray"""
import math, struct, sys, zlib

# M9 1.75v14.5  M9 5.5l3.5 3.5L9 12.5 5.5 9Z  M9 1.75l3 2.3  M9 16.25l-3-2.3
SEGMENTS = [
    ((9, 1.75), (9, 16.25)),
    ((9, 5.5), (12.5, 9)), ((12.5, 9), (9, 12.5)), ((9, 12.5), (5.5, 9)), ((5.5, 9), (9, 5.5)),
    ((9, 1.75), (12, 4.05)),
    ((9, 16.25), (6, 13.95)),
]
STROKE = 1.4
# The dot, in glyph units: its center, its radius, and the clear gap around it.
DOT = ((15.2, 2.8), 2.4, 0.9)

def distance(px, py, a, b):
    (ax, ay), (bx, by) = a, b
    dx, dy = bx - ax, by - ay
    t = max(0.0, min(1.0, ((px - ax) * dx + (py - ay) * dy) / (dx * dx + dy * dy)))
    return math.hypot(px - (ax + t * dx), py - (ay + t * dy))

def draw(size, glyph_px, color, halo=None, dot=False):
    scale = glyph_px / 18
    offset = (size - glyph_px) / 2
    segments = [tuple((x * scale + offset, y * scale + offset) for x, y in s) for s in SEGMENTS]
    radius = STROKE * scale / 2
    (cx, cy), dot_radius, gap = DOT
    center = (cx * scale + offset, cy * scale + offset)
    n = 16
    def inside(px, py, grow):
        """Whether a point is on the mark grown by `grow` pixels: the stroke outside the dot's gap,
        or the dot."""
        if not dot:
            return any(distance(px, py, a, b) <= radius + grow for a, b in segments)
        from_dot = math.hypot(px - center[0], py - center[1])
        if from_dot <= dot_radius * scale + grow:
            return True
        return from_dot > (dot_radius + gap) * scale - grow and any(
            distance(px, py, a, b) <= radius + grow for a, b in segments)
    def cover(x, y, grow):
        hits = 0
        for i in range(n):
            for j in range(n):
                if inside(x + (i + 0.5) / n, y + (j + 0.5) / n, grow):
                    hits += 1
        return hits / (n * n)
    rows = []
    for y in range(size):
        row = bytearray([0])
        for x in range(size):
            glyph = cover(x, y, 0)
            ring = cover(x, y, 1) * 0.85 if halo else 0
            alpha = glyph + ring * (1 - glyph)
            if alpha == 0:
                row += bytes(4)
                continue
            opacity = round(255 * alpha)
            # An opaque pixel is all stroke: a trace of halo rounded into it would tint the glyph.
            mixed = color if opacity == 255 else [round((c * glyph + h * ring * (1 - glyph)) / alpha)
                                                  for c, h in zip(color, halo or color)]
            row += bytes(mixed) + bytes([opacity])
        rows.append(bytes(row))
    raw = b"".join(rows)
    def chunk(kind, data):
        return struct.pack(">I", len(data)) + kind + data + struct.pack(">I", zlib.crc32(kind + data))
    header = struct.pack(">IIBBBBB", size, size, 8, 6, 0, 0, 0)
    return b"\x89PNG\r\n\x1a\n" + chunk(b"IHDR", header) + chunk(b"IDAT", zlib.compress(raw, 9)) + chunk(b"IEND", b"")

out = sys.argv[1]
# macOS: a template drawn at 22 pt from 44 px, the glyph at its own 18 pt.
for dot, suffix in ((False, ""), (True, "-dot")):
    open(f"{out}/tray-template{suffix}.png", "wb").write(draw(44, 36, (0, 0, 0), dot=dot))
    for size in (16, 20, 24, 32):
        black, white = (0, 0, 0), (255, 255, 255)
        open(f"{out}/tray-black{suffix}-{size}.png", "wb").write(draw(size, size, black, white, dot))
        open(f"{out}/tray-white{suffix}-{size}.png", "wb").write(draw(size, size, white, black, dot))
