#!/usr/bin/env python3
"""Decompose played controller stages into walking / text-menu / battle / overhead frames.

Input: one CSV per stage from `cargo run --release --example cal_trace` (crates/frlg-human/examples),
named `<prefix><stage>.csv`, plus the run's `events.jsonl` and `checkpoints.jsonl`.

    python3 human-run/tools/cal_decompose.py <csv prefix> <checkpoints.jsonl> <events.jsonl> [--json out]

Every frame is in exactly one class:

* held episodes (the player has no field control: not `CB2_Overworld`, `sLockFieldControls`, a script
  context running or waiting, a message box, a palette fade, or `gPlayerAvatar.preventStep`), labelled by
  what happens inside them:
    - `wild` / `trainer`: the episode contains `gMain.inBattle` (trainer when `gBattleTypeFlags` has
      BATTLE_TYPE_TRAINER, bit 3, `decompiled/include/constants/battle.h`); the whole episode counts, so
      a trainer's sighting walk and text, the battle and the return to the field are one cost;
    - `menu`: a non-field screen (party menu, bag, region map, TM case, summary, options) and no battle;
    - `warp`: the map changed inside the episode (doors, stairs, warp tiles, Fly's arrival);
    - `text`: anything else (NPC text, scripts, item pickups, Cut animations, the start menu).
* free frames (the player could step):
    - `walk`: tileTransitionState non-zero (a step in progress);
    - `bump`: runningState 2 with the tile settled (walking into something);
    - `turn`: runningState 1 (turning in place);
    - `idle`: free, standing, nothing moving (the controller thinking, re-planning, waiting, jitter).

Steps are counted per movement episode by tile changes and by transport (`gPlayerAvatar.flags`: ON_FOOT 1,
MACH_BIKE 2, ACRO_BIKE 4, SURFING 8, DASH 0x80; `decompiled/include/global.fieldmap.h`).
"""
import csv
import json
import os
import sys
from collections import defaultdict

DECOMP = os.environ.get("FRLG_DECOMP_RO", "/home/luuk/.cache/speedrun-frlg/decompiled")
# Frames per tile: walk 16, run and surf 8 (`PlayerWalkNormal` / `PlayerRun` / `PlayerWalkFast`,
# `decompiled/src/field_player_avatar.c:843-863`; speeds `event_object_movement.c:158-162`), bike 6 as in
# `route.rs` `Era` (measured here: 6.0 on flat ground, ~5.1 on Cycling Road's slope).
FPT = {"walk": 16, "run": 8, "surf": 8, "bike": 6}


def load_maps():
    """(group, num) -> (name, biking allowed, running allowed), from `data/maps/`.
    Biking needs the header's `allowCycling` and not an indoor map type (`decompiled/src/bike.c:245-252`);
    running needs `allowRunning` (`bike.c:253-259`)."""
    g = json.load(open(f"{DECOMP}/data/maps/map_groups.json"))
    out = {}
    for gi, grp in enumerate(g["group_order"]):
        for mi, name in enumerate(g[grp]):
            try:
                m = json.load(open(f"{DECOMP}/data/maps/{name}/map.json"))
            except FileNotFoundError:
                continue
            bike = m.get("allow_cycling", False) and m.get("map_type") != "MAP_TYPE_INDOOR"
            out[(str(gi), str(mi))] = (name, bike, m.get("allow_running", False))
    return out


MAPS = None


def best_mode(key, cur):
    if cur == "surf":
        return "surf"
    name, bike, run = MAPS.get(key, ("?", False, False))
    return "bike" if bike else ("run" if run else "walk")

FIELD_MENUS = (
    "PartyMenu",
    "BagMenu",
    "RegionMap",
    "FlyMap",
    "TMCase",
    "Summary",
    "PSS",
    "OptionMenu",
    "PSA",
    "UseItem",
)


def mode(av):
    av = int(av)
    if av & 8:
        return "surf"
    if av & 6:
        return "bike"
    if av & 0x80:
        return "run"
    return "walk"


def held(r):
    return (
        r["cb2"] != "CB2_Overworld"
        or r["lock"] != "0"
        or r["ctx"] in ("0", "1")
        or r["msg"] != "0"
        or r["fade"] != "0"
        or r["prevent"] != "0"
    )


