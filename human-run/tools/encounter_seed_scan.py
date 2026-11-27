#!/usr/bin/env python3
"""Wild encounters New Game -> Bug Catcher Sammy per title-screen A frame (journal 2026-11-27 encounter-seed).

The per-step encounter logic (decompiled/src/wild_encounter.c):
  TryStandardWildEncounter (:757-776) on every completed step (field_control_avatar.c:244,740-743):
  * HandleWildEncounterCooldown (:707-755): not an encounter tile -> no check. Else minSteps =
    8 - rate/10 (:673-687; 6 on Route 1/2 at rate 21, 7 in Viridian Forest at rate 14). If
    stepsSinceLastEncounter >= minSteps go on; else steps++ and go on only if Random() % 100 < 5
    (:750-754) -- the GLOBAL gRngValue.
  * StandardWildEncounter (:355-402): if the previous step's metatile behaviour differs from this
    one, DoGlobalWildEncounterDiceRoll (:348-353): Random() % 100 < 60 or no check -- GLOBAL again.
  * DoWildEncounterRateTest (:309-332): rate*16 + buff*16/200 vs WildEncounterRandom() % 1600 --
    the PRIVATE LCG sWildEncounterData.rngState (:667-671, ISO_RANDOMIZE2 = x*1103515245+12345,
    include/random.h:20), advanced by nothing else. Miss: buff += rate (:778-784).
  * hit -> wild battle; buff and steps reset (:765-768). Map loads (overworld.c:764,799) and every
    battle start (battle_setup.c:205) also reset buff and steps (field_control_avatar.c:735-738),
    never rngState. prevMetatileBehavior is only written by TryStandardWildEncounter.
  The private LCG is seeded once, at the title exit (new_game.c:103 via title_screen.c:737).

Subcommands
  validate <trace.tsv>...        replay each traced run with its observed gate outcomes; checks the model
                                 (every forced test happened, no test where impossible, hits = battles)
                                 and counts the global gates (bypass rate vs 5 %, boundary pass vs 60 %).
  path <trace.tsv> <out.json>    the step path up to Sammy (land/behaviour/map per step).
  scan <path.json> <title.tsv> <out.tsv> [procs]
                                 exact distribution of the encounter count to Sammy for every ready
                                 title frame, gates as independent Bernoulli(0.05)/(0.60) (the human's
                                 frame jitter scrambles gRngValue at every gate; checked in `validate`).
  seed <path.json> <wild_state>  the distribution for one wild seed, plus the deterministic best case.
"""
import json
import sys
from multiprocessing import Pool

M = 1103515245
RATE = {"Route1": 21, "Route2": 21, "ViridianForest": 14, "Route22": 21}
P_BYPASS = 0.05
P_GATE = 0.60


def min_steps(rate):
    if rate >= 80:
        return 0
    if rate < 10:
        return 8
    return 8 - rate // 10


def nxt(x):
    return (x * M + 12345) & 0xFFFFFFFF


