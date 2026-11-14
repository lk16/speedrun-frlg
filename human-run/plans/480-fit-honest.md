# Plan 480-fit-honest

Parent: `452-fidelity-retune` (same policy: `reset_aggr`, hunts, tactics, thresholds). **Model change, not a plan change.** The model
under it is the 2026-11-14 fidelity fit (`journal/2026-11-14-22-00-fidelity-fit.md`): per-segment unsalvaged-death hazards
(`fatal`) fitted to the runner's observed attempts, per-event execution losses (`slow`) priced from the runner's quotes and extra
noise on every WR split window (`win_cv`). `hazard` (the old guessed early deaths) and `regime_bad` (the bad-day mixture) are gone:
the fitted hazards are pooled over the runner's days, good and bad. (Numbered 480 so that it cannot collide with the plans of the
other descents that started from 452.)

What is fitted, on what, and what is a guess:

| Parameter | Evidence | n | Status |
| --- | --- | --- | --- |
| `fatal` forest-trainers 0.181, pewter-gym-trainer 0.095, brock 0.392 | survival of the attempts past the starter gate to the Brock split: 24 of 60 (40%, Wilson 29-53%), `video-analysis/data/accepted-runs.json`; the engine alone kills 11% | 60 attempts, 9 stream days | window total = data; split 2:1:5 = the `!deaths` list (sammy x2, liam, brock x5) |
| route3-trainers, mt-moon-miguel 0.021, misty, rival-cerulean 0.041 | Brock -> Misty split: 8 of 22 end (36%, CI 20-57%); the engine already kills 28% | 22 | total = data minus engine; split = guess |
| nugget-bridge-trainers, rock-tunnel-trainers 0.020, rival-ss-anne 0.059 | Misty -> Rock Tunnel: 4 of 14 (29%); engine 19% | 14 | total = data; split = guess (T06: boat rival deaths) |
| hideout-giovanni, surge 0.022, rival-tower 0.043 | 0 of 10 observed; partial pooling with the later windows (k = 6, pool 22% per window) gives 8% | 10 | shrunk estimate, interval 0-28% |
| koga 0.127, fuchsia-gym-trainers 0.033 | Snorlax -> Koga: 1 of 9; pooled 16%; "Koga deaths are a huge problem" (T09, T10) | 9 | shrunk estimate |
| blaine 0.200, mansion-trainers, three-island-paxton 0.072 | Koga -> Sevii: 3 of 8 (pooled 31%) | 8 | shrunk estimate; the split is a guess |
| rival-silph 0.070, sabrina, erika 0.103, giovanni 0.070 | Sevii -> Giovanni 3: 2 of 5 (pooled 30%) | 5 | shrunk estimate; the split is a guess |
| rival-route22 0.199, victory-road 0.071 | Giovanni 3 -> door: 1 of 3 (pooled 26%) | 3 | shrunk estimate |
| post-door `fatal` (lorelei .098 ... champion .186, Warehouse, Round 2 League; product of survivals S = 0.183) | **no direct evidence**: of five door arrivals four were reset at the pace gate (2:39-2:49) and one finished (3:29); S is the calibration residual that makes the attempts per WR finish equal the runner's 15 811 | 5 | **guess**; shape = anecdote weights (`tools/fit_hazards.py` POST_WEIGHTS); S = 0.088 with no `slow`, ~1 with `slow_scale` 2.4 |
| `slow` events | costs from the runner's quotes (Safari 3:20 for two mons, Power Plant duplicates ~2 min, five cave encounters ~1:30, a missed bike line ~40 s, spec-strat fallback 1.5-2 min); probabilities | quotes only | costs = quotes; **probabilities = guesses**; mean +1.8 min per run, no-reset p10/p50/p90 3:25:48 / 3:31:44 / 3:39:07 |
| `win_cv` 0.033 | runner's windows 7.3% rms (deep run vs LG PB, 13 windows) against the model's 6.5%; the model's Icefall/Warehouse/Bike windows are 1-4% | 13 windows | weak: the upper end of the runner's range (0 if the PB's own spread is subtracted) |

Result (seed 5, R = 3 200 000): WR rung 212 finishes, **15 094 attempts per finish, EHT(WR) 3 377 h** (452: 59.8 h, 151 attempts
per finish; the runner: 15 811 attempts, ~2 950 h at his 11.2 min per attempt). P(finish <= WR | reaches the door at 60 caught,
no pace resets): 0.39% (452: 4.5%; the runner: 1 WR in ~430 doors = 0.23%, interval 0.04-1.3%); the door share of attempts is 1.7%
(the runner's stream day: 2%), P(finish | door) with no pace resets 12.6% (the runner: 1 of 5). Ladder (seed 5, R = 1.6M): WR+1m 1 579 h,
WR+2m 946 h, WR+5m 338 h, WR+10m 166 h. **Read every number as the model's; the post-door hazards, the `slow` probabilities and
`win_cv` are guesses or weak fits**, and the whole result moves a lot with them (R = 3.2M, seed 5, paired seeds; each row changes one thing
from this plan):

| change | WR finishes | attempts per finish | EHT(WR), h |
| --- | --- | --- | --- |
| baseline (S .183, slow x1, win_cv .033) | 212 | 15094 | 3377 |
| win_cv 0 | 143 | 22378 | 4992 |
| slow_scale 0 | 529 | 6049 | 1398 |
| slow_scale 2 | 107 | 29907 | 6541 |
| post survival 0.1 | 112 | 28571 | 6391 |
| post survival 0.35 | 419 | 7637 | 1709 |
| post survival 1.0 | 1180 | 2712 | 607 |
| Brock pass 0.29 | 159 | 20126 | 3944 |
| Brock pass 0.53 | 267 | 11985 | 3065 |
| mid/late window hazards x0.5 | 455 | 7033 | 1656 |
| mid/late window hazards x1.5 | 87 | 36782 | 7870 |

```json
{
 "name": "480-fit-honest",
 "hunts": [
  {
   "after": "safari-walk",
   "site": "MAP_SAFARI_ZONE_EAST",
   "area": "land",
   "steps": 90,
   "detour_tiles": 0,
   "skip": [],
   "on_route": true
  },
  {
   "after": "safari-walk",
   "site": "MAP_SAFARI_ZONE_CENTER",
   "area": "land",
   "steps": 100,
   "detour_tiles": 0,
   "skip": [],
   "on_route": true
  },
  {
   "after": "safari-walk",
   "site": "MAP_SAFARI_ZONE_NORTH",
   "area": "land",
   "steps": 60,
   "detour_tiles": 0,
   "skip": [],
   "on_route": true
  },
  {
   "after": "powerplant-out",
   "site": "MAP_POWER_PLANT",
   "area": "land",
   "steps": 200,
   "detour_tiles": 0,
   "skip": [],
   "on_route": true
  },
  {
   "after": "berry-forest",
   "site": "MAP_THREE_ISLAND_BERRY_FOREST",
   "area": "land",
   "steps": 60,
   "detour_tiles": 0,
   "on_route": true,
   "stop_after": 3
  },
  {
   "after": "bond-bridge",
   "site": "MAP_THREE_ISLAND_BOND_BRIDGE",
   "area": "land",
   "steps": 40,
   "detour_tiles": 0,
   "on_route": true,
   "stop_after": 2
  },
  {
   "after": "fly-viridian",
   "site": "MAP_DIGLETTS_CAVE_B1F",
   "area": "land",
   "steps": 60,
   "detour_tiles": 50,
   "stop_after": 1
  },
  {
   "after": "route22",
   "site": "MAP_ROUTE22",
   "area": "land",
   "steps": 40,
   "detour_tiles": 0,
   "on_route": true,
   "stop_after": 2
  },
  {
   "after": "route23",
   "site": "MAP_ROUTE23",
   "area": "land",
   "steps": 40,
   "detour_tiles": 0,
   "on_route": true,
   "stop_after": 2
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
   "level": 25
  },
  {
   "after": "silph-giovanni",
   "kind": "gift",
   "species": [
    "SPECIES_LAPRAS"
   ],
   "frames": 700,
   "level": 25
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
   "after": "stones",
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
   "after": "stones",
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
   "after": "scope-and-dig",
   "kind": "buy",
   "species": [
    "SPECIES_ABRA"
   ],
   "cost": 2400,
   "frames": 2400,
   "level": 7
  },
  {
   "after": "koga-badge",
   "kind": "buy",
   "species": [
    "SPECIES_CLEFAIRY"
   ],
   "cost": 15000,
   "frames": 2400,
   "level": 12
  },
  {
   "after": "sabrina-badge",
   "kind": "gift",
   "species": [
    "SPECIES_HITMONLEE"
   ],
   "frames": 4000,
   "level": 25
  },
  {
   "after": "berry-forest",
   "kind": "static",
   "species": [
    "SPECIES_HYPNO"
   ],
   "frames": 0,
   "level": 30
  },
  {
   "after": "silph-giovanni",
   "kind": "gift",
   "species": [
    "SPECIES_KAKUNA",
    "SPECIES_BEEDRILL"
   ],
   "requires": [
    "SPECIES_WEEDLE"
   ],
   "frames": 2000
  },
  {
   "after": "silph-giovanni",
   "kind": "gift",
   "species": [
    "SPECIES_METAPOD",
    "SPECIES_BUTTERFREE"
   ],
   "requires": [
    "SPECIES_CATERPIE"
   ],
   "frames": 2000
  },
  {
   "after": "blaine-badge-and-sevii-scene",
   "kind": "gift",
   "species": [
    "SPECIES_OMANYTE"
   ],
   "frames": 1800,
   "level": 5
  },
  {
   "after": "route4",
   "kind": "buy",
   "species": [
    "SPECIES_MAGIKARP"
   ],
   "cost": 500,
   "frames": 900,
   "level": 5
  },
  {
   "after": "blaine-badge-and-sevii-scene",
   "kind": "gift",
   "species": [
    "SPECIES_AERODACTYL"
   ],
   "frames": 3600,
   "level": 30
  },
  {
   "after": "route5-6",
   "kind": "trade",
   "species": [
    "SPECIES_NIDORAN_M"
   ],
   "requires": [
    "SPECIES_NIDORAN_F"
   ],
   "frames": 1500
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
   "steps": 242,
   "detour_tiles": 20
  },
  {
   "after": "",
   "site": "MAP_ROUTE6",
   "area": "fish",
   "steps": 38,
   "detour_tiles": 60
  },
  {
   "after": "",
   "site": "MAP_FUCHSIA_CITY",
   "area": "fish",
   "steps": 36,
   "detour_tiles": 50
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
   "site": "MAP_SAFARI_ZONE_WEST",
   "area": "land",
   "steps": 95,
   "detour_tiles": 10
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
 "grind_pre_e4": 0,
 "grind_level": 42,
 "grind_yield": 90,
 "stones": [],
 "reset_aggr": 0.8,
 "weaken": true,
 "ball": "great",
 "fight_until_level": 15,
 "skip_groups": [
  "cerulean-gym-trainer",
  "celadon-gym-trainers",
  "viridian-gym-trainers",
  "blaine-quiz",
  "route3-trainers",
  "mt-moon-trainers",
  "fuchsia-gym-trainers",
  "mt-ember-trainers"
 ],
 "mistake_rate": 0.3,
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
  "koga-badge": 0.9,
  "sabrina-badge": 0.85,
  "blaine-badge-and-sevii-scene": 0.9,
  "giovanni-badge": 0.7,
  "oak-national-dex": 0.9,
  "brock": 0.8,
  "hall-of-fame-1": 0.75,
  "indigo-heal": 0.65
 },
 "type_strength": 1.0,
 "type_floor": 0.9,
 "audited_forced": true,
 "pass_min_catch": 45,
 "safari_model": true,
 "money_model": true,
 "ball_batch": 40,
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
  "route22",
  "tower-encounters"
 ],
 "calib_battle": true,
 "enc_path": true,
 "detour_map": true,
 "detour_bike": true,
 "battle_sim": true,
 "tms": [
  {
   "after": "misty-badge",
   "mv": "MOVE_WATER_PULSE",
   "frames": 600,
   "cost": 0
  },
  {
   "after": "route4",
   "mv": "MOVE_MEGA_KICK",
   "frames": 1500
  },
  {
   "after": "mansion",
   "mv": "MOVE_BLIZZARD",
   "frames": 1100
  },
  {
   "after": "mewtwo",
   "mv": "MOVE_FIRE_BLAST",
   "frames": 600,
   "cost": 0,
   "mewtwo": true
  },
  {
   "after": "mewtwo",
   "mv": "MOVE_EARTHQUAKE",
   "frames": 600,
   "cost": 0,
   "mewtwo": true
  }
 ],
 "keep_moves": [
  "MOVE_SURF",
  "MOVE_WATER_PULSE",
  "MOVE_BLIZZARD",
  "MOVE_BITE"
 ],
 "xbuys": [
  {
   "after": "celadon-tea",
   "attack": 4,
   "speed": 2,
   "frames": 1200,
   "special": 8
  },
  {
   "after": "fly-celadon-2",
   "special": 20,
   "speed": 6,
   "attack": 4,
   "revives": 0,
   "frames": 1800,
   "heals": 6
  }
 ],
 "fight_tactics": {
  "surge": {
   "x": []
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
   ]
  },
  "lorelei": {
   "x": [
    "special",
    "special",
    "special"
   ],
   "sacs": 0
  },
  "bruno": {
   "x": [
    "special",
    "special"
   ],
   "sacs": 0
  },
  "agatha": {
   "x": [
    "special",
    "special",
    "attack"
   ],
   "sacs": 0
  },
  "lance": {
   "x": [
    "special",
    "special"
   ],
   "sacs": 0
  },
  "champion": {
   "x": [
    "special"
   ],
   "sacs": 4
  },
  "rival-ss-anne": {
   "x": []
  },
  "rival-silph": {
   "x": [
    "special",
    "special",
    "speed"
   ]
  },
  "rival-route22": {
   "x": [
    "special",
    "special",
    "speed"
   ]
  },
  "giovanni": {
   "x": [
    "special",
    "speed"
   ]
  },
  "lorelei-2": {
   "x": [
    "special"
   ],
   "sacs": 0
  },
  "bruno-2": {
   "x": [
    "special"
   ],
   "sacs": 0
  },
  "agatha-2": {
   "x": [],
   "sacs": 0
  },
  "lance-2": {
   "x": [
    "special"
   ],
   "sacs": 0
  },
  "champion-2": {
   "x": [
    "special",
    "special"
   ],
   "sacs": 3
  },
  "rival-cerulean": {
   "x": []
  },
  "brock": {
   "entry": true
  },
  "misty": {
   "power": 1.0
  }
 },
 "early_dig": false,
 "erika_after_tower": true,
 "grinds": [],
 "elixirs": [
  {
   "after": "cycling-road",
   "kind": "max",
   "frames": 500
  },
  {
   "after": "silph-giovanni",
   "kind": "elixir",
   "frames": 400
  },
  {
   "after": "mansion",
   "kind": "elixir",
   "frames": 300
  },
  {
   "after": "route23",
   "kind": "max",
   "frames": 400
  },
  {
   "after": "cerulean-heal",
   "kind": "candy",
   "frames": 350
  },
  {
   "after": "route6-encounters",
   "kind": "candy",
   "frames": 350
  },
  {
   "after": "route9",
   "kind": "candy",
   "frames": 350
  },
  {
   "after": "super-rod",
   "kind": "candy",
   "frames": 300
  },
  {
   "after": "cycling-road",
   "kind": "candy",
   "frames": 300
  },
  {
   "after": "mansion",
   "kind": "candy",
   "frames": 300
  },
  {
   "after": "bill",
   "kind": "elixir",
   "frames": 500
  },
  {
   "after": "hideout-grunts",
   "kind": "moon",
   "frames": 400
  },
  {
   "after": "safari-walk",
   "kind": "leaf",
   "frames": 700
  },
  {
   "after": "powerplant-out",
   "kind": "thunder",
   "frames": 600
  },
  {
   "after": "mansion",
   "kind": "moon",
   "frames": 300
  },
  {
   "after": "deliver-meteorite",
   "kind": "moon",
   "frames": 60
  },
  {
   "after": "warden-strength",
   "kind": "candy",
   "frames": 350
  },
  {
   "after": "hideout-grunts",
   "kind": "candy",
   "frames": 400
  },
  {
   "after": "tower-channelers",
   "kind": "candy",
   "frames": 350
  },
  {
   "after": "silph-grunts",
   "kind": "candy",
   "frames": 400
  },
  {
   "after": "victory-road",
   "kind": "candy",
   "frames": 350
  },
  {
   "after": "mt-moon-1f",
   "kind": "candy",
   "frames": 400
  }
 ],
 "rope_cerulean_cave": true,
 "extra_heals": [
  "forest-trainers",
  "captain-hm01",
  "nugget-bridge-trainers",
  "hideout-giovanni",
  "fuji-flute",
  "koga-badge",
  "blaine-walk",
  "silph-walk",
  "sabrina-badge",
  "erika-badge",
  "giovanni-badge"
 ],
 "clemi_skip": true,
 "route_style": "wr",
 "calib_wr": true,
 "text_factor": 1.02,
 "path_overhead": 1.02,
 "no_dig": true,
 "candy_early": true,
 "carry_power": 1.9,
 "mewtwo_power": 1.5,
 "candy_early_n": 0,
 "starter_model": "manip",
 "manip": {
  "dials": 1,
  "natures": [
   "Rash",
   "Mild"
  ],
  "min_spa": 30,
  "min_spe": 20,
  "min_hp": 12,
  "quiet_window": 60,
  "quiet_keeps_wanted": true,
  "quiet_max_hits": 4
 },
 "league_heals": 8,
 "league_power": 2.2,
 "setup_min_turns": 2,
 "extra_fights": [
  {
   "name": "mt-moon-extra",
   "after": "mt-moon-1f",
   "scripts": [
    "MtMoon_1F_EventScript_Josh",
    "MtMoon_1F_EventScript_Miriam"
   ]
  }
 ],
 "poison_frames_per_hp": 70,
 "torrent_hits": 1.25,
 "torrent_heal_below": 0.3,
 "torrent_until": "hall-of-fame-1",
 "no_salvage_until": "rival-ss-anne",
 "evolutions": [
  {
   "after": "koga-badge",
   "species": "SPECIES_NIDORINA",
   "via": "candy"
  },
  {
   "after": "koga-badge",
   "species": "SPECIES_NIDORINO",
   "via": "candy"
  },
  {
   "after": "deliver-meteorite",
   "species": "SPECIES_NIDOQUEEN",
   "via": "stone",
   "item": "MOON"
  },
  {
   "after": "deliver-meteorite",
   "species": "SPECIES_NIDOKING",
   "via": "stone",
   "item": "MOON"
  },
  {
   "after": "deliver-meteorite",
   "species": "SPECIES_CLEFABLE",
   "via": "stone",
   "item": "MOON"
  },
  {
   "after": "stones",
   "species": "SPECIES_RAICHU",
   "via": "stone",
   "item": "THUNDER"
  },
  {
   "after": "stones",
   "species": "SPECIES_EXEGGUTOR",
   "via": "stone",
   "item": "LEAF"
  },
  {
   "after": "stones",
   "species": "SPECIES_VICTREEBEL",
   "via": "stone",
   "item": "LEAF",
   "buy": true
  },
  {
   "after": "stones",
   "species": "SPECIES_NINETALES",
   "via": "stone",
   "item": "FIRE",
   "buy": true
  },
  {
   "after": "stones",
   "species": "SPECIES_STARMIE",
   "via": "stone",
   "item": "WATER",
   "buy": true
  },
  {
   "after": "stones",
   "species": "SPECIES_PIDGEOT",
   "via": "level"
  },
  {
   "after": "koga-badge",
   "species": "SPECIES_DODRIO",
   "via": "level"
  },
  {
   "after": "stones",
   "species": "SPECIES_RATICATE",
   "via": "level"
  },
  {
   "after": "stones",
   "species": "SPECIES_MAGNETON",
   "via": "level"
  },
  {
   "after": "stones",
   "species": "SPECIES_WEEZING",
   "via": "level"
  },
  {
   "after": "stones",
   "species": "SPECIES_PERSIAN",
   "via": "level"
  },
  {
   "after": "erika-badge",
   "species": "SPECIES_NINETALES",
   "via": "stone",
   "item": "FIRE",
   "buy": true
  },
  {
   "after": "erika-badge",
   "species": "SPECIES_VICTREEBEL",
   "via": "stone",
   "item": "LEAF",
   "buy": true
  },
  {
   "after": "victory-road",
   "species": "SPECIES_MACHOKE",
   "via": "level"
  },
  {
   "after": "stones",
   "species": "SPECIES_VENOMOTH",
   "via": "level"
  },
  {
   "after": "victory-road",
   "species": "SPECIES_DUGTRIO",
   "via": "level"
  },
  {
   "after": "koga-badge",
   "species": "SPECIES_WIGGLYTUFF",
   "via": "stone",
   "item": "MOON",
   "buy": false
  }
 ],
 "level_gap": 8,
 "pass_off": [
  "route1-encounters",
  "route2-encounters"
 ],
 "pc_frames": 1500,
 "safari_stop": 5,
 "league_transit": true,
 "postgame_calib": true,
 "trainer_fit": 0.81,
 "item_swaps": true,
 "menu_slip": 0.01,
 "hazard": {},
 "fatal": {
  "forest-trainers": 0.1807,
  "pewter-gym-trainer": 0.0948,
  "brock": 0.3923,
  "route3-trainers": 0.0207,
  "mt-moon-miguel": 0.0207,
  "misty": 0.0409,
  "rival-cerulean": 0.0409,
  "nugget-bridge-trainers": 0.0201,
  "rival-ss-anne": 0.0592,
  "rock-tunnel-trainers": 0.0201,
  "hideout-giovanni": 0.0214,
  "rival-tower": 0.0424,
  "surge": 0.0214,
  "koga": 0.1272,
  "fuchsia-gym-trainers": 0.0334,
  "mansion-trainers": 0.0717,
  "blaine": 0.1999,
  "three-island-paxton": 0.0717,
  "rival-silph": 0.07,
  "sabrina": 0.1031,
  "erika": 0.1031,
  "giovanni": 0.07,
  "rival-route22": 0.1996,
  "victory-road": 0.0715,
  "lorelei": 0.09781,
  "bruno": 0.09781,
  "agatha": 0.09781,
  "lance": 0.18605,
  "champion": 0.18605,
  "icefall-grunt": 0.05016,
  "warehouse-admin-1": 0.05016,
  "warehouse-admin-2": 0.05016,
  "warehouse-gideon": 0.09781,
  "lorelei-2": 0.09781,
  "bruno-2": 0.09781,
  "agatha-2": 0.09781,
  "lance-2": 0.18605,
  "champion-2": 0.18605
 },
 "slow": [
  {
   "seg": "safari-walk",
   "p": 0.2,
   "mean_s": 120
  },
  {
   "seg": "hunt-power_plant-land@powerplant-out",
   "p": 0.3,
   "mean_s": 90
  },
  {
   "seg": "cerulean-cave",
   "p": 1.0,
   "mean_s": 40
  },
  {
   "seg": "route8-trainers",
   "p": 0.15,
   "mean_s": 40
  },
  {
   "seg": "misty",
   "p": 0.06,
   "mean_s": 100
  },
  {
   "seg": "rival-cerulean",
   "p": 0.06,
   "mean_s": 100
  }
 ],
 "win_cv": 0.033
}
```

## Score

- rung **WR**, EHT 3376.7 h (212 finishes in 3 200 000 attempts, seed 5); baseline of the fitted model, nothing to compare with.
- evaluation lines: `human-run/scores.jsonl` (plan `480-fit-honest`, MODEL CHANGE).
