# ROBUSTNESS: does the plan ranking survive the unidentified calibrations?

CURRENT = `608-evo581-habit`. Absolute EHT rests on constants the data cannot pin (post-door survival S, `slow` probabilities, `win_cv`,
`trainer_fit`, `mistake_rate`, the starter press hit rate, the quiet-seed saving, the off-timer wall costs). This file (a) tightens what the
committed data can check, (b) re-runs the top plans and some older ones with every calibration moved across its plausible range, (c) names a
recommended plan. Tools: `tools/fit_starters.py`, `tools/sens.py`, `tools/sens_table.py`, `tools/sens_cands.py`, `tools/sens_paired.py`; raw grids
`data/robust-*.json`. Journal: `journal/2026-11-15-14-00-robustness.md`. Every number is a score of the model, not a prediction.

## 1. What could be checked, and what it says

| constant | evidence | verdict |
| --- | --- | --- |
| press hit rate `p_hit` 0.692 | `video-analysis/data/starters/*.jsonl`, 9 usable sessions (11-05 box static, 11-09 RETARGETED left out): 85 kept / 325 resolved attempts = 26.2%, p = rate^(1/3) = **0.640** (Wilson 0.601-0.678). The 4 sessions of the old fit alone: 40/125 = 32% (p 0.684), so 0.692 was the high end. Session rates 16-44% (p 0.54-0.76), chi2 7.9 on 8 df: no overdispersion beyond binomial. | **moved to 0.640** (plan 800); drift regime 0.55 and good regime 0.76 are the sensitivity ends |
| hard-reset wall cost (`s_boot_title` 45 + `s_reset` 100) | start-to-start of a rejected attempt, consecutive finals of 11-11 and 11-12: n = 45, median 302 s, trimmed mean 296 s (3 breaks > 600 s dropped), mean 329. Model: 268 s. | model 10% short; `s_reset` 100 -> **128** in plan 800 (x1.3 on the off-timer part is the sensitivity level) |
| quiet-seed saving ("1:29 vs ~6 s") | `frlg-human splits` on 608, Brock split, no resets: quiet seed on 14:21, off **15:34**: the saving is **73 s** (the 1:29 was a 2026-11-14 state of the model). The video's 6 s is 14:14 (manip) against 14:20 (his *previous record*, STRATEGY 2, T09 3:46:30): a best-case run against a typical one. The same session says "always one minute ahead" (T09 3:44:34), "saves a minute" (T07). Against the model's no-manip Brock window (mean 15:34, sd ~50 s) 14:20 is a ~1.4 sd luck draw. | the 73 s is consistent with "a minute"; the 6 s is not a contradiction. Not changed. Sensitivity level: quiet off (saving 0, the lower bound the 6 s could mean). Number of allowed hits 2/4/8 give identical splits (`quiet_keeps_wanted`) |
| `trainer_fit` 0.81 | vs the WR splits (no-reset means, `sens.py tf`): End -29 s at 0.81, +110 s at 0.86, +248 s at 0.91; E4R1 -120 / +5 / +127 s; Koga -357 / -269 / -184 s. The WR is the best of ~16k attempts; a model whose *mean* run is 6% faster than the record at Koga (0.81) or 2 min ahead at the door is too fast. | evidence favours **0.81-0.86**; 0.76 is excluded as too fast (mean 4 min ahead of the record at the door). Sensitivity 0.76 .. 0.91 |
| post-door S 0.183, `slow`, `win_cv`, `mistake_rate` 0.3 | no new data. S is the residual that makes 15 811 attempts per WR finish; it trades against `slow` (journal 2026-11-14-22-00 sec 4). | unidentified; ranges below |

## 2. Sensitivity table (EHT at WR+2m, h; R = 500 000 per cell, seed 1, paired seeds across plans)

Older plans (492, 452, 386) carry none of the fidelity keys (`fatal`, `slow`, `win_cv`, `trainer_fit`, `league_transit`, ...): each is rebased to
608's model keys for every key it lacks and keeps its own policy (`sens.py rebase`); 556 rebased is bit-identical to 600 and is not listed. Each row
changes one calibration for all plans; **bold** = best of the row; `n/a` = fewer than 50 finishes in 500 000 attempts at WR+2m (EHT beyond the
rung). S rescales the product of the 14 League/post-game survivals; `prefatal` scales the log-survival of the pre-door hazards.

