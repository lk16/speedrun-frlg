#!/usr/bin/env python3
"""Candidates for the honest-PP model descent (plan 1057+): calm/guard/accuracy X tokens in the League fights,
PP Up / Ether pickups, Mewtwo PP floor, Max Elixir reserve, league_redo / salvage_at. stdlib only."""
import copy

LEAGUE = ['lorelei', 'bruno', 'agatha', 'lance', 'champion', 'lorelei-2', 'bruno-2', 'agatha-2', 'lance-2', 'champion-2']
TOKENS = ('calm', 'guard', 'accuracy')
# (anchor, kind, frames, source). Frames are guesses (hidden tiles), same 450 f as the three accepted PP Ups.
PICKS = [
    ('silph-grunts', 'ppup', 450, 'Silph 5F hidden (32,9)'),
    ('silph-giovanni', 'ppup', 450, 'Silph 5F hidden (32,9)'),
    ('fuji-flute', 'ppup', 450, 'Route 13 hidden (23,14)'),
    ('deliver-meteorite', 'ppup', 450, 'Three Island hidden (5,13)'),
    ('warden-strength', 'ppup', 450, 'Route 13 / Fuchsia detour'),
    ('powerplant-out', 'max', 600, 'Power Plant Max Elixir'),
    ('route23', 'ppup', 450, 'late 4th PP Up'),
    ('mt-moon-b2f', 'ether', 300, 'Mt Moon B2F hidden'),
    ('route23', 'maxether', 450, 'Route 23 hidden'),
]


def candidates(p):
    out = []
    ft = p.get('fight_tactics', {})
    for f in LEAGUE:
        t = ft.get(f, {})
        xs = list(t.get('x', []))
        for tok in TOKENS:
            q = copy.deepcopy(ft)
            q.setdefault(f, {'x': [], 'sacs': 0})
            q[f] = dict(q[f])
            q[f]['x'] = xs + [tok]
            out.append(('tactic', f'{f}: +x {tok}', {'fight_tactics': q}))
        if xs:
            for tok in ('calm', 'guard', 'special', 'speed'):
                if xs[-1] != tok:
                    q = copy.deepcopy(ft)
                    q[f] = dict(q[f])
                    q[f]['x'] = xs[:-1] + [tok]
                    out.append(('tactic', f'{f}: last x {xs[-1]}->{tok}', {'fight_tactics': q}))
            for tok in ('calm', 'guard'):
                if xs[0] != tok:
                    q = copy.deepcopy(ft)
                    q[f] = dict(q[f])
                    q[f]['x'] = [tok] + xs[1:]
                    out.append(('tactic', f'{f}: first x {xs[0]}->{tok}', {'fight_tactics': q}))
    cur = p.get('elixirs', [])
    have = {(e['after'], e['kind']) for e in cur}
    for a, kind, frames, src in PICKS:
        if (a, kind) in have:
            continue
        out.append(('pickup', f'pick up {kind}@{a} ({src})', {'elixirs': cur + [{'after': a, 'kind': kind, 'frames': frames}]}))
    for i, e in enumerate(cur):
        if e['kind'] in ('ppup', 'max', 'elixir', 'ether', 'maxether'):
            q = copy.deepcopy(cur)
            del q[i]
            out.append(('pickup', f"drop {e['kind']}@{e['after']}", {'elixirs': q}))
        if e['kind'] in ('ppup', 'max'):
            for fr in (300, 700):
                if e['frames'] != fr:
                    q = copy.deepcopy(cur)
                    q[i] = dict(q[i])
                    q[i]['frames'] = fr
                    out.append(('ppcost', f"{e['kind']}@{e['after']} frames {e['frames']}->{fr}", {'elixirs': q}))
    fl = p.get('mewtwo_pp_floor', 5)
    for v in (fl - 1, fl + 1):
        out.append(('ppfloor', f'mewtwo_pp_floor {fl}->{v}', {'mewtwo_pp_floor': v}))
    rs = p.get('league_max_reserve', 2)
    for v in (rs - 1, rs + 1):
        if v >= 0:
            out.append(('ppfloor', f'league_max_reserve {rs}->{v}', {'league_max_reserve': v}))
    for v in (0.5, 1.0):
        if p.get('league_redo') != v:
            out.append(('redo', f'league_redo {p.get("league_redo")}->{v}', {'league_redo': v}))
    sa = p.get('salvage_at', {})
    for v in (0.6, 0.7, 0.9):
        out.append(('redo', f'salvage_at all ->{v}', {'salvage_at': {k: v for k in sa}}))
    return out
