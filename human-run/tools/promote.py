#!/usr/bin/env python3
"""Promote an accepted variant to human-run/plans/NNN-name.md and update CURRENT.

usage: promote.py VARIANT.md NNN-name "what changed and why"
Reads the last scores.jsonl line for the variant's plan name. stdlib only.
"""
import json, os, sys

ROOT = os.path.join(os.path.dirname(os.path.abspath(__file__)), '..', '..')
variant, name, why = sys.argv[1], sys.argv[2], sys.argv[3]
params = json.loads(open(variant).read().split('```json')[1].split('```')[0])
old = params['name']
params['name'] = name
rec = None
for line in open(os.path.join(ROOT, 'human-run', 'scores.jsonl')):
    r = json.loads(line)
    if r.get('plan') == old:
        rec = r
POINTER = os.environ.get('FRLG_POINTER', 'CURRENT')
cur = open(os.path.join(ROOT, 'human-run', 'plans', POINTER)).read().strip()
parent_md = os.path.join(ROOT, 'human-run', 'plans', cur + '.md')
doc = f"""# Plan {name}

Parent: `{cur}`. Change: {why}

The step-by-step description, citations and branch rules in `001-crude.md` still apply except
where the parameters below differ; `frlg-human` reads only the block below. Every segment cost
is a model estimate (see `001-crude.md` for what is ledger, tile-count or guess).

```json
{json.dumps(params, indent=1)}
```

## Score

{('- rung **' + str(rec['rung']) + '**, EHT ' + str(rec.get('eht_hours')) + ' h at R=' + str(rec.get('R')) + ', 95% CI of the paired difference to the parent ' + str(rec.get('ci')) + ' (negative is better), parent EHT ' + str(rec.get('parent_eht'))) if rec else '- (no score line found)'}
- evaluation line: `human-run/scores.jsonl`, plan `{old}`; ledger of everything tried is in that file.
"""
out = os.path.join(ROOT, 'human-run', 'plans', name + '.md')
open(out, 'w').write(doc)
open(os.path.join(ROOT, 'human-run', 'plans', POINTER), 'w').write(name + '\n')
print('wrote', out)
