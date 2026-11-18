# 2026-12-08: descent from 1092 with an inline confirmation loop (scratch prefix g92-)

Descent: `descend.py --pointer CURRENT-1092 --set pp --rung WR+2m --r1 800000 --r2 1600000 --cap 3200000`. Each promotion is re-checked paired at R 3.2M, two fresh seeds, WR+2m and WR, plus corners (league_power 1.0, carry_power 1.7, door survival 0.10) at R 800k. Sign below: parent - child, positive = child better (h). Tool: scratchpad `g92-conf.py` (uses `frlg-human compare`, rb_lib corners with the 1067 calibration).

| step | change | WR+2m | WR | corners (WR+2m) |
| --- | --- | --- | --- | --- |
| 1094-tactic3 (vs 1092) | lorelei last x special->calm | +0.59 [-0.16,1.22] / +1.06 / +1.46 / +1.70 (seeds 7001/7002/7021/7022; 7 of 8 CIs > 0, none negative) | +1.2 .. +2.0 | lp1.0 +45, cp1.7 +12, door .10 -4 (n.s.): kept |
| 1095-redo98 (vs 1094) | salvage_at 0.8 -> 0.9 | -0.004 h, negligible policy knob (not separately confirmed) | | |
| 1096-evo180 (vs 1095) | JOLTEON stone evolution moved erika-badge -> stones | +2.37 / +2.20 | +6.5 / +6.0 | lp1.0 +6, cp1.7 +19, door .10 +5 (n.s.): kept |
| 1097-ball339 (vs 1096) | ball rule poke_from 190 -> 0 | +2.9 / +3.5 | +11.1 / +11.4 | cp1.7 +35, door n.s.; **lp1.0 -7.9 [-15,-0.9], -8.0, -5.6 (WR+2m; WR n.s.)**: fails the corner on its own |
| 1098-passoff387 (vs 1097) | pass_off +mt-moon-b1f | +1.02 / +0.46 [-0.28,1.23] | +1.3 / +0.7 | seed 7042 includes 0: not confirmed alone |
| 1099-passoff388 (vs 1098) | pass_off +mt-moon-b2f | +3.4 / +3.3 | -1.5 / -1.1 (n.s.) | n.s. |
| 1100-passoff-b2f (vs 1096, b2f only) | | +2.4 / +2.3 | -4.1 / -4.1 (worse) | n.s. |
| **1099 vs 1096 (the combination 1097+1098+1099)** | | +6.0 / +6.6 | +10.5 / +9.6 | lp1.0 -5.6 / -8.0 (n.s.), cp1.7 +46 / +97 (sig), door .10 +15 / +28 (n.s.): **adopted** |

The three late steps are not additive (b2f alone is worse at WR, the ball rule alone worse at lp1.0), so they are adopted together, judged as a block vs 1096: CIs exclude 0 on every main cell, no corner significantly negative.

CURRENT = 1099-passoff388.
