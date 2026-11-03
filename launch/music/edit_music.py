"""Cut the generated track (track48.wav, 120 BPM) down to the launch film's length on
downbeats, and write the edit map (source -> film times) the film is timed against.

    python launch/music/edit_music.py
"""
import json
from pathlib import Path

import numpy as np
import soundfile as sf
from scipy.signal import butter, sosfiltfilt

HERE = Path(__file__).parent
# (name, source start, source end), cut on downbeats measured by `earshot timeline`.
PIECES = [
    ("intro", 0.0, 11.12),        # felt piano: the black-and-white opening
    ("build_a", 11.12, 19.15),    # pulse joins
    ("build_b", 35.14, 43.14),    # the build's last bars, rising into the drop
    ("drop", 43.14, 59.14),       # punchy drums: the modern hand
    # The track's own breakdown (59.14 to 67.9) is a thin, high whine over nothing, which reads as
    # horror under the sign act: the drop's groove stays instead, softened by soften().
    ("soft_groove", 47.14, 55.92),
    ("hit", 67.90, 68.91),        # the big hit, on the same film time as before
    ("ride", 68.91, 84.91),       # tech-house groove with a lead
    ("end", 84.91, 89.91),        # one bar, then a 3 s fade under the final hit
]
XFADE = 0.02


def soften(seg, sr):
    """The groove under the sign act: warm (highs rolled off), about 6 dB down, easing in from the
    fill and dipping before the hit so the hit still lands."""
    t = np.arange(len(seg)) / sr
    warm = sosfiltfilt(butter(2, 4000, fs=sr, output="sos"), seg, axis=0)
    def ramp(a, b):
        return np.clip((t - a) / (b - a), 0, 1) ** 2 * (3 - 2 * np.clip((t - a) / (b - a), 0, 1))
    level = 0.5 * (0.35 + 0.65 * ramp(0, 0.6)) * (1 - 0.8 * ramp(len(seg) / sr - 1.9, len(seg) / sr - 0.15))
    return warm * level[:, None]


def main():
    audio, sr = sf.read(HERE / "track48.wav", always_2d=True)
    out = np.zeros((0, audio.shape[1]))
    edit = []
    n = int(XFADE * sr)
    for name, a, b in PIECES:
        seg = audio[int(a * sr):int(b * sr)].copy()
        if name == "soft_groove":
            seg = soften(seg, sr)
        start = len(out) / sr
        if len(out) >= n:
            ramp = np.sqrt(np.linspace(0, 1, n))[:, None]
            out[-n:] = out[-n:] * ramp[::-1] + seg[:n] * ramp
            seg = seg[n:]
        out = np.concatenate([out, seg])
        edit.append({"name": name, "src": [a, b], "film": [round(start, 3), round(len(out) / sr, 3)]})
    fade = int(3.0 * sr)
    out[-fade:] *= np.linspace(1, 0, fade)[:, None] ** 2
    out *= 0.89 / max(np.abs(out).max(), 1e-9)
    sf.write(HERE / "launch_music.wav", out, sr, subtype="PCM_24")
    beats = json.load(open(HERE / "timeline.json"))["music"]["beats"]
    film_beats = []
    for e in edit:
        (a, b), (fa, _) = e["src"], e["film"]
        film_beats += [round(fa + t - a, 3) for t in beats if a <= t < b]
    json.dump({"bpm": 120, "duration": round(len(out) / sr, 3), "pieces": edit,
               "beats": film_beats, "notes": {"dip_before_hit": [50.0, 51.8], "big_hit": 51.87, "final_hit": 70.92}},
              open(HERE / "edit.json", "w"), indent=1)
    print(f"{len(out) / sr:.2f} s")
    for e in edit:
        print(e)


if __name__ == "__main__":
    main()
