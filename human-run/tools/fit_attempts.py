#!/usr/bin/env python3
"""Fit the runner's observed attempts (fidelity fit 2026-11-14).

usage: fit_attempts.py [--json]

Reads `video-analysis/data/accepted-runs.json` (every attempt past the starter gate on the
streams, with the run-timer minute where it ended) and prints
  * the survival table by window: at risk, deaths, conditional survival, Wilson 95% interval,
  * the pass rate of the Brock split and the share that reaches the E4R1 door,
  * the runner's window-to-window deviations (deep run vs the LG PB, 13 windows) against the
    model's window CV (`frlg-human splits` prints it as WINDOW rows).
Stdlib only.
"""
import json, math, os, sys

HERE = os.path.dirname(os.path.abspath(__file__))
DATA = os.path.join(HERE, '..', 'video-analysis', 'data', 'accepted-runs.json')

# Window edges in run-timer minutes. The runner's splits (deep run) sit inside each window; a
# 0.4 min margin keeps a run that reset at the split in the window it was playing.
WINDOWS = [
    # name, lo, hi, model segments the window covers
    ('forest-brock', 0.0, 14.5, 'forest-trainers pewter-gym-trainer brock'),
    ('brock-misty', 14.5, 28.0, 'route3 .. misty rival-cerulean'),
    ('misty-rocktunnel', 28.0, 55.5, 'nugget-bridge .. bike .. rock-tunnel-trainers'),
    ('rt-snorlax', 55.5, 80.0, 'hideout, tower, surge, snorlax'),
    ('snorlax-koga', 80.0, 90.0, 'safari, koga'),
    ('koga-sevii', 90.0, 114.0, 'power plant, mansion, blaine, sevii'),
    ('sevii-gio3', 114.0, 144.0, 'silph, sabrina, erika, giovanni 3'),
    ('gio3-door', 144.0, 157.0, 'route 22, victory road, dex fill'),
]


def wilson(k, n, z=1.96):
    if n == 0:
        return (0.0, 1.0)
    p = k / n
    d = 1 + z * z / n
    c = (p + z * z / (2 * n)) / d
    h = z * math.sqrt(p * (1 - p) / n + z * z / (4 * n * n)) / d
    return (max(0.0, c - h), min(1.0, c + h))


def load():
    return json.load(open(DATA))


def table(runs):
    rows = []
    at_risk = sum(1 for r in runs if r['kind'] != 'censored')
    surv = 1.0
    out = []
    n = at_risk
    for name, lo, hi, segs in WINDOWS:
        deaths = sum(1 for r in runs if r['kind'] == 'death' and lo <= r['end'] < hi)
        cens = sum(1 for r in runs if r['kind'] == 'censored' and lo <= r['end'] < hi)
        s = 1 - deaths / n if n else 1.0
        lo_c, hi_c = wilson(n - deaths, n)
        out.append({'window': name, 'segs': segs, 'at_risk': n, 'deaths': deaths, 'cond_surv': s,
                    'ci': [lo_c, hi_c], 'cum_surv': surv * s})
        surv *= s
        n = n - deaths - cens
    door = [r for r in runs if r['end'] >= 157.0 and r['kind'] in ('pace', 'finish')]
    return out, n, door


def main():
    d = load()
    runs = d['runs']
    out, reach_door, door = table(runs)
    total = sum(1 for r in runs if r['kind'] != 'censored')
    print(f'attempts past the starter gate: {total} (+{len(runs) - total} censored); sessions: '
          + ', '.join(sorted({r["s"] for r in runs})))
    print(f'{"window":<18}{"at risk":>8}{"deaths":>7}{"cond":>7}{"95% CI":>16}{"cum":>7}  model segments')
    for o in out:
        print(f'{o["window"]:<18}{o["at_risk"]:>8}{o["deaths"]:>7}{o["cond_surv"]:>7.2f}'
              f'{"[%.2f, %.2f]" % tuple(o["ci"]):>16}{o["cum_surv"]:>7.2f}  {o["segs"]}')
    fin = [r for r in door if r['kind'] == 'finish']
    print(f'reached the E4R1 door (>= 2:37): {len(door)} of {total} = {len(door) / total:.3f} '
          f'(Wilson {wilson(len(door), total)[0]:.3f}-{wilson(len(door), total)[1]:.3f}); '
          f'finished: {len(fin)}; door resets at the pace gate: {sum(1 for r in door if r["kind"] == "pace")}')
    brock_ok = sum(1 for r in runs if r['kind'] != 'censored' and r['end'] >= 14.5)
    print(f'passed the Brock split: {brock_ok}/{total} = {brock_ok / total:.3f} '
          f'(Wilson {wilson(brock_ok, total)[0]:.3f}-{wilson(brock_ok, total)[1]:.3f}); '
          f'the runner quotes 2/12 and 3/14 on bad days, 75% on a good one (STRATEGY 2, 7)')
    # Window deviations of the deep run against the LG PB: how much a split window varies.
    sp = d['splits_min']
    deep, pb = sp['deep'][:13], sp['lg_pb'][:13]
    seg_d = [deep[0]] + [deep[i] - deep[i - 1] for i in range(1, 13)]
    seg_p = [pb[0]] + [pb[i] - pb[i - 1] for i in range(1, 13)]
    dev = [(a - b) * 60 for a, b in zip(seg_d, seg_p)]
    cv = [(a - b) / b for a, b in zip(seg_d, seg_p)]
    m = sum(dev) / len(dev)
    sd = math.sqrt(sum((x - m) ** 2 for x in dev) / (len(dev) - 1))
    mcv = math.sqrt(sum(x * x for x in cv) / len(cv))
    print(f'deep run vs LG PB, 13 windows: mean {m:+.0f} s, sd {sd:.0f} s, rms CV {mcv * 100:.1f}% '
          f'(windows are {sum(seg_p) / len(seg_p):.1f} min on average)')
    if '--json' in sys.argv:
        print(json.dumps({'table': out, 'door': len(door), 'total': total, 'window_dev_sd_s': sd,
                          'window_rms_cv': mcv}))


if __name__ == '__main__':
    main()
