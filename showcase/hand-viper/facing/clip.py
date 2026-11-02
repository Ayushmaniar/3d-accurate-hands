"""The hand viper facing the camera, 8 s: the v3 clip's build (press, cross, stack, curl, the
thumbs swinging under) into the face of head.py (face.json), the camera pushing in to look the
snake in the eye. The mouth works as in the tutorial: the thumbs resting on the index fingers
shut it, the thumbs dropping away open it; the index fingers stay. It hisses once, rears back
and strikes at the camera with the mouth wide, then snaps shut."""
import os, sys

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, os.path.join(HERE, '..', 'v3'))
import clip as v3                       # noqa: E402  (the v3 clip's helpers and build poses)
from hk3 import H, run, dump, LOOK      # noqa: E402
import json                             # noqa: E402

S, DIG = v3.S, v3.DIG
FACE = json.load(open(os.path.join(HERE, 'face.json')))['hands']
# In front of the snout (the head points +x), a little above it.
FACE_CAM = {'position': [0.29, 0.17, 0.0], 'target': [0.07, 0.09, 0.0], 'fov': 30}


def mouth(shut, drop):
    """The mouth `drop` open (0 shut, 1 wide): the thumbs fall away from the index fingers."""
    out = {s: {d: dict(shut[s][d]) for d in DIG} for s in S}
    for s in S:
        t = out[s]['thumb']
        t['mcp_flex'] = max(-15, t['mcp_flex'] - 30 * drop)
        t['ip_flex'] = max(-20, t['ip_flex'] - 35 * drop)
        t['cmc_abd'] = max(-10, t['cmc_abd'] - 14 * drop)
    return out


def build():
    flat = {s: dict({d: dict(v3.STRAIGHT) for d in DIG[1:]}, thumb=dict(v3.THUMB_UP)) for s in S}
    cross = {s: dict(flat[s], **v3.joints(v3.P0, s, ('ring', 'pinky'))) for s in S}
    stack = {s: dict(cross[s], **v3.joints(v3.P0, s, ('middle',))) for s in S}
    shut = {s: {d: dict(FACE[s][d]) for d in DIG} for s in S}
    hiss, wide = mouth(shut, 0.55), mouth(shut, 1.0)
    # The index fingers curl in first, the thumbs still up beside them; then the thumbs swing
    # under, flat in their own palms' planes so they pass each other.
    curl = {s: dict(shut[s], thumb=dict(v3.THUMB_UP)) for s in S}
    curl = v3.tidy(curl, 'face_curl', base=v3.PV)
    swing = {s: dict(curl[s], thumb={'cmc_flex': (v3.THUMB_UP['cmc_flex'] + wide[s]['thumb']['cmc_flex']) / 2,
                                     'cmc_abd': -10, 'cmc_rot': 0, 'mcp_flex': 0, 'ip_flex': 0}) for s in S}
    key = v3.key
    keys = [
        key(0.0, flat, gap=0.07),
        key(1.0, flat, {'move': 0.45}, gap=0.027),
        key(2.0, cross, {'move': 0.55, 'cascade': 0.06}),
        key(3.0, stack, {'move': 0.5}),
        key(3.9, curl, {'move': 0.7}, base=v3.PV),
        {'t': 4.0, 'camera': FACE_CAM, 'move': 1.6, 'ease': 'smooth'},
        key(4.5, swing, {'pass': True}, base=v3.PV),
        key(5.0, wide, {'move': 0.45}, base=v3.PV),
        key(5.5, shut, {'move': 0.25, 'ease': 'snappy'}, base=v3.PV),
        # A hiss: the jaw drops and closes.
        key(5.9, hiss, {'move': 0.18}, base=v3.PV),
        key(6.2, shut, {'move': 0.16, 'ease': 'snappy'}, base=v3.PV),
        # Rears back, strikes at the camera with the mouth wide, snaps shut.
        key(6.82, shut, {'move': 0.4}, base=v3.PV, shift=(-0.03, 0.0, 0)),
        key(7.0, wide, {'move': 0.16, 'ease': 'snappy'}, base=v3.PV, shift=(0.03, 0.025, 0)),
        key(7.32, shut, {'move': 0.14, 'ease': 'snappy'}, base=v3.PV, shift=(0.02, 0.012, 0)),
        key(7.9, shut, {'move': 0.5}, base=v3.PV),
    ]
    return {'version': 1, 'fps': 30, 'duration': 8.0, 'size': [1920, 1080], 'idle': {'amount': 0.12},
            'collide': False, 'camera': v3.WIDE_CAM, 'look': LOOK, 'keys': keys}


if __name__ == '__main__':
    clip = build()
    path = os.path.join(HERE, 'clip.json')
    dump(path, clip)
    print(run([H, 'check', path])[1][-2500:])
