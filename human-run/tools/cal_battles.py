#!/usr/bin/env python3
"""Per-battle frames of a played controller run, and a least-squares fit of the trainer-battle cost.

Reads the decomposition JSON of `cal_decompose.py --json` (held episodes with frames) and the run's
`events.jsonl` (trainer, party size, turns, outcome per battle).

    python3 human-run/tools/cal_battles.py <decomp.json> <events.jsonl>

An episode is the whole stretch without field control around one battle: a trainer's sighting walk and
text, the swirl, the fight, the defeat text and prize, the return to the field. The fit is
frames = a + t * turns + m * enemy mons, the same form as the model's `calib_wr` trainer cost
(`sim.rs`: intro x 0.8 + 345 per action + 630 per mon + 240 outro, level-ups 240 each).
"""
import json
import re
import sys


def solve3(A, b):
    # Gaussian elimination on a 3x3 system.
    M = [row[:] + [bb] for row, bb in zip(A, b)]
    for c in range(3):
        p = max(range(c, 3), key=lambda r: abs(M[r][c]))
        M[c], M[p] = M[p], M[c]
        for r in range(3):
            if r != c:
                f = M[r][c] / M[c][c]
                M[r] = [x - f * y for x, y in zip(M[r], M[c])]
    return [M[i][3] / M[i][i] for i in range(3)]


def main():
    dec = json.load(open(sys.argv[1]))
    ev = [json.loads(l) for l in open(sys.argv[2])]
    rows = []
    for stage, d in dec.items():
        for kind, s, e in d["episodes"]:
            if kind not in ("wild", "trainer"):
                continue
            inside = [x for x in ev if s - 1 <= x["frame"] <= e + 1]
            name, mons, turns, outcome, levels = "?", 1, 0, "?", 0
            for x in inside:
                t = x["text"]
                m = re.match(r"(\S+) sends out .* \((\d+) mons\)", t)
                if m:
                    name, mons = m.group(1), int(m.group(2))
                m = re.match(r"wild (\S+ Lv\d+): (\w+)", t)
                if m and x["kind"] == "encounter":
                    name, outcome = "wild " + m.group(1), m.group(2)
                m = re.match(r"battle over: (\S+) after (\d+) turns", t)
                if m:
                    outcome = outcome if kind == "wild" and outcome != "?" else m.group(1)
                    if kind == "wild":
                        outcome = m.group(1) if outcome == "?" else outcome + "/" + m.group(1)
                    turns = int(m.group(2))
                if x["kind"] == "catch":
                    outcome = "catch"
            rows.append((stage, kind, name, mons, turns, outcome, e - s + 1, s))
    print(f"{'stage':14s} {'kind':7s} {'who':28s} mons turns {'outcome':16s} frames    s")
    for r in rows:
        print(f"{r[0]:14s} {r[1]:7s} {r[2]:28s} {r[3]:4d} {r[4]:5d} {r[5]:16s} {r[6]:6d} {r[6] / 59.7275:5.1f}")
    tr = [r for r in rows if r[1] == "trainer"]
    if len(tr) >= 4 and all(r[4] > 0 for r in tr):
        A = [[0.0] * 3 for _ in range(3)]
        b = [0.0] * 3
        for r in tr:
            x = [1.0, r[4], r[3]]
            for i in range(3):
                b[i] += x[i] * r[6]
                for j in range(3):
                    A[i][j] += x[i] * x[j]
        a, t, m = solve3(A, b)
        res = [r[6] - (a + t * r[4] + m * r[3]) for r in tr]
        rms = (sum(x * x for x in res) / len(res)) ** 0.5
        print(f"\ntrainer fit over {len(tr)} battles: frames = {a:.0f} + {t:.0f} x turns + {m:.0f} x mons (rms {rms:.0f})")
        print("model calib_wr: ~0.8 x intro(600-700) + 240 outro = 720-800 + 345 x turns + 630 x mons (+240 per level-up)")
        for mt, mm in ((345, 630),):
            tot_p = sum(r[6] for r in tr)
            tot_m = sum(0.8 * 650 + 240 + mt * r[4] + mm * r[3] for r in tr)
            print(f"sum played {tot_p} f ({tot_p / 59.7275:.0f} s) vs calib_wr formula {tot_m:.0f} f ({tot_m / 59.7275:.0f} s), ratio {tot_p / tot_m:.2f}")


if __name__ == "__main__":
    main()