def hit(x, rate, buff):
    """x: state after the advance. True if the rate test hits."""
    r = min(rate * 16 + buff * 16 // 200, 1600)
    return (x >> 16) % 1600 < r


def read_trace(path):
    rows = []
    with open(path) as f:
        head = f.readline().rstrip("\n").split("\t")
        for line in f:
            v = line.rstrip("\n").split("\t")
            d = dict(zip(head, v))
            for k in head:
                if k != "map":
                    d[k] = int(d[k])
            rows.append(d)
    return rows


def checks_of(rows):
    """The encounter checks of a traced run, inferred from the wild data (every check on an
    encounter tile changes it: steps++ in the cooldown, rngState on a rate test, or
    prevMetatileBehavior on a failed boundary gate; a check elsewhere matters only through
    prevMetatileBehavior). The check of tile k runs on the frame the step to tile k+1 starts, or
    on arrival when the player stops; a turn in place (`PlayerTurnInPlace`, WALK_IN_PLACE_FAST)
    ends in T_TILE_CENTER too (`field_player_avatar.c:745-760,779-785`) and so checks the tile
    again (`field_control_avatar.c:141-142`). Returns events:
      {"kind": "check", map, x, y, land, beh, test, cool, turn}  |  {"kind": "reset"}  |
      {"kind": "battle", "flags"}"""
    ev = []
    last_check_pos = None
    moved_since = True
    for i in range(1, len(rows)):
        q, r = rows[i - 1], rows[i]
        pos_changed = (r["map"], r["x"], r["y"]) != (q["map"], q["x"], q["y"])
        tile = q if pos_changed else r
        d_rng = r["wild_rng"] != q["wild_rng"]
        d_steps = r["steps"] == q["steps"] + 1
        d_prev = r["prev_beh"] != q["prev_beh"]
        reset = (not d_rng and r["steps"] == 0 and r["buff"] == 0 and (q["steps"] != 0 or q["buff"] != 0))
        if r["battle"] == 1 and q["battle"] == 0:
            ev.append({"kind": "battle", "flags": r["bflags"], "frame": r["frame"]})
        if reset:
            ev.append({"kind": "reset", "frame": r["frame"]})
        elif d_rng or d_steps or d_prev:
            pos = (tile["map"], tile["x"], tile["y"])
            turn = pos == last_check_pos and not moved_since
            ev.append({"kind": "check", "frame": r["frame"], "map": tile["map"], "x": tile["x"], "y": tile["y"],
                       "land": tile["land"], "beh": tile["behavior"], "test": d_rng, "cool": d_steps,
                       "rng_out": r["wild_rng"], "turn": turn})
            last_check_pos = pos
            moved_since = False
        if pos_changed:
            moved_since = True
            if r["map"] != q["map"]:
                # LoadMapFromCameraTransition / LoadMapFromWarp (overworld.c:764,799)
                ev.append({"kind": "reset", "frame": r["frame"], "map": True})
    return ev


def validate(paths):
    tot = {"cool": 0, "bypass": 0, "boundary": 0, "gate_pass": 0, "forced": 0, "bad": 0, "hits": 0,
           "turn_checks": 0, "turn_after_wild": 0}
    for p in paths:
        rows = read_trace(p)
        ev = checks_of(rows)
        steps = buff = 0
        prev = None
        rng = rows[0]["wild_rng"]
        hits = wild = 0
        bad = []
        last_battle_wild = False
        for e in ev:
            if e["kind"] == "battle":
                if e["flags"] == 0:
                    wild += 1
                last_battle_wild = e["flags"] == 0
                if e["flags"] & 8 and wild >= 0 and rows and e is not None:
                    pass
                continue
            if e["kind"] == "reset":
                steps = buff = 0
                continue
            if e["turn"]:
                tot["turn_checks"] += 1
                tot["turn_after_wild"] += last_battle_wild
            rate = RATE.get(e["map"])
            if not e["land"] or rate is None:
                if e["test"] or e["cool"]:
                    bad.append((e["frame"], "test/cooldown on a non-encounter tile"))
                prev = e["beh"]
                continue
            last_battle_wild = False
            ms = min_steps(rate)
            passed = True
            if steps < ms:
                steps += 1
                tot["cool"] += 1
                if not e["cool"]:
                    bad.append((e["frame"], f"cooldown step not seen (steps {steps})"))
                passed = e["test"]
                tot["bypass"] += passed
            elif e["cool"]:
                bad.append((e["frame"], "unexpected cooldown step"))
            if passed and prev is not None and prev != e["beh"]:
                tot["boundary"] += 1
                passed = e["test"]
                tot["gate_pass"] += passed
            elif passed:
                tot["forced"] += 1
                if not e["test"]:
                    bad.append((e["frame"], f"forced test missing at {e['map']} ({e['x']},{e['y']})"))
            prev = e["beh"]
            if passed and e["test"]:
                rng = nxt(rng)
                if rng != e["rng_out"]:
                    bad.append((e["frame"], f"rng {rng} != traced {e['rng_out']}"))
                    rng = e["rng_out"]
                if hit(rng, rate, buff):
                    hits += 1
                    buff = steps = 0
                else:
                    buff += rate
        tot["bad"] += len(bad)
        tot["hits"] += hits
        print(f"{p.split('/')[-1]}: {sum(e['kind'] == 'check' for e in ev)} checks, wild battles {wild}, "
              f"model hits {hits}, problems {len(bad)}")
        for b in bad[:10]:
            print("   ", b)
    c = tot
    print(json.dumps(c))
    print(f"bypass {c['bypass']}/{c['cool']} = {c['bypass'] / max(c['cool'], 1):.3f} (code: 0.05); "
          f"boundary gate pass {c['gate_pass']}/{c['boundary']} = {c['gate_pass'] / max(c['boundary'], 1):.3f} (code: 0.60)")


def until_sammy(rows):
    """Rows up to Sammy's battle start (a trainer battle in Viridian Forest with the player west of
    x 8: Sammy is object 5 at (7, 22), `ViridianForest/map.json`)."""
    for i, r in enumerate(rows):
        if r["battle"] == 1 and r["bflags"] & 8 and r["map"] == "ViridianForest" and r["x"] <= 8:
            return rows[:i + 1]
    return rows


def path(trace, out):
    """The check path New Game -> Sammy: encounter-tile checks (tile behaviour, map), prev-only
    checks, map/trainer resets. Turn checks right after a wild battle are dropped (they belong to
    that run's encounters, not to the path)."""
    rows = until_sammy(read_trace(trace))
    ev = checks_of(rows)
    p = []
    cur = None
    after_wild = False
    dropped = 0
    for e in ev:
        if e["kind"] == "battle":
            after_wild = e["flags"] == 0
            if e["flags"] != 0:
                p.append({"kind": "reset", "why": f"trainer battle {e['flags']}"})
            continue
        if e["kind"] == "reset":
            continue
        if e["map"] != cur:
            cur = e["map"]
            p.append({"kind": "reset", "why": f"map {cur}"})
        if e["turn"] and after_wild:
            dropped += 1
            continue
        if e["land"]:
            after_wild = False
        p.append({"kind": "check", "map": e["map"], "x": e["x"], "y": e["y"], "land": e["land"], "beh": e["beh"],
                  "turn": e["turn"]})
    json.dump(p, open(out, "w"), indent=0)
    land = [s for s in p if s["kind"] == "check" and s["land"]]
    by = {}
    for s in land:
        by[s["map"]] = by.get(s["map"], 0) + 1
    print(f"{len(land)} encounter-tile checks to Sammy {by}; turn checks kept "
          f"{sum(s['turn'] for s in land)}, dropped after wild battles {dropped}")


def compile_path(p):
    """('reset',) / ('tile', rate, beh) / ('prev', beh) events from a `path` JSON."""
    ev = []
    for s in p:
        if s["kind"] == "reset":
            ev.append(("reset",))
            continue
        rate = RATE.get(s["map"])
        if s["land"] and rate is not None:
            ev.append(("tile", rate, s["beh"]))
        else:
            ev.append(("prev", s["beh"]))
    return ev


def dist(ev, seed, kmax=12):
    """Exact distribution of the number of hits along `ev` from wild state `seed`, the global
    gates as independent coin flips. State: (rng, buff, steps, prev, hits) -> probability."""
    states = {(seed, 0, 0, None, 0): 1.0}
    for e in ev:
        if e[0] == "reset":
            ns = {}
            for (x, b, c, pv, h), pr in states.items():
                k = (x, 0, 0, pv, h)
                ns[k] = ns.get(k, 0) + pr
            states = ns
            continue
        if e[0] == "prev":
            ns = {}
            for (x, b, c, pv, h), pr in states.items():
                k = (x, b, c, e[1], h)
                ns[k] = ns.get(k, 0) + pr
            states = ns
            continue
        _, rate, beh = e
        ms = min_steps(rate)
        ns = {}

        def add(k, pr):
            ns[k] = ns.get(k, 0) + pr

        for (x, b, c, pv, h), pr in states.items():
            # cooldown
            branches = []
            if c >= ms:
                branches.append((c, pr))
            else:
                add((x, b, c + 1, beh, h), pr * (1 - P_BYPASS))
                branches.append((c + 1, pr * P_BYPASS))
            for c2, p2 in branches:
                if pv is not None and pv != beh:
                    add((x, b, c2, beh, h), p2 * (1 - P_GATE))
                    p2 *= P_GATE
                x2 = nxt(x)
                if hit(x2, rate, b):
                    add((x2, 0, 0, beh, min(h + 1, kmax)), p2)
                else:
                    add((x2, b + rate, c2, beh, h), p2)
        states = ns
    out = [0.0] * (kmax + 1)
    for (x, b, c, pv, h), pr in states.items():
        out[h] += pr
    return out


def best_worst(ev, seed):
    """Fewest / most hits over all gate outcomes (what perfect / worst global-RNG luck gives)."""
    states = {(seed, 0, 0, None): (0, 0)}
    for e in ev:
        ns = {}

        def add(k, lo, hi):
            if k in ns:
                a, bb = ns[k]
                ns[k] = (min(a, lo), max(bb, hi))
            else:
                ns[k] = (lo, hi)

        for (x, b, c, pv), (lo, hi) in states.items():
            if e[0] == "reset":
                add((x, 0, 0, pv), lo, hi)
            elif e[0] == "prev":
                add((x, b, c, e[1]), lo, hi)
            else:
                _, rate, beh = e
                ms = min_steps(rate)
                branches = []
                if c >= ms:
                    branches.append(c)
                else:
                    add((x, b, c + 1, beh), lo, hi)
                    branches.append(c + 1)
                for c2 in branches:
                    if pv is not None and pv != beh:
                        add((x, b, c2, beh), lo, hi)
                    x2 = nxt(x)
                    if hit(x2, rate, b):
                        add((x2, 0, 0, beh), lo + 1, hi + 1)
                    else:
                        add((x2, b + rate, c2, beh), lo, hi)
        states = ns
    return min(v[0] for v in states.values()), max(v[1] for v in states.values())


_EV = None


def _init(ev):
    global _EV
    _EV = ev


def _job(row):
    d, seed = row
    dd = dist(_EV, seed)
    mean = sum(i * p for i, p in enumerate(dd))
    return d, seed, dd, mean


def scan(path_json, title_tsv, out, procs=12):
    ev = compile_path(json.load(open(path_json)))
    rows = []
    for line in open(title_tsv):
        v = line.split()
        if v[1] == "1":
            rows.append((int(v[0]), int(v[3])))
    seeds = sorted({s for _, s in rows})
    with Pool(procs, initializer=_init, initargs=(ev,)) as pool:
        res = pool.map(_job, [(0, s) for s in seeds], chunksize=8)
    by = {s: (dd, mean) for _, s, dd, mean in res}
    with open(out, "w") as f:
        f.write("d\twild_state\tmean\tp0\tp_le1\tp_le2\tp_le3\n")
        for d, s in rows:
            dd, mean = by[s]
            f.write(f"{d}\t{s}\t{mean:.4f}\t{dd[0]:.5f}\t{dd[0] + dd[1]:.5f}\t{sum(dd[:3]):.5f}\t{sum(dd[:4]):.5f}\n")


def allseeds(path_json, out, procs=12):
    """The distribution for every u16 wild seed (what an unrestricted title frame could reach)."""
    ev = compile_path(json.load(open(path_json)))
    with Pool(procs, initializer=_init, initargs=(ev,)) as pool:
        res = pool.map(_job, [(0, s) for s in range(65536)], chunksize=64)
    with open(out, "w") as f:
        f.write("wild_state\tmean\tp0\tp_le1\tp_le2\tp_le3\n")
        for _, s, dd, mean in res:
            f.write(f"{s}\t{mean:.4f}\t{dd[0]:.5f}\t{dd[0] + dd[1]:.5f}\t{sum(dd[:3]):.5f}\t{sum(dd[:4]):.5f}\n")


def seed(path_json, s):
    ev = compile_path(json.load(open(path_json)))
    dd = dist(ev, s)
    print("P(k encounters):", [round(p, 5) for p in dd])
    print("min/max over gate outcomes:", best_worst(ev, s))


if __name__ == "__main__":
    a = sys.argv[1:]
    if a[0] == "validate":
        validate(a[1:])
    elif a[0] == "path":
        path(a[1], a[2])
    elif a[0] == "scan":
        scan(a[1], a[2], a[3], int(a[4]) if len(a) > 4 else 12)
    elif a[0] == "allseeds":
        allseeds(a[1], a[2], int(a[3]) if len(a) > 3 else 12)
    elif a[0] == "seed":
        seed(a[1], int(a[2]))
