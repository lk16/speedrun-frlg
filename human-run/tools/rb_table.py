#!/usr/bin/env python3
"""Ratio table of an rb_var.py grid: EHT(variant)/EHT(base) per corner, worst and mean. usage: rb_table.py GRID.json"""
import json, sys

r = json.load(open(sys.argv[1]))
sc, vs = [], []
for x in r:
    if x['scen'] not in sc:
        sc.append(x['scen'])
    if x['variant'] not in vs:
        vs.append(x['variant'])
b = {x['scen']: x['eht'] for x in r if x['variant'] == 'base'}
print('variant'.ljust(16), ' '.join(s[:7].rjust(7) for s in sc), ' worst   mean')
for v in vs:
    row = [next((y['eht'] for y in r if y['variant'] == v and y['scen'] == s), None) for s in sc]
    rat = [a / b[s] if a else float('nan') for a, s in zip(row, sc)]
    print(v.ljust(16), ' '.join(f'{q:7.3f}' for q in rat), f'{max(rat):6.3f} {sum(rat) / len(rat):6.3f}')
print('base EHT', {s: round(b[s]) for s in sc})
