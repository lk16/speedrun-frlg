# 2026-12-02: post-door hazard decomposition, flat rows vs engine, and what could lower the true hazard

Base 1053-spe25. Rung WR+2m unless stated. Probes: `tools/probe.py`, paired, `+` = worse. New engine knobs `reload_*` (commit 41609a7).

## 1. Per fight: engine death vs flat `fatal` (fit_post_evidence.py, R 800k; 5 door attempts that fought, 2 finished)
| fight | engine | flat fatal | total | evidence posterior (k=3) | fatal if refit |
|---|---|---|---|---|---|
| Lorelei | .034 | 0 | .034 | .013 | 0 |
| Bruno | .071 | .187 | .245 | .217 | .157 |
| Agatha | .051 | 0 | .051 | .022 | 0 |
| Lance | .071 | .269 | .321 | .280 | .225 |
| Champion | .205 | 0 | .205 | .103 | 0 |
| Icefall..Gideon | 0 | .098-.149 | same | .09-.13 | .091-.133 |
| R2 Lorelei..Lance-2 | .001-.028 | .054-.096 | .056-.121 | .03-.07 | .03-.046 |
| Champion-2 (calm) | .078 | 0 | .078 | .047 | 0 |

HoF-1 window survival (Lorelei..Champion): engine alone .63, flat rows alone .59, total .37 (attr: 62% fail). About half the hazard (log scale) is flat stand-in, concentrated in Bruno and Lance; Champion is engine only. The engine's League deaths also rest on the league_power 1.4 stand-in (stats x1.4; with power 1 the L58 Blastoise loses the Champion in the `battle` tool), so even the engine share is calibrated, not mechanistic.
Door-to-finish: model .155; data 2/5 (Wilson .12-.77), posterior .23 (90% .08-.47). The data supports slightly lower rows (last column): a MODEL CHANGE, not a plan change. Sensitivity of 1053 to the rows (R 800k seed 7201): all x0.8 -33.9 h [-38.0,-30.1]; x1.25 +72.1 [+63.2,+82.2].

## 2. Ideas with mechanics evidence (paired vs 1053, R 800k seed 7201)
* Hyper Potions at Celadon (xbuys heals 4 -> 6 / 8): +12.5 [+8.4,+16.9] / +42.1 [+34.6,+50.6]. Money binds. Refuted.
* candy_early_n 1 -> 0: +47.6 [+37,+61.6]. Refuted.
* Mystic Water at celadon-tea: exactly 0.00, the segment name matched nothing; NOT tested.
* League whiteout play-on (`league_redo 1.0` + `salvage_at` 0.8 on the 10 League members): R 800k -3.3 [-4.5,-2.2]; R 3.2M seeds 7301/7302 -2.86 [-3.50,-2.31] / -2.79 [-3.42,-2.24]; lowered-rows model R 1.6M -1.8 [-2.3,-1.3]. Rules-safe, about 2%.
* Not testable in the engine: second fighter / switch tactics in the League (one fighter plus sacrifices; X queues and sacs already descended), Revive budget, League prep order.

## 3. Save-and-reload what-if (rules unknown; NOT CURRENT)
`reload_save_at`: saves (800 frames each) before listed segments; a lost League fight or flat-row death soft-resets and loads (1500 frames), full state restored, fresh draws, max 3, pace-gated. PROMPT.md category facts say only: timer starts at New Game, ends at the HoF-2 black frame, no glitches; nothing on reloading. STRATEGY 1.2/9: he refuses save-state starters, no save-and-quit manips. ASSUMPTION: loading an in-game save keeps the timer running and is allowed; must be verified.
R 800k: saves at Lorelei/Lorelei-2 only -14.9 [-19.9,-10.0]; five R1 fights -31.7 [-38.5,-24.9]; all 14 -38.1 [-46.8,-30.3]. R 1.6M seed 7303: all 14 -45.8 [-52.0,-39.7] (152.7 -> 106.9); lowered rows -15.9 [-19.8,-12.1]. At 2x cost (1600/3000 frames, max 1 reload): +317.7, since 14 saves cost minutes of timer: the result hinges on per-save cost and on picking few saves.
Pending at writing: WR-rung confirmations and hi/half-row robustness; appended to scores.jsonl only if finished.

## 4. Under Mewtwo-PP-on
Decomposition and row sensitivity are PP-independent; R2 engine rates will rise. League redo and reload act on every League death and stay; heal and candy refutations are R1/mid-game and PP-independent.

## Next
Mystic Water with a real segment name; per-save-cost grid and a minimal save subset; League redo at WR rung and 3.2M under PP-on; lowered rows as a labelled model-change plan.
