# Defeat Brock: the route

**Status: 40940 frames, tier-1 verified, tier-2 queued.** (~11m25s at 59.7275 Hz)
from power-on to `FLAG_DEFEATED_BROCK`, on **FireRed with Squirtle** (`turn_hold` 1,
`text_hold` 2, `seed_delay` 38), tier-1 verified from reset on 2026-08-14
(`route/defeat-brock/ledger.json`); tier-2 request `route-40940f-df8de7f27de8` is in
the queue. The previous accepted run was 43308 (tier-2 **passed** 2026-08-14,
`verify/results/route-43308f-a7d7d48232c4.json`, kept in git history); the 2368-frame
drop (−5.5%) came from the tooling rebuild of 2026-08-14
(`journal/2026-08-14-09-30-planner-tooling.md`): overworld walks are now *planned* by
A* over the decoded maps against the precomputed wild rate-test stream
(`crates/frlg-route/src/world.rs`, `plan.rs`) and executed with replanning
(`brock.rs::walk_planned`), and both battle-search stages run checkpointed on a
worker-emulator pool with turn delays 1..24 (`win_battle`). The forest alone gave
back 1658 frames (fewer flees, no wasted tiles); Route 1 south now ledge-hops.
Earlier history: semi-naive 49143 → 43308 via the seed sweep
(`journal/2026-08-13-23-00-where-the-frames-go.md`,
`journal/2026-08-14-00-30-seed-sweep-45276.md`,
`journal/2026-08-14-03-00-session-close.md`).

