# Plan 1058-hidden-candy-calcium-nugget

Parent `1057-calm-sp9-ppup-redo`. Three structural resources the sim could not express before, all found by auditing every item
the Kanto maps hold against what the plan picks up (journal `2026-12-04-10-00-structure-items-dims`), plus a refit of the six lane
windows on the new plan (`tools/fit_lane.py`, R 400k):

1. **Two hidden Rare Candies** (`elixirs` kind `candy`): Route 9 (63,2) (`decompiled/data/maps/Route9/map.json`, hidden item,
   `underfoot` false: press A facing it, no Itemfinder) and Route 17 (18,83) (`Route17/map.json`; the tile is a Cycling Road pull-down
   tile `MB_CYCLING_ROAD_PULL_DOWN`, `bike.c:56-72`: hold B to stop on it; the plan already takes the Route 17 PP Up and Max Elixir
   there). Frames 300 / 450, guesses (the Route 9 tile is 7 tiles off the road, `bin/frlg-mapgraph`). All Rare Candies not eaten on the spot go
   into the carry at the Indigo heal; one is eaten early (`candy_early_n` 1, evidence guard unchanged).
2. **Two Calciums** (new `elixirs` kinds `calcium` etc.: +10 SpA EVs each, refused at 100 EVs, stats recalculated on use):
   Rocket Hideout B4F item ball (6,23) (`RocketHideout_B4F/map.json`, `RocketHideout_B4F_EventScript_ItemCalcium`) and Silph Co. 7F item
   ball (0,12) (`SilphCo_7F/map.json`, the west corridor of the rival's floor; distance not path-checked). Frames 450 each, guesses.
3. **Selling the Route 24 Nugget** (new `specials` kind `sell`, `cost` = money received): the Nugget is handed over by the Route 24
   Rocket before the fight (`Route24/scripts.inc:5-51`, every run that reaches him has it), sells for half its 10000 price
   (`item_menu.c:1933`: `ItemId_GetPrice / 2`), 5000 at the Celadon Dept. Store visit that already buys the first X items. 350 frames.

Not in the plan (tested, refuted or saturated): Route 6 / Mansion 3F candies, a third / fourth Calcium, HP Up / Carbos / Zinc, the
Tower Nugget, TM sales, Mystic Water from the Game Corner (+19.7 .. +26 h), Rare Candy kept for Mewtwo (+24 h per candy).

```json
{
 "name": "1058-hidden-candy-calcium-nugget",
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
   "site": "MAP_SAFARI_ZONE_WEST",
   "area": "land",
   "steps": 100,
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
   "steps": 240,
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
   "steps": 80,
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
   "after": "celadon-tea",
   "kind": "sell",
   "species": [],
   "cost": 5000,
   "frames": 350
  }
 ],
 "fill": [
  {
   "after": "",
   "site": "MAP_SEAFOAM_ISLANDS_B3F",
   "area": "land",
   "steps": 120,
   "detour_tiles": 50
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
   "site": "MAP_POWER_PLANT",
   "area": "land",
   "steps": 242,
   "detour_tiles": 20
  },
  {
   "after": "",
   "site": "MAP_ROUTE18",
   "area": "land",
   "steps": 80,
   "detour_tiles": 50
  },
  {
   "after": "",
   "site": "MAP_SEAFOAM_ISLANDS_B4F",
   "area": "land",
   "steps": 55,
   "detour_tiles": 30
  },
  {
   "after": "",
   "site": "MAP_VICTORY_ROAD_2F",
   "area": "land",
   "steps": 92,
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
   "steps": 116,
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
 "fight_until_level": 18,
 "skip_groups": [
  "cerulean-gym-trainer",
  "celadon-gym-trainers",
  "viridian-gym-trainers",
  "blaine-quiz",
  "route3-trainers",
  "mt-moon-trainers",
  "fuchsia-gym-trainers",
  "mt-ember-trainers",
  "surge-gym-trainers",
  "mansion-trainers",
  "silph-grunts"
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
  "misty-badge": 0.9,
  "surge-badge": 0.9,
  "erika-badge": 0.7,
  "koga-badge": 0.85,
  "sabrina-badge": 0.65,
  "blaine-badge-and-sevii-scene": 0.7,
  "giovanni-badge": 0.7,
  "oak-national-dex": 0.9,
  "brock": 1.0,
  "hall-of-fame-1": 0.75,
  "indigo-heal": 0.8
 },
 "type_strength": 1.0,
 "type_floor": 0.9,
 "audited_forced": true,
 "pass_min_catch": 45,
 "safari_model": true,
 "money_model": true,
 "ball_batch": 20,
 "calib_enc": true,
 "rod": "none",
 "repel_on": [
  "mt-moon-1f",
  "cerulean-cave",
  "route3",
  "nugget-bridge",
  "cycling-road",
  "route6-encounters",
  "rock-tunnel",
  "route9"
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
  },
  {
   "after": "fly-celadon-2",
   "mv": "MOVE_ICE_BEAM",
   "frames": 2000,
   "cost": 80000,
   "mewtwo": false
  },
  {
   "after": "mewtwo",
   "mv": "MOVE_CALM_MIND",
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
   "attack": 0,
   "speed": 2,
   "frames": 1200,
   "special": 8,
   "heals": 0
  },
  {
   "after": "fly-celadon-2",
   "special": 9,
   "speed": 1,
   "attack": 0,
   "revives": 0,
   "frames": 1800,
   "heals": 4,
   "accuracy": 2,
   "guard": 2
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
   ],
   "free_torrent": true,
   "power": 0.667
  },
  "sabrina": {
   "x": [
    "special",
    "special",
    "speed"
   ],
   "free_torrent": true,
   "power": 0.667
  },
  "lorelei": {
   "x": [
    "special",
    "special",
    "special",
    "special"
   ],
   "sacs": 0,
   "heal_below": 0.0
  },
  "bruno": {
   "x": [
    "special",
    "special",
    "speed"
   ],
   "sacs": 0,
   "free_torrent": true
  },
  "agatha": {
   "x": [
    "special",
    "speed"
   ],
   "sacs": 0,
   "free_torrent": true
  },
  "lance": {
   "x": [
    "special",
    "accuracy",
    "special",
    "special"
   ],
   "sacs": 0
  },
  "champion": {
   "x": [
    "guard",
    "speed",
    "special"
   ],
   "sacs": 0
  },
  "rival-ss-anne": {
   "x": [],
   "risk_power": 2.0,
   "sacs": 0,
   "heal_below": 0.5
  },
  "rival-silph": {
   "x": [
    "special",
    "special",
    "speed",
    "special"
   ]
  },
  "rival-route22": {
   "x": [
    "special",
    "special",
    "speed",
    "special"
   ],
   "sacs": 0,
   "heal_below": 0.55
  },
  "giovanni": {
   "x": [
    "special",
    "speed"
   ],
   "potions": 1,
   "free_torrent": true,
   "power": 0.667
  },
  "lorelei-2": {
   "x": [
    "calm",
    "calm"
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
   "x": [
    "calm"
   ],
   "sacs": 0
  },
  "lance-2": {
   "x": [
    "calm",
    "calm"
   ],
   "sacs": 0
  },
  "champion-2": {
   "x": [
    "calm",
    "calm"
   ],
   "sacs": 0
  },
  "rival-cerulean": {
   "x": [],
   "risk_power": 3.5,
   "heal_below": 0.0,
   "potions": 1
  },
  "brock": {
   "entry": false,
   "potions": 0,
   "sacs": 0
  },
  "misty": {
   "power": 1.0,
   "risk_power": 2.0,
   "sacs": 0,
   "potions": 5
  },
  "rival-tower": {
   "x": [],
   "sacs": 0
  },
  "pewter-gym-trainer": {
   "heal_below": 0.5,
   "potions": 5
  },
  "nugget-bridge-trainers": {
   "sacs": 0
  },
  "mt-moon-miguel": {
   "potions": 1
  },
  "blaine": {
   "x": [
    "special"
   ]
  },
  "forest-trainers": {
   "potions": 3
  },
  "route3-trainers": {
   "potions": 3
  }
 },
 "early_dig": false,
 "erika_after_tower": true,
 "grinds": [],
 "elixirs": [
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
  },
  {
   "after": "cycling-road",
   "kind": "max",
   "frames": 500
  },
  {
   "after": "route23",
   "kind": "max",
   "frames": 400
  },
  {
   "after": "hideout-grunts",
   "kind": "ppup",
   "frames": 450
  },
  {
   "after": "cycling-road",
   "kind": "ppup",
   "frames": 450
  },
  {
   "after": "celadon-tea",
   "kind": "ppup",
   "frames": 450
  },
  {
   "after": "route9",
   "kind": "candy",
   "frames": 300
  },
  {
   "after": "cycling-road",
   "kind": "candy",
   "frames": 450
  },
  {
   "after": "hideout-giovanni",
   "kind": "calcium",
   "frames": 450
  },
  {
   "after": "rival-silph",
   "kind": "calcium",
   "frames": 450
  }
 ],
 "rope_cerulean_cave": true,
 "extra_heals": [
  "forest-trainers",
  "nugget-bridge-trainers",
  "hideout-giovanni",
  "fuji-flute",
  "koga-badge",
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
 "carry_power": 2.0,
 "mewtwo_power": 1.2,
 "candy_early_n": 1,
 "starter_model": "manip",
 "manip": {
  "dials": 1,
  "natures": [
   "Rash",
   "Mild"
  ],
  "min_spa": 29,
  "min_spe": 25,
  "min_hp": 15,
  "quiet_window": 60,
  "quiet_keeps_wanted": true,
  "quiet_max_hits": 6,
  "p_hit": 0.64,
  "s_reset": 128,
  "min_spd": 15
 },
 "league_heals": 14,
 "league_power": 1.4,
 "setup_min_turns": 2,
 "extra_fights": [
  {
   "name": "mt-moon-extra",
   "after": "mt-moon-1f",
   "scripts": [
    "MtMoon_1F_EventScript_Miriam"
   ]
  }
 ],
 "poison_frames_per_hp": 70,
 "torrent_hits": 1.5,
 "torrent_heal_below": 0.5,
 "torrent_until": "hall-of-fame-1",
 "no_salvage_until": "rival-ss-anne",
 "evolutions": [
  {
   "after": "koga-badge",
   "species": "SPECIES_NIDORINA",
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
   "item": "MOON",
   "buy": true
  },
  {
   "after": "stones",
   "species": "SPECIES_RAICHU",
   "via": "stone",
   "item": "THUNDER"
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
  },
  {
   "after": "victory-road",
   "species": "SPECIES_EXEGGUTOR",
   "via": "stone",
   "item": "LEAF",
   "buy": true
  },
  {
   "after": "koga-badge",
   "species": "SPECIES_SANDSLASH",
   "via": "level"
  },
  {
   "after": "erika-badge",
   "species": "SPECIES_PIDGEOTTO",
   "via": "candy"
  },
  {
   "after": "stones",
   "species": "SPECIES_WEEPINBELL",
   "via": "candy"
  },
  {
   "after": "erika-badge",
   "species": "SPECIES_PIDGEOT",
   "via": "level"
  },
  {
   "after": "koga-badge",
   "species": "SPECIES_NIDORINO",
   "via": "level"
  },
  {
   "after": "stones",
   "species": "SPECIES_PIDGEOTTO",
   "via": "level"
  },
  {
   "after": "victory-road",
   "species": "SPECIES_FEAROW",
   "via": "level"
  },
  {
   "after": "victory-road",
   "species": "SPECIES_SANDSLASH",
   "via": "level"
  },
  {
   "after": "victory-road",
   "species": "SPECIES_NINETALES",
   "via": "stone",
   "item": "FIRE",
   "buy": true
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
  "lorelei": 0.0,
  "bruno": 0.18659,
  "agatha": 0.0,
  "lance": 0.26868,
  "champion": 0.0,
  "icefall-grunt": 0.09806,
  "warehouse-admin-1": 0.10588,
  "warehouse-admin-2": 0.11489,
  "warehouse-gideon": 0.14908,
  "lorelei-2": 0.05457,
  "bruno-2": 0.05406,
  "agatha-2": 0.05354,
  "lance-2": 0.09588,
  "champion-2": 0.0
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
 "win_cv": 0.033,
 "rope_fills": true,
 "ultra_below": 0,
 "league_preheal": 0.85,
 "heal_below": 0.6,
 "poke_from": 190,
 "mewtwo_hp_bar": 384,
 "mewtwo_levels": true,
 "transit_outro": 240,
 "postgame_events": true,
 "ruby_text": 420,
 "resort_text": 5280,
 "route_variants": [
  "warden_early"
 ],
 "audit2": 1,
 "ev_cap": true,
 "ev_lag": true,
 "mewtwo_evs": true,
 "black_glasses": true,
 "risk_until": "victory-road",
 "risk_foe": 1.0,
 "risk_potions": 6,
 "risk_pay": true,
 "early_helpers": true,
 "risk_adopt": true,
 "risk_abandon": true,
 "risk_windows": [
  {
   "until": "rock-tunnel-trainers",
   "power": 1.111
  },
  {
   "until": "snorlax",
   "power": 2.565
  },
  {
   "until": "koga-badge",
   "power": 1.35
  },
  {
   "until": "blaine-badge-and-sevii-scene",
   "power": 0.773
  },
  {
   "until": "giovanni-badge",
   "power": 1.572
  },
  {
   "until": "indigo-heal",
   "power": 2.314
  }
 ],
 "torrent_stall": true,
 "sac_honest": true,
 "sac_honest_league": true,
 "helper_hp": 24,
 "reset_table": {
  "WR+2m": {
   "brock": 1057,
   "misty-badge": 1915,
   "bill-cottage": 2586,
   "bike-redeem": 2926,
   "scope-and-dig": 4103,
   "surge-badge": 4842,
   "super-rod": 4970,
   "koga-badge": 5726,
   "blaine-badge": 6504,
   "blaine-badge-and-sevii-scene": 7464,
   "sabrina-badge": 8402,
   "erika-badge": 8614,
   "giovanni-badge": 8873,
   "indigo-heal": 9445,
   "hall-of-fame-1": 10327,
   "celio-network-machine": 11533
  },
  "WR": {
   "brock": 1057,
   "misty-badge": 1915,
   "bill-cottage": 2586,
   "bike-redeem": 2926,
   "scope-and-dig": 4103,
   "surge-badge": 4842,
   "super-rod": 4970,
   "koga-badge": 5580,
   "blaine-badge": 6343,
   "blaine-badge-and-sevii-scene": 7279,
   "sabrina-badge": 8202,
   "erika-badge": 8452,
   "giovanni-badge": 8729,
   "indigo-heal": 9343,
   "hall-of-fame-1": 10212,
   "celio-network-machine": 11404
  }
 },
 "mewtwo_keep": [
  "MOVE_PSYCHIC",
  "MOVE_FIRE_BLAST",
  "MOVE_EARTHQUAKE",
  "MOVE_CALM_MIND"
 ],
 "mewtwo_pp": true,
 "mewtwo_pp_floor": 5,
 "league_redo": 1.0,
 "salvage_at": {
  "lorelei": 0.8,
  "bruno": 0.8,
  "agatha": 0.8,
  "lance": 0.8,
  "champion": 0.8,
  "lorelei-2": 0.8,
  "bruno-2": 0.8,
  "agatha-2": 0.8,
  "lance-2": 0.8,
  "champion-2": 0.8
 }
}
```

## Score

Paired against the parent `1057-calm-sp9-ppup-redo` (its own windows), R 3.2M, fresh seeds 5401 / 5402 (`frlg-human compare`, + is worse):
WR+2m 159.8 -> 117.7 h (-42.1 [-46.2, -38.6]) and 157.9 -> 118.7 h (-39.2 [-43.2, -35.8]); WR 222.6 -> 166.0 h (-56.6 [-63.1, -50.7])
and 221.4 -> 166.8 h (-54.6 [-61.5, -49.1]). The same three changes with the parent's windows (not refit): WR+2m -46.3 / -47.7, WR
-59.4 / -60.2 (seeds 5301 / 5302). Absolute EHT of the parent on these seeds is 155-162 h (WR+2m) and 211-223 h (WR), as logged for 1057.
Every stand-in corner (carry_power 1.7 / 2.3, league_power 1.0 / 1.8 / 2.2, mewtwo_power 1.0 / 1.5, trainer_fit .76 / .86, mistake
.16 / .4, level_gap 0 / 12; R 800k, run on the unrefit variant) favours it (-9 .. -235 h); the gain breaks even at about 4.2x the guessed
pickup frames. Lane windows of this plan against the runner's (`fit_lane.py --check`): start -> door survival .075 (1057: .060; runner
.056), every window inside the runner's 95% CI. `reset_table` not refit. Caveat: no stream shows the runner taking a hidden candy, a
Calcium, or selling the Nugget; the frames are guesses and the three pickups are in the model through the League (candy / Calcium raise
the carry's level and SpA, the gain is at Sabrina, rival-silph and the League).
