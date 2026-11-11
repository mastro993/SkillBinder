"""Pixel diff of a reference render against a native GPUI screenshot.

usage: pixdiff <ref.png> <native.png> <out-diff.png> [--crop x,y,w,h] [--ignore x,y,w,h ...]
               [--tolerance N] [--scale S] [--side out-side.png] [--no-snap] [--json]
               [--icon x,y,w,h ... [--icon-cap N] [--icon-slack N]]
               [--page ref-base.png native-base.png --fill x,y,w,h,r]

Rects are logical px in window coordinates (multiplied by --scale, default 2), also with
--crop. Both images must have the same size (the native window must be 1180x728 at 2x).
A pixel mismatches when its largest RGB channel delta exceeds --tolerance (default 0).
The window frame, which AppKit draws above the app, is left out and counted: every pixel that is
not opaque in either image (the rounded corners) and every pixel within RING device px of one of
them or of the image edge (the edge hairline), by 8-neighbour distance.

These numbers are reported:
  raw          every mismatching pixel
  snap         mismatching pixels, not antialiasing, that a shift of at most 1 device px explains
               both ways: in every channel the native value lies within the min..max (± tolerance)
               of the reference's 3x3 neighbourhood, and the reference value within the native's.
               GPUI puts icons and box edges on whole device pixels where Chrome draws them at
               fractional positions. A missing or extra line, a wrong fill or a 2 device px
               shift fails one direction and stays red. --no-snap turns the test off (snap = 0).
  icon         mismatching pixels, not antialiasing or snap, inside an SVG icon or logo box
               (--icon, from the reference's spec dump) whose origin is fractional in device px,
               taken 1 device px wider on each side. GPUI draws the sprite on whole device pixels,
               so a detail 1 or 2 device px wide can land at another sub-pixel phase (G18). A box
               keeps them only when there are at most --icon-cap (default 12) and each passes the
               snap test with --icon-slack more levels of tolerance (default 40); else all stay red.
  aa-filtered  raw minus antialiasing pixels (pixelmatch's detector: a pixel lying between a
               darker and a brighter neighbour, with at most 2 equal neighbours, next to a flat
               area in both images, applied to each image against the other), snap and icon.
               Isolated text-edge differences go; wrong colors, sizes and larger shifts stay.
  page         with --page and --fill: pixels outside an element's opaque fill (a rounded rect,
               x,y,w,h,radius; less than 1 device px inside its edge counts as outside) where the
               page under the element differs between the two bases by more than --tolerance.
               That is page content seen through a translucent ring or past rounded corners; it
               is left out like --ignore.
Writes out-diff.png (the reference faded to gray; red = mismatch, yellow = antialiasing only,
green = snap only, purple = icon only, blue = ignored, page or frame) and a side-by-side
ref | native | diff (default <out-diff>.side.png). Exit 0 after a comparison, 2 on bad input.
"""
import argparse
import json
import sys
from pathlib import Path

import numpy as np
from PIL import Image

OFFSETS = [(dy, dx) for dy in (-1, 0, 1) for dx in (-1, 0, 1) if (dy, dx) != (0, 0)]
RING = 2  # device px: the window's edge hairline, measured on #104's captures (spec/progress-4-parity.md)


def grow(m, n):
    """m dilated by n device px (each step adds the 8 neighbours); outside the image counts as False."""
    for _ in range(n):
        p = np.pad(m, 1)
        m = np.any([p[1 + dy:p.shape[0] - 1 + dy, 1 + dx:p.shape[1] - 1 + dx]
                    for dy in (-1, 0, 1) for dx in (-1, 0, 1)], axis=0)
    return m


def frame(opaque):
    """The window frame: not opaque in any image, or within RING device px of such a pixel or the image edge."""
    outside = np.pad(~np.logical_and.reduce(opaque), RING, constant_values=True)
    return grow(outside, RING)[RING:-RING, RING:-RING]


def rect(text):
    try:
        x, y, w, h = (float(v) for v in text.split(","))
    except ValueError:
        raise argparse.ArgumentTypeError(f"expected x,y,w,h, got {text!r}")
    if w <= 0 or h <= 0:
        raise argparse.ArgumentTypeError(f"empty rect {text!r}")
    return x, y, w, h


def rounded(text):
    try:
        x, y, w, h, r = (float(v) for v in text.split(","))
    except ValueError:
        raise argparse.ArgumentTypeError(f"expected x,y,w,h,radius, got {text!r}")
    return x, y, w, h, r


