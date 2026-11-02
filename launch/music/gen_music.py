"""Generate the launch film's music with MusicGen, section by section, each continuing the
previous one so it stays one piece: a black-and-white intro that builds into a modern drop.

    python launch/music/gen_music.py [--model facebook/musicgen-medium] [--seed 7]

Writes launch/music/sections/*.wav and launch/music/track.wav (32 kHz mono -> 48 kHz stereo
is done in the mix).
"""
import argparse
from pathlib import Path

import numpy as np
import soundfile as sf
import torch
from transformers import AutoProcessor, MusicgenForConditionalGeneration

HERE = Path(__file__).parent

# (name, seconds of new audio, prompt). 120 BPM, 4/4, A minor -> C major lift at the drop.
SECTIONS = [
    ("intro", 16, "120 bpm, solo felt piano with vinyl crackle and a ticking old film projector, "
                  "melancholic and curious, black and white silent-film mood, sparse, in A minor"),
    ("build", 14, "120 bpm, the felt piano continues and a pulsing synth bass and soft kick join, "
                  "rising tension, filtered risers, modern cinematic electronic, building"),
    ("drop", 22, "120 bpm, big confident drop: punchy modern drums, deep synth bass, bright "
                 "arpeggiated synths, uplifting tech product launch anthem, energetic, in C major"),
    ("ride", 16, "120 bpm, the energetic electronic groove continues with claps and plucked "
                 "synth melody, playful and confident, product launch, steady beat"),
    ("outro", 10, "120 bpm, the beat stops, one big final hit with a warm synth chord swelling "
                  "and fading out, resolved and hopeful ending"),
]
PROMPT_TAIL = 6.0  # seconds of the previous section given as the audio prompt


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--model", default="facebook/musicgen-medium")
    ap.add_argument("--seed", type=int, default=7)
    a = ap.parse_args()
    torch.manual_seed(a.seed)
    dev = "cuda" if torch.cuda.is_available() else "cpu"
    proc = AutoProcessor.from_pretrained(a.model)
    model = MusicgenForConditionalGeneration.from_pretrained(a.model, torch_dtype=torch.float16 if dev == "cuda" else torch.float32).to(dev)
    sr = model.config.audio_encoder.sampling_rate
    fps = model.config.audio_encoder.frame_rate  # tokens per second
    out_dir = HERE / "sections"
    out_dir.mkdir(exist_ok=True)
    track = np.zeros(0, dtype=np.float32)
    for name, secs, prompt in SECTIONS:
        if len(track) == 0:
            inputs = proc(text=[prompt], padding=True, return_tensors="pt").to(dev)
            new_tokens = int(secs * fps)
            audio = model.generate(**inputs, do_sample=True, guidance_scale=3.5, max_new_tokens=new_tokens)
            wav = audio[0, 0].float().cpu().numpy()
        else:
            tail = track[-int(PROMPT_TAIL * sr):]
            inputs = proc(audio=[tail], sampling_rate=sr, text=[prompt], padding=True, return_tensors="pt").to(dev)
            inputs["input_values"] = inputs["input_values"].to(model.dtype)
            new_tokens = int((PROMPT_TAIL + secs) * fps)
            audio = model.generate(**inputs, do_sample=True, guidance_scale=3.5, max_new_tokens=new_tokens)
            wav = audio[0, 0].float().cpu().numpy()[len(tail):]
        sf.write(out_dir / f"{name}.wav", wav, sr)
        track = np.concatenate([track, wav])
        print(f"{name}: {len(wav) / sr:.1f} s (total {len(track) / sr:.1f} s)", flush=True)
    peak = np.abs(track).max() or 1.0
    sf.write(HERE / "track.wav", track / peak * 0.89, sr)
    print("wrote", HERE / "track.wav")


if __name__ == "__main__":
    main()
