# Starter and encounter-seed manipulation: mechanism, measurement, simulator model

Derived 2026-11-13 from `decompiled/` and measured on libmgba (tier 1, FireRed ROM, the committed
`frlg-route` pipeline). Context: the WR holder is reported (user-supplied, not verified here) to use a
timer program with a millisecond offset to press at an exact frame so the starter Squirtle has chosen
IVs/nature, plus an "encounter manip" for Viridian Forest. Everything below is what the code says and
what tier 1 measured; where a claim is not cited it is labelled a guess. **Tier 1 only: nothing here has
been checked on BizHawk or hardware, and the sub-frame numbers in section 3 are the part most likely not
to transfer.**

Tools and data: `crates/frlg-route/examples/starter-sweep.rs`, `.../title-seed.rs`,
`human-run/tools/{starter_scan.c,wild_seed_scan.py,analyze_sweep.py}`,
`human-run/data/{starter-sweep-fr.tsv,starter-sweep-fr.summary.json,title-seed-fr.tsv,starter-scan-2e32.txt}`.
`segments.rs` gained a thread-local `set_naming_delay` (default 0, no route uses it).

## 1. Mechanism

### 1.1 There are two seeds, from two different timer reads

| Event | What happens | Cite |
| --- | --- | --- |
| Title screen init | `StartTimer1()`: `REG_TM1CNT_H = 0x80` (timer 1, prescaler 1, free-running 16-bit: 1 tick = 1 CPU cycle, wraps every 65536 cycles = 3.9 ms = 0.233 frame) | `src/title_screen.c:351`, `src/main.c:259-262` |
| Title exit (after the A press -> `TITLESCREENSCENE_CRY`, 90-frame wait, fade) | `SeedRngAndSetTrainerId()`: `val = REG_TM1CNT_L; SeedRng(val); TM1 off; gTrainerId = val`. Then `ResetMenuAndMonGlobals()` -> `SeedWildEncounterRng(Random())` | `src/title_screen.c:735-737`, `src/main.c:264-269`, `src/random.c:15-18`, `src/new_game.c:103`, `src/wild_encounter.c:661-664` |
| Player naming screen entry | `StartTimer1()` again (player template only) | `src/naming_screen.c:427-428` |
| Player naming screen exit (after fade-out) | `SeedRngAndSetTrainerId()` again: **`gRngValue = TM1` (16 bit), overwriting everything before it** | `src/naming_screen.c:716-723` (`MainState_Exit`) |

Consequences:

- **The wild-encounter stream is fixed by the title press and nothing later touches it.**
  `ResetMenuAndMonGlobals` is not called after the title exit, and the wild LCG is a private
  `rngState` advanced only by the rate dice roll (`src/wild_encounter.c:302-307,667-671`). Measured:
  `rngState` at NEW GAME equals `Random()` applied **twice** to the title TM1 value in 538/538 delays
  (`title-seed-fr.tsv`; one of the two is `new_game.c:103`'s own, the other is assumed to be a VBlank
  `Random()`, `src/main.c:412`, landing in between -- mechanism unverified, the count is measured).
  So wild seed = f(title TM1 value). Journal `2026-11-13-12-30` and
  `docs/defeat-brock/research/wild-encounters.md` agree.
- **The starter is not decided by the title press through `gRngValue`**: the naming-screen reseed
  erases it. The starter's PID/IVs are a function of (TM1 value at naming exit, number of `Random()`
  calls from then to `givemon`). Every route passes through the naming screen: the player name always
  goes to it (`src/oak_speech.c:1352-1379` fades straight into `DoNamingScreen(NAMING_SCREEN_PLAYER, ...)`,
  `:1440-1448`); the preset-name menu only exists on the say-NO re-ask path (`:1401-1415,1515-1517`). The
  rival's preset menu uses no `Random()` and no timer (`:1431-1436`, `:2117-2155`).
  Starting the naming screen restarts TM1 from 0, so the seed is the **cycle count spent in the naming
  screen** (entry to the fade-out end), mod 65536.
