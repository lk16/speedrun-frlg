#!/usr/bin/env python3
"""Starter-phase candidates (manip block): accept thresholds and quiet-seed knobs. Natures stay Rash/Mild.
`dials` is not searched (2 would make the model cheaper without evidence: a calibration, not a choice). stdlib only."""
import copy


def _m(p, **kw):
    q = copy.deepcopy(p.get('manip', {}))
    q.update(kw)
    return {'manip': q}


def candidates(p):
    m = p.get('manip', {})
    out = []
    for k, vals in (('min_spa', (28, 29, 31)), ('min_spe', (14, 16, 18, 22, 24, 26)),
                    ('min_hp', (6, 8, 10, 14, 16, 18)), ('quiet_max_hits', (2, 3, 5, 6)),
                    ('quiet_window', (30, 45))):
        for v in vals:
            if m.get(k) != v:
                out.append(('manip', f'manip.{k} {m.get(k)}->{v}', _m(p, **{k: v})))
    for k in ('min_def', 'min_atk', 'min_spd'):
        for v in (8, 14):
            out.append(('manip', f'manip.{k} {m.get(k, 0)}->{v}', _m(p, **{k: v})))
    return out
