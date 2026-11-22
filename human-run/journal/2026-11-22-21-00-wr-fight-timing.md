# 2026-11-22: WR trainer fights timed off the VOD (the per-fight skill factor)

Branch `human-run-play`, worktree agent. Plans 1285-1287, rows in `scores.jsonl`. Parent: model of record
`1280-wr-faithful` (`trainer_cost` = [838, 579, 223] f per battle / player action / enemy mon KO, fitted on
the played s11 run). Question: is the WR runner faster per fight than the bot, and by how much? (1283's x0.9
rested on two timed WR fights.)

## Method (frame-accurate, no hand timing)

- **Raw material.** The WR VOD is on the host mount (`/run/sandbox/source/ignored/downloaded/frlgr2-wr-3.25.25-main97.mp4`,
  1080p60, video = run + 35 s); the played run's render `$FRLG_ARTIFACTS/videos/1122-lgdescent-s11-giovanni1.mp4`
  is 59.7275 fps with video frame = game frame (start -> Giovanni 1), and the s11 / fwd-main per-frame RAM
  traces (`cal_trace` CSVs from the calibration session) cover the played run to Victory Road.
- **`video-analysis/tools/battle-feat.sh`** decodes every frame to 13 colour cells (window mean, 8 cells of the bottom
  band, 4 of the top band); **`battle-timeline.py`** classifies each frame (black / battle text box / action menu /
  move menu / white box / other), chains segments between black runs into battles (bag and party screens inside a
  battle are bridged; a trainer's speech + swirl ends a chain) and reports per battle: visible start (`t0`, end of the
  swirl's black) and end (`t1`, fade-out done), the number of player decisions (action-menu episodes), seconds in
  menus, and the pre-battle speech. Whole VOD in ~25 min of CPU.
- **Validation on the bot render against RAM**: every pre-Giovanni trainer battle is found; visible battle =
  RAM `inBattle` +34 f .. -16 f (constant, 15 battles checked); detected decisions = the events' turn count in every
  fight checked (Alicia 4/4, Liam 5/5, Brock 4/4, Colton 6/6, ...). The bot's numbers below therefore come from RAM
  (`inBattle` - 50 f, events' turns), the WR's from the detector.
- **Identification**: a contact sheet of every WR battle at t0 + 3.4-4.6 s shows "<CLASS NAME> sent out X!"; ids
  and parties from `decompiled/include/constants/opponents.h` / `src/data/trainer_parties.h`. **67 WR trainer
  fights** (Bug Catcher Sammy -> Route 22 rival; no League), 47 of them also fought by the played run (same trainer,
  same party). Five chains ended in a post-battle evolution scene ("X is evolving!", sheets at 1272, 7531, 7644,
  8382, 8508: the WR levels bugs for the dex) and were cut (24-27 s each). WR decisions were spot-checked on a
  sheet of action-menu frames (Cerulean rival 7/7, Silph rival: lead Weedle switch-out and X items from the bag).
- Table: `video-analysis/data/wr-fight-timing.json` (per fight: WR t0, visible battle s, decisions, menu s,
  speech s; bot RAM battle s, turns, level-ups, episode s). Scripts: `video-analysis/tools/fight-timing/`.

## Results

Per component (visible battle seconds, OLS `dur = a + t x decisions + m x mons`, bootstrap 95%):

| set | a (s) | per decision (s) | per mon KO (s) | rms |
| --- | --- | --- | --- | --- |
| WR, all 67 | 8.7 [4.8, 12.7] | 6.62 [5.22, 8.02] | 5.81 [3.23, 8.91] | 7.1 |
| WR, 47 paired | 13.3 [8.8, 18.4] | 7.00 [5.86, 8.63] | 3.04 [1.34, 4.95] | 6.3 |
| bot, same 47 | 12.3 [7.8, 18.5] | 8.55 [7.51, 9.56] | 3.27 [1.29, 5.26] | 8.3 |

- **Overhead and per-KO: no difference.** Pre-battle speech is identical to the frame (24 clean pairs: 54.1 s vs
  53.9 s summed; both mash through text at the same speed), intercept and per-mon terms agree.
- **Per action: the WR is faster in battle text, slower in menus.** Same decision count and no bag use on either side
  (24 fights): WR 0.912 of the bot's battle time (-3.5 s per fight); the WR spends 0.42 s per decision in the menus vs
  the bot's 0.29 s, so the gain is in the text between decisions (the bot's older stand-in driver and lower-level
  lead: more level-up text). Over all 47 pairs WR 0.816 [0.74, 0.89] of the bot: most of that is **turn count**,
  WR 156 vs bot 185 decisions (0.84 [0.72, 0.98]): the bot over-turns early (Sammy 15 vs 6, Surge 11 vs 3, Koga 10 vs
  7, Misty 8 vs 5) and uses potions mid-fight (James, hideout grunt 17: +31 s each).
- **Against `trainer_cost` at the WR's own decisions** (WR episode = visible battle + that fight's played overhead,
  median 5.9 s): **WR = 0.944 [0.910, 0.987] x `trainer_cost`** over 67 fights (0.939 [0.897, 0.991] on the 47
  pairs). The bot's own episodes are 1.010 x `trainer_cost` at its turns (the fit reproduces itself).