- **So "one press decides both" is not literally true in the decomp.** There are two decisive presses: the
  title A (wild seed) and the naming-screen OK (starter), and a third time-critical event, the ball
  confirmation (section 1.3). Measured coupling: the title press does perturb the naming TM1 value
  slightly (sub-frame audio-mixer phase, tens to hundreds of ticks; section 3), and any difference in
  TM1 scrambles the whole starter, so a table of "naming frame -> starter" is only valid for the exact
  title frame it was built with. Practically the two presses form one plan; mathematically they are two
  seeds.

### 1.2 gRngValue from naming exit to the starter

- `Random()`: `gRngValue = 1103515245*x + 24691`, return top 16 bits (`src/random.c:9-13`,
  `include/random.h:18-19`). Called once per VBlank, always (`src/main.c:412`), twice per frame in battle
  (`src/battle_main.c:1647-1650`). Outside battle: **1 step per frame** plus event consumers.
- Event consumers between naming exit and the ball (all deterministic given the stream and the frame count,
  none depend on text speed or A timing except by changing the number of elapsed frames):
  - Wandering NPCs: each "step" consumes `Random()` twice per cycle -- a delay roll
    `gMovementDelaysMedium[Random() & 3]` = 32/64/96/128 frames (`src/event_object_movement.c:679,3090`,
    LOOK_AROUND `:3037`) and a direction roll (`:3110`, LOOK_AROUND `:3061`). Oak's lab has three:
    WORKER_F (the "lab girl") `WANDER_UP_AND_DOWN` and two scientists `LOOK_AROUND`
    (`data/maps/PalletTown_ProfessorOaksLab/map.json`, object events 1-3); Pallet Town has two
    `WANDER_AROUND` NPCs (`data/maps/PalletTown/map.json`).
  - `lockall` (every ball script) freezes all other objects (`src/scrcmd.c:1195-1207`,
    `src/event_object_movement.c:8783-8791`), so **from the moment the player interacts with the ball no NPC
    draws happen**. Measured: from "facing the ball" to creation is exactly 170 stream steps in all 320
    sweeps (`m_at_j0 = {170: 320}`), 171-172 frames.
- Name length / text speed / A-press timing: they change only how many frames elapse (a printed character
  costs `sTextSpeedFrameDelays[speed]` frames, `src/new_menu_helpers.c:27`; held A zeroes the delay once
  one press landed, `src/text.c:639-650`; `textSpeed` is read at `src/oak_speech.c:761`). They have **no
  other effect on the stream**. So a human who presses the decisive buttons at the right *elapsed time since
  the naming exit* does not need the intermediate presses to be frame-exact, with one exception: NPC wander
  draws are deterministic in time but a blocked NPC step (player in the way) takes a different branch
  (`GetCollisionInDirection`, `:3113-3114`), so the walking path matters a little. Not measured.

### 1.3 Creation

`ChoseStarter` runs `msgbox OakThisMonIsEnergetic` and **then** `givemon PLAYER_STARTER_SPECIES, 5`
(`data/maps/PalletTown_ProfessorOaksLab/scripts.inc:1115-1122`) -> `ScrCmd_givemon` ->
`ScriptGiveMon` -> `CreateMon(mon, species, 5, 32, 0, 0, OT_ID_PLAYER_ID, 0)`
(`src/script_pokemon_util.c:48-72`, `:55`). **The decisive third press is the A that dismisses "this POKeMON
is really energetic" (after the YES), not the YES itself.** `CreateBoxMon` consumes exactly 4 `Random()`:
PID low half, PID high half (first call is the low half, measured), IV word 1 (HP/Atk/Def, bits 0-4/5-9/10-14),
IV word 2 (Spe/SpA/SpD) (`src/pokemon.c:1778,1836-1852`, `include/random.h:14`). Nature = PID % 25
(`src/pokemon.c:5020-5023`); no reroll loop for gift mons. Validated against the ROM: for all 2048 (N, j) sweep
rows, a pure LCG step from `gRngValue` at "facing the ball" reproduces the created PID and six IVs (no row
without a match).

