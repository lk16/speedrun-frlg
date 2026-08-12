# 2026-08-12 (sandbox, evening) -- the stale turn_hold falls to a re-sweep: 10531 -> 10085

`frlg route tune`, re-run for the first time since mGBA 0.10.5, and the stale knob was worth
446 frames: **turn_hold 2 wins at 10085** (battle 2466, plan `[1, 10, 0, 15]`, 5 turns),
where the carried-forward 8 scored 10531. Full sweep, every variant a complete build through
the two-stage battle search: 1 -> 10541, 2 -> **10085**, 3 -> 10540, 4 -> 10386, 5 -> 10267,
6 -> 10087, 7 -> **loses**, 8 -> 10531. Tier-1 verified from reset, exported and queued as
`route-10085f-65ef20333a57`; the 10531 queue entry withdrawn unreplayed. `Tuning::default()`
now says 2, with the sweep in its comment.

Two things the sweep surfaced beyond the number:

- **A knob value can lose outright, and the sweep must survive that.** turn_hold 7's stream
  gave 64 losing battles out of 64 start delays -- stage 2 never got a winner to refine. The
  first sweep aborted on it (the variant's build error propagated); `frlg route tune` now
  scores a timeout as "loses" and keeps sweeping, because a value that cannot win is an
  answer, not an outage.
- **The two leaders sit 450 frames clear of the pack and 2 frames apart** (10085 vs 10087 for
  turn_hold 6), and both got there through different battle plans on different streams. The
  knob is not doing anything mechanical -- it is picking which RNG lineage the battle search
  gets to fish in, same as every upstream frame.

**The crit census moved with the battle**: the 2466-frame battle contains *two* crit windows,
both attacker 0 = us (`gCritMultiplier`/`gBattlerAttacker` traced over the committed log,
`B_POSITION_PLAYER_LEFT`, `decompiled/include/constants/battle.h:28`). Two crits in five turns
is why it is short -- and it is found luck, not aimed-for luck; the route notes keep that
distinction.

**Unverified.** Tier 2 for `route-10085f-65ef20333a57`: queued, not replayed. Starter choice
and LeafGreen remain the two untouched fronts (`docs/rival-1/route.md`).
