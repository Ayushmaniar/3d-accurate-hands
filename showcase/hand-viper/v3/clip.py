"""The hand viper clip, 8 s: a step lands each second (1..6 s), the strike at 7 s.

Placements: the tight upright clasp (tight.py, z 32 mm; the left hand where Handkit's
interlaced grip puts it), opened into a 22 degree V about the pinky edge for the head
(vhead.py), all leaned forward by LEAN. Joint values come from the solved poses:
  1 press    palms flat together, fingers up
  2 cross    ring and pinky fingers hook over the other hand's back
  3 stack    middle fingers hook over them
  4 curl     the palms open into the V; index fingers curl forward over its front, stacked
  5 jaw      thumbs swing forward underneath, the mouth a little open
  6 squeeze  the index tips close onto the thumbs
  7 strike   rear back, lunge toward the head with the jaw wide, snap shut, settle.
The camera starts on the whole knot and pushes in on the head from 3.4 s."""
import json, math, sys
from hk3 import *
from manual import lean
import vopen

LEAN, V = 25, 8
FOREARM = {'right': [0.05, -1.0, -0.2], 'left': [0.05, -1.0, 0.2]}   # in the upright frame
WIDE_CAM = {'position': [0.29, 0.15, 0.33], 'target': [0.015, 0.07, 0.0], 'fov': 30}
HEAD_CAM = {'position': [0.27, 0.085, 0.30], 'target': [0.06, 0.07, 0.0], 'fov': 30}
S = ('right', 'left')
SIDE = {'right': -1, 'left': 1}
DIG = ('thumb', 'index', 'middle', 'ring', 'pinky')

P0 = json.load(open(os.path.join(OUT, 'upright_hands_z32.json')))
PV = vopen.turn_hands(P0, V)
HEAD = json.load(open(os.path.join(OUT, 'vh%dt-10_hands.json' % V)))
CLOSED = {'me.' + s: h for s, h in json.load(open(os.path.join(HERE, 'closed.json')))['hands'].items()}
STRAIGHT = {'mcp_abd': 0, 'mcp_roll': 0, 'mcp_flex': 4, 'pip_flex': 6, 'dip_flex': 3}
THUMB_UP = {'cmc_flex': 8, 'cmc_abd': -8, 'cmc_rot': 0, 'mcp_flex': 5, 'ip_flex': 5}


def joints(src, side, digits):
    return {d: dict(src['me.' + side][d]) for d in digits}


def placed(side, base=P0, gap=None, shift=(0, 0, 0)):
    """One hand's placement in the upright frame: `gap` sets the wrists' z apart (palms flat
    together at 0.027), `shift` moves the knot (x toward the head, y up)."""
    pl = copy.deepcopy(base['me.' + side]['place'])
    if gap is not None:
        pl['wrist'][2] = SIDE[side] * gap / 2
    pl['wrist'] = [pl['wrist'][i] + shift[i] for i in range(3)]
    pl['forearm_toward'] = FOREARM[side]
    return pl


def hands_at(digits, **where):
    return {'me.' + s: dict(copy.deepcopy(digits[s]), place=placed(s, **where)) for s in S}


def key(t, digits, timing=None, per_hand=None, **where):
    k = {'t': t, 'hands': lean(hands_at(digits, **where), LEAN)}
    k.update(timing or {})
    for s, own in (per_hand or {}).items():
        k['hands']['me.' + s].update(own)
    return k


def clear_thumbs(digits, **where):
    """Thumbs up pad to pad, as in prayer; the solver keeps them out of the knot."""
    goal = {'touch': ['me.right.thumb.pad', 'me.left.thumb.pad'], 'vary': ['me.right.thumb.*', 'me.left.thumb.*']}
    code, text, sol = solve('thumbs', pose(hands_at(digits, **where), [goal]))
    print('thumbs', text.splitlines()[0], [l.strip()[:90] for l in text.splitlines() if 'MISS' in l])
    return {s: sol['hands']['me.' + s]['thumb'] for s in S}


def tidy(digits, name, only=DIG, **where):
    """The solver pushes every finger (of `only`) out of the other hand and its own (a vary-only
    goal)."""
    goal = {'vary': ['me.%s.%s.*' % (s, d) for s in S for d in only]}
    code, text, sol = solve('tidy_' + name, pose(hands_at(digits, **where), [goal]))
    print('tidy', name, text.splitlines()[0])
    return {s: joints(sol['hands'], s, DIG) for s in S}


