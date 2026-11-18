# 2026-12-08 (evening): evidence-fitted League constants adopted as the model; League re-descent; plan 1122-lgdescent

Scratch prefix `lg-`. Tools: `descend.py --set league` (new: League X queues / sacs / heals / X buys / Mewtwo teaches / Calm Mind tokens only), `fit_post_evidence.py` (Beta clamp fix for fights with zero prior hazard).

## 1. MODEL CHANGE (coordinator decision): plan 1113-lgmodel = 1107-evo10 with league_power 1.0, mewtwo_power 1.3, scarce_factor 1.0
carry_power kept at 2.0 (calibration prefers <= 1.7 weakly, but at 1.7 the lane cannot express Sabrina and EHT is x1.8-2.5; documented, not changed). Refits:
* lane windows: `fit_lane.py --check` passes unchanged (every runner CI holds; the windows do not depend on the League constants); model starter -> door survival 0.107 vs runner 0.056 as before.
* post-door `fatal` rows (`fit_post_evidence.py`, now 5 fought door attempts / 2 finishes): Lorelei, Agatha, Champion, Champion-2 0 (the engine alone is already harsher than the data: Champion engine death .29), Bruno .194, Lance .232, chain .091-.133, round 2 .032-.056. Model D (door to finish) .22.
* reset_table refit on both rungs (train R 3M, eval 1.6M, 6 iterations; stable to 0.05%): paired vs the inherited table R 3.2M seed 7101 -0.35 h (WR+2m) / -1.30 h (WR), CI excludes 0; adopted as part of the refit.
Result: **1113: WR+2m 123.4 h, WR 166.1 h** (R 3.2M). 1107 at the old constants was 84.5 / 108.8. (An earlier 1112-lgmodel on 1103 gave 129.0 / 175.1; kept as `1113-lgmodel-old1103.md`, not CURRENT.)

## 2. League re-descent from 1113 (`--set league`, rung WR+2m, R1 800k, R2 1.6M, 217 candidates, pass 0 stopped at about lance-2 by the wrap-up; later candidates untried)
Confirmed, paired R 3.2M, two fresh seeds (7201/7202), parent - child, positive = child better (h):
| step | WR+2m | WR | corners WR+2m (R 800k) |
| --- | --- | --- | --- |
| 1114 agatha -x speed | +1.02 [.51,1.64] / +0.63 [.10,1.20] | +1.56 / +0.57 [-.28,1.41] | lp1.4 0, mp ~+0.4, cp1.7 +16 [-2,37], door -1.8: kept (small) |
| 1115 champion +x special (guard, speed, special, special) | **+23.0 / +23.6** | **+35.9 / +34.9** | lp1.4 **-16.0** (the known flip), others see journal 12-08-14 and the final block below: kept |
| 1116 league_preheal .85 -> .5 | +2.77 / +1.75 | +3.42 / +1.60 [-.41,3.58] | lp1.4 -1.4 [-3.1,.4], mp1.15 +.4: kept |
| 1120 lance first x special -> calm | screen only: +1.5 (R1.6M), confirm +0.93 [-1.86,-0.006] (R1.6M seed 2); NOT confirmed at R 3.2M | | in the final block |
| 1117 / 1118 / 1119 Mewtwo teaches (Water Pulse, Ice Beam, Thunderbolt) | effect 0.001-0.002 h (CI excludes 0 only because it is deterministic): **dropped**, not real | | |
The WR+2m rung is only the screen; 1115 is the whole story (the 4-item Champion queue predicted by the calibration journal). Nothing was refuted by a corner by more than the lp 1.4 flip of 1115 (-16 h on 62 h) and cp 1.7 of the late steps.

