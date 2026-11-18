#!/usr/bin/env python3
"""cal_fights.py PLAN [lp=..] [mp=..] [cp=..] [R=4000]: the `frlg-human splits` fight-length table (model player
actions vs the WR video) for a plan patched with the stand-in constants. Prints one line per fight. stdlib only."""
import os, re, subprocess, sys
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import rb_lib as L


def table(plan, tag, R=4000, seed=1, **kw):
    q = L.patch(L.load_plan(plan), **kw)
    f = L.write(q, tag)
    out = subprocess.run([L.BIN, 'splits', f, '--R', str(R), '--seed', str(seed)], capture_output=True, text=True).stdout
    res = {}
    for l in out.splitlines():
        m = re.match(r'\s+(\S+)\s+model\s+([\d.]+) actions core\s+(\d+) s .*won\s+(\d+)%, n=\s*(\d+)\) \| WR\s+([\d.]+) actions', l)
        if m:
            res[m.group(1)] = dict(actions=float(m.group(2)), core=int(m.group(3)), won=int(m.group(4)), n=int(m.group(5)), wr=float(m.group(6)))
    return res


if __name__ == '__main__':
    plan = sys.argv[1]
    kw = {}
    R = 4000
    for a in sys.argv[2:]:
        k, v = a.split('=')
        if k == 'R':
            R = int(v)
        else:
            kw[k] = float(v)
    t = table(plan, 'cf', R=R, **kw)
    for k, v in t.items():
        print('%-14s actions %5.1f (WR %4.1f) core %4d s won %3d%% n=%d' % (k, v['actions'], v['wr'], v['core'], v['won'], v['n']))
