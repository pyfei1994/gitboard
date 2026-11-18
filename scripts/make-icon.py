#!/usr/bin/env python3
"""生成 Git 看板 的 macOS 图标（squircle + 渐变 + git 分支字形）。

用法： python3 scripts/make-icon.py
产物： src-tauri/icons/icon.png（1024）、src-tauri/icons/AppIcon.icns
"""

import os
import subprocess
import sys

from PIL import Image, ImageDraw, ImageFilter

S = 1024          # 最终尺寸
SS = 4            # 超采样倍数
C = S * SS

CONTENT = int(824 / 1024 * C)   # macOS 图标内容区
MARGIN = (C - CONTENT) // 2
RADIUS = int(185 / 1024 * C)

TOP = (78, 124, 255)      # #4E7CFF
BOTTOM = (139, 92, 246)   # #8B5CF6


def lerp(a, b, t):
    return tuple(round(a[i] + (b[i] - a[i]) * t) for i in range(3))


def gradient(size):
    img = Image.new("RGB", (size, size))
    px = img.load()
    for y in range(size):
        for x in range(size):
            t = (x + y) / (2 * size - 2)
            px[x, y] = lerp(TOP, BOTTOM, t)
    return img


def squircle_mask(size, radius):
    mask = Image.new("L", (size, size), 0)
    ImageDraw.Draw(mask).rounded_rectangle([0, 0, size - 1, size - 1], radius=radius, fill=255)
    return mask


def build_icon():
    canvas = Image.new("RGBA", (C, C), (0, 0, 0, 0))

    # 背景：渐变 + squircle 遮罩
    bg = gradient(CONTENT).convert("RGBA")
    mask = squircle_mask(CONTENT, RADIUS)
    bg.putalpha(mask)
    canvas.alpha_composite(bg, (MARGIN, MARGIN))

    # 顶部柔光
    glow = Image.new("RGBA", (C, C), (0, 0, 0, 0))
    gd = ImageDraw.Draw(glow)
    gd.ellipse(
        [MARGIN + CONTENT * 0.05, MARGIN - CONTENT * 0.35,
         MARGIN + CONTENT * 0.95, MARGIN + CONTENT * 0.55],
        fill=(255, 255, 255, 46),
    )
    glow = glow.filter(ImageFilter.GaussianBlur(C * 0.06))
    glow.putalpha(Image.composite(glow.getchannel("A"), Image.new("L", (C, C), 0), mask.resize((C, C))))
    canvas.alpha_composite(glow)

    # 底部暗角，让字形更立
    shade = Image.new("RGBA", (C, C), (0, 0, 0, 0))
    sd = ImageDraw.Draw(shade)
    sd.ellipse(
        [MARGIN - CONTENT * 0.2, MARGIN + CONTENT * 0.45,
         MARGIN + CONTENT * 1.2, MARGIN + CONTENT * 1.5],
        fill=(20, 10, 60, 70),
    )
    shade = shade.filter(ImageFilter.GaussianBlur(C * 0.09))
    canvas.alpha_composite(shade)

    # 字形：git branch
    glyph = Image.new("RGBA", (C, C), (0, 0, 0, 0))
    g = ImageDraw.Draw(glyph)

    def pt(x, y):
        return (MARGIN + x / 824 * CONTENT, MARGIN + y / 824 * CONTENT)

    lw = int(30 / 824 * CONTENT)
    r = int(56 / 824 * CONTENT)

    ax, ay = pt(300, 300)
    bx, by = pt(300, 700)
    cx, cy = pt(600, 470)

    # 竖线 + 分叉
    g.line([(ax, ay), (bx, by)], fill=(255, 255, 255, 255), width=lw)
    g.line([(ax, ay + (by - ay) * 0.42), (cx, cy)], fill=(255, 255, 255, 255), width=lw)

    for (x, y) in [(ax, ay), (bx, by), (cx, cy)]:
        g.ellipse([x - r, y - r, x + r, y + r], fill=(255, 255, 255, 255))

    # 节点内挖空（透明），让渐变透出来
    hole = int(r * 0.46)
    for (x, y) in [(ax, ay), (bx, by), (cx, cy)]:
        g.ellipse([x - hole, y - hole, x + hole, y + hole], fill=(0, 0, 0, 0))

    # 轻微投影
    shadow = glyph.filter(ImageFilter.GaussianBlur(C * 0.012))
    shadow_alpha = shadow.getchannel("A").point(lambda v: int(v * 0.35))
    shadow.putalpha(shadow_alpha)
    canvas.alpha_composite(shadow, (0, int(C * 0.006)))
    canvas.alpha_composite(glyph)

    return canvas.resize((S, S), Image.LANCZOS)


def write_outputs(icon):
    root = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
    icons_dir = os.path.join(root, "src-tauri", "icons")
    os.makedirs(icons_dir, exist_ok=True)

    png_path = os.path.join(icons_dir, "icon.png")
    icon.save(png_path)
    print("wrote", png_path)

    iconset = os.path.join(root, "dist", "AppIcon.iconset")
    os.makedirs(iconset, exist_ok=True)
    for size in (16, 32, 128, 256, 512):
        icon.resize((size, size), Image.LANCZOS).save(os.path.join(iconset, f"icon_{size}x{size}.png"))
        icon.resize((size * 2, size * 2), Image.LANCZOS).save(os.path.join(iconset, f"icon_{size}x{size}@2x.png"))

    icns_path = os.path.join(icons_dir, "AppIcon.icns")
    subprocess.run(["iconutil", "-c", "icns", iconset, "-o", icns_path], check=True)
    print("wrote", icns_path)


if __name__ == "__main__":
    write_outputs(build_icon())
