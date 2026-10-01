#!/usr/bin/env python3
"""Offline asset preparation. Requires rlottie-python[full]==1.3.8.
Usage: python scripts/prepare-clawd.py /path/to/extracted-reference
Runtime/builds use the checked-in frames.bin and need neither Python nor Lottie.
"""
import collections
import hashlib
import itertools
import json
from pathlib import Path
import struct
import sys
from PIL import Image
from rlottie_python import LottieAnimation

SOURCE = Path(sys.argv[1])
OUTPUT = Path(__file__).resolve().parents[1] / 'assets/clawd'
WIDTH, HEIGHT = 32, 10
TRANSPARENT = (0, 0, 0, 0)
CLAY, EYES = (217, 119, 87), (20, 20, 19)


def sample(image):
    pixels = []
    for y in range(HEIGHT):
        for x in range(WIDTH):
            tile = image.crop((round(x * image.width / WIDTH), round(y * image.height / HEIGHT),
                               round((x + 1) * image.width / WIDTH), round((y + 1) * image.height / HEIGHT)))
            opaque = [p[:3] for p in tile.get_flattened_data() if p[3] > 200]
            if len(opaque) < tile.width * tile.height * .5:
                pixels.append(None)
                continue
            dark = [p for p in opaque if max(p) < 65]
            chosen = dark if len(dark) > len(opaque) * .4 else opaque
            pixels.append(collections.Counter(chosen).most_common(1)[0][0])
    # Preserve enclosed dark eye components that fall between coarse sample cells.
    # They otherwise disappear on walking frames as the face shifts by one source pixel.
    dark = {(x, y) for y in range(image.height) for x in range(image.width)
            if image.getpixel((x, y))[3] > 200 and max(image.getpixel((x, y))[:3]) < 40}
    while dark:
        component = [dark.pop()]
        edge = set()
        for x, y in component:
            for q in ((x-1,y),(x+1,y),(x,y-1),(x,y+1)):
                if q in dark:
                    dark.remove(q)
                    component.append(q)
                elif 0 <= q[0] < image.width and 0 <= q[1] < image.height:
                    edge.add(q)
        edge.difference_update(component)
        if not edge or len(component) < 8:
            continue
        skin = sum(image.getpixel(q)[3] > 128 and image.getpixel(q)[0] > 60
                   and image.getpixel(q)[0] > image.getpixel(q)[1] * 1.5 for q in edge)
        xs, ys = zip(*component)
        if skin < len(edge) * .45 or max(xs)-min(xs) > (max(ys)-min(ys)+1)*3:
            continue
        # Remove the coarse copy before placing the preserved eye below the forehead.
        # Include antialiased clay in the surrounding-skin check above.
        for sy in range(int(min(ys)*HEIGHT/image.height), min(HEIGHT, int(max(ys)*HEIGHT/image.height)+1)):
            for sx in range(int(min(xs)*WIDTH/image.width), min(WIDTH, int(max(xs)*WIDTH/image.width)+1)):
                c = pixels[sy*WIDTH+sx]
                if c is not None and max(c) < 65:
                    pixels[sy*WIDTH+sx] = CLAY
        x = max(0, min(WIDTH-1, round((min(xs)+max(xs)+1)*WIDTH/(2*image.width)-.5)))
        y = max(0, min(HEIGHT-1, round((min(ys)+max(ys)+1)*HEIGHT/(2*image.height)-.5)))
        top = next((r for r in range(HEIGHT) if pixels[r*WIDTH+x] == CLAY), y)
        y = max(y, min(HEIGHT-1, top+1))
        if pixels[y*WIDTH+x] is not None:
            pixels[y*WIDTH+x] = EYES
    # The square glyph is centered in a whole cell. Keep it off the right
    # forehead edge when the adjacent inner cell is entirely skin.
    for y in range(0, HEIGHT, 2):
        for x in range(2, WIDTH-2, 2):
            at = [y*WIDTH+x, y*WIDTH+x+1, (y+1)*WIDTH+x, (y+1)*WIDTH+x+1]
            colors = [pixels[i] for i in at]
            if (colors.count(EYES) == 1 and colors.count(CLAY) == 3
                    and pixels[y*WIDTH+x+2] is None
                    and all(pixels[i-2] == CLAY for i in at)):
                eye = at[colors.index(EYES)]
                pixels[eye], pixels[eye-2] = CLAY, EYES
    return pixels


