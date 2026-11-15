#!/usr/bin/env python3
"""Fit the extended risk lane (plan 940+) to the runner's windowed pass rates.

usage: fit_lane.py PLAN.md [--R 400000] [--seed 1] [--iters 6] [--k 6] [--out windows.json] [--power-only]
                   [--rung WR+160m]

The lane (`risk_until`, `risk_windows`, `risk_abandon`) replaces the flat `fatal` hazards of the fights it covers: every
fight is played a second time with the formula's own damage, and a loss there ends the attempt. The only fitted numbers
are the per-window carry power factors `risk_windows[i].power` (the stand-in for what the engine's greedy human lacks,
exactly as `risk_power` 2.0 is at Misty): this script bisects each one (log scale) until the model's window survival at
the lenient rung (no pace resets to speak of) equals the runner's, which is the partial-pooled estimate of
`fit_hazards.py` (windows after Misty shrink toward the pooled late death share, k observations of prior; the Misty
window and earlier are taken as observed).

Windows are the runner's (`fit_attempts.WINDOWS`): survival through a window = `reached` of the window's split segment
over `reached` of the previous window's split segment (`report` counts the attempts that complete a segment). Prints the table (runner, model, power) and the JSON
`risk_windows`. stdlib only."""
import json, math, os, subprocess, sys

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.join(HERE, '..', '..')
BIN = os.environ.get('FRLG_BIN', os.path.join(ROOT, 'target', 'release', 'frlg-human'))
sys.path.insert(0, HERE)
import fit_attempts as fa  # noqa: E402

# window name -> split segment ending the window
EDGES = [
    ('forest-brock', 'brock', None),
    ('brock-misty', 'rival-cerulean', None),
    ('misty-rocktunnel', 'rock-tunnel-trainers', None),
    ('rt-snorlax', 'snorlax', None),
    ('snorlax-koga', 'koga-badge', None),
    ('koga-sevii', 'blaine-badge-and-sevii-scene', None),
    ('sevii-gio3', 'giovanni-badge', None),
    ('gio3-door', 'indigo-heal', None),
]
LANE_WINDOWS = ['misty-rocktunnel', 'rt-snorlax', 'snorlax-koga', 'koga-sevii', 'sevii-gio3', 'gio3-door']
# The flat hazards the lane replaces (fit_hazards.MODEL).
REPLACED = ['rock-tunnel-trainers', 'hideout-giovanni', 'rival-tower', 'surge', 'koga', 'fuchsia-gym-trainers', 'blaine',
            'three-island-paxton', 'rival-silph', 'sabrina', 'erika', 'giovanni', 'rival-route22', 'victory-road']


def reach(path, rung, R, seed):
    out = subprocess.check_output([BIN, 'report', path, '--rung', rung, '--R', str(R), '--seed', str(seed)], text=True)
    r = {}
    for line in out.splitlines():
        t = line.split()
        if len(t) > 2 and t[1].endswith('%'):
            r[t[0]] = float(t[1][:-1]) / 100
    return r


def targets(k):
    runs = fa.load()['runs']
    table, _, _ = fa.table(runs)
    byw = {t['window']: t for t in table}
    late = [t for t in table if t['window'] not in ('forest-brock', 'brock-misty', 'misty-rocktunnel')]
    q_pool = sum(t['deaths'] for t in late) / max(1, sum(t['at_risk'] for t in late))
    tg = {}
    for w, t in byw.items():
        if w in ('forest-brock', 'brock-misty', 'misty-rocktunnel'):
            tg[w] = 1 - t['deaths'] / t['at_risk']
        else:
            tg[w] = 1 - (t['deaths'] + k * q_pool) / (t['at_risk'] + k)
    return tg, byw


