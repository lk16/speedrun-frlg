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

Second block, on 1099:

| step | change | WR+2m | WR | corners |
| --- | --- | --- | --- | --- |
| 1101-order434 | swap fill[5] Seafoam B4F with fill[6] | +1.06 / +0.79 | +1.25 / +0.97 | cp1.7 +3.3 sig, rest n.s.: kept |
| 1102-order437 | swap fill[8] Route 23 with fill[9] | +0.30 / +0.17 [-0.01,0.35] | n.s. | seed 7092 includes 0: not confirmed (rides along in 1103, effect < 0.3 h) |
| 1103-dropfill442 | drop fill[3] Power Plant | +0.78 / +0.58 | +1.36 / +1.54 | lp1.0 +1.5, cp1.7 +1.9, door .10 +2.1: all n.s. or positive: kept |
| **1103 vs 1099** | | +1.69 / +2.01 | +2.99 / +2.96 | lp1.0 +3.6 sig, cp1.7 +3.6 sig, door n.s. +: **adopted** |

CURRENT after block 2 = 1103-dropfill442.

Third block, on 1103 (the descent was restarted from 1103 with `--untried`, dropping 1104-1106):

| step | change | WR+2m | WR | corners |
| --- | --- | --- | --- | --- |
| 1104-addsite531 | add Seafoam B1F fill site | +0.40 / +0.25 [-0.02,0.58] | +0.45 / +0.48 | seed 7122 includes 0: not confirmed |
| 1105-tactic592 | koga +x special | +7.7 / +8.3 | +10.5 / +10.1 | lp1.0 +27, door .10 +38, **cp1.7 -47.8 [-61,-36] (WR -88.8): fails the corner**: rejected (a hedge candidate: wins everywhere but carry_power 1.7) |
| 1106-tactic616 | rival-tower +x special (rode on 1105) | not tested on 1103 | | |
| 1107-evo10 | PARASECT via level after koga-badge | +3.4 / +4.2 | +5.6 / +5.7 | lp1.0 +9.5, cp1.7 +5.1, door .10 +17.2, all CIs > 0: **adopted** |

CURRENT = 1107-evo10.