def physical(r, scale, bounds):
    """Logical rect -> physical (x0, y0, x1, y1), clipped to bounds (x0, y0, x1, y1)."""
    x, y, w, h = r
    bx0, by0, bx1, by1 = bounds
    x0, y0 = max(bx0, int(np.floor(x * scale))), max(by0, int(np.floor(y * scale)))
    x1, y1 = min(bx1, int(np.ceil((x + w) * scale))), min(by1, int(np.ceil((y + h) * scale)))
    return x0, y0, max(x0, x1), max(y0, y1)


def shifted(a, dy, dx, fill):
    """a[y + dy, x + dx] at each (y, x); outside the image = fill."""
    out = np.full_like(a, fill)
    h, w = a.shape[:2]
    out[max(0, -dy):h - max(0, dy), max(0, -dx):w - max(0, dx)] = a[max(0, dy):h - max(0, -dy), max(0, dx):w - max(0, -dx)]
    return out


def many_siblings(rgba):
    """True where a pixel has 3+ identical neighbours (an image edge counts as one)."""
    h, w = rgba.shape[:2]
    packed = rgba.astype(np.uint32)
    packed = (packed[..., 0] << 24) | (packed[..., 1] << 16) | (packed[..., 2] << 8) | packed[..., 3]
    count = np.zeros((h, w), np.int8)
    count[0, :] = count[-1, :] = 1
    count[:, 0] = count[:, -1] = 1
    for dy, dx in OFFSETS:
        count += (shifted(packed, dy, dx, 0xFFFFFFFF) == packed) & inside(h, w, dy, dx)
    return count > 2


def inside(h, w, dy, dx):
    m = np.zeros((h, w), bool)
    m[max(0, -dy):h - max(0, dy), max(0, -dx):w - max(0, dx)] = True
    return m


def antialiased(img, other, many_img, many_other):
    """pixelmatch's antialiasing test for every pixel of img, vectorized."""
    h, w = img.shape[:2]
    rgb = img[..., :3].astype(np.float32)
    luma = rgb[..., 0] * 0.29889531 + rgb[..., 1] * 0.58662247 + rgb[..., 2] * 0.11448223
    zeroes = np.zeros((h, w), np.int8)
    zeroes[0, :] = zeroes[-1, :] = 1
    zeroes[:, 0] = zeroes[:, -1] = 1
    lo, hi = np.zeros((h, w), np.float32), np.zeros((h, w), np.float32)
    lo_k, hi_k = np.full((h, w), -1, np.int8), np.full((h, w), -1, np.int8)
    for k, (dy, dx) in enumerate(OFFSETS):  # darkest and brightest neighbour, equal ones counted
        valid = inside(h, w, dy, dx)
        d = shifted(luma, dy, dx, 0) - luma
        zeroes += valid & (d == 0)
        darker, brighter = valid & (d < lo), valid & (d > hi)
        lo, lo_k = np.where(darker, d, lo), np.where(darker, k, lo_k)
        hi, hi_k = np.where(brighter, d, hi), np.where(brighter, k, hi_k)
    out = np.zeros((h, w), bool)
    for k, (dy, dx) in enumerate(OFFSETS):
        flat = shifted(many_img, dy, dx, False) & shifted(many_other, dy, dx, False)
        out |= ((lo_k == k) | (hi_k == k)) & flat
    return out & (zeroes <= 2) & (lo < 0) & (hi > 0)


def snapped(ref, nat, box, tolerance):
    """True where, per channel, each image's value lies within the other's 3x3 min..max (± tolerance).
    ref and nat are whole images and box the crop (x0, y0, x1, y1), so neighbours outside the crop
    count; the image edge repeats."""
    x0, y0, x1, y1 = box
    h, w = y1 - y0, x1 - x0
    ring = lambda img: np.pad(img[..., :3].astype(np.int16), ((1, 1), (1, 1), (0, 0)), mode="edge")[y0:y1 + 2, x0:x1 + 2]
    r, n = ring(ref), ring(nat)

    def within(v, around):
        views = [around[1 + dy:1 + dy + h, 1 + dx:1 + dx + w] for dy in (-1, 0, 1) for dx in (-1, 0, 1)]
        lo, hi = np.min(views, axis=0) - tolerance, np.max(views, axis=0) + tolerance
        v = v[1:1 + h, 1:1 + w]
        return ((v >= lo) & (v <= hi)).all(axis=2)

    return within(n, r) & within(r, n)


