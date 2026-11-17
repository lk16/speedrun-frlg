# 2026-11-27: post-door stand-ins on the current engine, Champion-2 length, and the play-on rule on 1040

Task: re-check the post-door survival stand-ins (`fatal` rows, S) and the Champion-2 fight length against the video data and the engine, tighten only what the evidence supports, and test plan 607's play-on rule (`no_salvage_until ""`) on CURRENT 1040. Rows in `scores.jsonl`: `1042-postfit` (MODEL CHANGE), `1040-playon`, `1042-playon`.

## 1. The `fatal` rows were fitted on an easier engine

`fatal` was fitted (journal 2026-11-15-18-00, plan 802) as the extra death rate on top of the engine's own, when the engine was gentle (engine-only death Champion .036, Champion-2 .094). The AI fixes (2026-11-21) and the League retune (2026-11-23) made the engine itself deadly at the two Champions and the `fatal` rows were never re-fitted (the retune journal: "untouched ... the flat League `fatal` rows"). `tools/fit_post_evidence.py` on 1040 (R 800k, seed 1, k = 3): engine-only death per fight Lorelei .034, Bruno .074, Agatha .065, Lance .069, **Champion .201**, chain 0, round 2 .001 / .002 / .006 / .027, **Champion-2 .450**. With the old `fatal` on top the model's door-to-finish survival is **0.068** (6.6% of door arrivals in the `report`: reached .072% of 1.091%). The data: 4 door attempts that fought, 1 finish (Wilson 0.05-0.70); joint with the runner's 15 811 attempts per WR finish the post-door survival is about .2 (earlier fit). The engine alone gives .33 (product of its own survivals, no `fatal`).

Refit (same prior weight, evidence unchanged): total survival posterior median .088, 90% [.018, .262]; the model's refit total is .108. Per-fight `fatal`: Lorelei 0, Bruno .187, Agatha 0, **Champion 0**, chain .098 / .106 / .115 / .149, Lorelei-2 .055, Bruno-2 .054, Agatha-2 .054, Lance-2 .096, **Champion-2 0**; product of fatal survivals .205 -> .277. The change is mostly structural: at the Champions the engine's own hazard (.20, .45) already exceeds the posterior total hazard (.167, .384) the evidence (0 deaths at 2 and 1 at risk, prior k = 3) supports, so the extra flat hazard goes to zero. The evidence is thin (Wilson intervals 0-.66 / 0-.79 at the Champions); this is a double-count removal, not a new measurement. The engine's Champion rates (the League retune journal) stay unverified against the one Champion draw of the WR.

Written as `plans/1042-postfit` (same policy as 1040; `reset_table` not refitted: journal 2026-11-26 shows it nearly independent of the calibration). Paired against 1040, R 3.2M, seeds 71 / 72: WR+2m 310.2 / 316.9 h against 431.4 / 427.1 h (diff -121 / -110 h, CI excludes 0), WR 424.5 / 438.3 h against 592.9 / 592.6 h. **MODEL CHANGE, not a plan evaluation.** CURRENT stays 1040 (pointer unchanged); quote 1042 for absolute EHT, with the old interval caveat (the model range is still a factor 2-3).

## 2. Champion-2 fight length: nothing to change

`frlg-human splits` on 1040 (R 40k): Champion-2 model 12.3 player actions against the WR's 11.5 (the recount of 2026-11-22), segment 3:39 against the WR's 3:42; core seconds 175 against 196 (the model is 11% short on core time; level-up/outro padding absorbs it). The WR used 0 X Specials there (3 -> 3, `wr-frame-facts.json`); the plan's `special, special` stays (the runner's own words: "there's supposed to be a Special here for Champion 2", 11-03 4:07:55; STRATEGY 17, X Special + Guard Spec on Champ 2). No other run has a timed Champion 2 in the committed data (11-03 only, no splits), so the length has one sample and the model sits inside it. No change.

## 3. Play-on rule (607) on 1040: refuted

Paired, R 3.2M each. 1040 vs play-on (diff = 1040 minus play-on, h; CI): WR+2m seed 71 +1.1 [-17.3, +18.9], seed 72 -13.2 [-31.6, +7.0]; WR seed 71 -7.7 [-38.0, +22.6], seed 72 -32.6 [-62.8, +5.5]. On the refit model: play-on minus 1042 +8.6 / +7.2 h (WR+2m, CIs include 0), +16.3 / +21.6 h (WR, CIs [-1.7, +35.8] and [-1.7 .. +41.9] touching 0). **Play-on is no better on either model**, worse on three of four rows, which matches the earlier descent (`d914-104`: `no_salvage_until rival-ss-anne` -26 h over `""`; later values worse). The 607 paper edge of ROBUSTNESS section 4 came from the 608-era model (flat hazards, no reset table); the reset table already abandons slow post-whiteout runs. Not adopted; the human rule stays "reset after an early whiteout".

## What this does not do

* `fatal` stays a flat hazard; the evidence is 4 fighters, and the engine's Champion difficulty (the other half of the post-door survival) has one WR sample.
* No descent run on the refit model; the policy ranking 1040 vs play-on was the only policy test.
* `reset_table` not refit on 1042.

## Next

* An attempt log with the E4 deaths (host transcripts, `HOST-REQUESTS.md`) is the only thing that moves S further.
* If the post-door model is trusted at S .277, a Lance (X item / sac) descent on 1042 is the cheapest next idea: Lance carries the largest remaining `fatal` (.269).
