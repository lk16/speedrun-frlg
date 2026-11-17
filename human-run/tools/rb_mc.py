#!/usr/bin/env python3
"""Evidence-weighted EHT interval for a plan (ROBUSTNESS.md section 14). stdlib only.

usage:
  rb_mc.py design N OUT.json CAL.json [--plan P] [--R 800000] [--seed0 100] [--rungs WR+2m,WR] [--prior pre|post] [--flatD]
      Latin-hypercube N draws of the stand-ins and calibrations, mapped to plan patches, EHT at each rung, appended to OUT.
  rb_mc.py fit OUT.json         regress ln EHT on the draws, report R2 / leave-one-out and standardised effects
  rb_mc.py interval OUT.json [--n 200000] [--rung WR+2m] [--prior pre|post] [--flatD]
      propagate the priors through the fitted surface: median, 68%, 95%.

Mapping of the evidence-pinned quantities (CAL.json from rb_cal.py / rb-cal2): the runner's pre-door survival Q
(posterior rb_post.sample_Q) fixes the risk-lane scale `lane` through the model's Q(cp, lane) surface, and the
door-to-finish survival D (posterior rb_post.sample_D) fixes the flat-row scale `flat` through the engine-only survival
S_eng(league_power, mewtwo_power) (a model's survival is log-additive: ln D = ln S_eng + flat * ln S_flat).
"""
import json, math, os, random, sys

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
import rb_lib as L  # noqa: E402
import rb_post as P  # noqa: E402

# name: (kind, a, b, lo, hi)   kind: n = normal(a, b) clipped to [lo, hi]; ln = lognormal(median a, sigma b) clipped; u = uniform(lo, hi)
PRIORS = {
    'pre': {
        'lp': ('n', 1.35, 0.15, 1.0, 1.8),
        'mp': ('n', 1.2, 0.10, 1.0, 1.6),
        'cp': ('n', 2.0, 0.20, 1.7, 2.4),
        'tf': ('n', 0.729, 0.066, 0.60, 0.86),
        'mr': ('ln', 0.162, 0.427, 0.05, 0.5),
        'win_cv': ('u', None, None, 0.0, 0.069),
        'slow': ('ln', 1.0, 0.35, 0.3, 3.0),
        'phit': ('n', 0.64, 0.02, 0.58, 0.70),
        'ppup_f': ('ln', 450.0, 0.35, 150.0, 1200.0),
        'hid': ('ln', 1.0, 0.30, 0.4, 2.5),
        'museum': ('ln', 1.0, 0.25, 0.5, 2.0),
    },
}
# after the video measurement (journal 2026-12-03 section 2): the PP Up pickup on the route is ~2.5 s of text
PRIORS['post'] = dict(PRIORS['pre'])
PRIORS['post']['ppup_f'] = ('ln', 300.0, 0.35, 120.0, 900.0)
PRIORS['post']['museum'] = ('ln', 1.0, 0.18, 0.6, 1.6)

CONT = ['lp', 'mp', 'cp', 'tf', 'mr', 'win_cv', 'slow', 'phit', 'ppup_f', 'hid', 'museum']
TRANS = {k: (math.log if PRIORS['pre'][k][0] == 'ln' else (lambda x: x)) for k in CONT}
ALLV = ['lnQ', 'lnD'] + CONT


def norm_ppf(p):
    # Acklam-free: Beasley-Springer-Moro would do; bisection on erf is plenty here
    lo, hi = -8.0, 8.0
    for _ in range(60):
        mid = (lo + hi) / 2
        if 0.5 * (1 + math.erf(mid / math.sqrt(2))) < p:
            lo = mid
        else:
            hi = mid
    return (lo + hi) / 2


def draw(spec, u):
    kind, a, b, lo, hi = spec
    if kind == 'u':
        return lo + (hi - lo) * u
    z = norm_ppf(min(max(u, 1e-6), 1 - 1e-6))
    x = a + b * z if kind == 'n' else a * math.exp(b * z)
    return min(max(x, lo), hi)


class Cal:
    def __init__(self, path, base):
        c = json.load(open(path))
        self.Q = c['Q']
        self.D = c['D']
        self.cps = sorted({r['cp'] for r in self.Q})
        self.lams = sorted({r['lam'] for r in self.Q})
        self.tab = {(r['cp'], r['lam']): math.log(r['Q']) for r in self.Q}
        self.lps = sorted({r['lp'] for r in self.D})
        self.mps = sorted({r['mp'] for r in self.D})
        self.dtab = {(r['lp'], r['mp']): math.log(r['D']) for r in self.D}
        self.lnSflat = L.flat_logsurv(base)

    @staticmethod
    def _interp(xs, x, f):
        if x <= xs[0]:
            i = 0
        elif x >= xs[-1]:
            i = len(xs) - 2
        else:
            i = max(j for j in range(len(xs) - 1) if xs[j] <= x)
        t = (x - xs[i]) / (xs[i + 1] - xs[i])
        return f(xs[i]) * (1 - t) + f(xs[i + 1]) * t

    def lnQ(self, cp, lam):
        ll = math.log(lam)
        return self._interp(self.cps, cp, lambda c: self._interp([math.log(v) for v in self.lams], ll, lambda lv: self.tab[(c, math.exp(lv))]))

    def lam_for(self, cp, Q):
        lo, hi = 0.6, 1.8
        t = math.log(Q)
        for _ in range(50):
            mid = math.sqrt(lo * hi)
            if self.lnQ(cp, mid) < t:
                lo = mid
            else:
                hi = mid
        return math.sqrt(lo * hi)

    def lnSeng(self, lp, mp):
        return self._interp(self.lps, lp, lambda a: self._interp(self.mps, mp, lambda b: self.dtab[(a, b)]))

    def flat_for(self, lp, mp, D):
        se = self.lnSeng(lp, mp)
        t = math.log(D)
        if t >= se:
            return 0.0
        return min(3.0, (t - se) / self.lnSflat)