def main():
    a = sys.argv[1:]

    def opt(n, d):
        if n in a:
            i = a.index(n)
            v = type(d)(a[i + 1])
            del a[i:i + 2]
            return v
        return d

    pots = opt('--potions', -1)
    R, seed, iters, k = opt('--R', 400000), opt('--seed', 1), opt('--iters', 6), opt('--k', 6.0)
    rung, outp = opt('--rung', 'WR+160m'), opt('--out', '')
    check = '--check' in a
    if check:
        a.remove('--check')
    plan = a[0]
    if check:
        # Window survival of the plan as it stands (lane windows, fatal, everything) against the runner's.
        r = reach(plan, rung, R, seed)
        runs = fa.load()['runs']
        table, _, _ = fa.table(runs)
        print(f'{"window":<18}{"at risk":>8}{"deaths":>7}{"runner":>8}{"95% CI":>16}{"model":>8}')
        cum = 1.0
        for (w, e, _), t in zip(EDGES, table):
            pv = EDGES[EDGES.index((w, e, _)) - 1][1] if w != 'forest-brock' else 'starter'
            m = r[e] / r[pv]
            cum *= m
            print(f'{w:<18}{t["at_risk"]:>8}{t["deaths"]:>7}{t["cond_surv"]:>8.3f}{"[%.2f, %.2f]" % tuple(t["ci"]):>16}{m:>8.3f}')
        print(f'model survival starter -> {EDGES[-1][1]}: {cum:.4f}; runner {table[-1]["cum_surv"]:.4f}')
        return
    params = json.loads(open(plan).read().split('```json')[1].split('```')[0])
    params['risk_adopt'] = True
    params['risk_abandon'] = True
    params['risk_until'] = 'victory-road'
    fatal = dict(params.get('fatal', {}))
    for s in REPLACED:
        fatal.pop(s, None)
    params['fatal'] = fatal
    ends = {w: e for w, e, _ in EDGES}
    prev = {w: EDGES[i - 1][1] if i else 'starter' for i, (w, _, _) in enumerate(EDGES)}
    close = {w: c or e for w, e, c in EDGES}
    wins = {w['until']: w for w in params.get('risk_windows', [])}
    for w in LANE_WINDOWS:
        wins.setdefault(ends[w], {'until': ends[w], 'power': 1.0})
    tg, byw = targets(k)
    scratch = os.environ.get('FRLG_SCRATCH', '/tmp/frlg-human-scratch')
    os.makedirs(scratch, exist_ok=True)
    # Grid: every lane window at the same power g, the window survival measured conditional on arrival; the
    # survival is made monotone in g (isotonic, cummax) and inverted by linear interpolation in log g. One verify
    # run at the fitted powers follows (the windows are independent only to first order).
    grid = [0.6, 0.8, 1.0, 1.3, 1.7, 2.2, 3.0, 4.5]
    surv = {w: [] for w in LANE_WINDOWS}
    model = {}

    def run(powers):
        params['risk_windows'] = [dict({'until': ends[w], 'power': round(powers[w], 4)}, **({'potions': pots} if pots >= 0 else {}))
                                  for w in LANE_WINDOWS]
        params['name'] = 'fitlane'
        path = os.path.join(scratch, 'fitlane-%d.md' % os.getpid())
        open(path, 'w').write('```json\n' + json.dumps(params) + '\n```\n')
        r = reach(path, rung, R, seed)
        return {w: (r[ends[w]] / r[prev[w]] if r.get(prev[w]) else float('nan')) for w in LANE_WINDOWS}

    for g in grid:
        m = run({w: g for w in LANE_WINDOWS})
        for w in LANE_WINDOWS:
            surv[w].append(m[w])
        print('g %.2f: ' % g + '  '.join('%s %.3f' % (w[:7], m[w]) for w in LANE_WINDOWS), flush=True)
    cur = {}
    for w in LANE_WINDOWS:
        ys, best = [], 0.0
        for y in surv[w]:
            best = max(best, y)
            ys.append(best)
        t = tg[w]
        if t <= ys[0]:
            cur[w] = math.log(grid[0])
        elif t >= ys[-1]:
            cur[w] = math.log(grid[-1])
        else:
            for i in range(1, len(grid)):
                if ys[i] >= t:
                    f = (t - ys[i - 1]) / max(ys[i] - ys[i - 1], 1e-9)
                    cur[w] = math.log(grid[i - 1]) + f * (math.log(grid[i]) - math.log(grid[i - 1]))
                    break
    model = run({w: math.exp(cur[w]) for w in LANE_WINDOWS})
    print('verify: ' + '  '.join('%s %.3f/%.3f@%.2f' % (w[:7], model[w], tg[w], math.exp(cur[w])) for w in LANE_WINDOWS), flush=True)
    print(f'{"window":<18}{"at risk":>8}{"deaths":>7}{"runner":>8}{"target":>8}{"model":>8}{"power":>8}')
    for w, _, _ in EDGES:
        t = byw[w]
        m = model.get(w)
        print(f'{w:<18}{t["at_risk"]:>8}{t["deaths"]:>7}{t["cond_surv"]:>8.3f}{tg[w]:>8.3f}' +
              (f'{m:>8.3f}{math.exp(cur[w]):>8.2f}' if w in cur else ''))
    res = [dict({'until': ends[w], 'power': round(math.exp(cur[w]), 3)}, **({'potions': pots} if pots >= 0 else {}))
           for w in LANE_WINDOWS]
    print(json.dumps(res))
    if outp:
        json.dump(res, open(outp, 'w'))


if __name__ == '__main__':
    main()
