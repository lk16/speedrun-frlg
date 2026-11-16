# 2026-11-23 (later): League retune result, CURRENT = 1019-guard-mew2x

Continues `2026-11-23-10-00-league-retune-diagnostics.md`. Own driver `lgdesc.py` (eval at R 3.2M seed 11, 3% bar, seed-12 confirmation, immediate accept); the 299 screened candidates are in scores.jsonl as `lg2-cand` rows, the accepts as `lg2-*` / plan rows.

## Accepts (WR+2m EHT)
Base 1017 913 h. Champion queue `guard, speed, special` with 2 Guard Specs bought (needs the Mist AI fix, commit c6d8084) 604 h; Lorelei `heal_below` 0 584 h; Lance `+special` twice (queue special, accuracy, special, special) 508 h;
Lorelei-2 `+special` 488 h; Champion-2 `special, special` 393 h. Pass 0 ran all 299 candidates; pass 1 was not run (wrap-up).

## Fresh-seed confirmation, R 3.2M
WR+2m: 404.4 h (seed 31, 1943 finishes), 375.4 h (seed 32, 2092). WR: 617.0 h (seed 31, 1045 finishes), 615.5 h (seed 32, 1047). 1017 at WR seed 31: 5047 h, 90 finishes. The WR rung is populated.

## Engine League difficulty at WR-like play (no `fatal`, R 100k seed 3), 1017 -> 1019
Engine-only survival R1: Lorelei 100 -> 97%, Bruno 92 -> 92, Agatha 94 -> 94, Lance 82 -> 93, Champion 38 -> 61; R2: 99 -> 100, 100, 99, 97 -> 96, Champion-2 39 -> 49.
Player actions, engine vs WR recount: Lorelei 12.7 (13), Bruno 7.2 (9), Agatha 6.5 (7), Lance 11.0 (10), Champion 13.4 (12), Bruno-2 5.5 (5), Lance-2 10.2 (9), Champion-2 10.8 (11.5).
Action counts match within about a turn; the remaining gap is a 40-50% engine loss rate at both Champions (Alakazam's Psychic, Venusaur's Solar Beam one-shot from full HP). The WR's one Champion draw cannot tell if that is too hard.

## Caveats
* Lance's 3 Specials and Champion-2's 2 are above what the WR used (1 and 0) but inside its bag (11 and 3 Specials left); they compensate the fitted stand-ins (league_power 1.4, mewtwo_power 1.2). Champion uses fewer Specials than the WR (1 vs 3): the engine's optimum.
* Untouched: sacrifices 0, candy cap 6, `no_salvage_until rival-ss-anne`, Rash/Mild, the flat League `fatal` rows (S, still about half of R1 attrition), evidence guards.
* Not done: early / mid / all descents on the new base (`descend.py --set earlymid / all`). The last two accepts (Lorelei heal_below, Lorelei-2 special) are near the noise level; re-check at R >= 6M.
* Tooling note: a 3.2M-attempt `eval` takes 21 s, so the earlier "2 min per candidate" was the paired-CI growth of `try.py`, not the simulator.
