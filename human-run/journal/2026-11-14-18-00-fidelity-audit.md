# 2026-11-14: fidelity audit, Misty -> Start E4R1 and post-E4R1 (plan 451)

Task: compare `frlg-human splits` / `report` with the WR splits (`video-analysis/wr-3.25.25-main97.md`) and the stream
(`stream-2026-11-12-main97.md`, `STRATEGY.md`), find where the model is systematically fast or slow, add what a human loses
and the model does not. Starting point: 450 (WR rung 26.0 h EHT, 6275 finishes in 400000; `no-reset` p50 3:27:45). All new
knobs default off; plan 451 turns them on. **Every row in `scores.jsonl` from this session is a MODEL CHANGE, not a plan
improvement.**

## Why the post-E4R1 half was fast (Start E4R1 +7.8%, End +1.3%: 9.4 min)

Per-window model vs WR (450, no-reset finishers): E4R1 -1:18, Icefall -2:03, Warehouse -3:40, R2 chain -2:28 (Bruno-2 -0:21,
Agatha-2 -0:14, Lance-2 -0:24, Champion-2 -0:51, End -0:30). Two causes, both missing time, neither a fight-engine error:

1. **Nothing between the rooms.** The fight constants are fitted to the video's battle-to-battle durations (Lorelei 148 s,
   Bruno 104, Agatha 88, Lance 120, Champion 154; `wrsplits.rs` `WR_FIGHTS`). The WR's split windows are 173/125/110/166 s: the
   difference (25/21/22/46 s in round 1, 32/15/44/26 s in round 2) is walking, doors, the speeches, bag trips. The model had
   zero. `league_transit` adds it as `e4-transit-*` text segments; `hall-of-fame-2` gets +30 s (the WR's End split is 73 s,
   the model's 43). +4.4 min.
2. **The Champ 1 -> Rocket Warehouse chain was guessed at 13:15; the WR's is 18:58** (Champ 1 -> Icefall 9:34, Icefall ->
   Warehouse 9:24). The scene log (`wr-3.25.25-main97.md`: Ruby at Celio video 2:55, Icefall 2:57-58, Warehouse 3:01-3:06,
   video = run + 35 s) places the missing time in the Ruby Path climb (+80 s), the Icefall ice rooms (+43 s), Four Island ->
   Five Island before the Warehouse (+100 s) and the Warehouse interior between the trainers (+120 s). `postgame_calib`
   adds these four text segments (342 s) so both windows equal the WR's within 20 s. The scene log is a 60 s sampling, so the
   split between the four is a guess; the sum is a measurement. (The scene log's "Dotted Hole 3:00" is not on the route: Sapphire
   is given in the Warehouse, `decompiled/data/maps/FiveIsland_RocketWarehouse/scripts.inc:59-60`.)

Effect at WR+10m (paired, R = 200 000): league_transit +1.59 h, postgame_calib +42.8 h. The second is huge because the rung
is only 10 minutes wide and the model's median finisher was already 2 minutes behind; the same plan lost 85% of its finishers.

## Why the first half is slow (Start E4R1 +7.8% at 450, +12 min once the post-game is fixed)

Nothing in the first half is a big missing piece; it is one constant. The fight fit (`turn` 345 f, `mon_overhead` 630 f, trainer
intro x0.8) was fitted on the sixteen video fights where eight are League fights, which run 1.0x; the eight others (Brock,
Misty, SS Anne rival, Koga, Blaine, Sabrina, Giovanni, Route 22 rival) take 727 s in the model against 596 s in the video (1.22x).
The same constants run in every trainer group of the first half (47% of the first 2:48 is fights). `trainer_fit` multiplies
the intro and per-mon overhead of non-League trainer battles. **0.71 reproduces the WR run's own splits within 5% at every
split** (Start E4R1 -47 s, End -28 s, no-reset p50 3:24:28), and matches the 8 fights (core seconds mean |log| 0.14 -> 0.11).

0.71 is the *WR run*, not the average WR-level run: the WR is the best of 15 811 attempts. Evidence for a typical run:
the runner resets a 60/60 run entering the Elite Four at 2:39:37 and calls "2:37 entry" necessary (STRATEGY section 7;
S12 deep run), and "most realistic good time 3:23, next PB high 3:24" (T07). The EHT at the WR rung is extremely sensitive to
this one number (`i1` copies of 451 with `trainer_fit` .71 / .76 / .81 / .86: 11.5 h / 25.9 h / 588 h / no finish at all in
200 000; Start E4R1 mean 2:35:17 / 2:37:59 / 2:40:15 / 2:42:32). **451 uses 0.81** (Start E4R1 2:40:15, End +2.4%, p50 finish
3:30:00, p10 3:25:25). The sanity alarm in PROMPT.md was the reason: at 0.71 the model finishes at or under the WR in 5% of
attempts; the runner has one in 15 811 (6e-5), and has not repeated it in the ~2 600 attempts since (S12 header). At 0.81 the
model has 3.5e-4 per attempt and EHT(WR) 635 h; the runner's own career is ~10^3 h. This is a calibration choice, labelled one.

