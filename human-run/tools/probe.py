#!/usr/bin/env python3
"""Paired probes of many patches against one parent plan (one frlg-human `compare` each).

usage: probe.py PARENT.md RUNG R SEED OUT.jsonl LABEL1 'PATCH1' [LABEL2 'PATCH2' ...]
Patches are JSON objects; a key like "manip.min_spe" or "fight_tactics.brock.potions" sets a nested
value (dotted path). Variants are written to $FRLG_SCRATCH (default /tmp/frlg-human-scratch).
Each result line (label, diff = parent - variant is NOT used: diff = variant - parent, + = worse)
is printed and appended to OUT.jsonl. Nothing is written to scores.jsonl. stdlib only.
"""
import json, os, subprocess, sys

ROOT = os.path.join(os.path.dirname(os.path.abspath(__file__)), '..', '..')
BIN = os.environ.get('FRLG_BIN', os.path.join(ROOT, 'target', 'release', 'frlg-human'))
SCRATCH = os.environ.get('FRLG_SCRATCH', '/tmp/frlg-human-scratch')
os.makedirs(SCRATCH, exist_ok=True)


def setpath(d, path, v):
    ks = path.split('.')
    for k in ks[:-1]:
        d = d.setdefault(k, {})
    if v is None:
        d.pop(ks[-1], None)
    else:
        d[ks[-1]] = v


def main():
    parent, rung, R, seed, out = sys.argv[1:6]
    rest = sys.argv[6:]
    params = json.loads(open(parent).read().split('```json')[1].split('```')[0])
    for label, patch in zip(rest[0::2], rest[1::2]):
        child = json.loads(json.dumps(params))
        for k, v in json.loads(patch).items():
            setpath(child, k, v)
        child['name'] = 'probe-' + label
        path = os.path.join(SCRATCH, 'probe-%s.md' % label)
        open(path, 'w').write('```json\n' + json.dumps(child, indent=1) + '\n```\n')
        r = subprocess.run([BIN, 'compare', parent, path, '--rung', rung, '--R', R, '--seed', seed],
                           capture_output=True, text=True)
        if r.returncode != 0:
            print(label, 'ERROR', r.stderr.strip()[-200:], flush=True)
            continue
        js = [l for l in r.stdout.splitlines() if l.startswith('JSON ')]
        j = json.loads(js[0][5:])
        # compare prints a = parent, b = variant; diff = a - b, so variant - parent = -diff
        d = None if j['diff'] is None else -j['diff']
        lo = None if j['ci_hi'] is None else -j['ci_hi']
        hi = None if j['ci_lo'] is None else -j['ci_lo']
        rec = dict(label=label, patch=json.loads(patch), rung=rung, R=int(R), seed=int(seed),
                   parent=j['a_eht'], variant=j['b_eht'], parent_fin=j['a_fin'], variant_fin=j['b_fin'],
                   diff=d, ci=[lo, hi])
        with open(out, 'a') as f:
            f.write(json.dumps(rec) + '\n')
        print('%-28s parent %s (%d) variant %s (%d)  diff %s  CI %s' % (
            label, j['a_eht'], j['a_fin'], j['b_eht'], j['b_fin'],
            None if d is None else '%+.2f' % d,
            None if lo is None or hi is None else '[%+.2f, %+.2f]' % (lo, hi)), flush=True)


main()
