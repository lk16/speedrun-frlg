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

## More model corrections found by checking the model against the decomp or libmgba

5. **Money** (plan 065): prize money `4 x last level x class value x (double ? 2 : 1)`
   (`battle_script_commands.c:5373`), start 3000 (`new_game.c:128`), balls 200/600/1200 (`items.json`),
   Great Balls only from Lavender, Ultra only from Fuchsia (mart scripts), Safari fee 500. "Ultra balls
   everywhere" was free in the old model. Under it Great Balls beat Ultra Balls early (EHT 27 h -> 10.6 h
   at WR+80m before re-tuning).
6. **Wild-encounter costs measured on libmgba** (plan 084, `calibration.md`, `crates/frlg-calib`):
   battle start to first menu is 358 frames (guess was 220), a ball throw cycle ~400, running from a
   wild battle works ~19% of the time at level 6 and costs 400+ frames per attempt (guess: instant, 150).
7. **Fishing** (plan 087): the game picks fishing slots by rod (old 0-1, good 2-4, super 5-9,
   `wild_encounter.c:120-150`); the model mixed all ten. Now `rod` (default super, Route 12 house).
8. **Repels** (code only): a Repel blocks encounters below the lead's level
   (`wild_encounter.c:286,601`). Used everywhere it kills the passing catches the dex needs; kept as a
   per-segment knob (`repel_on`) for the descent.
9. **Fights** (`calib_battle`, optional): turn 280-406, kill cycle 889-1177, rival intro 1040 frames,
   measured on libmgba. Consistent with the TAS ledger at a 1.2 human factor.

## Abilities, again

Lead-mon abilities do exist in the encounter code (`wild_encounter.c:305-340`): Stench halves and
Illuminate doubles the encounter rate. Not used: a hunt's time is dominated by what an encounter costs
(~450 intro + ~500 flee or ~650 catch) not by walking (9 frames/step at ~20% per step), so doubling the
rate saves little, and Staryu (the Illuminate lead) needs a Super Rod trip.

## Automation (tools)

`tools/descend.py` is a coordinate descent over single knobs (fill step budgets, detours, order, drops,
reset aggressiveness, starter gates, hunts, specials, balls, toggles, Repel segments) that applies the
PROMPT rules, requires the win on seeds 1 *and* 2, promotes and commits each accepted plan. Many of
the plans 046-091 come from it. It is greedy, so a plan that sits in a local optimum of the model is not a
claim about the game.

## Later in the session (12:00-13:00)

* **Grass steps from the map data** (plan 166, `tools/pathenc.py`, `data/pathenc.json`): the metatile
  encounter attribute along the shortest path of each leg. The old guesses were far off: Routes 3, 4, 9, 10,
  24, 25 and the Cycling Road have no grass on the shortest path (guess 30-60 each), Route 1 has 22 (50),
  Route 2 5 (60), Mt Moon 65+13+116 (290), Rock Tunnel 252 (138), Tower 3F-7F 108 (80), Victory Road 91
  (150), Route 21 and 23 water 36-47 and 39 that the model never rolled. Dex at Oak's check falls from 34 to
  29 (fewer passing catches) but the run is ~8 min faster. A walk that pays up to 50 tiles per grass tile
  (`avoid_grass`) is slightly worse than the shortest path (EHT 5.31 vs 5.05 h at WR+80m).
* **Mansion** (`tools/mansion.py`): with its switch modelled the Secret Key is reachable without crossing any
  static cone; Silph, Surge's gym and the Blaine quiz were already 0. If those four groups were forced after
  all (Silph 4, Mansion 3, quiz 2, Surge 2) the EHT at WR+80m doubles (5.5 -> 10.4 h).
* **Game Corner prizes** (LeafGreen coin prices x 20 money): a Clefairy for 15000 money after the first
  Hall of Fame is a small win (plan 131, -17% at the time); Pinsir (50000) drains the ball money.
* **Trades**: Mr. Mime for the Abra at the Route 2 house is accepted (plan ~160); Seel needs a Ponyta (Sevii
  only), Jynx a Poliwhirl, Lickitung a Slowbro/Golduck.
* **Not levers**: extra reset checks (`extra_splits`), save-in-the-lab starter re-rolls (also needs a rules
  ruling; it is not better anyway), the Saffron Dojo Hitmon gift (~5 min), a scheduled mart itinerary
  instead of on-demand buying (worse in the model, but it exposes that early money is what limits balls:
  1.7k at Cerulean, 13k at Lavender, 70k at the Hall of Fame).
* Starter alternatives under the final model: Charmander +4%, Bulbasaur +5-7% EHT vs Squirtle (refuted but
  close); FireRed +3% (untuned fill list).

## Sensitivity of the final bulk plan (239-fillsteps11, EHT at WR+80m = 4.50 h)

Model switches (each flips one modelling choice; guessed-vs-corrected): type model off 4.54; type_floor 1.0
4.43 / 0.8 4.89 / **0.7 7.76**; money limits off 4.45; Safari rules off 4.39; guessed encounter costs 4.15;
guessed battle costs (battle_scale 1.75) 5.32; **guessed grass steps 6.03**; guessed detours 4.39; mixed
fishing rods 4.58; Charmander 4.72. Unaudited trainer groups forced after all: Silph 4.65, Mansion 4.61,
all four (Silph, Mansion, quiz, Surge) 5.16. Constants (tune): level_up 200/450 -> 4.46/4.59; trainer_outro
150/600 -> 4.42/4.77; whiteout, p_event, power_bonus, text_factor, mistakes: within 3%;
**team_bonus 1.0 -> 5.76**, 1.2 -> 4.35; p_lose centre 1.15 -> 5.41, 1.35 -> 4.43; p_lose slope 0.1 -> 4.85;
weaken KO chance 0.15 -> 5.16. The plan's ranking against its ancestors holds at jitter 0/2/8, reaction
0 and 30-60, mistakes 0/2, text 1.0/1.5, path 1.0/1.3 (`tools/robust.py`). The type_floor and team_bonus
guesses (how a lone carry copes with bad matchups) and the loss curve are the model's weakest
battle assumptions. Re-tuning the plan under type_floor 0.8 (chain `CHAIN-F08`) changed one fill budget
only, so the plan's structure does not depend on it.

## Open
See the end of this file for the final state (updated as the session runs).
