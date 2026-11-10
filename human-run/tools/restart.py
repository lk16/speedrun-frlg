#!/usr/bin/env python3
"""Random-restart check of the coordinate descent: perturb a plan, descend from it, report where it lands.

usage: restart.py SEED [--from CHAIN-WR80] [--rung WR+80m]
Creates plans/CHAIN-RS<SEED> (a perturbed copy of the plan the pointer names: fill list shuffled, budgets and
detours scaled by U(0.5, 2), reset aggressiveness and starter total jittered) and runs `descend.py --pointer`
on it at the fixed rung. stdlib only.
"""
import json, os, random, subprocess, sys

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.join(HERE, '..', '..')
args = sys.argv[1:]
seed = int(args[0])
src = args[args.index('--from') + 1] if '--from' in args else 'CHAIN-WR80'
rung = args[args.index('--rung') + 1] if '--rung' in args else 'WR+80m'
rnd = random.Random(seed)
cur = open(os.path.join(ROOT, 'human-run', 'plans', src)).read().strip()
p = json.loads(open(os.path.join(ROOT, 'human-run', 'plans', cur + '.md')).read().split('```json')[1].split('```')[0])
fill = p['fill']
rnd.shuffle(fill)
for h in fill:
    h['steps'] = max(8, int(h['steps'] * rnd.uniform(0.5, 2.0)))
p['reset_aggr'] = round(min(1.0, max(0.2, p.get('reset_aggr', 0.7) + rnd.uniform(-0.2, 0.2))), 2)
p['starter_min_total'] = p['starter_min_total'] + rnd.choice([-1, 0, 1])
p['fight_until_level'] = max(5, p['fight_until_level'] + rnd.choice([-2, -1, 0, 1, 2]))
name = f'RS{seed}'
p['name'] = name
pointer = 'CHAIN-' + name
plans = os.path.join(ROOT, 'human-run', 'plans')
open(os.path.join(plans, name + '.md'), 'w').write(f'# Plan {name}\n\nParent: `{cur}` perturbed with seed {seed} (random restart).\n\n```json\n{json.dumps(p, indent=1)}\n```\n')
open(os.path.join(plans, pointer), 'w').write(name + '\n')
env = dict(os.environ, FRLG_SCRATCH=f'/tmp/frlg-human-scratch-rs{seed}')
os.makedirs(env['FRLG_SCRATCH'], exist_ok=True)
subprocess.run([sys.executable, os.path.join(HERE, 'descend.py'), '--pointer', pointer, '--rung', rung, '--passes', '4'], env=env)
final = open(os.path.join(plans, pointer)).read().strip()
print('final', final)
