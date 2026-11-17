#!/usr/bin/env python3
"""Plan variants x constant corners, paired (common seed), for ROBUSTNESS.md section 14. stdlib only.

usage: rb_var.py grid OUT.json CAL.json RUNG R SEED [--variants a,b,c] [--scen s1,s2] [--plan 1057-calm-sp9-ppup-redo]
Each cell is `frlg-human eval` of (variant patch, then corner patch) on the same seed; prints ln-ratio to the variant
'base' in the same corner. Corners that move the carry / league / mewtwo stand-ins re-map `lane` / `flat` through the
calibration so the evidence-pinned survivals (Q, D) stay at their median unless the corner is itself a Q / D corner.
"""
import json, math, os, sys

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
import rb_lib as L  # noqa: E402
import rb_mc as M  # noqa: E402

Q0, D0 = 0.0587, 0.2443   # the model's own values at the plan constants (lenient rung)


def corner(name, cal):
    """kwargs for rb_lib.patch for a named corner of the constant ranges."""
    base = dict(Q=Q0, D=D0, lp=1.4, mp=1.2, cp=2.0)

    def mk(**ch):
        v = dict(base)
        v.update(ch)
        kw = dict(lp=v['lp'], mp=v['mp'], cp=v['cp'])
        kw['lane'] = cal.lam_for(v['cp'], v['Q'])
        kw['flat'] = cal.flat_for(v['lp'], v['mp'], v['D'])
        return kw
    table = {
        'base': {},
        'Qlow': mk(Q=0.030), 'Qhigh': mk(Q=0.110),
        'Dlow': mk(D=0.10), 'Dhigh': mk(D=0.40),
        'lp1.0': mk(lp=1.0), 'lp1.7': mk(lp=1.7),
        'mp1.0': mk(mp=1.0), 'mp1.5': mk(mp=1.5),
        'cp1.7': mk(cp=1.7), 'cp2.3': mk(cp=2.3),
        'slow': dict(tf=0.85, mr=0.37, win_cv=0.069),
        'fast': dict(tf=0.65, mr=0.07, win_cv=0.0),
        'costly': dict(ppup_f=900, hid=2.0, museum=1.5),
        'cheap': dict(ppup_f=200, hid=0.6, museum=0.8),
        # the 1058 pickups (rb_lib.NEW_PICKS): pk = x on the plan's frames (300-450 f), pkg = x on the map-geometry medians
        'pk2': dict(pk=2.0), 'pk4': dict(pk=4.0), 'geo': dict(pkg=1.0), 'geo2': dict(pkg=2.0),
        'lp1.0pk4': mk(lp=1.0) | dict(pk=4.0), 'cp1.7pk4': mk(cp=1.7) | dict(pk=4.0),
        'lp1.8': mk(lp=1.8), 'cp2.3': mk(cp=2.3),
    }
    return table[name]


def variant(name, base):
    """Return the patched plan dict for a named variant (see VARIANTS)."""
    q = json.loads(json.dumps(base))
    VARIANTS[name](q)
    return q


def _set(**kv):
    def f(q):
        for k, v in kv.items():
            d = q
            parts = k.split('.')
            for p in parts[:-1]:
                d = d.setdefault(p, {})
            d[parts[-1]] = v
    return f


def _drop_ppup(after):
    def f(q):
        q['elixirs'] = [e for e in q['elixirs'] if not (e.get('kind') == 'ppup' and e['after'] in after)]
    return f


def _add_elixir(**e):
    def f(q):
        q['elixirs'].append(e)
    return f


def _xbuy_special(n):
    def f(q):
        q['xbuys'][1]['special'] = n
    return f


def _tactic(seg, x):
    def f(q):
        q['fight_tactics'][seg]['x'] = x
    return f


def _drop_new(keys):
    def f(q):
        for k in keys:
            lst, after, kind = L.NEW_PICKS[k]
            q[lst] = [e for e in q[lst] if not (e.get('after') == after and e.get('kind') == kind)]
    return f


