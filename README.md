# speedrun-frlg

An experiment in how powerful today's LLMs actually are: **can an AI build a tool-assisted
speedrun (TAS) of Pokémon FireRed/LeafGreen that is faster than what humans have created?**

A TAS is the perfect-play version of a speedrun: inputs are written frame by frame against an
emulator, so the ceiling is not human reflexes but understanding — of the game's code, its RNG,
its menus, its movement engine. That makes it a good benchmark for machine intelligence. Human
TASers have spent two decades building routes, RNG manipulations and glitch inventories for
these games; matching them means genuinely understanding the game, and beating them means
finding something they missed.

## The rules of the experiment

- **The AI (Claude, in a sandbox) does the routing.** Sessions run inside a network-closed
  sandbox: no TAS videos, no forums, no speedrun wikis, no downloads. The one reference is the
  [pret/pokefirered](https://github.com/pret/pokefirered) decompilation, mounted read-only.
  Every routing claim must cite a file in it; anything the model "remembers" but cannot cite is
  labelled a guess. The run has to be *derived*, not recalled.
- **Everything is verified twice.** Tier 1: a headless libmgba harness in the sandbox, replaying
  every input log from power-on and checking RAM against the claims (`docs/harness.md`). Tier 2:
  BizHawk on the host replaying the exported `.bk2` movie — the same format a human TAS is
  published and judged in. A route is only "done" when BizHawk agrees frame for frame.
- **FireRed or LeafGreen, whichever is faster.** The two versions are typically one speedrun
  category, so the route is free to pick; that choice is measured, not assumed.

## The targets, in order

1. **Defeat rival 1** — the first battle of the game, on Oak's lab floor. Small enough to build
   the entire toolchain around (emulator harness, input-log format, movie export, two-tier
   verification), and already a real TAS problem: the battle is decided by RNG manipulation.
2. **Defeat Brock** — the first badge. Adds wild encounters, trainer dodging, and a real level
   of route choice.
3. **A full glitchless run** — game start to Hall of Fame, the classic category.
4. **A "round 2" run** — through the post-game rematch, which requires catching 60 Pokémon in
   the Pokédex along the way. Routing catches is a different discipline than routing battles.
5. **Then back to the start:** rebuild the rival-1 TAS with everything learned on the way.

## Where it stands

The first target is routed end to end: power-on to `gBattleOutcome == WON` in **9658 frames**
(~2m42s) on LeafGreen with Bulbasaur, tier-1 verified, every segment's evidence recorded in
`route/ledger.json`. An early BizHawk replay desynced — traced to a boot-timing difference
between the two emulator setups, now fixed — and this movie has since passed tier 2
(`route-9658f-269d169cd6db`: all 9658 frames replayed, probe matching every one). The battle
itself is chosen from a searched set of RNG outcomes rather than lucky, and version, starter
and the tuning knobs are swept rather than assumed — but plenty is still unoptimised, and
`docs/route.md` keeps the honest list of what.

## Reading order

| File | What it is |
| --- | --- |
| `docs/route.md` | the route, its evidence, and what is not yet optimised |
| `docs/journal.md` | the lab notebook: what was tried, what failed, what is next |
| `docs/harness.md` | the tier-1 emulator harness and the input-log format |
| `docs/sandbox.md` | the sandbox environment the AI works in |
| `crates/` | the Rust toolchain: emulator FFI, harness, route builder, CLI |
| `route/` | the committed input logs, the ledger, the BizHawk movie template |
