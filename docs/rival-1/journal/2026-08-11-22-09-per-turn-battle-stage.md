# 2026-08-12 (sandbox, later) -- the battle search grew a per-turn stage: 10946 -> 10531

The battle was the one big item left after the morning's rebuild (entry below), and the "obvious
next machine" the route notes had described since 2026-08-11 now exists. `gBattleMainFunc`
re-enters `HandleTurnActionSelectionState` at the top of every turn (`BattleTurnPassed`,
`decompiled/src/battle_main.c:2998`), so each turn's action menu is a decision point; stage 2
of the search walks the stage-1 winner's turns greedily, tries idling 1-15 frames at each
menu with the rest of the battle replayed in full per trial, and adopts only a shorter battle.
Result: **battle 3322 -> 2907, total 10531**, plan `[0, 1, 0, 1, 6, 1]` over 9 turns, tier-1
verified from reset, exported and queued as `route-10531f-e037421ddd87` (the 10946 queue entry
was withdrawn unreplayed).

Two observations worth keeping:

- **The landscape is rugged, and the first step was huge.** Stage 2's first adopted delay --
  one idle frame at turn 1 -- cut ~1200 frames: a different damage-roll lineage, not a trim.
  The three later adoptions were worth a frame or two each. That asymmetry says a second
  greedy pass (or a joint start x turn search) could pay again; single-pass greed is in the
  route notes as the top remaining battle item.
- **The mash's phase is part of the stream.** Rewriting the drive from one continuous mash to
  advance-to-observable stages realigned which frames carry A presses, and the same 64 start
  delays produced a different family of battles (64/64 winning where the previous shape had
  32/64, stage-1 best 4109 where the old mash's was 3322). Neither shape is wrong -- the log
  is the artifact and it verifies -- but battle numbers are only comparable within one drive
  shape.

**Answered while the sweep ran: the committed battle's one crit is ours.** Tracing
`gCritMultiplier` and `gBattlerAttacker` over the replayed `09-battle-win` log shows a single
crit window (multiplier 2 from battle frame 1488) with attacker 0, `B_POSITION_PLAYER_LEFT`
(`decompiled/include/constants/battle.h:28`). The previous route ate a *rival* crit; this
stream hands the crit to us. Luck found, not aimed for -- noted in `docs/rival-1/route.md`.

**Unverified.** Tier 2 for `route-10531f-e037421ddd87`: queued, not replayed. The `turn_hold`
sweep on the final code is running as this is written; its result lands in the next entry.
