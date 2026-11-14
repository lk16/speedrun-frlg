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
