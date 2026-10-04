#!/usr/bin/env python3
"""Rebuild the Cussy logo kit. Requires CairoSVG, fontTools, and Pillow."""

from pathlib import Path
import hashlib
import json
import math
import shutil
import zipfile

import cairosvg
from cairosvg.surface import PDFSurface
import cairocffi as cairo
from fontTools.pens.boundsPen import BoundsPen
from fontTools.pens.svgPathPen import SVGPathPen
from fontTools.ttLib import TTFont
from fontTools.varLib.instancer import instantiateVariableFont
from PIL import Image

ROOT = Path(__file__).resolve().parent.parent
BRAND = ROOT / "assets/brand"
INK = "#1B2F3A"
CORAL = "#F16B50"
PAPER = "#FAF8F4"
LIGHT = "#F2F4F5"
NIGHT_CORAL = "#FF967E"
FONT_REVISION = "00a38a53f92aef923b9353f40128e8f4552ddae4"
FONT = instantiateVariableFont(TTFont(BRAND / "source/SpaceGrotesk.ttf"), {"wght": 700})
GLYPHS = FONT.getGlyphSet()
CMAP = FONT.getBestCmap()
UPM = FONT["head"].unitsPerEm


class LogoPDFSurface(PDFSurface):
    """Avoid build-time metadata changing otherwise identical vector PDFs."""

    def _create_surface(self, width, height):
        surface, width, height = super()._create_surface(width, height)
        surface.set_metadata(cairo.PDF_METADATA_CREATE_DATE, "2026-10-04T00:00:00Z")
        return surface, width, height

# Original C silhouette and rising curve, on a 256-unit grid.
C_PATH = (
    "M200 70C179 45 152 32 120 32C65 32 24 74 24 128"
    "C24 182 65 224 120 224C151 224 178 212 198 189L170 162"
    "C157 177 141 184 120 184C88 184 65 160 65 128"
    "C65 96 88 72 120 72C141 72 157 80 169 94Z"
)
CURVE = "M95 150C115 159 127 153 142 134C161 111 181 106 210 116"


def mark(body, accent):
    return (f'<path fill="{body}" d="{C_PATH}"/>'
            f'<path d="{CURVE}" fill="none" stroke="{accent}" stroke-width="16" '
            'stroke-linecap="round" stroke-linejoin="round"/>'
            f'<circle cx="210" cy="116" r="11" fill="{accent}"/>')


def lettering(text, size, color, x, baseline, tracking=0):
    scale = size / UPM
    output = []
    cursor = 0
    for character in text:
        glyph = GLYPHS[CMAP[ord(character)]]
        pen = SVGPathPen(GLYPHS)
        glyph.draw(pen)
        if pen.getCommands():
            output.append(f'<path d="{pen.getCommands()}" transform="translate({cursor:.4f} 0)"/>')
        cursor += glyph.width + tracking / scale
    return (f'<g fill="{color}" transform="translate({x} {baseline}) scale({scale} {-scale})">'
            + "".join(output) + "</g>", (cursor - tracking / scale) * scale)


def svg(width, height, title, body, description, background=None):
    backdrop = f'<path fill="{background}" d="M0 0H{width}V{height}H0Z"/>' if background else ""
    return (f'<svg xmlns="http://www.w3.org/2000/svg" width="{width}" height="{height}" '
            f'viewBox="0 0 {width} {height}" role="img" aria-labelledby="title desc">'
            f'<title id="title">{title}</title><desc id="desc">{description}</desc>'
            + backdrop + body + "</svg>\n")


def export(name, width, height, content, png_widths=()):
    source = BRAND / f"{name}.svg"
    source.write_text(content, encoding="utf-8")
    data = content.encode()
    LogoPDFSurface.convert(bytestring=data, write_to=str(BRAND / "pdf" / f"{name}.pdf"))
    # EPS is a flat-color vector export; white variants need a dark placement surface.
    eps = BRAND / "eps" / f"{name}.eps"
    cairosvg.svg2eps(bytestring=data, write_to=str(eps))
    eps.write_text("\n".join(line.rstrip() for line in eps.read_text().splitlines()
                             if not line.startswith("%%CreationDate:")) + "\n")
    for size in png_widths:
        cairosvg.svg2png(bytestring=data, write_to=str(BRAND / "png" / f"{name}-{size}.png"),
                        output_width=size, output_height=round(height * size / width))


