#!/usr/bin/env python3
"""Fit `fatal` hazards so the model's survival matches the runner's (fidelity fit 2026-11-14).

usage: fit_hazards.py PLAN.md [--R 200000] [--seed 1] [--k 6] [--out fatal.json]

1. Runs the plan with `fatal` cleared at the lenient rung (WR+160m: no pace resets to speak of),
   so the survival per window is the engine's own deaths (the fight engine plus `hazard`, which
   the plan should not carry any more).
2. Per window of `fit_attempts.py`: the runner's death share (Wilson-free point estimate with
   partial pooling: (d + k * q_pool) / (n + k), the pool being the windows after Misty) against
   the model's; the missing survival is spread over the window's named fights in log space,
   proportional to the weights below.
3. Prints the `fatal` map (JSON) and, with --out, writes it.

Weights inside a window are guesses from the runner's death lists (`!deaths` of 2026-11-12:
brock x5, sammy x2, liam, rival 1, rival 2) and quotes (STRATEGY section 4, 7); the window totals
are data. Stdlib only.
"""
import json, math, os, subprocess, sys

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.join(HERE, '..', '..')
BIN = os.environ.get('FRLG_BIN', os.path.join(ROOT, 'target', 'release', 'frlg-human'))
sys.path.insert(0, HERE)
import fit_attempts as fa  # noqa: E402

# window -> (entry segment, exit segment, {fight: weight})
MODEL = {
    'forest-brock': ('rival-lab', 'brock', {'forest-trainers': 2, 'pewter-gym-trainer': 1, 'brock': 5}),
    'brock-misty': ('brock', 'rival-cerulean', {'route3-trainers': 1, 'mt-moon-miguel': 1, 'misty': 2, 'rival-cerulean': 2}),
    'misty-rocktunnel': ('rival-cerulean', 'rock-tunnel-trainers', {'nugget-bridge-trainers': 1, 'rival-ss-anne': 3, 'rock-tunnel-trainers': 1}),
    'rt-snorlax': ('rock-tunnel-trainers', 'snorlax', {'hideout-giovanni': 1, 'rival-tower': 2, 'surge': 1}),
    'snorlax-koga': ('snorlax', 'koga-badge', {'koga': 4, 'fuchsia-gym-trainers': 1}),
    'koga-sevii': ('koga-badge', 'blaine-badge-and-sevii-scene', {'mansion-trainers': 1, 'blaine': 3, 'three-island-paxton': 1}),
    'sevii-gio3': ('blaine-badge-and-sevii-scene', 'giovanni-badge', {'rival-silph': 2, 'sabrina': 3, 'erika': 3, 'giovanni': 2}),
    'gio3-door': ('giovanni-badge', 'indigo-heal', {'rival-route22': 3, 'victory-road': 1}),
}


# After the door the data hold one finish in five door arrivals and no post-door death at all
# (accepted-runs.json), so only the shape is a guess: relative weights of the fights, anecdote-based
# (Lance and the Champion are where the runner says E4 runs are lost, STRATEGY 4 / 7; the Warehouse
# bosses and the Round 2 League likewise). The total survival `post_surv` is the calibration
# residual (see the journal).
POST_WEIGHTS = {
    'lorelei': 1, 'bruno': 1, 'agatha': 1, 'lance': 2, 'champion': 2,
    'icefall-grunt': 0.5, 'warehouse-admin-1': 0.5, 'warehouse-admin-2': 0.5, 'warehouse-gideon': 1,
    'lorelei-2': 1, 'bruno-2': 1, 'agatha-2': 1, 'lance-2': 2, 'champion-2': 2,
}


def post_fatal(post_surv):
    """Per-fight fatal hazards whose product of survivals is `post_surv`."""
    tot = sum(POST_WEIGHTS.values())
    return {s: round(1 - post_surv ** (w / tot), 5) for s, w in POST_WEIGHTS.items()} if post_surv < 1 else {}


def reach(plan, rung, R, seed):
    out = subprocess.check_output([BIN, 'report', plan, '--rung', rung, '--R', str(R), '--seed', str(seed)], text=True)
    r = {}
    for line in out.splitlines():
        t = line.split()
        if len(t) > 2 and t[1].endswith('%'):
            r[t[0]] = float(t[1][:-1]) / 100
    return r


def main():
    a = sys.argv[1:]

    def opt(n, d):
        if n in a:
            i = a.index(n)
            v = type(d)(a[i + 1])
            del a[i:i + 2]
            return v
        return d

    R, seed, k, outp = opt('--R', 200000), opt('--seed', 1), opt('--k', 6.0), opt('--out', '')
    plan = a[0]
    runs = fa.load()['runs']
    table, _, _ = fa.table(runs)
    byw = {t['window']: t for t in table}
    late = [t for t in table if t['window'] not in ('forest-brock', 'brock-misty')]
    q_pool = sum(t['deaths'] for t in late) / max(1, sum(t['at_risk'] for t in late))
    r = reach(plan, 'WR+160m', R, seed)
    fatal = {}
    print(f'pooled late death share per window {q_pool:.3f} (k = {k})')
    print(f'{"window":<18}{"runner q":>9}{"model q":>9}{"need":>8}  fatal')
    for w, (a_seg, b_seg, weights) in MODEL.items():
        t = byw[w]
        q = (t['deaths'] + (0 if w in ('forest-brock', 'brock-misty') else k * q_pool)) / (
            t['at_risk'] + (0 if w in ('forest-brock', 'brock-misty') else k))
        m = r[b_seg] / r[a_seg]
        f = min(1.0, (1 - q) / m)
        tot = sum(weights.values())
        hs = {}
        for s, wt in weights.items():
            hs[s] = 1 - math.exp(math.log(f) * wt / tot) if f < 1 else 0.0
        fatal.update({s: round(h, 4) for s, h in hs.items() if h > 0})
        print(f'{w:<18}{q:>9.3f}{1 - m:>9.3f}{1 - f:>8.3f}  ' + ' '.join(f'{s}={hs[s]:.3f}' for s in hs))
    post = opt('--post', 1.0)
    fatal.update(post_fatal(post))
    print(json.dumps(fatal))
    if outp:
        json.dump(fatal, open(outp, 'w'))


if __name__ == '__main__':
    main()
