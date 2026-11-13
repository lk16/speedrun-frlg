#!/usr/bin/env python3
"""Hit pattern of the wild-encounter dice stream for every u16 seed.

decompiled/src/wild_encounter.c:302-332 (rate test), :661-671 (seed + LCG), :778-784 (buff),
:760-770 (buff reset on hit).  A 'hit' resets the buff (the route flees/fights, which also resets
the cooldown, so the next test happens >=6-7 steps later; here every roll is a test).
Usage: wild_seed_scan.py  -> prints distribution summaries as JSON; importable: hits(seed, rates)
"""
import json, sys
M = 1103515245
def hits(seed, rates):
    """rates: list of map encounter rates (21 for Route 1/2, 14 forest), one per rate test.
    Returns list of indices that hit."""
    x = seed & 0xFFFF; buff = 0; out = []
    for i, r in enumerate(rates):
        x = (x * M + 12345) & 0xFFFFFFFF
        rate = min(r * 16 + buff * 16 // 200, 1600)
        if (x >> 16) % 1600 < rate:
            out.append(i); buff = 0
        else:
            buff += r
    return out
def summarize(rates, label):
    cnt = {}
    first_hit = []
    for s in range(65536):
        h = hits(s, rates)
        cnt[len(h)] = cnt.get(len(h), 0) + 1
        first_hit.append(h[0] if h else len(rates))
    tot = 65536
    cum = 0; cdf = {}
    for k in sorted(cnt):
        cum += cnt[k]; cdf[k] = round(cum / tot, 5)
    fh = sorted(first_hit)
    return {"label": label, "hits_pmf": {k: round(v / tot, 5) for k, v in sorted(cnt.items())},
            "P(hits<=k)": cdf, "first_hit_median": fh[tot // 2], "first_hit_p90": fh[int(tot * .9)]}
if __name__ == "__main__":
    print(json.dumps([summarize([14] * 60, "60 forest-rate (14) rolls from stream start"),
                      summarize([21] * 31 + [14] * 60, "31 Route1/2-rate (21) rolls then 60 forest rolls")], indent=1))
