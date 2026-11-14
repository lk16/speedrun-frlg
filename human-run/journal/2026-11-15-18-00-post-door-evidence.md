# 2026-11-15: post-door evidence, S(segment) hazards and slow events fitted to the streams (plan 802)

Task: the post-door survival S (0.183, a calibration residual), the `slow` probabilities and `win_cv` set the absolute EHT and had no direct evidence
behind them. This entry extracts what the existing transcripts and committed stream tables say, fits per-segment hazards and slow-event rates with
intervals, and re-checks plan ranking and the post-door reset thresholds. No new transcription was made (existing `.srt`/`.txt` under
`/run/sandbox/source/ignored/downloaded/` and the committed `video-analysis/` files only). Rows marked MODEL CHANGE / MODEL SENSITIVITY in
`scores.jsonl` are model statements, not plan evaluations.

## 1. What the data hold (`video-analysis/data/post-door-evidence.json`)

Door attempts (a run that reached Start E4R1) in the nine stream days. **There are very few, and the earlier note was wrong in two places.**

| attempt | end | what it is | quality |
| --- | --- | --- | --- |
| 11-03 | finish, credits 3:29:05 | passed E4R1, Icefall/Warehouse, Mewtwo, E4R2 | A |
| 11-06 | died at Bruno 1, ~2:45 | srt 8:45-8:49 "He misses the Tomb and switches the EQ and kills me"; same run as the 11-09 anecdote "my best run ever ... died to Bruno" (11th-try Pikachu, 8 Power Plant duplicates) | A |
| 11-09 | died at Lance 1 (or Champion 1), ~2:49 | srt 8:15:53 "we missed 7% Blizzard", 8:16:09 "we were meant to die" | B |
| 11-12 deep run | pace reset at the door, 2:39:37, 60/60 | no E4 fight played | A |
| older route | "the 235:33 [entry], which died in Warehouse" | srt 11-09 6:37:19; the death fight inside the 4-fight chain is not named | C |
| 11-02 | ended 2:42:10 | no transcript exists; death or pace reset unknown, **excluded** | - |

Corrections: (a) the fidelity-fit table calls the 2:42, 2:45 and 2:49 ends "pace-gate resets". Two are deaths inside E4R1 on the runner's own words
(11-06 at Bruno, 11-09 at Lance/Champion), so of the 5 door arrivals only one (11-12) was a pace reset and the model's lenient-rung `P(finish | door)` of 12.6%
now has one finish in four fighters beside it. (b) the 11-09 "died to Bruno" anecdote is the 11-06 run retold, not a second death (counted once).

**Fighters: 4 (finish, Bruno, Lance, Warehouse). Door to finish 1/4 = 25%, Wilson 5-70%.** Per fight (at risk / deaths): Lorelei 4/0, Bruno 4/1,
Agatha 3/0, Lance 3/1, Champion 2/0, Icefall-Warehouse chain 2/1 (0.25 on each of the four), round 2 1/0. The Bruno and Lance deaths are 1 in 4 and 1 in 3:
the intervals are 5-70% and 6-79%, which includes everything the model assumed (.10 and .20). **This is as thin as the pre-door table's
late windows (3-10 at risk each) and not thicker.**

## 2. Fit (`tools/fit_post_evidence.py`, `tools/post_joint.py`)

**Per fight.** A Beta posterior on the *total* hazard (engine deaths plus `fatal`), prior = the plan's current total worth k = 3 observations (so a
fight with no data keeps its value; k 1.5 / 6 give S 0.206 / 0.190), then `fatal` = what must be added to the engine's own death rate (fit_hazards style:
engine-only survival at the lenient rung, Lorelei .0, Bruno .005, Agatha .004, Lance .019, Champion .036, Champion 2 .094). Result (802): Lorelei .042,
Bruno .182, Agatha .047, Lance .253, Champion .097; chain .080 / .084 / .089 / .128; round 2 .073 / .073 / .073 / .137 / .114. Product **S = 0.205**
(was 0.183): the data move mass from Lorelei/Agatha/Champion to Bruno/Lance and leave the product where the residual had it. Moving the 11-09 death
from Lance to Champion gives S 0.201 (`--alt`).

