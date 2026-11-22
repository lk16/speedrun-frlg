#!/usr/bin/env python3
"""wr_faithful_mk.py PARENT NAME OUT STEP[,STEP...]: plan variants for the WR-faithful work (prefix wrf-)."""
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
    if st == 'wrlists':
        # Every trainer the WR VOD shows from Route 3 to the League (10 s contact sheets, 2-4 s where
        # needed; parties identified in decompiled/src/data/trainer_parties.h). Journal 2026-11-22 wr-faithful.
        fl = p['fight_lists']
        fl['route3-trainers'] = ['TRAINER_BUG_CATCHER_COLTON', 'TRAINER_BUG_CATCHER_GREG',
                                 'TRAINER_BUG_CATCHER_JAMES']
        for x in p['extra_fights']:
            if x['name'] == 'mt-moon-extra':
                x['scripts'] = ['MtMoon_1F_EventScript_Robby']
        fl['cerulean-gym-trainer'] = []
        fl['route25-trainers'] = ['TRAINER_HIKER_WAYNE', 'TRAINER_PICNICKER_KELSEY',
                                  'TRAINER_CAMPER_FLINT', 'TRAINER_LASS_HALEY']
        fl['route6-trainers'] = ['TRAINER_CAMPER_JEFF']
        fl['rock-tunnel-trainers'] = [
            'TRAINER_PICNICKER_ALICIA', 'TRAINER_BUG_CATCHER_CONNER', 'TRAINER_POKEMANIAC_ASHTON',
            'TRAINER_POKEMANIAC_WINSTON', 'TRAINER_PICNICKER_MARTHA', 'TRAINER_HIKER_ALLEN',
            'TRAINER_HIKER_LENNY', 'TRAINER_PICNICKER_SOFIA', 'TRAINER_HIKER_DUDLEY',
            'TRAINER_PICNICKER_DANA']
        fl['route8-trainers'] = ['TRAINER_GAMER_RICH']
        fl['hideout-grunts'] = ['TRAINER_TEAM_ROCKET_GRUNT_7', 'TRAINER_TEAM_ROCKET_GRUNT_18',
                                'TRAINER_TEAM_ROCKET_GRUNT_16', 'TRAINER_TEAM_ROCKET_GRUNT_17']
        fl['bond-bridge-nikki'] = []
        fl['celadon-gym-trainers'] = ['TRAINER_BEAUTY_TAMIA', 'TRAINER_BEAUTY_LORI']
    elif st == 'candywr':
        # Eat the candies where the WR does (video: Lv18 -> 19 in Cerulean before Misty, Lv33 -> 35 in the
        # hideout before Giovanni). A candy is worth the exp gap to the next level when eaten.
        for e in p['elixirs']:
            if e['kind'] == 'candy' and e['after'] == 'mt-moon-1f':
                e['after'] = 'cerulean-heal'
            elif e['kind'] == 'candy' and e['after'] == 'route9':
                e['after'] = 'hideout-grunts'
        # The hidden Rare Candy in Cerulean City (decompiled/data/maps/CeruleanCity/map.json, x 18 y 7,
        # FLAG_HIDDEN_ITEM_CERULEAN_CITY_RARE_CANDY): the WR enters Misty at Lv20 after Lv18 on Route 4
        # (video 1515 Lv18, 1569 Lv19, 1597 Lv20), i.e. two candies in Cerulean. 350 frames: guess.
        p['elixirs'].append({'after': 'cerulean-heal', 'kind': 'candy', 'frames': 350})
    elif st == 'candy6':
        # A third candy before Giovanni 1: the WR is Lv33 after the last hideout grunt (video 3784) and a
        # Blastoise (Lv36) at Giovanni (3864) with no fight between, i.e. three candies in the hideout.
        # The Route 6 hidden one (decompiled/data/maps/Route6/map.json x 19 y 5, on the WR's Route 6 walk)
        # is held to the hideout. 350 frames: guess.
        p['elixirs'].append({'after': 'hideout-grunts', 'kind': 'candy', 'frames': 350})
    elif st == 'warden':
        # The WR gives the Gold Teeth after Erika (video 8124 HM04 after TM19 at 8094).
        p['route_variants'] = [v for v in p['route_variants'] if v != 'warden_early']
    elif st == 'evos':
        # The WR evolves with stones / candy after the Warden (video 8144-8234: Nidoking, Nidoqueen, Clefable).
        for e in p['evolutions']:
            if e['after'] == 'erika-badge' or (e['after'] == 'deliver-meteorite' and e.get('item') == 'MOON'):
                e['after'] = 'warden-strength'
    elif st == 'nolee':
        p['specials'] = [x for x in p['specials'] if 'SPECIES_HITMONLEE' not in x['species']]
    elif st.startswith('early'):
        n = int(st[5:] or 5)
        p['early_balls'] = n
        p['repel_on'] = [r for r in p['repel_on'] if r != 'route3']
        p['hunts'] = [
            {"after": "route3", "site": "MAP_ROUTE3", "area": "land", "steps": 40, "detour_tiles": 0,
             "on_route": True, "stop_after": 1},
            {"after": "route4", "site": "MAP_ROUTE4", "area": "land", "steps": 40, "detour_tiles": 0,
             "on_route": True, "stop_after": 2},
        ] + p['hunts']
    elif st.startswith('north'):
        # The WR hunts the Safari North (Rhyhorn 26, Nidoran F / Nidorina / Nidorino 30, Exeggcute 27: video
        # 4944-5264) and the West (Gold Teeth), 7 species: safari_stop 7.
        k = int(st[5:] or 120)
        for h in p['hunts']:
            if h['site'] == 'MAP_SAFARI_ZONE_EAST':
                h['site'] = 'MAP_SAFARI_ZONE_NORTH'
                h['steps'] = k
        p['safari_stop'] = 7
    elif st == 'dig':
        if 'dig_after_bill' not in p['route_variants']:
            p['route_variants'] = p['route_variants'] + ['dig_after_bill']
    elif st.startswith('candy'):
        # Candies eaten on pickup (WR: the Mt. Moon one at Cerulean, two in the hideout before Giovanni).
        p['candy_early_n'] = int(st[5:])
    elif st == 'cost':
        p['trainer_cost'] = [838, 579, 223]
        p.pop('trainer_fit', None)
    elif st.startswith('costx'):
        # FIT: the WR's per-fight time over the played run's (Alicia 56 vs 62 s; journal 2026-11-22-17-30).
        f = float(st[5:])
        p['trainer_cost'] = [round(c * f) for c in (838, 579, 223)]
        p.pop('trainer_fit', None)
    elif st.startswith('tf'):
        p['trainer_fit'] = float(st[2:])
        p.pop('trainer_cost', None)
    elif st.startswith('mc'):
        p['mewtwo_candies'] = int(st[2:])
    elif st.startswith('win='):
        # win=a,b,c,d,e,f risk window powers in order
        vals = [float(x) for x in st[4:].split(':')]
        for w, v in zip(p['risk_windows'], vals):
            w['power'] = v
    elif st.startswith('set:'):
        k, v = st[4:].split('=', 1)
        p[k] = json.loads(v)
    else:
        raise SystemExit('unknown step ' + st)
p['name'] = name
with open(out, 'w') as f:
    f.write(f"# Plan {name}\n\nParent: `{parent_name}`. Steps: {steps}.\n\n```json\n")
    f.write(json.dumps(p, indent=1))
    f.write("\n```\n")
