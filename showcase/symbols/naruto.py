"""Another Naruto hand seal, built with the same tools as the tiger:
  snake (mi): palms together, all fingers interlocked over the other hand's backs, the thumbs
              pressed side by side (the interlaced grip, every finger closing)"""
import copy
from sym import *
import tiger

CAM = {'position': [0.12, 0.16, 0.34], 'target': [0.0, 0.09, 0.0], 'fov': 30}
UP = {'mcp_abd': 0, 'mcp_flex': 0, 'pip_flex': 0, 'dip_flex': 0}
CURL = {'mcp_abd': 0, 'mcp_flex': 75, 'pip_flex': 95, 'dip_flex': 50}


def report(name, text, sol):
    print(name, [l.strip()[:90] for l in text.splitlines() if 'MISS' in l],
          query(name, sol).split('penetrations')[1].split('contacts')[0].strip()[:80])


# Snake: the tiger's hands, every finger closing in the interlaced grip, thumbs crossed.
h = tiger.hands(gap=0.06, dz=0.008)
for s in h:
    for d in ('index', 'middle'):
        h[s][d] = {'mcp_abd': 0, 'mcp_flex': 30, 'pip_flex': 40, 'dip_flex': 20}
grip = {'grip': ['right', 'left'], 'style': 'interlaced',
        'vary': ['%s.%s.*' % (s, d) for s in ('right', 'left') for d in ('index', 'middle', 'ring', 'pinky')]}
thumbs = {'touch': ['right.thumb.side_palmar', 'left.thumb.side_palmar'], 'vary': ['right.thumb.*', 'left.thumb.*', 'left.place.position']}
code, text, snake = solve('seal_snake', {'version': 1, 'hands': h, 'camera': CAM, 'goals': [grip, thumbs]})
report('snake', text, snake)
still('seal_snake', snake, size=420)