- **The sim's turn counts are not the bot's.** `fightdist` on 1280 (seed 7401, 77 League-reaching attempts, the
  WR's fight list in the WR's order): the sim takes **202 decisions where the WR took 237** (ratio 1.17), with the
  whole gap in rivals and leaders (Cerulean rival 7 vs 4.2, S.S. Anne 8 vs 4.6, tower 11 vs 6, Silph 11 vs 7.1,
  Route 22 13 vs 9, Sabrina 7 vs 4, Viridian gym 10 vs 8; route trainers match: Nugget Bridge 14/14, Rock Tunnel
  30/28.8, tower grunts 9/9). The WR spends those turns on X items, switches (weak lead out) and extra hits.
- **The whole fight bill**, model vs WR on the same 67 fights: the model's clock (sim fight frames incl. reactions
  and jitter + outro) 3732 s, plus the level-up text it adds on top (~40 trainer level-ups x 240 f = 161 s; the
  played episodes the cost was fitted on already contain their level-ups, 0.43 per fight, so this is partly a double
  count: a refit with level-ups fixed at 240 f gives [798, 536, 251]) = **3893 s vs the WR's 3684 s (0.946)**.

| `trainer_cost` | model fight s (+ level-up text) | WR / model | WR / cost at WR decisions |
| --- | --- | --- | --- |
| 1280 [838, 579, 223] | 3732 (3893) | 0.987 (0.946) | 0.944 |
| x0.9 (1283) | 3376 (3536) | 1.091 (1.042) | 1.049 |
| **x0.944 (1285)** | 3535 (3696) | 1.042 (**0.997**) | **0.999** |

So x0.944 is right on both checks: it is the measured factor at equal decisions, and with the sim's (too few)
actions and its (double-counted) level-up text the model's total fight time equals the WR's. 1283's x0.9 makes the
model's fights 4 % faster than the WR's.

## Scores (WR+2m, seed 7401, R 800k, paired vs 1280)

| plan | what | kind | EHT | vs 1280 [95% CI] |
| --- | --- | --- | --- | --- |
| 1280 | model of record | | 1053.3 h | |
| **1285-wr-fight-speed** | `trainer_cost` x0.944 = [791, 547, 211] | MODEL CHANGE | **276.2 h** | **-777 [-945, -654]** |
| 1286-wr-fight-speed-hi | x0.987 (upper bound) | SENS | 691.2 h | -362 [-561, -236] |
| 1287-wr-fight-speed-lo | x0.910 (lower bound) | SENS | 166.6 h | -887 [-1065, -756] |
| (1283) | x0.9 | FIT (predecessor) | 146.3 h | -907 [-1084, -774] |

No-reset cumulative error at the WR splits (R 30k): 1280 Rock Tunnel -43 s, Giovanni 1 -14, Giovanni 3 +38, Start
E4R1 +261; 1285 -142, -128, -151, +53. With the fights priced at the WR's measured time the rest of the model runs
~2.5 min ahead of the WR by Giovanni 3: non-fight time is now the optimistic part (not chased here).

## Verdict

- **x0.9 (1283) is not supported; x0.944 is** (CI [0.910, 0.987] excludes both 1.0 and 0.9 is at its edge). The
  factor is about text speed in battle, not menus (the WR's menus are slower), and it is the same for overhead and KO
  terms, so a scalar on `trainer_cost` is the right shape.
- **Recommended: 1285 as model of record** (MODEL CHANGE, measured, -777 h vs 1280). The score is very sensitive to
  this factor: the factor's own 95% CI spans 691 h (x0.987) to 167 h (x0.910) at WR+2m; the per-fight factor is now pinned by 67 fights instead of two.
- Two compensating model errors remain and should be fixed together, not separately: (1) the sim plays boss/rival
  fights in ~40 % fewer decisions than the WR (engine/policy: X-item and switch turns, damage), (2) the level-up text
  is counted twice (in `trainer_cost`'s fit and in `level_up`). Fixing only (2) makes the model optimistic, fixing only
  (1) pessimistic.

## Not done

- League fights (League keys belong to the League agent; `trainer_cost` does not apply there).
- Silph Giovanni (not in the detector's list: no speech-led battle found near 7664; the predecessor's 7664 is the
  Kakuna evolution after the Silph grunt).
- The approach walk / "!" before the speech is not visible as a marker; the played overhead per fight is used for the
  WR too (speech times match to the frame, the walk is geometry).
