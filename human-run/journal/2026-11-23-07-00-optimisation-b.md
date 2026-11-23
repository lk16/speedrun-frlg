# 2026-11-23: optimiser B (dex fill and early/mid game), branch `human-run-play`

Worktree agent, scratch prefix `opb-`, plan block 1700+, pointer `plans/CURRENT-OPB` (descents promote onto it;
`plans/CURRENT` belongs to the other optimiser). Sim jobs `RAYON_NUM_THREADS=7`. Sign: child - parent, negative =
child better.

## Base

**1700-opb-base** = CURRENT `1504-reset1612` + `pass_reserve` 8 / `pass_reserve_rate` 100 (1620, MODEL) +
`safari_walk_steps` East 100 / West 260 (1621, MODEL) + 2 Antidotes at the Lavender ball stop (1626, PLAN).
vs 1504 (WR+2m, seed 7901, R 3.2M): **-1080 h [-1547, -690]** (274 vs 194 finishes).
The main session then promoted `1507-honest4` as CURRENT; its JSON equals 1700's except `name`, so every
comparison against 1700 here is a comparison against CURRENT 1507.

## Diagnosis: the dex fill's empty bag

`FRLG_BALL_TRACE`, 300 attempts of 1700: 960 `ball-empty` events in `dex-fill`, 352 in Viridian Forest (no balls
yet, by design), ~7 elsewhere. Ball takes before the fill: Route 23 hunt, Victory Road passing catches, Route 22.
The last ball stop is `mart@sabrina-badge` (Great 25). With scheduled marts `take_ball` never buys on demand, so an
empty bag turns every remaining fill encounter into a loss and the fill loops (up to 6 rounds) until the reset
rule fires.

New search set `descend.py --set dexfill` (`tools/dexmartknobs.py`): new top-up stops before the fill at marts
on the route (Indigo Plateau Center mart at the Victory Road exit, Great Balls,
`IndigoPlateau_PokemonCenter_1F/scripts.inc:54-55`; Celadon Dept. Store 2F after Erika,
`CeladonCity_DepartmentStore_2F/scripts.inc:29`; Viridian Mart, Poke Balls only, `ViridianCity_Mart/scripts.inc:66`),
Sabrina's Great target +5/+10, the fill list (steps, swaps, drops, added sites), then `dexknobs.py`.
Ultra stops are not searched: the ball rule throws Great and `bag_ball` never reaches up to an Ultra.

## Round 1: `--set dexfill` from 1700 (R1 1.6M seed 1, R2 3.2M seed 2, WR+2m)

(running)
