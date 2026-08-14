# 2026-08-14 13:30 — battles as constraints on the start state: solver built, measured

Question asked (user): given the optimal strategy for a fight, everything the
opponent does is a function of the RNG state `s` at battle start. Can we solve
for the satisfying 32-bit states (brute force or by known roll distances),
force the stream there with waiting frames, and then *skip* the battle in the
route search — advance the stream, charge the frames, apply the side effects?
Same for the starter and wild encounters. And: dedicated functions per
scenario or one generic solver — benchmark it.

## Answer: yes, and the machinery now exists — with one binding caveat

The inversion is sound because the game's LCG makes it arithmetic:
`jump(n)` is affine (`frlg-rng`, from `decompiled/src/random.c` /
`include/random.h:18-19`), so "the roll `n` calls after `s`" is one
multiply-add from `s` — no stepping, no emulation. A decided battle becomes a
conjunction of `(offset, residue-range)` constraints on `s`, and both
questions the route cares about are cheap:

- **"how long to wait here so the fight goes my way"** — scan the *reachable*
  states `anchor.jump(stride·w)` (stride 1/frame overworld, 2/frame in
  battle; `src/main.c:412`, `src/battle_main.c:1650`). ~6 ns per candidate
  wait, measured.
- **"which start states work at all"** — brute force all 2^32. **2.3 s** on
  this box (16 threads), measured. The user's "4 billion is doable but not
  the fastest thing in the world" concern is dead: it is faster than
  emulating *three* candidate battles.