def outside_fill(shape, ox, oy, scale, fill):
    """True where a pixel's centre lies outside the rounded rect fill, or less than 1 device px inside it."""
    fx, fy, fw, fh, r = fill
    ys, xs = np.mgrid[0:shape[0], 0:shape[1]]
    qx = np.abs((ox + xs + 0.5) / scale - fx - fw / 2) - (fw / 2 - r)
    qy = np.abs((oy + ys + 0.5) / scale - fy - fh / 2) - (fh / 2 - r)
    d = np.hypot(np.maximum(qx, 0), np.maximum(qy, 0)) + np.minimum(np.maximum(qx, qy), 0) - r
    return d > -1 / scale


def icon_residue(real, near, boxes, scale, bounds, cap):
    """Split the red pixels inside fractional icon boxes into accepted residue, per box; near is
    the snap test with the slack added."""
    x0, y0, x1, y1 = bounds
    icon, report = np.zeros_like(real), []
    whole = lambda v: abs(v * scale - round(v * scale)) < 1e-6
    for box in boxes:
        if whole(box[0]) and whole(box[1]):
            continue
        bx, by, bw, bh = box
        m = 1 / scale
        bx0, by0, bx1, by1 = physical((bx - m, by - m, bw + 2 * m, bh + 2 * m), scale, bounds)
        inside = np.zeros_like(real)
        inside[by0 - y0:by1 - y0, bx0 - x0:bx1 - x0] = True
        found = real & inside & ~icon
        n = int(found.sum())
        ok = n <= cap and not (found & ~near).any()
        if n:
            report.append({"box": list(box), "pixels": n, "accepted": ok})
        if n and ok:
            icon |= found
    return icon, report


def bbox(mask, ox, oy, scale):
    ys, xs = np.nonzero(mask)
    if not len(xs):
        return None
    x0, y0, x1, y1 = (xs.min() + ox) / scale, (ys.min() + oy) / scale, (xs.max() + 1 + ox) / scale, (ys.max() + 1 + oy) / scale
    return [round(x0, 1), round(y0, 1), round(x1 - x0, 1), round(y1 - y0, 1)]


