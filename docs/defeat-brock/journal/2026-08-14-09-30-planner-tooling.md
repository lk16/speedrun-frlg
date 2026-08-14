# 2026-08-14 09:30 — the walking becomes a planner, iteration gets --from

Session goal (from review of the accepted 43308 run): fix the tooling before
re-optimising — the forest cost hours of emulator search, the run has useless
encounters and visible up-then-down walking, and the backlog's own top item was
the model-driven path search.

## Landed

- **`world.rs`** — maps decoded from the decomp's data files (map_groups.json,
  map.json, layouts.json, map.bin, metatile_attributes.bin; formats cited in
  the module docs). Collision, grass (encounter-type LAND), behaviors, ledges,
  objects with trainer sight, warps, coord events. The forest decode matches
  the hand-made `research/forest-map.txt` cell for cell (unit test).
- **`plan.rs`** — A* over `(tile, cooldown, rate-test index)` with the wild
  second-LCG pass/fail sequence precomputed from `sWildEncounterData`
  (`research/wild-encounters.md`). Sight cones and NPC wander boxes priced;
  behavior-boundary steps get both a consume and a skip edge, priced at the
  expected delay cost of steering the 60% gate. Forest entrance→exit plans in
  ~0.1 s (was: most of an hour per Dijkstra exhaustion, per round).
- **`walk_planned`/`walk_smart` (brock.rs)** — the executor: try each planned
  step from a savestate, steer wrong gate branches with 1-frame delays
  (`STEER_DELAYS`), learn blocked edges from NPC collisions, replan on any
  divergence (a replan is milliseconds). `walk_fleeing` stays as fallback;
  the forest keeps its waypoint chain only for the fallback path.
- **`frlg route build --from <segment>`** — replay the ledger's prefix logs at
  emulator speed with a RAM-hash check, rebuild only the tail.

## Torrent / crit notes (user asked)

- Every Squirtle has Torrent — the species' ability slots are
  `{ABILITY_TORRENT, ABILITY_NONE}` (`src/data/pokemon/species_info.h:236`,
  Squirtle entry). Nothing to manipulate at the ball for the ability itself.
- Activation is `hp <= maxHP/3` and boosts water move *power* ×1.5
  (`src/pokemon.c:2500-2501`). At the run's L7 (23 max HP) that means
  arriving at Brock ≤7 HP. Napkin math with gen-3 integer floors: vs Geodude
  Bubble goes 13-16 → 20-24 per hit (saves ~1 turn); vs Onix the /50 floor
  eats the boost entirely at these stat values. Setup costs enemy turns
  elsewhere and survival at ≤7 HP needs miss/damage manipulation. Verdict:
  *measure later against the rebuilt route, likely marginal*; not assumed
  into the route.
- Crits: the per-turn delay search (`win_battle` stage 2, delays 1..16) is
  already the crit-manip mechanism — a crit that shortens the fight wins the
  shortest-wins scoring. Widening to 1..24+ is cheap with the checkpointed
  searcher; try on the rebuilt run.

## In flight

- Full planner-driven build into scratch (same knobs as the accepted run:
  squirtle, turn 1, text 2, seed 38) to compare against 43308. Unverified
  until `frlg route verify` replays it; nothing committed to `route/` yet.

## Next

- Compare per-segment frames vs the 43308 ledger; adopt if faster, verify
  tier 1, queue tier 2.
- Then: joint seed × path sweep (the planner makes each seed's walk cheap to
  price), wider Brock delay search, Torrent probe.