| scenario (primary: S .183, slow 1, win_cv .033, tf .81, mistakes .3, p .692, quiet on, wall 1, prefatal 1) | 608 | 607 | 600 | 492 | 452 | 386 |
|---|---:|---:|---:|---:|---:|---:|
| primary | 120 | **103** | 146 | 219 | 1317 | n/a |
| S 0.10 | 228 | **188** | 283 | 432 | n/a | n/a |
| S 0.40 | 55 | **48** | 67 | 100 | 640 | n/a |
| S 1.0 | 22 | **19** | 26 | 40 | 261 | n/a |
| slow 0 | 103 | **87** | 116 | 163 | 734 | n/a |
| slow 2.4 | 146 | **127** | 183 | 298 | n/a | n/a |
| win_cv 0 | 118 | **101** | 142 | 212 | 1799 | n/a |
| win_cv 0.066 | 127 | **108** | 155 | 225 | 1184 | n/a |
| trainer_fit 0.76 | 106 | **87** | 115 | 163 | 599 | n/a |
| trainer_fit 0.86 | 158 | **142** | 205 | 361 | n/a | n/a |
| trainer_fit 0.91 | 253 | **238** | 403 | 771 | n/a | n/a |
| mistake_rate 0 | 108 | **90** | 119 | 165 | 669 | n/a |
| mistake_rate 1.0 | 236 | **213** | 339 | 797 | n/a | n/a |
| p_hit 0.55 | 150 | **126** | 186 | 281 | n/a | n/a |
| p_hit 0.64 (data) | 131 | **111** | 157 | 240 | 1365 | n/a |
| p_hit 0.76 | 109 | **95** | 133 | 198 | 1159 | n/a |
| quiet seed off | 134 | **120** | 174 | 259 | 2119 | n/a |
| wall costs x1.3 | 123 | **106** | 151 | 227 | 1374 | n/a |
| pre-door hazards x0.5 | 56 | **51** | 67 | 96 | 530 | n/a |
| pre-door hazards x1.5 | 271 | **225** | 334 | 536 | n/a | n/a |
| **worst regret** (EHT / best of the row) | 1.21x | **1.00x** | 1.69x | 3.75x | > 10x | no finish |

Reading:

* **The ranking is invariant**: 607 < 608 < 600 (= 556) < 492 << 452 < 386 in all 20 rows. Plans 452 and 386 never reach the WR+2m rung under the
  honest model (their reset thresholds were tuned where most door arrivals finished); 492 is 1.8-3.4x 608.
* **The size is not**: the same plan moves from 19 h (S = 1) to 271 h (pre-door hazards x1.5); 608 is 22-271 h over the table, a factor 12. The
  absolute EHT is therefore not a prediction. The spreads that matter most, one at a time: pre-door hazards (0.5-1.5x: 56-271 h), mistake_rate (108-236 h), S
  (22-228 h), `trainer_fit` (106-253 h). `win_cv`, `wall`, the quiet seed and `p_hit` inside its data range move 608 by < 12% each.
* **The data-fitted starter phase** (p 0.640 and `s_reset` 128, plan 800, paired seed 31, R = 1.6M): WR+2m 128.9 h vs 119.4 h, diff +9.5 h CI [+7.6, +11.7];
  WR 159.2 h vs 145.9 h, +13.3 h CI [+10.9, +16.0]. Cost of correcting the hit rate: about +8%.

## 3. Is the policy (608's own thresholds) robust?

