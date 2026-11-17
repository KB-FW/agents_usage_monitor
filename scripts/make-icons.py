#!/usr/bin/env python3
"""Draw the AgentMeter icon set into src-tauri/icons (needs Pillow).

A #2B2B2B rounded tile with a 270° gauge: grey track, #F05C1A fill and a light needle.
Writes the PNG sizes Tauri bundles, the tray icons, icon.ico and icon.icns.
"""
import math
from pathlib import Path

from PIL import Image, ImageDraw

BASE = (0x2B, 0x2B, 0x2B, 255)
EDGE = (0x3C, 0x3C, 0x3C, 255)
TRACK = (0x52, 0x52, 0x52, 255)
ACCENT = (0xF0, 0x5C, 0x1A, 255)
NEEDLE = (0xF3, 0xF3, 0xF3, 255)
START, SWEEP, LEVEL = 135.0, 270.0, 0.68

icons = Path(__file__).resolve().parent.parent / "src-tauri" / "icons"


def polar(c, r, deg):
    a = math.radians(deg)
    return (c + r * math.cos(a), c + r * math.sin(a))


def dot(d, xy, r, fill):
    x, y = xy
    d.ellipse((x - r, y - r, x + r, y + r), fill=fill)


def arc(d, c, r, w, a0, a1, fill):
    d.arc((c - r, c - r, c + r, c + r), a0, a1, fill=fill, width=int(w))
    # round caps
    for a in (a0, a1):
        dot(d, polar(c, r - w / 2, a), w / 2, fill)


def draw(size, needle=True):
    s = 1024
    img = Image.new("RGBA", (s, s), (0, 0, 0, 0))
    d = ImageDraw.Draw(img)
    pad = s * 0.04
    rad = s * 0.22
    d.rounded_rectangle((pad, pad, s - pad, s - pad), rad, fill=EDGE)
    e = s * 0.012
    d.rounded_rectangle((pad + e, pad + e, s - pad - e, s - pad - e), rad - e, fill=BASE)

    c = s / 2
    r = s * 0.33
    w = s * (0.095 if needle else 0.13)
    arc(d, c, r, w, START, START + SWEEP, TRACK)
    end = START + SWEEP * LEVEL
    arc(d, c, r, w, START, end, ACCENT)
    if needle:
        tip = polar(c, r * 0.62, end)
        d.line((c, c, *tip), fill=NEEDLE, width=int(s * 0.05))
        dot(d, tip, s * 0.025, NEEDLE)
        dot(d, (c, c), s * 0.065, NEEDLE)
    else:
        dot(d, (c, c), s * 0.09, ACCENT)
    return img.resize((size, size), Image.LANCZOS)


def main():
    for name, size in [("32x32.png", 32), ("64x64.png", 64), ("128x128.png", 128), ("128x128@2x.png", 256), ("icon.png", 512)]:
        draw(size).save(icons / name)
    # tray icons: no needle, thicker ring so it reads at 16-32 px
    draw(32, needle=False).save(icons / "tray.png")
    draw(64, needle=False).save(icons / "tray@2x.png")
    big = draw(1024)
    big.save(icons / "icon.ico", sizes=[(16, 16), (24, 24), (32, 32), (48, 48), (64, 64), (128, 128), (256, 256)])
    big.save(icons / "icon.icns")
    print("wrote icons to", icons)


if __name__ == "__main__":
    main()
