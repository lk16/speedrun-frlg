# Session journal

Newest first. Continuity is something you write down; a sandbox ends mid-thought. Anything
unverified says so.

## 2026-08-12 (sandbox, night, addendum) -- "delay the first press, start later in the stream": tested, and it does not work that way

Question asked: the RNG moves before the first button press, so could the first press be
delayed to start later in the stream for free? Three-part answer, all measured
(`seed-probe` example):

- **The stream does move before any input** -- the VBlank `Random()` runs from boot
  (`decompiled/src/main.c:412`; 274 zeroed BIOS frames, then one step per frame from
  state 0) -- but both `SeedRng` calls overwrite the state, so nothing of the pre-press
  stream survives to the battle.
- **A delayed press re-rolls the seed; it does not slide the stream.** The seed is timer
  1 read at the title and naming screens' exit presses, and the timer starts *inside
  those screens* (`title_screen.c:351`, `naming_screen.c:428`, seed read at `:735`/`:722`
  via `main.c:264-269`). Measured stride 18753 ticks per frame: prepending 1 and 2 idle
  frames to the committed movie moved the naming seed `0xdf93 -> 0x4d7b -> 0x4cd2`, and
  the committed battle inputs lose on both. Unrelated seeds, not shifted ones.
- **And the delay is not free.** The run is scored in frames from power-on (README,
  `docs/route.md`, the ledger's `total_frames`; the tier-2 `.bk2` contains every frame
  from power-on). "TAS time starts at the first press" is not this project's rule, and --
  labelled as uncitable pretraining knowledge -- not how published TASes are timed either.

The constructive residue is a new dial in `docs/route.md`'s "What is not optimised":
idling N frames before the *naming-screen exit* press samples N fresh battle seeds at 1
frame each, which the sweeps only ever hit incidentally. A sampled seed wins if its
searched battle beats 2409 by more than N.

## 2026-08-12 (sandbox, night) -- the RNG gets a Rust model, the consumers get names, the free levers get priced

Task: model the RNG outside the emulator, verify it, and use it to ask whether the rival
battle can be had cheaper without paying idle frames. Everything landed in a new
zero-dependency crate, `crates/frlg-rng`, plus four examples; the route itself did not move
(the honest result below says why).

**The model, proven per frame.** `Rng` is the game's LCG
(`x' = 1103515245·x + 24691 mod 2^32`, top 16 bits returned --
`decompiled/include/random.h:18-19`, `src/random.c:11-12`), with an O(log n) `jump` over
precomputed power-of-two affine maps, an inverse step, and `distance_to` -- the discrete
log, total because the LCG is full-period, answering "how many `Random()` calls separate
these two observed states" in ~135 ns. Correctness is a replay, not an argument:
`tests/emulator.rs` replays the committed 9658-frame route and requires the model to
reproduce `gRngValue` exactly on **every** frame, with only the two `SeedRng` events
(title-screen and player-naming-screen exit) allowed to break stride, each verified to
unwind to a 16-bit seed within 3 steps. It passes. `random()` measures <1 ns, so a
million-state stream scan costs what one emulator frame costs.

