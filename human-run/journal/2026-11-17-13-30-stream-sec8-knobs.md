# 2026-11-17: stream 11-14 section 8 proposals as paired knobs (vs 1040-optstop)

Paired, R 1.6M seed 111 then 3.2M seed 112 (WR+2m and WR for the close ones). 1043-calm became CURRENT mid-session; it only changes the post-door
Mewtwo moves, so these pre-door knobs were run on 1040 (same engine). Rows in `scores.jsonl`. Nothing adopted; CURRENT unchanged.

| # | knob | result (h, WR+2m unless noted) |
| --- | --- | --- |
| 1 | remove viridian/celadon/route3/mt-moon from `skip_groups` | bit-identical: with `audited_forced` those are `forced()` lists, which ignore `skip_groups`. Real mechanism = sight cones (`decompiled/src/trainer_see.c:198-231`, `forced-audit.md`); A1's "2 Viridian trainers" would need a `group_n` that raises, which the code only truncates |
| 2 | keep_moves Surf/Blizzard/Bite/Water Gun | +204 [168,247] (Water Pulse power 60 vs Water Gun 40, `battle_moves.h:718-721,4579-4582`); also dropping the Pulse TM +833 |
| 3 | second Venonat Safari hunt after erika | +58 [48,69] |
| 5 | candy_early_n 2 / 0 | +83 / +209 |
| 6 | door gate: play out to ~2:39 (reset_table indigo-heal +95 s, +HoF) | +0.2 / +0.3 h (WR +0.3/+0.5): almost nobody arrives in that band, no gain |
| 7 | Rash only / HP IV 5 / both | +67 / +21 [3,38] / +59 |
| 9 | Persim skip (new knobs `persim`, `persim_frames`=300 guess) | free skip -6.6 [-9.2,-4.5] (WR -9.8); with a guessed Misty cost (power .85, 7 potions) n.s. -7.2 [-15.5,+2.5]. Break-even depends on the guessed cost: not adopted |
| 4,8,10-13 | not tested: 4 owned by the Power Plant agent; 8 = no change; 10-12 need engine work/statements; 11 center-skip not modelled |

Not done: Lv55 double candy as an explicit Lorelei-to-Lance reserve (model eats all kept candies at indigo-heal; extra candy pickups are item-agent territory).

## Aerodactyl/museum frame cost (coordinator request)
No stream gives a timed museum+revive: A1 (11-14) never shows the Pewter detour or a revive (see its section 3 pewter note); the only measurements are
"54 s per catch" for the 3-catch Pewter bundle (T03 1:16:59-1:17:32, T06 1:49:49 "55 s/Pokemon", STRATEGY 6 lever table), which is an average over
Aerodactyl, Diglett and the Mime trade. 54 s = 3225 f. The model's whole bundle (3600 + Mime 2600 + Diglett walk/hunt) is already about 160 s, matching 3 x 54;
there is no evidence for 2400. With 3225: 1043 WR+2m -3.8 h [-6.3,-1.5], WR -8.8 h [-14.4,-4.5] (R 800k, seed 114). Constant left at 3600 (guess, error bar +-400 f);
if it is changed it is a model recalibration, worth ~1.5-2.5%.

Addendum: seed 112, R 3.2M, WR rung: HP IV 5 +25.5 h [-3.2,+52.5] (n.s.), Venonat second hunt +154.6 h [+132.7,+180.7]. Verdicts unchanged.

Seed 113 (R 3.2M) ran after all: no-Persim (no cost) WR+2m -5.2 [-7.5,-3.3], WR -12.4 [-17.2,-7.8]; with the guessed Misty cost n.s. (-3.0, -15.9); door gate +0.2/+0.3 (WR +0.3/+0.5). Same verdicts as seed 112.
