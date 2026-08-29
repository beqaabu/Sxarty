#!/usr/bin/env python3
"""Generates every icon Sxarty ships from one geometric description.

The mark is the reading pane at icon scale: two bars for the word, a red block
for the pivot between them, and the guide ticks above and below it. The right
bar is longer than the left because the optimal recognition point sits left of
centre, so even the icon is telling the truth about the product.

Pure standard library, so it runs anywhere without ImageMagick or Pillow. Run it
only when the mark changes; the results are committed.

    python3 scripts/make_icons.py
"""

import os
import struct
import zlib

OUT = os.path.join(os.path.dirname(os.path.dirname(os.path.abspath(__file__))), "assets", "icon")

# Canvas is 1024 units square; every measurement below is in those units.
S = 1024
SS = 4  # supersampling factor for antialiasing

GROUND = (0x0B, 0x0E, 0x14)
WORD = (0xE4, 0xE8, 0xF0)
PIVOT = (0xFF, 0x5C, 0x5C)
TICK = (0xFF, 0x5C, 0x5C, 0.45)

CORNER = 232          # rounded-square radius
MID = S / 2

BAR_H = 132
BAR_R = BAR_H / 2
LEFT = (150, MID - BAR_H / 2, 452, MID + BAR_H / 2)
RIGHT = (572, MID - BAR_H / 2, 902, MID + BAR_H / 2)

PIVOT_W = 84
PIVOT_H = 236
PIVOT_BOX = (MID - PIVOT_W / 2, MID - PIVOT_H / 2, MID + PIVOT_W / 2, MID + PIVOT_H / 2)

TICK_W = 28
TICK_H = 74
TICK_GAP = 60  # distance from the pivot block
TOP_TICK = (MID - TICK_W / 2, PIVOT_BOX[1] - TICK_GAP - TICK_H, MID + TICK_W / 2, PIVOT_BOX[1] - TICK_GAP)
BOTTOM_TICK = (MID - TICK_W / 2, PIVOT_BOX[3] + TICK_GAP, MID + TICK_W / 2, PIVOT_BOX[3] + TICK_GAP + TICK_H)


def inside_round_rect(x, y, box, radius):
    x0, y0, x1, y1 = box
    if not (x0 <= x <= x1 and y0 <= y <= y1):
        return False
    if radius <= 0:
        return True

    cx = min(max(x, x0 + radius), x1 - radius)
    cy = min(max(y, y0 + radius), y1 - radius)
    dx, dy = x - cx, y - cy

    return dx * dx + dy * dy <= radius * radius


def blend(base, layer, alpha):
    return tuple(round(b + (l - b) * alpha) for b, l in zip(base, layer))


def render(size, margin=0.0):
    """Draws the mark at `size` pixels, supersampled, returning RGBA rows.

    `margin` insets the mark as a fraction of the canvas. macOS expects roughly
    a tenth of the canvas to be transparent on each side so an icon sits at the
    same visual weight as its neighbours in the Dock; every other platform
    wants the mark full bleed.
    """
    inner = S * (1.0 - 2.0 * margin)
    offset = S * margin
    scale = S / (size * SS)
    rows = []

    for py in range(size):
        row = bytearray()
        for px in range(size):
            r = g = b = 0
            a = 0.0

            for sy in range(SS):
                for sx in range(SS):
                    # Sample at the centre of each subpixel, in canvas units,
                    # then map into the inset box the mark is drawn in.
                    x = (((px * SS) + sx + 0.5) * scale - offset) * S / inner
                    y = (((py * SS) + sy + 0.5) * scale - offset) * S / inner

                    if not inside_round_rect(x, y, (0, 0, S, S), CORNER):
                        continue

                    colour = GROUND
                    if inside_round_rect(x, y, LEFT, BAR_R) or inside_round_rect(x, y, RIGHT, BAR_R):
                        colour = WORD
                    elif inside_round_rect(x, y, PIVOT_BOX, 12):
                        colour = PIVOT
                    elif inside_round_rect(x, y, TOP_TICK, 8) or inside_round_rect(x, y, BOTTOM_TICK, 8):
                        colour = blend(GROUND, TICK[:3], TICK[3])

                    r += colour[0]
                    g += colour[1]
                    b += colour[2]
                    a += 1.0

            samples = SS * SS
            if a == 0:
                row += bytes((0, 0, 0, 0))
            else:
                # Average only the covered samples, so edge pixels keep their
                # colour and vary in alpha instead of darkening toward black.
                row += bytes((round(r / a), round(g / a), round(b / a), round(255 * a / samples)))

        rows.append(bytes(row))

    return rows