Per-fight residuals that remain (451, core seconds model/WR): Misty 81/54 (9.6 actions vs 6.5), Koga 85/72, Giovanni 81/78,
Route 22 rival 143/125; Blaine 69/80 and Sabrina 78/80 are under. Misty is the only fight with a real action-count gap: the WR
one-shots with double Water Pulse; the model's Starmie fight takes 9.6 actions (plan `misty power 0.9`, an old tuning knob).
Dex schedule: the model has 12 species at Misty against the WR's 8 and 52 at Sabrina against 45 (its Sabrina window is
+413 s, Giovanni 3 +274 s); the plan prefers early catches because a catch costs ~55 s in the model, the WR defers them
(the LG PB has 34 at Sabrina and is 10:25 ahead there, S12). The door time is unaffected (it is a schedule, not a cost).

## What a human loses that the model had no row for

| Item | Knob | Evidence | Effect (WR+10m, paired vs previous step) |
| --- | --- | --- | --- |
| Persim Berry every run, potion menus, Black Glasses swap before the Lavender rival (27 s) | `item_swaps` | STRATEGY 4, 6 (T07 1:23:20, T06 2:55:15, 3:05:09-3:07:09, 3:33:25) | +0.10 h (CI [0.09, 0.11]) |
| Battle menu slip (Right+A same frame, wrong move row), 1% of actions | `menu_slip` | STRATEGY 4 (T10 2:49:21 "kick-select input errors nearly killed runs"), 1.3 (late-day errors); the value is a guess | +0.16 h ([0.08, 0.24]) |
| Early deaths the engine does not produce: the model's Brock fight is a 98% win at every power from 1.0 to 0.45 (the entry heal puts the carry in the safe window), the runner's Brock split passes 75% on a good day, 2/12 on a bad one | `hazard` (per segment) | S12: `!deaths` = brock x5, sammy x2, liam, rival 1, rival 2 in ~12 attempts past the starter gate; STRATEGY 7 | +0.20 h ([0.15, 0.24]); Brock-split pass 89% -> 76% |
| Bad days: manip miss streaks, drift, fatigue | `regime_bad` + `tools/regime.py` | T07 (30 misses, 1.5 h lost), T10/T12 (12 misses in a row), T06 (4 in a row, "90%") | time-weighted mixture, w_good 0.5 (guess): EHT(WR) 635 h -> 1270 h; EHT(WR+10m) 7.1 h -> 13.3 h |

Already in the model, so not added: Mega Kick (TM, 75% accuracy, PP via `slot_pp` and the Elixirs, `tm-mega_kick@route4`), Sammy's
poison (`poison_frames_per_hp`, 377), the potion bag (`league_heals`, `xbuys`), wall bumps (`mistake_rate` 0.3 per 100 tiles/inputs,
~1.4 min a run; STRATEGY section 8 has "a minute of mistakes" in a bad run). The Sammy camp / Persim choice itself (camp for both
bugs unless poisoned) is a branch the plan does not express; the camp costs HP, which the Torrent policy already trades.

## Not fixed (open, ordered by size)

* **EHT is dominated by the pace distribution, not the event list.** 27 s of item swaps moves EHT(WR+10m) by 0.1 h on a wide rung and
  by 55% when the rung is the margin (an earlier step, `v1i`). Any claim at the WR rung needs R >= 400 000 (100-300 finishes,
  +-10-17%); WR-rung differences below 20% are noise.
* **Finish-time spread.** No-reset finishers have sd ~3.7 min; the runner's good runs run 3:27-3:30 and bad ones far above
  (S12: reset at 2:39:37; STRATEGY 8: Safari 3:20 for two, spec strats 1.5-2 min, Power Plant duplicates 2 min). The tails are
  thinner than the real ones, so P(<= WR) is probably still high by a factor of a few at 0.81.
* **Late deaths.** Koga ("huge problem"), Erika, Lance's Blizzard (7% per turn), Champion 2 (the runner "died six times in round two")
  have no hazard row; the engine gives Koga 96%, Sabrina 96%, Route 22 rival 85%, Champion 2 76% of its fights. After SS Anne a loss is
  a salvage (a whiteout) and the pace reset ends the run, which has the right sign.
* Misty (9.6 vs 6.5 actions) and the dex schedule (12 vs 8 at Misty) are model-side gaps listed above; neither moves the door time.
* Plan retune on the new model: see the score rows after this entry.
