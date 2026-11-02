"""Mix launch_music.wav + earshot SFX placed on work/cues.json (written by build.py from the picture's constants).

Run with the earshot interpreter:  $EARSHOT_PYTHON mix.py
Writes assets/audio/mix.wav (48 kHz stereo, -14 LUFS integrated, true peak <= -1.2 dBTP) and work/stems/sfx.wav.
"""
import json, os, subprocess, sys, hashlib
import numpy as np, soundfile as sf
from scipy.signal import resample_poly, butter, sosfilt
from scipy.ndimage import minimum_filter1d

ROOT = os.path.dirname(os.path.abspath(__file__))
HK = os.path.dirname(os.path.dirname(ROOT))
MUSIC = os.path.join(HK, "launch", "music", "launch_music.wav")
CACHE = os.path.join(ROOT, "work", "sfx")
os.makedirs(CACHE, exist_ok=True)
os.makedirs(os.path.join(ROOT, "work", "stems"), exist_ok=True)
PY = os.environ.get("EARSHOT_PYTHON", sys.executable)
SR = 48000
FRAME = 1 / 30


def take(sid, seed=None, dur=None):
    key = hashlib.md5(f"{sid}|{seed}|{dur}".encode()).hexdigest()[:10]
    wav = os.path.join(CACHE, sid.replace("/", "_") + f"_{key}.wav")
    meta = wav + ".json"
    if not os.path.exists(meta):
        cmd = [PY, "-m", "earshot", "sfx", "render", sid, "-o", wav, "--json"]
        if seed is not None:
            cmd += ["--seed", str(seed)]
        if dur is not None:
            cmd += ["--dur", f"{dur:.3f}"]
        out = subprocess.run(cmd, capture_output=True, text=True, cwd=CACHE)
        if out.returncode:
            raise SystemExit(f"earshot failed for {sid}: {out.stderr[-500:]}")
        js = out.stdout[out.stdout.index("{"):]
        with open(meta, "w") as f:
            f.write(js)
    m = json.load(open(meta))["meta"]
    x, sr = sf.read(wav, always_2d=True)
    assert sr == SR
    if x.shape[1] == 1:
        x = np.repeat(x, 2, 1)
    return x, m


def db(g):
    return 10 ** (g / 20)


cues = json.load(open(os.path.join(ROOT, "work", "cues.json")))
DUR = cues["duration"]
N = int(round(DUR * SR))
music, sr = sf.read(MUSIC, always_2d=True)
assert sr == SR
music = music[:N]
if len(music) < N:
    music = np.pad(music, ((0, N - len(music)), (0, 0)))
sfx = np.zeros((N, 2))


def place(x, start_s, gain_db, pan=0.0):
    a = int(round(start_s * SR))
    b0 = max(0, -a)
    a = max(0, a)
    n = min(len(x) - b0, N - a)
    if n <= 0:
        return
    l, r = np.cos((pan + 1) * np.pi / 4), np.sin((pan + 1) * np.pi / 4)
    y = x[b0:b0 + n] * db(gain_db)
    sfx[a:a + n, 0] += y[:, 0] * l * np.sqrt(2)
    sfx[a:a + n, 1] += y[:, 1] * r * np.sqrt(2)


def projector(t0, t1, gain_db):
    """film-projector clatter in code: earshot ticks on the 24 fps claw cycle (pull-down + shutter), motor hum"""
    rng = np.random.default_rng(24)
    ticks = [take(s, seed=k)[0] for k, s in enumerate(["pro_tap/knock", "knock/plastic", "hover_tick/air", "pro_tap/glass"] * 2)]
    n0, n1 = int(t0 * SR), int(t1 * SR)
    out = np.zeros((n1 - n0, 2))
    t, i = 0.0, 0
    while t < t1 - t0:
        for off, amp in ((0.0, 1.0), (0.011, 0.55)):          # claw + shutter blade, 11 ms apart
            k = ticks[(i * 2 + (off > 0)) % len(ticks)]
            a = int((t + off + rng.normal(0, 0.0012)) * SR)
            g = amp * rng.uniform(0.7, 1.0)
            n = min(len(k), len(out) - a)
            if a >= 0 and n > 0:
                out[a:a + n] += k[:n] * g
        t += 1 / 24
        i += 1
    noise = rng.standard_normal((len(out), 2)) * 0.05
    noise = sosfilt(butter(2, [180, 2400], "bandpass", fs=SR, output="sos"), noise, axis=0)
    tt = np.arange(len(out)) / SR
    noise *= (0.6 + 0.4 * np.abs(np.sin(np.pi * 24 * tt)))[:, None]
    out = out * 0.5 + noise
    env = np.ones(len(out))
    ramp = int(0.5 * SR)
    env[:ramp] = np.linspace(0, 1, ramp) ** 2
    env[-int(0.02 * SR):] = np.linspace(1, 0, int(0.02 * SR))
    out *= env[:, None]
    out /= np.abs(out).max() + 1e-9
    sfx[n0:n1] += out * db(gain_db)


