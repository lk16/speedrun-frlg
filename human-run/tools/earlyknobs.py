#!/usr/bin/env python3
"""Early-game strategy candidates for the risk-lane model (plan 930+): potions per fight, early helpers as
sacrifices, wild-fight level cutoff, early grass hunts (grind by real encounters, which go through the lane), early
rare candies, Torrent window thresholds. The no-salvage habit, the Rash/Mild starters and the fitted lane constants
(`risk_foe`, `risk_power`, `fatal`) are never searched. X Defend is not a candidate: no X item can be had before
Route 11's ball / the Celadon 5F counter (`data/maps/Route11/map.json`, `CeladonCity_DepartmentStore_5F/scripts.inc:30`).
stdlib only."""
import copy

EARLY = ['forest-trainers', 'pewter-gym-trainer', 'brock', 'route3-trainers', 'mt-moon-miguel', 'misty',
         'rival-cerulean', 'nugget-bridge-trainers', 'rival-ss-anne']


def _ft(ft, f, **kw):
    q = copy.deepcopy(ft)
    q[f] = dict(q.get(f, {}))
    q[f].update(kw)
    return q


def candidates(p):
    out = []
    ft = p.get('fight_tactics', {})
    for f in EARLY:
        t = ft.get(f, {})
        for v in (0, 1, 2, 3, 5, 9):
            if f == 'brock':
                continue  # stipulation: the runner never heals in Brock (STRATEGY s4); heal-before-Brock is an upside note only
            if t.get('potions', -1) != v:
                out.append(('epot', f'{f} potions {t.get("potions", -1)}->{v}', {'fight_tactics': _ft(ft, f, potions=v)}))
        for d in (1, 2):
            patch = {'fight_tactics': _ft(ft, f, sacs=t.get('sacs', 0) + d)}
            if not p.get('early_helpers'):
                patch['early_helpers'] = True
            out.append(('ehelp', f'{f} sacs {t.get("sacs", 0)}->{t.get("sacs", 0) + d} (early helpers)', patch))
        if t.get('sacs', 0) > 0:
            out.append(('ehelp', f'{f} sacs ->0', {'fight_tactics': _ft(ft, f, sacs=0)}))
        out.append(('eentry', f'{f} entry {t.get("entry", False)}->{not t.get("entry", False)}',
                    {'fight_tactics': _ft(ft, f, entry=not t.get('entry', False))}))
        for v in (0.0, 0.3, 0.5):
            if t.get('heal_below', -1) != v:
                out.append(('eheal', f'{f} heal_below ->{v}', {'fight_tactics': _ft(ft, f, heal_below=v)}))
    for v in (1, 2, 4, 6):
        if p.get('risk_potions') != v:
            out.append(('epot', f'risk_potions {p.get("risk_potions")}->{v}', {'risk_potions': v}))
    for v in (1.0, 1.5, 2.0, 2.5, 3.0):
        if p.get('torrent_hits') != v:
            out.append(('etorrent', f'torrent_hits {p.get("torrent_hits")}->{v}', {'torrent_hits': v}))
    for v in (0.0, 0.2, 0.3, 0.5, 0.6):
        if p.get('torrent_heal_below') != v:
            out.append(('etorrent', f'torrent_heal_below {p.get("torrent_heal_below")}->{v}', {'torrent_heal_below': v}))
    out.append(('etorrent', f'torrent_stall {p.get("torrent_stall", True)}->{not p.get("torrent_stall", True)}',
                {'torrent_stall': not p.get('torrent_stall', True)}))
    for v in ('brock', 'route3-trainers', 'mt-moon-miguel', 'misty-badge', 'rival-cerulean', 'rival-ss-anne',
              'surge-badge', 'koga-badge', 'hall-of-fame-1'):
        if p.get('torrent_until') != v:
            out.append(('etorrent', f'torrent_until {p.get("torrent_until")}->{v}', {'torrent_until': v}))
    for v in (8, 10, 11, 12, 13, 14, 16, 18, 20):
        if p.get('fight_until_level') != v:
            out.append(('ewild', f'fight_until_level {p.get("fight_until_level")}->{v}', {'fight_until_level': v}))
    # Grind by real encounters before Brock / on Route 3: hunts in grass the route passes anyway.
    hs = list(p.get('hunts', []))
    have = {(h['after'], h['site']) for h in hs}
    for after, site in (('forest-encounters', 'MAP_VIRIDIAN_FOREST'), ('route2-encounters', 'MAP_ROUTE2'),
                        ('route1-encounters', 'MAP_ROUTE1'), ('route3', 'MAP_ROUTE3')):
        if (after, site) in have:
            continue
        for steps in (25, 60):
            new = copy.deepcopy(hs)
            new.append(dict(after=after, site=site, area='land', steps=steps, detour_tiles=0, skip=[], on_route=True))
            out.append(('ehunt', f'early hunt {site} {steps} steps after {after}', {'hunts': new}))
    for v in (0, 1, 2, 4, 5, 6):
        if p.get('candy_early_n') != v:
            out.append(('ecandy', f'candy_early_n {p.get("candy_early_n")}->{v}', {'candy_early_n': v}))
    cur_skip = list(p.get('skip_groups', []))
    for g in ('route3-trainers', 'mt-moon-trainers', 'cerulean-gym-trainer'):
        new = [x for x in cur_skip if x != g] if g in cur_skip else cur_skip + [g]
        out.append(('egroup', f"skip_groups {'-' if g in cur_skip else '+'}{g}", {'skip_groups': new}))
    ef = p.get('extra_fights', [])
    for i, e in enumerate(ef):
        for j, sc in enumerate(e.get('scripts', [])):
            q = copy.deepcopy(ef)
            q[i]['scripts'] = [s for k, s in enumerate(e['scripts']) if k != j]
            if q[i]['scripts']:
                out.append(('egroup', f'extra_fights {e["name"]} drop {sc.split("_")[-1]}', {'extra_fights': q}))
        q = copy.deepcopy(ef)
        del q[i]
        out.append(('egroup', f'extra_fights drop {e["name"]}', {'extra_fights': q}))
    return out
