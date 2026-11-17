# 2026-12-01: where the expected hours go, and the one lever the descent never had

New tool: `frlg-human attr PLAN --rung R --R N [--bysegment]` (Outcome now carries `off`, `end_seg`, `kind`). `human-run/tools/probe.py` runs many paired patches (dotted keys) against one parent.
Base 1051, R 3.2M, seed 5. EHT = wall of all attempts / finishes.

## Decomposition (h per finish)

| | WR+2m (160.4 h) | WR (202.8 h) |
|---|---:|---:|
| starter rejects (73.7% of attempts): off-timer reset/boot | 25.3 | 32.1 |
| starter rejects: on-timer part (intro to stats screen) | 15.3 | 19.4 |
| boot/pre-press off-timer of attempts that passed the starter | 2.8 | 3.5 |
| deaths / abandons after the starter (25.7% of attempts; risk lane + `fatal`) | 111.6 | 138.8 |
| resets at splits (pace policy) | 4.8 | 9.1 |
| the finishing attempts themselves | 3.4 | 3.4 |

* Starter phase total 43.4 h (27%). Deaths 70%. **Pace resets are 3% (WR+2m) / 4.5% (WR) of EHT**, so no pace policy can give more than a couple of percent; measured on 1053 (R 1.6M seed 7201, WR+2m): all reset-table cutoffs +60/+120/+240 s +0.1/+0.1/+0.25 h (n.s.), -60 +1.8, -120 +11.7; early splits (brock..surge) -90 +4.0, +90 0.0; late splits -90 +0.7, +90 +0.07. Flat above the table, costly below: no gain from looser or tighter.
* Where attempts end (share of all attempts that die/fail in the window, of those entering): starter 73.7%; Starter..Brock 49% (forest-trainers 15%, pewter-gym-trainer 9%, Brock 33%); Misty window 42% (Miguel 18%, Misty 15%); Sabrina window 39% (rival-silph 25%, Sabrina 18%); Blaine/Sevii window 22% (Paxton 18%); indigo-heal window 39% (rival-route22 27%, dex-fill 12%); hall-of-fame-1 window (Bruno 25%, Lance 31%, Champion 20%) 62%; round 2 40% / 42%.
* Elasticity: halving the failure rate of window w multiplies EHT by (cum + x(total-cum))/(x total): starter x0.82, Starter..Brock 0.83, Misty 0.84, Blaine/Sevii 0.89, Sabrina 0.77, indigo-heal 0.76, hall-of-fame-1 0.55, round 2 0.73. The post-door share of *time* is tiny (2.6 h, 1.6%, on-timer), but the post-door *failure* multiplies every hour spent before it: 38% of door arrivals fail before HoF-1 ... and 62% of those entering HoF-1 window fail; those rows are the flat `fatal` constants (Bruno .187, Lance .269) and engine deaths, hence not tactic-responsive except by the League-tactics work of the other agent.
* `frlg-human attr --bysegment` lists every segment; the top per-segment halve ratios are rival-route22 .85, Lance .82, Bruno .86, rival-silph .87, Brock .90, Sabrina .91, Paxton .91.

## Result: plan `1053-spe25` (CURRENT)

`manip.min_spe` was not in any descent knob set (`manipknobs.py` covers min_spa/hp/def/atk/spd and the quiet tier). Single knob on 1051 (R 800k seed 7001, + = worse): spe 0 +98.6, 18 +12.0, 19 +6.6, 21 -2.7 (n.s.), 23 -6.9, 25 -7.3, 27 +0.3, 29 +22.0, 31 +157. A joint grid (spe/spa/hp/spd) puts a plateau at spe 24-26, spa 28-29, hp 14-17, spd 15-16 at -10..-16 h; best (25,29,15,15) -15.9. Confirmed at R 3.2M on fresh seeds 7101/7102, paired vs 1051: WR+2m 147.8 vs 160.1 (-12.3 h, CI [-15.5,-9.4]) and 146.4 vs 161.2 (-14.8 [-18.1,-11.6]); WR 187.1 vs 202.2 (-15.1 [-19.5,-11.0]) and 186.1 vs 204.1 (-18.1 [-22.5,-13.4]). (25,29,15,16) and (24,29,15,15) tie. Rungs, rules (Rash/Mild, reset time, no_salvage_until, ball_batch 20, no Brock potions) unchanged; the reset_table was not refit. Mechanism in the model: a stricter table costs idle frames on the timer (`mean_idle_frames`, 1/(REACHABLE*rho)) but the kept starter is stronger in every pre-door fight (deaths are 70% of EHT).
Caveat: the benefit rests on the model's `strength`/battle sim using the starter's real Spe/SpA/HP IVs; the idle cost model is doc 4.2 (dials 1). If the real table has 2 dials the idle term is much smaller and the optimum moves to even stricter minima.

## Refuted (R 800k, seed 7001, vs 1051, WR+2m, + = worse)

* Extra Pokemon Center stops before pewter-gym-trainer / mt-moon-miguel / route3-trainers / rock-tunnel-trainers / rival-cerulean / rival-tower / rival-silph / rival-route22 / hideout-grunts / tower-rockets: +0.1..+0.5 each (pure cost; the risk lane gains nothing). risk_potions 8 +0.2, 4 +1.1.
* Early-game grinds (wild exp): after pewter-gym-trainer 3/6 battles +19/+35, forest-trainers +32, cerulean-gym-trainer 3/6 +5/+19, mt-moon-miguel +10, route25-trainers +22, rival-ss-anne +24: all worse.
* min_spd 12 +1.6, 16 -1.1, 0 +10.9 (n.s./worse); min_spa 31 + hp 14 +8.3; spa 29 + hp 18 -1.7; hp 20 +1.5.
* Pace table shifts: see above, no gain.

## Next

* Refit `reset_table` on 1053 (>= 2000 finishes in training, see optstop-noise rule) and rerun the descent from 1053 with min_spe in its knob set (`manipknobs.py`): the plateau is flat to +-1 in each IV so expect <1%.
* Starter reject time (25% of EHT): a miss is only seen at the stats screen in the model; an earlier tell (title/naming press visible on screen) would cut the 15 h on-timer share of rejects. No evidence for it in the committed transcripts: ask the host to check the stream video.
* Biggest elasticities left are the flat post-door `fatal` rows and rival-route22/rival-silph deaths; tactic knobs for those were descended (279/280 candidates), so gains must come from new mechanics (e.g. item/level changes upstream) rather than more knob tuning.
