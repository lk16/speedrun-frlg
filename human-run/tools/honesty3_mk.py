#!/usr/bin/env python3
"""honesty3_mk.py PARENT NAME OUT STEP[,STEP...]: plan variants for model-honesty-3 (controller-v3 bugs, plans 1600-1619).

Journal human-run/journal/2026-11-23-*-model-honesty-3.md. Each step is a MODEL change or a PLAN response (labelled
below). stdlib only.
"""
import json
import os
import sys

ROOT = os.path.join(os.path.dirname(os.path.abspath(__file__)), '..', '..')
par, name, out, steps = sys.argv[1:5]
path = par if os.path.exists(par) else os.path.join(ROOT, 'human-run/plans', par + '.md')
t = open(path).read()
p = json.loads(t.split('```json')[1].split('```')[0])
parent_name = p['name']
for st in steps.split(','):
    if st == 'levelevo':
        # MODEL: a `level` evolution of a non-fighting mon takes Rare Candies (Cmd_getexp pays sent-in mons only,
        # battle_script_commands.c:3148-3175; EVO_LEVEL param <= level, pokemon.c:5076-5078).
        p['level_evo_honest'] = True
    elif st.startswith('levelcap='):
        # PLAN: at most N candies per level evolution.
        p['level_evo_max_candies'] = int(st[9:])
    elif st == 'droplevel':
        # PLAN: drop every `level` evolution from the route.
        p['evolutions'] = [e for e in p['evolutions'] if e.get('via') != 'level']
    elif st == 'westonly':
        # PLAN: Safari West only (the East hunt dropped, its steps given to the West hunt).
        east = next(h for h in p['hunts'] if h['site'] == 'MAP_SAFARI_ZONE_EAST')
        west = next(h for h in p['hunts'] if h['site'] == 'MAP_SAFARI_ZONE_WEST')
        west['steps'] += east['steps']
        p['hunts'] = [h for h in p['hunts'] if h is not east]
    elif st == 'westfirst':
        # PLAN: the controller's order: West, then East.
        hs = p['hunts']
        ie = next(i for i, h in enumerate(hs) if h['site'] == 'MAP_SAFARI_ZONE_EAST')
        iw = next(i for i, h in enumerate(hs) if h['site'] == 'MAP_SAFARI_ZONE_WEST')
        hs[ie], hs[iw] = hs[iw], hs[ie]
    elif st == 'riskpaid':
        # MODEL: the risk lane does not inherit the timing lane's free pre-fight potions (Torrent entry heal).
        p['risk_entry_paid'] = True
    elif st == 'kogacenter':
        # PLAN: Fuchsia Pokemon Center heal after the Safari, before the gym (runs in the Torrent phase).
        if 'safari-walk' not in p['extra_heals']:
            p['extra_heals'].append('safari-walk')
        p['torrent_heals'] = sorted(set(p.get('torrent_heals', []) + ['safari-walk']))
    elif st == 'kogabadgeheal':
        # PLAN: the Fuchsia Center heal after the badge actually runs (the controller's rule).
        p['torrent_heals'] = sorted(set(p.get('torrent_heals', []) + ['koga-badge']))
    elif st == 'frprice':
        # MODEL: the League heal item acts as a Full Restore in the engine (battle.rs Action::FullRestore cures
        # status), so it costs a Full Restore: 3000 (items.json; Max Potion 2500 cannot cure).
        p['league_heal_price'] = 3000
    elif st.startswith('c2q='):
        # PLAN: Champion-2 X queue, e.g. c2q=special:attack:attack (the WR's: league-wr-r2.md).
        p['fight_tactics']['champion-2']['x'] = st[4:].split(':')
    else:
        sys.exit('unknown step ' + st)
p['name'] = name
open(out, 'w').write(f'# Plan {name}\n\nParent: `{parent_name}`. Steps: {steps} (`human-run/tools/honesty3_mk.py`).\n\n'
                     'The step-by-step description, citations and branch rules in `001-crude.md` still apply except\n'
                     'where the parameters below differ; `frlg-human` reads only the block below.\n\n'
                     '```json\n' + json.dumps(p, indent=1, ensure_ascii=False) + '\n```\n')
