"""Torn frames: what a frame has that neither neighbouring frame explains, measured as
row-to-row stripes (motion leaves smooth residue; a torn readback leaves horizontal bands).
Prints frame numbers whose stripe score passes the threshold (argv[2], default 1.5)."""
import subprocess, sys
import numpy as np
w, h = 640, 360
raw = subprocess.run(['ffmpeg', '-v', 'error', '-i', sys.argv[1], '-vf', 'scale=%d:%d' % (w, h), '-f', 'rawvideo', '-pix_fmt', 'gray', '-'], capture_output=True).stdout
f = np.frombuffer(raw, np.uint8).reshape(-1, h, w).astype(np.float32)
limit = float(sys.argv[2]) if len(sys.argv) > 2 else 1.5
for i in range(1, len(f) - 1):
    r = np.maximum(0.0, np.minimum(np.abs(f[i] - f[i - 1]), np.abs(f[i] - f[i + 1])) - np.abs(f[i - 1] - f[i + 1]) / 2 - 4)
    stripes = np.abs(np.diff(r, axis=0)).mean()
    if stripes > limit:
        print(i, round(float(stripes), 2))
