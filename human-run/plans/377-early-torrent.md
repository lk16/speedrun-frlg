# Plan 377-early-torrent

Parent: `376-wr-manip` (WR route order + manip starter). **MODEL CHANGE, not an improvement of the same model**: the early
game (first 30 minutes, where real attempts are lost) is modelled the way the video shows it. EHT(WR+10m) 132.8 h -> 53.1 h
(seed 1, R = 1,000,000; seed 2, R = 2,000,000: 127.7 -> 53.9 h; paired difference +79.7 h, CI [75.3, 84.0] / +73.7 h, CI [70.8, 76.5]).
Mechanisms in `crates/frlg-human/src/{battle,sim,manip,route}.rs`; the sweeps are rows `w377-*` in `scores.jsonl` and the
journal `journal/2026-11-14-00-00-early-game-torrent.md`.

## What changed against 376 (each is a plan knob, default off)

1. **Early deaths are not salvaged** (`no_salvage_until`): a whiteout ends the attempt (the runner resets: STRATEGY.md 7) instead
   of the old retry from the Centre with +42 s. Alone this helps (376: 132.8 -> 99.5 h) because retried dead runs ate time.
2. **Encounter-seed manip, corrected** (`manip.quiet_max_hits`/`quiet_window`/`quiet_keeps_wanted`): the quiet seed removes the
   *unwanted* encounters of the first 60 rolls but keeps the ones the human wants (the forest bug, the Route 1 kills for exp).
   The old knob suppressed every encounter, so the carry reached Brock at Lv7.4 with 3 species and the fill paid it back
   (+30 min of fill, refuted at +2.5 h at WR+40m). Corrected: Brock 15:56 -> 14:27, the "saves about a minute" of the runner (-1:29 here).
   99.5 -> 67.3 h. Hit-count tiers 2/4/6 are indistinguishable (14:23 / 14:35 / 14:36).
3. **Torrent as a tactic** (`torrent_until = indigo-heal`): Pokemon Center stops (`extra_heals`) are skipped, no pre-fight potion to
   85%; before every fight the carry heals only to `torrent_hits` (2) max-roll hits of the foes' best move (after one Withdraw),
   usually leaving the HP inside the <= 1/3 boost; in battle: Withdraw when the foe's max roll would KO and the carry moves first,
   a stall turn (Tackle) while above 1/3 HP when one hit is survivable and lands the carry in the boost, heal only below 30%
   (`torrent_heal_below`) under a real threat; poison is kept (field poison 1 HP / 5 steps = `poison_frames_per_hp` 70, Antidote
   at 3 HP) and not cured. The engine's own x1.5 Torrent (`battle.rs` `calc_damage`, `pokemon.c`) does the damage.
4. **Two optional Mt. Moon trainers** (`extra_fights`: Josh, Miriam): the carry reaches Misty at Lv20 as the WR does
   (it was Lv18, Misty took 9 actions).
5. Misty `power` 0.75 x the fitted carry_power (4.4 -> 5.1 actions, video 6-8).

## Torrent sweeps (seed 1, R = 1,000,000, WR+10m, paired; all rows in scores.jsonl)

`diff` = parent EHT - child EHT in hours, so positive means the child is faster (scores.jsonl stores child - parent).

| change (parent) | EHT parent -> child | diff | CI |
| --- | --- | --- | --- |
| Torrent to Mt. Moon, 1.5 hits (heal-up default, early game) | 74.25 -> 73.96 | +0.29 | n.s. |
| ... hits 0.5 / 1 / 2 / 3 | | -14.1 / -4.9 / **+1.3** / -0.8 | |
| ... no stall turns (hits 2) | 72.9 -> 75.3 | -2.35 | [-2.95, -1.77] |
| Torrent to giovanni-badge (hits 2) | 72.9 -> 62.0 | **+11.0** | [9.4, 12.4] |
| ... to rival-ss-anne / koga / sabrina | | -10.4 / -7.8 / -6.9 vs giovanni-badge | |
| ... to indigo-heal | 62.0 -> 58.4 | +3.6 | [2.9, 4.2] |
| in-battle heal below 0.3 (vs 0) | 58.4 -> 50.8 | **+7.6** | [6.7, 8.5] |
| ... 0.15 / 0.4 / 0.5 / 0.6 | | +4.7 / -0.9 / -3.0 / -9.7 vs 0.3 | |
| carry_power 1.6 / 1.3 with Torrent | | zero finishes at WR+10m | the fit is knife-edge |

The earlier refutation (a global "never heal" policy, +8.7 h) is not this: the early/mid-game Torrent phase with the window,
Withdraw, stall and a 30% emergency heal is worth about +19 h over the heal-up default once the model has real early death rates.

## Early splits and pass rates (model, no resets, R = 100000; `frlg-human splits`)

| | WR | 376 | 377 |
| --- | --- | --- | --- |
| Brock split | 14:40 | 15:56 (+8.7%, p10 14:53) | **14:27** (-1.5%, p10 13:35) |
| Misty split | 28:01 | 27:52 | **28:03** |
| Brock fight (core / actions) | 31 s / 2 | 42 s / 2.0 | 43 s / 2.0 (the video's 31 s is intro to EXP text with a 4 s intro; the model's intro is 8 s) |
| Misty fight (core / actions) | 54 s / 6-8 | 97 s / 9.1 | 60 s / 5.1 |
| reach Brock / Misty / Bill / Koga (of starter passers) | 25-75% | 100% / 99.9% / 99.9% / 99.9% | 83% / 77% / 73% / 64% |
| no-reset finish p10 / p50 | 3:25:25 (WR) | 3:34:02 / 3:41:07 | 3:30:35 / 3:37:05 |

The 83% at Brock is the video's good-day figure (75%); the bad days (2/12, 3/14, 4/9) are a regime the model does not
draw (hits 0.5..3 give 75-86%). Per-step deaths after Brock are 5-8% a split (the video: "1-6% crit/min-roll events", Koga/Sabrina
deaths). The 33% that pass the lab is the stat-screen gate of plan 374 (scans: 34%), not a pass rate.

## Still not tuned or measured

Reset thresholds are 376's (early ones are inert: 0.5/0.4 give identical scores; Brock 1.0 / Misty 1.0 are catastrophic). Persim Berry,
Sammy's poison rule (camp or leave), the 6 potions, X Special at Pewter, rival Growl/Quick Attack crits, the Mega Kick PP budget and Route 3
Jigglypuff are not modelled. `carry_power` 1.9 is still a stand-in; with Torrent it is not refit (1.6 gives no finishes).

```json
{
 "name": "377-early-torrent",
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
  },
  "brock": {
   "entry": true
  },
  "misty": {
   "power": 0.75
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
 "candy_early_n": 2,
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
  "quiet_max_hits": 4,
  "quiet_window": 60,
  "quiet_keeps_wanted": true
 },
 "no_salvage_until": "hall-of-fame-2",
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
 "torrent_until": "indigo-heal",
 "poison_frames_per_hp": 70,
 "torrent_hits": 2,
 "torrent_heal_below": 0.3
}
```
