"""The head on the V knot: index fingers curled forward over the front, one on the other (the
snout); thumbs forward underneath, pads together (the lower jaw), shut under the snout."""
import json, sys
from hk3 import *
from head import report
from vgrip import row

S = ('right', 'left')
IDX = {'mcp_abd': 10, 'mcp_flex': 50, 'pip_flex': 80, 'dip_flex': 45}
THB = {'cmc_flex': -10, 'cmc_abd': 25, 'cmc_rot': 10, 'mcp_flex': 0, 'ip_flex': 5}


def head(h, tag):
    h = copy.deepcopy(h)
    for s in S:
        h['me.' + s]['index'] = dict(IDX)
        h['me.' + s]['thumb'] = dict(THB)
    idx = ['me.right.index.*', 'me.left.index.*']
    thb = ['me.right.thumb.*', 'me.left.thumb.*']
    goals = [{'touch': ['me.right.index.pad', 'me.left.index.nail'], 'vary': idx},
             {'touch': ['me.right.thumb.pad', 'me.left.thumb.pad'], 'vary': thb},
             {'touch': ['me.left.index.tip', 'me.left.thumb.distal'], 'vary': idx + thb}]
    code, text, s = solve('head_' + tag, pose(h, goals))
    report('head ' + tag, code, text, s['hands'])
    print('\n'.join(l for l in text.splitlines() if 'MISS' in l))
    return s['hands']


if __name__ == '__main__':
    src = sys.argv[1]
    h = head(json.load(open(os.path.join(OUT, src + '.json'))), src)
    for s in S:
        print(s, {k: round(v) for k, v in h['me.' + s]['index'].items()}, {k: round(v) for k, v in h['me.' + s]['thumb'].items()})
    dump(os.path.join(OUT, 'head_%s_hands.json' % src), h)
    row('head_' + src, h)
