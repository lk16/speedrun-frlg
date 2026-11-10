# 2026-11-10: starter stats, sight-cone audit, type model, Safari rules (session 2, running notes)

User brief: search ~12 h; consider forced starter stats, abilities, optional trainers, routing, catch
priorities, bike/running shoes; test each in isolation first, then combine; commit as you go.
(Clock in this sandbox started the session around 09:00 on 2026-11-10; stop around 21:00.)

## Findings that changed the model (not plan improvements; scores before/after are not comparable)

1. **Rival ids were wrong.** Rival trainer ids are named after the *rival's* starter
   (`decompiled/data/maps/PalletTown_ProfessorOaksLab/scripts.inc:303-306`), so a Squirtle player faces
   `_BULBASAUR`. The model used `_SQUIRTLE`. Worse, `gen_dex.py` dropped Bulbasaur/Ivysaur/Venusaur (a
   regex swallowed the `[SPECIES_NONE] = {0},` line) and 103 trainers after any one-line party. Both fixed
   (plan 039, scores line `039-rivalfix`).
2. **Type model** (plan 040): per-mon type matchups for trainer fights from the decomp's type chart
   (`battle_main.c:312`) and species types. `type_floor` 0.9 is a guess (the human swaps in a covering mon);
   `type_strength` 1. Replaces the hand-set Brock factor 1.4. Under it Bulbasaur and Charmander starters
   are refuted against Squirtle (+14 h and +18 h at WR+40m).
3. **Sight-cone audit of forced trainers** (plan 041; `human-run/forced-audit.md`, `tools/forced.py`).
   Corrected both ways: Victory Road 0 forced (guess was 4), forest 1 (3), Saffron gym 0, but Route 3 (3),
   Route 25 (4), Route 6/8/9 (1+1+2), Rock Tunnel 8, Tower channelers 3, gyms 1 each (Celadon 2) were all
   missing. Net: +10 min on the no-reset time, EHT at WR+40m went 18 h -> 300 h. That is the honest price.
4. **Safari Zone rules** (decomp-cited): 600 steps, 30 Safari Balls, catch factor, flee rates. Fill hunts
   that leaned on Safari are much worse than modelled (see below).

## Starter stats (the user's first item)

* The earlier plans only reset on the level-5 stat *total*. Per-stat gates on the level-5 summary screen
  are far stronger: **level-5 SpA >= 12** (6% of Squirtles) plus HP >= 20 and total >= 71.
  EHT at WR+40m fell 54 h -> 31 h -> 30 h -> 29.7 h (plans 036-038), CI excluded 0 each time.
* A first attempt used the level-50 strength score as the gate. That is **void**: a human cannot see
  level-50 stats at the starter screen. It is in scores.jsonl marked not accepted.
* RNG manipulation is not feasible for a human here: `SeedRngAndSetTrainerId` seeds from the 16-bit
  hardware timer 1 (`decompiled/src/main.c:264-270`, called at `title_screen.c:735`), which wraps every
  ~3.9 ms, so frame-level timing cannot choose a seed. Not modelled.

## Abilities

Torrent/Blaze/Overgrow are not a plan choice. A sensitivity run (`ability_bonus`, a guess) shows the model
is very sensitive to small effective-level gains in close fights (a 5% bonus: -7 h) because `p_lose` is
a steep logistic around level ratio 1.25. Those lines are marked SENSITIVITY ONLY.

## Open
See the end of this file for the final state (updated as the session runs).
