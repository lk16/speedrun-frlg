#!/usr/bin/env python3
"""Extra candidates for the honest-model descent (plan 556+): early-fight tactics (Torrent, entry heals,
sacs), Torrent and no-salvage windows, version, safari/pc knobs. stdlib only.

`FightTactic.power` (a multiplier on the fitted carry power) is deliberately NOT searched upward: it is a
calibration stand-in, not a human choice, and a descent on it only makes the model's fights faster than the
WR video's (557-fpower16, Misty power 1.5, was accepted at -11 h and then withdrawn for that reason). Downward
values are kept to document the sensitivity."""
import copy


def _ft(ft, f, **kw):
    q = copy.deepcopy(ft)
    q[f] = dict(q.get(f, {}))
    q[f].update(kw)
    return q


def candidates(p):
    out = []
    ft = p.get('fight_tactics', {})
    for f in ('brock', 'misty', 'rival-cerulean', 'surge', 'rival-ss-anne', 'erika', 'koga'):
        t = ft.get(f, {})
        cur = t.get('power', 1.0)
        for v in (0.8, 0.9):
            nv = round(cur * v, 3)
            out.append(('fpower', f'{f} power {cur}->{nv}', {'fight_tactics': _ft(ft, f, power=nv)}))
        for k in ('torrent', 'entry'):
            nv = not t.get(k, False)
            out.append(('ftoggle', f'{f} {k} {t.get(k, False)}->{nv}', {'fight_tactics': _ft(ft, f, **{k: nv})}))
        for v in (0.2, 0.35, 0.5, 0.65):
            if t.get('heal_below', -1) != v:
                out.append(('fheal', f'{f} heal_below ->{v}', {'fight_tactics': _ft(ft, f, heal_below=v)}))
        for d in (-1, 1):
            if t.get('sacs', 0) + d >= 0:
                out.append(('fsacs', f'{f} sacs ->{t.get("sacs", 0) + d}', {'fight_tactics': _ft(ft, f, sacs=t.get('sacs', 0) + d)}))
    for v in (1.0, 1.5, 2.0, 2.5):
        if p.get('torrent_hits') != v:
            out.append(('torrent', f'torrent_hits {p.get("torrent_hits")}->{v}', {'torrent_hits': v}))
    for v in (0.0, 0.2, 0.4):
        if p.get('torrent_heal_below') != v:
            out.append(('torrent', f'torrent_heal_below {p.get("torrent_heal_below")}->{v}', {'torrent_heal_below': v}))
    for v in ('', 'brock', 'misty', 'rival-ss-anne', 'surge', 'koga', 'sabrina', 'giovanni', 'indigo-heal', 'hall-of-fame-1'):
        if p.get('torrent_until') != v:
            out.append(('torrent', f'torrent_until {p.get("torrent_until")}->{v}', {'torrent_until': v}))
    for v in ('', 'brock', 'misty', 'rival-cerulean', 'surge', 'koga', 'sabrina', 'giovanni', 'hall-of-fame-1'):
        if p.get('no_salvage_until') != v:
            out.append(('nosalv', f'no_salvage_until {p.get("no_salvage_until")}->{v}', {'no_salvage_until': v}))
    other = 'FR' if p.get('version') == 'LG' else 'LG'
    out.append(('version', f'version {p.get("version")}->{other}', {'version': other}))
    for v in (0, 1, 2, 3, 4):
        if p.get('candy_early_n') != v:
            out.append(('misc', f'candy_early_n {p.get("candy_early_n")}->{v}', {'candy_early_n': v}))
    for d in (-2, -1, 1, 2, 3):
        v = p.get('safari_stop', 5) + d
        if v > 0:
            out.append(('misc', f'safari_stop {p.get("safari_stop")}->{v}', {'safari_stop': v}))
    for d in (-2, 2):
        out.append(('misc', f'level_gap {p.get("level_gap")}->{p.get("level_gap", 8) + d}', {'level_gap': p.get('level_gap', 8) + d}))
    for k in ('no_dig', 'clemi_skip', 'league_transit', 'item_swaps', 'candy_early', 'teleport_abra', 'safari_model', 'skip_powerplant'):
        out.append(('toggle', f'{k} {p.get(k)}->{not p.get(k)}', {k: not p.get(k)}))
    return out
