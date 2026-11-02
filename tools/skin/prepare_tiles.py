"""Turns the generated skin photos into the small detail tiles the Rust bake embeds
(crates/handkit-mesh/src/skin.rs reads assets/skin/tiles/*.png at compile time).

Usage: python tools/skin/prepare_tiles.py [GEN_DIR]
GEN_DIR defaults to experiments/looks/gen (git-ignored: the sources are large generated PNGs).

Each skin tile (seamless, RGBA; 256 x 256, the knuckle tile 128 x 128 since it covers less skin,
so a tile pixel is about a texel of the 2048 atlas; lightly blurred so the bake does not alias):
  RGB  the photo's colour divided by its own blur (sigma 1/24 of the tile), so only the fine
       variation is left: 128 is 1.0, the range 0.5 to 1.5;
  A    height from the luminance, high-passed (sigma 1/40 of the back tile, 1/24 of the palm
       tile, 1/10 of the knuckle tile, so the palm's lines and the knuckle's wrinkles stay) and
       normalized to unit standard deviation: 128 is 0, 32 per standard deviation.
All filtering wraps around, so the tiles stay seamless.

The nail (256 x 288, RGB): the plate of nail.png cropped to its outline, root at the bottom,
free edge at the top.
"""
import sys
from pathlib import Path

import numpy as np
from PIL import Image

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "assets" / "skin" / "tiles"


def blur(a, sigma):
    """Gaussian blur with wrap-around, by FFT, over the first two axes."""
    h, w = a.shape[:2]
    fy = np.fft.fftfreq(h)[:, None]
    fx = np.fft.fftfreq(w)[None, :]
    g = np.exp(-2 * (np.pi * sigma) ** 2 * (fx * fx + fy * fy))
    if a.ndim == 3:
        g = g[..., None]
    return np.real(np.fft.ifft2(np.fft.fft2(a, axes=(0, 1)) * g, axes=(0, 1)))


def to_linear(c):
    return np.where(c <= 0.04045, c / 12.92, ((c + 0.055) / 1.055) ** 2.4)


def tile(src, name, size, hp_div):
    img = Image.open(src).convert("RGB").resize((size, size), Image.LANCZOS)
    rgb = blur(to_linear(np.asarray(img).astype(np.float64) / 255.0), 0.5)
    ratio = rgb / np.maximum(blur(rgb, size / 24), 1e-4)
    lum = rgb @ np.array([0.2126, 0.7152, 0.0722])
    hp = blur(lum, 0.9) - blur(lum, size / hp_div)
    hp /= hp.std()
    out = np.zeros((size, size, 4), np.uint8)
    out[..., :3] = np.clip((ratio - 0.5) * 255.0 + 0.5, 0, 255)
    out[..., 3] = np.clip(128 + hp * 32, 0, 255)
    Image.fromarray(out).save(OUT / f"{name}.png", optimize=True)
    print(f"{name}: ratio std {ratio.std(axis=(0, 1)).round(3)}, height clipped {np.mean(np.abs(hp) > 4):.4f}")


def nail(src):
    img = Image.open(src).convert("RGB")
    s = img.width / 1254
    crop = img.crop((int(205 * s), int(50 * s), int(1068 * s), int(1032 * s)))
    crop.resize((256, 288), Image.LANCZOS).save(OUT / "nail.png", optimize=True)


def main():
    gen = Path(sys.argv[1]) if len(sys.argv) > 1 else ROOT / "experiments" / "looks" / "gen"
    OUT.mkdir(parents=True, exist_ok=True)
    tile(gen / "skin_pores_tile.png", "back", 256, 40)
    tile(gen / "tile_palm.png", "palm", 256, 24)
    tile(gen / "tile_knuckle.png", "knuckle", 128, 10)
    nail(gen / "nail.png")


if __name__ == "__main__":
    main()
