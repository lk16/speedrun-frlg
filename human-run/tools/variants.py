#!/usr/bin/env python3
"""The WR run's technique list, as paired comparisons against plans/CURRENT.

Each variant is one try.py call (same CRN pairing, same scores.jsonl logging) at the
fixed rung. Run after a descend pass so the baseline is locally optimal; a variant that
wins should then be re-descended (its own tactics may want retuning).

usage: variants.py [--rung WR+160m] [--cap 320000] [--only name,...]
"""
import json, os, subprocess, sys

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.join(HERE, '..', '..')
PLANS = os.path.join(ROOT, 'human-run', 'plans')

args = sys.argv[1:]
def opt(name, default):
    if name in args:
        return args[args.index(name) + 1]
    return default
RUNG = opt('--rung', 'WR+160m')
CAP = opt('--cap', '320000')
ONLY = opt('--only', '').split(',') if '--only' in args else None

cur = open(os.path.join(PLANS, 'CURRENT')).read().strip()
p = json.loads(open(os.path.join(PLANS, cur + '.md')).read().split('```json')[1].split('```')[0])

def no_fishing_patch():
    fills = [h for h in p.get('fill', []) if h.get('area') != 'fish']
    hunts = [h for h in p.get('hunts', []) if h.get('area') != 'fish']
    return {'fill': fills, 'hunts': hunts, 'rod': 'mixed'}

# (name, idea, patch) — one knob group each, the WR list.
VARIANTS = [
    ('firered', '[wr] version LG->FR (wild tables)', {'version': 'FR'}),
    ('nofish', '[wr] no fishing: drop fish fills+hunts and the rod trips', no_fishing_patch()),
    ('noabra', '[wr-inverse] teleport_abra off', {'teleport_abra': False}),
    ('nomewtwo', '[wr-inverse] mewtwo off (carry fights E4 R2)', {'mewtwo': False}),
    ('torrent', '[wr] stay in Torrent range: heal_below 0.2', {'heal_below': 0.2}),
    ('evospa', '[wr] evolution gate SpA>=28 (the "SpA>=30" family)',
     {'evo_min_stats': [0, 0, 0, 0, 28, 0]}),
    ('evospa30', '[wr] evolution gate SpA>=30', {'evo_min_stats': [0, 0, 0, 0, 30, 0]}),
    ('bulbasaur', '[starter] bulbasaur under the exact engine', {'starter': 'bulbasaur'}),
    ('charmander', '[starter] charmander under the exact engine', {'starter': 'charmander'}),
]

for name, idea, patch in VARIANTS:
    if ONLY and name not in ONLY:
        continue
    print(f"=== {name}: {idea}", flush=True)
    r = subprocess.run([sys.executable, os.path.join(HERE, 'try.py'),
                        os.path.join(PLANS, cur + '.md'), idea, json.dumps(patch),
                        '--name', f'var-{name}', '--cap', CAP, '--rung', RUNG, '--seed', '1'],
                       capture_output=True, text=True)
    for l in r.stdout.strip().splitlines():
        print('  ' + l, flush=True)
