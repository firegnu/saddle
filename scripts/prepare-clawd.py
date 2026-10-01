#!/usr/bin/env python3
"""Build the curated, hand-timed Clawd poses. Python stdlib only; no runtime dependency."""
import json
from pathlib import Path
import struct

OUTPUT = Path(__file__).resolve().parents[1] / 'assets/clawd'
WIDTH, HEIGHT = 32, 6
# Transparent, clay, eyes, paper, coffee, blues, accent, bandana, leaf, rose, hoop, blush.
PALETTE = [(0, 0, 0), (217, 119, 87), (20, 20, 19), (239, 217, 179),
           (113, 74, 54), (79, 115, 139), (139, 177, 195), (165, 157, 139), (159, 66, 54),
           (106, 151, 101), (224, 113, 139), (220, 176, 91), (244, 151, 174)]
BASE = [
    '................................',
    '................................',
    '...........#########............',
    '.........#############..........',
    '...........#########............',
    '...........#.#...#.#............',
]


def rect(pixels, x, y, w, h, color):
    assert 0 <= x < x+w <= WIDTH and 0 <= y < y+h <= HEIGHT
    for row in range(y, y+h):
        pixels[row*WIDTH+x:row*WIDTH+x+w] = [color]*w


def shift(pixels, row, dx):
    # Slide one pixel row sideways; used for glances and leans above the planted waist and feet.
    line = pixels[row*WIDTH:(row+1)*WIDTH]
    pixels[row*WIDTH:(row+1)*WIDTH] = [0]*dx + line[:-dx] if dx > 0 else line[-dx:] + [0]*-dx


def glance_right(pixels):
    # The head top follows the eyes so a skin margin stays outside the right eye.
    shift(pixels, 2, 1)
    return (7, 9)


