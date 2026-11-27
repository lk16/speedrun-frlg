# human-run: a human-executable Round 2 plan, optimised by simulation

> **Open-ended run:** start the 10-minute watchdog cron from `CLAUDE.md` ("Open-ended tasks: the watchdog cron") in your first turn, use `bin/frlg-watchdog` for status, and read `human-run/NEXT.md` for the ranked search queue.
> **Now:** the *Review round 1* section below (fixes from the human review of the v4 video), steps R0-R4.
> **Previous phase:** a controller that plays the plan in libmgba and renders a review video. See
> *Next phase: a playing controller and a review video* below, and work its step plan in order.

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
   **A death resets** (human input, 2026-11-27): any whiteout ends the attempt. A League whiteout
   is not redone from Lorelei (that loses so much time that continuing is almost never worth it),
   and mid-game or wild losses are not salvaged either; no save-and-reload. Model key
   `death_resets`. In played runs (`frlg-human play`) a rollback after a death is a *debug branch*
   for coverage, not a strategy: the played run of record is one surviving branch, so its splits are
   luckier than a real attempt, and the score always comes from the model.
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
| `human-run/scores.jsonl` | one line per evaluation, accepted **or refuted**: plan, idea, diff from parent, rung, EHT, R, CI, commit. Older rows are in `scores-archive/*.jsonl.gz`; read the whole ledger with `tools/scores.py` |
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
   together. If not, commit the refuted line. **Never re-test an idea that `scores.jsonl` (with its
   archive, `scores-archive/`) already refutes at adequate R.** Combinations and reorderings of refuted ideas are
   new ideas, and are worth trying later.
6. **Refine.** As plans stabilise, make the decisions more fine-grained: per-trainer move
   choices, per-encounter run/fight/catch rules, item use, split thresholds, mistakes,
   then RNG-manipulation variants. Calibrate whatever segments those decisions hinge on.

Run the loop open-ended while context allows. Stop early after about 8 refuted ideas in a
row, and say why in the journal. Always finish with a journal entry naming the next ideas,
and commit it.

## Next phase: a playing controller and a review video

Everything above scores a *model*. The next phase builds a **controller**: a bot that plays the
CURRENT plan in libmgba, one input per frame, with the human input model. It records an input log
and a decision log, and renders both into one video of a whole attempt. The video has two jobs:

1. **Prove the time.** One played attempt is one sample of the no-reset clock, to set against the
   model's no-reset finish (about 4:23, p10 4:14, `README.md`). It does not prove an EHT, which is
   an expectation over resets. Each played segment is a measurement and replaces a Guess segment
   in the model.
2. **Human review.** A person watches it to catch strategy flaws and decision bugs. So every
   decision the bot makes is on screen, with the numbers behind it, as it happens.

### Principles

- **One decision source.** At every choice point (moves, X items, heals, catch/run, ball choice,
  reset verdicts) the controller calls the *same* policy code the simulator uses
  (`crates/frlg-human/src/battle.rs`, `sim.rs`, plan params), fed the real game state instead of
  sampled state. Copying the logic into a second implementation is not allowed. When the game
  gives the policy a state the sim never produces, that is a model bug: log it and fix the model.
- **Reuse before writing.** `frlg-emu` (frames, RAM, savestates, `.ilog`), `frlg-route`'s
  `nav.rs` / `observe.rs` / `world.rs` (walking and state reading for the TAS targets),
  `bin/frlg-mapgraph` (tile paths), and `frlg video` (`crates/frlg-route/src/video.rs`: replay +
  ffmpeg). Every RAM address and script fact cites `decompiled/`.
- **Human inputs, not TAS inputs.** Mashed inputs late by `U{0..N}`, reaction delays of
  `U{15..30}` on unpredictable events, same knobs and same `(seed, attempt)` keying as the sim.
  No frame-perfect movement and no RNG manipulation beyond what the plan models.
- **Deterministic and resumable.** The run is a single `.ilog` from power-on that replays to the
  same frames. During development each segment starts from a savestate checkpoint
  (`$FRLG_ARTIFACTS/states/human-play/`). The final log is the concatenation of the segment logs,
  checked by replaying from boot and comparing RAM hashes at every checkpoint.
