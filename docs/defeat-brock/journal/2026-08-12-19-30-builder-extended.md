# 2026-08-12 19:30 — the builder learns targets; the brock segments exist

`frlg route build --target defeat-brock` now runs rival-1's nine segments and continues:
lab exit → Route 1 → parcel → delivery → tutorial → Route 2 → forest → Pewter →
`FLAG_DEFEATED_BROCK`. The ledger records its target; rival-1 stays the default everywhere
and its files are untouched (its route test still passes).

Design decisions worth remembering:

- **Encounters are handled by the path search first.** A battle-bound edge reads as
  *blocked* in `nav`'s Dijkstra (a fix worth its own commit: an encounter edge is cheap —
  it ends at the position change — and would otherwise claim a tile's best-cost slot with
  a state that is a battle, walling the tile off for clean paths). Because every tree edge
  replays from its parent's exact savestate, any found path is encounter-free *by
  construction* — the search implicitly shapes around the second LCG's fated passes by
  approaching tiles at different rate-test indices. Only when no clean path exists does
  `walk_fleeing` force a step (bias direction), take the battle, flee it (wild, delay
  search on the escape roll) or win it (trainer, the two-stage delay search), and search
  again — and the battle resets the encounter cooldown, buying 6-7 nearly-free grass steps.
- **Move selection is one navigation, not per-turn**: `gMoveSelectionCursor` persists
  within a battle, so the Brock search steers to Bubble/Vine Whip at the first fight menu
  and A-mashes thereafter.
- **`gBattleOutcome` is stale between battles** (`BattleStartClearSetData` zeroes it,
  `battle_main.c:2265`) — every battle helper syncs on the first action menu before
  reading it.
- **Semi-naive starter: Squirtle.** The exp ledger says Bubble (L7, 236 exp) is covered by
  the two mandatory fights alone (135 + ~69 rival + ~99 Sammy = 303), while Bulbasaur's
  Vine Whip (L10, 560) needs the Camper Liam detour. Whether the L7 Bubble race against
  Onix's Rock Tomb actually wins — and whether Bulbasaur's stronger fight beats the
  detour cost — is for the build and the later sweep, not this journal.

First full build running. Expected weak spots, in advance: the forest's big Dijkstra
budget, the tutorial trigger approach, and the Brock fight's win condition at L7.
