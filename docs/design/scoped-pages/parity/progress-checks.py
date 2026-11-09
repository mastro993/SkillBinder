"""Progress-strip parity checks that do not depend on the content under the wash.

usage (through parity/py):
  progress-checks.py wash <base.png> <strip.png> <light|dark> [--alpha 0.7] [--color #rrggbb] [--rect x,y,w,h]
      [--ignore x,y,w,h ...]
      Every pixel of <strip> inside --rect (default: whole window) must equal
      round(alpha*color + (1-alpha)*base) within 1 level, outside the --ignore rects.
      color defaults to the page background (the wash); a Kit scrim is --color #000000
      --alpha 0.05 (light) or 0.2 (dark).
      base = the same window without the strip (native: a capture taken just before the click).
  halo <ref_base.png> <ref_strip.png> <nat_base.png> <nat_strip.png> <light|dark> --rect x,y,w,h
      [--ignore x,y,w,h ...] [--alpha 0.7]
      Shadow darkening s = 1 - strip/washed (washed = the pure wash over base) must match between
      the reference and the native capture within 3/255 (light only; dark is below 1 level).
      GPUI's 4-sample blur leaves flat page pixels 3/255 off (spec/progress-5-gpui.md G1).
      Pixels where the two bases differ by more than 1 level, and their 8 neighbours, are page
      content (the mock's page against the app's, edges up to a device px apart): left out.
Rects are logical px of the 1180x728 window; images are 2x. Exit 1 on failure.
"""
import argparse
import sys

import numpy as np
from PIL import Image

BG = {"light": (255, 255, 255), "dark": (10, 10, 10)}
HALO = 3  # /255: GPUI's shadow blur (G1)


def rect(text):
    return tuple(float(v) for v in text.split(","))


def load(path):
    return np.asarray(Image.open(path).convert("RGB")).astype(float)


def mask_for(shape, area, ignores):
    m = np.zeros(shape[:2], bool)
    x, y, w, h = area or (0, 0, shape[1] / 2, shape[0] / 2)
    m[int(y * 2):int(np.ceil((y + h) * 2)), int(x * 2):int(np.ceil((x + w) * 2))] = True
    for x, y, w, h in ignores:
        m[int(y * 2):int(np.ceil((y + h) * 2)), int(x * 2):int(np.ceil((x + w) * 2))] = False
    return m


def washed(base, theme, alpha, color=None):
    c = [int(color[i:i + 2], 16) for i in (1, 3, 5)] if color else BG[theme]
    return alpha * np.array(c, float) + (1 - alpha) * base


def main():
    p = argparse.ArgumentParser()
    p.add_argument("mode", choices=["wash", "halo"])
    p.add_argument("images", nargs="+")
    p.add_argument("--alpha", type=float, default=0.7)
    p.add_argument("--color")
    p.add_argument("--rect", type=rect)
    p.add_argument("--ignore", type=rect, action="append", default=[])
    a = p.parse_args()
    theme = a.images[-1]
    if a.mode == "wash":
        base, strip = (load(f) for f in a.images[:2])
        d = np.abs(strip - np.round(washed(base, theme, a.alpha, a.color))).max(axis=2)
        v = d[mask_for(d.shape, a.rect, a.ignore)]
        bad = int((v > 1).sum())
        print(f"wash: {v.size} px, max {v.max():.1f}, off by >1: {bad}")
        return 1 if bad else 0
    rb, rs, nb, ns = (load(f) for f in a.images[:4])
    page = np.pad(np.abs(rb - nb).max(axis=2) > 1, 1)
    page = np.any([page[1 + dy:page.shape[0] - 1 + dy, 1 + dx:page.shape[1] - 1 + dx]
                   for dy in (-1, 0, 1) for dx in (-1, 0, 1)], axis=0)
    area = mask_for(rb.shape, a.rect, a.ignore)
    m = area & ~page
    s_ref = 1 - rs / np.maximum(washed(rb, theme, a.alpha), 1)
    s_nat = 1 - ns / np.maximum(washed(nb, theme, a.alpha), 1)
    d = np.round((np.abs(s_ref - s_nat) * 255).max(axis=2), 3)[m]  # 3 levels must not read as 3.0000001
    bad = int((d > HALO).sum())
    print(f"halo: {d.size} px, max {d.max():.1f}/255, off by >{HALO}: {bad}; page content left out: {int((area & page).sum())}")
    return 1 if bad else 0


if __name__ == "__main__":
    sys.exit(main())