def main():
    p = argparse.ArgumentParser(prog="pixdiff", description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    p.add_argument("ref")
    p.add_argument("native")
    p.add_argument("out")
    p.add_argument("--crop", type=rect)
    p.add_argument("--ignore", type=rect, action="append", default=[])
    p.add_argument("--tolerance", type=int, default=0)
    p.add_argument("--scale", type=float, default=2.0)
    p.add_argument("--side")
    p.add_argument("--no-snap", action="store_true")
    p.add_argument("--icon", type=rect, action="append", default=[])
    p.add_argument("--icon-cap", type=int, default=12)
    p.add_argument("--icon-slack", type=int, default=40)
    p.add_argument("--page", nargs=2, metavar=("REF_BASE", "NATIVE_BASE"))
    p.add_argument("--fill", type=rounded)
    p.add_argument("--json", action="store_true")
    a = p.parse_args()
    if bool(a.page) != bool(a.fill):
        p.error("--page and --fill go together")

    ref = np.asarray(Image.open(a.ref).convert("RGBA"))
    nat = np.asarray(Image.open(a.native).convert("RGBA"))
    if ref.shape != nat.shape:
        print(f"pixdiff: sizes differ: ref {ref.shape[1]}x{ref.shape[0]}, native {nat.shape[1]}x{nat.shape[0]}", file=sys.stderr)
        sys.exit(2)
    H, W = ref.shape[:2]
    x0, y0, x1, y1 = physical(a.crop, a.scale, (0, 0, W, H)) if a.crop else (0, 0, W, H)
    if x1 <= x0 or y1 <= y0:
        print("pixdiff: the crop is outside the image", file=sys.stderr)
        sys.exit(2)
    near = False if a.no_snap else snapped(ref, nat, (x0, y0, x1, y1), a.tolerance)
    near_icon = snapped(ref, nat, (x0, y0, x1, y1), a.tolerance + a.icon_slack) if a.icon else None
    clear = frame([ref[..., 3] == 255, nat[..., 3] == 255])[y0:y1, x0:x1]
    ref, nat = ref[y0:y1, x0:x1], nat[y0:y1, x0:x1]
    h, w = ref.shape[:2]
    page = np.zeros((h, w), bool)
    if a.page:
        rb, nb = (np.asarray(Image.open(f).convert("RGB"))[y0:y1, x0:x1].astype(np.int16) for f in a.page)
        page = (np.abs(rb - nb).max(axis=2) > a.tolerance) & outside_fill((h, w), x0, y0, a.scale, a.fill)

    ignored = np.zeros((h, w), bool)
    for r in a.ignore:
        ix0, iy0, ix1, iy1 = physical(r, a.scale, (x0, y0, x1, y1))
        ignored[iy0 - y0:iy1 - y0, ix0 - x0:ix1 - x0] = True
    page &= ~ignored & ~clear
    compared = ~ignored & ~clear & ~page
    delta = np.abs(ref[..., :3].astype(np.int16) - nat[..., :3].astype(np.int16)).max(axis=2)
    raw = (delta > a.tolerance) & compared
    many_ref, many_nat = many_siblings(ref), many_siblings(nat)
    aa = raw & (antialiased(ref, nat, many_ref, many_nat) | antialiased(nat, ref, many_nat, many_ref))
    snap = raw & ~aa & near
    icon, icons = icon_residue(raw & ~aa & ~snap, near_icon, a.icon, a.scale, (x0, y0, x1, y1), a.icon_cap)
    real = raw & ~aa & ~snap & ~icon

    n = int(compared.sum())
    stat = lambda m: {"pixels": int(m.sum()), "percent": round(100 * m.sum() / n, 4) if n else 0.0,
                      "max_delta": int(delta[m].max()) if m.any() else 0, "bbox": bbox(m, x0, y0, a.scale)}
    side = a.side or str(Path(a.out).with_suffix("")) + ".side.png"
    result = {
        "size": [W, H], "scale": a.scale, "region_px": [x0, y0, x1 - x0, y1 - y0], "compared": n,
        "ignored": int(ignored.sum()), "frame": int((clear & ~ignored).sum()), "tolerance": a.tolerance,
        "max_delta": int(delta[compared].max()) if n else 0,
        "raw": stat(raw), "aa_filtered": stat(real), "antialiasing": int(aa.sum()), "snap": stat(snap),
        "icon": stat(icon), "icon_boxes": icons, "page": int(page.sum()), "diff": a.out, "side": side,
    }

    gray = ref[..., :3].astype(np.float32) @ np.array([0.299, 0.587, 0.114], np.float32)
    out = np.repeat((255 - (255 - gray) * 0.25)[..., None], 3, axis=2).astype(np.uint8)
    out[ignored | clear | page] = (205, 225, 255)
    out[aa] = (255, 196, 0)
    out[snap] = (0, 160, 80)
    out[icon] = (150, 60, 220)
    out[real] = (230, 0, 0)
    Image.fromarray(out, "RGB").save(a.out)
    gap = np.full((h, max(2, int(8 * a.scale)), 3), 128, np.uint8)
    Image.fromarray(np.hstack([ref[..., :3], gap, nat[..., :3], gap, out]), "RGB").save(side)

    if a.json:
        print(json.dumps(result, indent=1))
        return
    lg = lambda v: f"{v / a.scale:g}"
    print(f"region       {lg(x0)},{lg(y0)},{lg(x1 - x0)},{lg(y1 - y0)} logical ({x1 - x0}x{y1 - y0} px at {a.scale:g}x); "
          f"compared {n:,} px, ignored {result['ignored']:,}, page {result['page']:,}, frame {result['frame']:,}; "
          f"tolerance {a.tolerance}")
    for key, label in (("raw", "raw"), ("snap", "snap"), ("icon", "icon"), ("aa_filtered", "aa-filtered")):
        s = result[key]
        box = f"  bbox {','.join(f'{v:g}' for v in s['bbox'])} logical" if s["bbox"] else ""
        print(f"{label:<12} {s['pixels']:,} px mismatched ({s['percent']:.4f}%), max channel delta {s['max_delta']}{box}")
    print(f"antialiasing {result['antialiasing']:,} px; max channel delta over the region {result['max_delta']}")
    for b in icons:
        print(f"icon box     {','.join(f'{v:g}' for v in b['box'])}: {b['pixels']} px{'' if b['accepted'] else ', not accepted (over the cap or the slack)'}")
    print(f"diff         {a.out}\nside-by-side {side}")


if __name__ == "__main__":
    main()
