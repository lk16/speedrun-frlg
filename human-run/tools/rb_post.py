#!/usr/bin/env python3
"""Posterior of the runner's pre-door survival Q (past the starter gate to the door) and of door-to-finish D.
Windows from fit_attempts.py (63 attempts); the late windows (Misty onwards) are partially pooled (k = 6) as in
fit_hazards.py, the two early windows take a Jeffreys prior. stdlib only. Importable: sample_Q(rng), sample_D(rng, flat)."""
import math, os, random, sys

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
import fit_attempts as fa  # noqa: E402

K = 6.0


def windows():
    runs = fa.load()['runs']
    out, _, _ = fa.table(runs)
    return out


def _betas():
    w = windows()
    late = w[2:]
    pool = sum(o['deaths'] for o in late) / sum(o['at_risk'] for o in late)
    res = []
    for i, o in enumerate(w):
        s, d = o['at_risk'] - o['deaths'], o['deaths']
        if i < 2:
            res.append((s + 0.5, d + 0.5))
        else:
            res.append((s + K * (1 - pool), d + K * pool))
    return res


BETAS = _betas()


def sample_Q(rng):
    q = 1.0
    for a, b in BETAS:
        q *= rng.betavariate(a, b)
    return q


def sample_D(rng, flat=False):
    """Door-to-finish survival: the journal 2026-12-02 posterior (mean .23, 90% [.08,.47], prior centred on the model,
    k = 3) as Beta(2.2, 7.4); `flat`: data only, Beta(2.5, 3.5) (2 finishes of 5 fighters)."""
    return rng.betavariate(2.5, 3.5) if flat else rng.betavariate(2.2, 7.4)


if __name__ == '__main__':
    rng = random.Random(1)
    qs = sorted(sample_Q(rng) for _ in range(40000))
    ds = sorted(sample_D(rng) for _ in range(40000))
    df = sorted(sample_D(rng, True) for _ in range(40000))
    pick = lambda v, p: v[int(p * (len(v) - 1))]
    for name, v in (('Q', qs), ('D', ds), ('D flat', df)):
        print(name, ' '.join(f'{p:.3f}' for p in (pick(v, .025), pick(v, .16), pick(v, .5), pick(v, .84), pick(v, .975))))
