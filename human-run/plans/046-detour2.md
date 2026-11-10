# Plan 046-detour2

Parent: `045-passcatch45`. Change: fill[0] MAP_ROUTE21_SOUTH detour x0.5 (descend.py; confirmed on seeds 1 and 2)

The step-by-step description, citations and branch rules in `001-crude.md` still apply except
where the parameters below differ; `frlg-human` reads only the block below. Every segment cost
is a model estimate (see `001-crude.md` for what is ledger, tile-count or guess).

```json
{
 "name": "046-detour2",
 "starter_min_total": 71,
 "hunts": [
  {
   "after": "bill",
   "site": "MAP_ROUTE25",
   "area": "land",
   "steps": 120,
   "detour_tiles": 2,
   "skip": [
    "SPECIES_ODDISH",
    "SPECIES_PIDGEY",
    "SPECIES_CATERPIE",
    "SPECIES_WEEDLE",
    "SPECIES_KAKUNA",
    "SPECIES_METAPOD"
   ]
  }
 ],
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
  },
  {
   "after": "to-pewter",
   "kind": "gift",
   "species": [
    "SPECIES_AERODACTYL"
   ],
   "frames": 2800,
   "level": 5
  },
  {
   "after": "ss-anne",
   "kind": "trade",
   "species": [
    "SPECIES_FARFETCHD"
   ],
   "requires": [
    "SPECIES_SPEAROW"
   ],
   "frames": 2200
  },
  {
   "after": "blaine-walk",
   "kind": "trade",
   "species": [
    "SPECIES_TANGELA"
   ],
   "requires": [
    "SPECIES_VENONAT"
   ],
   "frames": 2400
  },
  {
   "after": "blaine-walk",
   "kind": "trade",
   "species": [
    "SPECIES_ELECTRODE"
   ],
   "requires": [
    "SPECIES_RAICHU"
   ],
   "frames": 2400
  }
 ],
 "fill": [
  {
   "after": "",
   "site": "MAP_ROUTE21_SOUTH",
   "area": "land",
   "steps": 48,
   "detour_tiles": 15
  },
  {
   "after": "",
   "site": "MAP_SEAFOAM_ISLANDS_B4F",
   "area": "land",
   "steps": 80,
   "detour_tiles": 60
  },
  {
   "after": "",
   "site": "MAP_SAFARI_ZONE_CENTER",
   "area": "land",
   "steps": 96,
   "detour_tiles": 60
  },
  {
   "after": "",
   "site": "MAP_POWER_PLANT",
   "area": "land",
   "steps": 80,
   "detour_tiles": 80
  },
  {
   "after": "",
   "site": "MAP_ROUTE11",
   "area": "land",
   "steps": 64,
   "detour_tiles": 30
  },
  {
   "after": "",
   "site": "MAP_ROUTE23",
   "area": "land",
   "steps": 48,
   "detour_tiles": 40
  },
  {
   "after": "",
   "site": "MAP_DIGLETTS_CAVE_B1F",
   "area": "land",
   "steps": 32,
   "detour_tiles": 60
  },
  {
   "after": "",
   "site": "MAP_SAFARI_ZONE_CENTER",
   "area": "fish",
   "steps": 25,
   "detour_tiles": 5
  },
  {
   "after": "",
   "site": "MAP_ROUTE13",
   "area": "land",
   "steps": 64,
   "detour_tiles": 30
  },
  {
   "after": "",
   "site": "MAP_SAFARI_ZONE_EAST",
   "area": "land",
   "steps": 96,
   "detour_tiles": 10
  },
  {
   "after": "",
   "site": "MAP_ROUTE12",
   "area": "fish",
   "steps": 25,
   "detour_tiles": 60
  },
  {
   "after": "",
   "site": "MAP_ROUTE6",
   "area": "fish",
   "steps": 25,
   "detour_tiles": 60
  },
  {
   "after": "",
   "site": "MAP_SAFARI_ZONE_NORTH",
   "area": "land",
   "steps": 96,
   "detour_tiles": 10
  },
  {
   "after": "",
   "site": "MAP_SAFARI_ZONE_WEST",
   "area": "land",
   "steps": 96,
   "detour_tiles": 10
  },
  {
   "after": "",
   "site": "MAP_ROUTE10",
   "area": "fish",
   "steps": 25,
   "detour_tiles": 40
  }
 ],
 "grind_pre_e4": 0,
 "grind_level": 42,
 "grind_yield": 90,
 "stones": [],
 "reset_aggr": 0.5,
 "weaken": false,
 "ball": "ultra",
 "fight_until_level": 7,
 "skip_groups": [
  "silph-grunts",
  "cerulean-gym-trainer",
  "surge-gym-trainers",
  "celadon-gym-trainers",
  "viridian-gym-trainers",
  "blaine-quiz",
  "route3-trainers",
  "mt-moon-trainers",
  "fuchsia-gym-trainers",
  "saffron-gym-trainers",
  "mansion-trainers",
  "mt-ember-trainers"
 ],
 "mistake_rate": 1.0,
 "version": "LG",
 "battle_scale": 1.75,
 "mewtwo": true,
 "group_n": {
  "hideout-grunts": 3,
  "victory-road-trainers": 4
 },
 "teleport_abra": true,
 "reset_aggr_at": {
  "misty-badge": 0.7,
  "surge-badge": 0.7,
  "erika-badge": 0.7,
  "koga-badge": 0.7,
  "sabrina-badge": 0.7,
  "blaine-badge-and-sevii-scene": 0.7,
  "giovanni-badge": 0.7
 },
 "starter_min_stats": [
  20,
  0,
  0,
  0,
  12,
  0
 ],
 "type_strength": 1.0,
 "type_floor": 0.9,
 "audited_forced": true,
 "pass_min_catch": 45
}
```

## Score

- rung **WR+40m**, EHT 65.158 h at R=40000, 95% CI of the paired difference to the parent [-5.5802, -0.4726] (negative is better), parent EHT 67.6339
- evaluation line: `human-run/scores.jsonl`, plan `d46-2`; ledger of everything tried is in that file.