def with_thumbs(digits, thumbs):
    return {s: dict(digits[s], thumb=dict(thumbs[s])) for s in S}


def nudged(digits, index=None, thumb=None):
    out = copy.deepcopy(digits)
    for s in S:
        for k, d in (index or {}).items():
            out[s]['index'][k] = out[s]['index'][k] + d
        for k, d in (thumb or {}).items():
            out[s]['thumb'][k] = out[s]['thumb'][k] + d
    return out


def build():
    flat = {s: dict({d: dict(STRAIGHT) for d in DIG[1:]}, thumb=dict(THUMB_UP)) for s in S}
    cross = {s: dict(flat[s], **joints(P0, s, ('ring', 'pinky'))) for s in S}
    stack = {s: dict(cross[s], **joints(P0, s, ('middle',))) for s in S}
    shut = {s: joints(HEAD, s, DIG) for s in S}
    # The mouth shut: each index tip resting on its thumb's last segment (closed.py solves it).
    for s in S:
        shut[s].update(joints(CLOSED, s, ('index', 'thumb')))
    curl = {s: dict(shut[s], thumb=dict(THUMB_UP)) for s in S}
    wide = nudged(shut, index={'mcp_flex': -18, 'pip_flex': -30}, thumb={'mcp_flex': -8, 'ip_flex': -10})
    for s in S:
        wide[s]['thumb']['cmc_flex'] = max(-30, wide[s]['thumb']['cmc_flex'] - 8)
    curl, wide = [tidy(d, n, base=PV) for d, n in ((curl, 'curl'), (wide, 'wide'))]
    shut = tidy(shut, 'shut', only=('middle', 'ring', 'pinky'), base=PV)
    jaw = {s: {d: {k: (shut[s][d][k] * 2 + wide[s][d][k]) / 3 for k in shut[s][d]} for d in DIG} for s in S}
    # On the way under, the thumbs stay flat in their own palms' planes, so they pass each other.
    swing = {s: dict(curl[s], thumb={'cmc_flex': (THUMB_UP['cmc_flex'] + jaw[s]['thumb']['cmc_flex']) / 2,
                                     'cmc_abd': -10, 'cmc_rot': 0, 'mcp_flex': 0, 'ip_flex': 0}) for s in S}
    # The index fingers curl only as far as the open jaw: the squeeze brings them down onto the thumbs.
    curl = {s: dict(curl[s], index=dict(jaw[s]['index']), thumb=dict(swing[s]['thumb'])) for s in S}
    swing = {s: dict(swing[s], index=dict(jaw[s]['index'])) for s in S}
    keys = [
        key(0.0, flat, gap=0.07),
        key(1.0, flat, {'move': 0.45}, gap=0.027),
        key(2.0, cross, {'move': 0.55, 'cascade': 0.06}),
        key(3.0, stack, {'move': 0.5}),
        {'t': 3.4, 'camera': WIDE_CAM},
        # The right index lifts over the top of the left one before curling down onto it.
        {'t': 3.78, 'pass': True, 'move': 0.3, 'hands': {'me.right': {'index': dict(curl['right']['index'], mcp_flex=-20, pip_flex=45, dip_flex=20)}}},
        key(4.0, curl, {'cascade': 0.08}, {'left': {'move': 0.7}, 'right': {'move': 0.22}}, base=PV),
        key(4.6, swing, {'pass': True}, base=PV),
        {'t': 4.9, 'camera': HEAD_CAM, 'move': 1.5, 'ease': 'smooth'},
        key(5.0, jaw, {'move': 0.5}, base=PV),
        key(6.0, shut, {'move': 0.35, 'ease': 'snappy'}, base=PV),
        key(6.82, shut, {'move': 0.4}, base=PV, shift=(-0.025, 0.008, 0)),
        key(7.0, wide, {'move': 0.16, 'ease': 'snappy'}, base=PV, shift=(0.04, 0.025, 0)),
        key(7.32, shut, {'move': 0.14, 'ease': 'snappy'}, base=PV, shift=(0.03, 0.004, 0)),
        key(7.9, shut, {'move': 0.5}, base=PV),
    ]
    return {'version': 1, 'fps': 30, 'duration': 8.0, 'size': [1920, 1080], 'idle': {'amount': 0.12},
            'collide': False, 'camera': WIDE_CAM, 'look': LOOK, 'keys': keys}


if __name__ == '__main__':
    clip = build()
    path = os.path.join(HERE, 'clip.json')
    dump(path, clip)
    print(run([H, 'check', path])[1][-3500:])
