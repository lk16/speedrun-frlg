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

## Seed wave and the 40106 adoption

With the planner priced in, four full builds ran in parallel against the
incumbent seed 38 (40865): seed 27 → **40106**, 36 → 41935, 39 → 43287,
40 → 44309. Seed 27 is the old scan's modeled-best that placed third under
the old walker — the planner realizes it: forest 6919, tutorial 5582,
to-viridian 2661 (worse), rival 3024 (worse, delay 44), net −759. No
fallback and no planner divergence anywhere on its build log. Tier-1
verified; export `route-40106f` queued (the unrun 40940/40865 requests were
pulled). Wave wall-clock: ~19 min for all four.

## Rival window probe: 256 delays reproduce 40106 exactly

Doubling the rival's start-delay window (128→256) on seed 27 found the same
optimum (delay 44, 3024; 62/256 win) and the `--from 09-battle-win` rebuild
reproduced every downstream segment byte-for-byte — a free determinism check
of the whole planner pipeline. The widening stays (search time only).

## Second optimisation pass (same day): reset model, knobs re-swept

- **Planner models the battle reset now** (`ResetEncounterRateModifiers`
  on battle start): the A* node carries fails-since-reset, so a planned
  flee reopens the belt behind it. The from-exit-lab probe on the old
  knobs came out 40162 (+56) — flee placement improved but battle-arrival
  streams shuffled Sammy/Brock by more; the model stays, the run did not.
- **text_hold is seed-coupled.** Re-swept on seed 27: text 4 → **38950**
  (−1156), text 1 → 39251, both beating the text-2 incumbent 40106. Every
  segment improved at once under text 4; Brock's fight hit 3166. Adopted,
  tier-1 verified, export `route-38950f-2f221a89` queued (40106 pulled).
- Next wave in flight: text 3/5/6 and turn 2, then wider seeds under the
  winning knobs.

## Battle-search economics (asked: can the Rust engine replace it?)

Where a ~13-min build goes: battle delay searches ~6 min (rival 256 +
Sammy 192 + Brock 384 candidates, each a full emulated battle), plain
playthrough emulation ~4 min, the rest walking/writes. Three cuts landed:

- **Running-best abort**: a candidate past the shortest completed winner
  cannot win; a shared atomic cap cuts it off. Provably selection-identical
  to the uncapped search; ~30-40% off battle time.
- **FRLG_WORKERS**: cap the per-process pool so sweep waves stop
  oversubscribing (4 builds x 14 threads on 16 cores was ~2x slowdown).
- **frlg route scan**: seeds screened in the model (~5 s each) instead of
  built (~13 min each); ranking validated by seed 27 landing #1.

Replacing the searches with frlg-battle wholesale is *possible* -- the
rival fight proves the method -- but pacing.rs is fitted per fight (~280
instrumented battles for the rival alone) because rolls and frames are
mutually dependent, and Sammy/Brock add poison, a mid-fight level-up
prompt, the special split, type chart, gym AI and a party switch.
Estimated ~a day for both, saving ~4 min/build: worth it when the route
extends past Brock, deferred for this target.

Also: wave A (seeds 20-23, pre-scan) confirmed the scan's ranking -- all
lost badly (43750/46930/42971) and seed 21's build died silently after
03-names with nothing in its log (one-off, unreproduced, noted).

## Sweep closed: 38950 stands

Wave B (scan-guided, seeds 38/26/13/6 at text 4): 41399 / 42938 / 43391 /
43297 (seed 6 rerun after its first attempt died). Every scan-top-5 seed
has now been built and lost to seed 27 -- the seed x knob neighborhood is
exhausted and **38950 (seed 27, turn 1, text 4)** is the final number of
this session. Tier-2 request `route-38950f-2f221a898d8e` remains queued.

Two operational findings worth keeping:

- **The "silent" build deaths were bus errors, self-inflicted**: rebuilding
  `target/release/frlg` while wave builds were running corrupted their
  mapped executable (seeds 21 and 6, both timed with a `cargo build`).
  Waves now run from a snapshotted binary copy in scratch.
- **The running-best abort measured**: the full Brock re-search (--from
  brock, 384 candidates + stage 2) runs in 2m41s at 8 workers and
  reproduces 38950 exactly -- versus ~4-5 min at 14 workers uncapped.
