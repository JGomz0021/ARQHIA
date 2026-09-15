#!/usr/bin/env python3
"""Genera los assets del icono de ARQHIA (v0.9.6).

Fuente vectorial: `assets/icon.svg` (versionada a mano).
Este script rasteriza la misma geometría (rounded-square + "A" en cápsulas)
sin dependencias externas y produce:

  * assets/icon-256.png, icon-128.png, icon-64.png, icon-32.png, icon-16.png
  * assets/icon.rgba  (256x256 RGBA crudo, embebido en el binario vía
                       `include_bytes!` para `iced::window::icon::from_rgba`)

Uso:  python3 packaging/make_icons.py
"""

import os
import struct
import zlib

SS = 3                      # supersampling para antialias
SIZE = 256
BIG = SIZE * SS

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(HERE)
ASSETS = os.path.join(ROOT, "assets")

# Paleta (coincide con icon.svg)
BG_TOP = (0x1B, 0x21, 0x40)
BG_BOT = (0x0B, 0x0E, 0x1C)
A_TOP = (0x67, 0xE8, 0xF9)
A_BOT = (0xA7, 0x8B, 0xFA)

# Geometría del "A" en coordenadas 256 (escala luego a BIG)
APEX = (128.0, 50.0)
FOOT_L = (66.0, 206.0)
FOOT_R = (190.0, 206.0)
BAR_L = (86.0, 156.0)
BAR_R = (170.0, 156.0)
LEG_W = 34.0
BAR_W = 26.0


def lerp(a, b, t):
    return tuple(round(x + (y - x) * t) for x, y in zip(a, b))


def rounded_rect_alpha(x, y, rx, radius):
    """Cobertura 1/0 de un rounded-rect (usamos supersampling, no AA analítico)."""
    if x < 0 or y < 0 or x > rx * 2 or y > rx * 2:
        return False
    r = radius
    cx = min(max(x, r), rx * 2 - r)
    cy = min(max(y, r), rx * 2 - r)
    dx = x - cx
    dy = y - cy
    return dx * dx + dy * dy <= r * r or (r <= x <= rx * 2 - r) or (r <= y <= rx * 2 - r)


def dist_seg(px, py, ax, ay, bx, by):
    vx, vy = bx - ax, by - ay
    wx, wy = px - ax, py - ay
    c1 = vx * wx + vy * wy
    if c1 <= 0:
        return ((px - ax) ** 2 + (py - ay) ** 2) ** 0.5
    c2 = vx * vx + vy * vy
    if c2 <= c1:
        return ((px - bx) ** 2 + (py - by) ** 2) ** 0.5
    t = c1 / c2
    return ((px - (ax + t * vx)) ** 2 + (py - (ay + t * vy)) ** 2) ** 0.5


def render_big():
    """Devuelve una lista BIGxBIG de (r,g,b,a) a resolución supersampleada."""
    scale = SS
    radius = 56.0 * scale
    bg_top = BG_TOP
    bg_bot = BG_BOT

    legs = [
        (APEX[0] * scale, APEX[1] * scale, FOOT_L[0] * scale, FOOT_L[1] * scale, LEG_W * scale),
        (APEX[0] * scale, APEX[1] * scale, FOOT_R[0] * scale, FOOT_R[1] * scale, LEG_W * scale),
        (BAR_L[0] * scale, BAR_L[1] * scale, BAR_R[0] * scale, BAR_R[1] * scale, BAR_W * scale),
    ]

    px = bytearray(BIG * BIG * 4)
    for yy in range(BIG):
        t = yy / (BIG - 1)
        bg = lerp(bg_top, bg_bot, t)
        a_col = lerp(A_TOP, A_BOT, t)
        row = yy * BIG * 4
        for xx in range(BIG):
            i = row + xx * 4
            if not rounded_rect_alpha(xx + 0.5, yy + 0.5, BIG / 2, radius):
                continue
            # fondo
            r, g, b = bg
            # ¿dentro de alguna cápsula del "A"?
            inside = False
            for ax, ay, bx, by, w in legs:
                if dist_seg(xx + 0.5, yy + 0.5, ax, ay, bx, by) <= w / 2:
                    inside = True
                    break
            if inside:
                r, g, b = a_col
            px[i] = r
            px[i + 1] = g
            px[i + 2] = b
            px[i + 3] = 255
    return px


def downsample(px, factor):
    """Box filter sobre la imagen supersampleada -> SIZE x SIZE RGBA."""
    out = bytearray(SIZE * SIZE * 4)
    n = factor * factor
    for y in range(SIZE):
        for x in range(SIZE):
            r = g = b = a = 0
            for dy in range(factor):
                sy = y * factor + dy
                base = sy * BIG * 4
                for dx in range(factor):
                    sx = x * factor + dx
                    i = base + sx * 4
                    r += px[i]
                    g += px[i + 1]
                    b += px[i + 2]
                    a += px[i + 3]
            o = (y * SIZE + x) * 4
            out[o] = r // n
            out[o + 1] = g // n
            out[o + 2] = b // n
            out[o + 3] = a // n
    return out


def box_scale(px, size, target):
    """Escala una imagen size x size RGBA a target x target (box filter)."""
    out = bytearray(target * target * 4)
    ratio = size / target
    for y in range(target):
        y0 = int(y * ratio)
        y1 = max(y0 + 1, int((y + 1) * ratio))
        for x in range(target):
            x0 = int(x * ratio)
            x1 = max(x0 + 1, int((x + 1) * ratio))
            r = g = b = a = 0
            cnt = 0
            for yy in range(y0, y1):
                base = yy * size * 4
                for xx in range(x0, x1):
                    i = base + xx * 4
                    r += px[i]
                    g += px[i + 1]
                    b += px[i + 2]
                    a += px[i + 3]
                    cnt += 1
            o = (y * target + x) * 4
            out[o] = r // cnt
            out[o + 1] = g // cnt
            out[o + 2] = b // cnt
            out[o + 3] = a // cnt
    return out


def write_png(path, px, size):
    raw = bytearray()
    stride = size * 4
    for y in range(size):
        raw.append(0)  # filtro None
        raw += px[y * stride:(y + 1) * stride]

    def chunk(tag, data):
        c = struct.pack(">I", len(data)) + tag + data
        return c + struct.pack(">I", zlib.crc32(tag + data) & 0xFFFFFFFF)

    ihdr = struct.pack(">IIBBBBB", size, size, 8, 6, 0, 0, 0)
    png = b"\x89PNG\r\n\x1a\n"
    png += chunk(b"IHDR", ihdr)
    png += chunk(b"IDAT", zlib.compress(bytes(raw), 9))
    png += chunk(b"IEND", b"")
    with open(path, "wb") as f:
        f.write(png)


def main():
    big = render_big()
    px256 = downsample(big, SS)
    os.makedirs(ASSETS, exist_ok=True)
    write_png(os.path.join(ASSETS, "icon-256.png"), px256, SIZE)
    for target in (128, 64, 32, 16):
        small = box_scale(px256, SIZE, target)
        write_png(os.path.join(ASSETS, f"icon-{target}.png"), small, target)
    with open(os.path.join(ASSETS, "icon.rgba"), "wb") as f:
        f.write(px256)
    print("assets generados en", ASSETS)


if __name__ == "__main__":
    main()
