"""Draws the Planora icon: a glossy dark-navy rounded square with a glowing outline,
a calendar page and a green check mark, in the NIMVORO suite style.

Run (Python 3 with Pillow and numpy):
    python tools/make_icon.py [preview.png]

Writes assets/icon.png (512x512 RGBA) and assets/icon.ico (16, 24, 32, 48, 64, 128, 256).
With an argument, also writes a contact sheet (256, 32 and 16 px on light and dark pages).
"""
import os
import sys

import numpy as np
from PIL import Image, ImageChops, ImageDraw, ImageFilter

S = 4  # supersampling
N = 512 * S
HERE = os.path.dirname(os.path.abspath(__file__))
ASSETS = os.path.join(HERE, "..", "assets")


def px(v):
    return int(round(v * S))


def hexc(h, a=255):
    return ((h >> 16) & 255, (h >> 8) & 255, h & 255, a)


def vgrad(w, h, top, bottom):
    t = np.linspace(0, 1, h)[:, None, None]
    a = np.array(top, float)[None, None, :]
    b = np.array(bottom, float)[None, None, :]
    arr = np.repeat(a + (b - a) * t, w, axis=1)
    return Image.fromarray(arr.astype(np.uint8), "RGBA")


def hgrad(w, h, left, right):
    t = np.linspace(0, 1, w)[None, :, None]
    a = np.array(left, float)[None, None, :]
    b = np.array(right, float)[None, None, :]
    arr = np.repeat(a + (b - a) * t, h, axis=0)
    return Image.fromarray(arr.astype(np.uint8), "RGBA")


def rr_mask(box, r, size=(N, N)):
    m = Image.new("L", size, 0)
    ImageDraw.Draw(m).rounded_rectangle([px(box[0]), px(box[1]), px(box[2]), px(box[3])], px(r), fill=255)
    return m


def paste_masked(base, layer, mask):
    base.alpha_composite(Image.composite(layer, Image.new("RGBA", layer.size, (0, 0, 0, 0)), mask))


def shadow(base, mask, dy, blur, alpha):
    sh = Image.new("RGBA", base.size, (0, 0, 0, 0))
    sh.putalpha(mask.point(lambda v: int(v * alpha)))
    sh = ImageChops.offset(sh, 0, px(dy)).filter(ImageFilter.GaussianBlur(px(blur)))
    base.alpha_composite(sh)