### 1.4 Parity / "unreachable" targets

Waiting `j` idle frames facing the ball shifts the creation index by `j`, except that every NPC draw during
the wait inserts an extra `Random()` between two VBlank draws, so the index jumps by 2+ and the skipped
index cannot be hit by any press frame. Measured over 2040 consecutive-`j` pairs (8 different naming seeds,
j = 0..255): step 1: 1956 (95.9%), step 2: 47, step 3: 24, step 5: 13. **6.7% of stream indices in the
wait window are unreachable** (147 skipped / 2187 spanned) -- the quantitative form of "if the lab girl moves
on frame 30 and the parity is odd, the target is unreachable". Which indices are skipped depends on the naming
seed (it picks the NPC delay rolls), so a miss-this-index target is fixed by choosing another naming frame.

## 2. Stats and nature for a SpA / Torrent / Water Pulse plan

Stat formulas (`src/pokemon.c:2093-2100,2130-2131`, `:5404-5438`, `frlg-mon/src/stats.rs`): HP
`(2b+IV)L/100 + L + 10`; others `floor(((2b+IV)L/100 + 5) * n)`, n = 1.1/0.9 truncating. EV = 0. Squirtle base
44/48/65/43/**50**/64 (`src/data/pokemon/species_info.h:212-239`). Torrent: Water power x1.5 at HP <= 1/3
(`src/pokemon.c:2500`, as cited in `docs/defeat-brock/research/starter-and-brock.md`).

SpA by IV (neutral / +SpA nature):

| Level | IV 0 | IV 15 | IV 20 | IV 25 | IV 30-31 |
| ---: | --- | --- | --- | --- | --- |
| 5 | 10 / 11 | 10 / 11 | 11 / 12 | 11 / 12 | 11 / 12 |
| 16 | 21 / 23 | 23 / 25 | 24 / 26 | 25 / 27 | 25 / 27 |
| 30 | 35 / 38 | 39 / 42 | 41 / 45 | 42 / 46 | 44 / 48 |
| 50 | 55 / 60 | 62 / 68 | 65 / 71 | 67 / 73 | 70 / 77 |

