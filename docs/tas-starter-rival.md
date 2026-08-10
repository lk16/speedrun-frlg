# TAS: pick a starter, beat the rival

The first real route segment: power-on to `gBattleOutcome == B_OUTCOME_WON` in the Oak's Lab rival
battle. Everything below is tier-1 only (libmgba, in this sandbox). Nothing here claims tier-2
acceptance; BizHawk does not run here.

## Ground rules

- Every routing claim cites a path under `decompiled/`. A claim that cannot be cited is labelled a
  guess, in the text, where it is made.
- The canonical artifact is the raw `u16`-per-frame input log (`.ilog`), not a `.bk2` -- the `.bk2`
  column order is not derivable in this sandbox (`docs/harness.md`).
- A segment is only "done" when a replay *from reset* reaches its observable, and the ledger records
  the digest, the frame cost and the RAM fingerprint that proves it.

## Checklist

### Infrastructure

- [x] `decompiled/` symlink to `$FRLG_DECOMP` restored at the repo root, and documented as the
      citation root (it is gitignored, so every sandbox has to recreate it)
- [x] `frlg-route` crate with a `Recorder`: drives an `Emu`, records exactly one mask per frame, and
      exposes `hold`/`wait`/`tap`/`until` so a segment is code, not hand-counted frame numbers
- [x] `observe` module: named RAM probes with decomp citations -- map group/number, player position
      and facing, main callback, script-context state, menu cursor, battle state, battle outcome,
      party species/level/HP
- [ ] `route/ledger.json`: segment -> parent -> input-log digest -> frame cost -> tier-1 evidence ->
      tier-2 status, plus a `frlg route` subcommand that verifies it rather than trusting it
- [ ] `docs/journal.md` started: what was tried, what failed, what is next

### Route

- [ ] Segment 01 `boot`: reset -> title -> main menu -> NEW GAME accepted
- [ ] Segment 02 `intro-oak`: Oak's speech through the boy/girl choice
- [ ] Segment 03 `names`: player name and rival name chosen (preset names, if presets are cheaper --
      measured, not assumed)
- [ ] Segment 04 `house`: bedroom -> ground floor -> outside, observable is the map change to
      Pallet Town
- [ ] Segment 05 `to-lab`: north out of Pallet, Oak's interruption, arrive inside Oak's Lab
- [ ] Segment 06 `starter`: walk to the ball, take the starter, party count 1 with the species
      recorded
- [ ] Segment 07 `battle-start`: rival battle entered (battle-state observable set)
- [ ] Segment 08 `battle-win`: `gBattleOutcome == B_OUTCOME_WON`
- [ ] Starter choice decided by measurement: frames-to-win compared across all three, result written
      up with the numbers

### Verification

- [ ] One-pass replay from reset over the concatenated log reproduces every segment observable, and
      the final RAM fingerprint is recorded
- [ ] Checkpoint savestates per segment in `$FRLG_ARTIFACTS/states`, `bin/frlg-artifacts-gc` run
      after
- [ ] `cargo test --release` gains a test that replays the committed route log and asserts the win
- [ ] `cargo fmt` and `cargo clippy` clean
- [ ] `docs/route.md` written: the route, its observables, and the evidence for each claim
- [ ] Ledger records tier-2 as blocked on `route/template.bk2`; no `.bk2` is emitted until that file
      exists

### Optimisation (only once the route wins at all)

- [ ] Per-segment frame costs in the ledger, obvious idle trimmed, each trim re-verified
- [ ] The battle's RNG use understood well enough to say whether the win is manipulated or
      luck-independent, with the decomp citations for the damage/crit path
