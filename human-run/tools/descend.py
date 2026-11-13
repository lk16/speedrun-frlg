#!/usr/bin/env python3
"""Coordinate descent over the plan's knobs, following human-run/PROMPT.md's comparison rules.

usage: descend.py [--max-accepts N] [--passes N] [--only KIND,...]

Starts from plans/CURRENT. For every candidate patch (one knob changed) runs try.py against the
current plan (seed 1); on ACCEPT it re-runs on seed 2 to filter chance wins (many candidates are
tried, so a lone 95% win is not enough), and only if both accept does it promote the plan
(promote.py), commit and continue from it. Every try is logged in scores.jsonl by try.py.
Stops when a full pass yields no accepted plan. stdlib only.
"""
import json, os, subprocess, sys, copy

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.join(HERE, '..', '..')
PLANS = os.path.join(ROOT, 'human-run', 'plans')
SCRATCH = os.environ.setdefault('FRLG_SCRATCH', '/tmp/frlg-human-scratch')
SPLITS = ['brock', 'misty-badge', 'surge-badge', 'erika-badge', 'koga-badge', 'sabrina-badge',
          'blaine-badge-and-sevii-scene', 'giovanni-badge', 'indigo-heal', 'hall-of-fame-1',
          'oak-national-dex', 'celio-network-machine']

# Extra splits were tried (extra_splits, aggr 0.7: no resets triggered; aggr 1.0: slightly worse), so none here.
EXTRA_SPLIT_CANDIDATES = []

args = sys.argv[1:]
def opt(name, default):
    if name in args:
        i = args.index(name)
        v = args[i + 1]
        return type(default)(v)
    return default
max_accepts = opt('--max-accepts', 50)
max_passes = opt('--passes', 3)
only = opt('--only', '').split(',') if '--only' in args else None
cap = opt('--cap', 400000)
POINTER = opt('--pointer', 'CURRENT')
FIXED_RUNG = opt('--rung', '')
os.environ['FRLG_POINTER'] = POINTER


def current():
    return open(os.path.join(PLANS, POINTER)).read().strip()


def load(name):
    return json.loads(open(os.path.join(PLANS, name + '.md')).read().split('```json')[1].split('```')[0])


def next_number():
    return max(int(f.split('-')[0]) for f in os.listdir(PLANS) if f[:3].isdigit()) + 1


_SEGS = None


def grass_segments():
    global _SEGS
    if _SEGS is None:
        try:
            plan = os.path.join(PLANS, current() + '.md')
            r = subprocess.run([os.path.join(ROOT, 'target', 'release', 'frlg-human'), 'segs', plan],
                               capture_output=True, text=True)
            _SEGS = [l.split()[0] for l in r.stdout.splitlines() if l.strip()]
        except Exception:
            _SEGS = []
    return _SEGS


