# Defeat Brock: the route

**Status: 38862 frames, tier-2 verified** (~10m51s at 59.7275 Hz)
from power-on to `FLAG_DEFEATED_BROCK`, on **FireRed with Squirtle** (`turn_hold` 1,
`text_hold` **4**, `seed_delay` **27**), tier-1 verified from reset on 2026-08-14
(`route/defeat-brock/ledger.json`) and tier-2 **passed** the same day as
`route-38862f-8ee04c5b8bfc`. This is the accepted number; the previous accepted
run -- 38950, passed as `route-38950f-2f221a898d8e` -- is superseded and its logs
live in git history.

The 88-frame drop came from the constraint-solver pass on the rival battle
(`journal/2026-08-14-16-00-solver-on-the-rival-battle.md`): the fight model was
generalized and refitted to this route's Squirtle-vs-Bulbasaur lab battle
(`frlg_battle::pacing::SQUIRTLE_LAB`, validated leaf-for-leaf by
`tests/squirtle_committed_battle.rs`), the 2^32 start-state space was floored at
2376, and the arbitrated winner -- 3 idle frames before the battle trigger, plan
[0,2,2,3] -- plays the battle in **2445** (was 2613; on that stream 0/256 plain
start delays even win, so the old search could never have found it). The battle
gain then had to be defended downstream: head-wait dials
(`FRLG_WAIT_<SEGMENT>`) re-lucked the forest (6660, Sammy 2603) and the gym
approach, netting 38862. Earlier history: semi-naive 49143 → 43308 (seed sweep)
→ 38950 (planner rebuild, `journal/2026-08-14-09-30-planner-tooling.md`).

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

## The segments — measured 2026-08-14 (turn 1, text 4, seed 27, solver build)

The rival-1 prefix (01..09) plus the continuation. Segment code:
`crates/frlg-route/src/brock.rs`; names are semantic, order lives in the ledger.

| Segment | Frames | Ends | What happens |
| --- | ---: | ---: | --- |
| `01-boot`..`09-battle-win` | 9652 | 9652 | the prefix: 27 title idles pick the streams; 3 idles before the trigger, rival battle **2445** (solver seed [0,2,2,3]) |
| `exit-lab` | 880 | 10532 | post-battle script, out to Pallet |
| `to-viridian` | 2673 | 13205 | Route 1 north, planned crossing |
| `parcel` | 1278 | 14483 | mart scene, Oak's Parcel |
| `deliver` | 4422 | 18905 | Route 1 south **via the ledges**, lab, Pokédex scene |
| `tutorial` | 5552 | 24457 | Route 1 north again, catching demo |
| `to-forest` | 1150 | 25607 | Viridian north, Route 2, entrance building (3 head-wait frames) |
| `forest` | 6660 | 32267 | one planned A* crossing; Sammy (2603, delay 13) is the only fight |
| `to-pewter` | 552 | 32819 | Route 2 north into Pewter — no Pokémon Center |
| `to-gym` | 980 | 33799 | the gym door (1 head-wait frame) |
| `brock` | 5063 | 38862 | talk, Bubble fight at L7, unhealed (delay 164 of 384, 3305) |

The run reaches Brock at L7 (rival 68 + Sammy 100 exp; Bubble learned mid-Sammy).
The head-wait frames are stream dials, not slack: each one re-picks the
`gRngValue` family a later segment runs on while the step-indexed wild rate
sequence stays put (`FRLG_WAIT_<SEGMENT>` in `ledger::build_from`; the sweep
evidence is in `journal/2026-08-14-16-00-solver-on-the-rival-battle.md`).
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
- **The walking after a battle that looks like wandering is the cooldown being spent.**
  Audited 2026-08-14 (`examples/audit-run.rs`): zero reversal steps and zero ≥8-frame
  free-idle runs in all 38862 frames; the post-battle steps a viewer flags are the
  planner deliberately routing grass while the reset cooldown makes it nearly free
  (`src/battle_setup.c:205`). The four remaining flees (501-506 frames each) are fated
  rate passes measured cheaper to take than to dodge.

