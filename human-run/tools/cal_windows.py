#!/usr/bin/env python3
"""Sum `cal_decompose.py --json` stage decompositions into WR split windows.

    python3 human-run/tools/cal_windows.py <decomp.json> [<decomp.json> ...]

A window is named by its last stage (the controller's checkpoint at the split). Seconds at 59.7275 fps.
"""
import json
import sys

FPS = 59.7275
WINDOWS = [
    ("Brock", "brock-done"),
    ("Misty", "misty"),
    ("Bill", "bill"),
    ("Bike", "bike"),
    ("Rock Tunnel", "rock-tunnel"),
    ("Giovanni 1", "giovanni1-beaten"),
    ("Snorlax", "cycling-road"),
    ("Koga", "koga"),
]


def main():
    stages = {}
    order = []
    for p in sys.argv[1:]:
        d = json.load(open(p))
        for k, v in d.items():
            if v["frames"] == 0:
                continue
            stages[k] = v
            order.append(k)
    cols = ["frames", "trainer", "wild", "walk", "text+menu+warp", "bump+turn+idle", "transport"]
    print(f"{'window':12s} " + " ".join(f"{c:>15s}" for c in cols))
    w = 0
    acc = None
    for k in order:
        v = stages[k]
        c = v["classes"]
        row = {
            "frames": v["frames"],
            "trainer": c.get("trainer", 0),
            "wild": c.get("wild", 0),
            "walk": c.get("walk", 0),
            "text+menu+warp": c.get("text", 0) + c.get("menu", 0) + c.get("warp", 0),
            "bump+turn+idle": c.get("bump", 0) + c.get("turn", 0) + c.get("idle", 0),
            "transport": max(0, c.get("walk", 0) - v["ideal_walk"]),
        }
        acc = row if acc is None else {x: acc[x] + row[x] for x in cols}
        if w < len(WINDOWS) and k == WINDOWS[w][1]:
            print(f"{WINDOWS[w][0]:12s} " + " ".join(f"{acc[x] / FPS:15.0f}" for x in cols))
            acc = None
            w += 1


if __name__ == "__main__":
    main()
