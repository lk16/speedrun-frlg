#!/usr/bin/env python3
"""Candidate patches over the plan's shop stops (`marts`, model honesty 2: top-up targets for balls and the finite potion
bag), for descend.py `--set shops`.

Per stop: Great / Poke / Ultra Ball targets +-5 / +-10, potion target +-1 / +-2 (stops that sell potions, i.e. the
ones that already carry a `potions` target or come after `potion_bag_from`), drop the stop; plus `league_heals` +-2
(the Indigo purchase competes for the same money). The stops themselves are the honesty agent's list of marts on the
route (`human-run/tools/honesty2_mk.py`, each map's mart script); no new stop locations are invented here.
stdlib only.
"""
import copy


def candidates(p):
    marts = p.get('marts') or []
    out = []
    for i, m in enumerate(marts):
        for k, ds in (('great', (-10, -5, 5, 10)), ('poke', (-5, 5)), ('ultra', (5,)), ('potions', (-2, -1, 1, 2))):
            if k == 'potions' and 'potions' not in m:
                continue
            if k in ('poke', 'ultra') and not m.get(k) and k != 'ultra':
                continue
            for d in ds:
                v = m.get(k, 0) + d
                if v < 0 or v == m.get(k, 0):
                    continue
                q = copy.deepcopy(marts)
                q[i][k] = v
                out.append(('shops', f"marts[{i}]@{m['after']} {k} {m.get(k, 0)}->{v}", {'marts': q}))
        q = copy.deepcopy(marts)
        del q[i]
        out.append(('shops', f"drop marts[{i}]@{m['after']}", {'marts': q}))
    for d in (-2, 2):
        v = p.get('league_heals', 6) + d
        if v >= 0:
            out.append(('shops', f"league_heals {p.get('league_heals', 6)}->{v}", {'league_heals': v}))
    return out
