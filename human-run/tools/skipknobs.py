#!/usr/bin/env python3
"""Conditional-skip candidates (`cond_skips`, `crates/frlg-human/src/params.rs` `CondSkip`).

A lever is a group of route segments the human drops when his clock at the last WR split he passed is more than
`behind` seconds past the WR's (`wrsplits::WR_SPLITS`); the species they would have caught are then left to the
dex fill before the League (the plan's `fill` list) or lost. The levers are the optional dex pieces the runner names:
Power Plant ("skip Plants", STRATEGY 5.3: ~2 min of duplicates, 11 tries for a Pikachu), the Safari hunts (7 planned
catches, bad Safari = -2 to -3 min), the Pewter-side detours (Diglett's Cave, the Mr. Mime trade) and the late
route hunts. Segment names are the ones `frlg-human report` prints. stdlib only."""
import copy

LEVERS = {
    # Fly to Lavender, surf to the plant, the Thunder Stone pickup and the hunt (decomp: PowerPlant/map.json items).
    'powerplant': ['fly-lavender-2', 'powerplant-walk', 'powerplant-out', 'elixir-thunder@powerplant-out',
                   'hunt-power_plant-land@powerplant-out'],
    'powerplant-hunt': ['hunt-power_plant-land@powerplant-out'],
    'safari-hunts': ['hunt-safari_zone_east-land@safari-walk', 'hunt-safari_zone_center-land@safari-walk'],
    'safari-center': ['hunt-safari_zone_center-land@safari-walk'],
    'diglett': ['hunt-digletts_cave_b1f-land@fly-viridian'],
    'mrmime': ['trade-mr_mime@fly-viridian'],
    'route22-hunt': ['hunt-route22-land@route22'],
    'route23-hunt': ['hunt-route23-land@route23'],
    'bond-hunt': ['hunt-three_island_bond_bridge-land@bond-bridge'],
}
# Seconds behind the WR at the last split passed. The model's mean sits 2-7 min ahead of the WR record at the
# mid-game splits (`frlg-human splits`), so most of the useful range is negative.
THRESHOLDS = (-420, -300, -180, -60, 60, 240)


def candidates(p):
    out = []
    cur = p.get('cond_skips', [])
    have = {tuple(c['segs']): c for c in cur}
    for name, segs in LEVERS.items():
        key = tuple(segs)
        for t in THRESHOLDS:
            if key in have and have[key]['behind'] == t:
                continue
            new = [c for c in cur if tuple(c['segs']) != key] + [{'segs': list(segs), 'behind': t}]
            out.append(('cskip', f'cond_skip {name} behind>{t}s', {'cond_skips': new}))
        if key in have:
            out.append(('cskip', f'cond_skip drop {name}', {'cond_skips': [c for c in cur if tuple(c['segs']) != key]}))
    return out
