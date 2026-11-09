"""Pixel diff of a reference render against a native GPUI screenshot.

usage: pixdiff <ref.png> <native.png> <out-diff.png> [--crop x,y,w,h] [--ignore x,y,w,h ...]
               [--tolerance N] [--scale S] [--side out-side.png] [--no-snap] [--json]

Rects are logical px in window coordinates (multiplied by --scale, default 2), also with
--crop. Both images must have the same size (the native window must be 1180x728 at 2x).
A pixel mismatches when its largest RGB channel delta exceeds --tolerance (default 0).
Pixels that are not opaque in either image (macOS window corners) are left out and counted.

Three numbers are reported:
  raw          every mismatching pixel
  snap         mismatching pixels, not antialiasing, that a shift of at most 1 device px explains
               both ways: in every channel the native value lies within the min..max (± tolerance)
               of the reference's 3x3 neighbourhood, and the reference value within the native's.
               GPUI puts icons and box edges on whole device pixels where Chrome draws them at
               fractional positions. A missing or extra line, a wrong fill or a 2 device px
               shift fails one direction and stays red. --no-snap turns the test off (snap = 0).
  aa-filtered  raw minus antialiasing pixels (pixelmatch's detector: a pixel lying between a
               darker and a brighter neighbour, with at most 2 equal neighbours, next to a flat
               area in both images, applied to each image against the other) and minus snap.
               Isolated text-edge differences go; wrong colors, sizes and larger shifts stay.
Writes out-diff.png (the reference faded to gray; red = mismatch, yellow = antialiasing only,
green = snap only, blue = ignored or not opaque) and a side-by-side ref | native | diff (default
<out-diff>.side.png). Exit 0 after a comparison, 2 on bad input.
"""
import argparse
import json
import sys
from pathlib import Path

import numpy as np
from PIL import Image

OFFSETS = [(dy, dx) for dy in (-1, 0, 1) for dx in (-1, 0, 1) if (dy, dx) != (0, 0)]


def rect(text):
    try:
        x, y, w, h = (float(v) for v in text.split(","))
    except ValueError:
        raise argparse.ArgumentTypeError(f"expected x,y,w,h, got {text!r}")
    if w <= 0 or h <= 0:
        raise argparse.ArgumentTypeError(f"empty rect {text!r}")
    return x, y, w, h


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
    p.add_argument("--json", action="store_true")
    a = p.parse_args()

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
    ref, nat = ref[y0:y1, x0:x1], nat[y0:y1, x0:x1]
    h, w = ref.shape[:2]

    ignored = np.zeros((h, w), bool)
    for r in a.ignore:
        ix0, iy0, ix1, iy1 = physical(r, a.scale, (x0, y0, x1, y1))
        ignored[iy0 - y0:iy1 - y0, ix0 - x0:ix1 - x0] = True
    clear = (ref[..., 3] < 255) | (nat[..., 3] < 255)
    compared = ~ignored & ~clear
    delta = np.abs(ref[..., :3].astype(np.int16) - nat[..., :3].astype(np.int16)).max(axis=2)
    raw = (delta > a.tolerance) & compared
    many_ref, many_nat = many_siblings(ref), many_siblings(nat)
    aa = raw & (antialiased(ref, nat, many_ref, many_nat) | antialiased(nat, ref, many_nat, many_ref))
    snap = raw & ~aa & near
    real = raw & ~aa & ~snap

    n = int(compared.sum())
    stat = lambda m: {"pixels": int(m.sum()), "percent": round(100 * m.sum() / n, 4) if n else 0.0,
                      "max_delta": int(delta[m].max()) if m.any() else 0, "bbox": bbox(m, x0, y0, a.scale)}
    side = a.side or str(Path(a.out).with_suffix("")) + ".side.png"
    result = {
        "size": [W, H], "scale": a.scale, "region_px": [x0, y0, x1 - x0, y1 - y0], "compared": n,
        "ignored": int(ignored.sum()), "not_opaque": int((clear & ~ignored).sum()), "tolerance": a.tolerance,
        "max_delta": int(delta[compared].max()) if n else 0,
        "raw": stat(raw), "aa_filtered": stat(real), "antialiasing": int(aa.sum()), "snap": stat(snap),
        "diff": a.out, "side": side,
    }

    gray = ref[..., :3].astype(np.float32) @ np.array([0.299, 0.587, 0.114], np.float32)
    out = np.repeat((255 - (255 - gray) * 0.25)[..., None], 3, axis=2).astype(np.uint8)
    out[ignored | clear] = (205, 225, 255)
    out[aa] = (255, 196, 0)
    out[snap] = (0, 160, 80)
    out[real] = (230, 0, 0)
    Image.fromarray(out, "RGB").save(a.out)
    gap = np.full((h, max(2, int(8 * a.scale)), 3), 128, np.uint8)
    Image.fromarray(np.hstack([ref[..., :3], gap, nat[..., :3], gap, out]), "RGB").save(side)

    if a.json:
        print(json.dumps(result, indent=1))
        return
    lg = lambda v: f"{v / a.scale:g}"
    print(f"region       {lg(x0)},{lg(y0)},{lg(x1 - x0)},{lg(y1 - y0)} logical ({x1 - x0}x{y1 - y0} px at {a.scale:g}x); "
          f"compared {n:,} px, ignored {result['ignored']:,}, not opaque {result['not_opaque']:,}; tolerance {a.tolerance}")
    for key, label in (("raw", "raw"), ("snap", "snap"), ("aa_filtered", "aa-filtered")):
        s = result[key]
        box = f"  bbox {','.join(f'{v:g}' for v in s['bbox'])} logical" if s["bbox"] else ""
        print(f"{label:<12} {s['pixels']:,} px mismatched ({s['percent']:.4f}%), max channel delta {s['max_delta']}{box}")
    print(f"antialiasing {result['antialiasing']:,} px; max channel delta over the region {result['max_delta']}")
    print(f"diff         {a.out}\nside-by-side {side}")


if __name__ == "__main__":
    main()
