#!/usr/bin/env python3
"""Evaluate one idea against the current best plan, following human-run/PROMPT.md.

usage: try.py PARENT.md "idea text" '{"param": value}' (or @patch.json) [--accept-name NAME] [--cap N] [--seed S]

Finds each plan's rung (tightest ladder rung with >=50 finishes), applies the comparison rules
(tighter rung wins; same rung -> paired EHT difference, R grown until the 95% CI excludes 0 or is
narrower than 1% of the score), appends a line to human-run/scores.jsonl and prints the verdict.
Writes the variant to scratch; promoting it to plans/NNN-*.md is a separate, manual step.
stdlib only.
"""
import json, os, subprocess, sys, hashlib

ROOT = os.path.join(os.path.dirname(os.path.abspath(__file__)), '..', '..')
BIN = os.environ.get('FRLG_BIN', os.path.join(ROOT, 'target', 'release', 'frlg-human'))
SCRATCH = os.environ.get('FRLG_SCRATCH', '/tmp/frlg-human-scratch')
RUNGS = ['WR', 'WR+1m', 'WR+2m', 'WR+5m', 'WR+10m', 'WR+20m', 'WR+40m', 'WR+80m', 'WR+160m']
os.makedirs(SCRATCH, exist_ok=True)

args = sys.argv[1:]
def opt(name, default):
    if name in args:
        i = args.index(name)
        v = args[i + 1]
        del args[i:i + 2]
        return type(default)(v)
    return default
cap = opt('--cap', 160000)
seed = opt('--seed', 1)
name = opt('--name', '')
fixed = opt('--rung', '')
parent, idea = args[0], args[1]
patch = json.loads(open(args[2][1:]).read() if args[2].startswith('@') else args[2])

def run(*a):
    return subprocess.check_output([BIN, *a], text=True)

def ladder(path):
    out = json.loads(run('ladder', path, '--R', '10000', '--cap', str(cap), '--seed', str(seed)).strip().splitlines()[-1])
    return out

params = json.loads(open(parent).read().split('```json')[1].split('```')[0])
child = dict(params)
for k, v in patch.items():
    if v is None:
        child.pop(k, None)
    else:
        child[k] = v
child['name'] = name or ('try-' + hashlib.sha1(json.dumps(patch, sort_keys=True).encode()).hexdigest()[:6])
cpath = os.path.join(SCRATCH, child['name'] + '.md')
open(cpath, 'w').write('```json\n' + json.dumps(child, indent=1) + '\n```\n')

if fixed:
    # Objective fixed at one rung: no ladder, compare the two plans there (finishes >= 50 on both sides).
    rec = dict(plan=child['name'], parent=params.get('name'), idea=idea, diff=patch, seed=seed)
    r = 10000
    while True:
        out = run('compare', cpath, parent, '--rung', fixed, '--R', str(r), '--seed', str(seed))
        j = json.loads([l for l in out.splitlines() if l.startswith('JSON ')][0][5:])
        lo, hi, b = j['ci_lo'], j['ci_hi'], j['b_eht']
        if lo is not None and (lo > 0 or hi < 0 or (b and (hi - lo) < 0.01 * b)):
            break
        if r >= 2_000_000 or (lo is None and r >= 400_000):
            break
        r *= 2
    better = j['ci_hi'] is not None and j['ci_hi'] < 0 and (j.get('a_fin') or 0) >= 50
    inconclusive = j['ci_lo'] is not None and j['ci_lo'] <= 0 <= j['ci_hi']
    verdict = ('ACCEPT' if better else ('refuted (not significant)' if inconclusive else 'refuted (worse)')) + \
        f" at {fixed} (fixed rung): diff {j['diff']} h CI [{j['ci_lo']},{j['ci_hi']}] R={j['R']}"
    rec.update(rung=fixed, eht_hours=j['a_eht'], R=j['R'], ci=[j['ci_lo'], j['ci_hi']], diff_vs_parent=j['diff'], parent_eht=j['b_eht'])
    rec['accepted'] = verdict.startswith('ACCEPT')
    rec['verdict'] = verdict
    rec['commit'] = None
    with open(os.path.join(ROOT, 'human-run', 'scores.jsonl'), 'a') as f:
        f.write(json.dumps(rec) + '\n')
    print(f"parent {fixed} {j['b_eht']} h | child {fixed} {j['a_eht']} h")
    print(verdict)
    print('variant file:', cpath)
    sys.exit(0)

