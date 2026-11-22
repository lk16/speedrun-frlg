#!/usr/bin/env python3
"""Candidate patches over which optional trainers the human fights (`fight_lists`, plan 1253+), for descend.py.

The exp-vs-time trade-off of the runner-level fight list (journal 2026-11-22-16-50-exp-and-koga.md): drop one listed
trainer, or add the played s11 run's fights (plan 1255's list) per segment.

Legality (`forced-audit.md`, sight cones from each `map.json`, `decompiled/src/trainer_see.c:198-231`):
* a trainer named forced for its leg is never dropped (for "one of A/B" groups both are protected: conservative);
* a list replacing a Parties/Group segment never goes below the segment's default (cone) fight count
  (`route.rs::apply_fight_lists` replaces the whole segment);
* additions: for segments that are not trainer groups the list is inserted as `<seg>+fights`, so single trainers can
  be added; for trainer-group segments only the whole s11 list is proposed (it contains the forced ones).
stdlib only.
"""
import copy
import json
import os

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.join(HERE, '..', '..')

# forced-audit.md, table "Leg | Forced": names (upper case, as in the trainer ids) protected per segment.
FORCED = {
    'cerulean-gym-trainer': {'DIANA'},
    'route25-trainers': {'HALEY', 'KELSEY', 'FLINT', 'FRANKLIN', 'NOB', 'WAYNE'},
    'route6-trainers': {'JEFF'},
    'route8-trainers': {'JULIA', 'RICH'},
    'rock-tunnel-trainers': {'ALICIA', 'CONNER', 'ASHTON', 'DANA', 'DUDLEY', 'MARTHA', 'SOFIA', 'WINSTON', 'LENNY',
                             'OLIVER', 'ALLEN', 'ERIC'},
    'route3-trainers': {'COLTON', 'JAMES', 'GREG', 'SALLY'},
    'tower-channelers': {'EMILIA', 'JENNIFER', 'KARINA', 'TAMMY'},
    'fuchsia-gym-trainers': {'KIRK'},
}
# Default (cone) fight counts of the trainer-group segments (`frlg-human segs PLAN --fights` on 1132).
GROUP_MIN = {'route3-trainers': 3, 'cerulean-gym-trainer': 1, 'route25-trainers': 4, 'route6-trainers': 1,
             'rock-tunnel-trainers': 10, 'route8-trainers': 1, 'hideout-grunts': 3, 'tower-channelers': 3,
             'surge-gym-trainers': 1, 'fuchsia-gym-trainers': 1}
S11_PLAN = '1255-played-fights-honest'


def _s11():
    for f in os.listdir(os.path.join(ROOT, 'human-run', 'plans')):
        if f.startswith(S11_PLAN):
            t = open(os.path.join(ROOT, 'human-run', 'plans', f)).read()
            return json.loads(t.split('```json')[1].split('```')[0]).get('fight_lists', {})
    return {}


def candidates(p):
    fl = p.get('fight_lists') or {}
    if not fl:
        return []
    out = []
    for seg, ids in fl.items():
        prot = FORCED.get(seg, set())
        for i, tid in enumerate(ids):
            if tid.split('_')[-1] in prot:
                continue
            if seg in GROUP_MIN and len(ids) - 1 < GROUP_MIN[seg]:
                continue
            q = copy.deepcopy(fl)
            del q[seg][i]
            if not q[seg] and seg not in GROUP_MIN:
                del q[seg]
            out.append(('fights', f'fight_lists {seg} -{tid}', {'fight_lists': q}))
    for seg, ids in _s11().items():
        have = fl.get(seg, [])
        extra = [t for t in ids if t not in have]
        if not extra:
            continue
        if seg in GROUP_MIN:
            q = copy.deepcopy(fl)
            q[seg] = list(ids)
            out.append(('fights', f'fight_lists {seg} = s11 list ({len(ids)})', {'fight_lists': q}))
        else:
            for t in extra:
                q = copy.deepcopy(fl)
                q[seg] = have + [t]
                out.append(('fights', f'fight_lists {seg} +{t}', {'fight_lists': q}))
    return out
