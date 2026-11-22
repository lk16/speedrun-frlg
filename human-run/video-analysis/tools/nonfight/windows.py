"""nfw-win.py ALL.txt FLOOR.txt BATTLES.jsonl: per WR window, model vs WR split into
trainer fights / wild battles / field (walking + text + menus).
Model: per-segment mean seconds from `splits --all` (rounded to 1 s), kinds and floor frames from `segs --all --floor`.
WR: trainer fights = the 67 VOD-timed episodes (visible + played overhead, as windows.py); every other battle chain
the detector finds = 'wild' (catches, flees, plus the two untabled trainers: lab rival, Silph Giovanni);
the rest of the window = field."""
import json, re, statistics as S, sys
REPO = __import__('os').path.join(__import__('os').path.dirname(__import__('os').path.abspath(__file__)), '..', '..', '..', '..') + '/'
allf, floorf, batf = sys.argv[1:4]
detail = '--detail' in sys.argv
wr = json.load(open(REPO + 'human-run/video-analysis/data/wr-fight-timing.json'))
P = [r for r in wr if r.get('bot_ep')]
OV = S.median(r['bot_ep'] - r['bot_dur'] for r in P)
B = [json.loads(l) for l in open(batf)]
SPL = [("Brock", 880, "brock"), ("Misty", 1681, "misty-badge"), ("Bill", 2290, "bill-cottage"),
       ("Bike", 2774, "bike-redeem"), ("Rock Tunnel", 3339, "rock-tunnel-trainers"),
       ("Giovanni 1", 3896, "scope-and-dig"), ("Snorlax", 4787, "snorlax"), ("Koga", 5502, "koga-badge"),
       ("Blaine", 6174, "blaine-badge"), ("Sevii", 7087, "blaine-badge-and-sevii-scene"),
       ("Sabrina", 7914, "sabrina-badge"), ("Erika", 8090, "erika-badge"), ("Giovanni 3", 8633, "giovanni-badge")]
segs = []
for l in open(allf):
    m = re.match(r'(\S+)\s+(\d+):(\d+):(\d+)\s+(\d+):(\d+):(\d+)\s+[\d.]+$', l.strip())
    if m:
        g = list(map(int, m.groups()[1:]))
        cum = g[0] * 3600 + g[1] * 60 + g[2]
        segs.append((m.group(1), cum, cum - (segs[-1][1] if segs else 0)))
fl = {}
for l in open(floorf):
    i, n, k, f = l.split()
    fl[int(i)] = (n, k, float(f) / 59.7275)
FIGHT = {'Trainer', 'Parties', 'Group'}
WILD = {'Wander', 'Hunt', 'Grind', 'Mewtwo', 'Evolve'}
print(f"{'window':12s} {'WR':>5s} {'M':>5s} | {'WRtr':>5s} {'Mtr':>5s} | {'WRwild':>6s} {'Mwild':>5s} | {'WRfield':>7s} {'Mfield':>6s} {'err':>5s}")
prev_wr, si = 0.0, 0
tot = [0.0] * 8
for name, t, last in SPL:
    # model segments up to and including `last`
    acc = {'tr': 0.0, 'wild': 0.0, 'field': 0.0}
    rows = []
    while si < len(segs):
        n, cum, sec = segs[si]
        k = fl[si][1]
        fsec = fl[si][2]
        if k in FIGHT:
            acc['tr'] += sec
            c = 'tr'
        elif k == 'Walk':
            w = max(0.0, sec - fsec)
            acc['wild'] += w
            acc['field'] += sec - w
            c = f'walk(floor {fsec:.0f})'
        elif k in WILD:
            acc['wild'] += sec
            c = 'wild'
        else:
            acc['field'] += sec
            c = 'field'
        rows.append((n, k, sec, c))
        si += 1
        if n == last:
            break
    a, b = prev_wr + 35, t + 35
    wtr = sum(r['wr_dur'] + ((r['bot_ep'] - r['bot_dur']) if r.get('bot_ep') else OV)
              for r in wr if a <= r['wr_t0'] < b)
    T0 = [r['wr_t0'] for r in wr]
    wild = [x for x in B if a <= x['t0'] < b and not any(abs(x['t0'] - q) < 3 for q in T0)]
    wwild = sum(x['dur_s'] + 1.5 for x in wild)
    W = t - prev_wr
    wfield = W - wtr - wwild
    M = acc['tr'] + acc['wild'] + acc['field']
    print(f"{name:12s} {W:5.0f} {M:5.0f} | {wtr:5.0f} {acc['tr']:5.0f} | {wwild:6.0f} {acc['wild']:5.0f} | {wfield:7.0f} {acc['field']:6.0f} {acc['field'] - wfield:+5.0f}")
    if detail:
        for n, k, sec, c in rows:
            if sec:
                print(f"      {n:40s} {k:14s} {sec:5.0f} {c}")
        for x in wild:
            print(f"      WR battle {x['t0']:.0f} {x['dur_s']:.0f} s")
    for j, v in enumerate([W, M, wtr, acc['tr'], wwild, acc['wild'], wfield, acc['field']]):
        tot[j] += v
    prev_wr = t
print(f"{'total':12s} {tot[0]:5.0f} {tot[1]:5.0f} | {tot[2]:5.0f} {tot[3]:5.0f} | {tot[4]:6.0f} {tot[5]:5.0f} | {tot[6]:7.0f} {tot[7]:6.0f} {tot[7] - tot[6]:+5.0f}")
