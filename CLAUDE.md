# speedrun-frlg

This project builds a **deterministic Rust simulation of Pokémon FireRed /
LeafGreen**, grounded in the pret `pokefirered` decompilation, with the goal of
**generating a tool-assisted speedrun (TAS) of the glitchless category** — and an
emulator-replayable input file to verify it.

## The bet we're making

We want to see how far an AI + simulation can get at routing this game **from
first principles**, without copying existing human/community routes.

**Hard rule — no borrowed routing knowledge.** We deliberately do NOT consult
the web (or existing TASes/speedruns) for *route* information: which Pokémon to
use, which fights to skip, glitch execution, movement optimizations, strategies,
etc. We derive all of that ourselves from the decompiled game logic + our
simulation.

**What we *may* consult the web for** (explicitly allowed):
- The **category rules** for FRLG glitchless TAS (precise start/end conditions,
  what counts as a glitch, ROM/region requirements) — see `docs/tas-rules.md`.
- **Technical/tooling facts**: emulator movie/replay file formats, button
  encodings, hardware/RNG behavior documentation — anything about *how to verify*
  rather than *how to route*.

If in doubt about whether something is "routing knowledge," treat it as
off-limits and derive it ourselves.

## Versions

FireRed and LeafGreen are the same glitchless category. We simulate **both** and
compare to determine which is faster for our route. Differences between versions
(version-exclusive availability, text, etc.) are tracked as we find them.

## Approach: small goals first, verify continuously

We build incrementally, and every milestone must be **verifiable in a real
emulator** via a generated replay file before we move on:

1. **Milestone 1 — Starter + first rival battle.** Simulate from power-on
   through the intro/new-game, picking a starter, and winning the first rival
   battle. Emit a replay file; confirm it plays back correctly in an emulator.
2. Then incrementally extend (next routes, fights, items, menus), confirming
   each step against the emulator before continuing.

The simulation must be **frame- and RNG-accurate enough that our input log,
when replayed from power-on, reproduces the same outcomes** — that's the bar.

## Repository layout

- `decompiled/` — read-only checkout of pret `pokefirered`. Ground truth for all
  game logic and data. We never modify it.
- `docs/` — our analysis and design:
  - `exploration.md` — where route-relevant *data* lives (encounters, maps,
    catch rates, trainers, learnsets, etc.).
  - `simulation-design.md` — implementation-level notes on the mechanics the
    simulation must replicate (RNG, input/frame model, intro/starter flow,
    battle engine) and what we need to extract from `decompiled/`.
  - `tas-rules.md` — category rules + emulator/replay-format facts (the only
    web-sourced material).
- (Rust crate(s) for the simulation will be added as top-level folders once the
  design settles.)

## Working notes

- Treat `decompiled/` as read-only reference; our code and artifacts live
  outside it.
- Prefer consulting `docs/` over re-searching the decomp tree.
- When a mechanic matters for determinism (RNG seeding/advancement, frame
  timing, input model), match the decomp **exactly** — approximate is not good
  enough for a replay to stay in sync.

## Shell/edit conventions

See the global instructions: prefer the dedicated file tools over piped shell
commands, and run `pre-commit run -a` after changes if a pre-commit config is
present (none currently in this repo).
