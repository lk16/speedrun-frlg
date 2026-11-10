#!/usr/bin/env python3
"""Run several try.py ideas against one parent: batch.py PARENT.md TAG idea1|patch1 idea2|patch2 ... (stdlib only).
Each argument is 'label=>{json patch}'. Prints one verdict line each."""
import subprocess, sys, os, json
here = os.path.dirname(os.path.abspath(__file__))
parent, tag = sys.argv[1], sys.argv[2]
cap = os.environ.get('CAP', '400000')
for i, a in enumerate(sys.argv[3:]):
    label, patch = a.split('=>', 1)
    out = subprocess.run([sys.executable, os.path.join(here, 'try.py'), parent, label, patch, '--name', f'{tag}{i}', '--cap', cap], capture_output=True, text=True).stdout.strip().splitlines()
    v = [l for l in out if l.startswith(('ACCEPT', 'refuted'))]
    p = [l for l in out if l.startswith('parent')]
    print(f'{tag}{i} {label}: {p[0] if p else ""} | {v[0] if v else out[-3:]}', flush=True)
