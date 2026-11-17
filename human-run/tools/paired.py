#!/usr/bin/env python3
"""One idea, paired against a parent at R = 3.2M on two fresh seeds and two rungs (WR+2m, WR).

usage: paired.py NAME "idea" '{"param": value}' (or @patch.json) [--parent PLAN.md] [--seeds 71,72] [--rungs WR+2m,WR] [--R 3200000]

Runs try.py (fixed-rung mode, one compare of R paired attempts per cell: the child and the parent see the same
seed set) and prints one line per cell: child EHT, parent EHT, paired difference and its 95% CI (negative is
better). Every cell is also logged in scores.jsonl by try.py. stdlib only.
"""
import json, os, subprocess, sys

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.join(HERE, '..', '..')
args = sys.argv[1:]


def opt(name, default):
    if name in args:
        i = args.index(name)
        v = args[i + 1]
        del args[i:i + 2]
        return v
    return default


parent = opt('--parent', os.path.join(ROOT, 'human-run', 'plans', open(os.path.join(ROOT, 'human-run', 'plans', 'CURRENT')).read().strip() + '.md'))
seeds = opt('--seeds', '71,72').split(',')
rungs = opt('--rungs', 'WR+2m,WR').split(',')
R = opt('--R', '3200000')
name, idea, patch = args[0], args[1], args[2]
env = dict(os.environ, FRLG_R0=R)
env.setdefault('FRLG_SCRATCH', os.path.join(os.environ.get('TMPDIR', '/tmp'), 'frlg-human-scratch'))
for rung in rungs:
    for seed in seeds:
        r = subprocess.run([sys.executable, os.path.join(HERE, 'try.py'), parent, idea + f' [{rung} seed {seed}]', patch,
                            '--name', name, '--cap', R, '--seed', seed, '--rung', rung],
                           capture_output=True, text=True, env=env)
        lines = [l for l in r.stdout.strip().splitlines() if l.startswith(('parent', 'ACCEPT', 'refuted'))]
        print(f'{name} {rung} seed {seed}: ' + ' | '.join(lines) + (r.stderr[-300:] if r.returncode else ''), flush=True)