def lhs(n, d, rng):
    cols = []
    for _ in range(d):
        v = [(i + rng.random()) / n for i in range(n)]
        rng.shuffle(v)
        cols.append(v)
    return [[cols[j][i] for j in range(d)] for i in range(n)]


def sample_vec(prior, u, flatD=False):
    """u: list of uniforms (2 + len(CONT)) -> dict of the draws (Q, D and the continuous constants)."""
    rng_q = random.Random(int(u[0] * 2 ** 40))
    rng_d = random.Random(int(u[1] * 2 ** 40) + 7)
    v = {'Q': P.sample_Q(rng_q), 'D': P.sample_D(rng_d, flatD)}
    for k, uk in zip(CONT, u[2:]):
        v[k] = draw(PRIORS[prior][k], uk)
    return v


def plan_patch(base, cal, v):
    lam = cal.lam_for(v['cp'], v['Q'])
    flat = cal.flat_for(v['lp'], v['mp'], v['D'])
    q = L.patch(base, lane=lam, flat=flat, lp=v['lp'], mp=v['mp'], cp=v['cp'], tf=v['tf'], mr=v['mr'], win_cv=v['win_cv'],
                slow=v['slow'], phit=v['phit'], ppup_f=v['ppup_f'], hid=v['hid'], museum=v['museum'])
    return q, lam, flat


def opt(a, name, d):
    if name in a:
        i = a.index(name)
        v = type(d)(a[i + 1])
        del a[i:i + 2]
        return v
    return d


def cmd_design(a):
    n, out, calp = int(a[0]), a[1], a[2]
    a = a[3:]
    plan = opt(a, '--plan', '1057-calm-sp9-ppup-redo')
    R = opt(a, '--R', 800000)
    seed0 = opt(a, '--seed0', 100)
    rungs = opt(a, '--rungs', 'WR+2m,WR').split(',')
    prior = opt(a, '--prior', 'pre')
    flatD = '--flatD' in a
    base = L.load_plan(plan)
    cal = Cal(calp, base)
    rng = random.Random(seed0)
    U = lhs(n, 2 + len(CONT), rng)
    rows = json.load(open(out)) if os.path.exists(out) else []
    done = {r['i'] for r in rows}
    for i, u in enumerate(U):
        if i in done:
            continue
        v = sample_vec(prior, u, flatD)
        q, lam, flat = plan_patch(base, cal, v)
        row = {'i': i, 'v': v, 'lam': lam, 'flat': flat, 'prior': prior, 'eht': {}, 'fin': {}}
        for rg in rungs:
            e, f = L.run_eval(q, f'mc{i}', rg, R, seed0 + i)
            row['eht'][rg], row['fin'][rg] = e, f
        rows.append(row)
        json.dump(rows, open(out, 'w'))
        print(i, {k: round(x, 3) for k, x in v.items()}, round(lam, 3), round(flat, 2), row['eht'], flush=True)


def features(v):
    x = []
    for k in ALLV:
        if k == 'lnQ':
            x.append(math.log(v['Q']))
        elif k == 'lnD':
            x.append(math.log(v['D']))
        else:
            x.append(TRANS[k](v[k]))
    return x


def design_matrix(X, mu, sd, cross):
    rows = []
    for x in X:
        z = [(a - m) / s for a, m, s in zip(x, mu, sd)]
        r = [1.0] + z + [a * a for a in z]
        for (i, j) in cross:
            r.append(z[i] * z[j])
        rows.append(r)
    return rows


def ridge(A, y, lam=1e-3):
    n = len(A[0])
    M = [[sum(r[i] * r[j] for r in A) + (lam if i == j and i > 0 else 0.0) for j in range(n)] for i in range(n)]
    b = [sum(r[i] * yy for r, yy in zip(A, y)) for i in range(n)]
    for i in range(n):  # gauss
        p = max(range(i, n), key=lambda k: abs(M[k][i]))
        M[i], M[p] = M[p], M[i]
        b[i], b[p] = b[p], b[i]
        for k in range(i + 1, n):
            f = M[k][i] / M[i][i]
            for j in range(i, n):
                M[k][j] -= f * M[i][j]
            b[k] -= f * b[i]
    w = [0.0] * n
    for i in range(n - 1, -1, -1):
        w[i] = (b[i] - sum(M[i][j] * w[j] for j in range(i + 1, n))) / M[i][i]
    return w


