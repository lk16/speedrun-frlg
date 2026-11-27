# Plan 1842-r2-notackle

Parent: `1841-r3-repel`. Change (controller-only, R2 2026-11-27): `early_torrent.tackle_only` emptied. Played with the fixed move pick (commit 72063c9f), Camper Liam with Tackle only is a dead end: a Lv8 Squirtle's Tackle does 1-3 to his Lv10 Geodude after Defense Curl (Geodude is Rock/Ground, Normal is not very effective) while Geodude's Tackle does 7-13; 12 of 12 branches blacked out from one `pewter-gym` checkpoint (seed 21, scratch run `ppdbg5`), on top of the 20 r2a-dev21 soft-locks. The simulator never modelled the restriction (`params.rs` `EarlyTorrent::tackle_only`), so the score is exactly 1841's. Liam is fought with the policy's moves (Bubble).

```json
{
 "name": "1842-r2-notackle",
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
   "stop_after": 1
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
   "site": "MAP_ROUTE18",
   "area": "land",
   "steps": 80,
   "detour_tiles": 50
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
   "site": "MAP_SEAFOAM_ISLANDS_B4F",
   "area": "land",
   "steps": 55,
   "detour_tiles": 30
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
   "site": "MAP_POKEMON_TOWER_7F",
   "area": "land",
   "steps": 152,
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
 "fight_until_level": 17,
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
  "misty-badge": 0.7,
  "surge-badge": 0.7,
  "erika-badge": 0.5,
  "koga-badge": 0.65,
  "sabrina-badge": 0.45,
  "blaine-badge-and-sevii-scene": 0.5,
  "giovanni-badge": 0.5,
  "oak-national-dex": 0.7,
  "brock": 0.8,
  "hall-of-fame-1": 0.55,
  "indigo-heal": 0.6
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
  "nugget-bridge",
  "cycling-road",
  "route6-encounters",
  "rock-tunnel",
  "route9",
  "mt-moon-b1f",
  "mt-moon-b2f"
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
    "special",
    "special"
   ],
   "free_torrent": true,
   "power": 0.667,
   "heal_below": 0.55
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
    "calm"
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
   "free_torrent": true,
   "heal_below": 0.65
  },
  "agatha": {
   "x": [
    "special"
   ],
   "sacs": 0,
   "free_torrent": true
  },
  "lance": {
   "x": [
    "calm",
    "accuracy",
    "special",
    "special",
    "accuracy"
   ],
   "sacs": 0
  },
  "champion": {
   "x": [
    "guard",
    "speed",
    "special",
    "special"
   ],
   "sacs": 0,
   "heal_below": 0.35
  },
  "rival-ss-anne": {
   "x": [],
   "risk_power": 1.0,
   "sacs": 0,
   "heal_below": 0.35,
   "potions": 9
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
    "calm",
    "calm"
   ],
   "sacs": 0
  },
  "lance-2": {
   "x": [
    "calm",
    "calm",
    "calm"
   ],
   "sacs": 0,
   "heal_below": 0.0
  },
  "champion-2": {
   "x": [
    "calm",
    "calm"
   ],
   "sacs": 0,
   "heal_below": 0.5
  },
  "rival-cerulean": {
   "x": [],
   "risk_power": 1.0,
   "heal_below": 0.35,
   "potions": 3
  },
  "brock": {
   "entry": false,
   "potions": 0,
   "sacs": 0
  },
  "misty": {
   "power": 1.0,
   "risk_power": 1.0,
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
   "potions": 5
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
   "after": "cerulean-heal",
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
   "after": "hideout-grunts",
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
   "after": "cerulean-heal",
   "kind": "candy",
   "frames": 350
  },
  {
   "after": "hideout-grunts",
   "kind": "candy",
   "frames": 350
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
 "mewtwo_power": 1.15,
 "candy_early_n": 5,
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
 "league_power": 1.2,
 "setup_min_turns": 2,
 "extra_fights": [
  {
   "name": "mt-moon-extra",
   "after": "mt-moon-1f",
   "scripts": [
    "MtMoon_1F_EventScript_Robby"
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
  },
  {
   "after": "stones",
   "species": "SPECIES_SLOWBRO",
   "via": "level"
  },
  {
   "after": "erika-badge",
   "buy": true,
   "item": "THUNDER",
   "species": "SPECIES_JOLTEON",
   "via": "stone"
  },
  {
   "after": "koga-badge",
   "species": "SPECIES_PARASECT",
   "via": "level"
  }
 ],
 "level_gap": 8,
 "pass_off": [
  "route1-encounters",
  "route2-encounters",
  "mt-moon-b1f",
  "mt-moon-b2f"
 ],
 "pc_frames": 1500,
 "safari_stop": 5,
 "league_transit": true,
 "postgame_calib": true,
 "item_swaps": true,
 "menu_slip": 0.01,
 "hazard": {},
 "fatal": {
  "lorelei": 0.0,
  "bruno": 0.17213,
  "agatha": 0.0,
  "lance": 0.09658,
  "champion": 0.0,
  "icefall-grunt": 0.08702,
  "warehouse-admin-1": 0.09498,
  "warehouse-admin-2": 0.10443,
  "warehouse-gideon": 0.12351,
  "lorelei-2": 0.01964,
  "bruno-2": 0.01946,
  "agatha-2": 0.01927,
  "lance-2": 0.03263,
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
 "league_preheal": 0.5,
 "heal_below": 0.6,
 "poke_from": 0,
 "mewtwo_hp_bar": 384,
 "mewtwo_levels": true,
 "transit_outro": 240,
 "postgame_events": true,
 "ruby_text": 420,
 "resort_text": 5280,
 "route_variants": [
  "warden_early",
  "dig_after_bill"
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
   "power": 1.035
  },
  {
   "until": "koga-badge",
   "power": 0.99
  },
  {
   "until": "blaine-badge-and-sevii-scene",
   "power": 0.661
  },
  {
   "until": "giovanni-badge",
   "power": 1.111
  },
  {
   "until": "indigo-heal",
   "power": 2.015
  }
 ],
 "torrent_stall": true,
 "sac_honest": true,
 "sac_honest_league": true,
 "helper_hp": 24,
 "reset_table": {
  "WR+2m": {
   "brock": 1080,
   "misty-badge": 1831,
   "bill-cottage": 2450,
   "bike-redeem": 2924,
   "scope-and-dig": 4089,
   "surge-badge": 4941,
   "snorlax": 5053,
   "koga-badge": 5651,
   "blaine-badge": 6420,
   "blaine-badge-and-sevii-scene": 7280,
   "sabrina-badge": 8164,
   "erika-badge": 8350,
   "giovanni-badge": 8754,
   "indigo-heal": 9450,
   "hall-of-fame-1": 10340,
   "celio-network-machine": 11487
  },
  "WR": {
   "brock": 1053,
   "misty-badge": 1804,
   "bill-cottage": 2444,
   "bike-redeem": 2786,
   "scope-and-dig": 3987,
   "surge-badge": 4737,
   "snorlax": 4841,
   "koga-badge": 5431,
   "blaine-badge": 6207,
   "blaine-badge-and-sevii-scene": 7140,
   "sabrina-badge": 8072,
   "erika-badge": 8315,
   "giovanni-badge": 8605,
   "indigo-heal": 9245,
   "hall-of-fame-1": 10172,
   "celio-network-machine": 11368
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
  "lorelei": 0.9,
  "bruno": 0.9,
  "agatha": 0.9,
  "lance": 0.9,
  "champion": 0.9,
  "lorelei-2": 0.9,
  "bruno-2": 0.9,
  "agatha-2": 0.9,
  "lance-2": 0.9,
  "champion-2": 0.9
 },
 "scarce_factor": 1.0,
 "reset_adj": {
  "WR+2m": {},
  "WR": {}
 },
 "sleep_no_potion": true,
 "potion_tiers": [
  {
   "after": "persim-and-potions",
   "hp": 50
  },
  {
   "after": "surf-cinnabar",
   "hp": 200
  }
 ],
 "ball_shop_from": "persim-and-potions",
 "early_balls": 0,
 "awakenings": 3,
 "fly_badge_gate": true,
 "coin_case_frames": 900,
 "trash_cans": true,
 "marowak_fight": true,
 "persim_held": true,
 "fight_lists": {
  "cerulean-gym-trainer": [],
  "route25-trainers": [
   "TRAINER_HIKER_WAYNE",
   "TRAINER_PICNICKER_KELSEY",
   "TRAINER_CAMPER_FLINT",
   "TRAINER_LASS_HALEY"
  ],
  "route6-trainers": [
   "TRAINER_CAMPER_JEFF"
  ],
  "rock-tunnel-trainers": [
   "TRAINER_PICNICKER_ALICIA",
   "TRAINER_BUG_CATCHER_CONNER",
   "TRAINER_POKEMANIAC_ASHTON",
   "TRAINER_POKEMANIAC_WINSTON",
   "TRAINER_PICNICKER_MARTHA",
   "TRAINER_HIKER_ALLEN",
   "TRAINER_HIKER_LENNY",
   "TRAINER_PICNICKER_SOFIA",
   "TRAINER_HIKER_DUDLEY",
   "TRAINER_PICNICKER_DANA"
  ],
  "route8-trainers": [
   "TRAINER_GAMER_RICH"
  ],
  "hideout-grunts": [
   "TRAINER_TEAM_ROCKET_GRUNT_7",
   "TRAINER_TEAM_ROCKET_GRUNT_18",
   "TRAINER_TEAM_ROCKET_GRUNT_16",
   "TRAINER_TEAM_ROCKET_GRUNT_17"
  ],
  "route3-trainers": [
   "TRAINER_BUG_CATCHER_COLTON",
   "TRAINER_BUG_CATCHER_GREG",
   "TRAINER_BUG_CATCHER_JAMES"
  ],
  "bond-bridge-nikki": [],
  "celadon-gym-trainers": [
   "TRAINER_BEAUTY_TAMIA",
   "TRAINER_BEAUTY_LORI"
  ]
 },
 "viridian_warren": true,
 "trainer_cost": [
  749,
  503,
  235
 ],
 "strength_carrier": true,
 "stone_shops": [
  "celadon-tea",
  "fly-celadon-2",
  "fly-celadon-3"
 ],
 "marts": [
  {
   "after": "walk-lavender",
   "great": 15,
   "topup": true
  },
  {
   "after": "surf-route21-s",
   "great": 20,
   "topup": true,
   "potions": 7
  },
  {
   "after": "sabrina-badge",
   "great": 25,
   "topup": true,
   "potions": 3
  },
  {
   "after": "silph-walk",
   "topup": true,
   "potions": 4
  }
 ],
 "potion_bag_from": "koga-badge",
 "tcost_whole": true,
 "turn_power": 1.4,
 "turn_power_boss": 1.0,
 "mewtwo_candies": 0,
 "vod_frames": {
  "misty-walk": 1075,
  "misty-badge": 345,
  "captain-hm01": 3350,
  "teleport-cerulean-2": 3225,
  "koga-badge": 880,
  "blaine-quiz-text": 4240,
  "blaine-badge": 1285,
  "sevii-bill-boat": 1925,
  "celio-tri-pass": 2475,
  "lostelle-hypno": 450,
  "blaine-badge-and-sevii-scene": 1980,
  "erika-badge": 405,
  "viridian-gym-walk": 1285,
  "giovanni-badge": 585,
  "mt-moon-1f": 1405,
  "mt-moon-b1f": 1000,
  "mt-moon-b2f": 2280,
  "route4": 1430
 },
 "evo_stone_frames": 1600,
 "evo_candy_frames": 1620,
 "level_evo_honest": true,
 "risk_entry_paid": true,
 "league_heal_price": 3000,
 "level_evo_max_candies": 1,
 "full_heals": 2,
 "full_heal_at": "walk-lavender",
 "full_heal_frames": 150.0,
 "full_heal_price": 100,
 "pass_reserve": 8,
 "pass_reserve_rate": 100,
 "safari_walk_steps": {
  "MAP_SAFARI_ZONE_EAST": 100,
  "MAP_SAFARI_ZONE_WEST": 260
 },
 "rival_name_len": 0,
 "early_potions": [
  "bedroom_pc",
  "route1_clerk",
  "forest_hidden"
 ],
 "early_bag": true,
 "tm39_sell": false,
 "pewter_balls": 0,
 "route4_magikarp": true,
 "mt_moon_heal": false,
 "early_torrent": {
  "fight": "brock",
  "steer_poison": true,
  "no_stall": true,
  "withdraw_first": false,
  "cure": true,
  "tackle_only": [],
  "cure_frames": 640.0
 }
}
```
