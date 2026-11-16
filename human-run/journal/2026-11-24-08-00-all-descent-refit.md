# 2026-11-24: `descend.py --set all` pass 0 on 1019, window refit, rechecks, robustness grid

Base `1019-guard-mew2x`. Early (132) and earlymid (303) descents had promoted nothing. `descend.py --set all --rung WR+2m --r1 3200000 --r2 3200000` (FRLG_MINGAIN 0.01, evidence guards on, 660 candidates, about 45 s each) ran pass 0 and was stopped at the start of pass 1 on the coordinator's request (windows were fitted to the old plan).

## Pass 0: 9 promotions (each confirmed on seed 2, R 3.2M, WR+2m, diffs seed 1 / seed 2)
1020-evo65 drop Ninetales stone evolution at `stones` (-13.7 / -17.0 h); 1021-evo169 Ninetales via Fire Stone after victory-road (-15.0 / -10.2); 1022-ball221 ball rule poke_from 190, ultra_below 0 (-17.7 / -28.2); 1023-hunt229 Route 10 land hunt after route10-north, stop after 2 (-16.2 / -13.9); 1024-xbuy578 xbuys[0] attack 4->0 (-29.2 / -27.0); 1025-xbuy581 xbuys[0] special 16->12 (-14.6 / -18.8); 1026-xbuy581 special 12->8 (-18.7 / -12.7); 1027-xbuy584 xbuys[0] heals 2->0 (-11.3 / -13.8); 1028-xbuy587 xbuys[1] speed 5->1 (-5.8 / -6.2).
1028: WR+2m 255.5 / 257.3 h (fresh seeds 51/52, R 3.2M), WR 411.0 / 418.9 h. (1019 was 392.6 h at seed 11.) Pass 1 not run.

## Item 2: risk windows
On 1028 (old windows) all eight windows sat inside the runner CIs at R 800k seed 2 (forest-brock .513 in [.29, .53], brock-misty .539, ... gio3-door .624); model survival starter -> indigo-heal .072 against the runner's .040. `fit_lane.py` (R 800k seed 1) refit the six lane windows to .988 / 2.609 / 1.452 / .781 / 1.433 / 2.371 (old .985 / 3.104 / 1.451 / .846 / 1.917 / 2.390). Plan **1029-refit-lane**: windows inside the CIs, cumulative .044 vs runner .040. The refit costs +180.5 h at WR+2m (1028 257.8 h -> 438.3 h, seed 11 R 3.2M, CI [165, 196]). It is a data-fit (the late windows have 3-10 attempts at risk each), not chosen for EHT; the cost is the honest price of the runner's evidence. The same windows on 1019 give 850.4 h (vs its 392.6 h), so at equal windows the descent halved EHT. Post-door: door -> Hall of Fame 1 26.5%, -> Hall of Fame 2 4.8% (WR rung, R 3.2M seed 31), inside the joint S posterior (90% [.038, .379]); `post_joint.py` not re-run.

## Item 3: the two borderline League accepts (R 6.4M, seeds 41/42, reverting on 1029)
Revert Lorelei `heal_below` 0 (lg2 accept 1): +10.8 h CI [5.1, 16.3] / +13.0 h [6.6, 18.9]. Revert Lorelei-2 `+special` (accept 4): +15.7 h [10.6, 20.8] / +17.3 h [12.2, 23.0]. Both confirmed, both kept. 1029 itself: 424.9 / 424.4 h at WR+2m.

## Item 4: Seafoam B3F steps x0.6 ("1020-fillsteps0")
That plan exists only on another worktree's branch (commit 104e530, a different descent; its name collides with 1020-evo65 here, so it is not carried in this tree). Re-tested on 1029 under the proper protocol (WR+2m, R 3.2M, 1% bar, two seeds, fill[0] 120 -> 72): -3.1 h CI [-12.0, 5.1] and -0.3 h CI [-7.8, 7.6]. Refuted, nothing promoted.

## Item 5: robustness grid
`human-run/data/robust-grid-1029-wr2m.json`, `sens.py grid` with SENS_BASE 1029, WR+2m, R 1.6M seed 11; each plan with its own windows. Primary: 1029 449 h, 1028 269 h, 1019 407 h. 1029 over the grid: 86 h (S 1.0) to 948 h (S 0.1); slow 0 / 2.4 360 / 579; trainer_fit .76 332, .86 34 029, .91 no finishes; mistake 0 / 1 359 / 1366; mp 1.8 847; lane x0.85 / x1.15 981 / 260; cp 1.7 2599; gap 0 / 12 637 / 422; quiet off 543. 1028 (old windows) is lower everywhere except `quiet off` and ties; its mistake 1.0 and cp 1.7 are 41 527 h and no finishes (fragile). With equal (refit) windows 1029 beats 1019 by about 2x (438 vs 850 h at R 3.2M). EHT range on 1029 across the stand-in intervals about 260-850 h (outliers 2600 h at carry_power 1.7).

## Item 6: fresh-seed confirmation of 1029 (R 3.2M, seeds 51/52)
WR+2m 419.5 h (1831 finishes), 407.4 h (1883); WR 595.2 h (1229), 582.3 h (1254).

## Next
Restart pass 1 (`descend.py --set all`) from 1029: the 9 promotions were all chosen against the old windows, so some may reverse under the refit windows.
