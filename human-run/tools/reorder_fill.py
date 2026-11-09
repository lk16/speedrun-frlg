#!/usr/bin/env python3
"""Write a patch file reordering a plan's fill list by measured seconds per new species.

usage: reorder_fill.py PLAN.md OUT.json   (runs `frlg-human dist`; stdlib only)
"""
import json, re, subprocess, sys, os
root = os.path.join(os.path.dirname(os.path.abspath(__file__)), '..', '..')
plan = sys.argv[1]
params = json.loads(open(plan).read().split('```json')[1].split('```')[0])
out = subprocess.check_output([os.path.join(root, 'target', 'release', 'frlg-human'), 'dist', plan, '--R', '20000'], text=True)
cost = {}
for l in out.splitlines():
    m = re.match(r'\s+fill (\S+)\s+(\S+)\s+run.*->\s+(\d+) s/species', l)
    if m:
        cost[('MAP_' + m.group(1), m.group(2))] = int(m.group(3))
fill = sorted(params['fill'], key=lambda h: cost.get((h['site'], h['area']), 10**6))
json.dump({'fill': fill}, open(sys.argv[2], 'w'))
print([(h['site'][4:], cost.get((h['site'], h['area']))) for h in fill])
