# Plan 003-nohunts

Parent: `002-aggr05`. Change: no in-route hunts: only opportunistic catches while passing, gifts, stones, then the fill list at Oak's check. In-route hunts cost 100-500 s per new species against ~200 s for Safari-type fill sites

The step-by-step description, citations and branch rules in `001-crude.md` still apply except
where the parameters below differ; `frlg-human` reads only the block below. Every segment cost
is a model estimate (see `001-crude.md` for what is ledger, tile-count or guess).

```json
{
 "name": "003-nohunts",
 "starter_min_total": 72,
 "hunts": [],
 "specials": [
  {
   "after": "route4",
   "kind": "gift",
   "species": [
    "SPECIES_MAGIKARP"
   ],
   "frames": 900,
   "level": 30
  },
  {
   "after": "celadon-tea",
   "kind": "gift",
   "species": [
    "SPECIES_EEVEE"
   ],
   "frames": 1500,
   "level": 30
  },
  {
   "after": "silph-giovanni",
   "kind": "gift",
   "species": [
    "SPECIES_LAPRAS"
   ],
   "frames": 700,
   "level": 30
  },
  {
   "after": "snorlax",
   "kind": "static",
   "species": [
    "SPECIES_SNORLAX"
   ],
   "frames": 400,
   "level": 30
  },
  {
   "after": "blaine-walk",
   "kind": "gift",
   "species": [
    "SPECIES_KABUTO"
   ],
   "frames": 1800,
   "level": 30
  }
 ],
 "fill": [
  {
   "after": "",
   "site": "MAP_SAFARI_ZONE_CENTER",
   "area": "land",
   "steps": 300,
   "detour_tiles": 60
  },
  {
   "after": "",
   "site": "MAP_SAFARI_ZONE_NORTH",
   "area": "land",
   "steps": 300,
   "detour_tiles": 10
  },
  {
   "after": "",
   "site": "MAP_SAFARI_ZONE_EAST",
   "area": "land",
   "steps": 300,
   "detour_tiles": 10
  },
  {
   "after": "",
   "site": "MAP_SAFARI_ZONE_WEST",
   "area": "land",
   "steps": 300,
   "detour_tiles": 10
  },
  {
   "after": "",
   "site": "MAP_SAFARI_ZONE_CENTER",
   "area": "fish",
   "steps": 80,
   "detour_tiles": 5
  },
  {
   "after": "",
   "site": "MAP_ROUTE12",
   "area": "fish",
   "steps": 80,
   "detour_tiles": 60
  },
  {
   "after": "",
   "site": "MAP_ROUTE6",
   "area": "fish",
   "steps": 80,
   "detour_tiles": 60
  },
  {
   "after": "",
   "site": "MAP_ROUTE13",
   "area": "land",
   "steps": 200,
   "detour_tiles": 30
  },
  {
   "after": "",
   "site": "MAP_ROUTE11",
   "area": "land",
   "steps": 200,
   "detour_tiles": 30
  },
  {
   "after": "",
   "site": "MAP_DIGLETTS_CAVE_B1F",
   "area": "land",
   "steps": 100,
   "detour_tiles": 60
  },
  {
   "after": "",
   "site": "MAP_POWER_PLANT",
   "area": "land",
   "steps": 250,
   "detour_tiles": 80
  },
  {
   "after": "",
   "site": "MAP_SEAFOAM_ISLANDS_B4F",
   "area": "land",
   "steps": 250,
   "detour_tiles": 60
  },
  {
   "after": "",
   "site": "MAP_ROUTE21_SOUTH",
   "area": "land",
   "steps": 150,
   "detour_tiles": 30
  },
  {
   "after": "",
   "site": "MAP_ROUTE10",
   "area": "fish",
   "steps": 80,
   "detour_tiles": 40
  },
  {
   "after": "",
   "site": "MAP_ROUTE23",
   "area": "land",
   "steps": 150,
   "detour_tiles": 40
  },
  {
   "after": "",
   "site": "MAP_CERULEAN_CAVE_B1F",
   "area": "land",
   "steps": 300,
   "detour_tiles": 100
  }
 ],
 "grind_pre_e4": 20,
 "grind_level": 42,
 "grind_yield": 90,
 "stones": [
  "FIRE",
  "THUNDER",
  "MOON",
  "MOON"
 ],
 "reset_aggr": 0.5
}
```

## Score

- rung **WR+80m**, EHT 124.6859 h at R=10000, 95% CI of the paired difference to the parent [-3099.6217, -1279.5191] (negative is better), parent EHT 1966.9048
- evaluation line: `human-run/scores.jsonl`, plan `003-nohunts`; ledger of everything tried is in that file.