clips, provenance = [], []
for entry in json.loads((SOURCE / 'manifest.json').read_text()):
    if not entry['file'].endswith('.json') or 'fps' not in entry:
        continue
    path = SOURCE / entry['file']
    data = json.loads(path.read_text())
    name = path.stem.removeprefix('clawd-').removeprefix('Clawd-').lower()
    with LottieAnimation.from_file(str(path)) as anim:
        frames = [sample(anim.render_pillow_frame(frame_num=f, width=274, height=184))
                  for f in range(int(data['op'] - data['ip']))]
    clips.append((name, frames))
    provenance.append(dict(name=name, url=entry['url'], sha256=hashlib.sha256(path.read_bytes()).hexdigest(), frames=len(frames)))
    print(name, len(frames), flush=True)

kite = json.loads((SOURCE / 'animations/Clawd-Kite.frames.json').read_text())
# Original page scales the character's lower body to 59.4 px, not the entire kite.
body = [(i % kite['w']) for i, p in enumerate(kite['frames'][0])
        if i // kite['w'] >= int(.6 * kite['h']) and p != 255]
scale = 59.4 / (max(body) - min(body) + 1)
frames = []
for f in kite['frames']:
    im = Image.new('RGBA', (kite['w'], kite['h']))
    im.putdata([TRANSPARENT if p == 255 else (*tuple(bytes.fromhex(kite['palette'][p][1:])), 255) for p in f])
    im = im.resize((round(kite['w'] * scale * 2), round(kite['h'] * scale * 2)), Image.Resampling.NEAREST)
    stage = Image.new('RGBA', (274, 184))
    stage.paste(im, ((274-im.width)//2, 184-im.height))
    frames.append(sample(stage))
clips.append(('kite', frames))
provenance.append(dict(name='kite', source='https://claude.dev/_next/static/chunks/3ap3r640smdew.js',
                       sha256=hashlib.sha256((SOURCE/'animations/Clawd-Kite.frames.json').read_bytes()).hexdigest(), frames=len(frames)))
counts = collections.Counter(p for _, frames in clips for f in frames for p in f if p is not None)
palette = [None, CLAY, EYES]
palette += [p for p, _ in counts.most_common() if p not in palette][:61]
nearest = {None: 0}

def distance(a, b):
    return sum((x-y)**2 for x, y in zip(a, b))


def index(p):
    if p not in nearest:
        nearest[p] = min(range(1, len(palette)), key=lambda i: distance(p, palette[i]))
    return nearest[p]


def cell(samples):
    # A square glyph keeps an enclosed eye small and above the half-cell baseline.
    # This cell is already fully opaque, so its clay background preserves the head.
    if samples.count(2) == 1 and samples.count(1) == 3:
        return bytes([16, 2, 1])
    colors = sorted(set(samples))
    if len(colors) == 1:
        return bytes([0 if colors[0] == 0 else 15, colors[0], 0])
    # Preserve transparency. Otherwise choose the closest two colors for the 4 quadrants.
    pairs = [(0, c) for c in colors if c] if 0 in colors else itertools.combinations(colors, 2)
    def error(pair):
        return sum(0 if c in pair else min(distance(palette[c], palette[v]) for v in pair if v) for c in samples)
    bg, fg = min(pairs, key=error)
    mask = 0
    for bit, c in enumerate(samples):
        if c == fg or (c != bg and (bg == 0 or distance(palette[c], palette[fg]) < distance(palette[c], palette[bg]))):
            mask |= 1 << bit
    return bytes([mask, fg, bg])

out = bytearray(b'CLWD2') + bytes([WIDTH//2, HEIGHT//2, len(palette), len(clips)])
for p in palette:
    out += bytes(p or (0, 0, 0))
for name, frames in clips:
    out += bytes([len(name)]) + name.encode() + struct.pack('<H', len(frames))
    for frame in frames:
        p = list(map(index, frame))
        for y in range(0, HEIGHT, 2):
            for x in range(0, WIDTH, 2):
                out += cell([p[y*WIDTH+x], p[y*WIDTH+x+1], p[(y+1)*WIDTH+x], p[(y+1)*WIDTH+x+1]])
OUTPUT.mkdir(exist_ok=True, parents=True)
(OUTPUT/'frames.bin').write_bytes(out)
(OUTPUT/'sources.json').write_text(json.dumps(provenance, ensure_ascii=False, indent=2)+'\n')
print('bytes', len(out), 'clips', len(clips), flush=True)
