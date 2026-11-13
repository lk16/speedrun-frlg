#!/usr/bin/env python3
"""Candidate patches for the dex economy of a WR-order plan (`route_style = "wr"`), for descend.py.

`candidates(p)` returns (kind, label, patch) triples like descend.py's own list, over the knobs that
price the 60-species requirement: evolutions as dex currency (stones, Rare Candies, lead-swap levels),
the in-game trades, candy and stone pickups, the Safari target count and area order, the Power Plant
skip, the Pewter detour, the ball rule, extra on-route hunts and where passing catches are allowed.

Every source is decomp-cited next to its catalogue entry. Evolutions are only proposed when the
pre-evolution is in the bag with enough probability at the anchor (`frlg-human bag`), so the search
does not spend evaluations on events that cannot fire. stdlib only.
"""
import copy
import json
import os
import subprocess

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.join(HERE, '..', '..')
BIN = os.environ.get('FRLG_BIN', os.path.join(ROOT, 'target', 'release', 'frlg-human'))
SCRATCH = os.environ.get('FRLG_SCRATCH', '/tmp/frlg-human-scratch')
DEX = json.load(open(os.path.join(ROOT, 'human-run', 'data', 'dex.json')))

# Where the plan can place an evolution, in route order (segment names of the WR order). The
# anchors with a Pokemon Center or a quiet moment: after Koga (Safari done), after the Power Plant /
# Mansion, after Blaine, after the Sevii trip, after Erika, before Route 22, before the Indigo door.
EVO_ANCHORS = ['koga-badge', 'blaine-badge', 'stones', 'erika-badge', 'giovanni-badge', 'victory-road']

# In-game trades (`decompiled/src/data/ingame_trades.h`, scripts in `data/maps/<map>/scripts.inc`).
# (give, get, map, anchors the trade fits, frames, version or None)
TRADES = [
    ('SPECIES_SPEAROW', 'SPECIES_FARFETCHD', 'VermilionCity_House2', ['ss-anne', 'surge-badge', 'fly-vermilion'], 2200, None),
    ('SPECIES_VENONAT', 'SPECIES_TANGELA', 'CinnabarIsland_PokemonLab_Lounge', ['stones', 'blaine-badge'], 2400, None),
    ('SPECIES_RAICHU', 'SPECIES_ELECTRODE', 'CinnabarIsland_PokemonLab_Lounge', ['stones', 'blaine-badge'], 2400, None),
    ('SPECIES_ABRA', 'SPECIES_MR_MIME', 'Route2_House', ['fly-viridian'], 2600, None),
    ('SPECIES_PONYTA', 'SPECIES_SEEL', 'CinnabarIsland_PokemonLab_ExperimentRoom', ['stones'], 2400, None),
    ('SPECIES_POLIWHIRL', 'SPECIES_JYNX', 'CeruleanCity_House3', ['bike-redeem', 'misty-badge'], 2000, None),
    # LeafGreen asks for Slowbro, FireRed for Golduck (neither is reachable before the League).
    ('SPECIES_SLOWBRO', 'SPECIES_LICKITUNG', 'Route18_EastEntrance_2F', ['cycling-road'], 2400, 'LG'),
    ('SPECIES_GOLDUCK', 'SPECIES_LICKITUNG', 'Route18_EastEntrance_2F', ['cycling-road'], 2400, 'FR'),
    # Nidoran swap at the Underground Path's north entrance (Route 5): FireRed gives Nidoran F for M,
    # LeafGreen Nidoran M for F. Nidorino/Nidorina at the Route 11 east gate house.
    ('SPECIES_NIDORAN_F', 'SPECIES_NIDORAN_M', 'UndergroundPath_NorthEntrance', ['route5-6'], 1500, 'LG'),
    ('SPECIES_NIDORAN_M', 'SPECIES_NIDORAN_F', 'UndergroundPath_NorthEntrance', ['route5-6'], 1500, 'FR'),
    ('SPECIES_NIDORINA', 'SPECIES_NIDORINO', 'Route11_EastEntrance_2F', ['koga-badge', 'stones'], 3000, 'LG'),
    ('SPECIES_NIDORINO', 'SPECIES_NIDORINA', 'Route11_EastEntrance_2F', ['koga-badge', 'stones'], 3000, 'FR'),
]

