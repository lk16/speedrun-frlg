# Plan 375-manip-rashmild

Parent: `372-tactic181`. Change: the starter phase is modelled as Main97's frame-exact press procedure
(`starter_model = "manip"`, `crates/frlg-human/src/manip.rs`, mechanism in `docs/tricks/starter-manip.md`)
instead of a random starter gated on the level-5 stat total. The route, tactics and hunts are those of 372
(`001-crude.md` still gives the step-by-step plan and its citations); `frlg-human` reads only the block below.

## What the starter phase does in the model

Per attempt three presses matter: the title A (wild seed), the naming-screen exit and the A that dismisses
"this POKeMON is really energetic" (the starter's PID/IVs/nature, `docs/tricks/starter-manip.md` 1.1-1.3).
Each lands on the target frame with probability `p` (below); a miss on any of them gives an independent random
starter, and the human finds out at the stats screen and soft-resets. When all three hit, the starter is drawn
from the reachable-good set: nature uniform over {Rash, Modest}, SpA IV >= 30, Spe IV >= 20, HP IV >= 12 and
every other IV uniform; its REAL IVs/nature go into the carry's stats for every later fight (`battle_sim`).
A tier costs on-timer idle frames waiting for the table's frame: one dial (ball idle `j` only) with mean
`1 / (0.933 rho)` = 914 frames = 15.3 s (the "15 seconds later" trade of STRATEGY.md 1.2).

## Fitted parameters (labelled guesses unless the scan data fix them)

| Parameter | Value | Basis |
| --- | --- | --- |
| per-press hit `p` | **0.692** (`sigma` 0.49 frames, `mu` 0) | fitted so that P(a starter survives the lab) = p^3 = 0.333; scans: 7/20 (11-11), 7/21 (11-07), 11/45 (11-10), 13/27 (11-06) = 34% overall, session range 24-48% |
| off-timer, power-on to the title press (`s_boot_title`) | 45 s on top of the route's 13.5 s boot | guess, fitted: start-to-start cycle of a rejected attempt 298 s vs scan median 282-335 s (11-10: 282, 11-11: 335, 11-07: ~300, 11-06: 412) |
| off-timer, soft reset after the miss at the stats screen (`s_reset`) | 100 s | same fit; scans show 100-250 s of grey timer/title between attempts |
| reject point | the existing `starter` check, ~129 s of timer + idle | scans: rejected timers 108-200 s, medians 133-148 s |
| tier | Rash+Modest, SpA >= 30, Spe >= 20, HP >= 12, one dial | sweep below (minimum EHT among tiers with <= ~15-25 s idle) |
| quiet wild seed | off | refuted at every tier in this model, see below |

Starter phase of this plan (`frlg-human starter-phase`, 40000 attempts): P(good starter kept) 0.333, mean
attempts to a good starter **3.00** (p10/50/90 1/2/6), wall to a good starter **809 s = 13.5 min**
(p10/50/90 203/518/1711 s); an accepted attempt reaches the stats screen at 213 s wall, a miss costs 298 s.
The old random model (372) kept 14.3% of attempts at ~2.5 min each (about 7 attempts, ~17 min).

## Score (paired against 372, rung WR+160m, seed 1, R = 640000)

EHT 372 = 11.759 h, EHT 374 = 10.593 h; paired difference -1.166 h, 95% CI [-1.256, -1.082] (seed 2, R = 320000:
-1.221 h, CI [-1.361, -1.083]). Significant and above 1% of the score; accepted. Every number is a score of the
model: the press hit rate, the off-timer costs and the tier cost are fits/guesses, not measurements.

## Sweep (R = 160000, seed 1, rung WR+160m, vs 372; `scores.jsonl` has every row)

| Setting | EHT (h) | idle on timer |
| --- | ---: | ---: |
| 372 (random starter, total >= 74) | 11.83 | - |
| T1 only (SpA >= 30), 1 dial | 11.79 (n.s.) | 3.6 s |
| T1 + Spe >= 20, 1 dial | 11.32 | 9.6 s |
| Rash+Mild, Spe >= 20, HP >= 12 (the first main) | 10.93 | 15 s |
| **Rash+Modest, Spe >= 20, HP >= 12 (this plan)** | **10.66** | 15 s |
| Rash+Modest, Spe >= 20, HP >= 12, Def >= 12 | 10.48 | 24 s |
| Rash+Modest, Spe >= 20, HP >= 16 | 10.54 | 22 s |
| Spe >= 20, HP >= 25, Def >= 25, Rash+Mild | 10.31 | 200 s (not Main97-like) |
| doc T3 (Spe/HP/Def >= 25), 2 dials, Rash+Mild | 10.15 | 2.5 s |
| Rash+Modest, Spe/HP/Def >= 25, 2 dials | 10.08 | 3 s |
| natures: Rash 10.65, Rash+Modest 10.66, Modest 10.64, Rash+Mild 10.93, all three 10.88 | | |
| per-press hit 0.5 / 0.69 / 0.9 / 1.0 (Rash+Mild tier) | 11.91 / 10.93 / 10.56 / 10.49 | |
| press jitter sigma 1.0 frame (p = 0.38) | 13.93 | |
| off-timer costs x0.5 / x1 / x2 | 10.81 / 10.93 / 11.18 | |
| quiet seed <= 2 / 4 / 8 hits in the first 60 rolls | 12.50 / 12.31 / 11.38 | |
| quiet seed <= 4 hits in the first 30 rolls | 11.08 | |

Rash+Modest beats Rash+Mild by 0.30 h (paired, R = 320000, CI [0.24, 0.36]): Mild's -Def costs more than the
nature's extra density gains, consistent with STRATEGY.md 1.5 ("Mild is the worst of the three"). Natures are
the knob with a clear minimum; the IV tiers have a broad flat optimum (HP/Def >= 12-16 with Spe >= 20 within
the noise of each other, +-0.2 h), and the hit rate is a large lever (p 0.5 -> 0.9 is 1.4 h).

### Quiet wild seed: refuted in this model

The seed suppresses forest-rate encounters in the first `quiet_window` rolls (cap on the number of hits, the
wait for the title frame is off the timer and in the wall time, a title miss gives an unconstrained seed).
At every tier it makes EHT worse (+0.15 h for a 30-roll window, +0.5 h at <= 8 hits, +1.4-1.6 h at <= 4 and <= 2 hits, vs the same tier without it): the forest's ~3.8 min of fights is saved, but the
carry reaches Brock at level 7.4 instead of 8.2 (Brock whiteouts 1610 vs 619 per 20000) and 3.3 instead of 6.0
species are caught by Brock, so the dex fill later costs more. The only mitigation tried (2 grind battles after
the forest trainers) recovers 0.2 h of it. This contradicts the runner's "saves a minute"; the model credits
neither a route re-tuned around the quiet seed nor encounters chosen by the seed (Pikachu / the slots he
wants), so treat the result as "not worth it for THIS plan", not as a claim about the real procedure.

## Uncalibrated

- The press hit probability is a fit to survival-past-lab counts that mix hit/miss, rejects for other reasons
  (rival fight, Context failures) and regime drift; `p_accept` = 1 and there is no drift regime.
- One vs two dials (`manip.dials`): the scans cannot tell. Two dials (needs an emulator-built naming-seed
  table, doc 4.2) would be worth ~1 h more; the accepted IVs in the scans (HP 10-30, Def 11-31) show no HP/Def
  filter of the strength the sweep would like, so this plan keeps one dial and a light tier.
- The quiet-seed model (hits capped inside the first N rolls, rolls counted per step) is a guess about
  stream index drift (journal 2026-11-13-12-30).
- Off-timer costs are fitted to cycle times, not measured per segment.
- LeafGreen tables and BizHawk/hardware agreement are not covered (`docs/tricks/starter-manip.md` 4.5).

```json
{
 "name": "375-manip-rashmild",
 "starter_min_total": 74,
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
   "steps": 15,
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
  },
  {
   "after": "hall-of-fame-1",
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
   "after": "victory-road",
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
   "after": "victory-road",
   "kind": "gift",
   "species": [
    "SPECIES_METAPOD",
    "SPECIES_BUTTERFREE"
   ],
   "requires": [
    "SPECIES_CATERPIE"
   ],
   "frames": 2000
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
 "grind_pre_e4": 4,
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
   "after": "giovanni-badge",
   "mv": "MOVE_EARTHQUAKE",
   "frames": 800,
   "cost": 0
  },
  {
   "after": "mewtwo",
   "mv": "MOVE_WATER_PULSE",
   "frames": 600,
   "cost": 0,
   "mewtwo": true
  },
  {
   "after": "mewtwo",
   "mv": "MOVE_FIRE_BLAST",
   "frames": 600,
   "cost": 0,
   "mewtwo": true
  }
 ],
 "keep_moves": [
  "MOVE_DIG",
  "MOVE_SURF",
  "MOVE_BLIZZARD",
  "MOVE_EARTHQUAKE"
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
   "x": [],
   "sacs": 4
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
   "sacs": 1
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
    "attack",
    "attack",
    "attack",
    "attack",
    "speed"
   ],
   "sacs": 2
  },
  "bruno": {
   "x": [
    "special",
    "special",
    "special",
    "speed"
   ],
   "sacs": 2
  },
  "agatha": {
   "x": [
    "special",
    "special",
    "special",
    "speed"
   ],
   "sacs": 2
  },
  "lance": {
   "x": [
    "special",
    "special",
    "special",
    "special",
    "speed"
   ],
   "sacs": 2
  },
  "champion": {
   "x": [
    "special",
    "special",
    "special",
    "special",
    "speed",
    "special",
    "special"
   ],
   "sacs": 4
  },
  "rival-ss-anne": {
   "x": [],
   "sacs": 1
  },
  "rival-silph": {
   "x": [
    "special",
    "special",
    "speed"
   ],
   "sacs": 3
  },
  "rival-route22": {
   "x": [
    "special",
    "special",
    "speed"
   ],
   "sacs": 3
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
    "special",
    "special"
   ],
   "sacs": 4
  },
  "rival-cerulean": {
   "x": [],
   "sacs": 2
  }
 },
 "early_dig": true,
 "erika_after_tower": true,
 "grinds": [
  {
   "after": "cerulean-heal",
   "battles": 2,
   "level": 12,
   "yield_": 58
  },
  {
   "after": "captain-hm01",
   "battles": 8,
   "level": 20,
   "yield_": 70
  },
  {
   "after": "koga-badge",
   "battles": 5,
   "level": 30,
   "yield_": 90
  },
  {
   "after": "misty-badge",
   "battles": 5,
   "level": 16,
   "yield_": 60
  }
 ],
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
  "silph-walk"
 ],
 "clemi_skip": true,
 "starter_model": "manip",
 "manip": {
  "dials": 1,
  "natures": [
   "Rash",
   "Mild"
  ],
  "min_spa": 30,
  "min_spe": 20,
  "min_hp": 12
 }
}
```
