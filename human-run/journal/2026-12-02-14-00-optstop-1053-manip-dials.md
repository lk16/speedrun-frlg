# 2026-12-02: optstop refit on 1053, manip-knob descent under 1 and 2 dials

Tool: `human-run/tools/manipgrid.py PARENT DIALS RUNG R SEED PREFIX` (coordinate descent over min_spe/hp/spd/spa, quiet_max_hits, s_reset; p_hit fixed; Rash/Mild; `dials` is set, not searched). The full 1051 descent runs elsewhere; this does not duplicate it.

## 1. reset_table refit on 1053 (noise rules: train R 3M, eval 1.6M common seed 8801, best-iterate guard)
Training sets 2632 (WR+2m) / 2053 (WR) finishes. Eval EHT per iterate, start table first: WR+2m 146.53, 146.31, 146.30, 146.30, 146.30; WR 187.05, 186.91, 186.83, 186.80, 186.83. Table moves <= 70 s per split (brock 1080, indigo-heal 9444, hof-1 10309). Paired vs 1053, R 3.2M: WR+2m seed 8901 -0.14 h (CI [-0.33,+0.06]), seed 8902 -0.10 [-0.31,+0.04]; WR 8901 -0.13 [-0.40,+0.15], 8902 -0.08 [-0.34,+0.14]. About 0.1% and CI includes 0 on all four: **not adopted**; the inherited table is already at the optimum (consistent with pace resets being 3-4.5% of EHT).

## 2. Manip knobs, 1053 base (R 800k, seed 7301, WR+2m; moves taken if CI upper bound < 0; one pass, second pass cut short)
Dials 1 (doc 4.2 as in the plan): spe 25 stays (22 +2.8, 24 +0.5, 26 -0.7, 28 +4.4, 30 +29), hp 15 stays (12 +0.4, 14 +0.4, 16 -1.5, 18 -4.2, 20 -4.6, all CI contain 0), spd 15 stays (16/18/20 -0.6/-1.0/-0.1 n.s.), spa 29 stays (28 +3.6, 30 +7.4, 31 +28). quiet_max_hits 5 +0.01, 7 -0.1, 8 -2.7 (CI hi -0.07), quiet 8 confirmed below. s_reset 108 -2.5, 148 +5.0: it is a calibration (data-fitted 128), taken here by the mechanical rule but not a plan choice; every s_reset result is sensitivity only (about 0.12 h per second).
Dials 2: min_spe 26 -3.3, 28 -2.6 more, 30 -1.4 n.s.; hp 16 -3.3, 20 -3.6 (n.s. vs 16); spd 18 -1.9, 20 -4.4; spa 29 stays (28 +5.4, 30 +0.2, 31 -1.1 n.s.). Best: spa 29, spe 28, hp 16, spd 20 (about 130 h vs 141 h for the 1053 tier at dials 2).

## 3. One dial vs two dials, paired R 3.2M, fresh seeds (a = reference, WR+2m h unless said)
Idle-cost model: rho(1053 tier) = 4.6e-4. One dial: mean idle 1/(0.933 rho) = 2300 frames (38 s); two dials: sqrt(pi/(2*0.933*rho)) = 60 frames (1 s). The tier's price is on the timer only; deaths are 70% of EHT.

| comparison | seed | WR+2m | WR |
|---|---|---|---|
| dials 2: tier (29,28,16,20) vs 1053 tier | 9101 | 140.8 -> 130.2 (-10.7, CI [-12.8,-8.2]) | 174.4 -> 160.6 (-13.8 [-16.9,-10.8]) |
| same | 9102 | 135.3 -> 125.7 (-9.7 [-12.0,-7.6]) | 165.0 -> 152.7 (-12.3 [-15.6,-9.5]) |
| dials 2: milder tier (spe 26, hp 16, spd 18) | 9101/9102 | -6.1 / -6.4 | |
| dials 1: the strict tier (29,28,16,20) vs 1053 tier | 9101 | 151.7 -> 162.7 (+11.1 worse [7.9,14.0]) | 194.6 -> 216.7 (+22.1 [17.2,26.9]) |
| same | 9102 | 144.4 -> 154.0 (+9.6 [6.5,12.5]) | 182.8 -> 202.3 (+19.5 [15.1,24.6]) |
| dials 1: quiet_max_hits 8 vs 6 | 9101/9102 | 151.7 -> 148.7 (-2.9 [-4.3,-1.6]); 144.4 -> 142.5 (-1.9 [-3.3,-0.6]) | |

Reading: the optimum does shift with the number of dials. With two dials the best tier is strictly stronger (spe 28, hp 16, spd 20, about -10 h at WR+2m, -13 h at WR, i.e. -7..-8%) and the same tier is a 7-12% loss if the table really has one dial (+10..+22 h). The 1053 tier is the minimax-ish pick: it loses 10/13 h against the 2-dial optimum if there are two dials, and nothing if there is one. EHT itself is 7-8% lower with two dials at the same tier (135-141 vs 144-152). Evidence for the number of dials is still absent (doc 3.1 says the naming seed is chaotic per N: needs an emulator-built table of naming seeds); the starter scans cannot tell. So 1053's one-dial tier stays; the strict tier is the right plan *if* the runner's table has a naming dial.

quiet_max_hits 8 (up from 6) is -2..-3 h at dials 1; check on 1054 below.
