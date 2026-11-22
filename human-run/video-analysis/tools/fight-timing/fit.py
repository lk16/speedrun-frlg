"""Fit WR vs bot fight times per component, with bootstrap CIs."""
import json, random, statistics as S
SP = __import__('os').environ.get('WFT_DIR', '/tmp/wft') + '/'
rows = json.load(open(SP + 'pairs.json'))
F = 59.7275
TC = (838, 579, 223)


def ols(X, y):
    k = len(X[0])
    A = [[sum(x[i] * x[j] for x in X) for j in range(k)] for i in range(k)]
    b = [sum(x[i] * yy for x, yy in zip(X, y)) for i in range(k)]
    M = [A[i][:] + [b[i]] for i in range(k)]
    for c in range(k):
        p = max(range(c, k), key=lambda r: abs(M[r][c]))
        M[c], M[p] = M[p], M[c]
        for r in range(k):
            if r != c:
                f = M[r][c] / M[c][c]
                M[r] = [u - f * v for u, v in zip(M[r], M[c])]
    return [M[i][k] / M[i][i] for i in range(k)]


def boot(rows, fn, n=4000, seed=1):
    rnd = random.Random(seed)
    vals = []
    for _ in range(n):
        s = [rnd.choice(rows) for _ in rows]
        try:
            vals.append(fn(s))
        except ZeroDivisionError:
            pass
    vals.sort()
    return vals[int(0.025 * len(vals))], vals[int(0.975 * len(vals))]


# exclusions: chains the detector merged with a non-fight screen (checked on sheets below)
EXCL = set(json.load(open(SP + 'excl.json'))) if __import__('os').path.exists(SP + 'excl.json') else set()
P = [r for r in rows if 'bot_dur' in r and r['name'] not in EXCL]
W = [r for r in rows if r['name'] not in EXCL]
print(f"paired fights: {len(P)}; WR fights: {len(W)}; excluded {sorted(EXCL)}")

# 1. totals on the paired set
wd = sum(r['wr_dur'] for r in P); bd = sum(r['bot_dur'] for r in P)
wt = sum(r['wr_menus'] for r in P); bt = sum(r['bot_turns'] for r in P)
print(f"paired battle time WR {wd:.0f} s vs bot {bd:.0f} s: ratio {wd / bd:.3f} {boot(P, lambda s: sum(r['wr_dur'] for r in s) / sum(r['bot_dur'] for r in s))}")
print(f"decisions WR {wt} vs bot {bt}: ratio {wt / bt:.3f} {boot(P, lambda s: sum(r['wr_menus'] for r in s) / sum(r['bot_turns'] for r in s))}")
# 2. same decision count: pure speed
Q = [r for r in P if r['wr_menus'] == r['bot_turns']]
print(f"same decisions (n {len(Q)}): WR {sum(r['wr_dur'] for r in Q):.0f} vs bot {sum(r['bot_dur'] for r in Q):.0f} s, ratio "
      f"{sum(r['wr_dur'] for r in Q) / sum(r['bot_dur'] for r in Q):.3f} {boot(Q, lambda s: sum(r['wr_dur'] for r in s) / sum(r['bot_dur'] for r in s))}")
print("  per fight diffs (WR - bot, s):", sorted(round(r['wr_dur'] - r['bot_dur'], 1) for r in Q))
print(f"  menu seconds per decision: WR {sum(r['wr_menu_s'] for r in Q) / sum(r['wr_menus'] for r in Q):.2f}, bot "
      f"{sum(r['botv_menu_s'] for r in Q if 'botv_menu_s' in r) / max(1, sum(r['bot_turns'] for r in Q if 'botv_menu_s' in r)):.2f}")
# 3. per-component fits: dur = a + t * decisions + m * mons
for lab, set_, d, t in (('WR (all)', W, 'wr_dur', 'wr_menus'), ('WR (paired)', P, 'wr_dur', 'wr_menus'), ('bot (paired)', P, 'bot_dur', 'bot_turns')):
    X = [[1, r[t], r['mons']] for r in set_]
    y = [r[d] for r in set_]
    c = ols(X, y)
    res = [yy - sum(a * b for a, b in zip(c, x)) for x, yy in zip(X, y)]
    ci = [boot(set_, lambda s, i=i: ols([[1, r[t], r['mons']] for r in s], [r[d] for r in s])[i], n=2000) for i in range(3)]
    print(f"{lab:13s} n {len(set_)}: a {c[0]:5.1f} s [{ci[0][0]:.1f}, {ci[0][1]:.1f}], per decision {c[1]:5.2f} [{ci[1][0]:.2f}, {ci[1][1]:.2f}], "
          f"per mon {c[2]:5.2f} [{ci[2][0]:.2f}, {ci[2][1]:.2f}], rms {(sum(e * e for e in res) / len(res)) ** .5:.1f}")
# 4. against trainer_cost: episode = visible battle + the bot's own overhead for that fight (speech, swirl, walk,
#    return; equal speech times on both sides), at each side's own decisions
OV = S.median(r['bot_ep'] - r['bot_dur'] for r in P if r.get('bot_ep'))
print(f"bot overhead outside the visible battle: median {OV:.1f} s")


def tc(dec, mons):
    return (TC[0] + TC[1] * dec + TC[2] * mons) / F


for lab, set_ in (('paired', P), ('all WR', W)):
    we = sum(r['wr_dur'] + ((r['bot_ep'] - r['bot_dur']) if r.get('bot_ep') else OV) for r in set_)
    wm = sum(tc(r['wr_menus'], r['mons']) for r in set_)
    print(f"{lab}: WR episode {we:.0f} s vs trainer_cost at the WR's decisions {wm:.0f} s: {we / wm:.3f} "
          f"{boot(set_, lambda s: sum(r['wr_dur'] + ((r['bot_ep'] - r['bot_dur']) if r.get('bot_ep') else OV) for r in s) / sum(tc(r['wr_menus'], r['mons']) for r in s))}")
be = sum(r['bot_ep'] for r in P if r.get('bot_ep'))
bm = sum(tc(r['bot_turns'], r['mons']) for r in P if r.get('bot_ep'))
print(f"paired: bot episode {be:.0f} s vs trainer_cost at the bot's turns {bm:.0f} s: {be / bm:.3f}")
wtc = sum(tc(r['wr_menus'], r['mons']) for r in P)
btc = sum(tc(r['bot_turns'], r['mons']) for r in P)
print(f"paired: trainer_cost at WR decisions / at bot turns = {wtc / btc:.3f} {boot(P, lambda s: sum(tc(r['wr_menus'], r['mons']) for r in s) / sum(tc(r['bot_turns'], r['mons']) for r in s))}")
