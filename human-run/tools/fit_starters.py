#!/usr/bin/env python3
"""Starter-box evidence for the manip model (hit rate p, reject cycle time). stdlib only.

p^3 = P(an attempt keeps its starter) = ACCEPTED / (ACCEPTED + REJECTED*) over the session files of
video-analysis/data/starters (the box is a prediction overlay; a reject at the stats screen / lab is a
missed press). Sessions 2026-11-05 (box static, lower-bound readings) and 2026-11-09 (RETARGETED, other
mode) are listed but left out of the pool.
"""
import glob, json, math, os, statistics as S

D = os.path.join(os.path.dirname(os.path.abspath(__file__)), '..', 'video-analysis', 'data', 'starters')
SKIP = {'2026-11-05.jsonl', '2026-11-09.jsonl'}


def wilson(k, n, z=1.96):
    p = k / n
    d = 1 + z * z / n
    c = p + z * z / (2 * n)
    s = z * math.sqrt(p * (1 - p) / n + z * z / (4 * n * n))
    return (c - s) / d, (c + s) / d


rows = {}
for f in sorted(glob.glob(os.path.join(D, '*.jsonl'))):
    rows[os.path.basename(f)] = [json.loads(l) for l in open(f)]

print('session                acc  rej   rate   p=rate^(1/3)  95% CI of p')
A = N = 0
rates = []
for name, rs in rows.items():
    a = sum(r['outcome'] == 'ACCEPTED' for r in rs)
    j = sum(r['outcome'].startswith('REJ') for r in rs)
    if a + j == 0:
        continue
    lo, hi = wilson(a, a + j)
    tag = '  (not pooled)' if name in SKIP else ''
    print(f'{name:22s} {a:3d} {j:4d}  {a / (a + j):.3f}  {(a / (a + j)) ** (1 / 3):.3f}        {lo ** (1 / 3):.3f}-{hi ** (1 / 3):.3f}{tag}')
    if name not in SKIP:
        A += a
        N += a + j
        rates.append((a, a + j))
lo, hi = wilson(A, N)
print(f'pooled {A}/{N} = {A / N:.3f}  p = {(A / N) ** (1 / 3):.3f}  CI {lo ** (1 / 3):.3f}-{hi ** (1 / 3):.3f}')
# overdispersion: chi-square against the pooled rate
chi = sum((a - n * A / N) ** 2 / (n * (A / N) * (1 - A / N)) for a, n in rates)
print(f'heterogeneity chi2 = {chi:.1f} on {len(rates) - 1} df (session-to-session spread beyond binomial)')
r = [a / n for a, n in rates]
print(f'session rates min {min(r):.2f} max {max(r):.2f}  -> p {min(r) ** (1 / 3):.2f}..{max(r) ** (1 / 3):.2f}')

# the four sessions the old fit used
old = ['2026-11-06.jsonl', '2026-11-07.jsonl', '2026-11-10.jsonl', '2026-11-11.jsonl']
a = sum(sum(r['outcome'] == 'ACCEPTED' for r in rows[n]) for n in old)
j = sum(sum(r['outcome'].startswith('REJ') for r in rows[n]) for n in old)
print(f'the 4 sessions of the 0.692 fit: {a}/{a + j} = {a / (a + j):.3f} (p {(a / (a + j)) ** (1 / 3):.3f})')


# reject cycle: start-to-start of consecutive final starters, a rejected attempt followed directly by another
def cycles(name, tkey='t'):
    rs = [r for r in rows[name] if r.get('role') == 'final']
    rs.sort(key=lambda r: r[tkey])
    out = []
    for a, b in zip(rs, rs[1:]):
        if a['outcome'] == 'REJECTED':
            out.append(b[tkey] - a[tkey])
    return out


allc = []
for name in ('2026-11-11.jsonl', '2026-11-12-twitch.jsonl'):
    c = cycles(name)
    allc += c
    print(f'{name}: reject start-to-start n={len(c)} median {S.median(c):.0f} s mean {S.mean(c):.0f} s trimmed(<=600) mean {S.mean([x for x in c if x <= 600]):.0f} s')
tr = [x for x in allc if x <= 600]
print(f'pooled n={len(allc)} median {S.median(allc):.0f} mean {S.mean(allc):.0f} trimmed mean {S.mean(tr):.0f} ({len(allc) - len(tr)} above 600 s dropped as breaks)')
