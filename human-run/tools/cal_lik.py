#!/usr/bin/env python3
"""cal_lik.py FIGHTDIST.jsonl [round]: log-likelihood of the observed per-fight player action counts (WR video + the
2026-11-14 stream, two samples per fight) under the simulator's per-fight distribution among fights WON, from
`frlg-human fightdist`. P(sample) = P(|actions - obs| <= 1) / 3 per sample (the video counts are +-1), floored at
0.5/n. round: r1 (League 1), r2 (League 2), mid, all. stdlib only."""
import json, math, sys
OBS = {
    # League round 1: WR, A1 (stream 2026-11-14)
    'lorelei': [13, 11], 'bruno': [9, 10], 'agatha': [7, 7], 'lance': [10, 7], 'champion': [12, 13],
    # League round 2: WR (agent recount 2026-12), A1
    'lorelei-2': [8, 7], 'bruno-2': [5, 6], 'agatha-2': [6, 6], 'lance-2': [8, 9], 'champion-2': [11, 11],
    # mid game: WR (wrsplits.rs), A1
    'misty': [7, 9], 'rival-ss-anne': [6, 9], 'koga': [8, 5], 'blaine': [6, 4], 'sabrina': [9, 7], 'giovanni': [6, 5],
    'rival-route22': [12, 9],
}
GROUPS = {'r1': ['lorelei', 'bruno', 'agatha', 'lance', 'champion'],
          'r2': ['lorelei-2', 'bruno-2', 'agatha-2', 'lance-2', 'champion-2'],
          'mid': ['misty', 'rival-ss-anne', 'koga', 'blaine', 'sabrina', 'giovanni', 'rival-route22']}


def load(path):
    xs = {}
    for l in open(path):
        r = json.loads(l)
        for n, a, won, fr in r['fights']:
            if won:
                xs.setdefault(n, []).append(a)
    return xs


def lik(xs, names):
    tot, parts = 0.0, []
    for f in names:
        v = xs.get(f)
        if not v:
            continue
        n = len(v)
        ll = 0.0
        for o in OBS[f]:
            k = sum(1 for x in v if abs(x - o) <= 1)
            ll += math.log(max(k, 0.5) / n / 3)
        tot += ll
        parts.append('%s %.2f' % (f, ll))
    return tot, parts


if __name__ == '__main__':
    xs = load(sys.argv[1])
    for g in ([sys.argv[2]] if len(sys.argv) > 2 and sys.argv[2] != 'all' else GROUPS):
        t, p = lik(xs, GROUPS[g])
        print('%s %-4s loglik %.2f | %s' % (sys.argv[1].split('/')[-1], g, t, ' '.join(p)))