- **Roll back, don't restart.** A human-routed attempt dies mid-run more often than not (the model's
  door-to-finish survival is about 0.23, `NEXT.md`). The controller saves a checkpoint savestate
  after every split and every gym, rival and Elite Four fight, and before known high-risk fights
  (Lance, Champion). A branch is **dead** if it blacks out, soft-locks, or falls past the plan's
  reset threshold. A dead branch rolls back to the last checkpoint and replays with a **fresh
  human-model seed**: the same inputs from the same state reproduce the same death, so the input
  timing, and with it the game RNG, has to change. The dead branch's inputs are discarded. The
  final `.ilog` is still one continuous run from boot, replayable and verifiable. This is not an
  in-game save/reload (still banned) and nothing of it is visible in the video.
  - **First survivor, not best of k.** Keep the first branch that survives each checkpoint, never
    the fastest of several, so the video is an unbiased sample of a *finishing* run. A best-of-k
    showcase is allowed only if it is labelled as one in the video and the journal.
  - **Count the deaths.** Log every rollback (checkpoint, cause, retries) in `events.jsonl` and
    show it in the chat column (`rollback #3 at Lance: Dragonite crit`). Retries per checkpoint are
    a measured hazard: compare them to the model's death rates in the S9 journal.
- **No resets in the review attempt.** The controller plays on. At every split it logs the reset
  verdict the plan *would* have given (`elapsed + best_case_remaining` vs T, and the optstop
  table), so the reviewer sees each one. Starter resets are not filmed one by one. The controller
  finds the first qualifying starter by actually resetting (or by a seed search that a human could
  have hit), and the video opens with one message giving the reset count and the off-timer time
  spent.

### The decision log (`events.jsonl`)

There is one JSON line per event: `frame` (absolute), `t` (run timer in seconds, null before New Game),
`kind` (`split`, `decision`, `battle`, `catch`, `encounter`, `reset-verdict`, `rollback`, `model-vs-actual`,
`warn`), `text` (one human-readable sentence), and `nums` (a small object with the numbers behind
it). Examples of `nums`: win probability, expected turns, HP, PP left, money, balls, catch
chance, split time against the model's p50/p10, and the running total against the 4:23 forecast.
The log is a run artifact, never hand-edited, and it is enough to rebuild the overlay without
replaying.

### The video layout (1920x1080, modelled on Main97's overlay, `video-analysis/METHOD.md`)

| Region | Box (x,y w x h) | Content |
| --- | --- | --- |
| Game | 360,16 1200x800 | GBA picture, nearest-neighbour 5x, aspect kept (3:2) |
| Left column | 16,16 328x1048 | big run timer (starts at New Game per the category rules); splits table: split, actual, model p50, delta; attempt/seed line; starter box (species, nature, IVs, reset count) |
| Dex bar | 360,832 1200x232 | the plan's species grid: caught ones lit, planned-but-missing dim, with where/when each was caught; `CAUGHT n / 60` at the right edge |
| Chat column | 1576,16 328x1048 | the "webcam + chat" area: decision feed, new lines at the bottom, old lines scroll off the top |

The chat column uses a monospace font (DejaVu Sans Mono, about 14 px, line height 16 px). That
gives roughly 38 columns by 64 lines, many messages at once and still readable at 1080p. Each
message starts with the run timer at the moment it was produced (`[1:23:45]`, `[--:--]` before New
Game). The renderer wraps text at the column width itself, with continuation lines indented under
the message text, so nothing overflows. Event kinds get a colour each (decision, warning, reset
verdict, model-vs-actual) so a reviewer can skim.

Rendering goes through ffmpeg, since no font crate is vendored: the game stream is scaled and
padded, the side panels are an ASS subtitle track (libass handles fonts, wrapping, colour and
position), and the timer is a `drawtext` with a pts offset so it does not need an event per frame.
Extend `frlg video` with an overlay option. Do not write a second renderer.

### Where the video goes (never into git)

Write it to **`$FRLG_ARTIFACTS/videos/`**. That is already a directory on the host's disk, so it
outlives the sandbox and needs no `sbx cp`. Do not use the default `ignored/videos`, which lives in
the clone and dies with the sandbox. Give `videos` its own cap in `bin/frlg-artifacts-gc` (start at
8 GiB) and run the gc after rendering. The review video is **lossy** (x264, CRF about 20, yuv420p,
AAC). The existing lossless `frlg video` formats would run to hundreds of GB for four hours.
`docs/sandbox.md`'s "never dump video" is about debugging dumps; this deliverable is the one
exception. If ffmpeg or its fonts are missing from the sandbox image (`frlg-doctor` checks ffmpeg),
the sandbox still produces the `.ilog` and `events.jsonl`, and the same command renders on the host.

### Step plan

