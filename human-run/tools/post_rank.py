#!/usr/bin/env python3
"""Plan ranking and post-door reset retune under the evidence-based post-door hazards (2026-11-15). stdlib only.

usage:
  post_rank.py rank OUT.json RUNG R SEEDS [PLAN ...]     plans x post-door scenarios, paired seeds (comma list)
  post_rank.py aggr OUT.json RUNG R SEEDS PLAN           reset_aggr of the post-door groups x levels x scenarios

Scenarios replace the post-door `fatal` map of each plan: `own` (the plan's, 801: S 0.183), `ev` (802's evidence map, S 0.205),
`ev-lo` / `ev-hi` (the evidence map rescaled to S 0.06 / 0.5, the 90% interval of the joint posterior (post_joint.py)), `S0.1` / `S0.35`.
Older plans lack the honest-model keys and are rebased on 608 (sens.rebase; the ROBUSTNESS.md convention) before the post-door map is applied.
"""
import json, os, sys

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
import sens  # noqa: E402

CUR = 'human-run'
EV = sens.load_plan('802-post-evidence')['fatal']
SCEN = ['own', 'ev', 'ev-lo', 'ev-hi', 'S0.1', 'S0.35']
GROUPS = {
    'e4r1': ['indigo-heal', 'e4-transit-lorelei', 'lorelei', 'e4-transit-bruno', 'bruno', 'e4-transit-agatha', 'agatha', 'e4-transit-lance', 'lance', 'champion', 'hall-of-fame-1'],
    'chain': ['fly-pallet-2', 'pallet-to-oak', 'oak-national-dex', 'to-one-island', 'celio-ruby-request', 'kindle-road-in', 'mt-ember-grunt-1', 'mt-ember-grunt-2', 'ruby-path',
              'kindle-road-out', 'mt-ember-ruby-climb', 'celio-rainbow-pass', 'sail-to-four', 'icefall-cave-walk', 'icefall-grunt', 'icefall-ice-slides', 'icefall-cave',
              'icefall-cave-out', 'five-island-approach', 'meadow-grunt', 'five-island-resort', 'warehouse-grunt-1', 'warehouse-admin-1', 'warehouse-admin-2',
              'warehouse-interior', 'warehouse-gideon', 'sapphire-and-sail-back', 'celio-network-machine', 'fly-cerulean', 'cerulean-cave', 'mewtwo', 'tm-fire_blast@mewtwo',
              'tm-earthquake@mewtwo', 'fly-indigo', 'league-entry'],
    'r2': ['lorelei-2', 'e4-transit-bruno-2', 'bruno-2', 'e4-transit-agatha-2', 'agatha-2', 'e4-transit-lance-2', 'lance-2', 'e4-transit-champion-2', 'champion-2', 'hall-of-fame-2'],
}


def scenario(p, name):
    q = json.loads(json.dumps(p))
    if name == 'own':
        return q
    q['fatal'] = dict(q.get('fatal', {}))
    q['fatal'].update({k: v for k, v in EV.items() if k in sens.POST})
    if name == 'ev':
        return q
    tgt = {'ev-lo': 0.06, 'ev-hi': 0.5, 'S0.1': 0.1, 'S0.35': 0.35}[name]
    return sens.patch(q, 'S', tgt)


def seeds_of(s):
    return [int(x) for x in s.split(',')]


def cmd_rank(out, rung, R, seeds, plans):
    cur = sens.load_plan('608-evo581-habit')  # NOT 801: it would hand 801's route_variants / recalibrated manip to every older plan
    res = []
    for sc in SCEN:
        for name in plans:
            base = sens.rebase(sens.load_plan(name), cur)
            q = scenario(base, sc)
            for sd in seeds_of(seeds):
                eht, fin = sens.run_eval(q, f'rk_{name[:3]}_{sc}_{sd}'.replace('.', '_'), rung, R, sd)
                r = dict(plan=name, scen=sc, seed=sd, eht=eht, fin=fin)
                res.append(r)
                print(r, flush=True)
                json.dump(res, open(out, 'w'))


def cmd_aggr(out, rung, R, seeds, plan):
    cur = sens.load_plan('801-warden-recalib')
    base = sens.rebase(sens.load_plan(plan), cur)
    levels = [0.4, 0.55, 0.7, 0.8, 0.9, 1.0]
    res = []
    for sc in SCEN[:4]:
        b = scenario(base, sc)
        for g, segs in GROUPS.items():
            for v in levels:
                q = json.loads(json.dumps(b))
                for s in segs:
                    q['reset_aggr_at'][s] = v
                for sd in seeds_of(seeds):
                    eht, fin = sens.run_eval(q, f'ag_{g}_{v}_{sc}_{sd}'.replace('.', '_'), rung, R, sd)
                    r = dict(scen=sc, group=g, level=v, seed=sd, eht=eht, fin=fin)
                    res.append(r)
                    print(r, flush=True)
                    json.dump(res, open(out, 'w'))


if __name__ == '__main__':
    c = sys.argv[1]
    if c == 'rank':
        cmd_rank(sys.argv[2], sys.argv[3], int(sys.argv[4]), sys.argv[5], sys.argv[6:])
    elif c == 'aggr':
        cmd_aggr(sys.argv[2], sys.argv[3], int(sys.argv[4]), sys.argv[5], sys.argv[6])
