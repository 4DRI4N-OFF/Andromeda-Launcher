#!/usr/bin/env python3
"""Rebuild the static backdrop and occluder alpha from the accepted v23 source.
Requires numpy, Pillow and cairosvg. Does not change original logo paths.
"""
from pathlib import Path
import io
import json
import zipfile
import numpy as np
from PIL import Image
import cairosvg
ROOT = Path(__file__).resolve().parents[1]
assets = ROOT / 'app/assets/startup'
assets.mkdir(parents=True, exist_ok=True)
width, height = 1920, 1080
y, x = np.mgrid[:height, :width]
# CSS radial-gradient(ellipse at 50% 47%, #201638 0%, #100b20 45%, #080611 80%).
t = np.sqrt(((x + .5 - 960) / (960 * 2**.5))**2 + ((y + .5 - 507.6) / (572.4 * 2**.5))**2)
c0, c1, c2 = np.array([32,22,56]), np.array([16,11,32]), np.array([8,6,17])
a, b = np.clip(t/.45,0,1)[...,None], np.clip((t-.45)/.35,0,1)[...,None]
rgb = np.where((t<.45)[...,None], c0+(c1-c0)*a, c1+(c2-c1)*b)
image = Image.fromarray(np.uint8(np.round(rgb)), 'RGB')
image.save(assets/'backdrop.png')
(assets/'backdrop.rgb').write_bytes(image.tobytes())
with zipfile.ZipFile(ROOT/'docs/startup-v23/andromeda-intro-v23-source.zip') as archive:
    path = json.loads(archive.read('intro-v23/src/occluder.json'))
svg = f'<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 100 100" width="1000" height="1000"><path fill="white" stroke="white" stroke-width=".25" d="{path}"/></svg>'
png = cairosvg.svg2png(bytestring=svg.encode())
alpha = Image.open(io.BytesIO(png)).convert('RGBA').getchannel('A')
(assets/'outer-alpha.bin').write_bytes(alpha.tobytes())
print('Rebuilt backdrop.png, backdrop.rgb and outer-alpha.bin from accepted v23 geometry.')
