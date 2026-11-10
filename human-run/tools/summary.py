#!/usr/bin/env python3
"""Print a human-readable summary of a plan's parameter block (default: plans/CURRENT).

usage: summary.py [plan-name]   stdlib only.
Reads human-run/plans/NAME.md and human-run/scores.jsonl; every rule it states is the simulator's
parameter, not a claim about the game (see 001-crude.md for the citations).
"""
import json, os, sys

ROOT = os.path.join(os.path.dirname(os.path.abspath(__file__)), '..', '..')
PLANS = os.path.join(ROOT, 'human-run', 'plans')
name = sys.argv[1] if len(sys.argv) > 1 else open(os.path.join(PLANS, 'CURRENT')).read().strip()
p = json.loads(open(os.path.join(PLANS, name + '.md')).read().split('```json')[1].split('```')[0])
stats = ['HP', 'Atk', 'Def', 'Spe', 'SpA', 'SpD']
print(f'# Plan {name} in words\n')
print(f"Version {p.get('version', 'FR')}, starter {p.get('starter', 'squirtle')}.\n")
print('## Starter')
sm = p.get('starter_min_stats', [0] * 6)
gates = [f'{s} >= {v}' for s, v in zip(stats, sm) if v]
print(f"Soft-reset after choosing the starter unless its level-5 stat total is >= {p.get('starter_min_total', 0)}"
      + (f" and {', '.join(gates)}" if gates else '') + ' (read off the summary screen).\n')
print('## Resets during the run')
print(f"At each split reset when elapsed + floor + a*(mean - floor) > T; a = {p.get('reset_aggr', 0)} unless overridden:")
for k, v in p.get('reset_aggr_at', {}).items():
    print(f'  - {k}: {v}')
print()
print('## Catching and fighting')
print(f"* Balls: prefer {p.get('ball', 'poke')} (money model: {p.get('money_model', False)}, batch {p.get('ball_batch', 10)}).")
print(f"* Passing wild encounters: throw at new Kanto species with catch rate >= {p.get('pass_min_catch', 0)}; flee the rest.")
print(f"* Fight wild mons only while the carry is below level {p.get('fight_until_level', 0)}.")
print(f"* Weaken before throwing: {p.get('weaken', False)}; teleport back with Abra: {p.get('teleport_abra', False)}; bike: {p.get('bike', True)}; rod for fish hunts: {p.get('rod', 'mixed')}.")
if p.get('repel_on'):
    print('* Repels (bought from Pewter on) on the grass of: ' + ', '.join(p['repel_on']) + '.')
print(f"* Mewtwo (Master Ball) in the post-game: {p.get('mewtwo', False)}.\n")
if p.get('hunts'):
    print('## In-route hunts')
    for h in p['hunts']:
        print(f"  - after {h['after']}: {h['site']} {h.get('area', 'land')}, <= {h['steps']} steps, leave alone {len(h.get('skip', []))} species")
    print()
print('## Gifts, statics, trades')
for s in p.get('specials', []):
    print(f"  - after {s['after']}: {s['kind']} {', '.join(x.replace('SPECIES_', '').title() for x in s['species'])}"
          + (f" (needs {', '.join(x.replace('SPECIES_', '').title() for x in s['requires'])})" if s.get('requires') else ''))
print()
print("## Oak's check: fill list (after the Hall of Fame, when fewer than 60 Kanto species are caught)")
for i, h in enumerate(p.get('fill', [])):
    print(f"  {i + 1}. {h['site'].replace('MAP_', '')} {h.get('area', 'land')}: up to {h['steps']} {'steps' if h.get('area', 'land') == 'land' else 'casts'}, detour {h.get('detour_tiles', 0)} tiles each way")
print()
print('## Trainers walked around (assumptions; see human-run/forced-audit.md for the audited ones)')
AUDITED = {'forest-trainers', 'route3-trainers', 'mt-moon-trainers', 'cerulean-gym-trainer', 'celadon-gym-trainers',
           'viridian-gym-trainers', 'fuchsia-gym-trainers', 'saffron-gym-trainers', 'mt-ember-trainers', 'victory-road-trainers'}
skips = p.get('skip_groups', [])
if p.get('audited_forced', False):
    skips = [g for g in skips if g not in AUDITED]
print('  unaudited, assumed walked around: ' + ', '.join(skips) + f"; audited_forced={p.get('audited_forced', False)}")
last = None
for line in open(os.path.join(ROOT, 'human-run', 'scores.jsonl')):
    r = json.loads(line)
    if r.get('plan') == name:
        last = r
if last:
    print(f"\nScore recorded with the plan: rung {last.get('rung')}, EHT {last.get('eht_hours')} h at R={last.get('R')} (a score of the model).")
