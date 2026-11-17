#!/usr/bin/env python3
"""Write a plan file from a base plan with overrides and/or an optimal-stopping reset table. stdlib only.

usage: optstop_plan.py BASE_PLAN OUT_PLAN [--name N] [--set key=json ...] [--table optstop.json ...] [--clear-table]

--table takes the JSON line printed by `frlg-human optstop` ({"rung":..,"table":{seg:secs}}); several may be
given (one per rung). --set key=json sets a top-level plan key (e.g. trainer_fit=0.85). The output keeps the
base plan's text outside the json block, with a one-line header naming the change.
"""
import json, re, sys

args = sys.argv[1:]
base, out = args[0], args[1]
name = None
sets, tables, clear = [], [], False
i = 2
while i < len(args):
    if args[i] == '--name':
        name = args[i + 1]; i += 2
    elif args[i] == '--set':
        sets.append(args[i + 1]); i += 2
    elif args[i] == '--table':
        tables.append(args[i + 1]); i += 2
    elif args[i] == '--clear-table':
        clear = True; i += 1
    else:
        sys.exit('bad arg ' + args[i])
text = open(base).read()
m = re.search(r'```json\n(.*?)\n```', text, re.S)
d = json.loads(m.group(1))
if clear:
    d['reset_table'] = {}
for s in sets:
    k, v = s.split('=', 1)
    d[k] = json.loads(v)
for t in tables:
    for line in open(t):
        line = line.strip()
        if line.startswith('{'):
            j = json.loads(line)
            d.setdefault('reset_table', {})[j['rung']] = j['table']
            d.setdefault('reset_adj', {})[j['rung']] = j.get('adj', {})
if name:
    d['name'] = name
new = text[:m.start(1)] + json.dumps(d, indent=1) + text[m.end(1):]
open(out, 'w').write(new)
