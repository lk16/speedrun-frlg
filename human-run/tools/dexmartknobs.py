#!/usr/bin/env python3
"""Candidate patches for the pre-League dex fill's ball supply and its hunt list, for descend.py `--set dexfill`.

The model's `dex-fill` (route_wr.rs, after `victory-road`) runs the bag dry (journal 2026-11-23-06-40
model-honesty-4: 866 empty-bag events in 300 attempts): the last ball stop is `mart@sabrina-badge`, and the
Route 22 / 23 hunts and Victory Road passing catches spend from it before the fill. With scheduled marts the
fill never buys on demand (`take_ball`, sim.rs), so an empty bag makes every remaining fill encounter a loss.

New stops, each at a mart on the route at that point (inventory from each map's scripts.inc):
* `erika-badge`: Celadon Department Store 2F sells Great Balls (`CeladonCity_DepartmentStore_2F/scripts.inc:29`).
* `fly-viridian` / `giovanni-badge`: Viridian Mart sells Poke Balls only (`ViridianCity_Mart/scripts.inc:66`).
* `victory-road`: the Indigo Plateau Center's mart, at the Victory Road exit, sells Ultra and Great Balls
  (`IndigoPlateau_PokemonCenter_1F/scripts.inc:54-55`); `dex-fill` starts from there.
Then the existing stops' ball targets (+5 / +10 Great at Sabrina), and the fill list itself (per-site steps,
adjacent swaps, drops; descend.py's own fill candidates). stdlib only.
"""
import copy

NEW_STOPS = [
    # Great only: the plan's ball rule throws Great Balls and never reaches up to an Ultra (`bag_ball`).
    ('victory-road', 'great', (10, 15, 20, 30)),
    ('erika-badge', 'great', (10, 20)),
    ('giovanni-badge', 'poke', (10, 20)),
]


def candidates(p, base_candidates):
    out = []
    marts = p.get('marts') or []
    have = {m['after']: i for i, m in enumerate(marts)}
    for after, kind, qtys in NEW_STOPS:
        for q in qtys:
            new = copy.deepcopy(marts)
            if after in have:
                m = new[have[after]]
                if m.get(kind, 0) == q:
                    continue
                m[kind] = q
                label = f"marts@{after} {kind} ->{q}"
            else:
                new.append({'after': after, kind: q, 'topup': True})
                label = f"add mart@{after} {kind} {q}"
            out.append(('dexmart', label, {'marts': new}))
    for i, m in enumerate(marts):
        if m['after'] == 'sabrina-badge':
            for d in (5, 10):
                new = copy.deepcopy(marts)
                new[i]['great'] = m.get('great', 0) + d
                out.append(('dexmart', f"marts[{i}]@sabrina-badge great {m.get('great', 0)}->{m.get('great', 0) + d}",
                            {'marts': new}))
    out += [c for c in base_candidates if c[0] in ('fillsteps', 'order', 'dropfill', 'addsite')]
    return out