Reset thresholds (`reset_aggr_at` on every split except Brock and the dex check shifted by -0.15 .. +0.20) over 11 calibrations
(`data/robust-thresholds-wr2m.json`): lowering them is never worse than 608's by more than 1% and raising them costs up to 28% (+0.2). The plan is on the
plateau of "play on, trust the floor" under every calibration; no threshold change is more robust (best worst-case regret 1.00x at -0.05..-0.15 vs 1.01x,
all inside noise). Policy variants (`data/robust-policy-wr2m.json`, 9 scenarios): looser/tighter manip tier (+4..+6%), `league_heals` 14 (0%),
`torrent_heal_below` 0.5 (-2..+2%, noise), `ball_batch` 40 (+6%) are all no better than 608 in any scenario. **`no_salvage_until ""` (plan 607) wins every
scenario (-12..-17%).**

## 4. Recommendation

* **Robust plan: `607-evo581` on paper (best in all 20 scenarios; paired seed 31 R = 1.6M WR+2m 102.8 h vs 119.4 h, -16.6 h CI [-18.7, -14.6]; seed 32, R = 1M:
  -14.1 h CI [-16.8, -11.8]; WR rung seed 31, R = 2.4M: -15.0 h CI [-17.2, -12.7]), but its entire edge is one habit**: it plays on after an early whiteout (before
  the Rival at the S.S. Anne), where the runner resets (STRATEGY 7). 608 is that plan with his habit and loses 12-17% to it in every row. **CURRENT stays
  `608-evo581-habit`**: no plan other than 607 (a behaviour change, not a calibration) is more robust, and nothing in the tables moves 608 below any older plan.
* If the runner can adopt the play-on rule, 607 is the plan; the advantage is a policy gain, not an artefact of a calibration (it is there at S = 0.1 and 1.0,
  at p 0.55 and 0.76, at mistakes 0 and 1).
* Plan `800-recalib-starter` = 608 with the data-fitted starter phase (p 0.640, `s_reset` 128) is the better *model* of 608 (use it for absolute EHT quotes:
  WR+2m ~129 h, WR ~159 h at the primary calibration, still S-dependent); it is not a better plan, same policy.
