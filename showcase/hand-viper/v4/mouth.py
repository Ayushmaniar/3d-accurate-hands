"""Shut and open mouth on the palms head (a 15, c -55, wrists 50 mm out)."""
import copy
from palms import *

A, C, HALF = 15, -55, 0.056
SHUT = [{'touch': ['%s.index.tip' % s, '%s.thumb.tip' % s], 'vary': ['%s.index.*' % s, '%s.thumb.*' % s]} for s in ('right', 'left')]


def head_shut():
    # Middle, ring and little fingers close round each other the way an interlaced grip does
    # (closing step by step until they meet the other hand), the hands staying where placed.
    grip = {'grip': ['right', 'left'], 'style': 'interlaced', 'vary': CLEAR['vary']}
    code, text, sol = solve('m_shut', {'version': 1, 'hands': palms(A, C, HALF), 'goals': [grip, ARCH] + SHUT})
    print('shut', [l.strip()[:90] for l in text.splitlines() if 'MISS' in l or 'penetr' in l])
    return sol['hands']


def head_open(shut, amount=1.0):
    out = copy.deepcopy(shut)
    for s in out:
        t, ix = out[s]['thumb'], out[s]['index']
        # The thumbs swing down and out, away from the index tips (not back into the knot).
        t['cmc_flex'] = round(max(-30, t['cmc_flex'] - 30 * amount), 1)
        t['mcp_flex'] = round(max(-15, t['mcp_flex'] - 10 * amount), 1)
        t['ip_flex'] = round(max(-20, t['ip_flex'] - 10 * amount), 1)
        ix['mcp_flex'] = round(ix['mcp_flex'] - 15 * amount, 1)
        ix['pip_flex'] = round(ix['pip_flex'] - 15 * amount, 1)
    return out


if __name__ == '__main__':
    shut = head_shut()
    dump(os.path.join(OUT, 'm_shut_hands.json'), shut)
    still('m_shut', {'version': 1, 'hands': shut}, cam=FRONT, size=380)
    still('m_open', {'version': 1, 'hands': head_open(shut)}, cam=FRONT, size=380)