## 3. Re-checks of the old-model knob steps on the new model (variant = F with the step undone / added; F = 1113 + 1114/1115/1116/1120; R 1.6M, seeds 7301/7302, both rungs, corners R 800k; parent F - variant, negative = the step helps)
| step | WR+2m | WR | corners | verdict |
| --- | --- | --- | --- | --- |
| **1096** (Jolteon Thunder Stone evolution erika-badge -> stones), undone | **+1.21 / +1.80** (undo better) | +1.20 / +2.13 | lp1.4 +1.6, mp +2.8, cp1.7 +17 [6,30], door +7.8: undo better in **every** cell | **flips: reverted** |
| 1097 ball rule (poke_from 190 -> 0), undone | -3.7 / -4.2 (undo worse) | not run (stopped) | | survives |
| **1099 block** (ball rule + pass-off Mt Moon B1F/B2F), undone | -7.3 / -8.0 | -15.4 / -14.9 | lp1.4 -8.6, mp -6.3 / -8.0, cp1.7 -51 [-98,-6], door -17.9: worse in every cell | **survives (the ball rule is robust here; at lp 1.0 the old corner failure is gone)** |
| 1107 (Parasect level evolution after Koga), undone | -3.7 / -4.3 | -6.9 / -6.6 | lp1.4 -2.8, mp -4.1 / -3.6, cp1.7 -15.5 [-29.5,-1.6], door -10.7: worse in every cell | **survives** |
| **1105 Koga +1 X special** (variant added) | **+8.1 / +7.1** (variant better) | +9.7 / +8.2 | lp1.4 +6.1, mp +6.2 / +5.4, door.10 +20.5, **cp1.7 -42 [-83, +0.9]** | **adopted**: significantly better in 4 of 5 corners; the cp 1.7 corner (EHT 430 h there) is a trade-off, CI touches 0 |
Not re-run: 1098 alone (not confirmed before), 1101 / 1102 / 1103 fill reorderings (effects < 1 h on the old model), the lp 1.0 / cp 1.7 corners for the rest.

## 4. Plan 1122-lgdescent = 1113 + 1114 + 1115 + 1116 + 1120 - 1096 + 1105 (no Mewtwo teaches)
Paired against 1113, R 3.2M, seeds 7401 / 7402: **WR+2m 84.6 / 85.1 h vs 122.4 / 122.9 (-37.8 / -37.8, CI excludes 0); WR 112.2 / 111.0 h vs 167.1 / 166.7 (-54.9 / -55.8)**.
Corners (WR+2m, R 800k, 1122 vs 1113): lp 1.4 **+6.7 h worse** (69.3 vs 62.7, the old stand-in where the Champion queue loses), mp 1.15 / 1.45 -36.6 / -35.8 better, door survival .10 -93.9 better, carry_power 1.7 -205 better (456 vs 661).
CURRENT = 1122-lgdescent. The reset_table is 1113's (fitted before the tactic changes; refit not re-run: NEXT).
The honest EHT is therefore about **85 h at WR+2m and 111 h at WR** at the fitted constants (door survival pinned by the fatal rows, D ~ .22); at lp 1.4 it is 69 h. The evidence-weighted interval (rb_mc, ROBUSTNESS 14) was not recomputed: too expensive for the wrap-up. The earlier 1103 interval (68% [46,203] / 95% [28,584] at WR+2m) still brackets it; the cp 1.7 corner (456 h) is the upper tail that dominates.

## NEXT (most valuable searches, in order)
1. Finish the league pass: 217 candidates, stopped around lance-2; not run: remaining lance-2 / champion-2 / Round 2 candidates, xbuys ([i] special/speed/guard/accuracy +-), league_heals, setup_min_turns, heal_below for all League fights, more Mewtwo teaches under larger Champion-2 power. Then a second pass from 1122 (the 1115 step moves the optimum of the others).
2. Confirm 1120 (Lance first x) at R 3.2M on two seeds, both rungs, with corners (only screened + one R 1.6M confirm); same for 1114 at WR seed 7202 (CI includes 0).
3. Refit the reset_table on 1122 (both rungs; the table in 1122 is 1113's), re-run `fit_post_evidence.py` (the Champion engine death changed: Champion 1 now has the 4-item queue), `fit_lane.py --check`.
4. Sets not run under the new model: `--set all`, `earlymid`, `pp` picks, `dex`, fill/hunt orders (1101-1103 and 1112-evo48 GOLBAT evolution, screened -0.7 h only), Koga / Sabrina / Giovanni X queues (the +1 Koga special was a corner trade-off; Sabrina / Giovanni / Blaine untested), and the 1098 pass-off alone.
5. Uncertainty items that move EHT most: carry_power (x1.8-2.5 at 1.7; the double Torrent at Koga / Sabrina / Giovanni, `free_torrent` x1.5 times `power` .667 x cp 2.0, is a candidate mechanism: test cp 1.7 with the Torrent factor removed once), door-to-finish D (now 2/5 finishers), mewtwo_power (a Mewtwo held-item / stat sighting would pin it), league_power 1.0 vs 1.1 (1.1 gives -18 h per the calibration journal).
6. Evidence-weighted interval for 1122 (rb_mc with the new League stand-ins posterior, ROBUSTNESS 14), and a rank-flip grid (rb_var) for 1115, 1105 and 1096-undone at 8-12 corners at R 500k.
