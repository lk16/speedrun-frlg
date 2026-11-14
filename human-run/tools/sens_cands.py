#!/usr/bin/env python3
"""Policy candidates of CURRENT across the calibration extremes (ROBUSTNESS.md section 4).
usage: sens_cands.py OUT.json RUNG R SEED     (writes the same row format as `sens.py grid`)

Each candidate is a patch of plan 608 (manip tier, salvage habit, Torrent, League heals);
each scenario is one calibration level of `sens.py patch`. Stdlib only."""
import json, os, re, sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import sens

CANDS = {
    'base': {},
    'tier-loose': {'manip': {'min_spe': 16, 'min_hp': 8}},
    'tier-tight': {'manip': {'min_spe': 24, 'min_hp': 16}},
    'play-on': {'no_salvage_until': ''},
    'torrent-heal-0.5': {'torrent_heal_below': 0.5},
    'league-heals-14': {'league_heals': 14},
    'ball-batch-40': {'ball_batch': 40},
}
SCEN = [('none', 0), ('p_hit', 0.55), ('p_hit', 0.76), ('mistake', 1.0), ('trainer_fit', 0.86), ('quiet', 0), ('S', 0.10), ('wall', 1.3), ('prefatal', 1.5)]


def main(out, rung, R, seed):
    base = sens.load_plan('608-evo581-habit')
    res = []
    for name, ch in CANDS.items():
        cand = json.loads(json.dumps(base))
        for k, v in ch.items():
            if isinstance(v, dict):
                cand[k] = dict(cand.get(k, {}), **v)
            else:
                cand[k] = v
        for knob, v in SCEN:
            tag = re.sub(r'[^A-Za-z0-9]+', '_', f'k_{name}_{knob}_{v}')
            eht, fin = sens.run_eval(sens.patch(cand, knob, v), tag, rung, R, seed)
            r = dict(plan=name, knob=knob, level=v, eht=eht, fin=fin)
            res.append(r)
            print(r, flush=True)
            json.dump(res, open(out, 'w'))


if __name__ == '__main__':
    main(sys.argv[1], sys.argv[2], int(sys.argv[3]), int(sys.argv[4]))