def main():
    shutil.copy2(ROOT / "LICENSE", BRAND / "LICENSE.txt")
    for directory in ["png", "pdf", "eps"]:
        (BRAND / directory).mkdir(parents=True, exist_ok=True)
    description = "An open C with a rising graph curve and a plotted endpoint."
    variants = {
        "light": (INK, CORAL),
        "dark": (LIGHT, NIGHT_CORAL),
        "black": ("#000000", "#000000"),
        "white": ("#FFFFFF", "#FFFFFF"),
    }
    bounds = []
    for c in "Cussy":
        pen = BoundsPen(GLYPHS)
        GLYPHS[CMAP[ord(c)]].draw(pen)
        bounds.append(pen.bounds)
    baseline = 128 + (max(b[3] for b in bounds) + min(b[1] for b in bounds)) / 2 * 200 / UPM
    for variant, (body, accent) in variants.items():
        export(f"cussy-mark-{variant}", 256, 256,
               svg(256, 256, "Cussy", mark(body, accent), description),
               [128, 256, 512, 1024, 2048] if variant in {"light", "dark"} else [512])
        word, advance = lettering("Cussy", 200, body, 282, round(baseline, 4), -4)
        width = math.ceil(282 + advance + 22)
        export(f"cussy-wordmark-{variant}", width, 256,
               svg(width, 256, "Cussy", mark(body, accent) + word,
                   "Cussy wordmark with its C and graph symbol."),
               [840, 1680] if variant in {"light", "dark"} else [840])
    avatar = svg(256, 256, "Cussy app icon", '<g transform="translate(16 16) scale(.875)">'
                 + mark(LIGHT, NIGHT_CORAL) + "</g>", description, INK)
    export("cussy-avatar", 256, 256, avatar, [256, 512, 1024])
    with Image.open(BRAND / "png/cussy-avatar-256.png") as icon:
        icon.save(BRAND / "cussy.ico", sizes=[(s, s) for s in (16, 24, 32, 48, 64, 128, 256)])
    # Social preview uses the same geometry and outlined type, not an embedded screenshot.
    word, _ = lettering("Cussy", 176, INK, 380, 327, -3)
    tagline, _ = lettering("C-inspired code. Graphs built in.", 34, INK, 142, 451)
    social = svg(1280, 640, "Cussy — C-inspired code. Graphs built in.",
                 '<g transform="translate(132 161) scale(.84)">' + mark(INK, CORAL) + "</g>"
                 + word + tagline,
                 "The Cussy programming language logo and tagline on a warm paper background.", PAPER)
    export("cussy-social", 1280, 640, social, [1280])
    editor_icon = ROOT / "editors/vscode/images/cussy-icon.png"
    editor_icon.parent.mkdir(parents=True, exist_ok=True)
    shutil.copy2(BRAND / "png/cussy-avatar-256.png", editor_icon)
    # Keep a stable manifest; no build timestamps or machine-specific paths.
    files = sorted(p for p in BRAND.rglob("*") if p.is_file()
                   and p.name not in {"manifest.json", "cussy-brand-kit.zip"})
    manifest = {
        "name": "Cussy", "version": 1,
        "creation": "Original vector artwork generated by scripts/build_brand.py; no stock mark or raster tracing.",
        "font": {"name": "Space Grotesk Bold", "license": "SIL OFL 1.1",
                 "source": f"https://github.com/google/fonts/tree/{FONT_REVISION}/ofl/spacegrotesk",
                 "exported_as": "outlines"},
        "colors": {"ink": INK, "coral": CORAL, "paper": PAPER, "dark_background_lettering": LIGHT,
                   "dark_background_accent": NIGHT_CORAL},
        "files": [{"path": p.relative_to(BRAND).as_posix(), "bytes": p.stat().st_size,
                   "sha256": hashlib.sha256(p.read_bytes()).hexdigest()} for p in files],
    }
    (BRAND / "manifest.json").write_text(json.dumps(manifest, indent=2) + "\n", encoding="utf-8")
    with zipfile.ZipFile(BRAND / "cussy-brand-kit.zip", "w", zipfile.ZIP_DEFLATED) as archive:
        for path in files + [BRAND / "manifest.json"]:
            info = zipfile.ZipInfo("cussy-brand-kit/" + path.relative_to(BRAND).as_posix(), (2026, 10, 4, 0, 0, 0))
            info.compress_type = zipfile.ZIP_DEFLATED
            info.external_attr = 0o100644 << 16
            archive.writestr(info, path.read_bytes())
    print(f"Wrote {len(files)} logo assets and metadata to {BRAND}")


if __name__ == "__main__":
    main()
