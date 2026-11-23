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

Logs: scratch `opb-descent1/2/3.log`; every row in `scores.jsonl` (plans `d1701-*`, `d1702-*`).
From the second run on `FRLG_MINGAIN=0.003` (a CI excluding 0 by 0.03 h is not a gain; the first run promoted one).

* **More balls before the fill is worse, not better.** Indigo mart Great 10 +15 h [0.03, 60], 15 +76 n.s., 20 +514,
  30 +1740; Viridian Poke 10 / 20 +277 / +559; Sabrina Great 30 / 35 +171 / +331; Celadon after Erika 0.0 (the bag
  is still above any target there). The money goes to the Indigo heal purchase; the empty-bag fills are mostly in
  attempts the reset rule ends anyway. The 866/960 empties are not a plan gap worth paying for in this model.
* Fill list: steps x0.6 / x1.5 per site, all adjacent swaps, all drops, 26 added sites: nothing significant better.
  Drops of Seafoam B3F +1207, Route 11 +720, Victory Road 2F +451, Seafoam B4F +139; added sites at the front
  +410..+2390. Closest: drop / swap Diglett's Cave (fill[6]) -43 [-105, +0.4] (twice, same effect), Route 18 drop
  -85 [-267, +125]. **1701-fillsteps14** (Route 21 North steps x0.6) was promoted on the pointer at -0.03 h
  [-0.037, -0.026] / -0.03 (seed 2): a near-zero artefact, not a gain (Route 23 / Safari Center steps likewise
  -0.3..-0.6 h on seed 1, n.s. on seed 2).
* dexknobs (first part, evolutions): drop Nidorina +851, Nidoqueen +1140, Nidoking +557; Nidorina later +157;
  stone buy flips and the Clefable drop 0.0 (never fire).
* Honest-Safari subset (`--kinds safari,ball,hunt,candy,...`): `safari_stop` 0/6/7/8/9/10 all n.s. (+68..+262),
  East/West swap +333 n.s., East / West steps x0.6 / x1.5 n.s. (-21..+211): the step counter does not open a
  better Safari plan. `candy_early_n` 0/1/2 +13 700..+20 200 h (much worse); skipping the Power Plant +5931.
* Stopped at the main session's wrap-up (Pewter / hunt / ball-rule / passoff candidates of the subset not run;
  `--set earlymid` not started).

## Result

No confirmed significant improvement. Best plan = **1700-opb-base = CURRENT 1507-honest4** (WR+2m ~2100 h at seed
7901 R 3.2M; -1080 h [-1547, -690] vs 1504). Do not adopt 1701 (artefact). Well over 8 refuted in a row.

## Next (ranked)

1. `--set earlymid --notkinds fights` from 1507 (not started), with `FRLG_MINGAIN=0.003`.
2. Rest of the dexfill subset: `--kinds hunt,ball,passoff,pewter` (`--set dexfill`, after the candy/safari part).
3. Drop Diglett's Cave from the fill at R 6.4M (-43 h, CI just touching 0, twice).
4. Ball money vs League money: fewer balls at the earlier stops (shops agent's area) is the direction the
   pre-fill stop results point to.