# Item pickups that feed the stock: (anchor, kind, frames, source). Rare Candy balls and hidden items
# from each map's map.json; stones from the same files and `TwoIsland_JoyfulGameCorner/scripts.inc:164-170`.
PICKUPS = [
    ('warden-strength', 'candy', 350, 'FuchsiaCity_WardensHouse/map.json:58 (item ball)'),
    ('mt-moon-1f', 'candy', 400, 'MtMoon_1F/map.json:170 (item ball)'),
    ('hideout-grunts', 'candy', 400, 'RocketHideout_B3F/map.json:58 (item ball)'),
    ('tower-channelers', 'candy', 350, 'PokemonTower_6F/map.json:72 (item ball)'),
    ('silph-grunts', 'candy', 400, 'SilphCo_10F/map.json:100 (item ball)'),
    ('victory-road', 'candy', 350, 'VictoryRoad_1F/map.json:58 (item ball)'),
    ('mt-moon-1f', 'moon', 400, 'MtMoon_1F/map.json:198 (item ball)'),
    ('mt-moon-b2f', 'moon', 300, 'MtMoon_B2F/map.json:223 (hidden)'),
    ('hideout-grunts', 'moon', 400, 'RocketHideout_B2F/map.json:58 (item ball)'),
    ('mansion', 'moon', 300, 'PokemonMansion_1F/map.json:176 (hidden)'),
    ('deliver-meteorite', 'moon', 60, 'TwoIsland_JoyfulGameCorner/scripts.inc:164-170 (forced scene)'),
    ('safari-walk', 'leaf', 700, 'SafariZone_East/map.json:72 (item ball)'),
    ('powerplant-out', 'thunder', 600, 'PowerPlant/map.json:72 (item ball)'),
]

# Species the Pewter detour adds (Old Amber from the museum scientist, `PewterCity_Museum_1F/scripts.inc:144-151`,
# revived at the Cinnabar lab like the Sevii fossil; Diglett's Cave from the Pewter side).
PEWTER = {
    'specials': [
        {'after': 'blaine-badge-and-sevii-scene', 'kind': 'gift', 'species': ['SPECIES_AERODACTYL'], 'frames': 3600, 'level': 30},
    ],
    'hunts': [
        {'after': 'fly-viridian', 'site': 'MAP_DIGLETTS_CAVE_B1F', 'area': 'land', 'steps': 60, 'detour_tiles': 50, 'stop_after': 2},
    ],
}

# Hunts the plan can add on the way (site, area, anchor, steps, stop_after).
ROUTE_HUNTS = [
    ('MAP_ROUTE3', 'land', 'route3', 40, 2),
    ('MAP_ROUTE4', 'land', 'route4', 40, 2),
    ('MAP_ROUTE9', 'land', 'route9', 40, 2),
    ('MAP_ROUTE10', 'land', 'route10-north', 40, 2),
    ('MAP_ROUTE8', 'land', 'route8-7', 40, 2),
    ('MAP_ROUTE16', 'land', 'cycling-road', 40, 2),
    ('MAP_ROUTE17', 'land', 'cycling-road', 40, 2),
    ('MAP_ROUTE22', 'land', 'route22', 40, 2),
    ('MAP_ROUTE23', 'land', 'route23', 40, 2),
    ('MAP_VICTORY_ROAD_2F', 'land', 'victory-road', 60, 3),
    ('MAP_VICTORY_ROAD_1F', 'land', 'victory-road', 60, 3),
    ('MAP_ROUTE23', 'water', 'route23', 40, 2),
    ('MAP_MT_MOON_B1F', 'land', 'mt-moon-b1f', 40, 2),
    ('MAP_ROCK_TUNNEL_1F', 'land', 'rock-tunnel', 50, 2),
    ('MAP_POKEMON_TOWER_5F', 'land', 'tower-walk', 50, 2),
    ('MAP_POKEMON_MANSION_1F', 'land', 'mansion', 60, 3),
    ('MAP_SAFARI_ZONE_WEST', 'land', 'safari-walk', 100, 2),
    ('MAP_THREE_ISLAND_BERRY_FOREST', 'land', 'berry-forest', 60, 3),
    ('MAP_THREE_ISLAND_BOND_BRIDGE', 'land', 'bond-bridge', 40, 2),
]

_cache = {}


