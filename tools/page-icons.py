#!/usr/bin/env python3
"""The page's icons for a phone's home screen (docs/icon-192.png, icon-512.png and
apple-touch-icon.png): tools/musicbox.svg's bars and peaks, drawn square to the edges (a phone
cuts its own corners). Standard library only.

    tools/page-icons.py [DOCS]
"""
import os
import struct
import sys
import zlib

BG, BAR, PEAK = (0x0E, 0x11, 0x16), (0x5F, 0xD3, 0xC6), (0xF2, 0xB5, 0x44)
# (x, y, w, h) on the svg's 64 x 64
BARS = [(10, 34, 6, 20), (19, 24, 6, 30), (28, 16, 6, 38), (37, 28, 6, 26), (46, 38, 6, 16)]
PEAKS = [(10, 28, 6, 3), (19, 18, 6, 3), (28, 10, 6, 3), (37, 22, 6, 3), (46, 32, 6, 3)]


def png(size):
    px = [[BG] * size for _ in range(size)]
    for rects, colour in ((BARS, BAR), (PEAKS, PEAK)):
        for x, y, w, h in rects:
            for yy in range(round(y * size / 64), round((y + h) * size / 64)):
                for xx in range(round(x * size / 64), round((x + w) * size / 64)):
                    px[yy][xx] = colour
    raw = b"".join(b"\0" + bytes(c for p in row for c in p) for row in px)

    def chunk(kind, data):
        return struct.pack(">I", len(data)) + kind + data + struct.pack(">I", zlib.crc32(kind + data))

    head = struct.pack(">IIBBBBB", size, size, 8, 2, 0, 0, 0)
    return b"\x89PNG\r\n\x1a\n" + chunk(b"IHDR", head) + chunk(b"IDAT", zlib.compress(raw, 9)) + chunk(b"IEND", b"")


docs = sys.argv[1] if len(sys.argv) > 1 else os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "docs")
for name, size in (("icon-192.png", 192), ("icon-512.png", 512), ("apple-touch-icon.png", 180)):
    with open(os.path.join(docs, name), "wb") as f:
        f.write(png(size))
    print(name, size)