def decompose(path):
    rows = list(csv.DictReader(open(path)))
    out = defaultdict(int)
    steps = defaultdict(int)
    step_frames = defaultdict(int)
    map_steps = defaultdict(lambda: defaultdict(int))
    ideal = 0
    episodes = []  # (kind, start, end)
    bumps = 0
    i = 0
    n = len(rows)
    while i < n:
        r = rows[i]
        if held(r):
            j = i
            battle = None
            menu = False
            maps = {(r["mg"], r["mn"])}
            while j < n and held(rows[j]):
                q = rows[j]
                if q["in_battle"] == "1" and battle is None:
                    battle = "trainer" if int(q["btf"]) & 8 else "wild"
                if any(m in q["cb2"] for m in FIELD_MENUS):
                    menu = True
                maps.add((q["mg"], q["mn"]))
                j += 1
            if battle:
                k = battle
            elif menu:
                k = "menu"
            elif len(maps) > 1:
                k = "warp"
            else:
                k = "text"
            out[k] += j - i
            episodes.append((k, int(rows[i]["frame"]), int(rows[j - 1]["frame"])))
            i = j
            continue
        # A free frame: tileTransitionState non-zero is a step in progress; runningState 2 with the
        # tile settled is walking into something (the bump animation repeats while the direction is
        # held); runningState 1 is a turn in place (`decompiled/src/field_player_avatar.c`).
        p = (r["mg"], r["mn"], r["x"], r["y"])
        if i and p != (rows[i - 1]["mg"], rows[i - 1]["mn"], rows[i - 1]["x"], rows[i - 1]["y"]):
            m = mode(r["av_flags"])
            steps[m] += 1
            key = (r["mg"], r["mn"])
            map_steps[MAPS.get(key, ("?",))[0]][m] += 1
            ideal += FPT[best_mode(key, m)]
        if r["tile_state"] != "0":
            out["walk"] += 1
            step_frames[mode(r["av_flags"])] += 1
        elif r["run_state"] == "2":
            out["bump"] += 1
            if i and (rows[i - 1]["run_state"] != "2" or rows[i - 1]["tile_state"] != "0"):
                bumps += 1
        elif r["run_state"] == "1":
            out["turn"] += 1
        else:
            out["idle"] += 1
        i += 1
    total = n
    return {
        "frames": total,
        "classes": dict(out),
        "steps": dict(steps),
        "step_frames": dict(step_frames),
        "bumps": bumps,
        "ideal_walk": ideal,
        "map_steps": {k: dict(v) for k, v in map_steps.items()},
        "episodes": episodes,
    }


def main():
    global MAPS
    MAPS = load_maps()
    prefix, ckpath, evpath = sys.argv[1:4]
    ck = [json.loads(l) for l in open(ckpath)]
    names = [c["name"] for c in ck]
    i0 = names.index(sys.argv[sys.argv.index("--from") + 1]) if "--from" in sys.argv else 0
    res = {}
    for a, b in zip(ck[i0:], ck[i0 + 1 :]):
        try:
            d = decompose(f"{prefix}{b['name']}.csv")
        except FileNotFoundError:
            continue
        d["start"], d["end"] = a["frame"], b["frame"]
        res[b["name"]] = d
    cols = ["walk", "bump", "turn", "idle", "text", "menu", "warp", "wild", "trainer"]
    print(f"{'stage':15s} {'frames':>6s} " + " ".join(f"{c:>7s}" for c in cols) + "  steps")
    for k, d in res.items():
        c = d["classes"]
        st = " ".join(f"{m}:{d['steps'][m]}/{d['step_frames'][m]}" for m in d["steps"])
        print(f"{k:15s} {d['frames']:6d} " + " ".join(f"{c.get(x, 0):7d}" for x in cols) + f"  {st} bumps:{d['bumps']} ideal:{d['ideal_walk']}")
    if "--json" in sys.argv:
        json.dump(res, open(sys.argv[sys.argv.index("--json") + 1], "w"), indent=1)


if __name__ == "__main__":
    main()