The target: power-on to `FLAG_DEFEATED_BROCK` (`data/maps/PewterCity_Gym/scripts.inc:14`). This is a strict superset of the rival-1 target:
the lab rival battle is on the way. **The rival-1 route is a baseline, not a prefix to reuse
blindly** — that route optimised "frames to rival win" with no regard for what the starter is
*for* afterwards. For this target the starter's species, nature and IVs fight Brock, so the
version × starter choice, the naming-exit seed, and the frames spent before `givemon` (which
pick the starter's PID/IVs) all have to be re-decided against the new objective. Expect the
first ~7000 frames to look like rival-1's and refuse to assume they should.

## The plan, in steps (1-4 done; 5-6 in progress)

1. **Scaffold** (this commit): target directories, this plan, the journal. rival-1 files are
   frozen — nothing under `docs/rival-1/` or `route/rival-1/` is touched by this effort.
2. **Derive the mechanics from `decompiled/`** (in progress, citations land here):
   - story gates between the lab battle and Pewter Gym (Oak's Parcel chain, the old man,
     what actually blocks Route 2 / Viridian Forest / Pewter);
   - the wild encounter pipeline: per-step check, rate accumulation, slot/level/species
     rolls, and mon generation (PID/nature/IV roll order) — everything `gRngValue` feeds;
   - starter generation at `givemon` (what a frame of delay before taking the ball buys in
     nature/IVs), base stats, learnsets to ~L15, exp formula and yields;
   - Brock's party, moves, AI; the mandatory-vs-dodgeable trainer set on the way.
3. **Model in Rust, prove against the emulator** (`frlg-mon` or an extension of
   `frlg-rng`/`frlg-battle`): given a `gRngValue`, predict per-grass-step encounter yes/no and
   the generated mon. Same standard as `frlg-battle`: a test replays real emulator runs and
   the model must match roll for roll. This is what turns "walk the forest without
   encounters" from savestate trial-and-error into a search.
4. **Semi-naive full run** (`route/defeat-brock/`): a complete tier-1-verified run, power-on
   to Brock beaten, correctness first. Encounters dodged by delay-search (or absorbed if
   cheaper to flee), battles won with the rival-1 two-stage delay search, no global
   optimisation. This gives the baseline number and the segment skeleton.
5. **Optimise segment by segment**, largest first, with the ledger tracking what moved:
   starter/version re-sweep scored on the *whole* run, starter IV/nature manipulation at the
   ball, exp routing (which fights, which levels, which move unlocks), encounter-check
   minimisation (path shape vs check count), per-battle RNG search, seed dials.
6. **Export and queue tier 2** once the run is stable enough to be worth a host replay.

## Open questions the plan hinges on

- **Which starter.** Brock is Rock/Ground; the naive type answer is Bulbasaur or Squirtle,
  but the real question is total frames: battle lengths (rival + forest + Brock), required
  exp (when does the grass/water move come online), text printed per name, and which RNG
  stream families the whole run gets. Measured, not assumed — and the answer may differ from
  rival-1's Bulbasaur pick.
- **Whether every trainer between Viridian and Brock is dodgeable**, and what the minimum
  mandatory exp intake is. Determines whether the run needs deliberate exp fights.
- **Whether level-ups are needed at all**: can a manipulated (nature/IV) starter with its
  starting moveset take Brock at the level the mandatory fights leave it? Needs the damage
  model (already in `frlg-battle`) fed with real stat calc.
- **How expensive encounter dodging is** in delay frames per grass step, which prices path
  choices through the forest.

## Evidence so far

The decomp research is done and lives in `research/` (every claim cited; geometry claims
marked as decoded-from-binary lower bounds pending emulator confirmation):

- **[story-gates.md](research/story-gates.md)** — the mandatory chain: lab battle → Route 1
  (≥20 forced grass steps) → Viridian → **Oak's Parcel round trip (mandatory)** → **catching
  tutorial (mandatory)** → Route 2 (0 forced grass) → Viridian Forest (≥48 forced grass,
  **Bug Catcher Sammy forced** — one Weedle L9; four other trainers dodgeable) → Pewter →
  gym (Camper Liam skippable, Brock interaction-only). Route 22's rival is off-path.
  Defeat observables: `FLAG_DEFEATED_BROCK` / `FLAG_BADGE01_GET`.
- **[wild-encounters.md](research/wild-encounters.md)** — the per-step pipeline. The
  encounter-rate dice roll runs on a **second LCG** seeded once per new game and advanced
  only per rate test, so its pass/fail sequence is step-count-indexed, not frame-indexed;
  frame delays reach only the cooldown gate (first 6-7 steps per map entry) and the
  behavior-change 60% roll. Dodging is path shaping against a precomputable sequence.
- **[starter-and-brock.md](research/starter-and-brock.md)** — the starter is exactly 4
  rolls at `givemon` (nature/IVs are a frame dial at the ball); Brock is Geodude L12
  {Tackle, Defense Curl} + Onix L14 {Tackle, Bind, Rock Tomb}, IVs 0, no items; Grass and
  Water hit both at 4×; Bulbasaur's Vine Whip unlocks at L10 (560 exp), Squirtle's Bubble
  at L7, Charmander is resisted until L13. Exp math and stat formulas worked and cited.

## The segments — measured 2026-08-14 (turn 1, text 2, seed 38, planner build)

The rival-1 prefix (01..09) plus the continuation. Segment code:
`crates/frlg-route/src/brock.rs`; names are semantic, order lives in the ledger.

| Segment | Frames | Ends | What happens |
| --- | ---: | ---: | --- |
| `01-boot`..`09-battle-win` | 9909 | 9909 | the prefix: 38 title idles pick the streams; rival battle 2580 (pool search, delay 31) |
| `exit-lab` | 863 | 10772 | post-battle script, out to Pallet |
| `to-viridian` | 2112 | 12884 | Route 1 north, planned crossing |
| `parcel` | 1276 | 14160 | mart scene, Oak's Parcel |
| `deliver` | 4485 | 18645 | Route 1 south **via the ledges**, lab, Pokédex scene |
| `tutorial` | 6275 | 24920 | Route 1 north again, catching demo |
| `to-forest` | 1147 | 26067 | Viridian north, Route 2, entrance building |
| `forest` | 8137 | 34204 | one planned A* crossing; Sammy (2823, delay 28) is the only fight |
| `to-pewter` | 554 | 34758 | Route 2 north into Pewter — no Pokémon Center |
| `to-gym` | 979 | 35737 | the gym door |
| `brock` | 5203 | 40940 | talk, Bubble fight at L7, unhealed (plan [59,0,7], 3458) |

The run reaches Brock at L7 (rival 68 + Sammy 100 exp; Bubble learned mid-Sammy).
Every walking leg is planned by `plan.rs` against the decoded map and the fated
rate-test sequence, then executed with replanning; the emulator-Dijkstra
(`walk_fleeing`) remains only as the fallback and never fired on this build's
committed path. The previous 43308 table lives in git history.

Route-shaping facts the builds measured (all reproduced in `journal/`):

- **The flee count is a property of the wild seed, not of timing.** On the old seed, 9
  of 10 encounters were second-LCG rate passes no frame delay can dodge
  (`journal/2026-08-13-23-00-where-the-frames-go.md`); the sweep replaced the stream
  instead. On seed 38 the walker also found a corridor past Rick's sight line — the exp
  loss (L7 vs L9 at Brock) did not cost the Bubble 2-turn plan.
- **Sammy is forced** (the forest's column 1 is sealed except through his sight row) and
  is now the run's only forest fight. On the old seed Rick was fought too (and Doug,
  despite the first build report, never was — `gBattleMons` trace, 2026-08-13).
- **The heal is gone.** The semi-naive run arrived at 6/28 HP and lost all 192 Brock
  delays unhealed; this stream arrives at **20/23, no status**, the no-heal probe beat
  Brock from there, and the rebuild banked −812 net (walk −1145, unhealed fight +333).
  The healed variant lives in git history for any future stream that arrives low.
- The wild-encounter model's practical shape: clean paths exist when the search can vary
  the rate-test index; where a belt is index-walled, one fled battle resets the cooldown
  and opens 6-7 free steps (`research/wild-encounters.md`).

## What is not optimised — the backlog, re-ranked after the planner build

1. **The Brock fight regressed +314** (5203 vs 4889): every upstream frame moved its
   stream. Its 192-delay search found 42 winners; a wider start-delay window, per-turn
   move choice, and the Torrent probe (arrive ≤7/23 HP for ×1.5 Bubble,
   `src/pokemon.c:2500` — likely marginal per the napkin math in
   `journal/2026-08-14-09-30-planner-tooling.md`) are untried on this stream.
2. **Joint seed × path sweep.** The planner prices a seed's whole walk in ~0.1 s/map;
   `bin/frlg-sweep`-style parallel builds over the scan's top seeds are now ~25 min
   each end-to-end. The 38 seed was picked for the *old* walker's step pattern.
3. **Executor consumption attribution is one step late** (the check fires at
   tile-center, after the position change) — costs nothing in frames but causes
   replan churn and blocks precise gate steering; fix before the next deep pass.
4. **Remaining flees.** The forest crossing still fled wilds where the fated stream
   walled the corridor; a longer-horizon plan (or a different seed, see 2) could
   trade a few tiles for zero flees.
5. **Starter × version sweep**, **starter IVs/nature at the ball**, and **per-scene
   text_hold** — unchanged from before, cheaper to measure now.

### Tier 2 — `route-40940f-df8de7f27de8` queued 2026-08-14

The 40940 export (ilog digest `df8de7f2…`, bk2 `bf6dc68c…`, round-trip checked) is in
`verify/queue/`. Until its result lands, the newest *passed* tier-2 run is the
superseded 43308 (`route-43308f-a7d7d48232c4`, replayed 2026-08-14 with the per-frame
gRngValue probe matching every frame); 49143, 45276, 44464 and 43346 passed earlier.
Only 40940 is the route; the rest are history.

The export follows rival-1's contract (`docs/rival-1/route.md`): `.ilog`s are canonical,
the `.bk2` is an export, the queue is drained by a human on the host.
