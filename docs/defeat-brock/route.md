# Defeat Brock: the route

**Status: seed sweep + heal deletion done.** 44464 frames (~12m24s at 59.7275 Hz) from
power-on to `FLAG_DEFEATED_BROCK`, on **FireRed with Squirtle** (`turn_hold` 2,
`text_hold` 4, `seed_delay` 38), tier-1 verified from reset on 2026-08-14
(`route/defeat-brock/ledger.json`), tier-2 queued (`route-44464f-8d3644886b9c`) and not
yet replayed. The semi-naive baseline was 49143. The 4679-frame drop, in order of landing:
re-picking the wild-encounter stream at the title screen (`Tuning::seed_delay`, −3867 —
six candidates scored by simulating each seed's rate-test stream, then built end-to-end
and measured), and deleting the Pewter heal the new stream no longer needs (−812). See
`journal/2026-08-13-23-00-where-the-frames-go.md` and
`journal/2026-08-14-00-30-seed-sweep-45276.md`.

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

## The segments — measured 2026-08-13 (seed_delay 38)

The rival-1 prefix (01..09) plus the continuation. Segment code:
`crates/frlg-route/src/brock.rs`; names are semantic, order lives in the ledger.

| Segment | Frames | Ends | What happens |
| --- | ---: | ---: | --- |
| `01-boot`..`09-battle-win` | 9986 | 9986 | the prefix: 38 title idles pick the streams; rival battle 2770 (plan `[4,14]`, 103/128 delays win) |
| `exit-lab` | 880 | 10866 | post-battle script, out to Pallet |
| `to-viridian` | 2165 | 13031 | Route 1 north, one fled encounter |
| `parcel` | 1260 | 14291 | mart scene, Oak's Parcel |
| `deliver` | 5060 | 19351 | Route 1 south, lab, Pokédex scene; one flee |
| `tutorial` | 6329 | 25680 | Route 1 north again, catching demo; two flees |
| `to-forest` | 1307 | 26987 | Viridian north, Route 2, entrance building |
| `forest` | 10583 | 37570 | the decoded-maze waypoint chain; **Rick dodged on this stream** — Sammy is the only fight (2839, plan `[27,0,0,3]`); seven wilds fled |
| `to-pewter` | 573 | 38143 | Route 2 north into Pewter — no Pokémon Center (see below) |
| `to-gym` | 962 | 39105 | the gym door |
| `brock` | 5359 | 44464 | talk, 3-turn Bubble fight at L7, unhealed at 20/23 (plan `[169,0,7]`, 55/192 start delays win), `FLAG_DEFEATED_BROCK` |

Eleven wild flees at ~500 frames each remain (one more than the old seed, but Rick's
4327-frame fight is gone and every crossing walks tighter); the run reaches Brock at L7
(rival 68 + Sammy 100 exp; Bubble learned mid-Sammy), not L9 as before.

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

## What is not optimised — the backlog, re-ranked after the sweep

1. **Eleven flees (~5.5k frames) and walk bloat.** Still the top item: a model-driven
   path search (frlg-mon has the proven RNG model) could shape crossings against the
   known pass/fail stream instead of discovering it edge by edge; the seed and the path
   should be optimised *jointly* (the sweep held the walker fixed).
2. **Wider seed neighbourhood.** 64 delays scanned, 6 built; the scan's flee model ranked
   candidates imperfectly (27 modeled best, placed third) — building more of the scan's
   top-10 is cheap now (~55 min/build, parallelisable).
3. **Starter × version sweep.** Squirtle/FireRed won by default (Bubble at L7); Bulbasaur
   needs the Liam detour but fights differently; LeafGreen un-raced.
4. **Starter IVs/nature** are 4 rolls at the ball — a frame dial never turned.
5. **Battle plans**: A-mash + delays only (now 192-wide with the checkpointed searcher);
   move choice beyond the Bubble steer is first-pass.
6. **The tutorial/deliver/parcel text** runs at one global text_hold.

### Tier 2 — `route-44464f-8d3644886b9c` queued 2026-08-14, not yet replayed
(the 49143 and 45276 requests are superseded and can be skipped)

The export follows rival-1's contract (`docs/rival-1/route.md`): `.ilog`s are canonical,
the `.bk2` is an export, the queue is drained by a human on the host.
