"""Third pass, after the tutorial's 42 s frame: palms toward the viewer, the fingers pointing up
and in toward each other and curled toward the viewer, so each thumb lies at the bottom (the jaw),
the index just above it across the front (the snout) and the little finger on top. The right
hand is on the viewer's left (the person faces the viewer)."""
from head import *


def hands3(half=0.045, up=1.0, z=0.008, curl=(55, 85, 40), index=(70, 80, 25), thumb=None):
    h = {}
    for side, s in (('right', -1), ('left', 1)):
        h[side] = {'place': {'wrist': [s * half, 0.0, -s * z], 'palm_facing': [0, -0.2, 1], 'fingers_toward': [-s * 0.75, up, 0],
                             'forearm_toward': [s * 0.35, -1, -0.5]},
                   'index': {'mcp_abd': 0, 'mcp_flex': index[0], 'pip_flex': index[1], 'dip_flex': index[2]},
                   'thumb': thumb or {'cmc_flex': 5, 'cmc_abd': 30, 'cmc_rot': 30, 'mcp_flex': 10, 'ip_flex': 20}}
        for d in ('middle', 'ring', 'pinky'):
            h[side][d] = {'mcp_abd': 0, 'mcp_flex': curl[0], 'pip_flex': curl[1], 'dip_flex': curl[2]}
    return h


if __name__ == '__main__':
    pose = {'version': 1, 'hands': hands3(), 'camera': FRONT}
    print(run([H, 'query', os.path.join(OUT, 'h3.json')])[1][:0])
    dump(os.path.join(OUT, 'h3.json'), pose)
    q = run([H, 'query', os.path.join(OUT, 'h3.json')])[1]
    print(q.split('contacts')[0][-600:])
    still('h3', pose)
