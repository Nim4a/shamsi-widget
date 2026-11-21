import struct
from PIL import Image, ImageDraw

def make(sz):
    s = sz / 32.0
    im = Image.new("RGBA", (sz, sz), (0, 0, 0, 0))
    d = ImageDraw.Draw(im)
    c = (10, 10, 10, 255)
    w = max(1, round(2.4 * s))
    d.rounded_rectangle([4 * s, 6 * s, 28 * s, 28 * s], radius=3 * s, outline=c, width=w)
    d.line([4 * s, 12 * s, 28 * s, 12 * s], fill=c, width=w)
    d.line([10 * s, 3 * s, 10 * s, 8 * s], fill=c, width=w)
    d.line([22 * s, 3 * s, 22 * s, 8 * s], fill=c, width=w)
    for ry in (17, 22):
        for rx in (10, 16, 22):
            d.rectangle([rx * s - 1.4 * s, ry * s - 1.4 * s, rx * s + 1.4 * s, ry * s + 1.4 * s], fill=c)
    return im

imgs = [make(sz) for sz in (16, 32, 48, 64)]
payload = b"".join(im.tobytes() for im in imgs)
hdr = struct.pack("<HHH", 0, 1, len(imgs))
entries = b""
off = 6 + 16 * len(imgs)
for im in imgs:
    b = im.tobytes()
    w, h = im.size
    entries += struct.pack("<BBBBHHII", w % 256, h % 256, 0, 0, 1, 32, len(b), off)
    off += len(b)
for path in (r"Z:\hermes\apps\shamsi-widget\widget.ico", r"Z:\hermes\projects\shamsi-widget\widget.ico"):
    import os
    os.makedirs(os.path.dirname(path), exist_ok=True)
    open(path, "wb").write(hdr + entries + payload)
print("ico ok")
