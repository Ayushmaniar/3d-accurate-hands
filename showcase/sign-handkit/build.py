import json
c = json.load(open('spell.json'))
c['keys'] = c['keys'][:-1]
c['keys'].append({"t": 5.85, "move": 0.4, "hands": {"right": {"from": "asl_ily",
  "place": {"wrist": [0.0, -0.02, 0.02], "palm_facing": [0, 0, 1], "fingers_toward": [0, 1, 0]}}}})
c['duration'] = 7.5
c['name'] = 'sign_handkit'
c['description'] = '"HANDKIT" fingerspelled in ASL, then I-LOVE-YOU.'
json.dump(c, open('clip.json', 'w'), indent=1)
c = json.load(open('clip.json'))
c['camera'] = {"position": [0.0, 0.035, 0.66], "target": [0.0, 0.035, 0.0], "fov": 26}
json.dump(c, open('clip.json', 'w'), indent=1)