* What would shrink the spread: the real attempt log (every attempt's last split) to fit S and the pre-door hazards jointly, and the per-fight death rates; a
  hazard that responds to tactics (`fatal` is flat, so the descent can never find a safer Brock). None of the evidence here changes a choice; it only bounds the
  absolute numbers.

## 5. Update 2026-11-15 (evening): S from the door attempts, `slow` checked, thresholds past the door

Journal `journal/2026-11-15-18-00-post-door-evidence.md`; data `video-analysis/data/post-door-evidence.json`; tools `fit_post_evidence.py`, `post_joint.py`, `post_rank.py`.

* **S is now data-bounded, not a free residual**: 4 attempts fought after the door, 1 finished (Wilson 5-70%); joint with the runner's 15 811 attempts per finish (plan 480
  as his policy) S = 0.2, 90% [0.06, 0.5]. Per fight (Beta posterior, prior k = 3): Bruno .18, Lance .25, Lorelei/Agatha .04-.05, Champion .10, Warehouse chain .08-.13,
  round 2 .07-.14, product 0.205. Plan 802 = 801 with this map (CURRENT; WR 141 h, WR+2m ~112 h at the point estimate; 801's S .183 gave 159 h / 130 h).
* The ranking of section 2 survives: 607 < 700 < 608 < 801 < 800 < 600 < 492 in six post-door models (S .06 .. .5) and two seeds. 801 spans 48 h (S .5) to 387 h (S .06).
* Slow events: Safari 3/12 bad, Power Plant 1/8, cave 1 run: inside the model's values, left unchanged; slow off is -14%, Safari p .5 +7%.
* `reset_aggr_at` past the door: no retune (best -0.8%, under the 1% bar; raising it costs up to +3%).

## 6. Update 2026-11-20: the honest-model grid (journal `journal/2026-11-20-10-00-refit-skips-robustness.md`)

CURRENT is `1012-notmdup` (the sections above describe the 608-era model). `data/robust-grid-honest-wr2m.json`, `data/robust-thresholds-honest-wr2m.json`; `sens.py grid` with `SENS_BASE=1012-notmdup`, R 500k, seed 1, WR+2m: 1012 / 983-epot43 / 976-lp22 over 30 scenarios (the old ones plus lane power x.85/1.15, league_power 1.8/2.6, mewtwo_power 1.2/1.8, carry_power 1.7/2.3, level_gap 0/12). Ranking 1012 < 983 < 976 in 27/30; it inverts at trainer_fit .91, mistake 1.0 and carry_power 1.7. Primary 151 / 196 / 285 h; 1012 spans 32 h (S 1.0) to 309 h (S 0.1), outliers up to 748 h (cp 1.7). 1009-heal658 is the more robust variant (cp 1.7 517 h, mistake 1.0 521 h, primary +9.8 h).

## 7. Update 2026-11-21: the stand-ins on the AI-faithful engine (journal `journal/2026-11-21-10-00-engine-audit-standins.md`)

The trainer AI was too gentle in three ways (TRY_TO_FAINT ignored Hyper Beam damage, mons came out in party order instead of `GetMostSuitableMonToSwitchInto`, no `ShouldSwitch`); CURRENT moved from 1012
(155.8 h on the old engine) to **1016-manip131: WR+2m 171.8 h, WR 205.7 h** after a window refit and a descent. EHT against the three stand-ins on 1016 (windows fixed, R 1.6M seed 11, WR+2m):
carry_power 1.7 / 1.8 / 1.9 / **2.0** / 2.2 / 2.4 = 545 / 384 / 242 / **172** / 163 / 160 h (345 h at 1.7 with the windows refit there); league_power 1.8 / 2.0 / **2.2** / 2.4 / 2.6 = 296 / 250 / **172** / 151 / 148 h;
mewtwo_power 1.0 / 1.2 / **1.4** / 1.6 = 586 / 221 / **172** / 156 h. Likelihood intervals of the fits (`tools/fit_lik.py`): league_power 2.0-2.4 (best 2.2), mewtwo_power 1.2-1.6 (best 1.2), carry_power 1.2-1.8 (best 1.4; the plan's 2.0 is 2.8 log units worse):
the model's 95% range at WR+2m is about 150-550 h. The mid-game `carry_power` is Torrent x1.5 times a 1.0-1.2 level gap (the WR walks into the gyms at <= 1/3 HP and 3 levels higher); `mewtwo_power` is about one X Special stage;
`league_power` has no mechanism yet. The grids above predate this and the ranking statements of section 6 were not re-run on the new engine.

## 8. Update 2026-11-22: frame facts and the recounted WR League actions (journal `journal/2026-11-22-10-00-frame-facts.md`)

The WR's League action counts were 1-3 item turns too low (11 / 7 / 6 / 8 / 10 -> 13 / 9 / 7 / 10 / 12; Lance 2 7.5 -> 9). Refit on the recounted values, with the WR's real X queues and Torrent windows (plan `1017-wrfacts`): league_power 1.4 [1.0, 1.6],
mewtwo_power 1.2 [1.0, 1.4] (were 2.2 / 1.4). EHT at WR+2m (R 1.6M seed 11) at the optimum 921 h, over the intervals about 540 h to 14 000 h; 1016's 171.8 h belonged to the old stand-ins. The ranking statements of sections 6-7 were made at 2.2 / 1.4 and are not re-run.

## 9. Update 2026-11-23: League retune (journal `journal/2026-11-23-14-00-league-retune-result.md`)

The trainer AI no longer avoids stat-drop moves into Mist (decomp: it never looked), which makes the WR Champion Guard Spec useful in the engine. With the League queues retuned (plan `1019-guard-mew2x`): WR+2m 375-404 h and WR 616 h (1045 finishes) at R 3.2M, fresh seeds 31/32 (1017: 913 h and 5047 h). Stand-ins unchanged (league_power 1.4, mewtwo_power 1.2); sections 6-8 ranking statements not re-run.

## 10. Update 2026-11-24: `--set all` pass 0 and the window refit (journal `journal/2026-11-24-08-00-all-descent-refit.md`)

Nine promotions on 1019 (evolutions, ball rule, Route 10 hunt, xbuys) gave 1028-xbuy587: WR+2m 256 h, WR 411-419 h (R 3.2M, seeds 51/52). The risk windows refit on 1028 (`fit_lane.py`) keep every window inside the runner CIs (model survival .044 vs runner .040) and cost +180 h: **CURRENT 1029-refit-lane, WR+2m 419.5 / 407.4 h, WR 595.2 / 582.3 h** (R 3.2M, seeds 51/52). On equal windows 1019 is 850 h, so the descent is about 2x. Grid (R 1.6M, seed 11, `data/robust-grid-1029-wr2m.json`): 1029 449 h primary, 86 h (S 1.0) to 948 h (S 0.1), outliers 2600 h (carry_power 1.7) and no finishes at trainer_fit .91; 1028 and 1019 listed in the journal. The two borderline League accepts (Lorelei heal_below 0, Lorelei-2 special) hold at R 6.4M on two seeds (+10.8..+17.3 h if reverted).

Pass 1 of `descend.py --set all` from 1029 (667 candidates): 5 screen accepts, none confirmed on seed 2, no promotion; 1029 is converged on the descent knob sets (early, earlymid, all). The nine pass-0 promotions did not reverse under the refit windows.

## 11. Update 2026-11-25: execution-constant evidence and version (journal `journal/2026-11-25-09-00-exec-constants-version.md`)

mistake_rate 0.162 [0.07, 0.37], trainer_fit 0.729 [0.60, 0.86], win_cv [0, 0.069] fitted from the stream; plan values (0.3, 0.81, 0.033) are inside, not changed. 1029 EHT(WR+2m) over those intervals: median 292 h, 68% [280, 352], 95% [276, 600] (above tf .81 cliff-like: .83 1367 h, .86 40 849 h). FireRed re-descended partially: 1035-dropfill348 vs 1029 -3.2 / -7.0 h (CI spans 0); LG kept.

## 12. Update 2026-11-26: optimal-stopping reset table, the trainer_fit cliff removed (journal `journal/2026-11-26-10-00-optimal-stopping.md`)

CURRENT is `1040-optstop` (= 1029 with `reset_table`, per rung, fitted by `frlg-human optstop`; `tools/optstop_plan.py` builds plans). The cliff of section 11 was the pilot-mean reset formula (cutoffs 100-300 s too tight at Brock..Surge when the mean pace is slower), not the plan. EHT(WR+2m), table refitted at each cell, R 3.2M seed 91, vs the 1029 formula: tf .76 330.6/332.3, .81 409.4/410.9, .85 546.9/595.2, .86 **582.4/51 059** (mistake .3); mistake .16: 312/314, 377/379, 474/525, 510/2074. WR: .81 565/582, .85 852/5576, .86 975/no finish. A table fitted once at tf .81 and used unchanged at .85/.86 gives 548 / 584 h (WR+2m): the table is nearly independent of trainer_fit, so it is a robust policy, not a fit to a calibration. At the plan constants: WR+2m -1.4 / -0.7 h (seeds 92/93; CI touches 0), WR -14.2 / -18.2 h (CI excludes 0), 1040 411.5 / 408.6 h WR+2m, 570.8 / 567.0 h WR.

## 13. Update 2026-11-27: post-door stand-ins on the current engine, Champion-2 length, play-on rule (journal `journal/2026-11-27-10-00-post-door-standins-playon.md`)

The `fatal` rows (S) were fitted on the old gentle engine and never refit after the AI fixes: with them 1040's door-to-finish survival is .068 although the engine alone gives .33 (Champion .20, Champion-2 .45 engine death) and the evidence (1 finish of 4 fighters, joint S about .2) says .09 [.02, .26]. `fit_post_evidence.py` on 1040 sets Champion / Champion-2 / Lorelei / Agatha `fatal` to 0, Bruno .187, Lance .269, chain .098-.149, round 2 .054-.096 (plan `1042-postfit`, MODEL CHANGE, same policy): WR+2m 310 / 317 h and WR 424 / 438 h against 1040's 431 / 427 and 593 / 593 (R 3.2M, seeds 71 / 72). CURRENT stays 1040. Champion-2 length: 12.3 actions vs the WR's 11.5, 3:39 vs 3:42 segment: no change. **Play-on (607 rule) on 1040 and on 1042: not better** (WR+2m +1 / -13 h, WR -8 / -33 h in 1040's favour; refit model WR +16 / +22 h worse for play-on; CIs include or touch 0): refuted, the 607 edge of section 4 does not survive the current model and reset table.

## 14. Update 2026-12-03: evidence-weighted EHT interval for 1057 (journal `journal/2026-12-03-10-00-eht-interval.md`, interim)

96 Latin-hypercube draws of the stand-ins and runner posteriors on 1057 (`tools/rb_mc.py`; data `data/robust-mc-1057-pre.json`), quadratic surface R2 .98, CV .95. **EHT(WR+2m) median 142 h, 68% [69, 332], 95% [39, 865]; EHT(WR) median 173 h, 68% [80, 432], 95% [43, 1196]** (plan point 160 / 221 h). With the door-to-finish prior replaced by data only (2/5) the medians are 75 / 91 h. Variance is dominated by the two hazard posteriors (door-to-finish D, pre-door Q: ln-EHT effect -.60 and -.33 per sd), then trainer_fit (+.23/+.33), carry_power (-.17/-.21), league_power (-.09); hidden-item pickup frames, museum, Persim, win_cv, mistake_rate are < 5% each. The "model .155 vs posterior .23" door-to-finish gap of journal 12-02 is a pace-reset effect: the survival-only value is .244, at the posterior. Evidence: the WR picks up the Rocket Hideout B1F PP Up in ~2.5 s (video 59:16, 1 s frames) and carries PP UP x2 by 1:03 and Max Elixir + 2 Elixirs by 2:24: the PP Up plan is the WR's, and 450 f is about 3x too pessimistic for on-route pickups. Rank flips (21 variants x 8-12 corners, R 500k; confirmation R 3.2M, seeds 5101/5102): no structural choice of 1057 flips (PP Up count 2-3 equal, 4th worse, none +23-39%; candy 2, Torrent-to-Koga, special 7 all worse everywhere). The one hedge is special 11 + heal_below .5 (`combo_hs`): -0.6 h to -2.6 h (n.s.) at the plan constants, -29 h (WR+2m) / -45 h (WR) at league_power 1.0, -24 / -46 h at carry_power 1.7 (CIs exclude 0), never worse; x0.96 over the whole prior (74/96 draws better); interval WR+2m 135 h [67, 312], WR 164 h [77, 405]. Not adopted: the point-estimate gain is not significant. Data `data/robust-var*-1057-wr2m.json`, `robust-conf-hs-1057.json`, `robust-mc-1057hs.json`.

## 15. Update 2026-12-04: plan 1058 (hidden Rare Candy x2, Calcium x2, Nugget sale) against the stand-ins (journal `journal/2026-12-04-10-00-structure-items-dims.md`)

CURRENT is `1058-hidden-candy-calcium-nugget` (1057 + two hidden candies on Routes 9 / 17, two Calciums in the Hideout B4F / Silph 7F, the Route 24 Nugget sold at Celadon, lane windows refit): WR+2m 159.8 -> 117.7 h and 157.9 -> 118.7 h, WR 222.6 -> 166.0 h and 221.4 -> 166.8 h (R 3.2M, seeds 5401 / 5402, paired -42 / -39 and -57 / -55 h, CIs exclude 0).
Stand-in corners, the same three changes against 1057 in the same corner (R 800k, WR+2m, seed 4403; difference in h, never positive): carry_power 1.7 -235, 2.3 -29; league_power 1.0 -198, 1.8 -9, 2.2 -167; mewtwo_power 1.0 -53, 1.5 -32; trainer_fit .76 -26, .86 -40; mistake_rate .16 -26, .4 -36; level_gap 0 -53, 12 -31.
The gain is larger where the League stand-ins are weaker (the extra levels and SpA matter most when the carry is short) and smallest at league_power 1.8 (-9). The guessed pickup frames (300-450 each, sale 350) can be 2x / 3x without losing it (-37 / -25 h) and break even at about 4x (+15 h at 5x).
The interval of section 14 was drawn on 1057 and is not redrawn: the corner table above says the point shift is negative everywhere, so the 1057 interval is an upper bound for 1058 only to the extent that the stand-ins enter additively.

## 16. Update 2026-12-05: item audit continued (journal `journal/2026-12-05-10-00-item-audit-leftovers.md`)

CURRENT stays `1058` (WR+2m 116.8 h, WR 162.9 h, R 1.6M seed 5601). Every item ball and hidden item (`data/item-audit-maps.json`, 349) was classified: further Rare Candies (Route 6, Mansion 3F, Cape Brink), vitamins (Calcium Mansion / Silph 9F, HP Up, Iron, Protein), PP Ups / PP Max, heal items, Revives, Never-Melt Ice are all saturated or n.s. at their guessed frames (free: -3.5 .. 0; at 400-450 f: -1.8 .. +2.8; a four-pickup bundle -0.7 at x1 and +9.1 [+4.4, +13.1] at x2). Money is not binding.
The one large item is **Leftovers** (Route 16 (31,13), 0 tiles off the Celadon -> Cycling Road path): -25.7 h free at `fly-celadon-2`, but the tile is `underfoot` and needs the Itemfinder (Route 11 aide, 30 Kanto species, `field_control_avatar.c:497-500`). Trip estimate 4 000-6 000 f; break-even about 4 000 f (WR+2m) / 3 500 f (WR); at 2 350 f -13.5 / -16.1 h, at 4 700 f -2.2 / +8.9 h, at 9 400 f +30.7 / +85.6 h. Not adopted (not robust to x2); the trip's frame cost is the open measurement. Model fixes: Rare Candy at the Indigo heal now recalculates stats (-0.08 h).

## 17. Update 2026-12-06: Itemfinder / pickup correctness of CURRENT (journal `journal/2026-12-06-10-00-itemfinder-gating.md`)

CURRENT stays `1067-d57-combo`; 1057 / 1058 / 1067 are unchanged (bit-identical on the gated build). Checked against `decompiled/`: only six of the 183 hidden items are `underfoot` (Cape Brink PP Max, Tower 7F Soothe Bell, Navel Rock Sacred Ash, Route 12 and Route 16 Leftovers, Viridian Gym Macho Brace); every pickup of the plan
(hidden Rare Candies Route 9 / 17, the Calciums = item balls, Hideout / Route 17 / Celadon PP Ups, Elixirs, Max Elixirs, stones, the Route 24 Nugget gift) is a non-underfoot hidden item (A facing the tile, `field_control_avatar.c:497-507`), an item ball or a script gift, and needs no Itemfinder (`human-run/tools/pickup_audit.py`).
The sim now gates the underfoot kinds (`leftovers`, `ppmax`) behind a new `itemfinder` pickup (30 caught Kanto species, aide trip charged). Leftovers with the measured trip (~5 300 f: 128 tiles each way on Route 11 with a 2F on foot, aide text ~1 000 f, extra Fly 700, Route 16 76 tiles each way, dig and give 750) is -0.5 [-4.5, +3.2] h at WR+2m and +17.5 [+10.3, +24.8] h at WR, +38 / +109 h at x2:
not adopted. The pickup-frame robustness of section 15 (x2 / x3 / x5 of the guessed 300-450 f per pickup, break-even about 4x) is unaffected: it concerns the non-underfoot pickups.

## 18. Update 2026-12-07: 1058 / 1067 interval, pickup frames from map geometry, rank flips; plan 1090 (journal `journal/2026-12-07-10-00-1058-interval-pickups.md`)

* reset_table refit on 1058 and on 1067 (train R 3M, eval 1.6M, best-iterate): paired R 3.2M, two seeds, both rungs, -0.1 .. -0.5 h (0.1-0.3%, under the 1% bar): not adopted.
* Pickup frames from geometry (Route 9 candy 170 f, Route 17 candy ~1030 f (lane change on the Cycling Road), Hideout Calcium 400, Silph 7F Calcium ~1170 (pocket reachable only from the east side), Nugget sale 300): their uncertainty is < 0.1% of the EHT variance.
* EHT interval, 1067: pinned runner survivals WR+2m 137 h, 68% [67, 327], 95% [38, 934]; WR 166 h [76, 429] / [41, 1317]. With the 1057 hazard residual carried over (engine gains kept): WR+2m 88 h [46, 203] / [28, 584]; WR 107 h [52, 266] / [30, 836]. Pinning Q and D cancels the plan's engine gains (1058 pinned 148 h, no better than 1057's 142 h), so the pinned interval is conservative for any plan that changes survival. 1058: 148 / 183 h pinned.
* Rank flips on 1067 (15 corners incl. pickups x2/x4, lp, cp, mp, Q, D): the five 1058 pickups hold up to ~4x cost; every one is kept except the **Silph 7F Calcium, which loses in 13 of 15 corners**: **plan 1090-no-silph-calcium** (1067 minus it) -1.5 h WR+2m / -2.1..-3.2 h WR at the plan frames, -4.4..-5.0 / -9.5..-10.2 h at the geometry frames (R 3.2M, seeds 9801/9802, CIs exclude 0). Lane windows pass `fit_lane.py --check`, no refit.

