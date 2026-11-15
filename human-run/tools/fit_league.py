#!/usr/bin/env python3
"""Refit league_power (round 1) and mewtwo_power (round 2) against the WR video's League fights.

usage: fit_league.py PLAN.md [--R 200000] [--seeds 3,4] [--lp 1.8,2.0,...] [--mp 1.0,1.2,...] [--patch JSON]

For every grid value runs `frlg-human splits` and sums |log(model/WR)| over the League fights of that round
(player actions and core seconds, `wrsplits.rs` fight table; `--patch` is merged into the plan first, e.g. the
honest-sacrifice flags). Round 1 = lorelei bruno agatha lance champion, round 2 = bruno-2 agatha-2 lance-2
champion-2 (the WR video has no Lorelei-2 measurement). Prints one row per value; the minimum is the fit. stdlib only.
"""
import json, math, os, re, subprocess, sys

ROOT = os.path.join(os.path.dirname(os.path.abspath(__file__)), '..', '..')
BIN = os.environ.get('FRLG_BIN', os.path.join(ROOT, 'target', 'release', 'frlg-human'))
SCRATCH = os.environ.setdefault('FRLG_SCRATCH', '/tmp/frlg-human-scratch')
os.makedirs(SCRATCH, exist_ok=True)
a = sys.argv[1:]


def opt(n, d):
    if n in a:
        i = a.index(n)
        v = a[i + 1]
        del a[i:i + 2]
        return v
    return d


R = int(opt('--R', '200000'))
seeds = [int(s) for s in opt('--seeds', '3,4').split(',')]
lps = [float(x) for x in opt('--lp', '1.6,1.8,2.0,2.2,2.4,2.6').split(',') if x]
mps = [float(x) for x in opt('--mp', '1.0,1.2,1.4,1.6,1.8,2.0').split(',') if x]
cps = [float(x) for x in opt('--cp', '').split(',') if x]  # carry_power over the mid-game gym/rival fights
patch = json.loads(opt('--patch', '{}'))
R1 = ['lorelei', 'bruno', 'agatha', 'lance', 'champion']
RM = ['misty', 'rival-ss-anne', 'koga', 'blaine', 'sabrina', 'giovanni', 'rival-route22']
R2 = ['bruno-2', 'agatha-2', 'lance-2', 'champion-2']
plan = a[0]
base = json.loads(open(plan).read().split('```json')[1].split('```')[0])
base.update(patch)
ROW = re.compile(r'^\s+(\S+)\s+model\s+([\d.]+) actions core\s+(\d+) s .*\| WR\s+([\d.]+) actions\s+(\d+) s')


def run(kw, seed):
    q = dict(base, **kw)
    q['name'] = 'fitleague'
    path = os.path.join(SCRATCH, 'fitleague-%d.md' % os.getpid())
    open(path, 'w').write('```json\n' + json.dumps(q) + '\n```\n')
    out = subprocess.run([BIN, 'splits', path, '--R', str(R), '--seed', str(seed)], capture_output=True, text=True).stdout
    rows = {}
    for l in out.splitlines():
        m = ROW.match(l)
        if m:
            rows[m.group(1)] = tuple(float(x) for x in m.groups()[1:])
    return rows


def score(rows, names):
    ta = ts = 0.0
    det = []
    for n in names:
        if n not in rows:
            return None
        ma, ms, wa, ws = rows[n]
        ta += abs(math.log(ma / wa))
        ts += abs(math.log(ms / ws))
        det.append('%s %.1f/%g %d/%d' % (n, ma, wa, ms, ws))
    return ta / len(names), ts / len(names), det


def grid(key, vals, names):
    print(f'-- {key} ({len(names)} fights, R={R}, seeds {seeds})')
    best = None
    for v in vals:
        sa = ss = 0.0
        det = None
        for sd in seeds:
            s = score(run({key: v}, sd), names)
            if s is None:
                sa = None
                break
            sa += s[0] / len(seeds)
            ss += s[1] / len(seeds)
            det = s[2]
        if sa is None:
            print(f'{key} {v}: fights missing')
            continue
        tot = sa + ss
        print(f'{key} {v}: actions {sa:.3f} seconds {ss:.3f} sum {tot:.3f} | {"; ".join(det)}', flush=True)
        if best is None or tot < best[0]:
            best = (tot, v)
    print(f'best {key} = {best[1]} (sum {best[0]:.3f})' if best else 'no fit')


if cps:
    grid('carry_power', cps, RM)
if lps:
    grid('league_power', lps, R1)
if mps:
    grid('mewtwo_power', mps, R2)
