"""The launch film's cut: HANDKIT with every letter landing on a beat (one a second) and held still
for most of it, then I-LOVE-YOU. Reads spell_slow.json (`handkit spell HANDKIT --rate 1 --hold 0.75`)
and re-times its keys; writes film.json."""
import json

LAND = [0.5 + i for i in range(7)]   # H A N D K I T arrive on the beat
MOVE = 0.30                          # each change of letter takes this long; the rest of the second is held
ILY = 7.533                          # I-LOVE-YOU arrives with the film's "I love you." caption
DUR = 8.8

c = json.load(open('spell_slow.json'))
keys = c['keys']
rest, letters = keys[0], []
for k in keys[1:-1]:                 # group each letter's keys: an optional pass-through, then its arrival
    if letters and letters[-1][-1].get('pass'):
        letters[-1].append(k)
    else:
        letters.append([k])
assert len(letters) == 7, len(letters)

out = [rest]
for t, group in zip(LAND, letters):
    arrive = group[-1]
    mv = 0.5 if t == LAND[0] else MOVE   # the first letter rises from rest
    if len(group) == 2:
        way = group[0]
        way['t'], way['move'] = round(t - mv * 0.45, 3), round(mv * 0.55, 3)
        out.append(way)
        arrive['move'] = round(mv * 0.45, 3)
    else:
        arrive['move'] = mv
    arrive['t'] = t
    out.append(arrive)
out.append({"t": ILY, "move": 0.35, "hands": {"right": {"from": "asl_ily",
    "place": {"wrist": [0.0, -0.02, 0.02], "palm_facing": [0, 0, 1], "fingers_toward": [0, 1, 0]}}}})

c['keys'] = out
c['duration'] = DUR
c['name'] = 'sign_handkit_film'
c['description'] = '"HANDKIT" fingerspelled in ASL on the beat (a letter a second), then I-LOVE-YOU.'
c['camera'] = {"position": [0.0, 0.035, 0.66], "target": [0.0, 0.035, 0.0], "fov": 26}
c['look'] = {"style": "realistic", "skin": "brown", "motion_blur": 4}
json.dump(c, open('film.json', 'w'), indent=1)