def load_fit(path, rung, lam=1.0, cross_top=0):
    rows = [r for r in json.load(open(path)) if r['eht'].get(rung) and r['fin'][rung] >= 20]
    X = [features(r['v']) for r in rows]
    y = [math.log(r['eht'][rung]) for r in rows]
    mu = [sum(c) / len(c) for c in zip(*X)]
    sd = [math.sqrt(sum((a - m) ** 2 for a in c) / len(c)) or 1.0 for c, m in zip(zip(*X), mu)]
    cross = []
    if cross_top:
        w0 = ridge(design_matrix(X, mu, sd, []), y, lam)
        k = len(ALLV)
        eff = sorted(range(k), key=lambda i: -abs(w0[1 + i]))[:cross_top]
        cross = [(i, j) for a, i in enumerate(eff) for j in eff[a + 1:]]
    A = design_matrix(X, mu, sd, cross)
    w = ridge(A, y, lam)
    return rows, X, y, mu, sd, cross, w, A


def cmd_fit(a):
    path = a[0]
    cross_top = opt(a, '--cross', 4)
    lam = opt(a, '--ridge', 1.0)
    for rung in ('WR+2m', 'WR'):
        rows, X, y, mu, sd, cross, w, A = load_fit(path, rung, lam, cross_top)
        if len(rows) < 30:
            continue
        pred = [sum(wi * ai for wi, ai in zip(w, r)) for r in A]
        ss = sum((p - q) ** 2 for p, q in zip(pred, y))
        my = sum(y) / len(y)
        r2 = 1 - ss / sum((q - my) ** 2 for q in y)
        # k-fold CV
        n = len(y)
        err = 0.0
        for f in range(5):
            tr = [i for i in range(n) if i % 5 != f]
            te = [i for i in range(n) if i % 5 == f]
            wf = ridge([A[i] for i in tr], [y[i] for i in tr], lam)
            err += sum((sum(wi * ai for wi, ai in zip(wf, A[i])) - y[i]) ** 2 for i in te)
        cv = 1 - err / sum((q - my) ** 2 for q in y)
        print(f'{rung}: n={n} R2={r2:.3f} CV-R2={cv:.3f} resid sd(ln)={math.sqrt(ss / n):.3f}  sd(ln EHT)={math.sqrt(sum((q - my) ** 2 for q in y) / n):.3f}')
        k = len(ALLV)
        print('  standardised effect on ln EHT (per +1 sd of the draw), linear / quadratic:')
        for i, nme in sorted(enumerate(ALLV), key=lambda t: -abs(w[1 + t[0]])):
            print(f'    {nme:8s} {w[1 + i]:+.3f}  {w[1 + k + i]:+.3f}')


def cmd_interval(a):
    path = a[0]
    n = opt(a, '--n', 200000)
    rung = opt(a, '--rung', 'WR+2m')
    prior = opt(a, '--prior', 'pre')
    flatD = '--flatD' in a
    cross_top = opt(a, '--cross', 4)
    lam = opt(a, '--ridge', 1.0)
    rows, X, y, mu, sd, cross, w, A = load_fit(path, rung, lam, cross_top)
    # residual sd (sim noise + lack of fit), added to each draw
    pred = [sum(wi * ai for wi, ai in zip(w, r)) for r in A]
    res = math.sqrt(sum((p - q) ** 2 for p, q in zip(pred, y)) / len(y))
    rng = random.Random(7)
    base = L.load_plan(opt(a, '--plan', '1057-calm-sp9-ppup-redo'))
    out = []
    for _ in range(n):
        u = [rng.random() for _ in range(2 + len(CONT))]
        v = sample_vec(prior, u, flatD)
        # sample_vec seeds the Q / D draws from u, so fresh uniforms are fresh posterior draws
        z = [(f - m) / s for f, m, s in zip(features(v), mu, sd)]
        r = [1.0] + z + [t * t for t in z] + [z[i] * z[j] for (i, j) in cross]
        out.append(math.exp(sum(wi * ri for wi, ri in zip(w, r)) + 0.0))
    out.sort()
    pick = lambda p: out[int(p * (len(out) - 1))]
    print(f'{rung} prior={prior}{" flatD" if flatD else ""}: median {pick(.5):.0f} h, 68% [{pick(.16):.0f}, {pick(.84):.0f}], 95% [{pick(.025):.0f}, {pick(.975):.0f}] '
          f'(surface only; residual sd {res:.3f} in ln; fit rows {len(rows)})')


if __name__ == '__main__':
    c = sys.argv[1]
    {'design': cmd_design, 'fit': cmd_fit, 'interval': cmd_interval}[c](sys.argv[2:])
