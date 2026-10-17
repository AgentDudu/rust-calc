"""Regenerate assets/icon.ico — the exe's file icon.

Same design as `Ui::icon()` in src/ui/theme.rs: a rounded accent tile, a dark
display band near the top, and an equals sign below it. Kept in sync by hand;
tests/parity.rs checks the two agree on colours.

    python tools/make_icon.py

No dependencies: the ICO container, the BMP payload and the one 256 px PNG entry
are all written here.
"""

import struct
import sys
import zlib
from pathlib import Path

ACCENT = (0x00, 0x5F, 0xB8)  # Win 11 light accent — theme::ACCENT
DISPLAY = (0x1B, 0x1B, 0x1B)  # theme::LIGHT surface_container_highest
BARS = (0xCF, 0xE4, 0xFA)  # theme::ACCENT_ON_CONTRAST
SIZES = [16, 24, 32, 48, 256]

SS = 4  # supersample factor; edges are box-filtered down, no image lib needed


def inside_rounded(x, y, x0, y0, x1, y1, r):
    """Point test for a rounded rectangle, in supersampled pixel units."""
    if x < x0 or x > x1 or y < y0 or y > y1:
        return False
    cx = min(max(x, x0 + r), x1 - r)
    cy = min(max(y, y0 + r), y1 - r)
    return (x - cx) ** 2 + (y - cy) ** 2 <= r * r


def render(size):
    """The glyph at `size`, as a flat list of (r, g, b, a) with alpha from coverage."""
    n = size * SS
    tile_r = n * 0.16
    pad = n * 0.04
    # Same geometry as Ui::icon(): band across the top, two bars below it.
    band = (n * 0.16, n * 0.14, n * 0.84, n * 0.30)
    bar1 = (n * 0.30, n * 0.50, n * 0.70, n * 0.575)
    bar2 = (n * 0.30, n * 0.665, n * 0.70, n * 0.74)

    pixels = []
    for py in range(size):
        for px in range(size):
            hits = 0
            for sy in range(SS):
                for sx in range(SS):
                    x = px * SS + sx + 0.5
                    y = py * SS + sy + 0.5
                    if inside_rounded(x, y, pad, pad, n - pad, n - pad, tile_r):
                        hits += 1
            if not hits:
                pixels.append((0, 0, 0, 0))
                continue
            alpha = int(255 * hits / (SS * SS) + 0.5)

            # Overlays are opaque, so a centre sample is enough.
            cx, cy = (px + 0.5) * SS, (py + 0.5) * SS
            colour = ACCENT
            for box, over in ((band, DISPLAY), (bar1, BARS), (bar2, BARS)):
                if box[0] <= cx <= box[2] and box[1] <= cy <= box[3]:
                    colour = over
            pixels.append((*colour, alpha))
    return pixels


def bmp_entry(size):
    """ICO image entry as an uncompressed 32 bpp BMP: XOR pixels bottom-up + AND mask."""
    px = render(size)
    rows = bytearray()
    for y in range(size - 1, -1, -1):
        for x in range(size):
            r, g, b, a = px[y * size + x]
            rows += bytes((b, g, r, a))
    mask_row = ((size + 31) // 32) * 4
    mask = bytes(mask_row * size)  # all zero: alpha carries the shape
    header = struct.pack("<IiiHHIIiiII", 40, size, size * 2, 1, 32, 0, len(rows), 0, 0, 0, 0)
    return header + bytes(rows) + mask


def png_entry(size):
    """The 256 px entry as PNG — what Explorer uses for the large icon views."""
    px = render(size)
    raw = b"".join(
        b"\x00" + b"".join(bytes(px[y * size + x]) for x in range(size)) for y in range(size)
    )  # filter 0 per row, RGBA

    def chunk(kind, data):
        body = kind + data
        return struct.pack("<I", len(data)) + body + struct.pack("<I", zlib.crc32(body) & 0xFFFFFFFF)

    return (
        b"\x89PNG\r\n\x1a\n"
        + chunk(b"IHDR", struct.pack(">IIBBBBB", size, size, 8, 6, 0, 0, 0))
        + chunk(b"IDAT", zlib.compress(raw, 9))
        + chunk(b"IEND", b"")
    )


def main():
    out = Path(sys.argv[1] if len(sys.argv) > 1 else "assets/icon.ico")
    images = [png_entry(256)] + [bmp_entry(s) for s in SIZES if s != 256]
    order = [256] + [s for s in SIZES if s != 256]

    header = struct.pack("<HHH", 0, 1, len(images))
    offset = 6 + 16 * len(images)
    entries = b""
    for size, image in zip(order, images):
        w, h = (0 if size >= 256 else size), (0 if size >= 256 else size)
        entries += struct.pack("<BBBBHHII", w, h, 0, 0, 1, 32, len(image), offset)
        offset += len(image)

    out.parent.mkdir(parents=True, exist_ok=True)
    out.write_bytes(header + entries + b"".join(images))
    print(f"{out}: {out.stat().st_size} bytes, sizes {order}")


if __name__ == "__main__":
    main()