def png(rows, size):
    raw = b"".join(b"\x00" + row for row in rows)

    def chunk(tag, data):
        body = tag + data
        return struct.pack(">I", len(data)) + body + struct.pack(">I", zlib.crc32(body) & 0xFFFFFFFF)

    return (
        b"\x89PNG\r\n\x1a\n"
        + chunk(b"IHDR", struct.pack(">IIBBBBB", size, size, 8, 6, 0, 0, 0))
        + chunk(b"IDAT", zlib.compress(raw, 9))
        + chunk(b"IEND", b"")
    )


def ico(images):
    """A PNG-compressed .ico, which every Windows since Vista reads."""
    count = len(images)
    header = struct.pack("<HHH", 0, 1, count)
    offset = 6 + 16 * count

    entries, blobs = b"", b""
    for size, blob in images:
        entries += struct.pack(
            "<BBBBHHII",
            0 if size >= 256 else size,
            0 if size >= 256 else size,
            0, 0, 1, 32,
            len(blob),
            offset,
        )
        blobs += blob
        offset += len(blob)

    return header + entries + blobs


def svg(margin=0.0):
    def rect(box, radius, fill, opacity=None):
        x0, y0, x1, y1 = box
        extra = f' opacity="{opacity}"' if opacity else ""
        return (
            f'  <rect x="{x0:g}" y="{y0:g}" width="{x1 - x0:g}" height="{y1 - y0:g}" '
            f'rx="{radius:g}" fill="{fill}"{extra}/>'
        )

    hexes = {name: "#%02X%02X%02X" % c for name, c in
             (("ground", GROUND), ("word", WORD), ("pivot", PIVOT))}

    inner = 1.0 - 2.0 * margin
    group = (
        f'  <g transform="translate({S * margin:g} {S * margin:g}) scale({inner:g})">'
        if margin else "  <g>"
    )

    return "\n".join([
        f'<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {S} {S}" width="{S}" height="{S}">',
        '  <title>Sxarty</title>',
        group,
        rect((0, 0, S, S), CORNER, hexes["ground"]),
        rect(TOP_TICK, 8, "#%02X%02X%02X" % PIVOT, 0.45),
        rect(LEFT, BAR_R, hexes["word"]),
        rect(PIVOT_BOX, 12, hexes["pivot"]),
        rect(RIGHT, BAR_R, hexes["word"]),
        rect(BOTTOM_TICK, 8, "#%02X%02X%02X" % PIVOT, 0.45),
        '  </g>',
        '</svg>',
        '',
    ])


# Apple's own icons leave about a tenth of the canvas clear on each side.
MACOS_MARGIN = 0.10


def write(path, data):
    mode = "w" if isinstance(data, str) else "wb"
    with open(os.path.join(OUT, path), mode) as f:
        f.write(data)
    print(f"assets/icon/{path}")


def main():
    os.makedirs(os.path.join(OUT, "macos"), exist_ok=True)

    write("sxarty.svg", svg())

    # Linux hicolor sizes, plus what .ico needs.
    blobs = {}
    for size in (16, 24, 32, 48, 64, 128, 256, 512, 1024):
        blobs[size] = png(render(size), size)
        write(f"sxarty-{size}.png", blobs[size])

    write("sxarty.ico", ico([(s, blobs[s]) for s in (16, 32, 48, 64, 128, 256)]))

    # iconset layout that `iconutil` compiles into sxarty.icns.
    for size in (16, 32, 128, 256, 512):
        write(f"macos/icon_{size}x{size}.png", png(render(size, MACOS_MARGIN), size))
        write(
            f"macos/icon_{size}x{size}@2x.png",
            png(render(size * 2, MACOS_MARGIN), size * 2),
        )


if __name__ == "__main__":
    main()