**The consumers, measured and cited** (`rng-trace` and `field-experiments` examples;
citations in `docs/route.md`'s RNG section, which this session rewrote):

- The VBlank interrupt rolls once per frame unconditionally, **twice** in battle
  (`VBlankCB_Battle`, `decompiled/src/battle_main.c:1650` -- measured: all 2409 battle
  frames move the stream by exactly 2).
- **Pressing A never rolls** -- zero `Random()` on the whole text/menu/interaction path,
  and 600 frames idle vs hold-A vs mash-A consume identically. **Player walking never
  rolls** (only fishing does, `src/field_player_avatar.c:1711+`). Two same-length paths to
  the same tile left the stream identical at a common horizon: on open ground, path shape
  is RNG-neutral.
- What does roll in the field: wander/look-around NPCs
  (`src/event_object_movement.c:2716,2737,3037,3061,3090,3110` -- Pallet Town's two
  wanderers, the lab's three aides), 3 rolls per map load (`src/load_save.c:75,126`), the
  outdoor ambient-cry timer (`src/overworld.c:1141-1172`), and one trainer-id roll at new
  game (`src/new_game.c:56`). Idle in the NPC-free bedroom 2F: exactly 0 extra steps in
  600 frames. Two silencing gates matter for manipulation: the despawn window around the
  player (`event_object_movement.c:1798-1801`) and script freezes
  (`lockall`/`lock`, `scrcmd.c:1195-1221`, so the aides are silent through the whole
  scripted rival sequence).

**The free-shift question, asked directly.** If NPC-roll avoidance could shift the
battle-start stream by k at zero frame cost, what would k buy? `battle-scan` loads the
battle-start state (`gRngValue 0xed94271d` at frame 7249), writes `jump(k)` of it, and
races the battle as a pure `text_hold` mash across worker threads; `battle-plan-scan` runs
the route's full two-stage delay search under the same shift, anchored by reproducing the
committed battle bit-for-bit at shift 0 (2409 frames, plan `[4, 3, 3, 3]`, 38/64 start
delays winning -- and the anchor caught a real trap first: with a mash phase that runs
continuously instead of restarting at every advance stage like the route's
`advance_while`, the "same" search only reaches 2492. The drive shape is part of the
stream; the scanner now matches it exactly.)

Results, RNG-write exploration (tier 0, not evidence about any input log):

- Pure mash, shifts -60..60 x start delays {0,1,2,3,4,5,6,8}: **nothing beats 2409**.
  Best is shift 29, delay 3, at 2413 (+4); best pure-mash-no-delay is shift -6 at 2495.
  The top rows come in families of constant `shift + 2·delay`, which is the battle's
  2-rolls-per-frame arithmetic showing through: an idle frame in battle is a +2 shift
  that also costs a frame.
- The full two-stage search, run under shifts {-6, -4, -2, 21, 23, 25, 27, 29} (the
  grid's most promising, plus the realizable negatives): **-2 and -4 tie the committed
  2409 exactly**, with different plans (`[3, 0, 3, 2]` and `[4, 0, 3, 6]` against the
  committed `[4, 3, 3, 3]`); -6 and 29 reach 2410, 27 reaches 2411, and nothing beats
  2409. The committed battle sits on a floor that many neighbouring streams can reach by
  re-spending delays and none undercuts -- so no route change is recorded, and that is
  the result: the existing delay search was already extracting everything this
  neighbourhood of streams has to give.

**Who actually rolls in Pallet Town -- attribution done properly** (`who-rolls` example,
diffing `gObjectEvents` on every consuming frame). The map.json reads two wanderers; the
running game has **one**: the map's on-load script parks the sign lady at (5,15) as
`MOVEMENT_TYPE_FACE_UP` until her scene var moves
(`decompiled/data/maps/PalletTown/scripts.inc:27-28`), so she never rolls on this route.
Every extra field step while idling outside the house is the fat man -- each of his wander
moves pairs a direction roll with a delay roll ~16 frames later, ~13 steps per 600 frames
-- and his spawn is player-position-dependent (despawned at the house door until the
player steps south). That per-position spawn is the one genuinely free stream lever this
route has: a same-length path that changes how long he stays in the spawn window shifts
the battle stream by a few steps at zero frame cost. The scans above say those few steps
buy nothing here; the lever is machinery for future routes, not this one.

**Unverified / open.** The RNG-write scans are what-ifs by construction; any shift worth
realizing must be reproduced by real inputs and re-verified from reset before it touches
the ledger. Move choice in battle remains unsearched. The lab aides' roll pattern during
`07-starter`'s unfrozen stretches was traced but not itemised.

## 2026-08-12 (host) -- tier 2 passes the 9658 LeafGreen route, and with it the Header.txt rewrite

`tools/verify-runner.sh` replayed `route-9658f-269d169cd6db` on the host (`--realtime`, 177s,
watched): **pass** -- fingerprint `08cf8de0a6a46f6df6bd322b4b51a80a3cbe93ba` matches the
ledger's `09-battle-win`, and the per-frame `gRngValue` probe matched all 9658 frames, no
desync. Every segment's `tier2` field is stamped and `docs/route.md`'s status flips from
"queued, not replayed" to passed.

Two things it settles, the first of them the entry below's only open item:

- **The first non-FireRed replay.** BizHawk loaded the ROM whose sha1 the movie's own
  `Header.txt` names (`574fa542…`, BPGE) rather than a configured one, and stayed in sync for
  the whole movie. The version-aware header rewrite is proven format knowledge now, not
  tier-1-verified plumbing.
- **The three route changes hold outside tier 1.** `text_hold`, the fixpoint battle search and
  the six-cell sweep all re-rolled the battle RNG; the movie they produced desyncs nowhere
  against BizHawk's core, so the 9658 has the same frame-for-frame evidence 10085 and 12713 had.

Two notes for whoever queues the next one. The run used the artifacts-side Lua override
(`$FRLG_ARTIFACTS/verify/verify-runner.lua`), which is byte-identical to `tools/verify-runner.lua`
at `8e37210` -- verified by diff before stamping, since an override that had drifted would make
the pass a verdict on some other runner. And the exported `.bk2`'s sha1 is *not* reproducible:
two exports of the identical route gave `fd542690…` and `552449ba…` against the queued movie's
`be798799…`, because the zip container carries timestamps. The `ilog` digest is the movie's
identity -- re-exporting the committed logs reproduces `269d169cd6db…` exactly, which is what
ties this result to the tree.

**Unverified:** nothing new. The crit census still has not been re-run on the 9658 battle, and
the sweep tables below remain tier-1 evidence.

## 2026-08-12 (sandbox, after the tier-2 pass) -- hold A, search to a fixpoint, race all six cells: 10085 -> 9658

Three route changes landed together, because each re-rolls the battle and the honest score
is one full rebuild:

1. **`text_hold`** -- every dialogue mash now *holds* A (or B) for N frames per one-frame
   release instead of alternating. `RenderText` prints a character on every held frame once
   one press lands in the box (`decompiled/src/text.c:639-650`); the `[A, 0]` mash only held
   half the frames. Measured on the intro alone (upstream of the naming-screen reseed):
   hold 4 = 3229 frames vs mash's 3699, non-monotonic across N because the release phase has
   to line up with when boxes become ready (full table in `docs/route.md`).
2. **The battle search's per-turn stage repeats until a pass adopts nothing** (bounded at 8).
   Across the day's 146 builds, pass 2 adopted further cuts in 13, and one build (the
   LeafGreen/Charmander th8-xh1 re-run) kept adopting into pass 3 -- the repeat loop is not
   paranoia. Two lg-charmander variants were re-run because orphaned builds from a killed
   sweep briefly raced their directories; both re-runs reproduced the table's numbers, so
   the published table stands.
3. **Version and starter are swept, not assumed.** `bin/frlg-sweep` runs a 24-variant tuning
   sweep (`turn_hold` 1-8 x `text_hold` {1,2,4}) as parallel builds; six sweeps covered
   every version x starter cell. Best of each, total frames:

   |            | Squirtle | Charmander | Bulbasaur |
   | ---        | ---:     | ---:       | ---:      |
   | FireRed    | 9789     | 9749       | 9666      |
   | LeafGreen  | 9747     | 9741       | **9658**  |

   **LeafGreen with Bulbasaur wins at 9658** (`turn_hold` 4, `text_hold` 4, battle plan
   `[4, 3, 3, 3]`, 3 turns, 2409 frames) and is now the committed route. Bulbasaur was the
   *worst* starter in the old mashed table; both its cells win here with 3-turn battles, and
   both are also the most fragile (10/24 and 5/24 variants lose outright). The `text_hold 1`
   column of the FireRed/Squirtle sweep reproduced the previous sweep's totals exactly where
   the stream was unchanged (10085 at th2, 10531 at th8, th7 loses), which is the
   determinism check for free.

   LeafGreen needed: the version read from the ROM header (BPRE/BPGE at 0xAC,
   `decompiled/config.mk:29-57`) because the rival's preset rows differ
   (`sRivalNameChoices`, `decompiled/src/oak_speech.c:649-658` -- RED is row 1 on LG, one
   DOWN, where FR's KAZ was two wrapping UPs); the `.bk2` export writing the movie's own
   ROM name and sha1 into `Header.txt` (everything else stays the template's bytes); and
   `tools/verify-runner.sh` picking the ROM the movie header names out of
   `$FRLG_ARTIFACTS/rom` instead of playing everything on FireRed.

**Unverified:** the 9658 movie's tier-2 replay (queued), and with it the Header.txt rewrite
-- the first LeafGreen replay is what proves that format move. The crit census has not been
re-run on the new battle. The sweep tables above are tier-1 evidence.

### The six sweeps, per variant (total frames; tier-1 evidence, sweep dirs die with the sandbox)

**fr-squirtle** (rows `turn_hold` 1-8, columns `text_hold` 1/2/4):

| `turn_hold` | xh1 | xh2 | xh4 |
| ---: | ---: | ---: | ---: |
| 1 | 10483 | 10043 | 9852 |
| 2 | 10085 | 10207 | 9960 |
| 3 | 10540 | 9951 | 9846 |
| 4 | 10386 | 9953 | 9789 |
| 5 | 10267 | 10206 | 9847 |
| 6 | 10087 | 9952 | 9929 |
| 7 | loses | 9852 | 10175 |
| 8 | 10531 | 10002 | 10117 |

**fr-charmander** (rows `turn_hold` 1-8, columns `text_hold` 1/2/4):

| `turn_hold` | xh1 | xh2 | xh4 |
| ---: | ---: | ---: | ---: |
| 1 | 10075 | 10165 | 9945 |
| 2 | 10526 | 10162 | 9978 |
| 3 | 10076 | 10166 | 9937 |
| 4 | 10551 | 10043 | 9753 |
| 5 | 10077 | 10051 | 9938 |
| 6 | 10356 | 10167 | 9749 |
| 7 | 10165 | 10090 | 9939 |
| 8 | 10352 | 10164 | 9947 |

**fr-bulbasaur** (rows `turn_hold` 1-8, columns `text_hold` 1/2/4):

| `turn_hold` | xh1 | xh2 | xh4 |
| ---: | ---: | ---: | ---: |
| 1 | 10205 | 10041 | 9748 |
| 2 | 10318 | 10038 | 10211 |
| 3 | 10359 | loses | loses |
| 4 | 10356 | 9776 | 9951 |
| 5 | 10360 | loses | 9861 |
| 6 | 10357 | 10230 | 9666 |
| 7 | 10358 | 9780 | 9939 |
| 8 | loses | 9780 | loses |

**lg-squirtle** (rows `turn_hold` 1-8, columns `text_hold` 1/2/4):

| `turn_hold` | xh1 | xh2 | xh4 |
| ---: | ---: | ---: | ---: |
| 1 | 10260 | 9946 | 10105 |
| 2 | 10343 | 9747 | 9921 |
| 3 | 10261 | 9947 | 9839 |
| 4 | 10572 | 9751 | 9954 |
| 5 | 10208 | 10213 | 9845 |
| 6 | 10573 | 10228 | 9840 |
| 7 | 10578 | 10023 | 9802 |
| 8 | 10396 | 10023 | 9781 |

**lg-charmander** (rows `turn_hold` 1-8, columns `text_hold` 1/2/4):

| `turn_hold` | xh1 | xh2 | xh4 |
| ---: | ---: | ---: | ---: |
| 1 | 10160 | 9750 | 9741 |
| 2 | 10474 | 10168 | 9947 |
| 3 | 10161 | 10280 | 9742 |
| 4 | 10475 | 10170 | 10063 |
| 5 | 10276 | 10197 | 9748 |
| 6 | 10476 | 10181 | 9932 |
| 7 | 10473 | 10167 | 9744 |
| 8 | 10476 | 10167 | 9744 |

**lg-bulbasaur** (rows `turn_hold` 1-8, columns `text_hold` 1/2/4):

| `turn_hold` | xh1 | xh2 | xh4 |
| ---: | ---: | ---: | ---: |
| 1 | 10344 | 10117 | loses |
| 2 | loses | 10227 | loses |
| 3 | 10349 | 10033 | 9858 |
| 4 | loses | 10109 | 9658 |
| 5 | 10350 | loses | 10047 |
| 6 | loses | 9766 | loses |
| 7 | 10345 | 10044 | loses |
| 8 | loses | 10048 | loses |

Sweep mechanics worth keeping: 12 parallel builds on 16 cores, ~12.5 min per build wall
clock, six 24-variant sweeps in an afternoon. `frlg route tune` serially would have been
~30 hours; `bin/frlg-sweep` exists because of that arithmetic.

## 2026-08-12 (host, then sandbox) -- tier 2 passes the 10085 route; three observations from watching it

`tools/verify-runner.sh` replayed `route-10085f-65ef20333a57` on the host (`--realtime`, 172s):
**pass** -- fingerprint `e65e93b6712b408ed915f55e46c9a79f874016cd` matches the ledger, and the
per-frame `gRngValue` probe matched all 10085 frames. The fully optimised route now has the
same evidence the 12713 predecessor had; every segment's `tier2` field is stamped, and
`docs/route.md` no longer carries the "queued, not replayed" caveat.

Luuk watched the replay and asked for three things. Two are answered by the decomp, one is
real work:

- **"Set text speed FAST at game boot, before the Oak speech"** -- not available on this game.
  FireRed's main menu has no OPTION entry (`decompiled/src/main_menu.c:23-35`: NEW GAME /
  CONTINUE / MYSTERY GIFT are the only rows; the RSE options row does not exist here), and a
  clean power-on *resets* the options anyway: with no valid save,
  `Sav2_ClearSetDefault()` runs at the title screen (`decompiled/src/title_screen.c:740-741`)
  and forces `OPTIONS_TEXT_SPEED_MID` / animations on (`decompiled/src/new_game.c:60-68`).
  The first options access is the field start menu (`StartMenuOptionCallback`,
  `decompiled/src/start_menu.c:531`), which is exactly where `04-options` sits. **But the
  equivalent is reachable without the menu**: Oak's speech enables `canABSpeedUpPrint`
  (`decompiled/src/oak_speech.c:761-762`), and `RenderText` zeroes the per-character delay on
  any frame where A or B is *held* once one press has landed (`decompiled/src/text.c:639-650`).
  The route's `[A, 0]` mash only holds A half the frames, so intro text prints at ~2
  frames/char; a hold-heavy duty cycle prints at ~1, which is FAST-speed text before the
  option exists. That is the next route change.
- **"Battle animations off at boot"** -- same lock: the option resets at the title screen and
  there is no menu until the field. It also would buy nothing the route does not already have:
  the only battle is after `04-options`, which turns animations off before it.
- **"No indication LeafGreen or other starters were tried"** -- correct, and now a task, not a
  footnote. The starter table in `docs/route.md` is one *mashed* sample each on mGBA 0.10.5,
  three route generations stale; LeafGreen has never been built into a route at all. Both get
  redone against the current core with the full two-stage battle search.

**Unverified:** nothing in this entry; the pass is a host result and the three items above are
citations plus planned work.

## 2026-08-12 (sandbox, evening) -- the stale turn_hold falls to a re-sweep: 10531 -> 10085

`frlg route tune`, re-run for the first time since mGBA 0.10.5, and the stale knob was worth
446 frames: **turn_hold 2 wins at 10085** (battle 2466, plan `[1, 10, 0, 15]`, 5 turns),
where the carried-forward 8 scored 10531. Full sweep, every variant a complete build through
the two-stage battle search: 1 -> 10541, 2 -> **10085**, 3 -> 10540, 4 -> 10386, 5 -> 10267,
6 -> 10087, 7 -> **loses**, 8 -> 10531. Tier-1 verified from reset, exported and queued as
`route-10085f-65ef20333a57`; the 10531 queue entry withdrawn unreplayed. `Tuning::default()`
now says 2, with the sweep in its comment.

Two things the sweep surfaced beyond the number:

- **A knob value can lose outright, and the sweep must survive that.** turn_hold 7's stream
  gave 64 losing battles out of 64 start delays -- stage 2 never got a winner to refine. The
  first sweep aborted on it (the variant's build error propagated); `frlg route tune` now
  scores a timeout as "loses" and keeps sweeping, because a value that cannot win is an
  answer, not an outage.
- **The two leaders sit 450 frames clear of the pack and 2 frames apart** (10085 vs 10087 for
  turn_hold 6), and both got there through different battle plans on different streams. The
  knob is not doing anything mechanical -- it is picking which RNG lineage the battle search
  gets to fish in, same as every upstream frame.

**The crit census moved with the battle**: the 2466-frame battle contains *two* crit windows,
both attacker 0 = us (`gCritMultiplier`/`gBattlerAttacker` traced over the committed log,
`B_POSITION_PLAYER_LEFT`, `decompiled/include/constants/battle.h:28`). Two crits in five turns
is why it is short -- and it is found luck, not aimed-for luck; the route notes keep that
distinction.

**Unverified.** Tier 2 for `route-10085f-65ef20333a57`: queued, not replayed. Starter choice
and LeafGreen remain the two untouched fronts (`docs/route.md`).

## 2026-08-12 (sandbox, later) -- the battle search grew a per-turn stage: 10946 -> 10531

The battle was the one big item left after the morning's rebuild (entry below), and the "obvious
next machine" the route notes had described since 2026-08-11 now exists. `gBattleMainFunc`
re-enters `HandleTurnActionSelectionState` at the top of every turn (`BattleTurnPassed`,
`decompiled/src/battle_main.c:2998`), so each turn's action menu is a decision point; stage 2
of the search walks the stage-1 winner's turns greedily, tries idling 1-15 frames at each
menu with the rest of the battle replayed in full per trial, and adopts only a shorter battle.
Result: **battle 3322 -> 2907, total 10531**, plan `[0, 1, 0, 1, 6, 1]` over 9 turns, tier-1
verified from reset, exported and queued as `route-10531f-e037421ddd87` (the 10946 queue entry
was withdrawn unreplayed).

Two observations worth keeping:

- **The landscape is rugged, and the first step was huge.** Stage 2's first adopted delay --
  one idle frame at turn 1 -- cut ~1200 frames: a different damage-roll lineage, not a trim.
  The three later adoptions were worth a frame or two each. That asymmetry says a second
  greedy pass (or a joint start x turn search) could pay again; single-pass greed is in the
  route notes as the top remaining battle item.
- **The mash's phase is part of the stream.** Rewriting the drive from one continuous mash to
  advance-to-observable stages realigned which frames carry A presses, and the same 64 start
  delays produced a different family of battles (64/64 winning where the previous shape had
  32/64, stage-1 best 4109 where the old mash's was 3322). Neither shape is wrong -- the log
  is the artifact and it verifies -- but battle numbers are only comparable within one drive
  shape.

**Answered while the sweep ran: the committed battle's one crit is ours.** Tracing
`gCritMultiplier` and `gBattlerAttacker` over the replayed `09-battle-win` log shows a single
crit window (multiplier 2 from battle frame 1488) with attacker 0, `B_POSITION_PLAYER_LEFT`
(`decompiled/include/constants/battle.h:28`). The previous route ate a *rival* crit; this
stream hands the crit to us. Luck found, not aimed for -- noted in `docs/route.md`.

**Unverified.** Tier 2 for `route-10531f-e037421ddd87`: queued, not replayed. The `turn_hold`
sweep on the final code is running as this is written; its result lands in the next entry.

## 2026-08-12 (sandbox) -- 12713 -> 10946: the three cheap inefficiencies are routed out

Task: optimise the route to the rival win. The three items the 2026-08-11 tier-2 viewing put on
the list -- the seven-character names, MID text speed, battle animations -- are now all gone,
one build, measured end-to-end through the battle. **10946 frames, -1767 (-13.9%), tier-1
verified from reset, exported and queued as `route-10946f-b1a0875a77e9`.** Segment numbering
shifted: the new `04-options` pushes everything after it up by one (`09-battle-win` is the
battle now).

**What the 1767 frames are.** `03-names` types one letter, START (a documented cursor shortcut
to OK, `decompiled/src/naming_screen.c:1485`), A -- and takes KAZ off the rival's preset menu
instead of a second naming screen (rows are `sRivalNameChoices`, `oak_speech.c:647`; the menu
wraps, so it is two UPs from the top). 1450 -> 1238. `04-options` opens START -> OPTION in the
bedroom and sets text speed FAST plus battle scene OFF in one 197-frame detour. Everything
downstream got cheaper: `07-starter` -794 (its text at 1 frame/char instead of 4), `06-to-lab`
-150, `08-battle-start` -88, `05-house` -24, and the battle -- fresh stream, re-searched, 8/16
start delays win -- came in at 3322, -696.

**Two wrong assumptions the decomp corrected, worth keeping:**

- **There is no preset menu for the player.** The 2026-08-11 route notes implied preset names
  were "two D-pad presses away" for both names. The flow is asymmetric:
  `Task_OakSpeech_YourNameWhatIsIt` fades the player straight into the naming screen
  (`oak_speech.c:1352-1379`); the player's preset menu exists only on the say-NO re-ask path.
  The rival's menu is real and literal. (Near-miss worth noting: the player's name buffer is
  *prefilled* with `sMaleNameChoices[Random() % 19]` before the naming screen opens
  (`Task_OakSpeech_DoNamingScreen` -> `GetDefaultName`, `oak_speech.c:1444,2146`), so
  START+A on an untouched screen keeps a random 3-6 char preset. Rejected: a searched-delay
  3-char draw is never better than the deterministic 1-char typed name.)
- **Single-frame taps die in the start menu.** The first options attempt tapped UP twice and
  pressed A on EXIT: while the start menu is up, `gMain.newKeys` goes stale in runs of 2-3
  frames -- input reads get skipped -- and the field swallows everything for ~20 frames after
  the walk-in transition (`Task_ExitNonDoor`). The fix is structural, not a longer wait: every
  press in `04-options` is a mash-until-effect against a RAM observable
  (`sStartMenuCursorPos`, the option menu's working values, its `loadState`), which stops on
  the frame the effect lands and cannot overshoot. New observer probes for all of it, each
  checked against the running game in `tests/observe.rs`.

**Also written down while in there:** the run's RNG stream is seeded twice, both from timer 1
-- at title-screen exit and again at *player* naming-screen exit (`SeedRngAndSetTrainerId`,
`title_screen.c:735`, `naming_screen.c:722`, `main.c:264`) -- so manipulation upstream of the
naming exit cannot reach the battle except by moving the exit itself. In `docs/route.md`'s RNG
section now.

**Unverified.** Tier 2 for the new movie: queued, not replayed (the 12713 pass covers the
previous movie only -- same boot, core and format, but plausible is not proven). The
`turn_hold` sweep is still the mGBA-0.10.5 one, now two route generations stale; `frlg route
tune` on the current route has not been re-run. Whether the new battle contains a crit either
way: not checked.

## 2026-08-11 (host, night) -- TIER 2 PASSES: BizHawk replays the whole route frame for frame

**The headline, for whoever picks this up in a sandbox: `route-12713f-a4ad4280bbdc` passed.**
Luuk ran `tools/verify-runner.sh` on the host and the result is in
`$FRLG_ARTIFACTS/verify/results/route-12713f-a4ad4280bbdc.json`:

    "verdict": "pass"
    "ram_hash": "73b329af5d561a864cc4b0d46e8d4c409ce1b6df"   (== expected_ram_hash)
    "notes":   "replayed 12713 frames; fingerprint matches; probe trace matched every frame"

Read the third line twice. The final fingerprints agreeing would have been a pass; the
**`gRngValue` trace matching on all 12713 frames** means the two emulators never diverged for a
single frame anywhere in the run. The boot fix was the whole desync, the 2026-08-12 audit that
found no second cause was right, and the trace machinery that was built to name a desync frame
instead got used to prove there was no desync to name.

What this closes: the ledger's per-segment `tier2` no longer says "not replayed" (it names the
passing run and the `ilog` digest it was built from); `docs/route.md`'s header no longer says
"tier 1 only"; and `tools/verify-runner.lua`, three sessions old and never once having completed
a report, has now completed one end to end -- status file, 288K RAM dump, per-frame compare.
Rebuilding the route resets the tier-2 stamp, and should: a rebuilt movie has not been replayed.

**One inconsistency worth knowing before it wastes someone's afternoon.** Re-exporting the same
route produces the same `ilog_sha1` and a *different* `bk2_sha1` (`f60e7120…` then `4d947b73…`).
The `.bk2` is a zip and its entry timestamps move; the input log is identical, which is why the
re-exported movie replayed to the same fingerprint. The `.ilog` digest is the identity, the
`.bk2` hash is not. Noted in `docs/route.md`.

### Three inefficiencies, now written down with citations

Luuk watched the run and named three. All three are real, and one of them contradicts something
this journal has claimed since 2026-08-10.

**Battle animations are on.** `NewGameInitData` sets `optionsBattleSceneOff = FALSE`
(`decompiled/src/new_game.c:66`) and nothing in the route ever opens OPTIONS, so
`BattleStartClearSetData` never sets `HITMARKER_NO_ANIMATIONS` (`decompiled/src/battle_main.c:2259`
-- and neither of the two battle types that would block it, LINK and POKEDUDE, applies here).
Every attack in the 4018-frame battle plays its animation. The switch is `MENUITEM_BATTLESCENE`
(`decompiled/src/option_menu.c:514`), reached through START -> OPTION
(`decompiled/src/start_menu.c:531`).

**The name is seven characters, and each surplus character has a price now.** The naming screen
mash types A until the name fills up, and every later message box that prints the name pays for
all seven. The price: `sTextSpeedFrameDelays` is `{SLOW: 8, MID: 4, FAST: 1}` frames per
character (`decompiled/src/new_menu_helpers.c:27-32`), and the route runs at the default MID. Six
surplus characters is 24 frames per message box that prints the name. This also prices the text
speed item that has been on the list since 2026-08-10: FAST is a 4x on every character in the
run, and it is the *same* OPTIONS detour as the battle animations, so the two should be priced
together rather than one at a time.

**"Bulbasaur crits us" -- and this journal said that was impossible.** It was wrong, and the
decomp says so plainly. The 2026-08-10 entry (and `docs/route.md`) claimed criticals are off for
the whole first battle because of `BATTLE_TYPE_FIRST_BATTLE`. The actual condition is
`&& (!(gBattleTypeFlags & BATTLE_TYPE_FIRST_BATTLE) || BtlCtrl_OakOldMan_TestState2Flag(1))`
(`decompiled/src/battle_script_commands.c:1200`), and that second clause was never read.
`BtlCtrl_OakOldMan_TestState2Flag(1)` tests `FIRST_BATTLE_MSG_FLAG_INFLICT_DMG`
(`decompiled/src/battle_controller_oak_old_man.c:2228`, `decompiled/include/battle_controllers.h:287`),
which `CompleteOnHealthbarDone` sets the first time an opponent's hit finishes draining the health
bar, on its way to Oak's "inflicting damage is key" line
(`decompiled/src/battle_controller_opponent.c:304-306`). So criticals are suppressed for the
opening exchange only and are live for the rest of the battle, for both sides. Two things follow:
the crit `Random()` call is consumed on every damaging hit regardless, because `&&` short-circuits
left to right and the roll sits *before* the `FIRST_BATTLE` clause; and the rival's crit is a
search target, not a fact of life -- it costs damage (possibly a turn) plus a message box, in a
stream `08-battle-win` already re-searches. Nobody has measured the battle without it yet.

The lesson is the boring one: a citation that stops at the first `&&` is not a citation. This one
survived two sessions because it was *nearly* right.

### The runner: 213s -> 31s, once it stopped talking to a real X server

Replaying at 100% costs exactly what the TAS costs. `tools/verify-runner.sh` now seeds EmuHawk's
`config.ini` (plain JSON) before each launch: `Unthrottled`, no clock/vsync/sound throttle,
`DispSpeedupFeatures: 0` (its `MainForm::Render` returns immediately -- read from EmuHawk.exe IL,
not guessed), sound off, and the dialog suppressors that keep an unattended run from parking on a
modal window. `--realtime` puts the desk settings back, because watching a replay is what
produced three of the findings above and must stay one flag away.

**Determinism is checked, not assumed**: the same movie, replayed fast and silent, produced the
same fingerprint *and* the same 12713-frame probe trace as the 100%-with-sound run. That is the
pass quoted at the top of this entry -- it was replayed twice.

Unthrottling on the desktop bought only 1.6x -- 134s, ~95 fps -- and the shape of the miss was
the clue: 36s of CPU across 134s of wall clock, so the process was *waiting* ~8ms per frame
rather than working. Luuk installed `xvfb`, and `--headless` (`xvfb-run`) answered it:

    stock EmuHawk                      ~213s    59.7 fps, i.e. the movie's own length
    seeded config, on the desktop       134s    ~95 fps
    seeded config, --headless            31s    ~410 fps      <- default
    --headless, DispSpeedupFeatures 1    47s    ~270 fps
    --headless, DispSpeedupFeatures 2    47s    ~270 fps

**6.9x, and the win is the X server rather than the throttle.** Headless is 32s of CPU across
31s of wall -- the replay is finally CPU-bound, nothing waits. So the ~8ms/frame was the desktop
X connection, most likely the per-frame `UpdateWindowTitle()` that `DispSpeedupFeatures == 0`
switches on (`CalcFramerateAndUpdateDisplay`, EmuHawk.exe IL): a round trip per frame, cheap
against a local Xvfb, expensive against a real desktop. The last two rows are the same
experiment run the other way and they justify keeping `DispSpeedupFeatures: 0` -- letting
EmuHawk render costs 16s even with nothing to display it on. All four replays passed with the
identical fingerprint and trace, which is four more independent confirmations of the tier-2
result at the top of this entry.

`FRLG_VERIFY_CONFIG_EXTRA` (a JSON object applied on top of the seeded config) exists so the
next person to doubt one of these settings can measure it instead of arguing with the IL.

Note what headless is and is not: the sandbox still cannot run BizHawk (no mono, no installs),
so this does not move tier 2 into the sandbox. What it buys is `--watch --headless` draining
the queue on the host without taking over a screen.

**Unverified.** Segment-level tier-2 requests, which have never been made -- only the whole
route has ever been replayed. Everything else in this entry was measured.

## 2026-08-11 (sandbox, late) -- hunted for a second desync cause and found none; hardened the thing that names the frame

Task: find the desync and fix it. The desync on record is the tier-2 bedroom stall, root-caused
last session to the skipped BIOS intro and fixed; the rebuilt `route-12713f-a4ad4280bbdc` sits
in the queue unreplayed, and the only tier-2 result is the pre-fix runner error. So this session
did the two things the sandbox can do: audit every remaining divergence axis against the mounted
sources, and make sure the next host run cannot fail to produce a frame number again.

**The audit came back empty -- no second desync cause.** Checked, with citations (BizHawk/mGBA
claims cite the read-only mounts; this is tier-2 material, not routing):

- *Input latch order*: `bizinterface.c:518` (`$FRLG_DEPS/mgba/src.tar.gz`) does
  `core->setKeys(keys)` then runs the frame -- identical to our `frlg_run_frame` (`shim.c:182`).
  The `keyCallback` BizHawk installs (`bizinterface.c:360`) returns the same per-frame mask
  `setKeys` stored, so KEYINPUT reads see equal values on both tiers.
- *Movie latch indexing*: `MovieSession.cs:96,322` latches input-log row `Emulator.Frame`
  before each advance, and `MGBAHawk.IEmulator.cs:83` increments `Frame` after -- so row 0
  drives the first advanced frame, exactly like tier 1's `log[0]`. Playback flips to FINISHED
  at `Frame == FrameCount` (`MovieSession.cs:112`), no off-by-one.
- *Savedata*: BizHawk hands mGBA a 0xFF-memset buffer (`bizinterface.c:347`); tier 1 attaches
  no save VFile, and `GBASavedataInitFlash` (`savedata.c`) memsets the anonymous map to 0xFF in
  that case. Same erased-flash bytes either way.
- *Idle loop*: mGBA's default is `IDLE_LOOP_REMOVE` (`gba.c:120`), but removal needs a known
  address and BPRE's override row says `GBA_IDLE_LOOP_NONE` (`overrides.c:134`), so it equals
  BizHawk's forced `IDLE_LOOP_IGNORE`. Both sides also converge on FLASH1M + HW_NONE for retail
  BPRE (`bizinterface.c:450`, crc `0xDD88761C` in the known-Pokémon table, so no romhack
  compat), confirming last session's shorter check.
- *`.bk2` decode*: `Bk2Controller.SetFromMnemonic` parses rows strictly in `LogKey` order, so
  the template-copied key plus our column table is the whole story.

**The queue entry re-verifies from scratch.** `frlg log cat` of the eight committed logs
reproduces digest `a4ad4280…`; a cold tier-1 replay reproduces the request's
`ram_hash 73b329af…` and matches the queued 12713-frame `gRngValue` trace byte-for-byte; an
independent Python decoder (game key bits, not the exporter's code) decodes the queued `.bk2`
to exactly those masks. Whatever tier 2 says, it will be about the emulators, not the artifact.

**The Lua's assumptions are no longer guesses.** BizHawk ships typed Lua API docs
(`$BIZHAWK_HOME/Lua/_docs_luacats/`) that the desync hunt had never opened:
`memory.read_u32_le(addr, domain)` and `memory.readbyterange(addr, length, domain)` are the
real signatures, `readbyterange` returns a zero-indexed table (its own doc string, extracted
from `BizHawk.Client.Common.dll`), `movie.mode()` returns exactly
`"PLAY"|"RECORD"|"FINISHED"|"INACTIVE"`, the mGBA domains are named `EWRAM`/`IWRAM`
(`BizHawk.Emulation.Cores.dll`), and `event.onframeend`/`event.onexit`/`client.exit` all
exist. Every assumption the script makes checked out as written.

**The fix this session: the runner can no longer lose the frame number.** The watched
2026-08-11 replay ran, desynced, was closed by hand -- and recorded nothing, because the Lua
wrote its status only at a finish it never reached (`EmuHawkMono_last*.txt` in the deps tree
shows it: Lua loaded, no report). Now `verify-runner.lua` writes the status file the moment
the probe first mismatches, every 300 frames as a heartbeat, and from `event.onexit`; the
shell's timeout branch reads the partial status into the result instead of discarding it. The
new Lua is installed at `$FRLG_ARTIFACTS/verify/verify-runner.lua`, the override the runner
prefers, so the next host run uses it even though the host checkout cannot see this commit.
The shell half only lands when the host pulls.

**Unverified.** Still the same one thing: no tier-2 result for `route-12713f-a4ad4280bbdc`.
The audit narrows the space -- if it still desyncs, the trace frame number is the lead and
there is no named suspect left -- but narrowing is not a pass.

## 2026-08-12 (sandbox) -- the bedroom desync was the boot: BizHawk never skips the BIOS intro for a movie

**Root cause, cited.** `MGBAHawk.cs:41` (2.11.1 sources in
`$FRLG_ARTIFACTS/reference/bizhawk-2.11.1/`) constructs the core with
`skipBios: _syncSettings.SkipBios && !lp.DeterministicEmulationRequested`. Movie playback
requests deterministic emulation -- that is the same condition that made line 30 throw
`MissingFirmwareException` on the host until the BIOS existed -- so the template's
`SkipBios: true` is dead on replay, `bizinterface.c:171`'s `GBASkipBIOS` never runs, and EmuHawk
plays every movie through the ~272-frame BIOS boot animation while consuming movie input. Tier 1
booted `opts.skipBios = true`, so on BizHawk the entire log ran ~272 frames early. The failure
shape matches the watched replay exactly: mash segments absorb a constant shift, and the first
frame-exact walking (bedroom -> downstairs) dies. The other suspects were checked against the
sources while getting here and came up equal for retail BPRE: overrides/savetype paths converge
(`GBAOverrideFind` static table, FLASH1M, HW_NONE), idle loop is a no-op on both (REMOVE with
`idleLoop == NONE` vs IGNORE), RTC is inert (no RTC hardware on BPRE), vbaBugCompat only touches
HLE SWIs and GPIO, neither of which this cartridge exercises.

**The fix.** `frlg_core_load_bios` grew a `skip_intro` flag; `boot_with_default_bios` boots
real-BIOS-with-intro and stamps the ledger `bios+intro:<sha1>` -- a new marker on purpose, so
the retired skip-intro `bios:<sha1>` evidence can never be mistaken for the new boot. Route
rebuilt: **12713 frames** (the intro costs ~272 at boot; the battle re-rolled to 4018 frames,
16/16 start delays win, delay 1 kept), tier-1 verified, exported, queued as
`route-12713f-a4ad4280bbdc`. The stale 12222-frame queue entry was withdrawn. Two tests assumed
the old boot (replay-from-HLE-reset, copyright screen at frame 60) and now boot/wait properly.

**Desyncs now come with a frame number (when the Lua cooperates).** `frlg route export` replays
the exported movie once on tier 1 and queues `<id>.trace` beside it: gRngValue after every frame
(the game advances it once per VBlank, `decompiled/src/main.c:412`), u32 LE per frame. The
replay doubles as an export gate -- a movie whose final fingerprint is not the ledger's refuses
to queue. `verify-runner.lua` compares the probe each frame and reports `desync_frame=`;
`verify-runner.sh` forwards it into the result json. The trace sanity-checks itself: its first
273 values are zero (BIOS animation, RNG unseeded), which independently measures the intro
length. Contract updated in `docs/route.md`.

**Unverified.** The fix's *effect*: no tier-2 result exists yet for the rebuilt movie. The
reasoning is cited but BizHawk has not replayed it. The Lua trace compare has never run (the Lua
has still never completed a report of any kind); its read API
(`memory.read_u32_le(offset, domain)`, framecount-1 indexing per `MGBAHawk.IEmulator.cs:83`) is
the least-tested part.

## 2026-08-11 (evening, host) -- tier 2 ran for the first time, and the first watched replay desyncs in the bedroom

**The BIOS exists.** A downloaded `gba_bios.zip` hashed to exactly
`300c20df6731a33952ded8c436f7f186d25d3492` (16384 bytes, the World BIOS) and is installed at
`$BIZHAWK_HOME/Firmware/GBA_bios.rom`. Doctor is green on it. The route was rebuilt booting from
it: **12222 frames** (real-BIOS boot costs 13 over HLE's 12209, spread over segments 01-03/07),
the battle re-rolled again and now **16/16 start delays win** -- delay 0 kept, 3797 frames.
Verified tier 1, exported, ledger says `bios:300c20df…`.

**Two runner bugs stood between the queue and EmuHawk**, both now fixed in
`tools/verify-runner.sh`: `--lua` was passed relative, and `EmuHawkMono.sh` cd's into the BizHawk
directory first, so the script was never found; and `--userdata` is not a data directory at all
-- it is movie key:value metadata whose parser exits 1 on a bare path ("malformed userdata",
found by bisecting the flags against a live EmuHawk). `--config="$USERDATA/config.ini"` is what
keeps the churn out of the deps tree.

**Then the real result: the movie plays and desyncs.** Watched on the GUI: power-on, menu mash,
naming screen, into the bedroom -- and the player never walks downstairs; the run stalls there.
The shape of the failure is informative: mash segments are robust to small input misalignment,
`nav`'s frame-exact walking is not, and walking is exactly where it died. Prime suspects, in
order: input-delivery timing (when BizHawk's mGBA latches a movie frame's keys vs. our
`setKeys`-then-`runFrame`), then the rest of `SyncSettings`/RTC. Both tiers run the same core
commit and BIOS boot now, so the emulator itself is off the suspect list -- which is what this
whole week of pinning bought. The runner's Lua report has still never completed, so there is no
frame number yet; that diagnosis is the top tier-2 item (`docs/route.md`).

**Watching the replay also put three route questions on the record** (now in `docs/route.md`,
"What is not optimised"): the name is a seven-A wall typed one press at a time and re-printed at
every name mention (one-character name and preset name both unmeasured); text speed is never set
to FAST and every message box in the run pays for it; and LeafGreen builds byte-exact but has
never been raced against FireRed. All three must be measured through the battle, and the version
question through a full build-and-tune.

**Unverified.** The desync location (eyeballed, no frame number); everything downstream of it.

## 2026-08-11 (later, host) -- same core on both tiers, a .bk2 writer, and BIOS wiring; the route re-rolled to 12209

Worked the three items `docs/route.md` still listed under tier 2, on the host (network, mono,
docker all present). Two are closed; the third is wired and waits on one file.

**Both tiers now run mGBA `94b1578f`** -- BizHawk 2.11.1's own submodule gitlink, self-reported
0.11.0. `MGBA_REF` defaults to it, the deps tree is rebuilt, and `bin/frlg-doctor`'s `mgba pin`
check now passes when our pin equals the recorded submodule. The shim port
(`crates/mgba-sys/csrc/shim.c`) took four changes: `getGameTitle`/`getGameCode` →
`getGameInfo` (the "AGB-BPRE" format is reconstructed so the Rust side is untouched),
`desiredVideoDimensions` → `baseVideoSize`, `color_t` → `mColor`, and an explicit
`#include <mgba/flags.h>` since 0.11's `common.h` no longer pulls it in. The trap worth
remembering: **the installed `flags.h` lies about `ENABLE_DIRECTORIES`** -- upstream
`CMakeLists.txt:869` appends the compile definition whenever `ENABLE_VFS` is on, but no cmake
*variable* of that name exists, so `#cmakedefine ENABLE_DIRECTORIES` stays undefined. The flag
gates a 4152-byte `struct mDirectorySet` embedded in `struct mCore` ahead of the vtable, so the
shim compiled clean and then called a NULL pointer. Diagnosed by dumping the real allocation
(vtable starts at byte 4856; our `offsetof(init)` said 704; the difference is exactly
`sizeof(mDirectorySet)`); the shim now defines the flag itself, with the citation.

**The pin moved the route: 11873 → 12209.** On the new core, segments 01-07 replay bit-identically
to their observables (same frame counts; RAM digests differ, as expected between core versions),
and the old `08-battle-win` log *loses* -- the battle RNG stream is not the same. `frlg route
build` re-searched the 16 start delays (8 win now), kept delay 0, and the chosen battle is 3797
frames. Every number that predates the pin is labelled as such in `docs/route.md`. The lesson from
2026-08-10 generalises: the battle is a hash of everything upstream, *including the emulator*.

**`frlg route export` writes the `.bk2`** (`crates/frlg-route/src/bk2.rs`). Template entries are
copied verbatim, only `Input Log.txt` is generated; the ledger's digests gate which logs may be
exported; every export decodes its own output back to masks and compares before reporting
success, and deletes the file on mismatch. The button mnemonics (`U D L R S s B A l r P`) came
out of BizHawk's `ControllerDefinition.MnemonicsCache` under mono -- `Bk2MnemonicLookup`, which
older notes named, no longer exists in 2.11.1 -- and were cross-checked by generating a log entry
per button with BizHawk's own `Bk2LogEntryGenerator`. The exported route reads back through
BizHawk's `Bk2Movie.Load`: 12209 frames, header intact. Export queues
`verify/queue/<id>.bk2` + `<id>.json` (the `docs/route.md` contract, plus `bios`), and the
ledger's `tier2` line now says "not replayed", not "blocked".

**The BIOS gap is wired shut from our side.** `frlg_emu::boot_with_default_bios` boots every
route/run/info core from `$FRLG_GBA_BIOS`, else `$BIZHAWK_HOME/Firmware/GBA_bios.rom`, the moment
the file exists -- sha1-pinned to the World BIOS (`300c20df…`), refusing anything else, intro
skipped via `opts.skipBios`, which lands in the same `GBASkipBIOS` BizHawk's glue calls
(`src/platform/bizhawk/bizinterface.c:171` at the pinned commit; its `skipbios` comes from the
movie SyncSettings, where `SkipBios` is true in our template). The ledger records `bios: "hle" |
"bios:<sha1>"` per build; `verify` refuses a boot mismatch; `export` warns on an HLE route;
doctor prints the BIOS state every startup. **When the file lands: rebuild, verify, export** --
the battle will re-roll again (real-BIOS SWIs are not HLE-cycle-identical), and that rebuild is
the point, not a regression.

**Unverified.** Everything tier 2 still: the runner has never replayed a movie, and the queued
`route-12209f-fb2fc4969219.bk2` is expected to desync if replayed before the route is rebuilt on
the real BIOS -- it exists to exercise the pipeline, and its request json says `"bios": "hle"`.

## 2026-08-11 -- tier 2 has a format, a runner, and one thing left that money cannot buy

Worked through `docs/sanity-2026-08-11.md` on the host. The route did not move; what moved is how
much of tier 2 is knowable.

**`route/template.bk2` exists, and was not recorded by hand.** The plan was to open BizHawk and
record a one-frame movie. That path dead-ends: loading a ROM into the mGBA core for a movie sets
`DeterministicEmulationRequested`, and `MGBAHawk`'s constructor throws
`MissingFirmwareException("A BIOS is required for deterministic recordings!")` — which EmuHawk
shows as the Firmware Manager dialog rather than returning an exit code. So the template is built
instead: `tools/bk2-template.sh` loads the shipped assemblies under mono, asks
`Bk2LogEntryGenerator.GenerateLogKey(MGBAHawk.GBAController)` and `ConfigService.SaveWithType(new
SyncSettings())`, and writes the file with BizHawk's own `Bk2Movie.Write`. No GUI, no core
instance, no BIOS, and it reads back through BizHawk's own loader. Reproducible beats recorded.

The column order, which two sessions had called underivable:

    #Tilt X|Tilt Y|Tilt Z|Light Sensor|Up|Down|Left|Right|Start|Select|B|A|L|R|Power|

Four analogue columns before any button, and `Power` last. Anyone emitting the ten buttons from
`defctrl.json` in `defctrl.json` order would have produced a file that loads and desyncs.

**Tier 2 needs a real GBA BIOS, and that is now the whole blocker.** Same IL as above. It has two
consequences worth carrying: an unattended runner hangs on a dialog instead of failing (so
`tools/verify-runner.sh` preflights for the file, sha1
`300C20DF6731A33952DED8C436F7F186D25D3492`), and **tier 1 and tier 2 do not boot the same way** —
tier 1 runs mGBA's HLE BIOS and tier 2 cannot. `Emu::load_bios` exists, so closing that is
configuration, not code. Until it is closed, an early desync has an obvious suspect that has
nothing to do with the route.

**The two tiers also run different emulators, and the fix is not one line.** BizHawk 2.11.1's
`[PortedCore]` attribute says mGBA `0.11`; its submodule gitlink is `94b1578f` (2026-03-03), an
untagged master commit. Built it and pointed the workspace at it: `crates/mgba-sys/csrc/shim.c`
does not compile, because 0.11 dropped `getGameTitle`/`getGameCode` from `struct mCore` and moved
`VFileOpen`. So `MGBA_REF` is now an explicit `0.10.5` rather than "newest 0.10.x", both versions
are recorded in the deps `MANIFEST`, and `bin/frlg-doctor` says the delta out loud at every
startup. Porting the shim is real work and is the next tier-2 item after the BIOS.

**Also done, from the same review.** A repo `CLAUDE.md` (the inherited one contradicts this
sandbox on nearly every point); seven new doctor checks, of which "the writable decomp copy still
matches the read-only mount" is the one that protects every citation in these docs; a decomp
revision stamp in the kit spec so a restarted sandbox cannot silently cite a stale tree; the
`decompiled/` symlink made at startup rather than only by doctor; the empty Python wheelhouse and
its `PIP_*` variables removed rather than filled in.

**Unverified.** `tools/verify-runner.sh` and `tools/verify-runner.lua` have never completed a
replay — they cannot until the BIOS exists. Treat the Lua side, especially its memory-domain
names, as the least-tested code here.

## 2026-08-10 -- the battle was luck, and the obvious trim made things worse

**The win was a coin flip.** Delaying the same A mash into the rival battle by one frame flipped it
from a win to a loss, alternating over twelve consecutive delays. So `08-battle-win` now searches:
16 start delays, keep the shortest that wins, print how many did. Same 11873 frames as before -- the
route did not get faster, it got *chosen* instead of lucky, which is what stops it losing the next
time something upstream moves.

**Criticals are off in this battle.** `gBattleTypeFlags` is `0x1C`, which includes
`BATTLE_TYPE_FIRST_BATTLE`, and the crit check is gated on that flag being clear (or on the tutorial
having spoken): `decompiled/src/battle_script_commands.c:1199`. So the spread is damage variance
(85-100%, `:1558`) and accuracy (`:1093`) only. An earlier guess in this journal that a critical was
what made Squirtle's battle short was wrong; it cannot have been.

**Local trims are not free.** `06-starter` held UP for 8 frames to face the ball; 1 is enough.
Trimming it saved 6 frames and cost 391 in the battle, because the shifted `gRngValue` produced a
battle needing two more attacks. Net 385 slower. That is now the reason `Tuning` exists: knobs like
this are route-level, recorded in the ledger, and swept end-to-end by `frlg route tune`, which scores
each variant on total frames to the win rather than on the segment it lives in.

Swept all eight values end-to-end afterwards: 8 (untrimmed) is the best at 11873, and every trim is
163 to 958 frames worse, with no monotonicity at all. So the route is unchanged in frames and much
better understood.

**Carry this forward.** Every future optimisation upstream of a fight has to be measured through the
fight. Anything that reports "saved N frames" without re-running the battle is not evidence.

## 2026-08-10 -- first working route: power-on to a beaten rival

**Done.** 11873 frames, Squirtle, tier-1 verified (`docs/route.md`, `route/ledger.json`). Segment
code in `crates/frlg-route`, logs in `route/logs`, checkpoints in `$FRLG_ARTIFACTS/states/route`
(which do not survive the sandbox -- the logs are the artifact).

**What the route is built on.** Three pieces, in the order they were needed:

- `Recorder` -- one mask per advanced frame, no exceptions. This is what lets a segment be written
  as "wait until X" and still be a frame-exact log.
- `Observer` -- struct offsets transcribed from the decomp with citations, then checked against the
  running game (`tests/observe.rs`). `gMain.callback2` resolved through `pokefirered.sym` turned out
  to be the single most useful probe: it names the screen the game is on, which is what most of the
  intro's segment boundaries are.
- `nav` -- path search inside the emulator. Never reads the collision map. Walked bedroom -> lab in
  895 frames on its first run, which is when it became clear the route did not need any hand-written
  movement at all.

**What went wrong, and is worth not repeating.**

- `gSaveBlock1Ptr->playerPartyCount` is *not* the live party count. It is a copy that
  `SavePlayerParty` makes (`decompiled/src/load_save.c:164`); the live one is `gPlayerPartyCount`.
  The starter segment sat there mashing A at a Squirtle it had already been given, because the probe
  was reading a field that only updates when the game saves. Cost a debugging round trip; the
  screenshot is what settled it, not the numbers.
- `player_can_step` (runningState/tileTransitionState/preventStep all clear) is never true while a
  direction is held, because the game just keeps walking. The nav edge therefore ends when
  `gSaveBlock1Ptr->pos` changes -- mid-animation, deliberately -- and chaining edges from there is
  exactly what holding the button does. The first version waited for the player to settle and priced
  every tile as a fresh standing start; it also never terminated an edge, so the search expanded one
  node and gave up.
- Mashing A through the starter dialogue says YES to the nickname prompt and buys a naming screen.
  The tail of that segment is a B mash for that one reason.

**Next, in the order that pays.**

1. The battle, 3461 frames of the 11873. Nothing about it is manipulated yet -- it is a mash, and it
   wins on whatever rolls the RNG happens to hand out. `gRngValue` is the lever; the damage and crit
   path in `decompiled/src/battle_script_commands.c` is the thing to read first.
2. Starter choice by measurement: build all three, compare frames-to-win. The rival always takes the
   counter, so this is not obvious in either direction.
3. Naming: preset name vs. the current mash. ~3300 frames sit in those two segments.
4. Text speed. The route never opens OPTIONS; whether the detour pays for itself over ~40 message
   boxes is arithmetic nobody has done here.

**Unverified.** Everything tier 2. No `.bk2` exists and none can be written until
`route/template.bk2` does. The HLE-BIOS caveat from `docs/harness.md` still stands and is untested.
*(2026-08-11: the template now exists and the BIOS caveat turned out to be load-bearing — see the
entry above.)*
