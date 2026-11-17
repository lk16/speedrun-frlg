#!/usr/bin/env python3
"""Pickup audit of a plan: does every item the plan collects need the Itemfinder, or not?

usage: pickup_audit.py [PLAN.md]      (default: plans/CURRENT)

For each `elixirs` pickup (and the `sell` special) of the plan, `SOURCES` names the decomp source the sim's
frames stand for (map, kind, tile, item). The tile is looked up in `decompiled/data/maps/<map>/map.json`
and, for a hidden item, its `underfoot` flag is read from there: `GetInteractedBackgroundEventScript`
returns NULL on an A press for an underfoot hidden item (`field_control_avatar.c:497-500`), so only the
Itemfinder (`itemfinder.c:241-246`: standing on the tile) finds it; the Itemfinder is given by the Route 11
east-gate aide at 30 caught Kanto species (`Route11_EastEntrance_2F/scripts.inc:56-73`,
`prof_pc.c:23-29`). Every other hidden item is picked up by pressing A facing the tile from an adjacent
tile (any direction, elevation of the tile = the player's), item balls and script gifts likewise.
Exit status 1 if a pickup is an underfoot item (or `leftovers` / `ppmax`, which model underfoot items) with
no `itemfinder` pickup at or before it in the plan's list. stdlib only.
"""
import json
import os
import sys

ROOT = os.path.join(os.path.dirname(os.path.abspath(__file__)), '..', '..')
MAPS = os.path.join(ROOT, 'decompiled', 'data', 'maps')

# (kind, anchor) -> [(map, x, y, item)] candidates; the plan's frames are for one of them. Anchors are the
# plan's `after`; the tile is the object / bg event in map.json. An empty tile means a script reward.
SOURCES = {
    ('elixir', 'silph-giovanni'): [('SilphCo_5F', 13, 16, 'ELIXIR')],
    ('elixir', 'mansion'): [('PokemonMansion_B1F', 35, 5, 'ELIXIR')],
    ('leaf', 'safari-walk'): [('SafariZone_East', 36, 17, 'LeafStone')],
    ('thunder', 'powerplant-out'): [('PowerPlant', 45, 4, 'ThunderStone')],
    ('moon', 'mansion'): [('PokemonMansion_1F', 2, 21, 'MOON_STONE')],
    ('moon', 'deliver-meteorite'): [('TwoIsland_JoyfulGameCorner', None, None, 'script: Moon Stone from Lostelle\'s dad')],
    ('candy', 'warden-strength'): [('FuchsiaCity_WardensHouse', 11, 5, 'RareCandy')],
    ('candy', 'hideout-grunts'): [('RocketHideout_B3F', 12, 12, 'RareCandy')],
    ('candy', 'tower-channelers'): [('PokemonTower_6F', 5, 15, 'RareCandy')],
    ('candy', 'silph-grunts'): [('SilphCo_10F', 3, 17, 'RareCandy')],
    ('candy', 'victory-road'): [('VictoryRoad_1F', 12, 3, 'RareCandy')],
    ('candy', 'mt-moon-1f'): [('MtMoon_1F', 42, 35, 'RareCandy')],
    ('candy', 'route9'): [('Route9', 63, 2, 'RARE_CANDY')],
    ('candy', 'cycling-road'): [('Route17', 18, 83, 'RARE_CANDY')],
    ('max', 'cycling-road'): [('Route17', 14, 157, 'MAX_ELIXIR')],
    ('max', 'route23'): [('Route23', 9, 8, 'MAX_ELIXIR')],
    ('ppup', 'hideout-grunts'): [('RocketHideout_B1F', 16, 17, 'PP_UP')],
    ('ppup', 'cycling-road'): [('Route17', 4, 68, 'PP_UP')],
    ('ppup', 'celadon-tea'): [('CeladonCity', 55, 20, 'PP_UP')],
    ('calcium', 'hideout-giovanni'): [('RocketHideout_B4F', 6, 23, 'Calcium')],
    ('calcium', 'rival-silph'): [('SilphCo_7F', 0, 12, 'Calcium')],
    # modelled but not in 1067: the two underfoot items the route could use
    ('leftovers', None): [('Route16', 31, 13, 'LEFTOVERS')],
    ('ppmax', None): [('TwoIsland_CapeBrink', 16, 28, 'PP_MAX')],
    ('neverice', None): [('FourIsland_IcefallCave_B1F', 21, 7, 'NeverMeltIce')],
}
NEEDS_FINDER = {'leftovers', 'ppmax'}


def lookup(m, x, y):
    f = os.path.join(MAPS, m, 'map.json')
    if x is None or not os.path.exists(f):
        return 'script', None
    j = json.load(open(f))
    for b in j.get('bg_events', []):
        if b.get('type') == 'hidden_item' and (b['x'], b['y']) == (x, y):
            return 'hidden', bool(b.get('underfoot'))
    for o in j.get('object_events', []):
        if (o['x'], o['y']) == (x, y) and 'ITEM_BALL' in o.get('graphics_id', ''):
            return 'ball', False
    return 'NOT FOUND', None


def main():
    plan = sys.argv[1] if len(sys.argv) > 1 else None
    if plan is None:
        cur = open(os.path.join(ROOT, 'human-run', 'plans', 'CURRENT')).read().strip()
        plan = os.path.join(ROOT, 'human-run', 'plans', cur + '.md')
    p = json.loads(open(plan).read().split('```json')[1].split('```')[0])
    bad = 0
    finder = False
    for e in p.get('elixirs', []):
        k, a = e['kind'], e['after']
        if k == 'itemfinder':
            finder = True
            print(f"{a:20s} itemfinder  Route 11 aide (30 Kanto species), {e['frames']} f")
            continue
        src = SOURCES.get((k, a)) or SOURCES.get((k, None))
        if src is None:
            print(f"{a:20s} {k:10s}  UNMAPPED (add to SOURCES)")
            bad += 1
            continue
        for m, x, y, item in src:
            how, uf = lookup(m, x, y)
            need = uf is True or k in NEEDS_FINDER
            flag = 'NEEDS ITEMFINDER' if need else 'A on adjacent tile' if how == 'hidden' else how
            if need and not finder:
                flag += ' -- NO itemfinder pickup before it'
                bad += 1
            print(f"{a:20s} {k:10s}  {m} ({x},{y}) {item}: {how}{' underfoot=' + str(uf) if uf is not None else ''} -> {flag}")
    for s in p.get('specials', []):
        if s.get('kind') == 'sell':
            print(f"{s['after']:20s} sell        Route24 Nugget from the Rocket (script `Route24/scripts.inc:5-51`): no Itemfinder")
    sys.exit(1 if bad else 0)


main()
