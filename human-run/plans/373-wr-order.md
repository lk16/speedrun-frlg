# Plan 373-wr-order

Parent: `372-tactic181` (parameters), but this is a **model re-baseline, not a descent step**: the route is
built in the world record's order (`route_style = "wr"`, `crates/frlg-human/src/route_wr.rs`) and the model
constants are calibrated against the WR run's splits and fight durations
(`human-run/video-analysis/wr-3.25.25-main97.md`, `human-run/video-analysis/data/calib/`,
`human-run/calibration.md` "WR fight fit"). Its score is not comparable with 372's.

The step-by-step description, citations and branch rules in `001-crude.md` still apply except where the
parameters below differ; `frlg-human` reads only the JSON block. Every segment cost is a model estimate (see
`001-crude.md` for what is ledger, tile-count or guess); what changed is listed here.

## Route order (the WR's split names in brackets)

Brock [Brock] -> Route 3, Mt Moon, Cerulean, Misty [Misty] -> rival, Nugget Bridge, Bill [Bill] -> Vermilion,
voucher, S.S. Anne, back to the Cerulean bike shop [Bike] -> Rock Tunnel [Rock Tunnel] -> Lavender, Celadon,
Hideout [Giovanni 1] -> Route 16 house (Fly), Pokemon Tower, flute, Surge (folded in), Celadon, Snorlax
[Snorlax] -> Safari (+3 hunts), Koga [Koga] -> Power Plant (+hunt), Mansion, Blaine [Blaine] -> first Sevii trip
[Sevii] -> stones -> Silph, Sabrina [Sabrina] -> Erika [Erika] -> Warden, Viridian, Giovanni [Giovanni 3] ->
Route 22 rival, Route 23, Victory Road, **the plan's `fill` list until 60 species** -> Indigo [Start E4R1] ->
E4 round 1 [Lorelei 1 ... Champ 1] -> Oak (no dex check: all 60 are done), Celio chain with Icefall Cave
(`decompiled/data/maps/FourIsland_IcefallCave_Back/scripts.inc`, Lorelei scene) [Icefall Cave] -> Rocket
Warehouse [Rocket Warehouse] -> Cerulean Cave Mewtwo -> E4 round 2 [Lorelei 2 ... Champ 2, End].
Not in the WR order: Dig (`no_dig`), Aerodactyl/Kabuto gifts (Omanyte after the Sevii trip instead), the grind
segments, a post-Hall-of-Fame fill phase.

## What was calibrated, and against what

| Knob | value | evidence |
| --- | --- | --- |
| `calib_wr` | true | turn 345 / enemy mon 630 / intro 0.8x / outro 240 / level-up 240 frames: least squares on 16 WR fight durations; encounter, throw, Run constants = libmgba floors x ~1.06 |
| `carry_power` | 1.9 | Attack and SpA x1.9 for the starter-line carry = Torrent x1.5 (the WR enters 5 of 7 sampled fights at <= 1/3 HP) x ~1.3 (Rash, SpA IV 30, EVs). Fitted so model player actions match the WR's 16 fights: mean abs log ratio 0.58 -> 0.28 |
| `mewtwo_power` | 1.5 | same fit on the 4 round-2 fights (the engine needs ~1.5x the video's actions with Mewtwo) |
| `candy_early` + `candy_early_n` 2 | true, 2 | the WR is Lv20/27/41 at Misty/S.S. Anne/Koga but Lv50 at the League; the first two Rare Candies are eaten on the spot, the other four kept for Indigo |
| `text_factor` / `path_overhead` / `mistake_rate` | 1.02 / 1.02 / 0.3 | a top runner's typical attempt (WR-level play is 1.0 / 1.0 / 0, which lands the model at the WR's splits within +-2%) |
| trainer fights | all audited (`audited_forced`), no grinds | window times: with the cone-forced trainer lists, Pewter's Camper Liam added, the model matches every WR split from Misty to Giovanni 3 within -1% .. +6% |
| specials | Abra and Magikarp bought, Clefairy bought after Koga, Omanyte, candy evolutions at Silph | the WR gets 60 species before the League; these are the cheap non-catch entries (`decompiled/src/data/ingame_trades.h`, `Route4_PokemonCenter_1F`, Game Corner prizes) |
| tactics | X Special lists per League fight, Champion 4 sacs, no other sacs | video: no sacrifices, ~1-3 X Specials per big fight; the Champion sacs are the model's price for the starter-quality gap |
| `fill` | 372's list, run before Indigo | no post-HoF fill in this order |

Splits, levels and species counts per WR split are in `human-run/journal/2026-11-13-22-00-wr-order-calibration.md`.

```json
{
 "name": "373-wr-order",
 "starter_min_total": 74,
 "hunts": [
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
   "site": "MAP_SAFARI_ZONE_EAST",
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
   "frames": 2400
  },
  {
   "after": "koga-badge",
   "kind": "buy",
   "species": [
    "SPECIES_CLEFAIRY"
   ],
   "cost": 15000,
   "frames": 2400
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
   "level": 30
  },
  {
   "after": "route4",
   "kind": "buy",
   "species": [
    "SPECIES_MAGIKARP"
   ],
   "cost": 500,
   "frames": 900
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
 "stones": [
  "MOON",
  "THUNDER",
  "FIRE",
  "LEAF",
  "WATER"
 ],
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
  "sabrina-badge": 0.7,
  "blaine-badge-and-sevii-scene": 0.9,
  "giovanni-badge": 0.7,
  "oak-national-dex": 0.9,
  "brock": 0.8,
  "hall-of-fame-1": 0.6,
  "indigo-heal": 0.5
 },
 "starter_min_stats": [
  19,
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
   "special": 34,
   "speed": 6,
   "attack": 4,
   "revives": 8,
   "frames": 1800
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
    "special"
   ]
  },
  "bruno": {
   "x": [
    "special"
   ]
  },
  "agatha": {
   "x": [
    "special"
   ]
  },
  "lance": {
   "x": [
    "special",
    "special"
   ]
  },
  "champion": {
   "x": [
    "special",
    "special",
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
    "special",
    "special"
   ]
  },
  "bruno-2": {
   "x": [
    "special",
    "special"
   ]
  },
  "agatha-2": {
   "x": [
    "special",
    "special"
   ]
  },
  "lance-2": {
   "x": [
    "special",
    "special"
   ]
  },
  "champion-2": {
   "x": [
    "special",
    "special",
    "speed",
    "special",
    "special"
   ]
  },
  "rival-cerulean": {
   "x": []
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
 "candy_early_n": 2
}
```

## Score

- rung **WR+20m**, EHT **22.395 h** at R=200000 (4605 finishes <= WR+20m), seed 1. Other rungs: WR+10m 16 finishes (EHT 1667 h), WR+40m 17.06 h, WR+80m 17.04 h, WR+160m 17.04 h; WR..WR+5m none in 200000.
- no-reset distribution (R=100000, 3074 finishers): min 3:24:56, p10 3:34:02, p50 3:41:07, p90 3:49:27 (WR 3:25:25).
- evaluation lines: `human-run/scores.jsonl`, plans `373-wr-order` and `w373-001..011` (calibration sensitivities, paired against this plan at WR+20m).
- `human-run/plans/CURRENT-WR` points here; `CURRENT` (the old-order chain) is unchanged.
