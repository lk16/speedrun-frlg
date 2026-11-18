#!/usr/bin/env python3
"""Post-door survival and slow-event rates from the runner's streams (2026-11-15 evidence fit).

usage: fit_post_evidence.py PLAN.md [--k 3] [--R 800000] [--seed 1] [--draws 20000] [--out post_fatal.json] [--alt]

Reads video-analysis/data/post-door-evidence.json (door attempts: where each ended). Per post-door fight:
at-risk and deaths, a Beta posterior for the TOTAL hazard (prior = the plan's current total hazard
1-(1-engine)(1-fatal), worth `k` observations, so a fight with no data keeps its value), and the `fatal`
that reproduces the posterior mean on top of the engine's own death rate (engine-only survival from the
plan run with `fatal` cleared at the lenient rung, as fit_hazards.py does). Prints the table, the door-to-finish
check (Wilson), the posterior of the product S, and the slow-event checks. --alt moves the 11-09 death to the
Champion. stdlib only.
"""
import json, math, os, random, subprocess, sys

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
import fit_hazards as fh  # noqa: E402

DATA = os.path.join(HERE, '..', 'video-analysis', 'data', 'post-door-evidence.json')
a = sys.argv[1:]


def opt(n, d):
    if n in a:
        i = a.index(n)
        v = type(d)(a[i + 1])
        del a[i:i + 2]
        return v
    return d


k, R, seed, draws = opt('--k', 3.0), opt('--R', 800000), opt('--seed', 1), opt('--draws', 20000)
out = opt('--out', '')
alt = '--alt' in a
if alt:
    a.remove('--alt')
plan = a[0]
ev = json.load(open(DATA))
FIGHTS = ev['fights']
WH = ['icefall-grunt', 'warehouse-admin-1', 'warehouse-admin-2', 'warehouse-gideon']


def wilson(x, n, z=1.96):
    if n == 0:
        return (0.0, 1.0)
    p = x / n
    d = 1 + z * z / n
    c = p + z * z / (2 * n)
    h = z * math.sqrt(p * (1 - p) / n + z * z / (4 * n * n))
    return ((c - h) / d, (c + h) / d)


def counts():
    """(at_risk, deaths) per fight, expected values for the unlocated warehouse death."""
    n = {f: 0.0 for f in FIGHTS}
    d = {f: 0.0 for f in FIGHTS}
    for at in ev['door_attempts']:
        r, o = at['reached'], at['outcome']
        if r == 'none':
            continue
        if r == 'all':
            for f in FIGHTS:
                n[f] += 1
            continue
        if r == 'warehouse':
            for f in FIGHTS[:5]:
                n[f] += 1
            for j, f in enumerate(WH):
                n[f] += (4 - j) / 4
                d[f] += 0.25
            continue
        loc = at.get('alt') if alt and at.get('alt') else r
        i = FIGHTS.index(loc)
        for f in FIGHTS[:i + 1]:
            n[f] += 1
        d[loc] += 1
    return n, d