report = []
for c in cues["cues"]:
    t, sid, g, role = c["t"], c["id"], c["gain"], c["role"]
    if role == "projector":
        projector(t, c["until"], g)
        report.append((t, role, "code: earshot ticks @24fps", g))
        continue
    x, m = take(sid, seed=c.get("seed"), dur=c.get("dur"))
    if role in ("whoosh", "swell", "riser", "drop"):
        start = t - m["peak_s"]                      # apex on the cut
        if role == "drop":
            start = t - m["onset_s"]
    else:
        start = t + FRAME - min(m["peak_s"], m["onset_s"] + 0.03)   # transient one frame after the picture
    pan = 0.0
    if role == "key":
        pan = ((hash(sid) % 7) - 3) * 0.05
    place(x, start, g, pan)
    report.append((round(t, 3), role, sid, g))

# the breakdown silence stays (almost) empty: keep SFX out of 50.26-51.80 except the one soft tap
mix_bed = music.copy()
music_gain = 1.0
full = mix_bed * music_gain + sfx


def lufs(x):
    import pyloudnorm as pyln
    return pyln.Meter(SR).integrated_loudness(x)


def limiter(x, ceiling_db=-2.4, look_ms=3.0, rel_ms=90.0):
    ceil = db(ceiling_db)
    up = resample_poly(x, 4, 1, axis=0)
    pk = np.abs(up).max(1).reshape(-1, 4).max(1)[: len(x)]
    if len(pk) < len(x):
        pk = np.pad(pk, (0, len(x) - len(pk)))
    need = np.minimum(1.0, ceil / np.maximum(pk, 1e-9))
    la = int(look_ms / 1000 * SR)
    need = minimum_filter1d(need, size=2 * la + 1)
    # release smoothing (attack instant thanks to lookahead min-filter)
    a = np.exp(-1 / (rel_ms / 1000 * SR))
    # vectorised one-pole release via scipy lfilter on the 'reduction' signal, then take min with need
    from scipy.signal import lfilter
    red = 1 - need
    sm = lfilter([1 - a], [1, -a], red)
    sm = np.maximum(sm, red)
    g = 1 - sm
    g = minimum_filter1d(g, size=la + 1, origin=-(la // 2))
    y = np.roll(x, 0, axis=0) * g[:, None]
    return y


try:
    import pyloudnorm  # noqa
    have_pyln = True
except ImportError:
    have_pyln = False


def ff_measure(x):
    tmp = os.path.join(ROOT, "work", "stems", "_meas.wav")
    sf.write(tmp, x, SR, subtype="FLOAT")
    ff = os.environ.get("FFMPEG", "ffmpeg")
    out = subprocess.run([ff, "-hide_banner", "-nostats", "-i", tmp, "-af", "ebur128=peak=true", "-f", "null", "-"],
                         capture_output=True, text=True).stderr
    I = float(out.split("Integrated loudness:")[1].split("I:")[1].split("LUFS")[0])
    TP = float(out.split("True peak:")[1].split("Peak:")[1].split("dBFS")[0])
    return I, TP


target = -14.0
gain_db = 0.0
for it in range(4):
    y = limiter(full * db(gain_db))
    I, TP = ff_measure(y)
    print(f"iter {it}: gain {gain_db:+.2f} dB -> {I:.2f} LUFS, TP {TP:.2f} dBTP")
    if abs(I - target) < 0.15:
        break
    gain_db += target - I
# last 3 s: the music's own fade carries the end; clean digital tail
y[-480:] = 0
sf.write(os.path.join(ROOT, "assets", "audio", "mix.wav"), y, SR, subtype="PCM_24")
sf.write(os.path.join(ROOT, "work", "stems", "sfx.wav"), sfx, SR, subtype="PCM_24")
I, TP = ff_measure(y)
json.dump(dict(lufs=I, true_peak=TP, gain_db=gain_db, cues=report), open(os.path.join(ROOT, "work", "mix_report.json"), "w"), indent=1)
print(f"mix.wav: {I:.2f} LUFS integrated, true peak {TP:.2f} dBTP, {len(report)} cues")