## 19. Update 2026-12-08: League stand-ins re-measured (journal `journal/2026-12-08-14-00-league-standins-recalibration.md`)

New per-turn evidence (WR VOD + 11-14 stream, `video-analysis/data/calib/league-*.json`): engine damage matches 7 non-lethal video hits within 1 px and the runner carry is not stronger than the sim carry, so league_power has no stat mechanism. It is (a) the engine rationing Blizzard (`scarce_factor` 0.6, now a param; the runners Blizzard Gyarados at once) and (b) the runners X queues at Lance / Champion. With those, **league_power 1.0 [1.0, 1.1]** (loglik -18.3 vs -30.1 at 1.4 on plan tactics); **mewtwo_power 1.3 [1.15, >= 1.4]** (no mechanism; Champion-2); carry_power prefers <= 1.7 (weak, edge of the structure). EHT of CURRENT at those constants with D pinned .23: **124 h WR+2m / 169 h WR** (106.6 / 139.6 at the old stand-ins); mp 1.15 / 1.45 132 / 129 h; cp 1.7 228 h. One rank flip: champion x = guard, speed, special, special is -23.5 h (WR+2m) / -43 h (WR) better at the fitted constants and +26.8 h worse at the old ones; 1094 (Lorelei calm) holds under both. CURRENT stays 1103.

