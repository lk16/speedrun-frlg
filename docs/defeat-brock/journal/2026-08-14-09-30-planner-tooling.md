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

## Shakedown build (old executor, 46230 — measured 2026-08-14 mid-session)

First full build with the planner + parallel battle search, before the
executor fixes landed. Segment deltas vs the accepted 43308:

- rival battle 2580 (−78): the checkpointed pool search with 1..24 turn
  delays beat the old greedy walk's 2658.
- deliver 4490 (−541), tutorial 6220 (−266), to-forest 1149 (−158): fully
  planned walks (Route 1 southbound uses the ledges).
- exit-lab/parcel (+~130): the arrow-warp/door bugs; fixed.
- **forest 13553 (+3758)**: the settle-timeout bug forced the waypoint
  fallback the whole way, and this walker's step pattern hit fated rate
  passes the old run's pattern missed — the flee count is path-dependent
  (`research/wild-encounters.md`), so a *worse path* on the same seed flees
  more. This is precisely the case the model-driven plan exists for.
- brock segment 4954 (+65), battle itself 3209 in 2 menus (delay 84 + turn
  delay 11) — the wider delay range finds a 2-menu fight.

Total 46230 — a regression end to end, but every regression is a fixed
executor bug, and every planned leg that ran beat its old segment.

## Candidate build adopted: 40940, tier-1 verified, tier-2 queued

Second full build, executor fixes in: **40940 frames** (−2368, −5.5% vs the
accepted 43308), same knobs. All 19 segments tier-1 verified from reset;
export `route-40940f-df8de7f27de8` queued for tier 2 (bk2 round-trip
checked). Segment story: forest 8137 (−1658, one planned A* crossing, Sammy
the only fight at 2823), deliver 4485 (−546, ledges), tutorial 6275,
to-forest 1147, to-viridian 2112, rival 2580; brock 5203 (+314 — its
stream moved; top backlog item). The diagnostics also caught the executor's
consumption check reading one step late (tile-center vs pos-change timing)
— accepted-and-replanned each time, costing ~0 frames but worth fixing
before the next optimisation pass (backlog #3 in route.md).