def draw():
    img = Image.new("RGBA", (N, N), (0, 0, 0, 0))
    tile = rr_mask((0, 0, 512, 512), 118)

    # Navy body with a soft blue glow behind the calendar.
    body = vgrad(N, N, hexc(0x1d2d66), hexc(0x0b1030))
    glow = Image.new("RGBA", (N, N), (0, 0, 0, 0))
    ImageDraw.Draw(glow).ellipse([px(70), px(90), px(442), px(462)], fill=hexc(0x4f6bff, 120))
    body.alpha_composite(glow.filter(ImageFilter.GaussianBlur(px(70))))
    paste_masked(img, body, tile)

    # Gloss: a pale ellipse across the top half, clipped to the tile.
    gloss = Image.new("RGBA", (N, N), (0, 0, 0, 0))
    ImageDraw.Draw(gloss).ellipse([px(-120), px(-330), px(632), px(250)], fill=(255, 255, 255, 30))
    gloss = gloss.filter(ImageFilter.GaussianBlur(px(10)))
    paste_masked(img, gloss, tile)

    # Glowing outline: blurred halo plus a crisp gradient stroke and a thin inner light line.
    ring = Image.new("L", (N, N), 0)
    ImageDraw.Draw(ring).rounded_rectangle([px(5), px(5), px(507), px(507)], px(114), outline=255, width=px(9))
    halo = hgrad(N, N, hexc(0x5b8cff, 255), hexc(0x9b6bff, 255))
    halo.putalpha(ring.filter(ImageFilter.GaussianBlur(px(7))).point(lambda v: int(min(255, v * 1.5))))
    img.alpha_composite(halo)
    stroke = hgrad(N, N, hexc(0x6ea0ff), hexc(0x9a72ff))
    paste_masked(img, stroke, ring)
    inner = Image.new("L", (N, N), 0)
    ImageDraw.Draw(inner).rounded_rectangle([px(16), px(16), px(496), px(496)], px(104), outline=255, width=px(2))
    il = Image.new("RGBA", (N, N), (170, 190, 255, 0))
    il.putalpha(inner.point(lambda v: int(v * 0.35)))
    img.alpha_composite(il)

    # Calendar page.
    card_box = (100, 120, 412, 414)
    card_mask = rr_mask(card_box, 46)
    shadow(img, card_mask, 12, 14, 0.55)
    card = vgrad(N, N, hexc(0xf4f6ff), hexc(0xcdd5f6))
    paste_masked(img, card, card_mask)

    # Header band.
    band_mask = ImageChops.multiply(card_mask, Image.new("L", (N, N), 0))
    band = Image.new("L", (N, N), 0)
    ImageDraw.Draw(band).rectangle([px(100), px(120), px(412), px(214)], fill=255)
    band_mask = ImageChops.multiply(card_mask, band)
    head = hgrad(N, N, hexc(0x4f6bff), hexc(0x8a5bff))
    paste_masked(img, head, band_mask)
    hl = Image.new("RGBA", (N, N), (0, 0, 0, 0))
    ImageDraw.Draw(hl).rectangle([px(100), px(120), px(412), px(160)], fill=(255, 255, 255, 38))
    paste_masked(img, hl, band_mask)

    # Binder rings.
    d = ImageDraw.Draw(img)
    for cx in (176, 336):
        m = rr_mask((cx - 15, 88, cx + 15, 158), 15)
        shadow(img, m, 4, 4, 0.5)
        paste_masked(img, vgrad(N, N, hexc(0xe8ecff), hexc(0x9aa6dc)), m)
        inner_m = rr_mask((cx - 8, 98, cx + 8, 146), 8)
        paste_masked(img, vgrad(N, N, hexc(0x1b2557), hexc(0x293a82)), inner_m)

    # Faint day cells.
    d = ImageDraw.Draw(img)
    cell = hexc(0xb9c3ec)
    for row in range(2):
        for col in range(4):
            x0 = 128 + col * 62
            y0 = 236 + row * 64
            if row == 1 and col >= 2:
                continue
            d.rounded_rectangle([px(x0), px(y0), px(x0 + 44), px(y0 + 40)], px(10), fill=cell)

    # Green check mark, big and bold so it reads at 16 px.
    pts = [(214, 330), (262, 378), (372, 258)]
    chk = Image.new("L", (N, N), 0)
    cd = ImageDraw.Draw(chk)
    w = px(44)
    cd.line([(px(x), px(y)) for x, y in pts], fill=255, width=w, joint="curve")
    for x, y in (pts[0], pts[-1]):
        cd.ellipse([px(x) - w // 2, px(y) - w // 2, px(x) + w // 2, px(y) + w // 2], fill=255)
    shadow(img, chk, 10, 9, 0.6)
    gl = Image.new("RGBA", (N, N), (0, 0, 0, 0))
    gl.putalpha(chk.filter(ImageFilter.GaussianBlur(px(14))).point(lambda v: int(v * 0.75)))
    gl_color = Image.new("RGBA", (N, N), hexc(0x2de08a, 255))
    gl_color.putalpha(gl.getchannel("A"))
    img.alpha_composite(gl_color)
    paste_masked(img, vgrad(N, N, hexc(0x5df2a4), hexc(0x14b866)), chk)

    return img.resize((512, 512), Image.LANCZOS), img


def main():
    icon, big = draw()
    os.makedirs(ASSETS, exist_ok=True)
    icon.save(os.path.join(ASSETS, "icon.png"))
    sizes = [16, 24, 32, 48, 64, 128, 256]
    frames = [big.resize((s, s), Image.LANCZOS) for s in sizes]
    frames[-1].save(os.path.join(ASSETS, "icon.ico"), format="ICO", sizes=[(s, s) for s in sizes], append_images=frames[:-1])
    print("wrote assets/icon.png and assets/icon.ico")
    if len(sys.argv) > 1:
        sheet = Image.new("RGBA", (2 * 400, 300), (0, 0, 0, 255))
        for i, bg in enumerate([(244, 245, 248, 255), (20, 22, 27, 255)]):
            sheet.paste(Image.new("RGBA", (400, 300), bg), (i * 400, 0))
            x = i * 400 + 20
            for s in (256, 32, 16):
                im = big.resize((s, s), Image.LANCZOS)
                sheet.alpha_composite(im, (x, 20 if s == 256 else 20))
                x += s + 16
        sheet.save(sys.argv[1])
        print("wrote", sys.argv[1])


if __name__ == "__main__":
    main()