## What is not optimised — the backlog after the sweep sessions

1. **The battle model for Sammy/Brock** (`frlg-battle` now covers *both*
   lab rival fights -- rival-1's and this route's Squirtle one, fitted and
   arbitrated 2026-08-14, worth −165 at the battle -- but not Sammy or
   Brock): would let their delay searches screen in microseconds and,
   more importantly, would put the solver's floor/wait machinery on the
   run's two remaining fights. Brock's fight swings 3157..3813 across the
   stream families the wait sweeps sampled, so the model is worth real
   frames, not just search time. ~A day of fit-pacing-grade work per
   `pacing.rs`'s own warning (poison, mid-fight level-up prompt, special
   split, party switch, gym AI). The generalization pattern is now
   established: `Pacing` as data, the fight's moves as parameters, the
   fitter parameterized by ledger/drive
   (`journal/2026-08-14-16-00-solver-on-the-rival-battle.md`). The
   rival-1 lesson stands: the engine enumerates and orders, the emulator
   arbitrates; any adopted plan still needs its emulator run.
2. **Torrent probe and starter IVs/nature: closed, measured (2026-08-14).**
   The audit's HP trace (`examples/audit-run.rs`) showed the committed
   Brock fight one-shots both mons with crits (Geodude 31, Onix 33) and
   Sammy falls to 4 Tackles with 2 crits — no reachable SpA beats a crit
   one-shot, which closes Torrent (`src/pokemon.c:2500`) with it. The
   `ball_delay` dial exists now (`Tuning::ball_delay`, `--ball-delay`,
   `examples/ball-scan.rs` maps delay → genome) and was built: Adamant
   genomes give the predicted faster raw rival fights (atk ≥ 11 opens a
   2-turn window against def 9 / 19 HP), but wave-1 builds landed
   +1082..+1295 behind — the genome's ~150 frames are an order of
   magnitude below the solver+wait-dial tuning any new stream forfeits
   (`journal/2026-08-14-19-30`).
3. **Starter × version sweep**: LeafGreen still un-raced; per-scene
   text_hold still one global knob.
4. **Seed neighborhood: exhausted through delay 255 (2026-08-14).** The
   full 0-255 scan (after the route2-south crossing fix) models seeds
   184/148 at 3 flees — the only ones below the committed 4 — and the
   forest index-walled at ≥ 2 on every seed, so zero encounters is
   unreachable on this dial. Both were built: 41742/41482, and seed 184
   *realized* 7 flees against its modeled 3 — the scan's threading
   carries ±4 flees of realization error, so treat it as a ranking, not
   a count. No repel exists to buy pre-forest
   (`data/maps/ViridianCity_Mart/scripts.inc:65-70`, and the clerk will
   not shop while the parcel is held, `:55`).

### Tier 2 — `route-38862f-8ee04c5b8bfc` passed 2026-08-14

The current 38862 is **accepted**: BizHawk 2.11.1 replayed all 38862 frames to
the ledger's final ram_hash (`868413a6…`, the `brock` segment's) with the
per-frame gRngValue probe matching every frame
(`verify/results/route-38862f-8ee04c5b8bfc.json`, `fast+headless`, 67s).
Re-exporting the committed logs reproduces the result's ilog digest
(`8ee04c5b…`), so the pass belongs to exactly the logs in
`route/defeat-brock/logs/`; the `.bk2` hash differs per export, as this
contract warns. The superseded 38950, 43308, 49143, 45276, 44464 and 43346
requests passed earlier; the 40940/40865/40106 requests were pulled unrun.

The export follows rival-1's contract (`docs/rival-1/route.md`): `.ilog`s are canonical,
the `.bk2` is an export, the queue is drained by a human on the host.
