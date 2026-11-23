#!/usr/bin/env python3
"""honesty4_mk.py PARENT NAME OUT STEP[,STEP...]: plan variants for model-honesty-4 (controller-v4 bugs, plans 1620-1639).

Journal human-run/journal/2026-11-23-*-model-honesty-4.md. Each step is a MODEL change or a PLAN response (labelled
below). stdlib only.
"""
import json
import os
import sys

ROOT = os.path.join(os.path.dirname(os.path.abspath(__file__)), '..', '..')
par, name, out, steps = sys.argv[1:5]
path = par if os.path.exists(par) else os.path.join(ROOT, 'human-run/plans', par + '.md')
t = open(path).read()
p = json.loads(t.split('```json')[1].split('```')[0])
parent_name = p['name']
for st in steps.split(','):
    if st.startswith('reserve'):
        # MODEL: a passing catch below catch rate 100 is skipped while the bag holds <= N balls (controller
        # `PASS_RESERVE` 8, play/dex.rs). reserve or reserve=N.
        p['pass_reserve'] = int(st.split('=')[1]) if '=' in st else 8
        p['pass_reserve_rate'] = 100
    elif st.startswith('throwcap='):
        # PLAN: throwcap=K:P -- at most K throws when the first ball's catch chance is below P.
        k, pr = st[9:].split(':')
        p['low_chance_throws'] = int(k)
        p['low_chance_below'] = float(pr)
    elif st.startswith('safwalk'):
        # MODEL: in-zone walking steps run the Safari counter (played v4: ~100 to the East grass, ~260 East -> West).
        e, w = (100, 260)
        if '=' in st:
            e, w = (int(x) for x in st.split('=')[1].split(':'))
        p['safari_walk_steps'] = {'MAP_SAFARI_ZONE_EAST': e, 'MAP_SAFARI_ZONE_WEST': w}
    elif st == 'drain':
        # PLAN: the West hunt takes every step left; time's up warps to the gate (no walk back).
        p['safari_drain'] = 'MAP_SAFARI_ZONE_WEST'
    elif st.startswith('hunt='):
        # PLAN: hunt=SITE_SUFFIX:STEPS, e.g. hunt=EAST:120.
        site, n = st[5:].split(':')
        h = next(h for h in p['hunts'] if h['site'] == 'MAP_SAFARI_ZONE_' + site)
        h['steps'] = int(n)
    elif st.startswith('fullheal='):
        # PLAN: fullheal=N -- N Full Heals bought at the Fuchsia mart on `koga-walk` (used on a poisoned fighter).
        p['full_heals'] = int(st[9:])
        p['full_heal_at'] = 'koga-walk'
    elif st.startswith('antidote='):
        # PLAN: antidote=N -- N Antidotes (100) added to the Lavender ball stop (`walk-lavender`), ~150 frames (guess).
        p['full_heals'] = int(st[9:])
        p['full_heal_at'] = 'walk-lavender'
        p['full_heal_price'] = 100
        p['full_heal_frames'] = 150.0
    elif st.startswith('kogax='):
        # PLAN: Koga's X queue, e.g. kogax=special:special.
        p['fight_tactics']['koga']['x'] = [x for x in st[6:].split(':') if x]
    elif st.startswith('kogahb='):
        p['fight_tactics']['koga']['heal_below'] = float(st[7:])
    else:
        sys.exit('unknown step ' + st)
p['name'] = name
open(out, 'w').write(f'# Plan {name}\n\nParent: `{parent_name}`. Steps: {steps} (`human-run/tools/honesty4_mk.py`).\n\n'
                     'The step-by-step description, citations and branch rules in `001-crude.md` still apply except\n'
                     'where the parameters below differ; `frlg-human` reads only the block below.\n\n'
                     '```json\n' + json.dumps(p, indent=1, ensure_ascii=False) + '\n```\n')
