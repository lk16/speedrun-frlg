# 2026-11-27: title-screen encounter seed, New Game -> Bug Catcher Sammy

Branch `worktree-agent-a6e61bd8887f7a9df` (merged `human-run` at a15601b2). LeafGreen, tier 1 (libmgba,
real BIOS with its intro, `frlg_emu::boot_with_default_bios`). Goal: the title A frame whose wild-encounter
seed gives the fewest wild encounters from New Game to Sammy, as a human instruction.

**Result.** With the controller's current walk: at best 4 encounters (title frame 2131), median about 10.
With a walk that avoids grass (`walk_grass_cost` 400) and the right title frame: **2 encounters, almost
certainly**. Frame 3327 (55.70 s after power-on) or 2013 (33.70 s) gives 2 in 8 of 8 emulator plays each.
**Exactly 1 is not reliably reachable.** The best seed in the first 43 000 frames (frame 11 555, 193.5 s)
gives at most 1 encounter with probability 0.42 (3 of 8 plays). Over all 65 536 seeds only 6 reach P(at most 1)
≥ 0.5 on that path, and none of those 6 appear in any title frame up to 43 000.

## 1. Seeding and advance (decomp + emulator)

- The wild check has **its own LCG**: `sWildEncounterData.rngState`, `x*1103515245+12345`
  (`src/wild_encounter.c:667-671`, `include/random.h:20`). It is seeded once, by `SeedWildEncounterRng(Random())`
  in `ResetMenuAndMonGlobals` (`src/new_game.c:103`). That function runs at the title exit, right after
  `SeedRngAndSetTrainerId` reads timer 1 (`src/title_screen.c:735-737`, `src/main.c:264-269`). Timer 1 starts
  at title init (`title_screen.c:351`). Only `DoWildEncounterRateDiceRoll` advances the LCG
  (`wild_encounter.c:302-307`; `grep WildEncounterRandom`). Nothing re-seeds it later: the naming screen
  re-seeds `gRngValue` and the trainer ID only (`naming_screen.c:716-723`).
- `gRngValue` advances every frame (VBlank `Random()`, `src/main.c:412`; twice in battle,
  `battle_main.c:1647-1650`) and on every event call. It is reached by the human's frame timing.
- So the seed fixes the hit/miss value of each rate test. Human timing only decides *which* steps get a
  test, through two `gRngValue` gates in `TryStandardWildEncounter` (`wild_encounter.c:707-776`):
  - **Cooldown.** `minSteps = 8 - rate/10` (`:673-687`): 6 on Route 1/2 (rate 21), 7 in the forest (rate 14,
    `src/data/wild_encounters.json` LeafGreen headers). Until then a grass step is tested only if
    `Random()%100 < 5` (`:750-754`).
  - **Boundary gate.** When the behaviour differs from the previous step's, `Random()%100 < 60` (`:348-353,371`).
  - Every map load (`overworld.c:764,799`) and every battle start (`battle_setup.c:205`) resets the
    cooldown and the miss buff (`field_control_avatar.c:735-738`). They never touch `rngState`.
- **A turn in place on grass is a check too.** `WALK_IN_PLACE_FAST` with `runningState == TURN_DIRECTION`
  ends in `T_TILE_CENTER` (`field_player_avatar.c:462-466,745-760,779-785,900-903`), and that sets
  `checkStandardWildEncounter` (`field_control_avatar.c:141-142`). Walking continuously through a corner
  does not turn. Starting from a stop in a new direction does: after a battle, a menu, or a stop.
- Emulator check (`encounter_seed_scan.py validate`, 12 played runs, about 2 200 checks): the per-step model
  reproduces every traced rate test, every `rngState` value and every wild battle when it is given the
  observed gate outcomes. Measured gate rates: cooldown bypass 35/914 (3.8 %, code 5 %), boundary pass 52/78
  (67 %, code 60 %).

## 2. What the human's timing does (question 4)

The count depends almost only on the seed. The seed sets the hit pattern of the stream, and a hit buys a
6-7 step cooldown, so moving the stream index by one test seldom changes the total. Exact distribution
over independent gates (`dist`): seed 53936 (the v5b-s41 starter checkpoint) gives 5 encounters with
P = 0.99998 on the current path. That run had 5. On the grass-avoiding path the model says 3 with P = 0.88
and 4 with P = 0.12; 13 plays of the controller (human seeds 7, 11-22, input jitter U{0..4}) gave ten 3s and
three 4s. Every traced run's observed count is at the model's mode or next to it.

## 3. The path (question 3)

The forced grass, from `frlg map Route1` / `ViridianForest` (the `MapData::ascii` of the decomp layouts):

- **Route 1 northbound, about 20 grass steps.** South entry rows 35-39 (5); the row-32 approach to the
  only ledge gap (6..8, 31) (3); the rows 25-27 band, because x 18-21 of row 26 is a ledge (3); the
  rows 14-17 band (4); the north band rows 6-10 (5). The first 6 grass steps after the map load are
  cooldown.
- **Route 1 southbound (the Pokédex trip), 5.** Ledges skip every other band. The south exit rows 35-39
  remain. All 5 are cooldown on a fresh map, so a test happens only on a 5 % bypass.
