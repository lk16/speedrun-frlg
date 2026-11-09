#!/usr/bin/env python3
"""Write a scratch plan file: the parent's parameter block with top-level keys replaced.

usage: variant.py PARENT.md OUT.md '{"reset_aggr": 1.0}'
A value of null deletes the key (back to the default). stdlib only.
"""
import json, sys

parent, out, patch = sys.argv[1], sys.argv[2], json.loads(sys.argv[3])
params = json.loads(open(parent).read().split('```json')[1].split('```')[0])
for k, v in patch.items():
    if v is None:
        params.pop(k, None)
    else:
        params[k] = v
open(out, 'w').write('```json\n' + json.dumps(params, indent=1) + '\n```\n')