## 20. Update 2026-12-08 (evening): MODEL CHANGE, evidence-fitted League stand-ins; League re-descent (journal `journal/2026-12-08-20-00-league-redescent-new-model.md`)

**MODEL CHANGE** (coordinator decision, like the PP change): `league_power` 1.0, `mewtwo_power` 1.3, `scarce_factor` 1.0 (section 19's fit), `carry_power` stays 2.0; flat post-door `fatal` rows and the `reset_table` refit, lane windows pass `--check` unchanged. 1107-evo10 at the old constants (84.5 / 108.8 h) becomes **1113-lgmodel: WR+2m 123.4 h, WR 166.1 h**.
League re-descent (`descend.py --set league`, partly run): confirmed Champion queue guard, speed, special, special (-23 / -36 h), Agatha -x speed, league_preheal .5, Lance first x calm (screen); Mewtwo teaches are noise (0.001 h). Re-checks on the new model: **1096 (Jolteon stone) flips and is reverted**; 1097 ball rule, 1099 block and 1107 (Parasect) survive in every cell and corner; **Koga +1 X special (1105) adopted** (+8 / +10 h; corner carry_power 1.7 -42 h, CI touches 0).
**CURRENT = `1122-lgdescent`: WR+2m 84.6 / 85.1 h, WR 112.2 / 111.0 h (R 3.2M, seeds 7401/7402; -37.8 / -55 h vs 1113)**. Corners vs 1113 (WR+2m): lp 1.4 +6.7 h worse, mp 1.15/1.45 -37 better, door .10 -94, cp 1.7 -205. Interval not recomputed (NEXT in the journal).