pl, cl = ladder(parent), ladder(cpath)
rank = lambda l: RUNGS.index(l['rung']) if l.get('rung') else 99
rec = dict(plan=child['name'], parent=params.get('name'), idea=idea, diff=patch, seed=seed)

def compare_at(rung, r0):
    r = r0
    while True:
        out = run('compare', cpath, parent, '--rung', rung, '--R', str(r), '--seed', str(seed))
        j = json.loads([l for l in out.splitlines() if l.startswith('JSON ')][0][5:])
        lo, hi, b = j['ci_lo'], j['ci_hi'], j['b_eht']
        if lo is not None and (lo > 0 or hi < 0 or (b and (hi - lo) < 0.01 * b)):
            return j
        if r >= 2_000_000 or lo is None and r >= 400_000:
            return j
        r *= 2

verdict = None
if rank(cl) == 99:
    verdict = 'refuted: fewer than 50 finishes even at WR+160m'
    rec.update(rung=None, eht_hours=None)
elif rank(cl) < rank(pl):
    rung = cl['rung']
    chk = json.loads([l for l in run('compare', cpath, parent, '--rung', rung, '--R', str(cl['R']), '--seed', str(seed)).splitlines() if l.startswith('JSON ')][0][5:])
    if chk['b_fin'] == 0:
        verdict = f"ACCEPT: tighter rung {rung} vs parent {pl.get('rung')} (parent has 0 finishes there)"
        rec.update(rung=rung, eht_hours=cl['eht_hours'], R=cl['R'], finishes=cl['finishes'])
    else:
        j = compare_at(rung, cl['R'])
        better = j['ci_hi'] is not None and j['ci_hi'] < 0
        verdict = ('ACCEPT' if better else 'refuted') + f" at tighter rung {rung}: diff {j['diff']} CI [{j['ci_lo']},{j['ci_hi']}]"
        rec.update(rung=rung, eht_hours=j['a_eht'], R=j['R'], ci=[j['ci_lo'], j['ci_hi']], diff_vs_parent=j['diff'])
elif rank(cl) > rank(pl):
    verdict = f"refuted: rung {cl['rung']} is looser than parent's {pl['rung']}"
    rec.update(rung=cl['rung'], eht_hours=cl['eht_hours'], R=cl['R'])
else:
    j = compare_at(cl['rung'], cl['R'])
    better = j['ci_hi'] is not None and j['ci_hi'] < 0
    inconclusive = j['ci_lo'] is not None and j['ci_lo'] <= 0 <= j['ci_hi']
    verdict = ('ACCEPT' if better else ('refuted (not significant)' if inconclusive else 'refuted (worse)')) + \
        f" at {cl['rung']}: diff {j['diff']} h CI [{j['ci_lo']},{j['ci_hi']}] R={j['R']}"
    rec.update(rung=cl['rung'], eht_hours=j['a_eht'], R=j['R'], ci=[j['ci_lo'], j['ci_hi']], diff_vs_parent=j['diff'],
               parent_eht=j['b_eht'])
rec['accepted'] = verdict.startswith('ACCEPT')
rec['verdict'] = verdict
rec['commit'] = None
with open(os.path.join(ROOT, 'human-run', 'scores.jsonl'), 'a') as f:
    f.write(json.dumps(rec) + '\n')
print(f"parent {pl.get('rung')} {pl.get('eht_hours')} h | child {cl.get('rung')} {cl.get('eht_hours')} h")
print(verdict)
print('variant file:', cpath)
