# 2026-08-12 21:30 — four build attempts, three lessons

The first full builds of the defeat-brock target found real bugs and one real routing fact.

1. **Per-tile path caching cannot cross grass.** The encounter-rate test's pass/fail is
   indexed by test count (the second LCG), so every minimal path reaches a frontier tile at
   the same index — one fated pass walls off the whole belt, and Dijkstra discards the
   index-shifted longer paths that would cross it. Fixed by keying search nodes on
   `sWildEncounterData`'s decision state (`Observer::wild_key`). The honest A* then drowns
   in same-row alternates (~120 ms/expansion, thousands of nodes per belt), so the crossing
   searches run greedy (`Goal::AnyOnVia`, VIA_SCALE 64) and fall back to *taking* a battle:
   fleeing resets the cooldown (`src/battle_setup.c:205`) and buys 6-7 nearly-free steps.
   Optimal grass routing is explicitly deferred to a model-driven search (frlg-mon has the
   model; it needs map-attribute data to run off-emulator).
2. **Two walk bugs**: the force-step required `player_can_step` mid-hold (never true — the
   first build's failure), and wall probes burned the full 240-frame edge budget (now bail
   at 64 frames of no movement with free-standing observed).
3. **A whole stream family can lose the rival mirror.** Build 3's one-frame prefix shift
   (from the nav changes) landed on a naming-exit seed whose battle loses **all 128** start
   delays — consistent with frlg-battle's finding that start delays collapse mod 5, so 128
   delays is only ~26 distinct battles. The escape dial is `text_hold`: it moves the
   naming-screen exit *frame*, which re-seeds the battle stream outright (`SeedRng` from
   timer 1, `naming_screen.c:722`), where `turn_hold` only slides within the already-searched
   window. Two builds (text_hold 2 and 4) are racing in parallel.

Also: `frlg-battle`'s engine could predict a winning family instead of rebuilding blind —
wiring it to the brock prefix (Squirtle stats from the rolled IVs) is on the optimization
list once the semi-naive run exists.
