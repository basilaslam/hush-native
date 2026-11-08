"""Rebuild Hush's vector and raster brand assets.
Requires Python fontTools and ImageMagick's magick CLI. Not needed for app builds.
"""
from pathlib import Path
import subprocess
from fontTools.ttLib import TTFont
from fontTools.pens.svgPathPen import SVGPathPen

ROOT = Path(__file__).resolve().parent.parent
OUT = ROOT / 'assets/brand'
OUT.mkdir(parents=True, exist_ok=True)
FOREST = '#16251f'
LEAF = '#c3e6a3'
# Two facing conversation forms join into one H. The lower right cut is a
# speech tail, and is deliberately retained at the smallest icon sizes.
MARK = 'M68 55H91Q107 55 107 71V109H149V71Q149 55 165 55H188Q204 55 204 71V166Q204 182 188 182H172L149 205V151H107V166Q107 182 91 182H68Q52 182 52 166V71Q52 55 68 55Z'
def svg(body, viewbox='0 0 256 256', title='Hush'):
    return f'<svg xmlns="http://www.w3.org/2000/svg" viewBox="{viewbox}" role="img" aria-label="{title}"><title>{title}</title>{body}</svg>\n'
tile = f'<rect width="256" height="256" rx="58" fill="{LEAF}"/><path d="{MARK}" fill="{FOREST}"/>'
(OUT / 'hush-icon.svg').write_text(svg(tile))
(OUT / 'hush-mark.svg').write_text(svg(f'<path d="{MARK}" fill="{FOREST}"/>'))
(OUT / 'hush-mark-light.svg').write_text(svg(f'<path d="{MARK}" fill="{LEAF}"/>'))
font = TTFont(ROOT / 'native/assets/Manrope-Display.ttf')
glyphs = font.getGlyphSet()
cmap = font.getBestCmap()
scale = 42 / font['head'].unitsPerEm
x = 0
paths = []
for character in 'hush':
    name = cmap[ord(character)]
    pen = SVGPathPen(glyphs)
    glyphs[name].draw(pen)
    paths.append(f'<path d="{pen.getCommands()}" transform="translate({x},0)"/>')
    x += font['hmtx'][name][0]
width = round(82 + x * scale + 4)
word = f'<g transform="scale(.25)">{tile}</g><g fill="{FOREST}" transform="translate(82,48) scale({scale},-{scale})">{"".join(paths)}</g>'
(OUT / 'hush-wordmark.svg').write_text(svg(word, f'0 0 {width} 64'))
for dest in [ROOT / 'public/favicon.svg', ROOT / 'assets/icon.svg']:
    dest.write_text(svg(tile))

def magick(*args):
    subprocess.run(['magick', *map(str, args)], check=True)

icon = OUT / 'hush-icon-1024.png'
native = ROOT / 'native/assets'
magick('-background', 'none', '-density', '384', OUT / 'hush-icon.svg', '-resize', '1024x1024', icon)
magick(icon, '-resize', '512x512', OUT / 'hush-icon-512.png')
magick(icon, '-resize', '256x256', native / 'icon.png')
magick(native / 'icon.png', '-depth', '8', f'RGBA:{native / "icon.rgba"}')
magick(icon, '-define', 'icon:auto-resize=256,128,64,48,32,16', native / 'icon.ico')
magick('-background', 'none', '-density', '600', OUT / 'hush-wordmark.svg', '-resize', '1000x', OUT / 'hush-wordmark.png')
assert (native / 'icon.rgba').stat().st_size == 256 * 256 * 4
print('Rebuilt vector, PNG, native RGBA, Windows ICO and favicon assets.')
