# out = L + R - BG: the build has no per-person skins, so each hand is rendered alone
# (light / dark) over the same backdrop and the two passes are merged.
import subprocess, sys, numpy as np
W, H = 1080, 1920
ff = "ffmpeg"
def reader(path):
    return subprocess.Popen([ff, "-v", "error", "-i", path, "-f", "rawvideo", "-pix_fmt", "rgb24", "-"], stdout=subprocess.PIPE)
bg = subprocess.run([ff, "-v", "error", "-i", "frames/0.png", "-f", "rawvideo", "-pix_fmt", "rgb24", "-"],
                    stdout=subprocess.PIPE, check=True).stdout
bg = np.frombuffer(bg, np.uint8).astype(np.int16).reshape(H, W, 3)
a, b = reader("L.mp4"), reader("R.mp4")
enc = subprocess.Popen([ff, "-v", "error", "-y", "-f", "rawvideo", "-pix_fmt", "rgb24", "-s", f"{W}x{H}", "-r", "30",
                        "-i", "-", "-c:v", "libx264", "-crf", "16", "-preset", "slow", "-pix_fmt", "yuv420p",
                        "-colorspace", "bt709", "-color_primaries", "bt709", "-color_trc", "bt709", "-movflags", "+faststart", "out.mp4"],
                       stdin=subprocess.PIPE)
n = W * H * 3
frames = 0
while True:
    x, y = a.stdout.read(n), b.stdout.read(n)
    if len(x) < n or len(y) < n:
        break
    x = np.frombuffer(x, np.uint8).astype(np.int16).reshape(H, W, 3)
    y = np.frombuffer(y, np.uint8).astype(np.int16).reshape(H, W, 3)
    dx, dy = np.abs(x - bg).sum(2, keepdims=True), np.abs(y - bg).sum(2, keepdims=True)
    # take whichever pass differs more from the backdrop (keeps codec noise from adding up)
    o = np.where(dx >= dy, x, y)
    enc.stdin.write(o.clip(0, 255).astype(np.uint8).tobytes())
    frames += 1
enc.stdin.close(); enc.wait()
print("frames", frames)