- **Route 2 to the forest: 0** (the walker's grass-avoiding plan).
- **Forest to Sammy, about 39.** The row-19 crossing to the middle (7). Up the x 19-23 column (10). Down
  the x 11-15 column (16). Then up the x 3-7 column into his line (6). Rick's sight line (47,45) forces 3
  more on the east side unless he is fought.

The current controller (`GRASS` tie-breaker 25) makes 136 encounter-tile checks to Sammy (Route 1 59,
Route 2 5, forest 72). With the new plan key `walk_grass_cost` 400 it makes 90 (Route 1 47, forest 43),
close to the floor of about 84. `walk_grass_cost` 3000 makes 84, but it fights Bug Catcher Rick (an extra
battle) to save forest grass; not recommended.

Same seed (53936) and the same starter checkpoint: the current walk reaches Sammy at frame 39 809 with 5
encounters. The grass-avoiding walk reaches him at about 36 600 (median of 13) with 3-4 encounters. The
extra tiles (about 48, about 13 s) are repaid by the encounters avoided.

## 4. Search (questions 4, 5)

`encounter_seed title` read the seed for every A frame 0..43 000 from power-on with **no input before the A**.
The title takes A on frames 2000-4436, then the attract loop restarts it (2700-frame timer,
`title_screen.c:431-440,646-649`): 24 026 ready frames, 16 906 distinct seeds. Later cycles repeat seeds.
Neighbouring frames are unrelated seeds: TM1 runs 4.3 times per frame. Tables:
`human-run/data/encounter-seed/` (`title-seed-lg-noinput.tsv`, `scan-current-path.tsv`,
`scan-grass400-path.tsv`, `allseeds-*.tsv`, the three `path-*.json`).

| title frame (s from power-on) | seed | current path: mean | grass400: P(≤1) / P(≤2) | emulator, grass400 |
| --- | --- | --- | --- | --- |
| 2013 (33.70) | 5475 | 6.1 | 0 / 0.994 | 2,2,2,2,2,2,2,2 |
| 3327 (55.70) | 63401 | 7.2 | 0 / 1.000 | 2,2,2,2,2,2,2,2 |
| 2539 (42.51) | 31859 | **4.0** (best on the current path) | 0 / 0.980 | - |
| 11555 (193.46) | 31467 | 4.9 | **0.416** / 1.000 | 2,2,2,1,1,2,1,2 |

(The emulator plays above are from `seedpoke-<seed>`: v5b-s41's starter checkpoint with `rngState` written
to the seed (`encounter_seed poke`). The title -> seed map was measured separately, from the real title exit.)

Over all seeds the grass400 path has a median of 6.1 encounters. 81 seeds give P(≤2) ≥ 0.9; 4 give
P(≤1) ≥ 0.9.

**Robustness.** The window is exactly one frame. The ±1 and ±2 neighbours of 2013 average 6.0/6.2/8.0/6.4
encounters, those of 3327 7.9/5.1/7.0/6.1. So P(target) is the title press's hit probability: 0.64 in the
plan's `p_hit`, or 0.90 at σ = 0.3 frames (`docs/tricks/starter-manip.md` 4.1). A miss also invalidates the
starter table (below), and the stats check catches it. Under the controller's input jitter the outcome
holds: P(≤2) = 0.994-1.000, 16 of 16 plays.

## 5. Human instruction and the starter manip

1. Power on and **press nothing**. The title is up at 33.5 s (frame 2000). Start the timer at power-on.
2. Press A at **55.703 s** (frame 3327). A one-frame window: aim with a frame-accurate timer, calibrated
   as for the starter. Or press A at 33.703 s (frame 2013), 0.2 s after the title appears.
3. Then New Game, and the starter manip as now (naming OK, ball wait).
4. Walk the grass-avoiding route (`walk_grass_cost` 400; the corridor choices in section 3). Do not stop
   on grass, and do not turn in place on grass.

Starter interaction:

- The naming screen re-seeds `gRngValue` from a fresh timer 1 (`naming_screen.c:427-428,716-723`). The
  starter is set by the naming exit and the ball press; the wild stream does not see them.
- The title frame still matters to the starter. It moves the naming TM1 by sub-frame ticks
  (`starter-manip.md` 3.1), so the (naming N, ball j) table must be built **for the chosen title frame**.
- With `dials` 2 such a table exists for any title frame (mean on-timer cost about 65 frames for T2,
  doc 4.2). The two manips are compatible.
- A title miss changes both the encounter seed and the starter, and the stats check shows it.
- No-input boot costs about 25 s more real time per attempt than mashing through the intro, all off the
  timer. In exchange the title frame counts from power-on, so the table is fixed.
- The controller's `starter()` still mashes the intro. It picks the first "quiet" frame (≤ 6 hits in 60
  forest-rate rolls) relative to a mashed title. Switching it to an absolute frame scored by this path
  model is left for the next session.

## Caveats

- Tier 1 only. Hardware and BizHawk agreement on the TM1 tick count is unverified (as for the starter).
- The path is the controller's. A human who stops on grass adds turn checks. A gate outcome the human
  gets wrong costs about one encounter, at the rates measured above.
- 2 encounters instead of 5-11 removes forest exp. The plan's `fight_until_level` 17 exp before Brock is not
  re-priced here. That needs an R3 model and plan change: the encounter count is in practice a plan choice.

## Tools

- `crates/frlg-human/examples/encounter_seed.rs`: `title` (A-frame -> seed, 12 threads), `trace`
  (per-step wild data from a run's ilog), `poke` (validation only).
- `human-run/tools/encounter_seed_scan.py`: `validate`, `path`, `scan`, `allseeds`, `seed`.
- Plan key `walk_grass_cost` (`params.rs`, default 25 = unchanged), applied by `walk::set_grass_cost`.