Work through these in order. Each step ends with a commit, a journal entry and a status update
here (`[ ]` → `[x]`, with the commit). A step is done only when its check passes. Measured segment
times go into `calibration.md` and, as a labelled MODEL CHANGE, into the model.

- [x] **S0. Plumbing check.** (3c199775; ffmpeg 8.0.1 has libx264, libass, drawtext; DejaVu Sans
  Mono present; test clip `$FRLG_ARTIFACTS/videos/s0-plumbing-test.mp4` from the 11-21 sandbox) In the sandbox: ffmpeg with libx264 and libass, a monospace font
  visible to fontconfig, `$FRLG_ARTIFACTS/videos/` writable, the gc cap added. *Check:* a 10 s
  test clip from the existing `defeat-brock` log, with a dummy ASS panel, renders and plays on
  the host. If something is missing, name the package for `IMAGE_PKGS` in `tools/host-prep.sh`.
- [x] **S1. Overlay renderer.** (34cd72c4, 2d726074; checked on the real 1122-lgdescent-s11 run
  instead of a hand-written log, `journal/2026-11-22-12-45-overlay-renderer.md`, `VIDEO.md`) `frlg video --overlay events.jsonl --plan <plan>` (layout above),
  driven by a hand-written `events.jsonl` over the `defeat-brock` TAS log. *Check:* timer, splits,
  dex bar and scrolling chat look right. Lines never overflow at a 38-column worst case. Timestamps
  match the timer.
- [x] **S2. Controller skeleton.** (41dcaa0c..b137c798, branch `human-run-play`; journal `2026-11-22-12-30-controller.md`) `frlg-human play <plan> --seed S --until <split>`: an emulator
  loop with the human input model, `.ilog` + `events.jsonl` output, savestate checkpoints, and
  replay-from-boot verification, plus rollback to the last checkpoint with a fresh human seed.
  *Check:* the same seed gives a byte-identical `.ilog`; the replay matches the checkpoint RAM
  hashes; a forced rollback produces a different continuation, and the stitched `.ilog` still
  verifies from boot.
- [x] **S3. Primitives.** (`play/walk.rs`, `prims.rs`, `menu.rs`, `items.rs`, `shop.rs`; exercised over the whole canonical run `pg-canon5`, journals `2026-11-22-12-30-controller.md`, `2026-11-22-13-15-dex-shop.md`) Advance text (mash with jitter), walk a tile path (mapgraph route, step
  until the position changes, handle bumps, NPCs and warps), menus by cursor position read from RAM
  (start, bag, party, shop, PC), HM field moves, Fly. Use `frlg-route` readers where they exist.
  *Check:* scripted mini-segments (Pallet → Viridian, buy at a mart, Fly) reach their target state.
