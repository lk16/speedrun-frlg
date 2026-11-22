#!/usr/bin/env python3
"""League stand-in fit at the WR's League level: grid over league_power / mewtwo_power, `frlg-human fightdist` per cell,
cal_lik log-likelihood of the WR + A1 per-fight action counts (round 1 and round 2), plus the win rates per fight.

usage: league_fit_grid.py ROOT BASE_PLAN OUT.jsonl --lp 0.9,1.0,... --mp 1.3 [--R 200000] [--seed 7701] [--wrq 1] [--set k=json ...]
--wrq 1: Lance / Champion / Champion-2 X queues = the WR and A1 queues (as the 1113 fit, journal 2026-12-08-14-00).
"""
import json, os, subprocess, sys

a = sys.argv[1:]


def opt(n, d):
    if n in a:
        i = a.index(n)
        v = a[i + 1]
        del a[i:i + 2]
        return v
    return d


lps = [float(x) for x in opt('--lp', '1.0').split(',')]
mps = [float(x) for x in opt('--mp', '1.3').split(',')]
R = opt('--R', '200000')
seed = opt('--seed', '7701')
wrq = opt('--wrq', '1') == '1'
sets = []
while '--set' in a:
    sets.append(opt('--set', ''))
root, base, out = a[:3]
BIN = os.path.join(root, 'target', 'release', 'frlg-human')
sys.path.insert(0, os.path.join(root, 'human-run', 'tools'))
import cal_lik  # noqa: E402

SCR = os.environ.get('FRLG_SCRATCH', '/tmp/frlg-human-scratch')
os.makedirs(SCR, exist_ok=True)
P = json.loads(open(base).read().split('```json')[1].split('```')[0])
if wrq:
    ft = P['fight_tactics']
    ft['lance'] = dict(ft.get('lance', {}), x=['special', 'accuracy'])
    ft['champion'] = dict(ft.get('champion', {}), x=['guard', 'speed', 'special', 'special', 'special'])
    ft['champion-2'] = dict(ft.get('champion-2', {}), x=['special', 'attack', 'attack'])
for s in sets:
    k, v = s.split('=', 1)
    P[k] = json.loads(v)
L = ['lorelei', 'bruno', 'agatha', 'lance', 'champion', 'lorelei-2', 'bruno-2', 'agatha-2', 'lance-2', 'champion-2']
for lp in lps:
    for mp in mps:
        q = dict(P, league_power=lp, mewtwo_power=mp, name=f'opt-lfit-{lp}-{mp}')
        f = os.path.join(SCR, f'opt-lfit-{os.getpid()}.md')
        open(f, 'w').write('```json\n' + json.dumps(q) + '\n```\n')
        fd = os.path.join(SCR, f'opt-lfit-{os.getpid()}.jsonl')
        with open(fd, 'w') as fo:
            subprocess.run([BIN, 'fightdist', f, '--rung', 'WR+160m', '--R', R, '--seed', seed], stdout=fo, check=True)
        xs = cal_lik.load(fd)
        r1, p1 = cal_lik.lik(xs, cal_lik.GROUPS['r1'])
        r2, p2 = cal_lik.lik(xs, cal_lik.GROUPS['r2'])
        # per-fight reach and win among fought
        fought, won, n_att = {}, {}, 0
        for l in open(fd):
            n_att += 1
            for n, act, w, fr in json.loads(l)['fights']:
                if n in L:
                    fought[n] = fought.get(n, 0) + 1
                    won[n] = won.get(n, 0) + (1 if w else 0)
        rec = dict(lp=lp, mp=mp, R=int(R), seed=int(seed), wrq=wrq, sets=sets, league_attempts=n_att, ll_r1=round(r1, 2),
                   ll_r2=round(r2, 2), parts=p1 + p2,
                   win={n: round(won.get(n, 0) / fought[n], 3) for n in L if fought.get(n)},
                   mean_actions={n: round(sum(v) / len(v), 1) for n, v in xs.items() if n in L})
        open(out, 'a').write(json.dumps(rec) + '\n')
        print(f"lp {lp} mp {mp}: league attempts {n_att}  ll r1 {r1:.2f}  r2 {r2:.2f}  win " +
              ' '.join(f"{n[:4]}{'2' if n.endswith('-2') else ''} {rec['win'].get(n, 0):.2f}" for n in L), flush=True)
print('LFIT-DONE')
