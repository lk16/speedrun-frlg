#!/usr/bin/env python3
"""EHT of plans at every ladder rung: final_table.py PLAN [PLAN ...] [--R 100000]

For each plan prints, per rung (WR, +1m ... +160m), "EHT h (finishes)" or "-" when fewer than 50 finishes
occur in R attempts. A rung with fewer than 50 finishes is not reached (PROMPT.md). stdlib only.
"""
import json, os, subprocess, sys

ROOT = os.path.join(os.path.dirname(os.path.abspath(__file__)), '..', '..')
BIN = os.environ.get('FRLG_BIN', os.path.join(ROOT, 'target', 'release', 'frlg-human'))
args = sys.argv[1:]
R = 100000
if '--R' in args:
    i = args.index('--R'); R = int(args[i + 1]); del args[i:i + 2]
RUNGS = ['WR', 'WR+1m', 'WR+2m', 'WR+5m', 'WR+10m', 'WR+20m', 'WR+40m', 'WR+80m', 'WR+160m']
print(f'{"plan":24s}' + ''.join(f'{r:>16s}' for r in RUNGS))
for name in args:
    path = os.path.join(ROOT, 'human-run', 'plans', name + '.md')
    cells = []
    for i, r in enumerate(RUNGS):
        out = subprocess.run([BIN, 'eval', path, '--rung', str(i), '--R', str(R)], capture_output=True, text=True).stdout.strip().splitlines()
        line = out[-1] if out else ''
        try:
            fin = int(line.split('finishes')[1].split('/')[0])
            eht = float(line.split('EHT')[1].strip().rstrip(' h'))
        except Exception:
            fin, eht = 0, None
        cells.append(f'{eht:.1f}h ({fin})' if fin >= 50 and eht is not None else '-')
    print(f'{name:24s}' + ''.join(f'{c:>16s}' for c in cells))
