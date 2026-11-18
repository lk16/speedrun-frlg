# 2026-12-08: descent from 1092 with an inline confirmation loop (scratch prefix g92-)

Descent: `descend.py --pointer CURRENT-1092 --set pp --rung WR+2m --r1 800000 --r2 1600000 --cap 3200000`. Each promotion is re-checked paired at R 3.2M, two fresh seeds, WR+2m and WR, plus corners (league_power 1.0, carry_power 1.7, door survival 0.10) at R 800k. Sign below: parent - child, positive = child better (h).

| step | change | WR+2m | WR | corners (WR+2m) |
| --- | --- | --- | --- | --- |
| 1094-tactic3 (vs 1092) | lorelei last x special->calm | +0.59 [-0.16,1.22] / +1.06 / +1.46 / +1.70 (seeds 7001/7002/7021/7022; 7 of 8 CIs > 0, none negative) | +1.2 .. +2.0 | lp1.0 +45, cp1.7 +12, door .10 -4 (n.s.): kept |
| 1095-redo98 (vs 1094) | salvage_at 0.8 -> 0.9 | -0.004 h, negligible policy knob (not separately confirmed) | | |
| 1096-evo180 (vs 1095) | JOLTEON stone evolution moved erika-badge -> stones | +2.37 [1.93,2.80] / +2.20 (7011/7012) | +6.5 / +6.0 | lp1.0 +6, cp1.7 +19, door .10 +5 (n.s.): kept |

CURRENT = 1096-evo180 (= 1092 + the three rows).
