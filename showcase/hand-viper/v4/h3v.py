from head3 import *
import sys
variants = {
    'v1': dict(curl=(70, 100, 60), index=(75, 90, 30), up=0.8),
    'v2': dict(curl=(80, 105, 65), index=(80, 95, 35), up=0.6, half=0.05),
    'v3': dict(curl=(85, 105, 70), index=(85, 95, 40), up=0.45, half=0.055),
}
for name, kw in variants.items():
    h = hands3(**kw)
    tilt = {'v1': 0.4, 'v2': 0.7, 'v3': 1.0}[name]
    for s in h:
        h[s]['place']['palm_facing'] = [0, tilt, 1]
    still('h3' + name, {'version': 1, 'hands': h, 'camera': FRONT}, size=420)