def _plan_file(p, tag):
    os.makedirs(SCRATCH, exist_ok=True)
    path = os.path.join(SCRATCH, f'_dexknobs-{tag}.md')
    q = dict(p)
    q['name'] = 'dexknobs-' + tag
    open(path, 'w').write('```json\n' + json.dumps(q) + '\n```\n')
    return path


def segment_names(p):
    key = ('names', json.dumps(p, sort_keys=True))
    if key not in _cache:
        r = subprocess.run([BIN, 'names', _plan_file(p, 'names')], capture_output=True, text=True)
        _cache[key] = r.stdout.split()
    return _cache[key]


def bag(p, after, R=12000):
    """{national number: (share, mean catch level)} after segment `after`, no resets."""
    key = ('bag', json.dumps(p, sort_keys=True), after)
    if key in _cache:
        return _cache[key]
    r = subprocess.run([BIN, 'bag', _plan_file(p, 'bag'), '--after', after, '--R', str(R)], capture_output=True, text=True)
    out = {}
    for line in r.stdout.splitlines():
        parts = line.split()
        if len(parts) >= 6 and parts[0].isdigit() and parts[2].endswith('%'):
            out[int(parts[0])] = (float(parts[2][:-1]) / 100, float(parts[4]))
    _cache[key] = out
    return out


def _dex_table():
    sp = DEX['species']
    by_dex = {v['dex']: k for k, v in sp.items() if v.get('dex', 0)}
    return sp, by_dex


def evolution_candidates(p):
    """Events proposed per anchor: pre-evolution likely in the bag, target likely not."""
    sp, by_dex = _dex_table()
    names = set(segment_names(p))
    have = {(e['after'], e['species'], e['via'], e.get('item', '')) for e in p.get('evolutions', [])}
    out = []
    for a in EVO_ANCHORS:
        if a not in names:
            continue
        b = bag(p, a)
        for pre_name, v in sp.items():
            if not v.get('dex', 0):
                continue
            share_pre = b.get(v['dex'], (0, 0))[0]
            if share_pre < 0.15:
                continue
            for kind, param, target in v['evo']:
                t = sp.get(target)
                if not t or not t.get('dex', 0) or b.get(t['dex'], (0, 0))[0] > 0.85:
                    continue
                lv = b[v['dex']][1]
                variants = []
                if kind == 'EVO_ITEM' and param.endswith('_STONE'):
                    item = param[len('ITEM_'):-len('_STONE')]
                    variants.append(dict(via='stone', item=item, buy=item not in ('MOON',)))
                elif kind == 'EVO_LEVEL':
                    at = int(param)
                    need = 1 if lv >= at else at - lv
                    if need <= 6:
                        variants.append(dict(via='candy'))
                    if lv + 8 >= at:
                        variants.append(dict(via='level'))
                for var in variants:
                    key = (a, target, var['via'], var.get('item', ''))
                    if key in have:
                        continue
                    ev = dict(after=a, species=target, **var)
                    out.append(('evo', f"+evolution {target[8:]} via {var['via']}{' ' + var.get('item', '') if var.get('item') else ''} after {a} (pre in bag {share_pre:.0%}, Lv {lv:.0f})",
                                {'evolutions': p.get('evolutions', []) + [ev]}))
    return out


