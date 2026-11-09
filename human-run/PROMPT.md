# human-run: a human-executable Round 2 plan, optimised by simulation

You are on branch `human-run`. This target is different from everything else in the repo:
the goal is **not** a TAS. It is the plan a *human* should follow to beat the Elite 4 Round 2
world record, found by simulating a human executing candidate plans many times in Rust and
keeping whichever plan scores best.

Read `CLAUDE.md` and `docs/sandbox.md` first, and run `bin/frlg-doctor`. Everything in them
still holds: closed network, the citation rule, stdlib-only Python, no installs, commit often.

## The target (category facts, supplied from outside the sandbox)

Retrieved from speedrun.com on 2026-11-09 by the human. You cannot re-check them here, so
treat this section as the committed source of truth for the category rules.

- **Category:** Pokémon FireRed/LeafGreen, *Elite 4 Round 2*, English, Console.
- **WR:** **3:25:25 RTA** (12325 s), Main97, 2026-08-18, on a Game Boy Interface (GBA timing).
  Runner-up: 3:25:47. #3: 3:26:41.
- **Timing:** starts when **New Game** is selected. Ends on the **first full black frame at the
  end of the second Hall of Fame sequence**, before the credits.
- **Requirements:** obtain the National Dex, clear the Sevii Islands Rocket Warehouse quest,
  and defeat the upgraded Elite 4 and Champion. **Glitches are not allowed.**
- The National Dex unlock condition (believed to be: Hall of Fame done plus a caught count,
  commonly said to be 60) and the rematch unlock chain are **guesses until you cite them in
  `decompiled/`**. Derive the real gating from the scripts before planning catches.
- FR vs LG is free. The leaderboard has no version variable.
- Frame rate: GBA ≈ 59.7275 fps. Convert simulated frames to seconds with that.

Time spent before New Game (soft reset, boot, title) is **off the timer but not off the
clock**. It counts toward the score below, not toward the run time.

## What a simulated human does

1. **Starter resets.** Soft reset → New Game → intro → pick a starter → check its stats. If
   they are below the plan's thresholds, reset. Version, species and thresholds are plan
   parameters chosen by the optimiser, and every reset costs real simulated wall time.
2. **Imperfect inputs.** Each mashed text or menu input is late by `U{0..N}` frames, **N = 4**.
   Each decision that depends on an unpredictable event (a crit, a miss, an encounter, a
   speed tie, a status) adds a reaction delay of `U{15..30}` frames. Both are knobs. Once a
   plan is stable, check that it holds at other N.
3. **Adaptation.** The human sees what happens and branches: switch move or target, use an
   item, reorder trainers or routes, skip optional parts, or catch something elsewhere when
   an encounter is missed. Plans carry these branches as explicit rules (see *Plans*).
4. **Mid-run resets.** At each split, reset if `elapsed + best_case_remaining > T`. T is the
   *active* target (see *Score*), not always the WR. On top of that floor, each split has an
   optional, more aggressive threshold that the optimiser tunes, because resetting a run that
   is merely unlikely to finish usually lowers the score.
5. **RNG.** v1 uses a fully random seed per attempt, so the human only reacts and resets.
   Human-feasible manipulation (timer-based, with modelled ±frame accuracy) is a later plan
   variant, scored like any other idea.
6. **Mistakes** (wrong menu slot, wall bumps, missed turns) are **not** in v1. Add them as a
   refinement once the plan is stable.

## Score: Expected Hours to Target, EHT(T)

`EHT(T)` = total simulated wall-clock time across all attempts, resets included, divided by
the number of finishes ≤ T. Lower is better.

A plan that never beats the WR would score ∞, so scores live on a **ladder**:
`T ∈ {WR, WR+1m, +2m, +5m, +10m, +20m, +40m, +80m, +160m}`.

- Each rung is its own simulation, because the reset floor uses that T.
- A plan's **rung** is the tightest T with **≥ 50 finishes** in its run set. Its score is
  `(rung, EHT(rung))`.
- **Comparison:** a tighter rung wins. On the same rung, lower EHT wins. When a challenger
  reaches a tighter rung than the incumbent, re-run the incumbent there. If it gets zero
  finishes, the challenger wins outright.
- **Statistics:** compare plans paired, on the **same seed set** (common random numbers).
  Start at R = 10 000 attempts. Grow R until the 95% CI of the paired difference excludes 0
  or is narrower than 1% of the score. Cap R at a few million. Accept a plan only if it is
  significantly better. Otherwise log the idea as refuted, with its CI.
- **Headline:** `EHT(WR)` once any plan reaches the WR rung. Until then, report the rung,
  e.g. `EHT(WR+10m) = 41.3 h`.
- **Sanity alarm:** human runners are at ~3:25. If the model reaches the WR rung with an
  implausibly low EHT, or finishes far under the WR, treat it as **uncalibrated segment
  costs**, not as a discovery. Recalibrate before trusting any comparison.

