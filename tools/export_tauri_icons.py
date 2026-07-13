#!/usr/bin/env python3
"""Export the Tauri icon ladder from a square source image."""

from __future__ import annotations

import argparse
from pathlib import Path

from PIL import Image, ImageDraw


PNG_SIZES = {
    "32x32.png": 32,
    "128x128.png": 128,
    "128x128@2x.png": 256,
    "icon.png": 512,
    "Square30x30Logo.png": 30,
    "Square44x44Logo.png": 44,
    "Square71x71Logo.png": 71,
    "Square89x89Logo.png": 89,
    "Square107x107Logo.png": 107,
    "Square142x142Logo.png": 142,
    "Square150x150Logo.png": 150,
    "Square284x284Logo.png": 284,
    "Square310x310Logo.png": 310,
    "StoreLogo.png": 50,
}

def load_square(path: Path) -> Image.Image:
    image = Image.open(path).convert("RGBA")
    if image.width != image.height:
        side = min(image.width, image.height)
        left = (image.width - side) // 2
        top = (image.height - side) // 2
        image = image.crop((left, top, left + side, top + side))
    return image


def resize_icon(image: Image.Image, size: int) -> Image.Image:
    return image.resize((size, size), Image.Resampling.LANCZOS)


def rounded_mask(image: Image.Image, radius: int) -> Image.Image:
    if radius <= 0:
        return image

    radius = min(radius, image.width // 2, image.height // 2)
    mask = Image.new("L", image.size, 0)
    draw = ImageDraw.Draw(mask)
    draw.rounded_rectangle((0, 0, image.width - 1, image.height - 1), radius=radius, fill=255)

    rounded = image.copy()
    rounded.putalpha(mask)
    return rounded


def fit_to_canvas(image: Image.Image, scale: float) -> Image.Image:
    if scale >= 1:
        return image
    if scale <= 0:
        raise ValueError("--content-scale must be greater than 0")

    size = max(1, round(image.width * scale))
    fitted = resize_icon(image, size)
    canvas = Image.new("RGBA", image.size, (0, 0, 0, 0))
    left = (image.width - size) // 2
    top = (image.height - size) // 2
    canvas.alpha_composite(fitted, (left, top))
    return canvas


def write_pngs(image: Image.Image, out_dir: Path) -> None:
    for name, size in PNG_SIZES.items():
        resize_icon(image, size).save(out_dir / name, optimize=True)


def write_ico(image: Image.Image, out_dir: Path) -> None:
    image.save(
        out_dir / "icon.ico",
        sizes=[(16, 16), (24, 24), (32, 32), (48, 48), (64, 64), (128, 128), (256, 256)],
    )


def write_icns(image: Image.Image, out_dir: Path) -> None:
    image.save(
        out_dir / "icon.icns",
        sizes=[(16, 16), (32, 32), (64, 64), (128, 128), (256, 256), (512, 512), (1024, 1024)],
    )


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--source", required=True, type=Path)
    parser.add_argument("--out", default=Path("src-tauri/icons"), type=Path)
    parser.add_argument(
        "--corner-radius",
        default=0,
        type=int,
        help="Optional source-pixel rounded-corner mask radius before exporting.",
    )
    parser.add_argument(
        "--content-scale",
        default=1.0,
        type=float,
        help="Optional optical scale for the source inside a transparent canvas.",
    )
    args = parser.parse_args()

    out_dir = args.out
    out_dir.mkdir(parents=True, exist_ok=True)
    image = fit_to_canvas(rounded_mask(load_square(args.source), args.corner_radius), args.content_scale)

    write_pngs(image, out_dir)
    write_ico(image, out_dir)
    write_icns(image, out_dir)


if __name__ == "__main__":
    main()