def candidates(p):
    out = []
    evs = p.get('evolutions', [])
    names = set(segment_names(p))
    # --- evolutions: drop each, move each one anchor along the route, flip stone buy.
    for i, e in enumerate(evs):
        q = copy.deepcopy(evs)
        del q[i]
        out.append(('evo', f"drop evolution {e['species'][8:]} via {e['via']}@{e['after']}", {'evolutions': q}))
        if e['after'] in EVO_ANCHORS:
            k = EVO_ANCHORS.index(e['after'])
            for d in (-1, 1):
                if 0 <= k + d < len(EVO_ANCHORS) and EVO_ANCHORS[k + d] in names:
                    q = copy.deepcopy(evs)
                    q[i]['after'] = EVO_ANCHORS[k + d]
                    out.append(('evo', f"move evolution {e['species'][8:]} {e['after']}->{EVO_ANCHORS[k + d]}", {'evolutions': q}))
        if e['via'] == 'stone':
            q = copy.deepcopy(evs)
            q[i]['buy'] = not e.get('buy', False)
            out.append(('evo', f"evolution {e['species'][8:]} buy {e.get('buy', False)}->{not e.get('buy', False)}", {'evolutions': q}))
    out += evolution_candidates(p)
    # --- trades
    version = p.get('version', 'LG')
    specials = p.get('specials', [])
    have = {(s['species'][0], s['after']) for s in specials if s.get('kind') == 'trade'}
    for give, get, mp, anchors, frames, ver in TRADES:
        if ver and ver != version:
            continue
        for a in anchors:
            if a not in names or (get, a) in have:
                continue
            if any(s['species'][0] == get and s.get('kind') == 'trade' for s in specials):
                continue
            new = copy.deepcopy(specials)
            new.append({'after': a, 'kind': 'trade', 'species': [get], 'requires': [give], 'frames': frames})
            out.append(('trade', f"trade {give[8:]} for {get[8:]} at {mp} after {a}", {'specials': new}))
    for i, s in enumerate(specials):
        if s.get('kind') == 'trade':
            q = copy.deepcopy(specials)
            del q[i]
            out.append(('trade', f"drop trade for {s['species'][0][8:]}@{s['after']}", {'specials': q}))
    # --- pickups
    cur = p.get('elixirs', [])
    have_p = {(e['after'], e['kind']) for e in cur}
    for a, kind, frames, src in PICKUPS:
        if a not in names or (a, kind) in have_p:
            continue
        out.append(('pickup', f"pick up {kind}@{a} ({src})", {'elixirs': cur + [{'after': a, 'kind': kind, 'frames': frames}]}))
    for i, e in enumerate(cur):
        if e['kind'] in ('candy', 'moon', 'leaf', 'thunder', 'fire', 'water'):
            q = copy.deepcopy(cur)
            del q[i]
            out.append(('pickup', f"drop {e['kind']}@{e['after']}", {'elixirs': q}))
    # --- version: LG or FR wild tables; the Game Corner prices and the gift levels differ
    # (`CeladonCity_GameCorner_PrizeRoom/scripts.inc:41-141`: Abra 180/120 coins, Clefairy 500/750, levels 9/7 and 8/12)
    other = 'FR' if version == 'LG' else 'LG'
    price = {'FR': {'SPECIES_ABRA': (3600, 9), 'SPECIES_CLEFAIRY': (10000, 8)},
             'LG': {'SPECIES_ABRA': (2400, 7), 'SPECIES_CLEFAIRY': (15000, 12)}}[other]
    sp2 = copy.deepcopy(specials)
    for s in sp2:
        if s.get('kind') == 'buy' and len(s['species']) == 1 and s['species'][0] in price:
            s['cost'], s['level'] = price[s['species'][0]]
    swap = {'SPECIES_NIDORAN_F': 'SPECIES_NIDORAN_M', 'SPECIES_NIDORAN_M': 'SPECIES_NIDORAN_F',
            'SPECIES_NIDORINA': 'SPECIES_NIDORINO', 'SPECIES_NIDORINO': 'SPECIES_NIDORINA'}
    for s in sp2:
        # the two Nidoran trades hand over the other form in the other version
        if s.get('kind') == 'trade' and s['species'][0] in swap and s.get('requires') and s['requires'][0] in swap:
            s['species'], s['requires'] = s['requires'], s['species']
    out.append(('version', f'version {version}->{other} (prices, levels, trades)', {'version': other, 'specials': sp2}))
    # --- how many Rare Candies the carry eats on the spot ("double candy", STRATEGY.md 5.2); the rest are
    # stock for evolutions and the League
    for n in (0, 1, 2, 3, 4):
        if p.get('candy_early_n', 99) != n:
            out.append(('candy', f"candy_early_n {p.get('candy_early_n', 99)}->{n}", {'candy_early_n': n, 'candy_early': True}))
    # --- Safari
    for n in (0, 5, 6, 7, 8, 9, 10):
        if p.get('safari_stop', 0) != n:
            out.append(('safari', f'safari_stop {p.get("safari_stop", 0)}->{n}', {'safari_stop': n}))
    hunts = p.get('hunts', [])
    idx = [i for i, h in enumerate(hunts) if h['site'].startswith('MAP_SAFARI_ZONE') and h.get('on_route')]
    for a, b_ in zip(idx, idx[1:]):
        q = copy.deepcopy(hunts)
        q[a], q[b_] = q[b_], q[a]
        out.append(('safari', f"swap Safari areas {hunts[a]['site'][9:]} and {hunts[b_]['site'][9:]}", {'hunts': q}))
    for i in idx:
        for f in (0.6, 1.5):
            q = copy.deepcopy(hunts)
            q[i]['steps'] = max(20, int(round(hunts[i]['steps'] * f)))
            out.append(('safari', f"Safari {hunts[i]['site'][9:]} steps x{f}", {'hunts': q}))
    # --- Power Plant skip: Raichu then needs a bought Thunder Stone.
    if p.get('skip_powerplant'):
        q = copy.deepcopy(evs)
        for e in q:
            if e.get('item') == 'THUNDER':
                e['buy'] = True
        out.append(('powerplant', 'bring the Power Plant detour back', {'skip_powerplant': False}))
    else:
        q = copy.deepcopy(evs)
        for e in q:
            if e.get('item') == 'THUNDER':
                e['buy'] = True
        out.append(('powerplant', 'skip the Power Plant (Thunder Stone bought if still wanted)',
                    {'skip_powerplant': True, 'evolutions': q}))
    # --- Pewter detour bundle
    has_aero = any('SPECIES_AERODACTYL' in s['species'] for s in specials)
    if has_aero:
        sp2 = [s for s in specials if 'SPECIES_AERODACTYL' not in s['species']]
        h2 = [h for h in hunts if h['site'] != 'MAP_DIGLETTS_CAVE_B1F' or h['after'] != 'fly-viridian']
        sp2 = [s for s in sp2 if not (s.get('kind') == 'trade' and 'SPECIES_MR_MIME' in s['species'])]
        out.append(('pewter', 'skip the Pewter detour (no Aerodactyl, Diglett, Mr. Mime)', {'specials': sp2, 'hunts': h2}))
    else:
        sp2 = copy.deepcopy(specials) + copy.deepcopy(PEWTER['specials'])
        if not any('SPECIES_MR_MIME' in s['species'] for s in specials):
            sp2.append({'after': 'fly-viridian', 'kind': 'trade', 'species': ['SPECIES_MR_MIME'], 'requires': ['SPECIES_ABRA'], 'frames': 2600})
        out.append(('pewter', 'add the Pewter detour (Aerodactyl, Diglett, Mr. Mime)',
                    {'specials': sp2, 'hunts': copy.deepcopy(hunts) + copy.deepcopy(PEWTER['hunts'])}))
    # --- ball rule
    for pf, ub in ((0, 0), (255, 0), (190, 0), (120, 0), (190, 90), (255, 60), (190, 60)):
        if (p.get('poke_from', 0), p.get('ultra_below', 0)) != (pf, ub):
            out.append(('ball', f'ball rule poke_from {pf} ultra_below {ub}', {'poke_from': pf, 'ultra_below': ub}))
    # --- on-route hunts
    have_h = {(h['after'], h['site'], h['area']) for h in hunts}
    for site, area, a, steps, stop in ROUTE_HUNTS:
        if a not in names or (a, site, area) in have_h:
            continue
        out.append(('hunt', f'add hunt {site[4:]} {area} after {a} (stop after {stop})',
                    {'hunts': copy.deepcopy(hunts) + [dict(after=a, site=site, area=area, steps=steps, detour_tiles=0, on_route=True, stop_after=stop)]}))
    for i, h in enumerate(hunts):
        if h.get('stop_after', 0) > 0:
            for d in (-1, 1):
                v = h['stop_after'] + d
                if v >= 1:
                    q = copy.deepcopy(hunts)
                    q[i]['stop_after'] = v
                    out.append(('hunt', f"hunts[{i}] {h['site'][4:]} stop_after {h['stop_after']}->{v}", {'hunts': q}))
    # --- where passing encounters may be caught
    off = p.get('pass_off', [])
    for sname in ('route1-encounters', 'route2-encounters', 'forest-encounters', 'route3', 'mt-moon-1f', 'mt-moon-b1f', 'mt-moon-b2f',
                  'route4', 'nugget-bridge', 'route6-encounters', 'route9', 'rock-tunnel', 'tower-encounters', 'cycling-road'):
        if sname in names:
            new = [x for x in off if x != sname] if sname in off else off + [sname]
            out.append(('passoff', f"pass_off {'-' if sname in off else '+'}{sname}", {'pass_off': new}))
    return out


if __name__ == '__main__':
    import sys
    plan = sys.argv[1]
    p = json.loads(open(plan).read().split('```json')[1].split('```')[0])
    for kind, label, patch in candidates(p):
        print(kind, '|', label)