def pose(kind='rest', stage=0):
    p = [int(c == '#') for row in BASE for c in row]
    eyes = [16, 16]
    eye_columns = (6, 8)
    extra = []
    if kind == 'walking':
        # Keep the silhouette and floor fixed; alternate only the inner feet.
        rect(p, 13, 5, 5, 1, 0)
        for x in ((14, 17) if stage == 1 else (13, 16) if stage == 2 else (13, 17)):
            rect(p, x, 5, 1, 1, 1)
    elif kind == 'swaying' and stage:
        # Head and arms lean one pixel; waist and all four feet stay planted.
        shift(p, 2, -1 if stage == 1 else 1)
        shift(p, 3, -1 if stage == 1 else 1)
        if stage == 2:
            eye_columns = (7, 9)
    elif kind == 'turning' and stage:
        # Compress into a side profile, then reopen at the same floor/centre.
        rect(p, 9, 2, 13, 4, 0)
        rect(p, 12, 2, 7, 3, 1)
        rect(p, 10, 3, 11, 1, 1)
        for x in (12, 14, 17, 18):
            rect(p, x, 5, 1, 1, 1)
        eyes = [16]
        eye_columns = (8,) if stage == 1 else (7,)
    elif kind == 'looking':
        if stage == 1:
            eye_columns = glance_right(p)
        elif stage == 2:
            # Peek left with the whole head: the approved eyes already sit left of centre,
            # so moving only the eyes would press the left eye against the forehead edge.
            shift(p, 2, -2)
            shift(p, 3, -1)
            eye_columns = (5, 7)
        elif stage == 3:
            eyes = [17, 17]
    elif kind == 'waving':
        rect(p, 9, 3, 2, 1, 0)
        rect(p, 9, 2, 2, 1, 1)
        if stage:
            rect(p, 8 if stage == 1 else 9, 1, 2, 1, 1)
            eyes = [21, 21]
    elif kind == 'thinking':
        if stage:
            eye_columns = glance_right(p)
            # A growing "..." replaces the old one-eye wink.
            extra = [(x, 0, 29, 6, 0) for x in (11, 12, 13)[:stage]]
        rect(p, 20, 3, 2, 1, 0)
        rect(p, 20, 2, 2, 1, 1)
    elif kind == 'coffee':
        y = 2 if stage else 3
        x = 20 if stage else 22
        rect(p, 20, 3, 2, 1, 1)
        rect(p, x, y, 2, 2, 3)
        rect(p, x, y, 2, 1, 4)
        rect(p, x+2, y+1, 1, 1, 3)
        if stage == 2:
            eyes = [17, 17]
            rect(p, 21, 1, 1, 1, 7)
    elif kind == 'notebook':
        eye_columns = (7, 9)
        rect(p, 21, 3, 1, 1, 1)
        rect(p, 22, 2, 4, 2, 5)
        rect(p, 23, 2, 2, 2, 3)
        if stage:
            rect(p, 24, 2, 1, 2, 5)
        if stage == 2:
            eyes = [17, 17]
    elif kind == 'headphones':
        rect(p, 11, 1, 9, 1, 5)
        rect(p, 10, 2, 1, 2, 6)
        rect(p, 9, 3, 1, 1, 0)
        rect(p, 21, 3, 1, 1, 0)
        rect(p, 20, 2, 1, 2, 6)
        if stage:
            eyes = [17, 17]
            # Step in place to the beat; all four feet stay visible.
            rect(p, 13, 5, 5, 1, 0)
            for x in ((14, 17) if stage == 1 else (13, 16)):
                rect(p, x, 5, 1, 1, 1)
    elif kind == 'watch':
        eye_columns = glance_right(p)
        rect(p, 20, 3, 3, 1, 1)
        rect(p, 22, 2, 2, 2, 5)
        # The light hand pixel ticks between the top and bottom of the dial.
        rect(p, 22, 3 if stage == 2 else 2, 1, 1, 3)
        if stage == 3:
            eyes = [17, 17]
    elif kind == 'snooze':
        eyes = [17, 17]
        if stage:
            extra = [(x, 0, 18, 7, 0) for x in (9, 11, 13)[:stage]]
    elif kind == 'desktop':
        eye_columns = glance_right(p)
        rect(p, 24, 0, 8, 2, 5)
        rect(p, 25, 1, 6, 1, 6)
        rect(p, 27, 2, 2, 2, 7)
        rect(p, 24, 4, 8, 1, 7)
        rect(p, 22, 5, 6, 1, 3)
        rect(p, 20, 3, 2, 1, 0)
        # The typing hand stays attached at the waist instead of reading as a fifth foot.
        rect(p, 20, 3 if stage == 1 else 4, 2, 1, 1)
        if stage == 2:
            extra = [(13, 0, 19, 3, 5)]
    elif kind == 'arcade':
        eye_columns = (7, 9)
        rect(p, 24, 0, 6, 6, 5)
        rect(p, 24, 0, 6, 1, 11)
        rect(p, 24, 2, 6, 1, 6)
        extra = [(12, 2, 16, 10, 5), (14, 2, 16, 3, 5),
                 (11, 1, 16, 10, 0), (11, 2, 30, 7, 0)]
        rect(p, 20, 3 if stage != 1 else 4, 2, 1, 1)
        if stage == 2:
            extra += [(13, 1, 22, 3, 5)]
            eyes = [21, 21]
    elif kind == 'bubble':
        rect(p, 20, 3, 2, 1, 0)
        rect(p, 20, 2, 2, 1, 1)
        extra = [(11, 1, 26, 6, 0)]
        if stage == 1:
            extra += [(12, 1, 28, 6, 0)]
        elif stage == 2:
            extra += [(13, 1, 27, 6, 0)]
        elif stage == 3:
            extra += [(14, 0, 27, 6, 0)]
        elif stage == 4:
            extra += [(14, 0, 22, 6, 0)]
    elif kind == 'spark':
        rect(p, 20, 3, 4, 1, 1)
        if stage == 1:
            extra = [(12, 1, 22, 11, 0)]
        elif stage == 2:
            extra = [(12, 0, 22, 3, 0), (13, 1, 22, 11, 0)]
        elif stage == 3:
            extra = [(13, 0, 29, 11, 0), (14, 1, 29, 11, 0)]
    elif kind == 'lightbulb':
        rect(p, 20, 3, 2, 1, 0)
        rect(p, 20, 2, 2, 1, 1)
        color = 7 if stage == 0 else 11
        rect(p, 25, 0, 2, 1, color)
        rect(p, 24, 1, 4, 1, color)
        rect(p, 25, 2, 2, 1, 7)
        if stage == 1:
            extra = [(14, 0, 22, 3, 0)]
        elif stage == 2:
            eyes = [21, 21]
    elif kind == 'watering':
        rect(p, 20, 3, 2, 1, 1)
        rect(p, 22, 2, 4, 2, 5)
        rect(p, 26, 2 if stage == 0 else 3, 2, 1, 5)
        rect(p, 28, 4, 4, 2, 4)
        rect(p, 30, 2, 1, 2, 9)
        rect(p, 31, 2, 1, 1, 9)
        if stage in (1, 2):
            extra = [(14, 1, 29, 6, 0)] if stage == 1 else [(14, 2, 29, 6, 4)]
        if stage == 3:
            rect(p, 29, 2, 1, 1, 9)
            eyes = [21, 21]
    elif kind == 'berrypicking':
        rect(p, 24, 2, 6, 3, 9)
        rect(p, 26, 5, 2, 1, 4)
        extra = [(14, 1, 16, 10, 9)]
        if stage < 2:
            extra += [(12, 1, 16, 10, 9)]
        if stage == 1:
            rect(p, 20, 3, 4, 1, 1)
        elif stage >= 2:
            rect(p, 20, 3, 2, 1, 1)
            extra += [(11, 1, 16, 10, 1)]
            if stage == 3:
                eyes = [21, 21]
    elif kind == 'blooming':
        rect(p, 24, 4, 6, 2, 4)
        rect(p, 26, 2, 2, 2, 9)
        if stage == 0:
            rect(p, 26, 1, 2, 1, 9)
        elif stage == 1:
            rect(p, 26, 1, 2, 1, 10)
        else:
            rect(p, 26, 0, 2, 1, 10)
            rect(p, 24, 1, 6, 1, 10)
            rect(p, 26, 1, 2, 1, 11)
            rect(p, 26, 2, 2, 1, 10)
            if stage == 3:
                eyes = [21, 21]
                extra = [(15, 0, 22, 3, 0)]
    elif kind == 'fishing':
        rect(p, 20, 3, 2, 1, 1)
        extra = [(11, 1, 19, 7, 0), (12, 1, 19, 7, 0),
                 (13, 1, 31, 7, 0), (13, 2, 30, 7, 0),
                 (12, 2, 32, 5, 0), (14, 2, 32, 5, 0), (15, 2, 32, 5, 0)]
        if stage == 1:
            extra += [(13, 2, 29, 6, 0)]
        elif stage >= 2:
            extra = [(11, 1, 19, 7, 0), (12, 1, 35, 7, 0), (13, 0, 31, 7, 0),
                     (13, 1, 30, 7, 0), (13, 2, 33, 11, 0),
                     (14, 2, 34, 11, 0), (15, 2, 33, 11, 0)]
            if stage == 3:
                eyes = [21, 21]
    elif kind in ('dancing', 'dancinghappy'):
        if stage:
            side = 9 if stage == 1 else 20
            rect(p, side, 3, 2, 1, 0)
            rect(p, side, 2, 2, 1, 1)
            rect(p, 13, 5, 5, 1, 0)
            for x in ((14, 17) if stage == 1 else (13, 16)):
                rect(p, x, 5, 1, 1, 1)
            if kind == 'dancinghappy':
                rect(p, 8 if stage == 1 else 21, 1, 2, 1, 1)
                extra = [(12 if stage == 1 else 3, 0, 36, 11, 0)]
                eyes = [21, 21]
    elif kind == 'phone':
        # Portrait screen remains outside the face; thumb taps its lower edge.
        eye_columns = (7, 9)
        x = 24 if stage == 0 else 22
        rect(p, 20, 3, x-20, 1, 1)
        rect(p, x, 3, 2, 3, 5)
        rect(p, x, 4, 1, 1, 6)
        if stage == 2:
            rect(p, 20, 4, 2, 1, 1)
            rect(p, x, 4, 1, 1, 3)
        if stage == 3:
            eyes = [17, 17]
    elif kind == 'laptop':
        eye_columns = (7, 9)
        rect(p, 22, 4, 10, 1, 7)
        if stage:
            rect(p, 22, 2, 8, 2, 5)
            rect(p, 23, 3, 6, 1, 6)
            rect(p, 20, 3, 2, 1, 0)
            rect(p, 20, 3 if stage == 2 else 4, 2, 1, 1)
        if stage == 3:
            eyes = [17, 17]
    elif kind == 'rose':
        # A flower held to the side: petals, green stem and a single leaf.
        rect(p, 20, 3, 4, 1, 1)
        rect(p, 24, 2, 1, 4, 9)
        rect(p, 25, 4, 2, 1, 9)
        rect(p, 24, 0, 4, 1, 10)
        rect(p, 22, 1, 6, 1, 10)
        rect(p, 24, 1, 2, 1, 8)
        if stage == 1:
            eyes = [17, 17]
        if stage == 2:
            eyes = [21, 21]
            extra = [(14, 0, 22, 3, 0)]
    elif kind == 'heart':
        rect(p, 20, 3, 4, 1, 1)
        if stage:
            eyes = [21, 21]
            extra = [(12, 1, 23, 10 if stage == 1 else 12, 0)]
            if stage == 2:
                extra += [(11, 0, 22, 10, 0), (13, 0, 22, 10, 0)]
    elif kind == 'meditating':
        eyes = [17, 17]
        if stage:
            # Palms up, tucked legs, no levitation or vertical body movement.
            rect(p, 9, 3, 2, 1, 0)
            rect(p, 20, 3, 2, 1, 0)
            rect(p, 9, 4, 2, 1, 1)
            rect(p, 20, 4, 2, 1, 1)
            rect(p, 11, 5, 9, 1, 0)
            rect(p, 12, 5, 3, 1, 1)
            rect(p, 16, 5, 3, 1, 1)
            if stage == 2:
                rect(p, 8, 4, 1, 1, 1)
                rect(p, 22, 4, 1, 1, 1)
    elif kind == 'hulahoop':
        # A shallow ellipse behind the waist, never covering the face or feet.
        if stage == 0:
            extra = [(11, 2, 24, 11, 0), (12, 2, 19, 11, 0), (13, 2, 25, 11, 0)]
        else:
            # Swing off-centre left then right, like the original hoop's orbit.
            left, right = (2, 11) if stage == 1 else (3, 12)
            extra = [(left, 2, 24, 11, 0), (right, 2, 25, 11, 0)]
            extra += [(x, 2, 19, 11, 0) for x in range(left+1, 5)]
            extra += [(x, 2, 19, 11, 0) for x in range(10, right)]
            rect(p, 13, 5, 5, 1, 0)
            for x in ((13, 17) if stage == 1 else (14, 16)):
                rect(p, x, 5, 1, 1, 1)
    elif kind == 'pirate':
        rect(p, 11, 1, 9, 1, 8)
        rect(p, 9, 1, 2, 1, 8)
        if stage in (0, 1):
            rect(p, 20, 3, 2, 1, 0)
            rect(p, 20, 2, 2, 1, 1)
        if stage in (2, 3):
            rect(p, 9, 3, 2, 1, 0)
            rect(p, 9, 2, 2, 1, 1)
            rect(p, 8, 1, 2, 1, 1)
        if stage:
            eyes = [17 if stage == 3 else 16]
            eye_columns = (6,)
            # Connected left-to-right strap, one inset patch, and an unobstructed other eye.
            extra = [(7, 1, 19, 2, 1), (8, 1, 20, 2, 1), (9, 1, 19, 2, 1)]
    elif kind == 'excited':
        eyes = [21, 21]
        if stage in (1, 2):
            # Two small arm pumps, planted outer feet, and alternating inner steps.
            rect(p, 9, 3, 2, 1, 0)
            rect(p, 20, 3, 2, 1, 0)
            rect(p, 9, 2, 2, 1, 1)
            rect(p, 20, 2, 2, 1, 1)
            rect(p, 13, 5, 5, 1, 0)
            for x in ((13, 17) if stage == 1 else (14, 16)):
                rect(p, x, 5, 1, 1, 1)
        if stage == 2:
            rect(p, 8, 1, 2, 1, 1)
            rect(p, 21, 1, 2, 1, 1)
            extra = [(3, 0, 22, 3, 0), (12, 0, 22, 3, 0)]
    elif kind == 'sunglasses':
        if stage:
            eyes = []
            extra = [(6, 1, 20, 2, 1), (7, 1, 19, 2, 1), (8, 1, 20, 2, 1)]
        else:
            rect(p, 22, 3, 4, 1, 2)
    cells = []
    for y in range(0, HEIGHT, 2):
        for x in range(0, WIDTH, 2):
            colors = [p[y*WIDTH+x], p[y*WIDTH+x+1], p[(y+1)*WIDTH+x], p[(y+1)*WIDTH+x+1]]
            unique = sorted(set(colors))
            # Shapes are aligned to the grid so a cell never needs a third color.
            assert len(unique) <= 2, (kind, stage, x, y, colors)
            bg, fg = (0, unique[0]) if len(unique) == 1 else unique
            mask = sum(1 << i for i, c in enumerate(colors) if c == fg) if fg else 0
            cells.append([mask, fg, bg])
    for x, glyph in zip(eye_columns, eyes):
        cells[16+x] = [glyph, 2, 1]
    for x, y, glyph, fg, bg in extra:
        cells[y*16+x] = [glyph, fg, bg]
    return bytes(c for cell in cells for c in cell)