The caveat, and it is the same one the planner-tooling entry priced: **the
offsets are frame pacing, and pacing is measured per fight.** "We know the
distances because we control everything" is true only after a
`pacing.rs`-grade fit exists for that fight — rolls land on frames, frames
depend on HP-bar drains and text, and those depend on the rolls. rival-1 has
that model (fitted from ~280 instrumented battles); Sammy and Brock do not
(backlog #1, estimated ~a day). The solver removes the *search* cost after
pacing exists; it does not remove the pacing fit. A constraint set is exactly
as trustworthy as the pacing model that produced its offsets.

## What landed (all committed, all validated)

- **`frlg-rng::constraint`** (b811330) — `Pred::{Exact, ModRange}` per roll,
  compiled through `Rng::jump_coeffs` to affine maps; `first_wait` /
  `wait_hits` (incremental affine step per wait), `scan_states` / `count_all`
  (full-space brute force, first-constraint-incremental, early exit,
  threaded). The two predicate shapes cover every decisive roll the pre-Brock
  game makes because the game only ever consumes rolls through `%` and
  compares residues (crit `%16`, accuracy `%100`, damage `%16`, AI `%256`,
  tie `%2` — citations in the module).
- **`frlg-battle/examples/constraint-solver.rs`** (6dafa8c) — extraction
  proven on the committed rival battle: walk the committed leaf (plan
  [4,3,3,3], gates 13/13/13) with an offset-recording stream, pin each
  *decisive* roll to the residue class reproducing its committed outcome.
  12 constraints over 4004 calls. Validated both directions: the committed
  anchor `0xed94271d` satisfies the set, and every wait-scan hit makes
  `engine::simulate` reproduce the 2409-frame [13,13,13] win from the
  shifted anchor.
- **`frlg-mon/examples/starter-wait-scan.rs`** (c6f736d) — the creation
  scenario, which deliberately does *not* use per-roll constraints (below).

## Measured (16 cores, release)

| shape | wait scan | full 2^32 |
| --- | --- | --- |
| generic, enum predicates | 5.96 ns/wait | 2.26 s |
| generic, boxed `dyn Fn` per roll | 6.12 ns/wait | 2.46 s |
| dedicated, flattened arrays, no enum | 7.75 ns/wait | 2.91 s |
| `engine::simulate` as predicate (all gate leaves) | 9.1 µs/wait | ~40 min (extrapolated) |
| starter: dedicated `gift_mon` state predicate | 2.6–12.2 ns/wait | — |

The committed-trace set has 3,684,096 satisfying states of 2^32 (8.578e-4
measured — the modeled independence estimate said 8.637e-4, so
constraint-independence is a fine planning approximation).

## Generic or dedicated? Generic — but per *scenario shape*, not per fight

The benchmark answer is unambiguous: the enum-predicate generic solver is the
fastest or tied in every test; hand-flattening it bought **nothing** (it
measured *slower* — the work per candidate is a handful of multiply-adds and
the bottleneck is not dispatch); even boxed closures cost only ~3-8%. There
is no case for per-fight dedicated checkers, and no case for codegen.

The real split is by *what the wish is a function of*:

- **Battle rolls** factor into single-roll residue ranges → the generic
  `ConstraintSet`.
- **Creation quantities do not**: nature is `PID % 25` across *both* PID
  rolls (`src/pokemon.c:5020`), an IV threshold is a bitfield range
  (`:1836-1852`) — inexpressible as `roll % m` ranges. The dedicated shape
  there is a plain state predicate running the 4-roll `gift_mon` model per
  candidate (2.6 ns/wait with the incremental-anchor step). Wild-mon
  creation (nature reroll loop, variable consumption) is the same story.

So: one generic roll-constraint solver + thin state-predicate scans where
quantities span rolls. Both are so far below emulator cost (~1 ms/frame)
that the choice between them is taste, not economics.

## Two findings that reshape how this should be *used*

1. **Exact-trace pinning is sound but over-strict.** In a 16k-wait window the
   constraint set hit 9 times while the engine reproduced the committed
   battle 22 times — a pinned AI branch can reach the same chosen move via
   the tie-break on the other branch, and the exact-trace set forbids that
   needlessly. Constraints answer "reproduce *this* battle"; the engine
   answers "reach *an equivalent outcome*". For screening, prefer the engine
   (9 µs is already ~10^5 waits/s/thread); reach for constraints when the
   question needs 2^32 (global optima: "what is the fastest battle this
   fight can *ever* play, and which states give it") or when a scan must run
   inside a hot joint search.
2. **Forcing a specific battle by waiting is usually a bad trade on its
   own.** The committed trace's density 8.6e-4 means an *expected* ~1160
   frames of waiting to force it from a random arrival state — half the
   battle's own length. The wait dial only wins jointly: enumerate the
   fastest battle *classes* (from the 2^32 solve), then minimise
   `wait + battle_frames` over classes × plans (menu delays shift offsets:
   `ConstraintSet::shifted` is that dial, valid because idle frames at a
   menu leave pacing unchanged downstream — the same assumption the
   existing delay search exploits). This is the user's "add waiting frames
   between the rolls that matter", priced correctly.

## Skipping the battle in the route search

Confirmed viable for any fight with a pacing model. What "skip" concretely
needs, all of it now known per trace: advance the stream by `total_calls`
(4004 for the committed rival battle), charge the leaf's frames (2409),
apply the trace's HP/exp deltas (`frlg-mon::stats` has exp/level), decrement
PP per move used (not yet in `frlg_battle::Mon` — add when wiring this in),
and reset the encounter-rate modifiers (battle start does that,
`src/battle_setup.c:205`; the planner already models it). The outer search
then continues model-side, exactly like `frlg route scan` already threads
the wild stream through crossings.

## Where this goes next (backlog updated)

Integration order that pays: (1) pacing fits for Sammy/Brock — unchanged,
still the binding item, unchanged estimate; (2) then `win_battle`'s stage-1/2
searches collapse to model scans with a single emulator confirmation of the
winner (the ~6 min of battle search per build becomes seconds); (3) the
joint seed × path × battle sweep gets battle streams *priced* instead of
blind (the scan's admitted ±600-frame blind spot); (4) starter manipulation
at the ball is now a wait-scan away once a `ball_delay` knob exists.

Wild encounters need nothing new here: the rate dice run on the second,
step-indexed LCG (already planned against); the `gRngValue` parts (slot,
level, creation, flee) join via the same state-predicate shape if flee-roll
manipulation ever becomes worth frames.
