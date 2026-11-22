#!/usr/bin/env python3
"""honesty2_mk.py PARENT NAME OUT STEP[,STEP...]: plan variants for model-honesty-2 (controller-2 bugs, plans 1400-1419).

Journal human-run/journal/2026-11-22-*-model-honesty-2.md. Steps are listed in the plan header. stdlib only.
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
    if st == 'strength':
        # MODEL CHANGE: Victory Road needs a Strength carrier (tmhm_learnsets.h; VictoryRoad_1F/map.json boulders).
        p['strength_carrier'] = True
    elif st == 'stones':
        # MODEL CHANGE: stones only after a Celadon Department Store visit (CeladonCity_DepartmentStore_4F
        # sells Fire/Thunder/Water/Leaf); Moon Stone never bought. celadon-tea's X-item trip (5F) passes 4F.
        p['stone_shops'] = ['celadon-tea', 'fly-celadon-2', 'fly-celadon-3']
    elif st == 'stones2':
        # Variant: only the Celadon visits the controller used for shopping (fly-celadon-2 on).
        p['stone_shops'] = ['fly-celadon-2', 'fly-celadon-3']
    elif st == 'notm13':
        # PLAN FIX: TM13 Ice Beam is a Game Corner prize (4000 coins = 80000 money,
        # CeladonCity_GameCorner_PrizeRoom/scripts.inc:264-266); the sim never has the money, so the teach is
        # dead weight that sent the controller looking for it.
        p['tms'] = [x for x in p['tms'] if x['mv'] != 'MOVE_ICE_BEAM']
    elif st == 'prize13':
        for x in p['tms']:
            if x['mv'] == 'MOVE_ICE_BEAM':
                x['prize'] = True
    elif st.startswith('marts'):
        # MODEL CHANGE + PLAN: balls (and, with potion_bag_from, potions) only at mart stops, topped up to
        # targets. Stocks from decompiled/data/maps/*_Mart/scripts.inc (Cerulean Poke, Celadon 2F Great,
        # Lavender Great, Fuchsia/Cinnabar Great+Ultra, Viridian Poke only).
        nb = int(st[5:] or 20)
        p['marts'] = [
            {'after': 'persim-and-potions', 'poke': 10, 'topup': True},   # Cerulean Mart
            {'after': 'xbuy@celadon-tea', 'great': nb, 'topup': True},    # Celadon Dept. 2F
            {'after': 'walk-lavender', 'great': nb, 'topup': True},       # Lavender Mart (tower)
            {'after': 'koga-badge', 'great': nb, 'topup': True},          # Fuchsia Mart
            {'after': 'fly-lavender-2', 'great': nb, 'topup': True},      # Lavender Mart before the Power Plant
            {'after': 'surf-cinnabar', 'great': nb, 'topup': True},       # Cinnabar Mart (Mansion, Sevii)
            {'after': 'xbuy@fly-celadon-2', 'great': nb, 'topup': True},  # Celadon Dept. 2F
            {'after': 'fly-viridian', 'poke': nb, 'topup': True},         # Viridian Mart (Diglett, Routes 22/23, VR)
        ]
    elif st.startswith('bag'):
        # MODEL CHANGE: finite potion bag from the Koga badge on, bought at the mart stops (Fuchsia Super,
        # Lavender Super, Cinnabar Hyper, Celadon 2F Super, Saffron Hyper, Viridian Potion). bagN = top-up target.
        n = int(st[3:] or 10)
        p['potion_bag_from'] = 'koga-badge'
        stops = {'koga-badge', 'fly-lavender-2', 'surf-cinnabar', 'xbuy@fly-celadon-2', 'fly-viridian'}
        for m in p['marts']:
            if m['after'] in stops:
                m['potions'] = n
        # Saffron Mart before Silph (Hyper Potions, SaffronCity_Mart/scripts.inc).
        p['marts'].append({'after': 'silph-walk', 'potions': n, 'topup': True})
    elif st.startswith('ppups='):
        # MODEL CHANGE: PP Ups on the carry, in this order.
        p['carry_pp_ups'] = ['MOVE_' + m.upper() for m in st[6:].split(':')]
    else:
        sys.exit('unknown step ' + st)
p['name'] = name
open(out, 'w').write(f'# Plan {name}\n\nParent: `{parent_name}`. Steps: {steps} (`human-run/tools/honesty2_mk.py`).\n\n'
                     '```json\n' + json.dumps(p, indent=1, ensure_ascii=False) + '\n```\n')
