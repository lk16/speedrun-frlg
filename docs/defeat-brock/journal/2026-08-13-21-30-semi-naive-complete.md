# 2026-08-13 21:30 — the semi-naive run is real: 49143 frames, tier-1 verified

`frlg route build --target defeat-brock --starter squirtle --text-hold 4` runs from
power-on to `FLAG_DEFEATED_BROCK` in **49143 frames** (~13m43s), and
`frlg route verify --write` replayed every committed log from reset and stamped tier 1 on
all 19 segments. The ledger and logs are committed; the movie is queued for tier 2.

The day's finds, in the order they bit:

- **Rick's ambush** froze the first forest walk: a sight-line script locks field controls
  and waits on its intro text. Walks now drive any ambush to its battle and win it.
- **The forest is a maze**, and six hours of blind round-robin at its dead-end block
  bought the lesson: decode the committed layout (`research/forest-map.txt`) and waypoint
  the canonical corridor. Sammy is confirmed forced; Rick and Doug are on the corridor and
  their exp turned out load-bearing.
- **The nurse is spoken to across the counter** (MB_COUNTER); the naive talk tile was
  solid wall and sent the tile-goal search out the door — tile goals now stay on their map.
- **Brock at 6/28 HP loses all 192 start delays; healed, 188/192 win** — the Pewter
  Pokémon Center segment (`heal-pewter`, 1754 frames) is the single change that made the
  fight winnable, and the first named optimisation fork for later.
- Brock falls in **2 turns** to Bubble (plan `[129]`).

Segment names dropped their numeric prefixes before the first commit: order lives in the
ledger, names describe intent, and future re-orderings (or deleting `heal-pewter`) will
not ripple through logs and docs.

Next: tier-2 verdict when the host runs the queue; then optimisation per route.md's
ranked list — model-driven encounter dodging first (it owns ~8-10k frames of flee/walk
bloat), then seed-family and starter sweeps.
