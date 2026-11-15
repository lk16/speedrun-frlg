#!/usr/bin/env python3
"""usage: mkplan.py PARENT NEWNAME OUTFILE 'patch json' -- write a plan file from PARENT (plan name under
human-run/plans) with a top-level patch; fight_tactics and manip patches merge per key. stdlib only."""
import json, os, sys
ROOT = os.path.join(os.path.dirname(os.path.abspath(__file__)), '..', '..')
par, new, out, patch = sys.argv[1:5]
t = open(par if os.path.exists(par) else os.path.join(ROOT, 'human-run', 'plans', par + '.md')).read()
p = json.loads(t.split('```json')[1].split('```')[0])
for k, v in json.loads(patch).items():
    if k == 'fight_tactics':
        for f, d in v.items():
            p['fight_tactics'].setdefault(f, {}).update(d)
    elif k == 'manip':
        p['manip'].update(v)
    else:
        p[k] = v
p['name'] = new
open(out, 'w').write(f"# Plan {new}\n\nParent: `{par}`.\n\n```json\n{json.dumps(p, indent=1)}\n```\n")