## Simulator: model first, emulator later

Build a new workspace crate **`crates/frlg-human`** (add it to the root `Cargo.toml` members),
with a binary that evaluates a plan file and prints its score.

- **Reuse before writing.** Use `frlg-rng` (game LCGs), `frlg-battle` (damage, turn order,
  pacing), `frlg-mon` (stats, wild generation, species data), and `bin/frlg-mapgraph` /
  `docs/glitchless-run/distances.json` (tile distances). `docs/glitchless-run/plan.md` and
  `route.md` already hold the dependency graph, the HM gates and the forced trainers for
  the Hall of Fame part. Extend them for the post-game, don't redo them.
- **Segment model.** A run is a sequence of segments. Each one costs a deterministic floor
  (tiles × frames/tile for the transport mode, plus text and menu frames), plus human
  jitter, plus stochastic events: wild encounters from the real rate and slot tables, battles
  through `frlg-battle`, catch success from the real catch formula. Pin each constant with a
  `decompiled/` citation, or label it a guess in the plan.
- **Calibrate later.** When a segment's modelled cost matters to a decision, measure it on
  tier 1 (libmgba) and record the measurement. The hot spots will be battles, catches and
  long text. Move a segment to emulator replay only when the model has been shown wrong
  there.
- **Speed.** Release builds only. Parallelise attempts with `rayon` (vendored). There is no
  `rand` crate, so use a small seeded PRNG (splitmix64/xoshiro) for the human's own
  randomness, keyed by `(seed, attempt index)` so runs are reproducible and pairable.
  Avoid allocation in the attempt loop. Use all cores (`nproc`).
- `cargo build --release`, `cargo test`, `cargo fmt` and `cargo clippy` must pass before every
  commit. Keep one small test asserting that the same seed gives the same score.

The tier-2 (BizHawk) process does not apply to this target: nothing here is a movie. Do not
queue tier-2 requests for human-run plans.

## Files

| Path | What it is |
| --- | --- |
| `human-run/PROMPT.md` | this file |
| `human-run/plans/NNN-short-name.md` | one file per **accepted** best plan: human-readable steps, branch rules, reset thresholds, the machine-readable parameters the simulator reads, and the score with R and CI |
| `human-run/plans/CURRENT` | name of the current best plan |
| `human-run/scores.jsonl` | one line per evaluation, accepted **or refuted**: plan, idea, diff from parent, rung, EHT, R, CI, commit |
| `human-run/journal/YYYY-MM-DD-HH-MM-topic.md` | session notebook: what was slow or inconsistent, what was tried, what's next |
| `crates/frlg-human/` | the simulator |

Keep the plan format as simple as the simulator allows: a plain text/JSON block inside the
plan's markdown is fine. No plan DSL beyond what the current plans need.

## The loop

1. **Initial plan.** Write a crude, complete Round 2 plan: any reasonable starter and
   thresholds, the `glitchless-run` order through the first Hall of Fame, then the National
   Dex catches and the post-game in the obvious order, with the reset floor only. The goal is
   only a first score. Commit it as `001-…` with its score and the simulator.
2. **Reflect.** Per segment, look at the distributions the simulator produced: mean,
   variance, how often it causes a reset, and how much of the lost time it accounts for.
   Find what is slow and what is inconsistent.
3. **Propose one change.** Examples: reorder trainers, routes or gyms; skip or add a part;
   change which mon carries which HM; catch dex entries in a different place or order; add a
   branch rule; retune a starter threshold or a split reset threshold.
4. **Evaluate** paired against the current best, with the statistics above.
5. **Record.** Every evaluation gets a line in `scores.jsonl`. If the change is significantly
   better, write the next `plans/NNN-…`, update `CURRENT`, and commit plan, code and score
   together. If not, commit the refuted line. **Never re-test an idea that `scores.jsonl`
   already refutes at adequate R.** Combinations and reorderings of refuted ideas are
   new ideas, and are worth trying later.
6. **Refine.** As plans stabilise, make the decisions more fine-grained: per-trainer move
   choices, per-encounter run/fight/catch rules, item use, split thresholds, mistakes,
   then RNG-manipulation variants. Calibrate whatever segments those decisions hinge on.

Run the loop open-ended while context allows. Stop early after about 8 refuted ideas in a
row, and say why in the journal. Always finish with a journal entry naming the next ideas,
and commit it.

## Rules that still apply

- Every routing claim cites `decompiled/`. Whatever cannot be cited is labelled a guess in
  the plan, and so is every uncalibrated model constant.
- Do not overstate: a score is a score of *the model*. Each plan says which segments are
  calibrated against libmgba and which are estimates.
- Commit locally and often. Only committed work survives the sandbox.