**Interval on S** (prior k = 3, 20 000 draws): total survival Door..HoF2 median 0.150, 90% [0.038, 0.379]; fatal-equivalent median 0.179, 90% [0.045, 0.453].

**Joint with the runner's 15 811 attempts per WR finish.** That number belongs to the runner's *own* policy, so it was applied to plan 480 (the fit
baseline: 800 000 attempts give 59 finishes = 13 600 per finish on today's code) over S = 0.06 .. 1.0 at R = 6.4M (`data/post-joint-480.json`):
attempts per WR finish 46 400 (S .06), 29 500 (.10), 20 900 (.14), **15 600 (.183)**, 11 600 (.25), 8 200 (.35), 5 650 (.5), 2 800 (1.0). Posterior
(flat prior in S; door binomial 1 of 4; geometric first passage at 15 811): door alone 5/50/95% = .06/.25/.70; 15 811 alone .06/.18/.50; **both .06/.2/.5**
(grid points; weighted mean .24). So the 0.183 residual was inside the data's interval and the evidence gives S = 0.2 [0.06, 0.5] at 90%, not a sharper
number; one observation of each kind cannot do better. Note 801 itself reaches WR in 650 attempts at S .183 (`post-joint-801.json`): the
15 811 is the runner's policy, not the optimised one, which is why the joint fit uses 480.

**What S buys** (802 policy, paired): WR+2m 115.7 / 108.2 h (seeds 1, 2, R = 1M) against 801's 131.6 / 127.6: -15.9 h CI [-20.1, -11.3] and -19.3 h CI [-23.4, -15.4];
WR rung R = 3.2M seed 3: **141.2 h (5 549 finishes) vs 159.0 h**, -17.8 h CI [-21.4, -14.1]. This is the S change (+12%) and the shape change together.
CURRENT = `802-post-evidence` (plan file; MODEL CHANGE: the policy and the route are 801's).

## 3. Slow events

Counts from the streams (`post-door-evidence.json`, quotes cited there). A "visit" is an attempt that ended after the step in the run timer.

| event | model | observed | verdict |
| --- | --- | --- | --- |
| Safari bad (lost >= ~2 min) | p .2, mean 120 s: P(extra >= 120 s) .074 | 3 of 12 visits (11-09 "one of three horrendous", 11-11 "worst safari ever", 11-12 deep "three and a half minutes to leave"), Wilson [.09, .53] | model low but inside the interval; the in-zone hunts add ~100 s of their own sd, so no change. A mapping p .5 costs +7% (see below) |
| Power Plant duplicates (>= ~2 min) | p .3, mean 90 s: P(extra >= 120 s) .079 | 1 of 8 (11-06: 8 duplicates, "11th try Pikachu", a 5.6% geometric tail at a 25% slot), [.02, .47] | consistent |
| Cerulean Cave | extra 40 s on top of a ~48 s base: mean 1:28 | 11-03: "five encounters, losing a minute and a half" | consistent (n = 1) |
| Route 8, Misty / Cerulean rival fallback | p .15 x 40 s, .06 x 100 s | no counts | stays a guess |

`win_cv`: no new evidence (window deviations exist only for the deep run, already used; the door entry times {2:35:33, 2:36:05, ~2:36 x2, 2:37:40, 2:39:37} are
selected by the runner's own gate, so their spread cannot be compared with the model's untruncated one). Sensitivity levels unchanged: slow off -14%
(-16.0 / -15.5 h on 802, seeds 21, 22, R = 1M), Safari p .25 +1.3%, Safari p .5 +7%.

## 4. Ranking and CURRENT under the evidence model

7 plans x 6 post-door models (own .183, evidence .205, S .06, S .5, .1, .35) x seeds 1, 2, WR+2m, R = 500 000, older plans rebased on 608 (first attempt rebased
on 801 and gave 800 = 801 and 700 = 608 because the route variant came along; thrown away). `data/post-rank-wr2m.json`:

| model | 607 | 700 | 608 | 801 | 800 | 600 | 492 |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| own (S .183) | 103 | 116 | 118 | 129 | 131 | 145 | 216 |
| evidence | 89 | 101 | 103 | 113 | 115 | 125 | 191 |
| S .06 | 307 | 345 | 350 | 387 | 391 | 432 | 700 |
| S .5 | 38 | 43 | 44 | 48 | 48 | 53 | 80 |
| S .1 | 184 | 208 | 212 | 235 | 239 | 263 | 395 |
| S .35 | 54 | 61 | 61 | 67 | 68 | 75 | 112 |

(mean of two seeds, h.) **The order 607 < 700 < 608 < 801 < 800 < 600 < 492 holds in all 6 models and both seeds**; the plan edge over 492 is 1.7-1.8x everywhere.
(700 and 801/800 are the same policy with different starter-phase calibrations; the starter-phase +10% of 801 over 700 is the 2026-11-15 recalibration, not
a ranking change.) 607 (play on after an early whiteout) stays the paper winner by 12-14% in every model, as in ROBUSTNESS.md. The absolute number moves 48 h .. 387 h for
801 across the S interval: the factor 8 that ROBUSTNESS section 2 reported for S is the data's own width now.

## 5. Reset thresholds past the door

`reset_aggr_at` of the E4R1 group (indigo-heal .. hall-of-fame-1: .65 / .8 / .75), the Icefall-Warehouse-Mewtwo chain (default .8) and the round-2 group
(default .8) at 0.4 .. 1.0, in four post-door models, seeds 1, 2, R = 800k (`data/post-aggr-wr2m.json`):

* lowering to 0.4-0.7: -0.2 to -0.4% in every model; raising to 0.9 / 1.0: +0.8..+1.1% / +2.1..+3.3%. Round 2: exactly 0 at every level (the pace gate never
  fires there at WR+2m, a door run that reaches Mewtwo is within the rung by then).
* confirmation on 802 (E4R1 + chain at 0.55, seeds 11, 12, R = 2M): -0.96 h CI [-1.27, -0.65] and -0.90 h CI [-1.21, -0.59], **-0.8%**; at 0.9: +1.6 / +2.1 h. The effect
  is real and below the 1% bar the descent uses. **No retune warranted**: playing on is marginally better past the door, but the gain is under 1 h out of 113 and
  the S interval does not change its sign (all four models agree).

## What this does not do

* Four fighting door attempts, one finish. Everything per fight (Bruno .18, Lance .25) is a posterior with a prior worth three observations; the intervals contain the old values.
* The slow-event probabilities and `win_cv` stay mostly guesses; only Safari, Power Plant and the cave have counts and those are consistent with the model.
* The 2:42:10 run of 11-02 has no transcript; with a Bruno/Agatha death in it the Bruno estimate goes up (not run). No transcript exists for 11-02, 11-04, 11-05.
* The 15 811 enters through plan 480 as a stand-in for the runner's policy; the rest of the model around S (pre-door hazards, slow, trainer_fit) is held fixed.
* `fatal` is still flat: a safer Lance (X items, sacs, Mewtwo level) cannot reduce its hazard in the model.

## Next

* A transcript of the 11-02 and 11-04/05 videos (host: whisper, `HOST-REQUESTS.md`) would add 3-5 door arrivals if the runner commented the E4 deaths.
* If the real attempt log appears (LiveSplit file), replace the table in `post-door-evidence.json` and re-run `fit_post_evidence.py` + `post_joint.py` (both stdlib, ~15 min).
