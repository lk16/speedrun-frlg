#!/usr/bin/env python3
"""Markdown sensitivity table from `sens.py grid` output: EHT per (scenario, plan), regret vs the best plan
of the scenario, worst case per plan. usage: sens_table.py GRID.json [PLAN ...]   (stdlib only)"""
import json, sys

res = json.load(open(sys.argv[1]))
plans = []
for r in res:
    if r['plan'] not in plans:
        plans.append(r['plan'])
if len(sys.argv) > 2:
    plans = sys.argv[2:]
scen = []
cell = {}
for r in res:
    key = 'primary' if r['knob'] == 'none' else f"{r['knob']} {r['level']}"
    if key not in scen:
        scen.append(key)
    cell[(key, r['plan'])] = r['eht'] if r['eht'] is not None and r['fin'] >= 50 else None
short = [p.split('-')[0] for p in plans]
print('| scenario | ' + ' | '.join(short) + ' | best |')
print('|---|' + '---:|' * (len(plans) + 1))
worst = {p: 0.0 for p in plans}
worst_abs = {p: 0.0 for p in plans}
rank_ok = 0
for s in scen:
    vals = {p: cell.get((s, p)) for p in plans}
    ok = [v for v in vals.values() if v is not None]
    best = min(ok) if ok else None
    cells = []
    for p in plans:
        v = vals[p]
        if v is None:
            cells.append('n/a')
            worst[p] = float('inf')
            worst_abs[p] = float('inf')
            continue
        cells.append(f'**{v:.0f}**' if v == best else f'{v:.0f}')
        worst[p] = max(worst[p], v / best)
        worst_abs[p] = max(worst_abs[p], v)
    bp = min((p for p in plans if vals[p] is not None), key=lambda p: vals[p]).split('-')[0] if ok else '-'
    print(f'| {s} | ' + ' | '.join(cells) + f' | {bp} |')
print()
print('| plan | worst regret (EHT / best of scenario) | worst EHT (h) |')
print('|---|---:|---:|')
for p in plans:
    print(f'| {p} | {worst[p]:.2f}x | {worst_abs[p]:.0f} |')
