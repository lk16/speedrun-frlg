# 2026-08-12 (sandbox, small hours) -- the battle engine closes: pacing measured, gates enumerated, whole battles predicted without the emulator

v2 exists: `frlg_battle::engine::simulate` plays the rival battle forward from a
`gRngValue` and a delay plan alone -- move choices, damage, HP, win or loss, and the
frame each roll lands on -- in ~a microsecond per battle, where the emulator pays ~1 ms
per *frame*. `examples/pure-search.rs` enumerates 3125 plans in 100 ms and reproduces the
committed [4, 3, 3, 3] at exactly 2409 frames from `Rng(0xed94271d)` with no emulator in
the loop (also a unit test, so it cannot rot silently).

**The v1 AI model was wrong, and the fitter caught it exactly as designed.** Growl is
never penalised by `AI_TryToFaint`: its power is < 2, so `if_can_faint` falls through and
`get_how_powerful_move_is` returns `MOVE_POWER_DISCOURAGED`, not `NOT_MOST_POWERFUL`
(`battle_ai_script_commands.c:966-1025,1475-1500`; the penalty jump is
`data/battle_ai_scripts.s:2767-2771`). And Scratch gets **+4** when its simulated damage
-- `AI_CalcDmg` with no crit, scaled by `simulatedRNG[slot 0]`, minimum 1 -- would faint
us. So Growl ties Scratch at 100 whenever our ATK stage is 6, our HP is above 70%, and
AttackDown4's roll comes up < 50 (p = 50/256), and the tie-break then Growls on odd rolls:
the rival Growls in ~10% of turns, at most once per battle (after one Growl our stage is
5, Growl takes -1, and 99 never ties 100 again). The old model said "never Growls" and
passed, because the committed battle happens not to contain one. 28 of ~200 training runs
did contain one, and every one of them failed the labeler's HP cross-check until the
scores matched the scripts. With the fix: **zero label failures over ~280 runs**,
including every Growl turn and every loss.

**The pacing, measured** (`examples/fit-pacing.rs`, constants in `src/pacing.rs`; ~280
instrumented battles over start delays, per-turn delay sweeps, and stream shifts
-10..=10). The anatomy of a turn is rigid: the turn-end roll and the `choosing_actions`
flip share a frame (`det`); the whole AI block rolls at `det+5` regardless of the plan's
delay; the commit mash starts at `det+delay+1`; the player's crit roll comes exactly 30
frames after the resolution mash starts; crit->damage is 3; the HP write follows the
damage roll by a pure function of the HP actually lost (kills drain only what is left) --
77..101 frames for deltas 1..10 on the rival's bar, +125 for Oak's first-hit
interjection, a slightly different table for our bar (20 max HP vs 18: different pixel
granularity); "A critical hit!" costs 79 frames between HP write and secondary roll; a
first Growl runs 29 frames from accuracy roll to the stage write and 213 more to the
turn-end roll (Oak again). Every one of those is single-valued across the whole fit.

**The intro collapses start delays mod 5.** The mash period is `text_hold + 1 = 5`, and
every intro text press quantizes to its grid: delays d and d+5 produce byte-identical
battles -- same pre-turn frame, same stream, same outcome, same total (verified over all
64 delays; the old "38/64 delays win" is exactly 3 residues of 5). Stage 1 of the search
was 64 emulator battles; it is five.

**What the model cannot decide, it enumerates.** Two moments are input-gated on scene
state the model cannot see (tested against detection parity, mash phase, turn index --
none classifies the residue): how many commit presses whiff (duration 8/13/18; delays
0..=3 are pair-{13,18} on hundreds of samples, sparser delays carry the full union -- a
held-out run promptly produced a 13 where three fit samples had said "always 8"), and a
±5 slip in the endgame text chain. `simulate` returns one `Leaf` per gate combination
(a handful per battle) instead of guessing. The evidence that this is sound and exact:
`tests/engine_vs_emulator.rs` plays twelve held-out (shift, plan) cases the fit never saw
-- shifts to ±20, five-turn plans, delay 15s -- and requires the leaf whose commit
durations match the emulator's to predict the result, frame-exact on wins. It does, on
all twelve.

**What this buys the route.** A plan whose every leaf loses is discarded for free;
survivors are ranked by their best winning leaf and only those need the emulator. The
pure search already surfaces plans whose best leaf is **2405** (-4 on the committed 2409)
-- e.g. [4, 0, 6, 4, 6] -- but a best leaf is an *if the gates cooperate* number, so
those are candidates to verify next session, not results. **Unverified: the 2405s.**
Also worth knowing: delays 0..=3 have two-leaf gates (least ambiguity), so plans built
from small delays are the cheapest to arbitrate.
