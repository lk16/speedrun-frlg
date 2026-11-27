#!/usr/bin/env python3
"""C1: classify every trainer-fight turn of a played run (surviving branch only) by cause.

    python3 human-run/tools/c1_turns.py <run dir> [<run dir> ...] [--fights]

Reads `events.jsonl` (the `battle` events: `nums` has action / policy / my_status / dealt_last ...) and the
decomp for the rules: type chart `decompiled/src/battle_main.c` `gTypeEffectiveness`, move type / power /
accuracy `decompiled/src/data/battle_moves.h`, species types `decompiled/src/data/pokemon/species_info.h`,
sleep counting `decompiled/src/battle_util.c` `AtkCanceller_UnableToUseMove` (Sleep(n): n-1 lost turns).

Causes of a turn (first match wins):
  sleep     the fighter is asleep and loses the turn (status Sleep(n) with n > 1) and did not use an item
  heal      the action is an item (potion, Full Restore, Awakening, cure)
  unable    the next turn's `my_last_move` is 0xFFFF: the move did not go off (full paralysis, flinch)
  fallback  the policy chose Heal / FullRestore but an attack went instead
  badmove   an attack scoring < 90 % of the best move known so far (power x effectiveness x STAB x accuracy)
  miss      an attack that dealt no damage to a foe that stayed (accuracy / immunity)
  attack    the rest (the attacks needed to KO the team)
  setup     a status move (Withdraw, Defense Curl ...)
A "known" move is one the run has used up to that point.
"""
import json
import os
import re
import sys

DEC = os.environ.get("FRLG_DECOMP", "/home/luuk/projects/speedrun-frlg/decompiled")
FPS = 59.7275
PER_TURN = 503  # frames per action (the fitted trainer price, journal r3e)


def types_chart():
    src = open(DEC + "/src/battle_main.c").read()
    body = src.split("gTypeEffectiveness[336]")[1].split("};")[0]
    ch = {}
    for a, d, m in re.findall(r"TYPE_(\w+),\s*TYPE_(\w+),\s*TYPE_MUL_(\w+)", body):
        if a in ("FORESIGHT", "ENDTABLE"):
            continue
        ch[(a, d)] = {"NO_EFFECT": 0.0, "NOT_EFFECTIVE": 0.5, "SUPER_EFFECTIVE": 2.0}[m]
    return ch


def moves():
    src = open(DEC + "/src/data/battle_moves.h").read()
    out = {}
    for name, body in re.findall(r"\[MOVE_(\w+)\]\s*=\s*\{(.*?)\n    \}", src, re.S):
        def g(k):
            return re.search(r"\." + k + r"\s*=\s*(\w+)", body).group(1)

        out[name] = dict(power=int(g("power")), type=g("type")[5:], acc=int(g("accuracy")), pp=int(g("pp")))
    return out


def species():
    src = open(DEC + "/src/data/pokemon/species_info.h").read()
    out = {}
    for name, body in re.findall(r"\[SPECIES_(\w+)\]\s*=\s*\{(.*?)\n    \}", src, re.S):
        m = re.search(r"\.types\s*=\s*\{TYPE_(\w+),\s*TYPE_(\w+)\}", body)
        if m:
            out[name] = (m.group(1), m.group(2))
    return out


CH, MV, SP = types_chart(), moves(), species()
ITEMS = ("potion", "super potion", "hyper potion", "full restore", "awakening", "antidote", "full heal", "max potion")


def eff(mt, foe):
    t = SP.get(foe)
    if not t:
        return 1.0
    e = 1.0
    for d in set(t):
        e *= CH.get((mt, d), 1.0)
    return e


def score(mv, foe):
    m = MV.get(mv)
    if not m or m["power"] == 0:
        return 0.0
    stab = 1.5 if m["type"] == "WATER" else 1.0
    return m["power"] * eff(m["type"], foe) * stab * min(m["acc"], 100) / 100


def alive_events(path):
    out = []
    for line in open(path):
        e = json.loads(line)
        if e["kind"] == "rollback":
            out = [x for x in out if x["frame"] <= e["frame"]]
            continue
        out.append(e)
    return out


def fights(path):
    cur = None
    res = []
    for e in alive_events(path):
        t = e["text"]
        m = re.match(r"(\w+) sends out", t)
        if e["kind"] == "battle" and m:
            cur = dict(who=m.group(1), turns=[], frame=e["frame"])
        elif e["kind"] == "battle" and t.startswith("battle over"):
            if cur:
                res.append(cur)
            cur = None
        elif cur is not None and e["kind"] == "battle" and "action" in e["nums"]:
            cur["turns"].append(e["nums"])
    return res


def classify(fl):
    known = set()
    out = []
    for f in fl:
        rows = []
        T = f["turns"]
        for i, n in enumerate(T):
            a = n["action"]
            al = a.lower()
            foe = n["enemy"]
            nxt = T[i + 1] if i + 1 < len(T) else None
            dealt = nxt["dealt_last"] if nxt else None
            m = re.match(r"Sleep\((\d+)\)", n["my_status"])
            item = al in ITEMS
            if m and int(m.group(1)) > 1 and not item:
                c = "sleep"
            elif item:
                c = "heal"
            elif nxt is not None and nxt["my_last_move"] == "#65535":
                # gLastMoves[0] = 0xFFFF: the move did not go off (full paralysis, a flinch, confusion)
                c = "unable"
            elif n["policy"] in ("Heal", "FullRestore") and a in MV:
                c = "fallback"
            elif a in MV and MV[a]["power"] > 0:
                # A turn that ends with this foe fainting (the next turn faces another mon, or the
                # fight ends) cannot have been shortened by a better move unless it needed two hits.
                killed = nxt is None or nxt["enemy"] != foe or nxt["enemy_hp"] > n["enemy_hp"]
                best = max([score(k, foe) for k in known | {a}] + [0.0])
                if score(a, foe) < 0.9 * best and not killed:
                    c = "badmove"
                elif dealt == 0 and nxt and nxt["enemy"] == foe:
                    c = "miss"
                else:
                    c = "attack"
            else:
                c = "setup"
            if a in MV:
                known.add(a)
            rows.append((c, a, foe))
        out.append((f, rows))
    return out


def main():
    runs = [a for a in sys.argv[1:] if not a.startswith("--")]
    for r in runs:
        res = classify(fights(r + "/events.jsonl"))
        tot = {}
        for f, rows in res:
            for c, *_ in rows:
                tot[c] = tot.get(c, 0) + 1
            if "--fights" in sys.argv:
                print(f"  {f['who']:34s} {len(rows):2d} " + " ".join(f"{c}:{a}" for c, a, *_ in rows))
        n = sum(tot.values())
        order = sorted(tot.items(), key=lambda x: -x[1])
        print(f"{os.path.basename(r)}: {len(res)} trainer fights, {n} turns, " + ", ".join(f"{k} {v}" for k, v in order))
        print("   seconds at 503 f/turn: " + ", ".join(f"{k} {v * PER_TURN / FPS:.0f}" for k, v in order))


main()
