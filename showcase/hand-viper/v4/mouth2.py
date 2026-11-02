"""The shut head with the folded fingers clear of each other: the two hands offset front to back
by `dz` (so their fingers alternate instead of meeting head on), the interlaced grip closing the
folded fingers, then a second pass that pushes them out of each other while the snout and the
jaw stay shut."""
import sys
from mouth import *


def shut_clear(dz, tag):
    h = palms(A, C, HALF)
    for side, s in (('right', 1), ('left', -1)):
        h[side]['place']['wrist'][2] = round(h[side]['place']['wrist'][2] + s * dz, 4)
    grip = {'grip': ['right', 'left'], 'style': 'interlaced', 'vary': CLEAR['vary']}
    code, text, first = solve(tag + 'a', {'version': 1, 'hands': h, 'goals': [grip, ARCH] + SHUT})
    code, text, sol = solve(tag, {'version': 1, 'hands': first['hands'], 'goals': [ARCH] + SHUT + [CLEAR]})
    q = run([H, 'query', os.path.join(OUT, tag + '_sol.json')])[1]
    pen = [l.strip() for l in q.split('penetrations')[1].split('contacts')[0].splitlines() if 'mm' in l]
    print(tag, [l.strip()[:70] for l in text.splitlines() if 'MISS' in l], len(pen), pen[:3])
    return sol['hands'], len(pen)


if __name__ == '__main__':
    for dz in (0.0, 0.006, 0.011, -0.006):
        shut_clear(dz, 'mc_%d' % round(dz * 1000))
