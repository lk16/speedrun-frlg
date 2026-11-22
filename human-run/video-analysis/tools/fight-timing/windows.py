"""bos-win.py splits.txt [...]: per WR window, model err split into trainer-fight err and non-fight err.
WR fight seconds per window from the 67 VOD-timed fights (episode = visible battle + the played overhead), placed by
run time = video t0 - 35 s. Windows after Giovanni 3 are not covered by the fight table."""
import json, re, statistics as S, sys
REPO = __import__('os').path.join(__import__('os').path.dirname(__import__('os').path.abspath(__file__)), '..', '..', '..', '..') + '/'
wr = json.load(open(REPO + 'human-run/video-analysis/data/wr-fight-timing.json'))
P = [r for r in wr if r.get('bot_ep')]
OV = S.median(r['bot_ep'] - r['bot_dur'] for r in P)
for fn in sys.argv[1:]:
    txt = open(fn).read()
    j = json.loads([l for l in txt.splitlines() if l.startswith('JSON ')][0][5:])
    kind = {}
    for l in txt.split('model time by kind')[1].splitlines()[1:]:
        m = re.match(r'(.+?)\s+fight\s+(\d+)\s+walk/wild\s+(\d+)\s+text/other\s+(\d+)', l)
        if not m:
            break
        kind[m.group(1).strip()] = (int(m.group(2)), int(m.group(3)), int(m.group(4)))
    print(fn)
    print(f"{'window':14s} {'WR s':>6s} {'err':>6s} | {'WR fights':>9s} {'model':>6s} {'f.err':>6s} | {'non-fight err':>13s}")
    prev_wr = prev_m = 0.0
    tot = [0.0] * 3
    for sp in j['splits']:
        if sp['n'] == 'Start E4R1':
            break
        ww = sp['wr'] - prev_wr; wm = sp['m'] - prev_m
        wf = sum(r['wr_dur'] + ((r['bot_ep'] - r['bot_dur']) if r.get('bot_ep') else OV)
                 for r in wr if prev_wr <= r['wr_t0'] - 35 < sp['wr'])
        mf = kind[sp['n']][0]
        e = wm - ww; fe = mf - wf
        tot[0] += e; tot[1] += fe; tot[2] += e - fe
        print(f"{sp['n']:14s} {ww:6.0f} {e:+6.0f} | {wf:9.0f} {mf:6.0f} {fe:+6.0f} | {e - fe:+13.0f}")
        prev_wr, prev_m = sp['wr'], sp['m']
    print(f"{'total':14s} {'':6s} {tot[0]:+6.0f} | {'':9s} {'':6s} {tot[1]:+6.0f} | {tot[2]:+13.0f}")
