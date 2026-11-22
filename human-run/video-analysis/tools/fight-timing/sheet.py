"""Contact sheet of frames at given video times: sheet.py VIDEO CROP OUT t1 t2 ... (cols 4)."""
import subprocess, sys, os
V, CROP, OUT = sys.argv[1], sys.argv[2], sys.argv[3]
ts = [float(x) for x in sys.argv[4:]]
D = OUT + '.d'
os.makedirs(D, exist_ok=True)
FONT = '/usr/share/fonts/truetype/dejavu/DejaVuSansMono.ttf'
files = []
for i, t in enumerate(ts):
    f = f'{D}/{i:03d}.png'
    subprocess.run(['ffmpeg', '-v', 'error', '-y', '-ss', str(t), '-i', V, '-frames:v', '1', '-vf',
                    f"crop={CROP},scale=360:-2,drawtext=fontfile={FONT}:text='{t:.1f}':x=2:y=2:fontsize=16:fontcolor=yellow:box=1:boxcolor=black@0.6",
                    f], check=True)
    files.append(f)
cols = 4
rows = (len(files) + cols - 1) // cols
args = ['ffmpeg', '-v', 'error', '-y']
for f in files:
    args += ['-i', f]
while len(files) < rows * cols:
    args += ['-f', 'lavfi', '-i', 'color=black:s=360x240']
    files.append(None)
layout = '|'.join(f'{(i % cols) * 360}_{(i // cols) * 240}' for i in range(rows * cols))
args += ['-filter_complex', f"xstack=inputs={rows * cols}:layout={layout}", '-frames:v', '1', OUT]
subprocess.run(args, check=True)
