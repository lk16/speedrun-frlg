#!/usr/bin/env python3
"""Per-draw paired ratio of two rb_mc design files (same seed0/N): rb_pair.py A.json B.json (B/A)."""
import json, math, sys

a = {r['i']: r for r in json.load(open(sys.argv[1]))}
b = {r['i']: r for r in json.load(open(sys.argv[2]))}
for rg in ('WR+2m', 'WR'):
    d = [math.log(b[i]['eht'][rg] / a[i]['eht'][rg]) for i in a if i in b and a[i]['eht'].get(rg) and b[i]['eht'].get(rg) and a[i]['fin'][rg] >= 30 and b[i]['fin'][rg] >= 30]
    m = sum(d) / len(d)
    sd = math.sqrt(sum((x - m) ** 2 for x in d) / (len(d) - 1))
    se = sd / math.sqrt(len(d))
    d.sort()
    print(f'{rg}: n={len(d)} mean ln ratio {m:+.3f} (x{math.exp(m):.3f}) se {se:.3f}; draws where B better: {sum(1 for x in d if x < 0)}/{len(d)}; worst +{max(d):.2f}, best {min(d):.2f}')
