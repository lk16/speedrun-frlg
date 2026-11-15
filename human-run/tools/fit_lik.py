#!/usr/bin/env python3
"""Likelihood refit of a stand-in against the WR video's single action counts.

usage: fit_lik.py PLAN.md KEY FIGHT,FIGHT,... V1,V2,... [--tol 1.0] [--R 60000] [--seed 3] [--free-torrent f1,f2]

`fit_league.py` compares the model's mean action count with the WR's one draw (mean |log ratio|). This one asks the
better question: how likely is the model to produce the WR's count in each fight, `P(|actions - WR| <= tol)` under the
model's own per-attempt distribution (`FRLG_SIM_TRACE` fight lines of `frlg-human splits`), summed over fights as a
log-likelihood. The turn counts of the video are +-1 estimates, hence `--tol`. KEY is any top-level plan parameter
(`league_power`, `mewtwo_power`, `carry_power`). `--free-torrent` sets `FightTactic.free_torrent` on those fights first.
Prints one row per value; a value within 2 log-likelihood units of the best is inside the ~95% interval. stdlib only.
"""
import collections, json, math, os, re, subprocess, sys

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


tol = float(opt('--tol', '1.0'))
R = int(opt('--R', '60000'))
seed = int(opt('--seed', '3'))
ft = [x for x in opt('--free-torrent', '').split(',') if x]
plan, key, names, vals = a[0], a[1], a[2].split(','), [float(x) for x in a[3].split(',')]
# `wrsplits.rs` WR_FIGHTS: the video's player actions per fight.
WR = {'misty': 6.5, 'rival-ss-anne': 6, 'koga': 8, 'blaine': 6, 'sabrina': 9, 'giovanni': 6, 'rival-route22': 12,
      'lorelei': 11, 'bruno': 7, 'agatha': 6, 'lance': 8, 'champion': 10, 'bruno-2': 5, 'agatha-2': 6.5,
      'lance-2': 7.5, 'champion-2': 11.5}
base = json.loads(open(plan).read().split('```json')[1].split('```')[0])
for n in ft:
    base.setdefault('fight_tactics', {}).setdefault(n, {})['free_torrent'] = True
best = None
rows = []
for v in vals:
    q = dict(base, **{key: v}, name='fitlik')
    path = os.path.join(SCRATCH, 'fitlik-%d.md' % os.getpid())
    open(path, 'w').write('```json\n' + json.dumps(q) + '\n```\n')
    env = dict(os.environ, FRLG_SIM_TRACE='1')
    r = subprocess.run([BIN, 'splits', path, '--R', str(R), '--seed', str(seed)], capture_output=True, text=True, env=env)
    got = collections.defaultdict(list)
    for ln in r.stderr.splitlines():
        if ln.startswith('fight '):
            nm = ln.split()[1]
            m = re.search(r'-> won (\w+) turns (\d+)', ln)
            if nm in names and m:
                got[nm].append(int(m.group(2)))
    ll, parts = 0.0, []
    for n in names:
        xs = got[n]
        if not xs:
            continue
        k = sum(1 for x in xs if abs(x - WR[n]) <= tol)
        p = max(k, 0.5) / len(xs)
        ll += math.log(p)
        parts.append('%s %.3f' % (n, p))
    rows.append((v, ll, parts))
    print('%s %g: loglik %.2f | %s' % (key, v, ll, ' '.join(parts)), flush=True)
top = max(rows, key=lambda x: x[1])
inside = [v for v, ll, _ in rows if ll >= top[1] - 2.0]
print('best %s = %g (loglik %.2f); within 2 units: %s' % (key, top[0], top[1], inside))
