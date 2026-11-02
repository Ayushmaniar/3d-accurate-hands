"""The viper's mouth shut: from the hand_viper gesture, each index tip comes down onto its own
thumb's last segment (the lower jaw), the thumbs' bases staying where they point forward.
Writes closed.json, which clip.py takes the index and thumb values from."""
import json
from hk3 import *

g = json.load(open(os.path.join(ROOT, 'crates', 'handkit-core', 'gestures', 'hand_viper.json')))
VARY = ['right.index.*', 'left.index.*'] + ['%s.thumb.%s' % (s, k) for s in ('right', 'left') for k in ('mcp_flex', 'ip_flex')]
pose = {'version': 1, 'hands': g['hands'], 'camera': g['camera'],
        'goals': [{'touch': ['%s.index.tip' % s, '%s.thumb.distal' % s], 'vary': VARY} for s in ('left', 'right')]}
code, text, sol = solve('closed', pose)
print(text.splitlines()[0], [l.strip() for l in text.splitlines() if 'touch' in l or 'penetr' in l])
assert code == 0, text
sol.pop('goals', None)
dump(os.path.join(HERE, 'closed.json'), sol)
