# Plan 451-fidelity-honest

Parent: `450-dex-on-386`. MODEL CHANGE (fidelity audit, `human-run/journal/2026-11-14-18-00-fidelity-audit.md`):
the plan's route and dex parameters are 450's, unchanged. What changed is the model the plan is scored on:

* `trainer_fit 0.81`: non-League trainer fights run 0.81x the fitted intro and per-mon overhead (0.71 reproduces the
  WR run's own splits within 5%; 0.81 is a typical WR-level run: Start E4R1 at 2:40:15, the runner's own typical entry),
* `league_transit`, `postgame_calib`: the E4 room-to-room time and the Champ 1 -> Warehouse chain the model lacked
  (the post-E4R1 half was 9.4 min faster than the WR), calibrated on the WR splits,
* `item_swaps` (Persim, Black Glasses, potion menus), `menu_slip 0.01` (wrong move row), `hazard` (good-day early
  deaths: Brock split 75%, `STRATEGY.md` 4 and 7),
* `regime_bad`: the bad-day patch read by `human-run/tools/regime.py` (time-weighted mixture with this plan as the good day).

```json
{
 "name": "451-fidelity-honest",
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
   "after": "route3",
   "site": "MAP_ROUTE3",
   "area": "land",
   "steps": 40,
   "detour_tiles": 0,
   "on_route": true,
   "stop_after": 2
  },
  {
   "after": "route4",
   "site": "MAP_ROUTE4",
   "area": "land",
   "steps": 40,
   "detour_tiles": 0,
   "on_route": true,
   "stop_after": 2
  },
  {
   "after": "route9",
   "site": "MAP_ROUTE9",
   "area": "land",
   "steps": 40,
   "detour_tiles": 0,
   "on_route": true,
   "stop_after": 2
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
 "reset_aggr": 0.94,
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
   "power": 0.9
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
 "hazard": {
  "forest-trainers": 0.035,
  "pewter-gym-trainer": 0.017,
  "rival-lab": 0.017,
  "brock": 0.086
 },
 "regime_bad": {
  "manip_p_hit": 0.3,
  "mistake_rate": 1.0,
  "jitter_n": 6,
  "react_hi": 45,
  "hazard": {
   "forest-trainers": 0.2,
   "pewter-gym-trainer": 0.12,
   "rival-lab": 0.1,
   "brock": 0.6
  },
  "weight": 0.5
 }
}
```