# Durations are in 12 fps ticks. Each action enters and leaves the approved rest pose.
# Stable holds make gestures legible instead of redrawing a noisy outline each tick.
def action(kind, beats):
    return [pose()] * 4 + [pose(kind, stage) for stage, ticks in beats for _ in range(ticks)] + [pose()] * 5


clips = {
    'walking': [pose('walking', stage) for stage in (0, 1, 0, 2) for _ in range(3)],
    'turning': [pose('turning', stage) for stage, ticks in [(0, 2), (1, 3), (2, 3), (0, 4)] for _ in range(ticks)],
    'looking': action('looking', [(1, 8), (0, 4), (2, 8), (0, 3), (3, 2)]),
    'waving': action('waving', [(0, 3), (1, 4), (2, 4), (1, 4), (2, 4), (0, 3)]),
    'thinking': action('thinking', [(0, 5), (1, 5), (2, 5), (3, 8), (0, 5)]),
    'coffee': action('coffee', [(0, 6), (1, 5), (2, 12), (1, 5), (0, 6)]),
    'notebook': action('notebook', [(0, 10), (1, 4), (0, 10), (2, 2), (0, 8)]),
    'headphones': action('headphones', [(0, 5), (1, 6), (2, 6), (1, 6), (2, 6), (0, 5)]),
    'watch': action('watch', [(0, 8), (2, 6), (0, 6), (2, 6), (3, 2), (0, 4)]),
    'snooze': action('snooze', [(0, 8), (1, 6), (2, 6), (3, 8), (0, 6), (1, 6), (2, 6), (3, 8)]),
    'sunglasses': action('sunglasses', [(0, 5), (1, 22), (0, 5)]),
    'pirate': action('pirate', [(0, 5), (1, 7), (2, 8), (3, 3), (2, 7), (4, 8), (1, 5), (0, 4)]),
    'excited': action('excited', [(0, 3), (1, 3), (2, 4), (1, 3), (2, 4), (1, 3), (2, 5), (0, 5)]),
    'phone': action('phone', [(0, 5), (1, 8), (2, 3), (1, 7), (2, 3), (3, 3), (1, 6), (0, 5)]),
    'laptop': action('laptop', [(0, 5), (1, 6), (2, 3), (1, 3), (2, 3), (1, 3), (3, 3), (1, 6), (0, 5)]),
    'rose': action('rose', [(0, 8), (1, 9), (0, 5), (2, 8), (0, 5)]),
    'heart': action('heart', [(0, 5), (1, 6), (2, 5), (1, 6), (2, 5), (1, 6), (0, 5)]),
    'meditating': action('meditating', [(0, 5), (1, 8), (2, 10), (1, 8), (2, 10), (1, 8), (0, 5)]),
    'hulahoop': action('hulahoop', [(0, 7), (1, 4), (2, 4), (1, 4), (2, 4), (1, 4), (2, 4), (0, 7)]),
    'desktop': action('desktop', [(0, 6), (1, 3), (0, 3), (1, 3), (0, 3), (2, 9), (0, 5)]),
    'arcade': action('arcade', [(0, 6), (1, 4), (0, 4), (1, 4), (0, 4), (2, 8), (0, 5)]),
    'bubble': action('bubble', [(0, 6), (1, 5), (2, 6), (3, 7), (4, 3), (0, 6)]),
    'spark': action('spark', [(0, 5), (1, 4), (2, 4), (3, 3), (0, 4), (1, 4), (2, 5), (3, 3), (0, 4)]),
    'lightbulb': action('lightbulb', [(0, 9), (1, 4), (2, 10), (1, 4), (2, 6), (0, 5)]),
    'watering': action('watering', [(0, 6), (1, 5), (2, 5), (1, 5), (2, 5), (0, 5), (3, 8), (0, 5)]),
    'berrypicking': action('berrypicking', [(0, 6), (1, 5), (2, 6), (3, 8), (2, 5), (0, 5)]),
    'blooming': action('blooming', [(0, 6), (1, 7), (2, 7), (3, 9), (2, 6)]),
    'fishing': action('fishing', [(0, 10), (1, 5), (0, 6), (1, 4), (2, 6), (3, 9), (2, 5), (0, 5)]),
    'dancing': action('dancing', [(1, 5), (2, 5), (1, 5), (2, 5), (1, 4), (2, 4)]),
    'dancinghappy': action('dancinghappy', [(1, 4), (2, 4), (1, 4), (2, 4), (1, 4), (2, 4)]),
    'swaying': action('swaying', [(1, 6), (0, 2), (2, 6), (0, 2), (1, 6), (0, 2), (2, 6)]),
}
out = bytearray(b'CLWD3') + bytes([16, 3, len(PALETTE), len(clips)])
for color in PALETTE:
    out += bytes(color)
for name, frames in clips.items():
    out += bytes([len(name)]) + name.encode() + struct.pack('<H', len(frames)) + b''.join(frames)
(OUTPUT/'frames.bin').write_bytes(out)
sources = {s['name']: s for s in json.loads((OUTPUT/'sources.json').read_text())}
for name, frames in clips.items():
    sources[name]['curated_frames'] = len(frames)
    sources[name]['adaptation'] = ('Original costume and grid animation designed for the user.' if name == 'pirate'
                                   else 'Hand-timed grid poses; original clip is a gesture reference, not a frame-for-frame conversion.')
(OUTPUT/'sources.json').write_text(json.dumps([sources[n] for n in clips], ensure_ascii=False, indent=2)+'\n')
print(f'{len(clips)} clips, {sum(map(len, clips.values()))} frames, {len(out)} bytes')
