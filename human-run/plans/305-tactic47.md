# Plan 305-tactic47

Parent: `304-starter31`. Change: surge: sacs 0->1 (descend.py; confirmed on seeds 1 and 2)

The step-by-step description, citations and branch rules in `001-crude.md` still apply except
where the parameters below differ; `frlg-human` reads only the block below. Every segment cost
is a model estimate (see `001-crude.md` for what is ledger, tile-count or guess).

```json
{
 "name": "305-tactic47",
 "starter_min_total": 73,
 "hunts": [
  {
   "after": "bill",
   "site": "MAP_ROUTE25",
   "area": "land",
   "steps": 240,
   "detour_tiles": 2,
   "skip": [
    "SPECIES_ODDISH",
    "SPECIES_PIDGEY",
    "SPECIES_CATERPIE",
    "SPECIES_WEEDLE",
    "SPECIES_KAKUNA",
    "SPECIES_METAPOD"
   ]
  },
  {
   "after": "route3",
   "site": "MAP_ROUTE3",
   "area": "land",
   "steps": 30,
   "detour_tiles": 2,
   "skip": []
  },
  {
   "after": "route5-6",
   "site": "MAP_ROUTE5",
   "area": "land",
   "steps": 120,
   "detour_tiles": 2,
   "skip": []
  }
 ],
 "specials": [
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
  },
  {
   "after": "fly-viridian",
   "kind": "trade",
   "species": [
    "SPECIES_MR_MIME"
   ],
   "requires": [
    "SPECIES_ABRA"
   ],
   "frames": 2600
  },
  {
   "after": "hall-of-fame-1",
   "kind": "buy",
   "species": [
    "SPECIES_ABRA"
   ],
   "cost": 2400,
   "frames": 2400
  }
 ],
 "fill": [
  {
   "after": "",
   "site": "MAP_SEAFOAM_ISLANDS_B3F",
   "area": "land",
   "steps": 80,
   "detour_tiles": 50
  },
  {
   "after": "",
   "site": "MAP_VERMILION_CITY",
   "area": "fish",
   "steps": 38,
   "detour_tiles": 25
  },
  {
   "after": "",
   "site": "MAP_POWER_PLANT",
   "area": "land",
   "steps": 161,
   "detour_tiles": 20
  },
  {
   "after": "",
   "site": "MAP_ROUTE6",
   "area": "fish",
   "steps": 64,
   "detour_tiles": 60
  },
  {
   "after": "",
   "site": "MAP_ROUTE12",
   "area": "fish",
   "steps": 20,
   "detour_tiles": 60
  },
  {
   "after": "",
   "site": "MAP_FUCHSIA_CITY",
   "area": "fish",
   "steps": 24,
   "detour_tiles": 50
  },
  {
   "after": "",
   "site": "MAP_SEAFOAM_ISLANDS_B2F",
   "area": "land",
   "steps": 42,
   "detour_tiles": 25
  },
  {
   "after": "",
   "site": "MAP_ROUTE13",
   "area": "land",
   "steps": 85,
   "detour_tiles": 30
  },
  {
   "after": "",
   "site": "MAP_SEAFOAM_ISLANDS_B4F",
   "area": "land",
   "steps": 92,
   "detour_tiles": 30
  },
  {
   "after": "",
   "site": "MAP_ROUTE11",
   "area": "land",
   "steps": 116,
   "detour_tiles": 30
  },
  {
   "after": "",
   "site": "MAP_ROUTE21_NORTH",
   "area": "land",
   "steps": 122,
   "detour_tiles": 25
  },
  {
   "after": "",
   "site": "MAP_DIGLETTS_CAVE_B1F",
   "area": "land",
   "steps": 62,
   "detour_tiles": 30
  },
  {
   "after": "",
   "site": "MAP_VICTORY_ROAD_2F",
   "area": "land",
   "steps": 154,
   "detour_tiles": 25
  },
  {
   "after": "",
   "site": "MAP_ROUTE23",
   "area": "land",
   "steps": 76,
   "detour_tiles": 40
  },
  {
   "after": "",
   "site": "MAP_SAFARI_ZONE_WEST",
   "area": "land",
   "steps": 95,
   "detour_tiles": 10
  },
  {
   "after": "",
   "site": "MAP_POKEMON_TOWER_7F",
   "area": "land",
   "steps": 152,
   "detour_tiles": 25
  },
  {
   "after": "",
   "site": "MAP_SAFARI_ZONE_CENTER",
   "area": "land",
   "steps": 77,
   "detour_tiles": 30
  }
 ],
 "grind_pre_e4": 8,
 "grind_level": 42,
 "grind_yield": 90,
 "stones": [],
 "reset_aggr": 0.84,
 "weaken": true,
 "ball": "ultra",
 "fight_until_level": 16,
 "skip_groups": [
  "cerulean-gym-trainer",
  "celadon-gym-trainers",
  "viridian-gym-trainers",
  "blaine-quiz",
  "route3-trainers",
  "mt-moon-trainers",
  "fuchsia-gym-trainers",
  "saffron-gym-trainers",
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
  "giovanni-badge": 0.7,
  "oak-national-dex": 0.9,
  "brock": 0.8,
  "hall-of-fame-1": 0.6,
  "indigo-heal": 0.5
 },
 "starter_min_stats": [
  20,
  0,
  0,
  0,
  11,
  0
 ],
 "type_strength": 1.0,
 "type_floor": 0.9,
 "audited_forced": true,
 "pass_min_catch": 45,
 "safari_model": true,
 "money_model": true,
 "ball_batch": 20,
 "calib_enc": true,
 "rod": "super",
 "repel_on": [
  "mt-moon-1f",
  "cerulean-cave",
  "route3",
  "nugget-bridge",
  "cycling-road",
  "route6-encounters",
  "rock-tunnel",
  "route9",
  "route22"
 ],
 "calib_battle": true,
 "enc_path": true,
 "detour_map": true,
 "detour_bike": true,
 "battle_sim": true,
 "tms": [
  {
   "after": "route4",
   "mv": "MOVE_MEGA_PUNCH",
   "frames": 1500
  },
  {
   "after": "tm28-dig",
   "mv": "MOVE_DIG",
   "frames": 800
  },
  {
   "after": "mansion",
   "mv": "MOVE_BLIZZARD",
   "frames": 1100
  },
  {
   "after": "mewtwo",
   "mv": "MOVE_SURF",
   "frames": 600,
   "mewtwo": true
  }
 ],
 "keep_moves": [
  "MOVE_DIG",
  "MOVE_SURF",
  "MOVE_BLIZZARD"
 ],
 "xbuys": [
  {
   "after": "celadon-tea",
   "attack": 6,
   "speed": 4,
   "frames": 1200
  },
  {
   "after": "fly-celadon-2",
   "attack": 2,
   "special": 30,
   "speed": 10,
   "frames": 1500
  }
 ],
 "fight_tactics": {
  "surge": {
   "x": [
    "attack",
    "attack"
   ],
   "sacs": 1
  },
  "erika": {
   "x": [
    "attack",
    "attack",
    "attack"
   ]
  },
  "koga": {
   "x": [
    "special",
    "special"
   ]
  },
  "sabrina": {
   "x": [
    "special",
    "special",
    "speed"
   ],
   "sacs": 1
  },
  "lorelei": {
   "x": [
    "special",
    "special",
    "speed"
   ],
   "sacs": 1
  },
  "bruno": {
   "x": [
    "special",
    "speed"
   ],
   "sacs": 1
  },
  "agatha": {
   "x": [
    "special",
    "speed"
   ],
   "sacs": 1
  },
  "lance": {
   "x": [
    "special",
    "special",
    "speed"
   ],
   "sacs": 1
  },
  "champion": {
   "x": [
    "special",
    "special",
    "speed",
    "special"
   ],
   "sacs": 3
  },
  "rival-ss-anne": {
   "x": []
  },
  "rival-silph": {
   "x": [
    "special",
    "special",
    "speed"
   ],
   "sacs": 1
  },
  "rival-route22": {
   "x": [
    "special",
    "special",
    "speed"
   ],
   "sacs": 1
  },
  "giovanni": {
   "x": [
    "special",
    "speed"
   ]
  },
  "lorelei-2": {
   "x": [
    "special",
    "special"
   ],
   "sacs": 1
  },
  "bruno-2": {
   "x": [
    "special",
    "special"
   ],
   "sacs": 1
  },
  "agatha-2": {
   "x": [
    "special",
    "special"
   ],
   "sacs": 1
  },
  "lance-2": {
   "x": [
    "special",
    "special"
   ],
   "sacs": 1
  },
  "champion-2": {
   "x": [
    "special",
    "special",
    "speed",
    "special"
   ],
   "sacs": 3
  }
 },
 "early_dig": true,
 "erika_after_tower": true,
 "grinds": [
  {
   "after": "cerulean-heal",
   "battles": 5,
   "level": 12,
   "yield_": 58
  },
  {
   "after": "captain-hm01",
   "battles": 8,
   "level": 20,
   "yield_": 70
  }
 ]
}
```

## Score

- rung **WR+160m**, EHT 30.184 h at R=40000, 95% CI of the paired difference to the parent [-2.1329, -0.0218] (negative is better), parent EHT 31.2475
- evaluation line: `human-run/scores.jsonl`, plan `d305-47`; ledger of everything tried is in that file.
