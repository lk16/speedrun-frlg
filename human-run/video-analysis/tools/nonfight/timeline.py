"""nfw-tl.py FEAT A B: video-second window A..B; prints segments between black runs (>=4 f):
start, dur, black-before, frac W (overworld text), frac T/A/M (battle), label B if battle segment."""
import sys, importlib.util
spec = importlib.util.spec_from_file_location('bt', __import__('os').path.join(__import__('os').path.dirname(__import__('os').path.abspath(__file__)), '..', 'battle-timeline.py'))
bt = importlib.util.module_from_spec(spec); spec.loader.exec_module(bt)
FPS = 60.0
path, A, B = sys.argv[1], float(sys.argv[2]), float(sys.argv[3])
f = open(path, 'rb')
i0 = int(A * FPS); i1 = int(B * FPS)
f.seek(i0 * 39); data = f.read((i1 - i0) * 39)
n = len(data) // 39
pal = bt.PAL['wr']
st = [bt.classify(data[i*39:(i+1)*39], pal)[0] for i in range(n)]
rs = bt.runs(st)
blacks = [(a, b) for s, a, b in rs if s == 'K' and b - a >= 4]
prev = 0
rows = []
for (a, b) in blacks + [(n, n)]:
    seg = st[prev:a]
    if seg:
        L = len(seg)
        rows.append((prev, a, seg.count('W') / L, sum(seg.count(c) for c in 'TAM') / L))
    rows.append(('K', a, b))
    prev = b
for r in rows:
    if r[0] == 'K':
        if r[2] > r[1]:
            print(f"   black {A + r[1]/FPS:8.2f} {(r[2]-r[1])/FPS:5.2f}")
    else:
        a, b, w, t = r
        lab = 'BATTLE' if t > 0.3 else ('text' if w > 0.3 else '')
        print(f"{A + a/FPS:8.2f} +{(b-a)/FPS:6.2f}  W{w:4.0%} T{t:4.0%} {lab}")
