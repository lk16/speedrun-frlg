# 2026-08-14 16:00 — the constraint solver pointed at this route's own rival battle

Task (user): use the RNG constraint solver to optimize the brock TAS. The
solver machinery existed (built and proven on rival-1,
`2026-08-14-13-30-rng-inversion-solver.md`) but covered only the rival-1
fight (Bulbasaur vs Charmander); this route fights the lab battle with
Squirtle vs the rival's Bulbasaur, under a different drive (`win_battle`'s
B-mash intro + per-menu delays), and its committed battle -- 2613 frames at
plan [0, 217, 0, 0] -- had only ever been delay-searched, never
solver-analyzed.

## What landed first (all committed, all tested)

1. **The fight model generalized** (481945a): `Pacing` became a struct (two
   instances: `RIVAL1`, `SQUIRTLE_LAB`), the rival's damaging move a
   parameter, and the engine/trace grew the miss branch this fight needs --
   Bulbasaur's Tackle is 95-accurate (`decompiled/src/data/battle_moves.h:432`)
   where Charmander's Scratch could not miss. The AI walk is otherwise
   identical: Tackle is EFFECT_HIT like Scratch, and the AttackDown4
   physical-type list (`decompiled/data/battle_ai_scripts.s:1153-1160`)
   contains neither Grass/Poison nor Water, so roll B fires for both
   targets. Ground truth measured from `gBattleMons`: Squirtle 20/20 HP,
   atk 10, def 11, spe 10 -- the player still acts first (spe 10 vs 9), so
   the engine's turn structure carries over.
2. **Pacing refitted for this fight and drive** (d820708): fit-pacing
   parameterized by ledger/drive, ~230 emulated battles at the committed
   tuning, **zero label failures** -- the v1 semantics with the miss branch
   explain every roll of every run. The commit gate turned out
   delay-structured on this fight (delay 1 always resolved 23; delays 5+
   mostly 13 with 8 appearing), so the gate table became a per-delay
   function. `rhp_drain[10]` was measured from the committed battle itself
   (the fit never rolled a full crit into 10+ remaining HP).
   `tests/squirtle_committed_battle.rs` now replays the committed
   09-battle-win, reconstructs plan [0,217,0,0] and gates [13,18,18] from
   the emulator's own menu marks, and requires the engine to enumerate the
   real 2613-frame win from anchor 0xdc1c23f5 -- the fight's regression
   anchor.
3. **Adoption path** (fb4ce51): `win_battle` accepts
   `FRLG_SEED_PLAN_BATTLE="pre,d0,d1,..."` -- the arbitrated solver winner
   replayed first and made the incumbent both search stages must strictly
   beat, including the `pre` in-battle idle dial (the engine's plan[0],
   five intro press-phase streams) the delay search could never express.

## The solver pass (`examples/squirtle-solver.rs`)

Full output: `$FRLG_ARTIFACTS/scratch/squirtle-solver-full.log` (phases
1-3, waits 1-8) and the first run's tail (waits to 48).

1. **Global floor: 2376.** 2^22 anchors sampled through the bit-mixer,
   plan grid [d0, 0, 0, 0] with d0 0..5 (on this fight every menu delay's
   best commit total ties delay 0 -- gate sets in `pacing::SQUIRTLE_LAB` --
   so the anchor plus the intro's five press phases span the reachable
   streams). 100.0% of anchors have some winning leaf; fastest classes
   2376/2377/2378 all at gates [13,13,13], three Tackle turns. Exact
   density via `extract_leaf_with` + `count_all`: 229736 states of 2^32
   (5.3e-5) for each of the top classes -- 14 constraints over ~3956
   calls, including the new mod-100 accuracy pins (the rival must *hit*
   three times in the fast classes; a miss's 166-frame text is slower than
   the damage it dodges... for the fast classes anyway).
   Committed 2613 sits 237 above the floor.
2. **On-anchor arbitration: the committed anchor is honest.** 22712 plans
   have a winning best leaf below 2613 on anchor 0xdc1c23f5 -- and **zero**
   of them have their whole gate envelope below it. All 24 arbitrated
   best-predicted plans (2440 predicted) played 2897-3824 for real. Same
   story as rival-1: optimistic leaves lose their gates. The committed
   plan [0,217,0,0] = 2613 is a real local optimum of its anchor.
3. **The wait dial won: w=3, battle 2445, total 2448 (-165).** Phase 3
   idles w frames at the head of 08-battle-start, *measures* the anchor
   it reaches (jump(w) exactly for w <= ~8; +5 extra rolls from w ~ 9 --
   an object event in the shifted window, not attributed further), and
   arbitrates that anchor's engine candidates. At w=3 (anchor
   0xab5f90ce = jump(3)) plan [0, 2, 2, 3] predicted 2440 and **played
   2445**, the gates cooperating within 5 frames. Waits 4..48 produced
   nothing better (best other totals 2685+); as the bar drops toward the
   floor the candidate density collapses, and no gate-guaranteed plan
   ever appears -- consistent with the committed-anchor finding that this
   fight's gate envelopes are wide.

## Adoption: the seeded rebuild, and where the frames actually went

`win_battle` takes the arbitrated winner as a seed
(`FRLG_SEED_PLAN_BATTLE`), and the per-segment head-wait knob
(`FRLG_WAIT_<SEGMENT>`, `ledger::build_from`) expresses the w=3 idle. The
rebuild from 08-battle-start reproduced the seed exactly: 08 = 389 (+3),
09-battle-win = **2445** (-168) -- and on that stream **0/256 plain start
delays win at all**, so the two-stage search alone could never have found
this battle; the seed was the only winner.

But the run total came out **38978 (+28)**: the new downstream stream
family paid +226 in the forest (Sammy 2865 vs 2784 inside it) against
-33 through the tutorial. Cumulative at to-forest's end the new route is
**-198 ahead**; every lost frame is the forest's stream luck. The brock
segment came out identical (4924, delay 25, 3166) -- its arrival stream
re-aligned by coincidence of the delay search.

Follow-up in flight: keep the 2445 battle, re-luck only the forest with
`FRLG_WAIT_FOREST` 1..8 (an overworld idle moves `gRngValue` and the
NPC/cooldown alignment but not the step-indexed rate-test sequence, so
the planner's fated pass/fail map stays; results land below).

PLACEHOLDER: forest-wait sweep results.

## What the rival-1 lessons bought here

- The **arbitration ordering** got sharper: candidates whose *entire gate
  envelope* beats the bar (every leaf a win below it) are arbitrated
  before best-leaf-optimistic ones -- rival-1 showed gates resolve 3-5
  frame margins against you, and a gate-guaranteed plan does not care.
- The model stays the enumerator, never the arbiter: every candidate
  below the incumbent is replayed on libmgba from the state the route
  actually reaches before any adoption.
