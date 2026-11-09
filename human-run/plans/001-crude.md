# Plan 001: crude Round 2 plan (FireRed, Squirtle)

Status: first complete plan, scored by the model only. **Nothing here is calibrated against
libmgba except the pre-Brock text segments**, which come from the `defeat-brock` ledger at
frame-perfect speed and are scaled by `text_factor`. Everything else is `Guess` or a static tile
count (`docs/glitchless-run/distances.json`). `frlg-human report` prints the split by source.

## What the plan says

1. Soft reset until the Squirtle's six level-5 stats sum to at least 72 (about the top quarter,
   `frlg-human starter`). FireRed, Squirtle, carry everything on Squirtle; Paras/Abra/Pidgey HM
   slaves are not modelled (the model gives exp to the carry only).
2. The `glitchless-run` route order through the first Hall of Fame (`docs/glitchless-run/route.md`),
   with the bicycle voucher detour. Fight non-target wild mons while the carry is below level 30,
   flee them afterwards. Catch every new Kanto-dex species met, and hunt the listed patches
   (Poke Balls, no weakening).
3. Gifts and one-off catches on the way: Magikarp (Route 4 Pokémon Center,
   `decompiled/data/maps/Route4_PokemonCenter_1F/scripts.inc`), Eevee
   (`CeladonCity_Condominiums_RoofRoom`), Lapras (`SilphCo_7F`), the Route 16 Snorlax
   (`setwildbattle SPECIES_SNORLAX, 30`), Kabuto (`CinnabarIsland_PokemonLab_ExperimentRoom`).
4. Grind 20 wild battles at level 42 before the Elite Four, then Lorelei, Bruno, Agatha, Lance,
   Champion, Hall of Fame.
5. National Dex: Oak's rating scene needs `VAR_0x8006 >= 60` where `GetPokedexCount` with
   `VAR_0x8004 = 0` returns the **Kanto** caught count (`decompiled/src/prof_pc.c:23-26`), and
   `FLAG_WORLD_MAP_ONE_ISLAND` set (`decompiled/data/maps/PalletTown/scripts.inc:50-58`;
   `.../PalletTown_ProfessorOaksLab/scripts.inc:904-911`). The One Island flag comes free with the
   Blaine-triggered Bill scene (`CinnabarIsland_Gym/scripts.inc:61`, `OneIsland/scripts.inc:7`).
   If fewer than 60 are caught, the **fill** list is hunted in order until 60 (the adaptation
   rule). Buy Fire/Thunder stones (Celadon 4F, `CeladonCity_DepartmentStore_4F/scripts.inc:31-34`)
   and use the two Moon Stones (`MtMoon_B2F/map.json`, `PokemonMansion_1F/map.json`) to evolve.
6. Rematch chain, all cited: Celio asks for the Ruby only once the National Dex is enabled
   (`OneIsland_PokemonCenter_1F/scripts.inc:207-209`); Ruby from Mt Ember
   (`MtEmber_RubyPath_B3F/scripts.inc:8`); the Rainbow Pass opens the rest
   (`OneIsland_PokemonCenter_1F/scripts.inc:317-325`); Sapphire drops from Gideon in the Five
   Island Rocket Warehouse (`FiveIsland_RocketWarehouse/scripts.inc:56-60`); handing it to Celio
   sets `FLAG_SYS_CAN_LINK_WITH_RS` (`OneIsland_PokemonCenter_1F/scripts.inc:276`), which turns
   every E4 room and the Champion into the `_2` rematch
   (`PokemonLeague_LoreleisRoom/scripts.inc:57-62` and the other four rooms).
   Rematch parties are level 63-75 (`decompiled/src/data/trainer_parties.h:10564-10760`).
7. Beat the five rematches; the timer ends at the black frame after the second Hall of Fame
   (`hall-of-fame-2` is a fixed guess).

## Reset rules

Soft reset at the starter below 72. At every badge and at Hall of Fame, reset only if
`elapsed + best_case_remaining > T` (`reset_aggr = 0`). No aggressive thresholds yet.

## Branch rules (what the simulator implements)

- Wild encounter, new Kanto species: throw balls (up to 8, Poke Ball) with the real catch
  formula (`battle_script_commands.c:9549-9640`).
- Wild encounter otherwise: fight if carry level < 30 else flee.
- Whited out three times in one battle: abandon the attempt.
- Under 60 caught at Oak: hunt the fill list in order; if exhausted, abandon.

## Parameters (read by `frlg-human`)

```json
{
 "name": "001-crude",
 "starter_min_total": 72,
 "hunts": [
  {
   "after": "forest-encounters",
   "site": "MAP_VIRIDIAN_FOREST",
   "area": "land",
   "steps": 150,
   "detour_tiles": 4
  },
  {
   "after": "route3",
   "site": "MAP_ROUTE3",
   "area": "land",
   "steps": 150,
   "detour_tiles": 4
  },
  {
   "after": "mt-moon-1f",
   "site": "MAP_MT_MOON_1F",
   "area": "land",
   "steps": 150,
   "detour_tiles": 4
  },
  {
   "after": "route4",
   "site": "MAP_ROUTE4",
   "area": "land",
   "steps": 100,
   "detour_tiles": 4
  },
  {
   "after": "bill",
   "site": "MAP_ROUTE25",
   "area": "land",
   "steps": 200,
   "detour_tiles": 4
  },
  {
   "after": "route9",
   "site": "MAP_ROUTE9",
   "area": "land",
   "steps": 80,
   "detour_tiles": 4
  },
  {
   "after": "rock-tunnel",
   "site": "MAP_ROCK_TUNNEL_1F",
   "area": "land",
   "steps": 200,
   "detour_tiles": 4
  },
  {
   "after": "route8-7",
   "site": "MAP_ROUTE8",
   "area": "land",
   "steps": 150,
   "detour_tiles": 4
  },
  {
   "after": "tower-encounters",
   "site": "MAP_POKEMON_TOWER_5F",
   "area": "land",
   "steps": 200,
   "detour_tiles": 4
  },
  {
   "after": "cycling-road",
   "site": "MAP_ROUTE17",
   "area": "land",
   "steps": 150,
   "detour_tiles": 4
  },
  {
   "after": "mansion",
   "site": "MAP_POKEMON_MANSION_1F",
   "area": "land",
   "steps": 150,
   "detour_tiles": 4
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
 ]
}
```

## Score

Model score at the measured ladder rung: see `human-run/scores.jsonl` (first line). The rung is
**WR+80m** with only 59 finishes in 40 000 attempts, so the EHT is dominated by attempts that run
to the end and miss the target: `reset_aggr = 0` barely resets because hunts and grinding have a
zero best-case floor.

Sanity: the no-reset finishing time is ~5h20m (p10 5h06m) against the 3h25m WR, i.e. ~1.6x slower,
which is plausible for a plan with a 1h20m catch-up hunt. Not trusted beyond that.