- [x] **S4. Battles and catches.** (`play/battle.rs` on the sim's `battle::decide`; checked on Misty, both early rivals and a wild Paras rather than Brock, whose fight is in the surviving Nov-21 prefix; journal `2026-11-22-11-57-battle-bridge.md`) Read the battle state from RAM and drive the sim's policy:
  moves, switches, X items, heals, run, ball throws. Log each choice with its numbers. *Check:*
  Brock and a wild catch, with the decisions in the log matching what the sim's policy says for
  that state.
- [x] **S5. Starter phase and opening to Brock.** (played by the lost 2026-11-21 controller; its surviving `.ilog` prefix to Giovanni 1 replays from boot to the saved state's RAM hash and is reused, journal `2026-11-22-10-00-controller-restart.md`; Brock 1074 s vs model p50 877 s; review video `videos/1122-lgdescent-s11-giovanni1.mp4`) Starter reset loop to the plan's thresholds, then
  New Game through Brock. *Check:* played split vs the model's Brock split, and vs the TAS
  `defeat-brock` time as a floor. **First full review video** of this stretch (S1 renderer).
- [x] **S6. Main story to the first Hall of Fame,** (v2 run `1122-lgdescent-s11-v2`, first HoF 12 874 s vs p50 9 970 s, verified at 45 checkpoints; journal `2026-11-22-14-00-controller-2.md`) one WR split at a time, in the plan's order
  (`route_wr.rs`). Each split gets a savestate checkpoint, a measured time against the model, and a
  model update. *Check:* every split reached; the deltas against the model listed in the journal.
- [x] **S7. Dex fill.** (done in the post-game in the played run, not in the plan's order: 18 species at the first HoF, 60 at 18 357 s via hunts + `dexfin` evolutions/trades/fossil; National Dex per `PalletTown/scripts.inc:41-57`; journals `2026-11-22-17-00-postgame.md`, `2026-11-22-23-00-finish.md`) Encounter loops, catch rules, Safari Zone, fishing, evolutions, all 60
  species in the plan's order. *Check:* National Dex unlock condition met (cite the script).
- [x] **S8. Post-game.** (canonical `runs/human-play/pg-canon5`, finish 22 247 s; journal `2026-11-22-23-00-finish.md`) Sevii Islands, Rocket Warehouse, National Dex, Elite Four Round 2 and the
  Champion, up to the first full black frame of the second Hall of Fame. *Check:* finish frame
  reached and the timer stopped by the category rule.
- [x] **S9. Full run and video.** (`pg-canon5`, finish 22 247 s = 6:10:47, verified from boot at 96 checkpoints; video `$FRLG_ARTIFACTS/videos/1122-lgdescent-s11-full.mp4`, 4.2 GiB, 22 260 s, rendered with `frlg video --overlay` rather than a `frlg-human video` wrapper; journal `2026-11-23-03-05-full-video.md`) `frlg-human video <plan> --seed S` runs (or reuses) the full
  `.ilog` and `events.jsonl` and writes `$FRLG_ARTIFACTS/videos/<plan>-<seed>.mp4`. *Check:* the
  replay from boot verifies. The journal states the played finish time against the model's
  no-reset distribution, plus every segment where they disagree.

Steps S6-S8 can be split across worktree agents per split range once S2-S4 exist. Each agent
starts from the previous range's checkpoint savestate. Keep a running list of model bugs the
controller exposes. They are the most valuable output after the video itself.

## Review round 1: fixes from the human review of the v4 video

A human watched `videos/1122-lgdescent-s11-v4-full.mp4` up to Mt. Moon and filed 20 items in
`video-analysis/1122-lgdescent-human-review.md` (**read-only: never edit that file**; it carries the
per-item Claude verdicts with decomp citations). The review continues later with item 21 onward;
when it does, add the new items to the table below in the same way.

**Standing rules the review adds** (they hold for every stage, not just the ones reviewed):

- **No bump is acceptable.** Every attempted step either turns the player in place or moves them
  to another tile. A "bumped ..." event is a controller bug (missing NPC/object in the collision
  grid, a flag ignored, a stale position) and counts against the run like a failed check. The
  played-run summary reports the bump count; the target is 0.
- **No detours.** Each walked leg is compared with the shortest legal path at the moment it is
  planned (ledges, live NPCs, trainer sight lines). Extra steps are logged as `warn` with the
  reason; an unexplained detour is a bug.
- **Trainers the plan skips are never fought by accident.** Sight lines are drawn from each
  trainer's *current* RAM position and facing; a wanderer is passed by waiting for it to face away.
- **The early game is played by the current controller.** The v4 prefix to Giovanni 1 came from
  the lost 2026-11-21 controller and is retired: the next played run starts at power-on, through
  new early stages in `play/` (`legs::stages()` gets a boot-to-Giovanni-1 head).

| # | Item | Verdict | Work (owner step) |
| --- | --- | --- | --- |
| 1 | No encounter-seed manip at the title | CONFIRMED, ~60-90 s to Brock | R2: time the title A for a quiet wild seed (human jitter, reset when not quiet), then the starter naming frame |
| 2 | Text speed Mid | REJECTED (Fast from the bedroom) | R2: re-check the 00:50 frame; set Fast at the first menu that allows it |
| 3 | Rival name | CONFIRMED, small | R2: one-letter "New name" path; rebuild the starter naming table for the shifted frames. R3: price it in the model |
| 4 | Wall/NPC bumps (85 in v4) | CONFIRMED | R1: walker collision grid from live objects, bump = bug, regression set from v4 bump sites |
| 5 | Detours (Pallet) | CONFIRMED | R1: `frlg-mapgraph blocked_tiles` respects object flags; detour check per leg. R2: early stages |
| 6 | Starter Sp. Atk IV 29 | UNSURE | R3: paired `min_spa` 30 vs 29 on CURRENT |
| 7 | Wilds not weakened before balls | CONFIRMED (controller) | R2: weaken rule live from the first catch; R1/R2 test on a Route 1/2 catch |
| 8 | Forest Antidote | REJECTED (-5 s) | none (Antidotes are bought at Lavender) |
| 9 | Potions not picked up | CONFIRMED | R2: Route 1 clerk Potion, Forest hidden Potion (3,22); bedroom PC Potion only if R3 says so |
| 10 | Starter fainted to Sammy | CONFIRMED | R2: heal from bag before/in Sammy; a starter faint is a dead branch |
| 11 | No Brock Torrent line | CONFIRMED, size unmeasured | R3: early-only Torrent window in the model (Brock, Route 3); R2: steer HP (Sammy poison, 5/7/9 thresholds, Withdraw vs Onix, Liam with Tackle, no heal before the gym) |
| 12 | Pewter Center visit | CONFIRMED (follows from 10) | R2: no Center before Brock |
| 13 | Pewter Mart: no balls, TM39 not sold | UNSURE, leaning yes for TM39 | R3: model the TM39 sale (+1500) and a Pewter ball buy; R2: do what R3 accepts |
| 14 | Repel on Route 3 (no grass on path) | CONFIRMED | R3: drop `route3` from `repel_on`; controller uses Repels only on paths that cross encounter tiles |
| 15 | No Torrent on Route 3 | UNSURE | R3: `torrent_heal_below` 0 and per-fight thresholds, after 16/17 |
| 16 | Youngster Ben fought | CONFIRMED (unintended) | R1: sight lines; walk around north |
| 17 | Lass Robin fought | CONFIRMED (~38 s) | R1: wait at (39,9) until she faces away |
| 18 | Mt. Moon Center heal | UNSURE | R3: model the heal as a knob after 11/14; R2 follows |
| 19 | Magikarp not bought at Route 4 | CONFIRMED | R3: model the purchase (money, slot, dex-finish trip removed); R2: buy if accepted |
| 20 | Persim Berry | UNSURE, break-even | none now; revisit with Misty's AI and our defences if Misty is slow |

### Review step plan

- [x] **R0. Plan.** This section, `NEXT.md`, the STRATEGY.md Water Gun correction (item 11).
- [x] **R1. Walker.** Live-object collision, flag-aware `blocked_tiles`, trainer sight lines from
  current position and facing, wait-for-facing for wanderers, bump and detour accounting.
  *Check:* replays of the v4 bump sites from their savestates walk through without a bump; Robin
  and Ben are passed without a fight; the summary reports bumps = 0 on every stage replayed.
  *Done* (`journal/2026-11-24-19-05-r1-walker.md`): `frlg-human play-walk` over `walk-regress.json`, 87/87 cases,
  0 bumps (v4: 85); Route 3 15 seeds, Ben/Robin never fought; replayed stages 0 bumps. Open: re-check Dan / Hideout
  B1F on R2b's stages with the spinner fix.
- [ ] **R2. Early stages, boot to Giovanni 1,** (in progress 2026-11-24: `play/early.rs` + `play/early2.rs` merged; verified pieces `r2a-s11` (1507) and `r2b-full3b`; 1841 from power-on blocked at Liam by a no-PP move pick; see `NEXT.md`) in the current controller (`play/`), for the CURRENT
  plan: title seed + starter manip with human timing, one-letter rival, Fast text, early Potions,
  weaken-then-catch, Sammy/Brock Torrent line without a Center, Pewter mart per R3, Route 3 without a
  Repel and without Ben/Robin, Route 4 per R3, Mt. Moon, then Misty, Bill, Bike, Rock Tunnel and the
  hideout with the existing primitives. *Check:* `play` from power-on reaches `giovanni1`, verifies
  from boot, and the splits are listed against the v4 prefix and the model p50.
- [x] **R3. Model and plan.** (branch `human-run-r3-model`, journal `journal/2026-11-24-18-00-r3-review-model.md`; CURRENT `1841-r3-repel`: `early_bag` MODEL + all three early Potions, the Brock Torrent line, no Route 3 Repel; the plan keys the controller follows are in `NEXT.md`) Early-only Torrent window, route 3 Repel, `min_spa` 30, TM39 sale and
  Pewter balls, Magikarp at Route 4, Mt. Moon heal knob, rival-name frames, early Potion pickups,
  `torrent_heal_below`. Each is a paired evaluation with a `scores.jsonl` row; accepted ones become a
  new plan and CURRENT.
- [ ] **R4. v5 run and video.** A full played run of CURRENT from power-on with R1-R3 in, verified from
  boot, rendered (`VIDEO.md`), journal with splits vs v4 and the model, bump count, and the review
  items each one closes.

## Rules that still apply

- Every routing claim cites `decompiled/`. Whatever cannot be cited is labelled a guess in
  the plan, and so is every uncalibrated model constant.
- Do not overstate: a score is a score of *the model*. Each plan says which segments are
  calibrated against libmgba and which are estimates.
- Commit locally and often. Only committed work survives the sandbox.
