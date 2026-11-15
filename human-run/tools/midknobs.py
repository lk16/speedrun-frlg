#!/usr/bin/env python3
"""Mid-game candidates for the extended risk lane (plan 940+): per-fight potions, sacrifices, X items and heal threshold
for the fights between Rock Tunnel and the E4 door, whose hazards the lane now computes (the flat `fatal` is gone
there). The fitted lane constants (`risk_windows[].power`), the no-salvage habit, the starters and the early-game set
(`earlyknobs.py`) are never searched here. stdlib only."""
import copy

MID = ['rock-tunnel-trainers', 'hideout-grunts', 'hideout-giovanni', 'rival-tower', 'surge', 'koga', 'blaine',
       'three-island-paxton', 'rival-silph', 'sabrina', 'erika', 'giovanni', 'rival-route22']
X_ITEMS = ('special', 'attack', 'speed', 'defend')


def _ft(ft, f, **kw):
    q = copy.deepcopy(ft)
    q[f] = dict(q.get(f, {}))
    q[f].update(kw)
    return q


def candidates(p):
    out = []
    ft = p.get('fight_tactics', {})
    for f in MID:
        t = ft.get(f, {})
        for v in (0, 1, 2, 5, 8):
            if t.get('potions', -1) != v:
                out.append(('mpot', f'{f} potions {t.get("potions", -1)}->{v}', {'fight_tactics': _ft(ft, f, potions=v)}))
        for d in (-1, 1):
            if t.get('sacs', 0) + d >= 0:
                out.append(('msacs', f'{f} sacs {t.get("sacs", 0)}->{t.get("sacs", 0) + d}',
                            {'fight_tactics': _ft(ft, f, sacs=t.get('sacs', 0) + d)}))
        xs = list(t.get('x', []))
        for x in X_ITEMS:
            if len(xs) < 4:
                out.append(('mx', f'{f} x +{x}', {'fight_tactics': _ft(ft, f, x=xs + [x])}))
        if xs:
            out.append(('mx', f'{f} x -{xs[-1]}', {'fight_tactics': _ft(ft, f, x=xs[:-1])}))
        for v in (0.25, 0.4, 0.55):
            if t.get('heal_below', -1) != v:
                out.append(('mheal', f'{f} heal_below ->{v}', {'fight_tactics': _ft(ft, f, heal_below=v)}))
    for i, w in enumerate(p.get('risk_windows', [])):
        for v in (3, 6, 10):
            if w.get('potions') != v:
                q = copy.deepcopy(p['risk_windows'])
                q[i]['potions'] = v
                out.append(('mwin', f'risk_windows[{i}] ({w["until"]}) potions {w.get("potions")}->{v}', {'risk_windows': q}))
    return out