VARIANTS = {
    'base': lambda q: None,
    'no_r9candy': _drop_new(['r9']),
    'no_r17candy': _drop_new(['r17']),
    'no_candy': _drop_new(['r9', 'r17']),
    'no_hcalcium': _drop_new(['hideout']),
    'no_scalcium': _drop_new(['silph']),
    'no_calcium': _drop_new(['hideout', 'silph']),
    'no_sell': _drop_new(['sell']),
    'no_new': _drop_new(['r9', 'r17', 'hideout', 'silph', 'sell']),
    'ppup2': _drop_ppup(['celadon-tea']),
    'ppup1': _drop_ppup(['celadon-tea', 'cycling-road']),
    'ppup0': _drop_ppup(['celadon-tea', 'cycling-road', 'hideout-grunts']),
    'ppup4': _add_elixir(after='silph-giovanni', kind='ppup', frames=450),
    'spec7': _xbuy_special(7),
    'spec11': _xbuy_special(11),
    'floor4': _set(mewtwo_pp_floor=4),
    'floor6': _set(mewtwo_pp_floor=6),
    'candy2': _set(candy_early_n=2),
    'calm_l2x1': lambda q: _tactic('lorelei-2', ['calm'])(q),
    'cham2_calm3': lambda q: _tactic('champion-2', ['calm', 'calm', 'calm'])(q),
    'heals16': _set(league_heals=16),
    'heal_below5': _set(heal_below=0.5),
    'heal_below7': _set(heal_below=0.7),
    'preheal9': _set(league_preheal=0.9),
    'torrent_to_koga': _set(torrent_until='koga-badge'),
    'nomaxe_route23': lambda q: q.__setitem__('elixirs', [e for e in q['elixirs'] if not (e['kind'] == 'max' and e['after'] == 'route23')]),
    'spe27': _set(**{'manip.min_spe': 27}),
    'spe23': _set(**{'manip.min_spe': 23}),
    'combo': lambda q: (_xbuy_special(11)(q), _set(heal_below=0.5)(q), _tactic('champion-2', ['calm', 'calm', 'calm'])(q)),
    'combo_hs': lambda q: (_xbuy_special(11)(q), _set(heal_below=0.5)(q)),
    'hb5_c3': lambda q: (_set(heal_below=0.5)(q), _tactic('champion-2', ['calm', 'calm', 'calm'])(q)),
    'phit_free': _set(**{'manip.quiet_max_hits': 4}),
}


def opt(a, name, d):
    if name in a:
        i = a.index(name)
        v = type(d)(a[i + 1])
        del a[i:i + 2]
        return v
    return d


def main():
    cmd = sys.argv[1]
    a = sys.argv[2:]
    assert cmd == 'grid'
    out, calp, rung, R, seed = a[0], a[1], a[2], int(a[3]), int(a[4])
    a = a[5:]
    plan = opt(a, '--plan', '1058-hidden-candy-calcium-nugget')
    vs = opt(a, '--variants', ','.join(VARIANTS)).split(',')
    sc = opt(a, '--scen', 'base,Qlow,Dlow,lp1.0,mp1.0,cp1.7,slow,costly').split(',')
    base = L.load_plan(plan)
    cal = M.Cal(calp, base)
    res = json.load(open(out)) if os.path.exists(out) else []
    have = {(r['variant'], r['scen']) for r in res}
    for s in sc:
        kw = corner(s, cal)
        for v in vs:
            if (v, s) in have:
                continue
            q = L.patch(variant(v, base), **kw)
            e, f = L.run_eval(q, f'var_{v}_{s}'.replace('.', '_'), rung, R, seed)
            res.append(dict(variant=v, scen=s, eht=e, fin=f, rung=rung, R=R, seed=seed))
            json.dump(res, open(out, 'w'))
            b = next((r['eht'] for r in res if r['variant'] == 'base' and r['scen'] == s), None)
            print(s, v, e, f, (round(e / b, 3) if (e and b) else None), flush=True)


if __name__ == '__main__':
    main()