def candidates(p):
    """(kind, label, patch) for single-knob changes around plan params p."""
    out = []
    fill = p.get('fill', [])
    for i, h in enumerate(fill):
        for f in (0.6, 1.5):
            q = copy.deepcopy(fill)
            q[i]['steps'] = max(8, int(round(h['steps'] * f)))
            if q[i]['steps'] != h['steps']:
                out.append(('fillsteps', f"fill[{i}] {h['site']} {h['area']} steps x{f}", {'fill': q}))
        for f in ((0.5, 1.6) if not p.get('detour_map') else ()):
            q = copy.deepcopy(fill)
            q[i]['detour_tiles'] = int(round(h.get('detour_tiles', 0) * f))
            if q[i]['detour_tiles'] != h.get('detour_tiles', 0):
                out.append(('detour', f"fill[{i}] {h['site']} detour x{f}", {'fill': q}))
    for i in range(len(fill) - 1):
        q = copy.deepcopy(fill)
        q[i], q[i + 1] = q[i + 1], q[i]
        out.append(('order', f"swap fill[{i}]={fill[i]['site']} {fill[i]['area']} with fill[{i+1}]", {'fill': q}))
    for i, h in enumerate(fill):
        q = copy.deepcopy(fill)
        del q[i]
        out.append(('dropfill', f"drop fill[{i}] {h['site']} {h['area']}", {'fill': q}))
    extra = list(p.get('extra_splits', []))
    for name in EXTRA_SPLIT_CANDIDATES:
        new = [x for x in extra if x != name] if name in extra else extra + [name]
        out.append(('split', f"extra split {'-' if name in extra else '+'}{name}", {'extra_splits': new}))
    aggr = p.get('reset_aggr_at', {})
    for s in SPLITS + extra:
        cur = aggr.get(s, p.get('reset_aggr', 0.0))
        for d in (-0.2, 0.2):
            v = round(min(1.0, max(0.0, cur + d)), 2)
            if v != cur:
                q = dict(aggr)
                q[s] = v
                out.append(('aggr', f"aggr at {s} {cur}->{v}", {'reset_aggr_at': q}))
    for d in (-0.1, 0.1):
        out.append(('aggr', f"global aggr {p.get('reset_aggr', 0)}->{round(p.get('reset_aggr', 0) + d, 2)}",
                    {'reset_aggr': round(p.get('reset_aggr', 0) + d, 2)}))
    for d in (-1, 1):
        out.append(('starter', f"starter_min_total {p['starter_min_total']}->{p['starter_min_total'] + d}",
                    {'starter_min_total': p['starter_min_total'] + d}))
        out.append(('fight', f"fight_until_level {p['fight_until_level']}->{p['fight_until_level'] + d}",
                    {'fight_until_level': p['fight_until_level'] + d}))
    sm = p.get('starter_min_stats', [0] * 6)
    for k in range(6):
        for d in (-1, 1):
            if sm[k] + d >= 0:
                q = list(sm)
                q[k] += d
                out.append(('starter', f"starter_min_stats[{k}] {sm[k]}->{q[k]}", {'starter_min_stats': q}))
    for i, h in enumerate(p.get('hunts', [])):
        for f in (0.5, 2.0):
            q = copy.deepcopy(p['hunts'])
            q[i]['steps'] = max(8, int(round(h['steps'] * f)))
            out.append(('hunt', f"hunts[{i}] {h['site']} steps x{f}", {'hunts': q}))
        q = copy.deepcopy(p['hunts'])
        del q[i]
        out.append(('hunt', f"drop hunts[{i}] {h['site']}", {'hunts': q}))
    for i, sp in enumerate(p.get('specials', [])):
        q = copy.deepcopy(p['specials'])
        del q[i]
        out.append(('special', f"drop special {sp['kind']} {sp['species']}", {'specials': q}))
    for d in (-10, 10):
        v = p.get('pass_min_catch', 0) + d
        if v >= 0:
            out.append(('catch', f"pass_min_catch {p.get('pass_min_catch', 0)}->{v}", {'pass_min_catch': v}))
    for ball in ('poke', 'great', 'ultra'):
        if p.get('ball', 'poke') != ball:
            out.append(('ball', f'ball {p.get("ball", "poke")}->{ball}', {'ball': ball}))
    if p.get('money_model'):
        for b in (5, 10, 20, 40):
            if p.get('ball_batch', 10) != b:
                out.append(('ball', f'ball_batch {p.get("ball_batch", 10)}->{b}', {'ball_batch': b}))
    for k in ('weaken', 'mewtwo', 'teleport_abra', 'bike', 'detour_bike', 'hunt_bike'):
        out.append(('toggle', f'{k} {p.get(k)}->{not p.get(k)}', {k: not p.get(k)}))
    if p.get('safari_model'):
        for r in (0, 1):
            if p.get('safari_rocks', 0) != r:
                out.append(('safari', f'safari_rocks {p.get("safari_rocks", 0)}->{r}', {'safari_rocks': r}))
    # Extra mainland fill sites (sitescan.py ranking; Cerulean Cave and the Sevii islands open only
    # after the Oak check, `human-run/journal`), tried at the front and the middle of the list.
    present = {(h['site'], h.get('area', 'land')) for h in fill}
    for site, area in (('MAP_SEAFOAM_ISLANDS_B2F', 'land'), ('MAP_SEAFOAM_ISLANDS_B3F', 'land'),
                       ('MAP_SEAFOAM_ISLANDS_B1F', 'land'), ('MAP_ROUTE18', 'land'), ('MAP_ROUTE21_NORTH', 'land'),
                       ('MAP_ROUTE16', 'land'), ('MAP_ROUTE14', 'land'), ('MAP_POKEMON_TOWER_7F', 'land'),
                       ('MAP_ROUTE22', 'fish'), ('MAP_ROUTE23', 'fish'), ('MAP_ROUTE25', 'fish'),
                       ('MAP_VICTORY_ROAD_2F', 'land'), ('MAP_VIRIDIAN_CITY', 'fish'), ('MAP_ROUTE15', 'land'),
                       ('MAP_FUCHSIA_CITY', 'fish'), ('MAP_VERMILION_CITY', 'fish'), ('MAP_PALLET_TOWN', 'fish')):
        if (site, area) in present:
            continue
        for pos in (0, len(fill) // 2):
            q = copy.deepcopy(fill)
            q.insert(pos, dict(after='', site=site, area=area, steps=80 if area == 'land' else 25, detour_tiles=50))
            out.append(('addsite', f'add {site} {area} at position {pos}', {'fill': q}))
    # In-game trades not yet in the plan (decompiled/src/data/ingame_trades.h; Mr. Mime for an Abra at the
    # Route 2 house, Nidoran F for a Nidoran M at the Underground Path north entrance).
    have_sp = {x['species'][0] for x in p.get('specials', [])}
    for sp, req, after, frames in (('SPECIES_MR_MIME', 'SPECIES_ABRA', 'fly-viridian', 2600),
                                   ('SPECIES_NIDORAN_F', 'SPECIES_NIDORAN_M', 'route5-6', 1500)):
        if sp not in have_sp:
            new = copy.deepcopy(p.get('specials', []))
            new.append({'after': after, 'kind': 'trade', 'species': [sp], 'requires': [req], 'frames': frames})
            out.append(('special', f'trade for {sp} (needs {req}) after {after}', {'specials': new}))
    # In-route hunts at grass the shortest path does not cross (pathenc.py): stand in the grass of a route
    # while passing until its new species are caught or the step budget runs out.
    hs = list(p.get('hunts', []))
    have_h = {(h['after'], h['site']) for h in hs}
    for after, site in (('route3', 'MAP_ROUTE3'), ('route4', 'MAP_ROUTE4'), ('nugget-bridge', 'MAP_ROUTE24'),
                        ('route5-6', 'MAP_ROUTE5'), ('route9', 'MAP_ROUTE9'), ('route10-north', 'MAP_ROUTE10'),
                        ('route8-7', 'MAP_ROUTE8'), ('cycling-road', 'MAP_ROUTE16'), ('cycling-road', 'MAP_ROUTE17'),
                        ('cycling-road', 'MAP_ROUTE18'), ('route22', 'MAP_ROUTE22')):
        if (after, site) in have_h:
            continue
        new = copy.deepcopy(hs)
        new.append(dict(after=after, site=site, area='land', steps=60, detour_tiles=2, skip=[]))
        out.append(('hunt', f'add an in-route hunt at {site} after {after}', {'hunts': new}))
    # Celadon Game Corner prizes (LeafGreen coin prices x 20 money per coin).
    have = {x['species'][0] for x in p.get('specials', []) if x.get('kind') == 'buy'}
    for sp, cost in (('SPECIES_ABRA', 2400), ('SPECIES_CLEFAIRY', 15000), ('SPECIES_PINSIR', 50000)):
        if sp not in have:
            new = copy.deepcopy(p.get('specials', []))
            new.append({'after': 'hall-of-fame-1', 'kind': 'buy', 'species': [sp], 'cost': cost, 'frames': 2400})
            out.append(('special', f'buy {sp} at the Game Corner', {'specials': new}))
    segs = grass_segments()
    cur = list(p.get('repel_on', []))
    for sname in segs:
        if sname in ('route1-encounters', 'route2-encounters', 'forest-encounters'):
            continue  # before the first mart that sells Repels
        new = [x for x in cur if x != sname] if sname in cur else cur + [sname]
        out.append(('repel', f"repel_on {'-' if sname in cur else '+'}{sname}", {'repel_on': new}))
    # --- battle_sim knobs (exact engine, plans 297+).
    if p.get('battle_sim'):
        # Per-fight tactics: add/drop an X item, sacs +-1.
        tactics = p.get('fight_tactics', {})
        key_fights = ['surge', 'misty', 'erika', 'koga', 'sabrina', 'blaine', 'giovanni',
                      'rival-cerulean', 'rival-ss-anne', 'rival-tower', 'rival-silph', 'rival-route22',
                      'lorelei', 'bruno', 'agatha', 'lance', 'champion',
                      'lorelei-2', 'bruno-2', 'agatha-2', 'lance-2', 'champion-2']
        # X items exist only from Celadon on (`CeladonCity_DepartmentStore_5F/scripts.inc:29-35`).
        no_x = {'surge', 'misty', 'rival-cerulean', 'rival-ss-anne'}
        for f in key_fights:
            t = tactics.get(f, {})
            xs = list(t.get('x', []))
            sacs = t.get('sacs', 0)
            kinds = () if f in no_x else ('attack', 'defend', 'speed', 'special', 'dire')
            for xi in kinds:
                q = copy.deepcopy(tactics)
                q.setdefault(f, {'x': [], 'sacs': 0})
                q[f] = dict(q[f]); q[f]['x'] = xs + [xi]
                out.append(('tactic', f"{f}: +x {xi}", {'fight_tactics': q}))
            if xs:
                q = copy.deepcopy(tactics)
                q[f] = dict(q[f]); q[f]['x'] = xs[:-1]
                out.append(('tactic', f"{f}: -x {xs[-1]}", {'fight_tactics': q}))
            for d in (-1, 1):
                if sacs + d >= 0:
                    q = copy.deepcopy(tactics)
                    q.setdefault(f, {'x': [], 'sacs': 0})
                    q[f] = dict(q[f]); q[f]['sacs'] = sacs + d
                    out.append(('tactic', f"{f}: sacs {sacs}->{sacs + d}", {'fight_tactics': q}))
        # X purchases.
        for i, xb in enumerate(p.get('xbuys', [])):
            for k in ('attack', 'defend', 'speed', 'special', 'dire'):
                for d in (-4, 4):
                    v = xb.get(k, 0) + d
                    if v >= 0:
                        q = copy.deepcopy(p['xbuys'])
                        q[i] = dict(q[i]); q[i][k] = v
                        out.append(('xbuy', f"xbuys[{i}] {k} {xb.get(k, 0)}->{v}", {'xbuys': q}))
        # TM / tutor teaches: drop what is there, add what is not. Sources, all decomp-cited:
        # Route 4 tutors (map.json:96,110), TM28 Cerulean (scripts.inc:179-187), TM03 Misty
        # (CeruleanCity_Gym/scripts.inc:23-25), TM26 Giovanni (ViridianCity_Gym/scripts.inc:28-30),
        # TM14 Mansion B1F (map.json:71), TM31/TM28 Celadon 2F mart (scripts.inc:54-61,
        # items.json prices), TM13 Ice Beam 4000 coins = 80000 money
        # (GameCorner_PrizeRoom/scripts.inc:264-266, 20/coin).
        tms = p.get('tms', [])
        have_tm = {(t['after'], t['mv'], t.get('mewtwo', False)) for t in tms}
        for i, t in enumerate(tms):
            q = copy.deepcopy(tms)
            del q[i]
            out.append(('tm', f"drop tm {t['mv']}@{t['after']}", {'tms': q}))
        tm_candidates = [
            ('route4', 'MOVE_MEGA_PUNCH', 1500, 0, False),
            ('route4', 'MOVE_MEGA_KICK', 1500, 0, False),
            ('tm28-dig', 'MOVE_DIG', 800, 0, False),
            ('misty-badge', 'MOVE_WATER_PULSE', 700, 0, False),
            ('giovanni-badge', 'MOVE_EARTHQUAKE', 800, 0, False),
            ('mansion', 'MOVE_BLIZZARD', 1100, 0, False),
            ('celadon-tea', 'MOVE_BRICK_BREAK', 900, 3000, False),
            ('fly-celadon-2', 'MOVE_ICE_BEAM', 2000, 80000, False),
            # Mewtwo cannot learn HM03 Surf (tmhm_learnsets.h). TM03 is Misty's gift,
            # TM31 is sold in Celadon 2F, TM13/TM24 cost 4000 coins (80000 money).
            ('mewtwo', 'MOVE_WATER_PULSE', 600, 0, True),
            ('mewtwo', 'MOVE_BRICK_BREAK', 600, 3000, True),
            ('mewtwo', 'MOVE_ICE_BEAM', 600, 80000, True),
            ('mewtwo', 'MOVE_THUNDERBOLT', 600, 80000, True),
        ]
        for after, mv, frames, cost, mt in tm_candidates:
            if (after, mv, mt) in have_tm:
                continue
            q = copy.deepcopy(tms)
            q.append({'after': after, 'mv': mv, 'frames': frames, 'cost': cost, 'mewtwo': mt})
            out.append(('tm', f"teach {mv}@{after}{' (mewtwo)' if mt else ''}", {'tms': q}))
        # Grinds: resize, drop, add at checkpoints.
        for i, g in enumerate(p.get('grinds', [])):
            for d in (-3, 3):
                if g['battles'] + d > 0:
                    q = copy.deepcopy(p['grinds'])
                    q[i] = dict(q[i]); q[i]['battles'] = g['battles'] + d
                    out.append(('grind', f"grinds[{i}]@{g['after']} battles {g['battles']}->{g['battles'] + d}", {'grinds': q}))
            q = copy.deepcopy(p['grinds'])
            del q[i]
            out.append(('grind', f"drop grind@{g['after']}", {'grinds': q}))
        have_g = {g['after'] for g in p.get('grinds', [])}
        for after, level, yld in (('misty-badge', 16, 60), ('surge-badge', 20, 70),
                                  ('lavender-heal', 24, 75), ('koga-badge', 30, 90)):
            if after in have_g:
                continue
            q = copy.deepcopy(p.get('grinds', []))
            q.append({'after': after, 'battles': 5, 'level': level, 'yield_': yld})
            out.append(('grind', f"add grind@{after}", {'grinds': q}))
        for d in (-4, 4):
            v = p.get('grind_pre_e4', 0) + d
            if v >= 0:
                out.append(('grind', f"grind_pre_e4 {p.get('grind_pre_e4', 0)}->{v}", {'grind_pre_e4': v}))
            v = p.get('grind_pre_rematch', 0) + d
            if v >= 0:
                out.append(('grind', f"grind_pre_rematch {p.get('grind_pre_rematch', 0)}->{v}", {'grind_pre_rematch': v}))
        # Trainer groups: skip or fight each known optional group.
        cur_skip = list(p.get('skip_groups', []))
        for g in ('silph-grunts', 'cerulean-gym-trainer', 'surge-gym-trainers', 'celadon-gym-trainers',
                  'viridian-gym-trainers', 'blaine-quiz', 'route3-trainers', 'mt-moon-trainers',
                  'fuchsia-gym-trainers', 'saffron-gym-trainers', 'mansion-trainers', 'mt-ember-trainers'):
            new = [x for x in cur_skip if x != g] if g in cur_skip else cur_skip + [g]
            out.append(('skipgroup', f"skip_groups {'-' if g in cur_skip else '+'}{g}", {'skip_groups': new}))
        # Pickups: toggle each known on-route Elixir/Max Elixir/Rare Candy ball
        # (each map's map.json; anchors are route segments the plan already walks).
        pick_sites = [('cerulean-heal', 'candy', 350), ('route6-encounters', 'candy', 350),
                      ('route9', 'candy', 350), ('super-rod', 'candy', 300),
                      ('cycling-road', 'candy', 300), ('mansion', 'candy', 300),
                      ('cycling-road', 'max', 500), ('route23', 'max', 400),
                      ('silph-giovanni', 'elixir', 400), ('mansion', 'elixir', 300),
                      ('bill', 'elixir', 500)]
        cur_picks = p.get('elixirs', [])
        have_p = {(e['after'], e['kind']) for e in cur_picks}
        for after, kind, frames in pick_sites:
            if (after, kind) in have_p:
                continue
            q = copy.deepcopy(cur_picks)
            q.append({'after': after, 'kind': kind, 'frames': frames})
            out.append(('pickup', f"pick up {kind}@{after}", {'elixirs': q}))
        for i, e in enumerate(cur_picks):
            q = copy.deepcopy(cur_picks)
            del q[i]
            out.append(('pickup', f"drop {e['kind']}@{e['after']}", {'elixirs': q}))
        for i, xb in enumerate(p.get('xbuys', [])):
            for d in (-3, 3):
                v = xb.get('revives', 0) + d
                if v >= 0:
                    q = copy.deepcopy(p['xbuys'])
                    q[i] = dict(q[i]); q[i]['revives'] = v
                    out.append(('xbuy', f"xbuys[{i}] revives {xb.get('revives', 0)}->{v}", {'xbuys': q}))
        # Pokémon Center stops along the route (towns with a Center the route passes:
        # Viridian, Pewter, Vermilion, Celadon, Fuchsia, Saffron, Cinnabar).
        cur_heals = list(p.get('extra_heals', []))
        for anchor in ('to-viridian', 'to-pewter', 'forest-trainers', 'captain-hm01',
                       'surge-walk', 'tea', 'koga-walk', 'silph-walk', 'blaine-walk',
                       'viridian-gym-walk'):
            new = [x for x in cur_heals if x != anchor] if anchor in cur_heals else cur_heals + [anchor]
            out.append(('heal', f"extra_heals {'-' if anchor in cur_heals else '+'}{anchor}", {'extra_heals': new}))
        # Evolution stat gate (visible stats of the evolved form; SpA is index 4).
        em = p.get('evo_min_stats', [0] * 6)
        for k in (1, 4):
            for d in (-1, 1):
                cur = em[k]
                base = {1: 27, 4: 28}[k]
                v = (cur + d) if cur > 0 else (base if d > 0 else 0)
                if v >= 0 and v != cur:
                    q = list(em)
                    q[k] = v
                    out.append(('evogate', f"evo_min_stats[{k}] {cur}->{v}", {'evo_min_stats': q}))
        if any(em):
            out.append(('evogate', 'drop evo gate', {'evo_min_stats': [0] * 6}))
        # Route and policy toggles.
        for k in ('early_dig', 'erika_after_tower', 'rope_fills', 'rope_cerulean_cave'):
            out.append(('toggle', f'{k} {p.get(k)}->{not p.get(k)}', {k: not p.get(k)}))
        for d in (-0.1, 0.1):
            v = round(p.get('heal_below', 0.4) + d, 2)
            if 0.0 <= v <= 0.9:
                out.append(('heal', f"heal_below {p.get('heal_below', 0.4)}->{v}", {'heal_below': v}))
    if only:
        out = [c for c in out if c[0] in only]
    return out


def try_it(parent, label, patch, name, seed):
    r = subprocess.run([sys.executable, os.path.join(HERE, 'try.py'), os.path.join(PLANS, parent + '.md'), label,
                        json.dumps(patch), '--name', name, '--cap', str(cap), '--seed', str(seed)]
                       + (['--rung', FIXED_RUNG] if FIXED_RUNG else []),
                       capture_output=True, text=True)
    lines = [l for l in r.stdout.strip().splitlines() if l.startswith(('ACCEPT', 'refuted'))]
    return (lines[0] if lines else 'error: ' + r.stdout[-200:] + r.stderr[-200:])


accepts = 0
for ps in range(max_passes):
    changed = False
    cur = current()
    p = load(cur)
    cands = candidates(p)
    print(f'pass {ps}: {len(cands)} candidates from {cur}', flush=True)
    for ci, (kind, label, patch) in enumerate(cands):
        cur = current()
        p = load(cur)
        # skip candidates that no longer differ from the current plan
        if all(p.get(k) == v for k, v in patch.items()):
            continue
        name = f'd{next_number()}-{ci}'
        v1 = try_it(cur, f'[{kind}] {label}', patch, name, 1)
        print(f'  {label}: {v1}', flush=True)
        if not v1.startswith('ACCEPT'):
            continue
        v2 = try_it(cur, f'[{kind}] {label} (confirm seed 2)', patch, name + 's2', 2)
        print(f'    confirm: {v2}', flush=True)
        if not v2.startswith('ACCEPT'):
            continue
        num = next_number()
        pname = f"{num:03d}-{kind}{ci}"
        subprocess.run([sys.executable, os.path.join(HERE, 'promote.py'), os.path.join(SCRATCH, name + '.md'), pname,
                        f'{label} (descend.py; confirmed on seeds 1 and 2)'], check=True, capture_output=True)
        import time
        for attempt_ in range(8):  # another process may hold the index lock for a moment
            a = subprocess.run(['git', '-C', ROOT, 'add', '-A'], capture_output=True)
            b = subprocess.run(['git', '-C', ROOT, 'commit', '-qm',
                                f'human-run: plan {pname}: {label}\n\nCo-Authored-By: Claude Sonnet 5.5 <noreply@anthropic.com>'],
                               capture_output=True)
            if a.returncode == 0 and b.returncode in (0, 1):
                break
            time.sleep(3)
        print(f'  ==> promoted {pname}', flush=True)
        accepts += 1
        changed = True
        if accepts >= max_accepts:
            break
    if not changed or accepts >= max_accepts:
        break
print('done; accepted', accepts)
