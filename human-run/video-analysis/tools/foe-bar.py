#!/usr/bin/env python3 -I
"""bar2.py VIDEO T0 T1 STEP -> foe HP-bar fill (of 48 native px) and player HP text region presence per frame.

One ffmpeg call decodes the whole window at 1/STEP fps. Frames are 480x320 (2x native). Foe bar: row y=68,
x 103..198 (96 px = 48 native). Filled = saturated pixel (max-min > 0x50). Prints t, fill/48, bar-visible flag.
"""
import subprocess, sys

W, H = 480, 320
v, t0, t1, step = sys.argv[1], float(sys.argv[2]), float(sys.argv[3]), float(sys.argv[4])
cmd = ['ffmpeg', '-v', 'error', '-ss', str(t0), '-t', str(t1 - t0), '-i', v,
       '-vf', 'fps=1/%g,crop=1190:795:365:30,scale=%d:%d:flags=neighbor' % (step, W, H), '-f', 'rawvideo', '-pix_fmt', 'rgb24', '-']
d = subprocess.run(cmd, capture_output=True).stdout
n = len(d) // (W * H * 3)
for k in range(n):
    f = d[k * W * H * 3:(k + 1) * W * H * 3]
    row = [(f[(68 * W + x) * 3], f[(68 * W + x) * 3 + 1], f[(68 * W + x) * 3 + 2]) for x in range(103, 199)]
    # the bar frame: dark border pixels at row 62 across the bar means the foe box is up
    border = sum(1 for x in range(103, 199, 4) if max(f[(62 * W + x) * 3:(62 * W + x) * 3 + 3]) < 0x70)
    sat = [1 if max(p) - min(p) > 0x50 else 0 for p in row]
    # native pixels: sample every 2nd px
    fill = sum(sat[0::2])
    print('%8.1f fill %2d/48 border %d' % (t0 + k * step, fill, border))
