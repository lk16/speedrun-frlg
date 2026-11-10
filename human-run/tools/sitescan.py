#!/usr/bin/env python3
"""Rank every encounter site by seconds per new species at Oak's check (one site at a time).

usage: sitescan.py [plan.md] [--R n]   (default plans/CURRENT, R=1500). stdlib only.
For each site/area in dex.json with a Kanto species, builds a variant whose fill list is that site
alone (80 steps or 25 casts, detour 60 tiles) and reads `frlg-human dist`'s s/species line.
"""
import json, os, subprocess, sys, re
ROOT = os.path.join(os.path.dirname(os.path.abspath(__file__)), '..', '..')
BIN = os.environ.get('FRLG_BIN', os.path.join(ROOT, 'target', 'release', 'frlg-human'))
SCRATCH = os.environ.get('FRLG_SCRATCH', '/tmp/frlg-human-scratch')
args = sys.argv[1:]
R = 1500
if '--R' in args:
    i = args.index('--R'); R = int(args[i + 1]); del args[i:i + 2]
plan = args[0] if args else os.path.join(ROOT, 'human-run', 'plans', open(os.path.join(ROOT, 'human-run', 'plans', 'CURRENT')).read().strip() + '.md')
p = json.loads(open(plan).read().split('```json')[1].split('```')[0])
dex = json.load(open(os.path.join(ROOT, 'human-run', 'data', 'dex.json')))
sites = dex['sites_lg'] if p.get('version') == 'LG' else dex['sites']
res = []
for site, areas in sorted(sites.items()):
    for key, area in (('land_mons', 'land'), ('fishing_mons', 'fish')):
        a = areas.get(key)
        if not a:
            continue
        if not any(dex['species'].get(sp, {}).get('kanto') for _, _, sp in a['slots']):
            continue
        q = dict(p)
        q['fill'] = [dict(site=site, area=area, steps=80 if area == 'land' else 25, detour_tiles=60)]
        q['name'] = 'scan'
        f = os.path.join(SCRATCH, 'scan.md')
        open(f, 'w').write('```json\n' + json.dumps(q) + '\n```\n')
        out = subprocess.run([BIN, 'dist', f, '--R', str(R)], capture_output=True, text=True).stdout
        m = re.search(r'fill \S+\s+\S+\s+\S+\s+[\d.]+% mean\s+(\d+)s gain\s+([\d.]+) ->\s+(\d+) s/species', out)
        if m:
            res.append((int(m.group(3)), float(m.group(2)), int(m.group(1)), site, area))
res.sort()
print('s/species  gain/visit  seconds/visit  site area')
for r in res[:int(os.environ.get("TOP", "40"))]:
    print(f'{r[0]:9d} {r[1]:10.2f} {r[2]:12d}  {r[3]} {r[4]}')