def main():
    params = json.loads(open(plan).read().split('```json')[1].split('```')[0])
    fatal = params['fatal']
    # engine-only per-fight death rate (lenient rung, fatal cleared)
    p0 = dict(params)
    p0['fatal'] = {}
    p0['hazard'] = {}
    scratch = os.environ.get('FRLG_SCRATCH', '/tmp/frlg-human-scratch')
    os.makedirs(scratch, exist_ok=True)
    f0 = os.path.join(scratch, '_post_ev_engine.md')
    open(f0, 'w').write('```json\n' + json.dumps(p0) + '\n```\n')
    reach = fh.reach(f0, 'WR+160m', R, seed)
    names = list(reach)
    eng = {}
    for f in FIGHTS:
        i = names.index(f)
        # `report` prints the share of attempts that COMPLETED a segment: survival of f = reach[f] / reach[previous]
        eng[f] = max(0.0, 1 - reach[f] / reach[names[i - 1]])
    n, d = counts()
    random.seed(7)
    rows, post_fatal, tot_prior, tot_post = [], {}, 1.0, 1.0
    posts = {}
    for f in FIGHTS:
        fa = fatal.get(f, 0.0)
        h0 = 1 - (1 - eng[f]) * (1 - fa)
        al, be = d[f] + k * h0, (n[f] - d[f]) + k * (1 - h0)
        mean = al / (al + be)
        posts[f] = (al, be)
        fnew = max(0.0, 1 - (1 - mean) / (1 - eng[f]))
        post_fatal[f] = round(fnew, 5)
        tot_prior *= 1 - h0
        tot_post *= 1 - mean
        lo, hi = wilson(d[f], n[f]) if n[f] >= 1 else (0, 1)
        rows.append((f, n[f], d[f], eng[f], fa, h0, mean, fnew, lo, hi))
    print(f'prior weight k = {k}; alt={alt}')
    print(f'{"fight":<20}{"at risk":>8}{"deaths":>7}{"engine":>8}{"fatal0":>8}{"total0":>8}{"post":>8}{"fatal1":>8}  Wilson(raw)')
    for r in rows:
        print(f'{r[0]:<20}{r[1]:>8.2f}{r[2]:>7.2f}{r[3]:>8.3f}{r[4]:>8.3f}{r[5]:>8.3f}{r[6]:>8.3f}{r[7]:>8.3f}  [{r[8]:.2f},{r[9]:.2f}]')
    s_prior_f = 1.0
    s_post_f = 1.0
    for f in FIGHTS:
        s_prior_f *= 1 - fatal.get(f, 0.0)
        s_post_f *= 1 - post_fatal[f]
    print(f'product of fatal survivals: prior S {s_prior_f:.3f} -> evidence S {s_post_f:.3f}; total (engine incl) {tot_prior:.3f} -> {tot_post:.3f}')
    # posterior of the product (total, then fatal-equivalent) by sampling the per-fight Betas
    tots, fats = [], []
    eng_prod = 1.0
    for f in FIGHTS:
        eng_prod *= 1 - eng[f]
    for _ in range(draws):
        t = 1.0
        for f in FIGHTS:
            t *= 1 - random.betavariate(max(posts[f][0], 1e-6), max(posts[f][1], 1e-6))
        tots.append(t)
        fats.append(t / eng_prod)
    tots.sort()
    fats.sort()
    q = lambda v, x: v[int(x * (len(v) - 1))]
    print(f'posterior total survival door..HoF2: median {q(tots, .5):.3f} 90% [{q(tots, .05):.3f}, {q(tots, .95):.3f}];'
          f' fatal-equivalent S: median {q(fats, .5):.3f} 90% [{q(fats, .05):.3f}, {q(fats, .95):.3f}]')
    # door -> finish raw check
    fought = [x for x in ev['door_attempts'] if x['reached'] != 'none']
    fin = sum(1 for x in fought if x['outcome'] == 'finish')
    lo, hi = wilson(fin, len(fought))
    print(f'door attempts that fought: {len(fought)}, finished {fin}: {fin / len(fought):.2f} Wilson [{lo:.3f}, {hi:.3f}]; engine-only product {eng_prod:.3f}'
          f' -> data-only S range [{lo / eng_prod:.3f}, {min(1, hi / eng_prod):.3f}]')
    # slow-event checks: P(extra >= x) in the model vs the observed bad rate
    for name, seg, x in (('safari', 'safari-walk', 120), ('power_plant', 'hunt-power_plant-land@powerplant-out', 120)):
        ev_s = ev[name]
        lo, hi = wilson(ev_s['bad'], ev_s['visits'])
        for s in params.get('slow', []):
            if s['seg'] == seg:
                pm = s['p'] * math.exp(-x / s['mean_s'])
                print(f'slow {name}: observed {ev_s["bad"]}/{ev_s["visits"]} bad = {ev_s["bad"] / ev_s["visits"]:.3f} Wilson [{lo:.3f}, {hi:.3f}];'
                      f' model P(extra >= {x} s) = {pm:.3f} (p {s["p"]}, mean {s["mean_s"]} s)')
    if out:
        json.dump(post_fatal, open(out, 'w'))
        print('wrote', out)
    print(json.dumps(post_fatal))


if __name__ == '__main__':
    main()
