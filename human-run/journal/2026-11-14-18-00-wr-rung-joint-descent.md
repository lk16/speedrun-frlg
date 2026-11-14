# 2026-11-14: joint descent at the WR rung on plan 450 (dex + early/League params together)

Objective fixed at the WR rung (`descend.py --rung WR --set all --r1 160000 --r2 320000`): every candidate is screened
on seed 1 at R = 160 000 (no early stop; `try.py` honours `FRLG_R0`) and an accept must be confirmed on seed 2 at
R = 320 000 before promotion. `descend.py` now continues from the same position after a promotion instead of
restarting the pass. 706-731 candidates per pass, 4 passes, 54 promotions (plans 451..555), converged (the last
pass had 1 promotion).

## Result (fresh seeds 21, 22, R = 400 000, paired vs 450)

| | 450 | 555-order343 |
| --- | --- | --- |
| EHT(WR) | 26.4 / 26.1 h (6165 fin) | **5.00 / 5.00 h** (63 450 fin), diff -21.4 h CI [-21.9, -20.8] |
| EHT(WR+10m) | 6.37 h | **4.63 h**, diff -1.73 h CI [-1.76, -1.69] |

## What mattered (WR-rung EHT, approx. diff at promotion)

* Evolutions: dropping Exeggutor-by-stone at the `stones` anchor -3.8 h and re-adding it after Victory Road (Lv 25 pre) -1.5 h;
  dropping Ninetales -0.25, Clefable via bought stone -0.24, then ~10 more evo moves/drops. Removing the Nidorina/Nidorino
  candies or the Nidoqueen/Nidoking/Clefable/Raichu stones costs +2..+18 h, so those stay.
* Mr. Mime trade: dropped -1.7 h, re-added later (-0.2) once the Abra timing changed; Snorlax static dropped.
* Hunt/fill economy: stop_after 2->1 on Berry Forest, Bond Bridge, Route 4/9/23, Safari North steps/removal, fill step
  scaling and order swaps (about 15 small accepts of -0.1..-0.3 h each), Route 8 hunt added.
* Reset aggressiveness: global 0.94 -> 0.84, Blaine 0.75, Sabrina 0.7, Indigo 0.8, Brock 0.95, Celio 0.74, Surge 0.9.
* Tactics/bag: Silph rival loses X Special/X Speed and sacs 0->1; later tactic, X-buy and heal-bag tweaks were each small;
  `rope_fills` on (seed 11: WR 7.69 -> 5.02 h).
* Ball rule poke_from 255 ultra_below 0, ball_batch 40->20, candy_early_n 1.

## Power refit

`frlg-human splits` fit line (mean |log(model/WR)| actions / core seconds) on 501: carry_power 1.6 .181/.138, 1.75 .199/.116,
1.9 .187/.107, 2.05 .173/.104, 2.2 .181/.099; league_power 2.0 .191, 2.2 .187, 2.4 .195, 2.6 .197; mewtwo_power 1.3 .187,
1.5 .187, 1.7 .185, 1.9 .191. The surface is flat within noise (R = 20 000-40 000), so 1.9 / 2.2 / 1.5 stay. The non-League
action mismatch (0.31) is not a power effect (Brock 2.6 vs 2.0, Sabrina 5.3 vs 9 WR actions).

## Sanity alarm (read before trusting 5.0 h)

The no-reset finishers of 555 have p50 3:20:17, min 3:10:34, i.e. 5 min under the WR median. The first half is calibrated
(+2..5% on the WR splits for finishers, dex 61 at Start E4R1), but Icefall Cave -> End is 4.5 min faster than the WR
(End -2.3% vs +3.3% at Start E4R1; WR 1912 s from Champ 1 to End, model 1681 s). That is the known League/post-game
residual (journal 2026-11-14-10-00), not a discovery. The ranking of plans is informative; the absolute EHT(WR) is
optimistic until the post-E4R1 segment costs are recalibrated. The starter reset phase (another agent) is untouched.

## Next

* Recalibrate the post-Champ-1 half (Icefall, Warehouse, rematch walks/text) against the WR splits, then re-descend.
* Winner's curse is controlled by seed 2 at 2x R and a fresh-seed check (seeds 21/22 above).
