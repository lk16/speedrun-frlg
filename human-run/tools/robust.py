#!/usr/bin/env python3
"""Does a plan hold when the human model's knobs change? (PROMPT: "check that it holds at other N".)

usage: robust.py PLAN [PLAN ...] [--rung 7] [--R 20000]

For every plan prints EHT at the given rung under: the plan's own model, jitter N in {0, 2, 8}, reaction
delays U{0,0} and U{30,60}, mistake_rate {0, 2}, text_factor {1.0, 1.5}, path_overhead {1.0, 1.3}.
Plans are names under human-run/plans/. stdlib only.
"""
import json, os, subprocess, sys

ROOT = os.path.join(os.path.dirname(os.path.abspath(__file__)), '..', '..')
BIN = os.environ.get('FRLG_BIN', os.path.join(ROOT, 'target', 'release', 'frlg-human'))
SCRATCH = os.environ.setdefault('FRLG_SCRATCH', '/tmp/frlg-human-scratch')
args = sys.argv[1:]
rung, R = 7, 20000
for flag in ('--rung', '--R'):
    if flag in args:
        i = args.index(flag)
        v = int(args[i + 1])
        if flag == '--rung':
            rung = v
        else:
            R = v
        del args[i:i + 2]
VARIANTS = [
    ('own', {}),
    ('jitter 0', {'jitter_n': 0}),
    ('jitter 2', {'jitter_n': 2}),
    ('jitter 8', {'jitter_n': 8}),
    ('react 0', {'react_lo': 0, 'react_hi': 0}),
    ('react 30-60', {'react_lo': 30, 'react_hi': 60}),
    ('mistakes 0', {'mistake_rate': 0.0}),
    ('mistakes 2', {'mistake_rate': 2.0}),
    ('text 1.0', {'text_factor': 1.0}),
    ('text 1.5', {'text_factor': 1.5}),
    ('path 1.0', {'path_overhead': 1.0}),
    ('path 1.3', {'path_overhead': 1.3}),
]
rows = {}
for name in args:
    base = json.loads(open(os.path.join(ROOT, 'human-run', 'plans', name + '.md')).read().split('```json')[1].split('```')[0])
    for label, patch in VARIANTS:
        q = dict(base)
        q.update(patch)
        q['name'] = 'rb'
        f = os.path.join(SCRATCH, 'rb.md')
        open(f, 'w').write('```json\n' + json.dumps(q) + '\n```\n')
        out = subprocess.run([BIN, 'eval', f, '--rung', str(rung), '--R', str(R)], capture_output=True, text=True).stdout.strip().splitlines()
        line = out[-1] if out else ''
        # "name @ WR+80m: finishes 123/20000 EHT 4.567 h"
        eht = line.split('EHT')[-1].strip().rstrip(' h') if 'EHT' in line else 'n/a'
        fin = line.split('finishes')[-1].split('/')[0].strip() if 'finishes' in line else '0'
        rows.setdefault(label, {})[name] = f'{eht} ({fin})'
w = max(len(n) for n in args) + 2
print(f'{"variant":14s}' + ''.join(f'{n:>{w + 8}s}' for n in args))
for label, _ in VARIANTS:
    print(f'{label:14s}' + ''.join(f'{rows[label][n]:>{w + 8}s}' for n in args))
