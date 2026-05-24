"""
Run once to generate kadr.ico.
Requires: pip install Pillow
"""
from PIL import Image, ImageDraw
import struct, zlib, os

def make_frame(size):
    img = Image.new("RGBA", (size, size), (0, 0, 0, 0))
    d = ImageDraw.Draw(img)

    bg    = (18, 18, 18, 255)
    frame = (99, 155, 255, 255)
    hole  = (11, 11, 11, 255)

    r = size // 8
    d.rounded_rectangle([0, 0, size-1, size-1], radius=r, fill=bg)

    b  = max(2, size // 16)
    b2 = b * 2

    # Outer frame border
    d.rounded_rectangle([b, b, size-1-b, size-1-b], radius=r, outline=frame, width=b)

    # Film strip holes — top and bottom
    hole_w = max(2, size // 8)
    hole_h = max(2, size // 12)
    gap    = size // 6
    y_top  = b2
    y_bot  = size - b2 - hole_h
    for col in range(3):
        x = b2 + gap // 2 + col * gap
        if x + hole_w < size - b2:
            d.rectangle([x, y_top, x + hole_w, y_top + hole_h], fill=hole)
            d.rectangle([x, y_bot, x + hole_w, y_bot + hole_h], fill=hole)

    # Inner bright rectangle (the "image" area)
    pad = size // 5
    d.rounded_rectangle(
        [pad, pad + size // 10, size - pad, size - pad - size // 10],
        radius=max(1, r // 2),
        fill=(99, 155, 255, 60),
        outline=(99, 155, 255, 200),
        width=max(1, b // 2),
    )

    return img

sizes = [16, 24, 32, 48, 64, 128, 256]
frames = [make_frame(s) for s in sizes]

out = os.path.join(os.path.dirname(__file__), "kadr.ico")
frames[0].save(out, format="ICO", sizes=[(s, s) for s in sizes], append_images=frames[1:])
print(f"Written {out}")