At L5 every SpA IV >= 20 is the same stat (11, or 12 with a nature): the **L5 stat is nearly insensitive**;
the IV pays out at L30+ (IV 30 vs 0 = +9 at L30, +15 at L50; the +10% nature adds +4 / +7). The previous audit
(journal `2026-08-14-19-30`) found no SpA breakpoint before Brock/Sammy; the payoff is a later-game one.
Nature candidates (`NATURE_STAT_TABLE`, `src/pokemon.c:1360-1385`): Modest (+SpA -Atk; Squirtle's Atk is not
used by a Water Pulse plan, so it is the free choice: Atk x0.9 costs 1-3 stat points on a stat that is unused),
Mild (-Def: loses a stat that is Squirtle's strength: L30 Def 50 -> 45), Rash (-SpD: 64 base, -4 at L30), Quiet
(-Speed, bad). Ranking for this plan: **Modest > Rash >= Mild**; Rash/Mild alone is 2/25 of natures, Modest
adds a third. Speed/HP/Def IVs: +1 stat per ~3 IV at L30, so "IV >= 20-25" is a +4..+6-point filter, with
diminishing value.

## 3. Measurements

### 3.1 Starter vs naming-exit frame (N), FireRed, Squirtle, `human-run/data/starter-sweep-fr.tsv`

Pipeline = committed rival-1/defeat-brock segments (1-letter name, KAZ rival, options, house, to-lab, lab
walk), N idle frames inserted between START and the final A on the naming screen; 320 values of N, genome at
j = 0, plus j = 0..255 for N in {0,40,...,280}.

- The naming seed (`gTrainerId` = TM1) is **not an arithmetic progression** in N. Per frame it advances by
  18752 ticks (= 280896 cycles/frame mod 65536) plus a jitter: step-minus-18752 has sd 5400 ticks and median
  1573 (only 3% of steps within 64 ticks). The jitter is deterministic (audio mixer / VBlank handler time
  before the timer read, `src/main.c:392-413`; mechanism = assumption) but not computable without the
  emulator. 320 values of N gave 320 distinct seeds. Treat each N as an independent uniform draw of a u16.
- Title-frame coupling (6 values of N, title idle 0/1/2/4/8): the naming seed moves by +32, +1, +64, +408 ticks
  and the genome changes completely each time. A starter table is therefore keyed on (title frame, N).
- Genome at j = 0 over the 320 N: nature uniform as expected; T1 (Modest/Mild/Rash and SpA IV >= 30):
  3/320 = 0.94% (expected 0.75%): at N = 1, 74, 154, isolated. T2 (+Spe/HP/Def >= 20): 1/320.
  **Window = 1 frame**: neighbouring N share nothing.

### 3.2 Exact densities over the full 2^32 stream (`starter-scan-2e32.txt`, `starter_scan.c`)

Every (seed, offset) is a state of the 32-bit cycle, so the starter's outcome statistics are exact over all
states; model validated by section 1.3.

| Tier | Definition | Density | 1 in | P(good at i+1 given good at i) |
| --- | --- | ---: | ---: | ---: |
| T0 | Modest/Mild/Rash, SpA IV >= 25 | 2.625% | 38 | 2.62% |
| T1 | Modest/Mild/Rash, SpA IV >= 30 | 0.750% | 133 | 0.75% |
| T1R | Mild/Rash, SpA IV >= 30 | 0.500% | 200 | 0.50% |
| T2 | T1 + Spe,HP,Def IV >= 20 | 0.0396% | 2528 | 0.11% |
| T3 | T1 + Spe,HP,Def IV >= 25 | 0.00785% | 12740 | 0.03% |
| T4 | T1R + Spe,HP,Def IV >= 25 | 0.00523% | 19109 | 0.02% |

Good outcomes are **memoryless along the stream**: run-length histogram of consecutive good indices is 99.2%
length 1 (T1: 31.7M runs of 1, 238k of 2, 1836 of 3, 13 of 4); the conditional probability for neighbours is
the base rate. There is no forgiving window in either dial. Natures are uniform (171.80M each).

### 3.3 Ball dial j (`starter-sweep-fr.tsv`)

`j` idle frames facing the ball shift the creation index 1:1 until an NPC draw (section 1.4); 6.7% of indices
are unreachable. j = 0..255 for 8 naming seeds: 2048 genomes, T1 16 (0.78%), T2 0, consistent with 1/133.

### 3.4 Wild seed vs title frame (`title-seed-fr.tsv`, `wild_seed_scan.py`)

Delays 0..647 of `Tuning::seed_delay`: the title TM1 value is 18752/frame plus larger jitter (sd 9300; and
delays do not map 1:1 to press frames: 19 of 648 pairs gave identical seeds, e.g. delays 640 and 642, because
the A-mash phase interacts with the 1-frame press registration). Exact scan of all 65536 wild seeds
(`rate = 16*mapRate + buff*16/200`, buff reset on hit; `wild_encounter.c:302-332,760-784`), counting hits among
the first 60 forest-rate (14) rate tests:

| hits <= | 2 | 3 | 4 | 6 | 8 |
| --- | ---: | ---: | ---: | ---: | ---: |
| P (all 65536 seeds) | 0.37% | 1.66% | 4.98% | 21.8% | 50.3% |
| observed in 648 title frames | 1 | | 19 (2.9%) | 118 | 302 |

(The journal's 0.7% / 6.3% ignore the miss-buff; with it the tail is a bit thinner. The 648-frame observation
is below the uniform expectation at <= 4 (19 vs 32): 19 duplicates in 648 mean the reachable seed set is
clustered. Not resolved.) If the 31 earlier rate tests at 21% (Route 1/Viridian/Route 2 in the committed route)
are included in the window, P(<= 8 hits in 91 tests) = 1.9%, P(<= 6) = 0.26%: "quiet to Brock" is far rarer
than "quiet forest", and the stream index drifts per route (two frame-locked gates, journal 2026-11-13-12-30).

## 4. Recommended simulator model

Entities: two exact-frame **decisive presses** with a small joint plan, and a table-driven target choice.

### 4.1 Parameters

| Symbol | Meaning | Value |
| --- | --- | --- |
| window | frames that yield the target outcome | **1** per dial (no forgiving run, section 3.2) |
| `sigma_f` | std dev of the human press time in frames | **0.30** (5 ms) reproduces "~90% when set up well" |
| `mu` | calibration offset in frames, drifting ~0.25/day (user observation) | 0 fresh, 0.25 at one day |
| `p(sigma, mu)` | P(registered frame = target) = `Phi((0.5-mu)/sigma) - Phi((-0.5-mu)/sigma)` | below |
| miss | what a miss yields | a *different* independent draw: wild seed is quiet with prob 0.37-5% (<= 2-4 hits), starter is T1 with prob 0.75%, T2 0.04%: treat as **fail** |
| `rho_j` | per-index density for target tier | T1 1/133, T2 1/2528, T3 1/12740; x0.933 reachable |

`p(sigma, mu)`:

| sigma (frames / ms) | mu=0 | mu=0.125 | mu=0.25 | mu=0.5 |
| --- | ---: | ---: | ---: | ---: |
| 0.15 / 2.5 | 0.999 | 0.994 | 0.952 | 0.50 |
| 0.20 / 3.3 | 0.988 | 0.969 | 0.894 | 0.50 |
| 0.30 / 5.0 | 0.904 | 0.876 | 0.791 | 0.50 |
| 0.40 / 6.7 | 0.789 | 0.767 | 0.704 | 0.49 |
| 0.50 / 8.4 | 0.683 | 0.668 | 0.625 | 0.48 |
| 0.75 / 12.6 | 0.495 | 0.489 | 0.472 | 0.41 |
| 1.00 / 16.7 | 0.383 | 0.380 | 0.372 | 0.34 |
| 2.00 / 33.5 | 0.197 | 0.197 | 0.196 | 0.19 |

### 4.2 Attempt structure (with a correct table)

The three presses are **independent given a table**; the *filter* (quiet seed AND good starter) is applied
when the plan is chosen, not by resets:

1. Choose a title frame `t` whose wild seed is quiet (free: every title frame has a seed; quiet <= 4 hits in
   60 forest rolls at ~3-5% of frames, <= 2 at ~0.4%; waiting at the title costs real time only).
2. For that `t`, choose (N, j) with a good starter. The pair count with N + j <= X is ~0.933 rho X^2/2, so the
   **on-timer cost** `N + j` of the best pair is `sqrt(pi/(2 * 0.933 rho))`: T1 15 frames, T1R 18, **T2 65
   (1.1 s)**, T3 147 (2.5 s), T4 179 (3.0 s) on average (j alone, no N dial: 143 / 214 / 2710 / 13655 / 20481
   frames, i.e. 2.4 s / 3.6 s / 45 s / 3.8 min / 5.7 min). Needs an emulator-built table of `val(N | t)`,
   because the TM1 jitter is chaotic (section 3.1); building it needs cycle-accurate agreement between the
   emulator and the human's machine, which is *unverified* (sub-tick differences scramble the starter).
3. Press frames at the three decisions: title A, naming OK, ball-dismiss A.

Per attempt: `P(success) = p_title * p_naming * p_ball`. Miss at the title: the wild seed is wrong (95-99.6%
not quiet) **and** the naming table is invalid (section 3.1) -> reset immediately if the timer program reports
the press error (assumed). Miss at naming: wrong starter; at the ball: wrong starter (visible at the summary
screen).

### 4.3 Real-time cost of resets (must be added to EHT; off the run timer)

Frames from boot, TAS pipeline (`route/defeat-brock/ledger.json` segment starts; FireRed 59.7275 fps):

| Decisive press | frames from reset | seconds (TAS pace) | human estimate (x1.25, **guess**, text speed MID before options) |
| --- | ---: | ---: | ---: |
| title A (wild seed) | ~620 (segment 01 = 646 includes NEW GAME) | 10.4 | 12 (+ up to a few s waiting for the chosen `t`) |
| naming OK (starter seed) | 2211 + 8 | 37 | 45 |
| ball-dismiss A (creation) | 2219 + 3755 + 172 - 2211 = 6137 | 102.8 | 120 (+8 s to open the summary) |

Expected real seconds per success with stage-wise abort (costs c1 = 12, c2 = 33, c3 = 83 + 8 = 91 s):
`E = (c1 + p c2 + p^2 c3) / p^3` for equal stage success `p`:

| p per press | attempts (1/p^3) | E seconds (abort at first miss) | E seconds (miss seen only at the starter screen: 128 s x attempts) |
| ---: | ---: | ---: | ---: |
| 0.95 | 1.17 | 146 | 159 |
| 0.90 | 1.37 | 158 | 187 |
| 0.80 | 1.95 | 189 | 266 |
| 0.70 | 2.92 | 232 | 397 |
| 0.50 | 8.0 | 410 | 1088 |

With the calibrated `sigma_f = 0.3` and `mu = 0..0.25` the single-press `p` is 0.90..0.79, giving **~160-210
seconds and 1.4-2.0 attempts per success**, plus the on-timer cost of section 4.2 (a few seconds for T2).

### 4.4 If the plan has no table (blind resets)

Then each attempt is a random (seed, offset) draw for both: P(starter T1) = 0.75%, P(T2) = 0.0396%; P(quiet <= 4
in the forest window) ~ 3-5%, <= 2 ~ 0.4%; joint (independent) T1 x quiet<=4 = 2.2-3.7e-4 (~2700-4500 attempts,
~110 hours at 128 s per attempt), T2 x quiet<=2 = 1.5e-6. Blind resets cannot get both; a table is
mandatory, which is why the model keeps hit probabilities of the *presses*, not of the outcomes.

### 4.5 What the model deliberately does not cover

- Real hardware / BizHawk agreement with the mGBA TM1 values to within ticks (tier 2 would need a `.bk2`
  with the planned presses; `docs/rival-1/route.md`). LeafGreen was not swept; the pipeline is the same but
  the seeds differ.
- Human jitter on intermediate presses (the model assumes the human aims the three decisive presses at an
  absolute elapsed time and the rest does not matter, true for the stream; false if the player blocks an NPC).
- Wild stream drift between the title seed and the forest (journal 2026-11-13-12-30): a "quiet" seed is quiet
  at roll 0, the route's own rate-test indices decide how much of it survives.

## 5. Reproduce

    cargo build --release -p frlg-route --examples
    ./target/release/examples/starter-sweep out.tsv 0 320 255 40 0 12   # N 0..320, j 0..255 for N%40==0
    ./target/release/examples/title-seed 0 648 > title.tsv
    python3 human-run/tools/analyze_sweep.py out.tsv
    python3 human-run/tools/wild_seed_scan.py
    cc -O2 human-run/tools/starter_scan.c -o starter_scan && ./starter_scan   # ~2 min
